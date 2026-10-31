//! Explicit opt-in native-engine diagnostics, never a preview producer.
use ferroui_media::frames::*;
use ferroui_media_qr::*;
use std::{ffi::c_void, time::Duration};
extern "C" {
    fn ferroui_qr_vision_fixture(luma: *const u8, width: usize, height: usize) -> *mut c_void;
}
/// Makes bounded CPU analysis input for native release-gate tests. The upload
/// is counted as Probe; it is not admitted for GPU preview or camera output.
pub fn luma_frame(luma: &[u8], width: u32, height: u32) -> Result<FrameLease, QrError> {
    if width == 0
        || height == 0
        || width > 1280
        || height > 720
        || luma.len() != width as usize * height as usize
    {
        return Err(QrError::InvalidPlane);
    }
    let budget = FrameBudget::new(Default::default());
    budget
        .counters()
        .cpu_transfer(
            PixelTransferPurpose::Probe,
            width as u64 * height as u64 * 4,
            PresentationRoute::SharedTexture,
        )
        .map_err(|_| QrError::DecodeFailure)?;
    let buffer = unsafe {
        super::PixelBuffer::from_owned(ferroui_qr_vision_fixture(
            luma.as_ptr(),
            width as usize,
            height as usize,
        ))
    }
    .ok_or(QrError::DecodeFailure)?;
    let (_, _, stride, bytes, _) = buffer.info();
    budget
        .lease(
            FrameMetadata {
                identity: FrameIdentity {
                    source: 9,
                    generation: 1,
                    sequence: 1,
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
                color: FrameColor::Srgb,
                alpha: FrameAlpha::Opaque,
                rotation: FrameRotation::None,
                mirrored: false,
                top_left_origin: true,
                timestamp: Duration::ZERO,
                device: None,
                device_generation: 1,
                planes: vec![FramePlane {
                    offset: 0,
                    stride,
                    length: stride * height as u64,
                }],
                retained_bytes: bytes,
            },
            buffer,
        )
        .map_err(|_| QrError::DecodeFailure)
}
