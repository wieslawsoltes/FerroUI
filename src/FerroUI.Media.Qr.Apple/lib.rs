//! Optional native QR recognition, independent of camera acquisition and video.
#![cfg(target_vendor = "apple")]
#![deny(unsafe_code)]
#[allow(unsafe_code)]
mod native;
use ferroui_media_apple::{poll_native, PixelBuffer};
use ferroui_media_qr::*;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    task::Poll,
};
static BUSY: AtomicBool = AtomicBool::new(false);
struct Slot;
impl Drop for Slot {
    fn drop(&mut self) {
        BUSY.store(false, Ordering::Release);
    }
}
/// Runs only on an analysis worker, never on the UI/render thread. The token
/// retains source storage and the gate's slot until native recognition finishes.
pub fn recognize_on_worker(analysis: &QrAnalysis) -> Result<Vec<QrCandidate>, QrError> {
    if BUSY
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err(QrError::Busy);
    }
    let _slot = Slot;
    recognize_admitted(analysis)
}
fn recognize_admitted(analysis: &QrAnalysis) -> Result<Vec<QrCandidate>, QrError> {
    if !analysis.is_valid() {
        return Err(QrError::Cancelled);
    }
    let metadata = analysis.frame.metadata();
    let options = analysis.options();
    if metadata.width > options.max_width || metadata.height > options.max_height {
        return Err(QrError::InputTooLarge);
    }
    let buffer = analysis
        .frame
        .resource::<PixelBuffer>()
        .ok_or(QrError::InvalidPlane)?;
    let values = native::recognize(buffer)?;
    if !analysis.is_valid() {
        return Err(QrError::Cancelled);
    }
    if analysis.frame.acquisition_age() > options.max_age {
        return Err(QrError::StaleFrame);
    }
    values
        .into_iter()
        .map(|value| {
            let points = value["corners"]
                .as_array()
                .filter(|v| v.len() == 4)
                .ok_or(QrError::DecodeFailure)?;
            let mut corners = [(0., 0.); 4];
            for (point, output) in points.iter().zip(&mut corners) {
                let x = point[0]
                    .as_f64()
                    .filter(|v| v.is_finite())
                    .ok_or(QrError::DecodeFailure)?;
                let y = point[1]
                    .as_f64()
                    .filter(|v| v.is_finite())
                    .ok_or(QrError::DecodeFailure)?;
                *output = (x, y);
            }
            let codewords = value["codewords"]
                .as_array()
                .ok_or(QrError::DecodeFailure)?
                .iter()
                .map(|v| {
                    v.as_u64()
                        .and_then(|v| u8::try_from(v).ok())
                        .ok_or(QrError::DecodeFailure)
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(QrCandidate {
                frame: metadata.identity,
                timestamp: metadata.timestamp,
                text: value["text"].as_str().map(String::from),
                codewords: (!codewords.is_empty()).then_some(codewords),
                byte_segments: Vec::new(),
                payload_bytes: None,
                corners,
                geometry: GeometryQuality::NativeCorners,
            })
        })
        .collect()
}
/// One bounded native job. Cancellation stops publication immediately while
/// the worker keeps the frame/token alive until Vision has actually finished.
pub fn recognize(
    analysis: QrAnalysis,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Vec<QrCandidate>, QrError>>>> {
    if BUSY
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Box::pin(async { Err(QrError::Busy) });
    }
    let slot = Slot;
    let state = Arc::new(Mutex::new(None));
    let output = state.clone();
    let worker = std::thread::Builder::new()
        .name("FerroUI.Media.Qr.Vision".into())
        .spawn(move || {
            let _slot = slot;
            let result = recognize_admitted(&analysis);
            *output.lock().unwrap() = Some(result);
        });
    if worker.is_err() {
        return Box::pin(async { Err(QrError::DecodeFailure) });
    }
    poll_native(move || {
        state
            .lock()
            .unwrap()
            .take()
            .map(Poll::Ready)
            .unwrap_or(Poll::Pending)
    })
}
#[cfg(feature = "test-fixtures")]
#[allow(unsafe_code)]
pub mod fixtures;
#[cfg(test)]
mod tests;
