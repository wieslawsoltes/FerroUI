//! GStreamer playback using one native pipeline for decoded video and audio.
#![cfg(target_os = "linux")]
#![forbid(unsafe_code)]
#[cfg(feature = "controls")]
pub use ferroui_media_linux::NativeGlTextureImporter;
use ferroui_media_linux::{admitted_caps, poll_native, LinuxFrameStream, NativeSession};
use ferroui_media_opengl::NativeGlFrameProducer;
use ferroui_media_video::{frames::*, *};
use std::{io::Write, path::PathBuf, rc::Rc, task::Poll, time::Duration};

pub struct LinuxVideoProvider {
    pub frames: NativeGlFrameProducer,
}
// Canceled filesystem preparation retains its slot until the actual write
// finishes. Repeated source replacement cannot accumulate unbounded workers.
static PREPARING: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
struct SourceWork;
impl SourceWork {
    fn acquire() -> Result<Self, MediaError> {
        use std::sync::atomic::Ordering;
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
impl Drop for SourceWork {
    fn drop(&mut self) {
        PREPARING.fetch_sub(1, std::sync::atomic::Ordering::AcqRel);
    }
}
struct Resource(PathBuf);
impl Drop for Resource {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
fn error(message: impl Into<String>) -> MediaError {
    MediaError::new(MediaErrorKind::InvalidSource, message)
}
fn file_uri(path: PathBuf) -> Result<String, MediaError> {
    use std::os::unix::ffi::OsStrExt;
    let absolute = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .map_err(|e| error(e.to_string()))?
            .join(path)
    };
    let mut uri = String::from("file://");
    for &byte in absolute.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~".contains(&byte) {
            uri.push(byte as char)
        } else {
            uri.push_str(&format!("%{byte:02X}"));
        }
    }
    Ok(uri)
}
fn resolve(source: MediaSource) -> Result<(String, Option<Resource>), MediaError> {
    match source {
        MediaSource::File(path) => Ok((file_uri(path)?, None)),
        MediaSource::Url(url)
            if ["https://", "http://", "file://"]
                .iter()
                .any(|scheme| url.starts_with(scheme)) =>
        {
            Ok((url, None))
        }
        MediaSource::Url(_) => Err(error("Unsupported network URL scheme")),
        MediaSource::Resource { encoded, .. } => {
            use std::os::unix::fs::OpenOptionsExt;
            let path = std::env::temp_dir().join(format!(
                "ferroui-resource-{}-{}.media",
                std::process::id(),
                ferroui_media::allocate_source_id()
            ));
            let mut file = std::fs::OpenOptions::new()
                .mode(0o600)
                .create_new(true)
                .write(true)
                .open(&path)
                .map_err(|e| error(e.to_string()))?;
            let owner = Resource(path);
            file.write_all(&encoded).map_err(|e| error(e.to_string()))?;
            Ok((file_uri(owner.0.clone())?, Some(owner)))
        }
    }
}
struct Session {
    stream: LinuxFrameStream,
    _resource: Option<Resource>,
}
impl MediaPlayerProvider for LinuxVideoProvider {
    fn open(
        &self,
        source: MediaSource,
        id: u64,
        generation: u64,
        _: PresentationPolicy,
    ) -> MediaFuture<Rc<dyn NativeMediaSession>> {
        let frames = self.frames.clone();
        Box::pin(async move {
            let caps = admitted_caps(&frames, None).await?;
            // Resource materialization can be large and must not block the UI.
            let (send, receive) = std::sync::mpsc::sync_channel(1);
            let permit = SourceWork::acquire()?;
            std::thread::Builder::new()
                .name("FerroUI.Media.Source".into())
                .spawn(move || {
                    let _permit = permit;
                    let _ = send.send(resolve(source));
                })
                .map_err(|e| MediaError::new(MediaErrorKind::NativeFailure, e.to_string()))?;
            let (url, resource) = poll_native(move || match receive.try_recv() {
                Ok(v) => Poll::Ready(v),
                Err(std::sync::mpsc::TryRecvError::Empty) => Poll::Pending,
                Err(_) => Poll::Ready(Err(error("Source preparation worker failed"))),
            })
            .await?;
            let native = NativeSession::open(&url, &caps, 0, -1, 0)?;
            let mut session = Some(Session {
                stream: LinuxFrameStream::new(native, frames, id, generation),
                _resource: resource,
            });
            poll_native(
                move || match session.as_ref().unwrap().stream.native.state() {
                    Err(e) => Poll::Ready(Err(e)),
                    Ok(s) if s.state == 0 => Poll::Pending,
                    Ok(_) => Poll::Ready(Ok(
                        Rc::new(session.take().unwrap()) as Rc<dyn NativeMediaSession>
                    )),
                },
            )
            .await
        })
    }
}
impl NativeMediaSession for Session {
    fn state(&self) -> Result<MediaPlaybackSession, MediaError> {
        let s = self.stream.native.state()?;
        Ok(MediaPlaybackSession {
            state: if s.buffering != 0 {
                MediaPlaybackState::Buffering
            } else {
                match s.state {
                    0 => MediaPlaybackState::Opening,
                    1 => MediaPlaybackState::Paused,
                    2 => MediaPlaybackState::Playing,
                    4 => MediaPlaybackState::Ended,
                    _ => MediaPlaybackState::Failed,
                }
            },
            position: Duration::from_nanos(s.position_ns),
            duration: (s.duration_ns != u64::MAX).then(|| Duration::from_nanos(s.duration_ns)),
            natural_size: (s.width > 0 && s.height > 0).then_some((s.width, s.height)),
            seekable: s.seekable != 0,
            decode: Acceleration::Unknown,
            error: None,
        })
    }
    fn play(&self) -> MediaFuture<()> {
        self.stream.native.intent(1);
        let native = self.stream.native.clone();
        poll_native(move || match native.state() {
            Err(e) => Poll::Ready(Err(e)),
            Ok(s) if s.state == 2 => Poll::Ready(Ok(())),
            _ => Poll::Pending,
        })
    }
    fn pause(&self) -> Result<(), MediaError> {
        self.stream.native.state()?;
        self.stream.native.intent(0);
        Ok(())
    }
    fn stop(&self) -> Result<(), MediaError> {
        self.pause()
    }
    fn seek(&self, position: Duration) -> MediaFuture<Duration> {
        let native = self.stream.native.clone();
        let ns = u64::try_from(position.as_nanos()).unwrap_or(u64::MAX);
        let epoch = native.seek(ns);
        self.stream.retire(epoch);
        poll_native(move || match native.state() {
            Err(e) => Poll::Ready(Err(e)),
            Ok(s) if s.seek_done >= epoch => Poll::Ready(Ok(Duration::from_nanos(s.position_ns))),
            _ => Poll::Pending,
        })
    }
    fn set_audio(&self, volume: f64, muted: bool) -> Result<(), MediaError> {
        self.stream.native.state()?;
        self.stream.native.audio(volume, muted);
        Ok(())
    }
    fn take_frame(&self) -> Result<Option<FrameLease>, MediaError> {
        self.stream.take_frame()
    }
}
