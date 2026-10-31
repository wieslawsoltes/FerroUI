//! AVFoundation playback, native timing/audio and retained CoreVideo storage.
#![cfg(target_vendor = "apple")]
#![deny(unsafe_code)]
#[allow(unsafe_code)]
mod native;
pub use ferroui_media_apple::frame_surface;
use ferroui_media_apple::poll_native as wait;
#[cfg(feature = "controls")]
pub use ferroui_media_apple::AppleTextureImporter;
use ferroui_media_video::{frames::*, *};
use std::{cell::Cell, path::PathBuf, rc::Rc, task::Poll, time::Duration};
struct ResourceFile(PathBuf);
impl Drop for ResourceFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
fn resolve(source: MediaSource) -> Result<(String, Option<ResourceFile>), MediaError> {
    match source {
        MediaSource::Url(url)
            if url.starts_with("https://")
                || url.starts_with("http://")
                || url.starts_with("file://") =>
        {
            Ok((url, None))
        }
        MediaSource::Url(_) => Err(MediaError::new(
            MediaErrorKind::InvalidSource,
            "Unsupported media URL scheme",
        )),
        MediaSource::File(path) => path.to_str().map(|s| (s.to_owned(), None)).ok_or_else(|| {
            MediaError::new(MediaErrorKind::InvalidSource, "Media path is not UTF-8")
        }),
        MediaSource::Resource {
            encoded,
            content_type,
            ..
        } => {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
            let suffix = if content_type == "video/mp4" {
                "mp4"
            } else {
                "media"
            };
            let path = std::env::temp_dir().join(format!(
                "ferroui-media-{}-{}.{suffix}",
                std::process::id(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            let mut file = std::fs::OpenOptions::new()
                .mode(0o600)
                .create_new(true)
                .write(true)
                .open(&path)
                .map_err(|_| {
                    MediaError::new(
                        MediaErrorKind::NativeFailure,
                        "Cannot create media resource file",
                    )
                })?;
            let owner = ResourceFile(path);
            file.write_all(&encoded).map_err(|_| {
                MediaError::new(
                    MediaErrorKind::NativeFailure,
                    "Cannot write media resource file",
                )
            })?;
            let text = owner
                .0
                .to_str()
                .ok_or_else(|| {
                    MediaError::new(MediaErrorKind::InvalidSource, "Resource path is not UTF-8")
                })?
                .to_owned();
            Ok((text, Some(owner)))
        }
    }
}
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
        let initial = resolve(source).and_then(|(url, resource)| {
            native::Player::new(&url).map(|player| AppleSession {
                player,
                _resource: resource,
                device,
                budget,
                id,
                generation,
                device_generation,
                sequence: Cell::new(0),
            })
        });
        let mut initial = Some(initial);
        let mut session = None;
        wait(move || {
            if let Some(result) = initial.take() {
                match result {
                    Ok(value) => session = Some(value),
                    Err(error) => return Poll::Ready(Err(error)),
                }
            }
            let native = session.as_ref().expect("pending native owner");
            match native.player.state().state {
                0 => Poll::Pending,
                5 => Poll::Ready(Err(native.player.error())),
                _ => Poll::Ready(Ok(
                    Rc::new(session.take().unwrap()) as Rc<dyn NativeMediaSession>
                )),
            }
        })
    }
}
struct AppleSession {
    player: native::Player,
    _resource: Option<ResourceFile>,
    device: DeviceIdentity,
    budget: FrameBudget,
    id: u64,
    generation: u64,
    device_generation: u64,
    sequence: Cell<u64>,
}
fn duration(seconds: f64) -> Option<Duration> {
    if seconds.is_finite() && seconds >= 0. {
        Duration::try_from_secs_f64(seconds).ok()
    } else {
        None
    }
}
impl NativeMediaSession for AppleSession {
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
        if self.budget.usage().frames >= self.budget.limits().frames {
            return Ok(None);
        }
        let Some(buffer) = self.player.frame() else {
            return Ok(None);
        };
        let (width, height, stride, bytes, color) = buffer.info();
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
            visible: FrameRegion {
                x: 0,
                y: 0,
                width,
                height,
            },
            format: FrameFormat::Bgra8,
            color: match color {
                1 => FrameColor::Srgb,
                2 => FrameColor::Rec709Full,
                _ => FrameColor::Unsupported,
            },
            alpha: FrameAlpha::Opaque,
            rotation: FrameRotation::None,
            mirrored: false,
            top_left_origin: true,
            timestamp: self.state()?.position,
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
        match self.budget.lease(metadata, buffer) {
            Ok(frame) => Ok(Some(frame)),
            Err(FrameError::BudgetExceeded) => Ok(None),
            Err(error) => Err(MediaError::new(
                MediaErrorKind::Unsupported,
                error.to_string(),
            )),
        }
    }
}
