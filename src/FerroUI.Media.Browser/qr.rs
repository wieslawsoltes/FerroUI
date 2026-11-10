use ferroui_browser::interop::completion_helper::await_promise;
use ferroui_media::frames::{PixelTransferPurpose, PresentationRoute};
use ferroui_media_qr::{GeometryQuality, QrAnalysis, QrCandidate, QrError};
use std::{future::Future, pin::Pin};

/// The gate owns the source lease until a bounded decoder worker finishes.
/// No pixels are read by ordinary camera/video preview.
pub fn recognize_qr(
    analysis: QrAnalysis,
) -> Pin<Box<dyn Future<Output = Result<Vec<QrCandidate>, QrError>>>> {
    Box::pin(async move {
        if !analysis.is_valid() {
            return Err(QrError::Cancelled);
        }
        let m = analysis.frame.metadata();
        let options = analysis.options();
        if m.width > options.max_width || m.height > options.max_height {
            return Err(QrError::InputTooLarge);
        }
        let (id, transferred) =
            if let Some(frame) = analysis.frame.resource::<super::native::TransferredFrame>() {
                (frame.id(), true)
            } else if let Some(frame) = analysis
                .frame
                .resource::<ferroui_opengl::SharedFrameTexture>()
            {
                if !super::native::current(frame.context_id() as i32, frame.generation()) {
                    return Err(QrError::StaleFrame);
                }
                (frame.texture(), false)
            } else {
                return Err(QrError::InvalidPlane);
            };
        analysis
            .counters()
            .cpu_transfer(
                PixelTransferPurpose::Analysis,
                u64::from(m.width) * u64::from(m.height) * 4,
                PresentationRoute::SharedTexture,
            )
            .map_err(|_| QrError::InvalidPlane)?;
        let promise = super::native::recognize(id, transferred, m.width, m.height)
            .map_err(|_| QrError::DecodeFailure)?;
        let result = await_promise(&promise).await.map_err(|e| {
            if e.name() == "InvalidStateError" {
                QrError::Busy
            } else {
                QrError::DecodeFailure
            }
        })?;
        if !analysis.is_valid() {
            return Err(QrError::Cancelled);
        }
        if analysis.frame.acquisition_age() > options.max_age {
            return Err(QrError::StaleFrame);
        }
        let json: serde_json::Value =
            serde_json::from_str(&result.as_string().ok_or(QrError::DecodeFailure)?)
                .map_err(|_| QrError::DecodeFailure)?;
        if let Some(copies) = json["inputCopies"].as_array() {
            if copies.len() > 6 {
                return Err(QrError::DecodeFailure);
            }
            for copy in copies {
                let bytes = copy
                    .as_u64()
                    .filter(|n| *n <= u64::from(m.width) * u64::from(m.height) * 4)
                    .ok_or(QrError::DecodeFailure)?;
                analysis
                    .counters()
                    .cpu_transfer(
                        PixelTransferPurpose::Analysis,
                        bytes,
                        PresentationRoute::SharedTexture,
                    )
                    .map_err(|_| QrError::InvalidPlane)?;
            }
        }
        json["results"]
            .as_array()
            .ok_or(QrError::DecodeFailure)?
            .iter()
            .map(|value| {
                let points = value["corners"]
                    .as_array()
                    .filter(|p| p.len() == 4)
                    .ok_or(QrError::DecodeFailure)?;
                let mut corners = [(0., 0.); 4];
                for (point, output) in points.iter().zip(&mut corners) {
                    *output = (
                        point[0]
                            .as_f64()
                            .filter(|v| v.is_finite())
                            .ok_or(QrError::DecodeFailure)?,
                        point[1]
                            .as_f64()
                            .filter(|v| v.is_finite())
                            .ok_or(QrError::DecodeFailure)?,
                    );
                }
                let payload = value["payload"]
                    .as_array()
                    .ok_or(QrError::DecodeFailure)?
                    .iter()
                    .map(|b| {
                        b.as_u64()
                            .and_then(|n| u8::try_from(n).ok())
                            .ok_or(QrError::DecodeFailure)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(QrCandidate {
                    frame: m.identity,
                    timestamp: m.timestamp,
                    frame_geometry: m.into(),
                    text: value["text"].as_str().map(str::to_owned),
                    codewords: None,
                    byte_segments: Vec::new(),
                    payload_bytes: Some(payload),
                    corners,
                    geometry: GeometryQuality::ProjectedGrid,
                })
            })
            .collect()
    })
}
