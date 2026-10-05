use crate::interop::promise_helper::{JsError, JsTask};
use crate::interop::{stream_helper, JsObject};
use ferroui_base::threading::Dispatcher;
use std::cell::RefCell;
use std::future::Future;
use std::io::{self, Seek, SeekFrom, Write};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

/// Loose wrapper implementation of a stream on top of the File System
/// Access API's `FileSystemWritableFileStream`.
///
/// The page writes only asynchronously. [`write_async`](Self::write_async)
/// and [`close_async`](Self::close_async) are upstream's `WriteAsync` and
/// `DisposeAsync`. The synchronous [`Write`] the storage contracts hand out
/// issues the same writes without waiting for them: the script stream
/// queues them in order, and the first failure is reported by the next
/// call to `write` or `flush`. Dropping the stream closes it, which commits
/// the file once the queued writes are done; await
/// [`close_async`](Self::close_async) to know when that happened. The
/// storage items wait for such closes before they open a file or read its
/// properties (see [`closes_settled`]).
pub struct WriteableStream {
    js_reference: Option<JsObject>,
    // Unfortunately we can't read current length/position, so we need to keep it on this side
    // only.
    length: u64,
    position: u64,
    error: Rc<RefCell<Option<JsError>>>,
}

fn disposed() -> io::Error {
    io::Error::other("Cannot access a disposed object. Object name: 'WriteableStream'.")
}

impl WriteableStream {
    pub(crate) fn new(js_reference: JsObject, initial_length: u64) -> Self {
        Self { js_reference: Some(js_reference), length: initial_length, position: 0, error: Rc::default() }
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

    fn write_internal(&mut self, buffer: &[u8]) -> io::Result<JsTask> {
        let js_reference = self.js_reference()?;
        let task = stream_helper::write_async(js_reference, buffer);
        self.position += buffer.len() as u64;
        self.length = self.length.max(self.position);
        Ok(task)
    }

    /// Closes the stream, which commits the written content to the file.
    pub async fn close_async(mut self) -> io::Result<()> {
        let Some(js_reference) = self.js_reference.take() else {
            return Ok(());
        };
        stream_helper::close_async(&js_reference).await.map(|_| ()).map_err(io::Error::from)?;
        self.take_error()
    }

    /// Records the failure of a request nobody waits for, for the next call
    /// to report.
    fn observe(&self, task: JsTask) {
        let error = self.error.clone();
        Dispatcher::ui_thread().to_task_scheduler().start_local(async move {
            if let Err(failure) = task.await {
                error.borrow_mut().get_or_insert(failure);
            }
        });
    }

    fn take_error(&self) -> io::Result<()> {
        match self.error.borrow_mut().take() {
            Some(error) => Err(error.into()),
            None => Ok(()),
        }
    }
}

impl Write for WriteableStream {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.take_error()?;
        let task = self.write_internal(buffer)?;
        self.observe(task);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // Nothing to flush: the page queues every write. Reports a failed earlier write.
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
        if let Some(js_reference) = self.js_reference.take() {
            // Upstream's `Dispose`: close without waiting. The close is counted until it settles.
            let close = stream_helper::close_async(&js_reference);
            PENDING_CLOSES.with(|pending| pending.borrow_mut().count += 1);
            Dispatcher::ui_thread().to_task_scheduler().start_local(async move {
                let _ = close.await;
                close_settled();
            });
        }
    }
}

/// Closes of dropped streams that have not settled yet.
#[derive(Default)]
struct PendingCloses {
    count: usize,
    wakers: Vec<Waker>,
}

thread_local! {
    static PENDING_CLOSES: RefCell<PendingCloses> = RefCell::new(PendingCloses::default());
}

fn close_settled() {
    let wakers = PENDING_CLOSES.with(|pending| {
        let mut pending = pending.borrow_mut();
        pending.count -= 1;
        if pending.count == 0 {
            std::mem::take(&mut pending.wakers)
        } else {
            Vec::new()
        }
    });
    wakers.into_iter().for_each(Waker::wake);
}

/// Resolves once no stream dropped so far is still closing: a file written
/// through the synchronous [`Write`] has its content committed then.
pub(crate) fn closes_settled() -> impl Future<Output = ()> {
    ClosesSettled
}

struct ClosesSettled;

impl Future for ClosesSettled {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        PENDING_CLOSES.with(|pending| {
            let mut pending = pending.borrow_mut();
            if pending.count == 0 {
                Poll::Ready(())
            } else {
                pending.wakers.push(cx.waker().clone());
                Poll::Pending
            }
        })
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream (upstream has no tests of the browser streams): the position arithmetic,
    // which needs no page.
    use super::*;

    #[test]
    fn closes_settled_waits_for_every_pending_close() {
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

        assert_eq!(Pin::new(&mut closes_settled()).poll(&mut context), Poll::Ready(()));

        PENDING_CLOSES.with(|pending| pending.borrow_mut().count += 2);
        let mut settled = closes_settled();
        assert_eq!(Pin::new(&mut settled).poll(&mut context), Poll::Pending);
        close_settled();
        assert_eq!(counter.0.load(Ordering::SeqCst), 0);
        close_settled();
        assert_eq!(counter.0.load(Ordering::SeqCst), 1);
        assert_eq!(Pin::new(&mut settled).poll(&mut context), Poll::Ready(()));
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
