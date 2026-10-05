use crate::interop::completion_helper::{PromiseError, PromiseFuture};
use crate::interop::{storage_helper, stream_helper, JsObject};
use ferroui_base::logging::{LogArea, LogEventLevel, Logger};
use ferroui_base::threading::Dispatcher;
use std::cell::RefCell;
use std::future::Future;
use std::io::{self, Seek, SeekFrom, Write};
use std::rc::Rc;
use std::task::{Poll, Waker};

/// Loose wrapper implementation of a stream on top of the File System
/// Access API's `FileSystemWritableFileStream`.
///
/// The page writes only asynchronously. [`write_async`](Self::write_async),
/// [`flush_async`](Self::flush_async) and [`close_async`](Self::close_async)
/// are upstream's `WriteAsync`, `FlushAsync` and `DisposeAsync`. The
/// synchronous [`Write`] the storage contracts hand out issues the same
/// writes without waiting for them (the browser cannot block): the script
/// stream queues them in order, and `write` and `flush` report a failure
/// that has already settled. Dropping the stream closes it, which commits
/// the file once the queued writes are done. Until that close settles, the
/// storage items wait before they open the same file or read its
/// properties, and a failed write or close is logged and reported by the
/// next opening of that file (see [`settle_pending_closes`]).
pub struct WriteableStream {
    js_reference: Option<JsObject>,
    // The storage item of the file, to which a failed close is reported.
    item: Option<JsObject>,
    // Unfortunately we can't read current length/position, so we need to keep it on this side
    // only.
    length: u64,
    position: u64,
    requests: Rc<RefCell<Requests>>,
}

/// The requests of a stream nobody waits for, and the first failure among
/// them.
#[derive(Default)]
struct Requests {
    pending: usize,
    error: Option<PromiseError>,
    wakers: Vec<Waker>,
}

fn disposed() -> io::Error {
    io::Error::other("Cannot access a disposed object. Object name: 'WriteableStream'.")
}

impl WriteableStream {
    /// The stream `js_reference` over the file of the storage item `item`,
    /// which is `initial_length` bytes long when the stream opens.
    pub(crate) fn new(js_reference: JsObject, item: Option<JsObject>, initial_length: u64) -> Self {
        Self { js_reference: Some(js_reference), item, length: initial_length, position: 0, requests: Rc::default() }
    }

    fn js_reference(&self) -> io::Result<&JsObject> {
        self.js_reference.as_ref().ok_or_else(disposed)
    }

    pub fn can_read(&self) -> bool {
        false
    }

    pub fn can_seek(&self) -> bool {
        true
    }

    pub fn can_write(&self) -> bool {
        true
    }

    pub fn length(&self) -> u64 {
        self.length
    }

    pub fn position(&self) -> u64 {
        self.position
    }

    /// Moves the position to `position`.
    pub fn set_position(&mut self, position: u64) -> io::Result<()> {
        self.seek(SeekFrom::Start(position)).map(|_| ())
    }

    /// Resizes the file to `value` bytes.
    pub fn set_length(&mut self, value: u64) -> io::Result<()> {
        let js_reference = self.js_reference()?.clone();
        self.length = value;

        // See https://docs.w3cub.com/dom/filesystemwritablefilestream/truncate
        // If the offset is smaller than the size, it remains unchanged. If the offset is larger
        // than size, the offset is set to that size.
        if self.position > self.length {
            self.position = self.length;
        }

        self.observe(stream_helper::truncate(&js_reference, value));
        Ok(())
    }

    /// Writes `buffer` at the position; resolves once the page has written
    /// it.
    pub async fn write_async(&mut self, buffer: &[u8]) -> io::Result<()> {
        let task = self.write_internal(buffer)?;
        task.await.map(|_| ()).map_err(io::Error::from)
    }

    fn write_internal(&mut self, buffer: &[u8]) -> io::Result<PromiseFuture<JsObject>> {
        let js_reference = self.js_reference()?;
        let task = stream_helper::write_async(js_reference, buffer);
        self.position += buffer.len() as u64;
        self.length = self.length.max(self.position);
        Ok(task)
    }

    /// Resolves once every write, seek and truncation issued through the
    /// synchronous members has settled, with the first failure among them.
    pub async fn flush_async(&mut self) -> io::Result<()> {
        requests_settled(self.requests.clone()).await;
        self.take_error()
    }

    /// Closes the stream, which commits the written content to the file.
    pub async fn close_async(mut self) -> io::Result<()> {
        let Some(js_reference) = self.js_reference.take() else {
            return Ok(());
        };
        let close = stream_helper::close_async(&js_reference).await;
        requests_settled(self.requests.clone()).await;
        self.take_error()?;
        close.map(|_| ()).map_err(io::Error::from)
    }

    /// Counts a request nobody waits for and records its failure, for the
    /// next call to report.
    fn observe(&self, task: PromiseFuture<JsObject>) {
        self.requests.borrow_mut().pending += 1;
        let requests = self.requests.clone();
        Dispatcher::ui_thread().to_task_scheduler().start_local(async move {
            let result = task.await;
            let wakers = {
                let mut requests = requests.borrow_mut();
                if let Err(failure) = result {
                    requests.error.get_or_insert(failure);
                }
                requests.pending -= 1;
                if requests.pending == 0 {
                    std::mem::take(&mut requests.wakers)
                } else {
                    Vec::new()
                }
            };
            wakers.into_iter().for_each(Waker::wake);
        });
    }

    fn take_error(&self) -> io::Result<()> {
        match self.requests.borrow_mut().error.take() {
            Some(error) => Err(error.into()),
            None => Ok(()),
        }
    }
}

/// Resolves once no request of `requests` is pending.
fn requests_settled(requests: Rc<RefCell<Requests>>) -> impl Future<Output = ()> {
    std::future::poll_fn(move |cx| {
        let mut requests = requests.borrow_mut();
        if requests.pending == 0 {
            Poll::Ready(())
        } else {
            requests.wakers.push(cx.waker().clone());
            Poll::Pending
        }
    })
}

impl Write for WriteableStream {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.take_error()?;
        let task = self.write_internal(buffer)?;
        self.observe(task);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // The page queues every write and cannot be waited for here: reports a failure that has
        // settled. `flush_async` waits.
        self.take_error()
    }
}

impl Seek for WriteableStream {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let js_reference = self.js_reference()?.clone();
        let position = seek_target(position, self.position, self.length)?;
        self.observe(stream_helper::seek(&js_reference, position));
        // Upstream returns the new position without storing it.
        self.position = position;
        Ok(position)
    }
}

fn seek_target(position: SeekFrom, current: u64, length: u64) -> io::Result<u64> {
    let position = match position {
        SeekFrom::Current(offset) => current as i64 + offset,
        SeekFrom::End(offset) => length as i64 + offset,
        SeekFrom::Start(offset) => offset as i64,
    };
    u64::try_from(position).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "An attempt was made to move the position before the beginning of the stream.",
        )
    })
}

impl Drop for WriteableStream {
    fn drop(&mut self) {
        let Some(js_reference) = self.js_reference.take() else {
            return;
        };
        // Upstream's `Dispose`: close without waiting. The close stays registered for its file
        // until it settles, and with its failure until an opening of the file reports it.
        let close = stream_helper::close_async(&js_reference);
        let pending = Rc::new(PendingClose { item: self.item.take(), state: RefCell::default() });
        PENDING_CLOSES.with(|closes| closes.borrow_mut().push(pending.clone()));
        let requests = self.requests.clone();
        Dispatcher::ui_thread().to_task_scheduler().start_local(async move {
            let close = close.await;
            requests_settled(requests.clone()).await;
            let error = requests.borrow_mut().error.take().or(close.err());
            if let Some(error) = &error {
                if let Some(logger) = Logger::try_get(LogEventLevel::Error, LogArea::BROWSER_PLATFORM) {
                    logger.log(None, &format!("Writing a file failed: {error}"));
                }
            }
            pending.settle(error);
        });
    }
}

/// The close of a dropped stream: settled once the page has committed the
/// file or failed to.
#[derive(Default)]
struct CloseState {
    settled: bool,
    error: Option<PromiseError>,
    wakers: Vec<Waker>,
}

struct PendingClose {
    item: Option<JsObject>,
    state: RefCell<CloseState>,
}

impl PendingClose {
    fn settle(self: &Rc<Self>, error: Option<PromiseError>) {
        let wakers = {
            let mut state = self.state.borrow_mut();
            state.settled = true;
            state.error = error;
            std::mem::take(&mut state.wakers)
        };
        // A close that succeeded has nothing left to report.
        if self.state.borrow().error.is_none() {
            PENDING_CLOSES.with(|closes| closes.borrow_mut().retain(|close| !Rc::ptr_eq(close, self)));
        }
        wakers.into_iter().for_each(Waker::wake);
    }

    fn settled(self: Rc<Self>) -> impl Future<Output = ()> {
        std::future::poll_fn(move |cx| {
            let mut state = self.state.borrow_mut();
            if state.settled {
                Poll::Ready(())
            } else {
                state.wakers.push(cx.waker().clone());
                Poll::Pending
            }
        })
    }

    /// Takes the failure of the settled close, which is then reported.
    fn take_error(self: &Rc<Self>) -> Option<PromiseError> {
        let error = self.state.borrow_mut().error.take();
        PENDING_CLOSES.with(|closes| closes.borrow_mut().retain(|close| !Rc::ptr_eq(close, self)));
        error
    }
}

thread_local! {
    static PENDING_CLOSES: RefCell<Vec<Rc<PendingClose>>> = const { RefCell::new(Vec::new()) };
}

/// Waits for the closes of the dropped streams over the file of the storage
/// item `item` (the same entry, whichever item opened it). A file written
/// through the synchronous [`Write`] has its content committed then.
///
/// With `report`, fails with the first failed write or close among them,
/// which is then reported; without, failures are left for the next
/// opening.
pub(crate) async fn settle_pending_closes(item: &JsObject, report: bool) -> io::Result<()> {
    let closes = PENDING_CLOSES.with(|closes| closes.borrow().clone());
    let mut first_error = None;
    for close in closes {
        let Some(close_item) = &close.item else { continue };
        if !is_same_entry(item, close_item).await {
            continue;
        }
        close.clone().settled().await;
        if report {
            if let Some(error) = close.take_error() {
                first_error.get_or_insert(error);
            }
        }
    }
    match first_error {
        Some(error) => Err(io::Error::other(format!("Writing the file failed: {error}"))),
        None => Ok(()),
    }
}

async fn is_same_entry(item: &JsObject, other: &JsObject) -> bool {
    if item == other {
        return true;
    }
    storage_helper::is_same_entry(item, other).await.ok().and_then(|same| same.as_bool()).unwrap_or(false)
}

#[cfg(test)]
mod tests {
    // Not from upstream (upstream has no tests of the browser streams): the parts that need no
    // page.
    use super::*;
    use std::task::Context;

    #[test]
    fn a_close_settles_for_its_waiters_and_keeps_only_its_failure() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use std::task::Wake;

        struct CountingWaker(AtomicUsize);
        impl Wake for CountingWaker {
            fn wake(self: Arc<Self>) {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
        }
        let counter = Arc::new(CountingWaker(AtomicUsize::new(0)));
        let waker = Waker::from(counter.clone());
        let mut context = Context::from_waker(&waker);

        let succeeded = Rc::new(PendingClose { item: None, state: RefCell::default() });
        let failed = Rc::new(PendingClose { item: None, state: RefCell::default() });
        PENDING_CLOSES.with(|closes| closes.borrow_mut().extend([succeeded.clone(), failed.clone()]));

        let mut waiting = Box::pin(succeeded.clone().settled());
        assert_eq!(waiting.as_mut().poll(&mut context), Poll::Pending);
        succeeded.settle(None);
        assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        assert_eq!(waiting.as_mut().poll(&mut context), Poll::Ready(()));

        failed.settle(Some(PromiseError::new("", "The disk is full")));
        assert_eq!(PENDING_CLOSES.with(|closes| closes.borrow().len()), 1, "only the failed close is kept");
        assert_eq!(failed.take_error(), Some(PromiseError::new("", "The disk is full")));
        assert_eq!(PENDING_CLOSES.with(|closes| closes.borrow().len()), 0, "a reported failure is dropped");
    }

    #[test]
    fn requests_settled_waits_for_the_pending_requests() {
        let requests = Rc::new(RefCell::new(Requests { pending: 1, ..Default::default() }));
        let mut settled = Box::pin(requests_settled(requests.clone()));
        let waker = Waker::noop();
        let mut context = Context::from_waker(waker);
        assert_eq!(settled.as_mut().poll(&mut context), Poll::Pending);
        requests.borrow_mut().pending = 0;
        assert_eq!(settled.as_mut().poll(&mut context), Poll::Ready(()));
    }

    #[test]
    fn seek_target_is_relative_to_each_origin() {
        assert_eq!(seek_target(SeekFrom::Start(3), 7, 10).unwrap(), 3);
        assert_eq!(seek_target(SeekFrom::Current(2), 7, 10).unwrap(), 9);
        assert_eq!(seek_target(SeekFrom::End(-4), 7, 10).unwrap(), 6);
        assert_eq!(seek_target(SeekFrom::End(5), 7, 10).unwrap(), 15);
        assert!(seek_target(SeekFrom::Current(-8), 7, 10).is_err());
    }
}
