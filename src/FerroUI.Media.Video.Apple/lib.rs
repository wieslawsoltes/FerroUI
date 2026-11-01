//! AVFoundation playback, native timing/audio and retained CoreVideo storage.
#![cfg(target_vendor = "apple")]
#![deny(unsafe_code)]
mod geometry;
#[allow(unsafe_code)]
mod native;
pub use ferroui_media_apple::frame_surface;
use ferroui_media_apple::poll_native as wait;
#[cfg(feature = "controls")]
pub use ferroui_media_apple::AppleTextureImporter;
use ferroui_media_video::{frames::*, *};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    task::Poll,
    time::Duration,
};
/// Configure with the admitted compositor device identity, not a new private
/// device. Decode acceleration remains Unknown unless independently profiled.
pub struct AppleVideoProvider {
    pub device: DeviceIdentity,
    pub device_generation: u64,
    pub budget: FrameBudget,
}
impl MediaPlayerProvider for AppleVideoProvider {
    fn open(
        &self,
        source: MediaSource,
        id: u64,
        generation: u64,
        _: PresentationPolicy,
    ) -> MediaFuture<Rc<dyn NativeMediaSession>> {
        let device = self.device.clone();
        let budget = self.budget.clone();
        let device_generation = self.device_generation;
        Box::pin(async move {
            let PreparedMediaSource {
                location,
                temporary,
            } = source.prepare_native(None).await?;
            let url = match location {
                NativeMediaLocation::Url(url) => url,
                NativeMediaLocation::File(path) => {
                    path.to_str().map(str::to_owned).ok_or_else(|| {
                        MediaError::new(MediaErrorKind::InvalidSource, "Media path is not UTF-8")
                    })?
                }
            };
            // AVPlayer stays on the UI thread after filesystem work completes.
            let mut session = Some(AppleSession {
                player: native::Player::new(&url)?,
                _resource: temporary,
                device,
                budget,
                id,
                generation,
                device_generation,
                sequence: Cell::new(0),
                retrying: Cell::new(false),
                pending: RefCell::new(None),
            });
            wait(move || {
                let native = session.as_ref().unwrap();
                match native.player.state().state {
                    0 => Poll::Pending,
                    5 => Poll::Ready(Err(native.player.error())),
                    _ => Poll::Ready(Ok(
                        Rc::new(session.take().unwrap()) as Rc<dyn NativeMediaSession>
                    )),
                }
            })
            .await
        })
    }
}
struct AppleSession {
    player: native::Player,
    _resource: Option<TemporaryMediaFile>,
    device: DeviceIdentity,
    budget: FrameBudget,
    id: u64,
    generation: u64,
    device_generation: u64,
    sequence: Cell<u64>,
    retrying: Cell<bool>,
    pending: RefCell<Option<(ferroui_media_apple::PixelBuffer, f64)>>,
}
fn duration(seconds: f64) -> Option<Duration> {
    if seconds.is_finite() && seconds >= 0. {
        Duration::try_from_secs_f64(seconds).ok()
    } else {
        None
    }
}
impl NativeMediaSession for AppleSession {
    fn has_pending_frame_work(&self) -> bool {
        self.retrying.get()
    }
    fn state(&self) -> Result<MediaPlaybackSession, MediaError> {
        if self.budget.quarantined_reads() > 0 {
            return Err(MediaError::new(
                MediaErrorKind::NativeFailure,
                "GPU completion could not be established; native resources remain quarantined",
            ));
        }
        let state = self.player.state();
        if state.state == 5 {
            return Err(self.player.error());
        }
        Ok(MediaPlaybackSession {
            state: match state.state {
                0 => MediaPlaybackState::Opening,
                1 => MediaPlaybackState::Paused,
                2 => MediaPlaybackState::Playing,
                3 => MediaPlaybackState::Buffering,
                4 => MediaPlaybackState::Ended,
                _ => MediaPlaybackState::Failed,
            },
            position: duration(state.position).unwrap_or_default(),
            duration: duration(state.duration),
            natural_size: if state.width > 0 && state.height > 0 {
                Some((state.width, state.height))
            } else {
                None
            },
            seekable: state.seekable != 0,
            decode: Acceleration::Unknown,
            error: None,
        })
    }
    fn play(&self) -> MediaFuture<()> {
        self.player.play();
        Box::pin(async { Ok(()) })
    }
    fn pause(&self) -> Result<(), MediaError> {
        self.player.pause();
        Ok(())
    }
    fn seek(&self, position: Duration) -> MediaFuture<Duration> {
        self.pending.borrow_mut().take();
        self.retrying.set(false);
        match self.player.seek(position.as_secs_f64()) {
            Err(error) => Box::pin(async move { Err(error) }),
            Ok(request) => wait(move || match request.poll() {
                None => Poll::Pending,
                Some(Err(error)) => Poll::Ready(Err(error)),
                Some(Ok(seconds)) => Poll::Ready(duration(seconds).ok_or_else(|| {
                    MediaError::new(
                        MediaErrorKind::NativeFailure,
                        "Invalid native seek position",
                    )
                })),
            }),
        }
    }
    fn set_audio(&self, volume: f64, muted: bool) -> Result<(), MediaError> {
        self.player.audio(volume, muted);
        Ok(())
    }
    fn take_frame(&self) -> Result<Option<FrameLease>, MediaError> {
        self.retrying.set(false);
        if self.budget.limits().frames == 0 {
            return Err(MediaError::new(
                MediaErrorKind::Unsupported,
                "Frame budget admits no frames",
            ));
        }
        if self.budget.usage().frames >= self.budget.limits().frames {
            self.retrying.set(true);
            return Ok(None);
        }
        let Some((buffer, timestamp)) = self
            .pending
            .borrow_mut()
            .take()
            .or_else(|| self.player.frame())
        else {
            return Ok(None);
        };
        let (width, height, stride, bytes, color) = buffer.info();
        let (visible, rotation, mirrored) =
            self.player.geometry(&buffer)?.decompose(width, height)?;
        if buffer.surface().is_none() {
            return Err(MediaError::new(
                MediaErrorKind::Unsupported,
                "Decoder frame has no shareable IOSurface",
            ));
        }
        let sequence = self
            .sequence
            .get()
            .checked_add(1)
            .expect("frame sequence exhausted");
        self.sequence.set(sequence);
        let metadata = FrameMetadata {
            identity: FrameIdentity {
                source: self.id,
                generation: self.generation,
                sequence,
            },
            width,
            height,
            visible,
            format: FrameFormat::Bgra8,
            color: match color {
                1 => FrameColor::Srgb,
                2 => FrameColor::Rec709Full,
                _ => FrameColor::Unsupported,
            },
            alpha: FrameAlpha::Opaque,
            rotation,
            mirrored,
            top_left_origin: true,
            timestamp: duration(timestamp).ok_or_else(|| {
                MediaError::new(
                    MediaErrorKind::NativeFailure,
                    "Native decoded-frame timestamp is unavailable",
                )
            })?,
            device: Some(self.device.clone()),
            device_generation: self.device_generation,
            planes: vec![FramePlane {
                offset: 0,
                stride,
                length: stride.checked_mul(height as u64).ok_or_else(|| {
                    MediaError::new(MediaErrorKind::Unsupported, "Native frame size overflow")
                })?,
            }],
            retained_bytes: bytes,
        };
        if bytes > self.budget.limits().bytes {
            return Err(MediaError::new(
                MediaErrorKind::Unsupported,
                "Native frame exceeds the retained byte budget",
            ));
        }
        match self.budget.reserve(metadata) {
            Ok(reservation) => Ok(Some(reservation.lease(buffer))),
            Err(FrameError::BudgetExceeded) => {
                self.pending.replace(Some((buffer, timestamp)));
                self.retrying.set(true);
                Ok(None)
            }
            Err(error) => Err(MediaError::new(
                MediaErrorKind::Unsupported,
                error.to_string(),
            )),
        }
    }
}
