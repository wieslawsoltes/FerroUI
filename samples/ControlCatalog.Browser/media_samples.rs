//! Browser media on the page or render worker, with opt-in QR analysis.
use control_catalog::media_samples::MediaSampleProviders;
use ferroui_media_browser::{BrowserTextureImporter, WebGlFrameProducer};
use std::rc::Rc;

pub fn install() {
    control_catalog::media_samples::install(MediaSampleProviders {
        video: Rc::new(|interop, budget| {
            Ok((
                Rc::new(ferroui_media_video_browser::BrowserVideoProvider {
                    frames: WebGlFrameProducer::from_interop(interop, budget)?,
                }),
                Rc::new(BrowserTextureImporter),
            ))
        }),
        camera: Rc::new(|interop, budget| {
            let frames = WebGlFrameProducer::from_interop(interop, budget).map_err(|e| {
                ferroui_media_camera::CameraError::new(
                    ferroui_media_camera::CameraErrorKind::Unsupported,
                    e.to_string(),
                )
            })?;
            Ok((
                Rc::new(ferroui_media_camera_browser::BrowserCameraProvider::new(
                    frames,
                )),
                Rc::new(BrowserTextureImporter),
            ))
        }),
        qr: Some(Rc::new(ferroui_media_browser::recognize_qr)),
    });
}
