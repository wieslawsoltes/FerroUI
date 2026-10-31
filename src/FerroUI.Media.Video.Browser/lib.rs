//! Native HTML video/audio with direct active-context WebGL presentation.
#![forbid(unsafe_code)]
use ferroui_browser::interop::completion_helper::{await_promise, PromiseError};
#[cfg(feature = "controls")]
pub use ferroui_media_browser::BrowserTextureImporter;
use ferroui_media_browser::{BrowserFrameStream, BrowserImageFrame, WebGlFrameProducer};
use ferroui_media_video::{frames::*, *};
use std::{rc::Rc, time::Duration};
use wasm_bindgen::prelude::*;
#[wasm_bindgen(module = "/video.js")]
extern "C" {
    type BrowserVideo;
    #[wasm_bindgen(catch,js_name=openUrl)]
    fn open_url(url: &str) -> Result<BrowserVideo, JsValue>;
    #[wasm_bindgen(catch,js_name=openResource)]
    fn open_resource(encoded: &[u8], content_type: &str) -> Result<BrowserVideo, JsValue>;
    #[wasm_bindgen(method)]
    fn prepare(this: &BrowserVideo) -> JsValue;
    #[wasm_bindgen(method, getter)]
    fn state(this: &BrowserVideo) -> i32;
    #[wasm_bindgen(method, getter)]
    fn position(this: &BrowserVideo) -> f64;
    #[wasm_bindgen(method, getter)]
    fn duration(this: &BrowserVideo) -> f64;
    #[wasm_bindgen(method,getter,js_name=naturalWidth)]
    fn natural_width(this: &BrowserVideo) -> u32;
    #[wasm_bindgen(method,getter,js_name=naturalHeight)]
    fn natural_height(this: &BrowserVideo) -> u32;
    #[wasm_bindgen(method, getter)]
    fn seekable(this: &BrowserVideo) -> bool;
    #[wasm_bindgen(method, getter)]
    fn error(this: &BrowserVideo) -> String;
    #[wasm_bindgen(method)]
    fn play(this: &BrowserVideo) -> JsValue;
    #[wasm_bindgen(method, catch)]
    fn pause(this: &BrowserVideo) -> Result<(), JsValue>;
    #[wasm_bindgen(method, catch)]
    fn audio(this: &BrowserVideo, volume: f64, muted: bool) -> Result<(), JsValue>;
    #[wasm_bindgen(method, catch)]
    fn seek(this: &BrowserVideo, position: f64) -> Result<BrowserSeek, JsValue>;
    #[wasm_bindgen(method)]
    fn close(this: &BrowserVideo);
    #[wasm_bindgen(method, catch)]
    fn sample(this: &BrowserVideo) -> Result<Option<NativeImage>, JsValue>;
    type BrowserSeek;
    #[wasm_bindgen(method, getter)]
    fn promise(this: &BrowserSeek) -> JsValue;
    #[wasm_bindgen(method,js_name=close)]
    fn close_seek(this: &BrowserSeek);
    type NativeImage;
    #[wasm_bindgen(method, getter)]
    fn image(this: &NativeImage) -> JsValue;
    #[wasm_bindgen(method, getter)]
    fn sequence(this: &NativeImage) -> f64;
    #[wasm_bindgen(method, getter)]
    fn timestamp(this: &NativeImage) -> f64;
    #[wasm_bindgen(method, getter)]
    fn width(this: &NativeImage) -> u32;
    #[wasm_bindgen(method, getter)]
    fn height(this: &NativeImage) -> u32;
}
struct Video(BrowserVideo);
impl Drop for Video {
    fn drop(&mut self) {
        self.0.close();
    }
}
struct Seek(BrowserSeek);
impl Drop for Seek {
    fn drop(&mut self) {
        self.0.close_seek();
    }
}
pub struct BrowserVideoProvider {
    pub frames: WebGlFrameProducer,
}
impl MediaPlayerProvider for BrowserVideoProvider {
    fn open(
        &self,
        source: MediaSource,
        id: u64,
        generation: u64,
        _policy: PresentationPolicy,
    ) -> MediaFuture<Rc<dyn NativeMediaSession>> {
        let opened = match source {
            MediaSource::Url(url) => open_url(&url).map_err(js_error),
            MediaSource::Resource {
                encoded,
                content_type,
                ..
            } => resource(&encoded, &content_type),
            MediaSource::File(path) => std::fs::read(path)
                .map_err(|e| MediaError::new(MediaErrorKind::InvalidSource, e.to_string()))
                .and_then(|encoded| resource(&encoded, "video/mp4")),
        }
        .map(Video);
        let stream = self.frames.stream(id, generation);
        // The provisional owner exists before polling so an abandoned open,
        // including a never-polled future, still tears down native playback.
        Box::pin(async move {
            let video = opened?;
            await_promise(&video.0.prepare())
                .await
                .map_err(promise_error)?;
            Ok(Rc::new(Session { video, stream }) as Rc<dyn NativeMediaSession>)
        })
    }
}
fn resource(encoded: &[u8], content_type: &str) -> Result<BrowserVideo, MediaError> {
    if encoded.len() > 256 * 1024 * 1024 {
        return Err(MediaError::new(
            MediaErrorKind::InvalidSource,
            "Encoded browser media resource exceeds 256 MiB",
        ));
    }
    open_resource(encoded, content_type).map_err(js_error)
}
struct Session {
    video: Video,
    stream: BrowserFrameStream,
}
impl NativeMediaSession for Session {
    fn state(&self) -> Result<MediaPlaybackSession, MediaError> {
        let video = &self.video.0;
        let state = match video.state() {
            0 => MediaPlaybackState::Opening,
            1 => MediaPlaybackState::Paused,
            2 => MediaPlaybackState::Playing,
            3 => MediaPlaybackState::Buffering,
            4 => MediaPlaybackState::Ended,
            _ => {
                return Err(MediaError::new(
                    MediaErrorKind::NativeFailure,
                    video.error(),
                ))
            }
        };
        let width = video.natural_width();
        let height = video.natural_height();
        Ok(MediaPlaybackSession {
            state,
            position: seconds(video.position()).unwrap_or_default(),
            duration: seconds(video.duration()),
            natural_size: (width > 0 && height > 0).then_some((width, height)),
            seekable: video.seekable(),
            decode: Acceleration::Unknown,
            error: None,
        })
    }
    fn play(&self) -> MediaFuture<()> {
        let pending = await_promise(&self.video.0.play());
        Box::pin(async move {
            pending.await.map_err(promise_error)?;
            Ok(())
        })
    }
    fn pause(&self) -> Result<(), MediaError> {
        self.video.0.pause().map_err(js_error)
    }
    fn seek(&self, position: Duration) -> MediaFuture<Duration> {
        self.stream.clear();
        let request = self
            .video
            .0
            .seek(position.as_secs_f64())
            .map(Seek)
            .map_err(js_error);
        Box::pin(async move {
            let request = request?;
            let result = await_promise(&request.0.promise())
                .await
                .map_err(promise_error)?;
            seconds(result.as_f64().unwrap_or(f64::NAN)).ok_or_else(|| {
                MediaError::new(
                    MediaErrorKind::NativeFailure,
                    "Native seek returned an invalid timestamp",
                )
            })
        })
    }
    fn set_audio(&self, volume: f64, muted: bool) -> Result<(), MediaError> {
        self.video.0.audio(volume, muted).map_err(js_error)
    }
    fn stop(&self) -> Result<(), MediaError> {
        self.stream.clear();
        self.pause()
    }
    fn take_frame(&self) -> Result<Option<FrameLease>, MediaError> {
        self.stream.poll(|| {
            self.video
                .0
                .sample()
                .map_err(js_error)?
                .map(|sample| {
                    Ok(BrowserImageFrame {
                        image: sample.image(),
                        sequence: sample.sequence() as u64,
                        width: sample.width(),
                        height: sample.height(),
                        timestamp: seconds(sample.timestamp()).ok_or_else(|| {
                            MediaError::new(
                                MediaErrorKind::NativeFailure,
                                "Invalid decoded-frame timestamp",
                            )
                        })?,
                    })
                })
                .transpose()
        })
    }
}
fn seconds(value: f64) -> Option<Duration> {
    Duration::try_from_secs_f64(value).ok()
}
fn js_error(error: JsValue) -> MediaError {
    MediaError::new(
        MediaErrorKind::NativeFailure,
        format!("Browser playback: {error:?}"),
    )
}
fn promise_error(error: PromiseError) -> MediaError {
    MediaError::new(
        match error.name() {
            "NotAllowedError" => MediaErrorKind::AutoplayDenied,
            "SecurityError" => MediaErrorKind::Permission,
            "AbortError" => MediaErrorKind::Cancelled,
            _ => MediaErrorKind::NativeFailure,
        },
        error.to_string(),
    )
}
