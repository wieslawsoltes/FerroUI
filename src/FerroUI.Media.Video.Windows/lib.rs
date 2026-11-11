//! Media Foundation playback with one source reader and native WASAPI timing.
#![cfg(windows)]
#![forbid(unsafe_code)]
use ferroui_media_opengl::NativeGlFrameProducer;
use ferroui_media_video::{frames::*, *};
#[cfg(feature = "controls")]
pub use ferroui_media_windows::NativeGlTextureImporter;
use ferroui_media_windows::{poll_native, NativeDevice, NativeSession, WindowsFrameStream};
use std::{path::PathBuf, rc::Rc, task::Poll, time::Duration};

pub struct WindowsVideoProvider {
    pub frames: NativeGlFrameProducer,
}
fn error(message: impl Into<String>) -> MediaError {
    MediaError::new(MediaErrorKind::InvalidSource, message)
}
fn file_path(path: PathBuf) -> Result<String, MediaError> {
    let absolute = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()
            .map_err(|e| error(e.to_string()))?
            .join(path)
    };
    absolute
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| error("Media path is not valid Unicode"))
}
struct Session {
    stream: WindowsFrameStream,
}
impl MediaPlayerProvider for WindowsVideoProvider {
    fn open(
        &self,
        source: MediaSource,
        id: u64,
        generation: u64,
        _: PresentationPolicy,
    ) -> MediaFuture<Rc<dyn NativeMediaSession>> {
        let frames = self.frames.clone();
        Box::pin(async move {
            let device = NativeDevice::from_renderer(&frames).await?;
            let PreparedMediaSource {
                location,
                temporary: resource,
            } = source.prepare_native(None).await?;
            let url = match location {
                NativeMediaLocation::Url(url) => url,
                NativeMediaLocation::File(path) => file_path(path)?,
            };
            let native = NativeSession::open(&device, &url, false, resource.is_some(), None)?;
            // The native worker removes resource files after releasing its reader,
            // including cancellation while preparation is still opening the file.
            if let Some(resource) = resource {
                let _ = resource.transfer_cleanup();
            }
            let mut session = Some(Session {
                stream: WindowsFrameStream::new(native, frames, id, generation),
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
        // MF keeps both streams at EOS after Ended. Restart the seekable source
        // in a fresh publication epoch before requesting playback again.
        let restart_epoch = match self.stream.native.state() {
            Ok(s) if s.state == 4 && s.seekable != 0 => {
                let epoch = self.stream.native.seek(0);
                self.stream.retire(epoch);
                Some(epoch)
            }
            Err(error) => return Box::pin(async move { Err(error) }),
            _ => None,
        };
        self.stream.native.intent(1);
        let native = self.stream.native.clone();
        poll_native(move || match native.state() {
            Err(e) => Poll::Ready(Err(e)),
            Ok(s) if s.state == 2 && restart_epoch.is_none_or(|epoch| s.seek_done >= epoch) => {
                Poll::Ready(Ok(()))
            }
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
    fn supports_playback_rate(&self) -> bool {
        self.stream.native.supports_playback_rate()
    }
    fn set_playback_rate(&self, rate: f64) -> Result<(), MediaError> {
        self.stream.native.set_playback_rate(rate)
    }
    fn take_frame(&self) -> Result<Option<FrameLease>, MediaError> {
        self.stream.take_frame()
    }
}
