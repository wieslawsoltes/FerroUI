//! Optional bundled native QR recognition. Call attach/take/drop on the UI
//! thread. An analyzer does not own camera hardware or keep a session running.
#![cfg(target_os = "android")]
#![forbid(unsafe_code)]
use base64::Engine;
use ferroui_android::interop::java::*;
use ferroui_media::frames::FrameIdentity;
use ferroui_media_camera::CameraSession;
use ferroui_media_camera_android::AndroidCameraContext;
use ferroui_media_qr::{GeometryQuality, QrCandidate, QrError, QrOptions};
use std::{rc::Rc, time::Duration};
pub struct AndroidQrAnalyzer {
    camera: AndroidCameraContext,
    java: JavaObject,
    // The native controller remains authoritative for session closure. Rc
    // marker enforces the UI-thread attachment / detachment contract.
    _ui: std::marker::PhantomData<Rc<()>>,
}
impl AndroidQrAnalyzer {
    pub fn attach(session: &CameraSession, options: QrOptions) -> Result<Self, QrError> {
        if session.is_stopped()
            || options.max_width == 0
            || options.max_height == 0
            || options.max_width > 1280
            || options.max_height > 720
            || options.max_age.is_zero()
        {
            return Err(QrError::InvalidPlane);
        }
        let camera = session
            .platform_context::<AndroidCameraContext>()
            .ok_or(QrError::InvalidPlane)?
            .clone();
        let java = new_object(
            &JavaClass::find("org/ferroui/media/qr/QrAnalyzer"),
            "(IIJJ)V",
            &[
                JavaValue::Int(options.max_width as i32),
                JavaValue::Int(options.max_height as i32),
                JavaValue::Long(options.interval.as_millis().min(i64::MAX as u128) as i64),
                JavaValue::Long(options.max_age.as_millis().min(250) as i64),
            ],
        )
        .to_global();
        let message = call_object(
            &camera.java(),
            "setAnalyzer",
            "(Lorg/ferroui/media/camera/FrameAnalyzer;)Ljava/lang/String;",
            &[JavaValue::Object(Some(&java))],
        )
        .map(|v| string_of(&v))
        .unwrap_or_else(|| "Attachment failed".into());
        if !message.is_empty() {
            call_void(&java, "close", "()V", &[]);
            return Err(QrError::DecodeFailure);
        }
        Ok(Self {
            camera,
            java,
            _ui: std::marker::PhantomData,
        })
    }
    pub fn statistics(&self) -> (u64, u64) {
        let text = call_object(&self.java, "statistics", "()Ljava/lang/String;", &[])
            .map(|v| string_of(&v))
            .unwrap_or_default();
        let mut parts = text.split(',').filter_map(|v| v.parse().ok());
        (parts.next().unwrap_or(0), parts.next().unwrap_or(0))
    }
    /// Latest completed result; native work is limited to one task, at most
    /// 10Hz, and results expire after 250ms. No framework pixel copying occurs.
    pub fn take(&self) -> Result<Vec<QrCandidate>, QrError> {
        if call_boolean(&self.camera.java(), "closed", "()Z", &[]) {
            return Err(QrError::Cancelled);
        }
        let error = call_object(&self.java, "error", "()Ljava/lang/String;", &[])
            .map(|v| string_of(&v))
            .unwrap_or_default();
        if !error.is_empty() {
            return Err(QrError::DecodeFailure);
        }
        let text = call_object(&self.java, "take", "()Ljava/lang/String;", &[])
            .map(|v| string_of(&v))
            .unwrap_or_default();
        if text.is_empty() {
            return Ok(Vec::new());
        }
        if text.len() > 1024 * 1024 {
            return Err(QrError::InputTooLarge);
        }
        let value: serde_json::Value =
            serde_json::from_str(&text).map_err(|_| QrError::DecodeFailure)?;
        let timestamp = value["timestamp"].as_u64().ok_or(QrError::DecodeFailure)?;
        value["symbols"]
            .as_array()
            .ok_or(QrError::DecodeFailure)?
            .iter()
            .take(64)
            .map(|v| {
                let points = v["corners"]
                    .as_array()
                    .filter(|p| p.len() == 4)
                    .ok_or(QrError::DecodeFailure)?;
                let mut corners = [(0., 0.); 4];
                for (i, p) in points.iter().enumerate() {
                    corners[i] = (
                        p[0].as_f64()
                            .filter(|v| v.is_finite())
                            .ok_or(QrError::DecodeFailure)?,
                        p[1].as_f64()
                            .filter(|v| v.is_finite())
                            .ok_or(QrError::DecodeFailure)?,
                    );
                }
                Ok(QrCandidate {
                    frame: FrameIdentity {
                        source: self.camera.source(),
                        generation: self.camera.generation(),
                        sequence: timestamp,
                    },
                    timestamp: Duration::from_nanos(timestamp),
                    text: v["text"].as_str().map(str::to_owned),
                    codewords: None,
                    byte_segments: Vec::new(),
                    payload_bytes: v["bytes"]
                        .as_str()
                        .map(|b| {
                            base64::engine::general_purpose::STANDARD
                                .decode(b)
                                .map_err(|_| QrError::DecodeFailure)
                        })
                        .transpose()?,
                    corners,
                    geometry: GeometryQuality::NativeCorners,
                })
            })
            .collect()
    }
}
impl Drop for AndroidQrAnalyzer {
    fn drop(&mut self) {
        // Clear only our attachment; replacing an analyzer must not let the
        // old Rust handle detach its newer replacement.
        call_void(
            &self.camera.java(),
            "removeAnalyzer",
            "(Lorg/ferroui/media/camera/FrameAnalyzer;)V",
            &[JavaValue::Object(Some(&self.java))],
        );
        call_void(&self.java, "close", "()V", &[]);
    }
}
