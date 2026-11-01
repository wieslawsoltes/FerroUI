//! Android Media3 native audio/timing with one GPU texture output stream.
#![cfg(target_os = "android")]
#![forbid(unsafe_code)]
use ferroui_android::{interop::java::*, FerroAndroidApplication};
#[cfg(feature = "controls")]
pub use ferroui_media_android::NativeGlTextureImporter;
use ferroui_media_android::{
    native_error, poll_native, text, AndroidFrameStream, AndroidSurfaceSource,
    NativeGlFrameProducer,
};
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
            "Unsupported Android media URL",
        )),
        MediaSource::File(path) => path
            .to_str()
            .map(|v| (v.to_owned(), None))
            .ok_or_else(|| native_error("Media path is not UTF-8")),
        MediaSource::Resource { encoded, .. } => {
            use std::io::Write;
            use std::os::unix::fs::OpenOptionsExt;
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
            let context = FerroAndroidApplication::context();
            let directory = call_object(&context, "getCacheDir", "()Ljava/io/File;", &[])
                .ok_or_else(|| native_error("No private resource cache"))?;
            let directory = call_object(&directory, "getAbsolutePath", "()Ljava/lang/String;", &[])
                .map(|v| string_of(&v))
                .ok_or_else(|| native_error("No cache path"))?;
            let path = PathBuf::from(directory).join(format!(
                "ferroui-media-{}-{}.mp4",
                std::process::id(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            let mut file = std::fs::OpenOptions::new()
                .mode(0o600)
                .create_new(true)
                .write(true)
                .open(&path)
                .map_err(|e| native_error(e.to_string()))?;
            let owner = ResourceFile(path);
            file.write_all(&encoded)
                .map_err(|e| native_error(e.to_string()))?;
            Ok((owner.0.to_string_lossy().into_owned(), Some(owner)))
        }
    }
}
pub struct AndroidVideoProvider {
    pub frames: NativeGlFrameProducer,
}
pub fn native_diagnostics() -> String {
    call_static_object(
        &JavaClass::find("org/ferroui/media/video/VideoPlayer"),
        "diagnostics",
        "()Ljava/lang/String;",
        &[],
    )
    .map(|v| string_of(&v))
    .unwrap_or_default()
}
pub fn live_native_players() -> u32 {
    call_static_int(
        &JavaClass::find("org/ferroui/media/video/VideoPlayer"),
        "livePlayers",
        "()I",
        &[],
    )
    .max(0) as u32
}
impl MediaPlayerProvider for AndroidVideoProvider {
    fn open(
        &self,
        source: MediaSource,
        id: u64,
        generation: u64,
        _: PresentationPolicy,
    ) -> MediaFuture<Rc<dyn NativeMediaSession>> {
        let producer = self.frames.clone();
        let initial = resolve(source);
        // Resolve custom classes on the application class loader, before GL jobs.
        let class = JavaClass::find("org/ferroui/media/video/VideoPlayer");
        Box::pin(async move {
            let (source, resource) = initial?;
            let surface = AndroidSurfaceSource::create(&producer).await?;
            let java = new_object(
                &class,
                "(Landroid/content/Context;Lorg/ferroui/media/SurfaceFrames;Ljava/lang/String;)V",
                &[
                    JavaValue::Object(Some(&FerroAndroidApplication::context())),
                    JavaValue::Object(Some(&surface.java())),
                    JavaValue::String(&source),
                ],
            )
            .to_global();
            let mut session = Some(AndroidSession {
                stream: AndroidFrameStream::new(producer, surface, id, generation)
                    .with_frame_clock(java.clone()),
                java,
                _resource: resource,
                seek: Cell::new(0),
            });
            poll_native(move || {
                let value = session.as_ref().unwrap();
                if let Err(error) = value.check() {
                    return Poll::Ready(Err(error));
                }
                if call_int(&value.java, "state", "()I", &[]) == 0 {
                    return Poll::Pending;
                }
                Poll::Ready(Ok(
                    Rc::new(session.take().unwrap()) as Rc<dyn NativeMediaSession>
                ))
            })
            .await
        })
    }
}
struct AndroidSession {
    java: JavaObject,
    stream: AndroidFrameStream,
    _resource: Option<ResourceFile>,
    seek: Cell<i64>,
}
impl AndroidSession {
    fn check(&self) -> Result<(), MediaError> {
        let error = text(&self.java, "error");
        if error.is_empty() {
            Ok(())
        } else {
            Err(if call_boolean(&self.java, "unsupported", "()Z", &[]) {
                MediaError::new(MediaErrorKind::Unsupported, error)
            } else {
                native_error(error)
            })
        }
    }
}
impl Drop for AndroidSession {
    fn drop(&mut self) {
        self.stream.stop();
        call_void(&self.java, "close", "()V", &[]);
    }
}
impl NativeMediaSession for AndroidSession {
    fn has_pending_frame_work(&self) -> bool {
        self.stream.has_pending_frame_work()
    }
    fn state(&self) -> Result<MediaPlaybackSession, MediaError> {
        self.check()?;
        let state = match call_int(&self.java, "state", "()I", &[]) {
            0 => MediaPlaybackState::Opening,
            1 => MediaPlaybackState::Paused,
            2 => MediaPlaybackState::Playing,
            3 => MediaPlaybackState::Buffering,
            4 => MediaPlaybackState::Ended,
            _ => MediaPlaybackState::Failed,
        };
        let duration = call_long(&self.java, "duration", "()J", &[]);
        let width = call_int(&self.java, "naturalWidth", "()I", &[]).max(0) as u32;
        let height = call_int(&self.java, "naturalHeight", "()I", &[]).max(0) as u32;
        Ok(MediaPlaybackSession {
            state,
            position: Duration::from_millis(
                call_long(&self.java, "position", "()J", &[]).max(0) as u64
            ),
            duration: (duration >= 0).then(|| Duration::from_millis(duration as u64)),
            natural_size: (width > 0 && height > 0).then_some((width, height)),
            seekable: call_boolean(&self.java, "seekable", "()Z", &[]),
            decode: Acceleration::Unknown,
            error: None,
        })
    }
    fn play(&self) -> MediaFuture<()> {
        call_void(&self.java, "play", "()V", &[]);
        let result = self.check();
        Box::pin(async move { result })
    }
    fn pause(&self) -> Result<(), MediaError> {
        call_void(&self.java, "pause", "()V", &[]);
        self.check()
    }
    fn stop(&self) -> Result<(), MediaError> {
        self.stream.clear();
        self.pause()
    }
    fn seek(&self, position: Duration) -> MediaFuture<Duration> {
        self.stream.clear();
        let request = call_long(
            &self.java,
            "seek",
            "(J)J",
            &[JavaValue::Long(
                position.as_millis().min(i64::MAX as u128) as i64
            )],
        );
        self.seek.set(request);
        let java = self.java.clone();
        poll_native(move || {
            let error = text(&java, "error");
            if !error.is_empty() {
                return Poll::Ready(Err(native_error(error)));
            }
            if request == 0 {
                return Poll::Ready(Err(native_error("Native source is not prepared")));
            }
            if call_long(&java, "completedSeek", "()J", &[]) < request {
                return Poll::Pending;
            }
            Poll::Ready(Ok(Duration::from_millis(
                call_long(&java, "position", "()J", &[]).max(0) as u64,
            )))
        })
    }
    fn set_audio(&self, volume: f64, muted: bool) -> Result<(), MediaError> {
        call_void(
            &self.java,
            "audio",
            "(FZ)V",
            &[JavaValue::Float(volume as f32), JavaValue::Boolean(muted)],
        );
        self.check()
    }
    fn take_frame(&self) -> Result<Option<FrameLease>, MediaError> {
        self.check()?;
        if self.seek.get() > call_long(&self.java, "completedSeek", "()J", &[]) {
            return Ok(None);
        }
        self.stream.take_frame(
            call_int(&self.java, "width", "()I", &[]).max(0) as u32,
            call_int(&self.java, "height", "()I", &[]).max(0) as u32,
        )
    }
}
