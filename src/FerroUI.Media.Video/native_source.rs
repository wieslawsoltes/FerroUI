//! Native source preparation. Encoded resources are materialized off the UI
//! thread; no decoded pixels or preview transfer pass through this module.
use crate::{MediaError, MediaErrorKind, MediaFuture, MediaSource};
use std::{
    io::Write,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    task::{Poll, Waker},
};

/// Encoded resources above this bound should use a file or network source.
pub const MAX_NATIVE_RESOURCE_BYTES: usize = 256 * 1024 * 1024;
#[derive(Debug)]
pub enum NativeMediaLocation {
    File(PathBuf),
    Url(String),
}
/// Owns the materialized file until the native player releases its source.
#[derive(Debug)]
pub struct TemporaryMediaFile(Option<PathBuf>);
impl TemporaryMediaFile {
    /// Transfer cleanup only after a native worker accepted responsibility for
    /// deleting this exact file after releasing its reader/source handle.
    pub fn transfer_cleanup(mut self) -> PathBuf {
        self.0.take().unwrap()
    }
}
impl Drop for TemporaryMediaFile {
    fn drop(&mut self) {
        if let Some(path) = self.0.take() {
            let _ = std::fs::remove_file(path);
        }
    }
}
#[derive(Debug)]
pub struct PreparedMediaSource {
    pub location: NativeMediaLocation,
    pub temporary: Option<TemporaryMediaFile>,
}
static PREPARING: AtomicUsize = AtomicUsize::new(0);
struct Permit;
impl Permit {
    fn acquire() -> Result<Self, MediaError> {
        let mut count = PREPARING.load(Ordering::Acquire);
        loop {
            if count >= 4 {
                return Err(MediaError::new(
                    MediaErrorKind::NativeFailure,
                    "Native source preparation slots are saturated",
                ));
            }
            match PREPARING.compare_exchange_weak(
                count,
                count + 1,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return Ok(Self),
                Err(current) => count = current,
            }
        }
    }
}
impl Drop for Permit {
    fn drop(&mut self) {
        PREPARING.fetch_sub(1, Ordering::AcqRel);
    }
}
struct State<T> {
    result: Option<Result<T, MediaError>>,
    waker: Option<Waker>,
}
struct Abandon<T> {
    canceled: Arc<AtomicBool>,
    state: Arc<Mutex<State<T>>>,
}
impl<T> Drop for Abandon<T> {
    fn drop(&mut self) {
        self.canceled.store(true, Ordering::Release);
        let waker = self.state.lock().unwrap().waker.take();
        drop(waker);
    }
}
fn worker<T: Send + 'static>(
    work: impl FnOnce(&AtomicBool) -> Result<T, MediaError> + Send + 'static,
) -> MediaFuture<T> {
    Box::pin(async move {
        let permit = Permit::acquire()?;
        let state = Arc::new(Mutex::new(State {
            result: None,
            waker: None,
        }));
        let canceled = Arc::new(AtomicBool::new(false));
        let abandon = Abandon {
            canceled: canceled.clone(),
            state: state.clone(),
        };
        let output = state.clone();
        std::thread::Builder::new()
            .name("FerroUI.Media.Source".into())
            .spawn(move || {
                let _permit = permit;
                let result =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| work(&canceled)))
                        .unwrap_or_else(|_| {
                            Err(MediaError::new(
                                MediaErrorKind::NativeFailure,
                                "Source preparation worker failed",
                            ))
                        });
                let waker = {
                    let mut out = output.lock().unwrap();
                    out.result = Some(result);
                    out.waker.take()
                };
                if let Some(waker) = waker {
                    waker.wake();
                }
            })
            .map_err(|e| MediaError::new(MediaErrorKind::NativeFailure, e.to_string()))?;
        let result = std::future::poll_fn(move |cx| {
            let mut state = state.lock().unwrap();
            match state.result.take() {
                Some(result) => Poll::Ready(result),
                None => {
                    state.waker = Some(cx.waker().clone());
                    Poll::Pending
                }
            }
        })
        .await;
        drop(abandon);
        result
    })
}
fn invalid(message: impl Into<String>) -> MediaError {
    MediaError::new(MediaErrorKind::InvalidSource, message)
}
impl MediaSource {
    /// Resolve supported native URLs/files or write an encoded resource to a
    /// scoped private file. Pass Android's application cache directory there;
    /// other hosts may use their OS temporary directory. At most four writes
    /// run concurrently across providers. Abandoning preparation cancels between
    /// bounded chunks and retires a file produced after the future was dropped.
    pub fn prepare_native(
        self,
        cache_directory: Option<PathBuf>,
    ) -> MediaFuture<PreparedMediaSource> {
        match self {
            Self::Url(url) => Box::pin(async move {
                if !["https://", "http://", "file://"]
                    .iter()
                    .any(|s| url.starts_with(s))
                {
                    return Err(invalid("Unsupported native media URL scheme"));
                }
                Ok(PreparedMediaSource {
                    location: NativeMediaLocation::Url(url),
                    temporary: None,
                })
            }),
            Self::File(path) => Box::pin(async move {
                Ok(PreparedMediaSource {
                    location: NativeMediaLocation::File(path),
                    temporary: None,
                })
            }),
            Self::Resource {
                encoded,
                content_type,
                ..
            } => worker(move |canceled| {
                if encoded.len() > MAX_NATIVE_RESOURCE_BYTES {
                    return Err(invalid(
                        "Encoded resource exceeds 256 MiB; use a file or URL",
                    ));
                }
                if canceled.load(Ordering::Acquire) {
                    return Err(MediaError::cancelled());
                }
                let suffix = match content_type.as_str() {
                    "video/mp4" | "audio/mp4" => "mp4",
                    "video/quicktime" => "mov",
                    "video/webm" | "audio/webm" => "webm",
                    "video/x-matroska" => "mkv",
                    "audio/mpeg" => "mp3",
                    "audio/wav" | "audio/x-wav" => "wav",
                    "video/x-ms-wmv" => "wmv",
                    "audio/x-ms-wma" => "wma",
                    _ => "media",
                };
                let path = cache_directory
                    .unwrap_or_else(std::env::temp_dir)
                    .join(format!(
                        "ferroui-resource-{}-{}.{suffix}",
                        std::process::id(),
                        ferroui_media::allocate_source_id()
                    ));
                let mut options = std::fs::OpenOptions::new();
                options.create_new(true).write(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    options.mode(0o600);
                }
                // Declare cleanup before the open handle so Windows closes
                // the writer before attempting removal on every error path.
                let mut owner = TemporaryMediaFile(None);
                let mut file = options.open(&path).map_err(|e| invalid(e.to_string()))?;
                owner.0 = Some(path.clone());
                for chunk in encoded.chunks(64 * 1024) {
                    if canceled.load(Ordering::Acquire) {
                        return Err(MediaError::cancelled());
                    }
                    file.write_all(chunk).map_err(|e| invalid(e.to_string()))?;
                }
                if canceled.load(Ordering::Acquire) {
                    return Err(MediaError::cancelled());
                }
                drop(file);
                Ok(PreparedMediaSource {
                    location: NativeMediaLocation::File(path),
                    temporary: Some(owner),
                })
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        future::Future,
        pin::Pin,
        sync::mpsc,
        task::{Context, Wake},
        time::Duration,
    };
    static SERIAL: Mutex<()> = Mutex::new(());
    fn idle() {
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while PREPARING.load(Ordering::Acquire) != 0 {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
    }
    struct Signal(mpsc::Sender<()>);
    impl Wake for Signal {
        fn wake(self: Arc<Self>) {
            let _ = self.0.send(());
        }
    }
    fn finish<T>(mut future: MediaFuture<T>) -> Result<T, MediaError> {
        let (send, receive) = mpsc::channel();
        let waker = Waker::from(Arc::new(Signal(send)));
        let mut cx = Context::from_waker(&waker);
        loop {
            match future.as_mut().poll(&mut cx) {
                Poll::Ready(v) => return v,
                Poll::Pending => receive.recv_timeout(Duration::from_secs(5)).unwrap(),
            }
        }
    }
    fn poll<T>(
        future: &mut Pin<Box<dyn Future<Output = Result<T, MediaError>>>>,
    ) -> Poll<Result<T, MediaError>> {
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
    }
    struct Directory(PathBuf);
    impl Directory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "ferroui-source-tests-{}-{}",
                std::process::id(),
                ferroui_media::allocate_source_id()
            ));
            std::fs::create_dir(&path).unwrap();
            Self(path)
        }
    }
    impl Drop for Directory {
        fn drop(&mut self) {
            std::fs::remove_dir(&self.0).expect("all owned files must retire");
        }
    }
    #[test]
    fn preparation_is_lazy_and_encoded_output_is_scoped_and_private() {
        let _serial = SERIAL.lock().unwrap();
        idle();
        let directory = Directory::new();
        let bytes: Arc<[u8]> = vec![42; 2 * 1024 * 1024].into();
        let source = MediaSource::Resource {
            name: "unused name/path".into(),
            content_type: "video/mp4".into(),
            encoded: bytes.clone(),
        };
        let abandoned = source.clone().prepare_native(Some(directory.0.clone()));
        assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 0);
        drop(abandoned);
        let prepared = finish(source.prepare_native(Some(directory.0.clone()))).unwrap();
        let NativeMediaLocation::File(path) = &prepared.location else {
            panic!("Resource was not materialized")
        };
        let path = path.clone();
        assert_eq!(path.extension().unwrap(), "mp4");
        assert_eq!(std::fs::read(&path).unwrap(), &*bytes);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        drop(prepared);
        assert!(!path.exists());
        assert!(finish(
            MediaSource::Url("ftp://example.invalid/video".into()).prepare_native(None)
        )
        .is_err());
    }
    #[test]
    fn canceled_native_write_keeps_its_slot_and_removes_late_output() {
        let _serial = SERIAL.lock().unwrap();
        idle();
        let directory = Directory::new();
        let path = directory.0.join("late.mp4");
        let file_path = path.clone();
        let (begun, started) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let (done, removed) = mpsc::channel();
        struct Late {
            file: Option<TemporaryMediaFile>,
            done: mpsc::Sender<()>,
        }
        impl Drop for Late {
            fn drop(&mut self) {
                drop(self.file.take());
                let _ = self.done.send(());
            }
        }
        let mut pending = worker(move |canceled| {
            std::fs::write(&file_path, b"encoded source").unwrap();
            let file = TemporaryMediaFile(Some(file_path));
            begun.send(()).unwrap();
            wait.recv().unwrap();
            assert!(canceled.load(Ordering::Acquire));
            Ok(Late {
                file: Some(file),
                done,
            })
        });
        assert!(poll(&mut pending).is_pending());
        started.recv_timeout(Duration::from_secs(5)).unwrap();
        drop(pending);
        assert_eq!(PREPARING.load(Ordering::Acquire), 1);
        assert!(path.exists());
        release.send(()).unwrap();
        removed.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(!path.exists());
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while PREPARING.load(Ordering::Acquire) != 0 {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
    }
    #[test]
    fn rapid_replacement_cannot_accumulate_filesystem_workers() {
        let _serial = SERIAL.lock().unwrap();
        idle();
        let mut futures = Vec::new();
        let mut exits = Vec::new();
        let mut ready = Vec::new();
        for _ in 0..4 {
            let (send, receive) = mpsc::channel();
            let (begun, started) = mpsc::channel();
            let mut pending = worker(move |_| {
                begun.send(()).unwrap();
                receive.recv().unwrap();
                Ok(())
            });
            assert!(poll(&mut pending).is_pending());
            futures.push(pending);
            exits.push(send);
            ready.push(started);
        }
        for begun in ready {
            begun.recv_timeout(Duration::from_secs(5)).unwrap();
        }
        drop(futures);
        let mut fifth = worker(|_| Ok(()));
        assert!(matches!(poll(&mut fifth), Poll::Ready(Err(_))));
        for exit in exits {
            exit.send(()).unwrap();
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while PREPARING.load(Ordering::Acquire) != 0 {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        finish(worker(|_| Ok(()))).unwrap();
    }
}
