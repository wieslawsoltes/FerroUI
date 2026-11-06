//! GStreamer playback using one native pipeline for decoded video and audio.
#![cfg(target_os = "linux")]
#![forbid(unsafe_code)]
#[cfg(feature = "controls")]
pub use ferroui_media_linux::NativeGlTextureImporter;
use ferroui_media_linux::{admitted_caps, poll_native, LinuxFrameStream, NativeSession};
use ferroui_media_opengl::NativeGlFrameProducer;
use ferroui_media_video::{frames::*, *};
use std::{path::PathBuf, rc::Rc, task::Poll, time::Duration};

pub struct LinuxVideoProvider {
    pub frames: NativeGlFrameProducer,
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
struct Session {
    stream: LinuxFrameStream,
    _resource: Option<TemporaryMediaFile>,
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
            let PreparedMediaSource {
                location,
                temporary: resource,
            } = source.prepare_native(None).await?;
            let url = match location {
                NativeMediaLocation::Url(url) => url,
                NativeMediaLocation::File(path) => file_uri(path)?,
            };
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
    fn has_pending_frame_work(&self) -> bool {
        self.stream.has_pending_frame_work()
    }
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
