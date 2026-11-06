use super::*;
use ferroui_media::frames::*;
use rxing::Writer;
use std::{ffi::c_void, time::Duration};
extern "C" {
    fn ferroui_qr_vision_fixture(luma: *const u8, width: usize, height: usize) -> *mut c_void;
}
#[allow(unsafe_code)]
fn frame(pixels: &[u8], width: u32, height: u32, sequence: u64) -> FrameLease {
    assert_eq!(pixels.len(), width as usize * height as usize);
    let buffer = unsafe {
        PixelBuffer::from_owned(ferroui_qr_vision_fixture(
            pixels.as_ptr(),
            width as usize,
            height as usize,
        ))
    }
    .unwrap();
    let (_, _, stride, bytes, _) = buffer.info();
    let budget = FrameBudget::new(Default::default());
    budget
        .counters()
        .cpu_transfer(
            PixelTransferPurpose::Probe,
            (width as u64) * (height as u64) * 4,
            PresentationRoute::SharedTexture,
        )
        .unwrap();
    budget
        .lease(
            FrameMetadata {
                identity: FrameIdentity {
                    source: 9,
                    generation: 1,
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
        .unwrap()
}
#[test]
fn native_vision_decodes_multiple_symbols_and_unrotated_geometry_without_preview_copy() {
    let width = 512;
    let height = 256;
    let mut pixels = vec![255u8; width * height];
    for (index, text) in ["ferroui-native-qr-left", "ferroui-native-qr-right"]
        .into_iter()
        .enumerate()
    {
        let qr = rxing::qrcode::QRCodeWriter::default()
            .encode(text, &rxing::BarcodeFormat::QR_CODE, 200, 200)
            .unwrap();
        for y in 0..200 {
            for x in 0..200 {
                pixels[(y + 28) * width + x + 28 + index * 256] =
                    if qr.get(x as u32, y as u32) { 0 } else { 255 };
            }
        }
    }
    if let Ok(path) = std::env::var("FERROUI_QR_FIXTURE_PGM") {
        let mut pgm = format!("P5\n{} {}\n255\n", width, height).into_bytes();
        pgm.extend_from_slice(&pixels);
        std::fs::write(path, pgm).unwrap();
    }
    let gate = QrAnalysisGate::new(QrOptions {
        max_age: Duration::from_secs(30),
        ..Default::default()
    });
    gate.activate(9, 1);
    let analysis = gate
        .begin(
            frame(&pixels, width as u32, height as u32, 1),
            Duration::ZERO,
        )
        .unwrap();
    let mut found = recognize_on_worker(&analysis).unwrap();
    found.sort_by(|a, b| a.text.cmp(&b.text));
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].text.as_deref(), Some("ferroui-native-qr-left"));
    assert_eq!(found[1].text.as_deref(), Some("ferroui-native-qr-right"));
    for (index, candidate) in found.iter().enumerate() {
        assert_eq!(candidate.frame, analysis.frame.metadata().identity);
        assert!(candidate.payload_bytes.is_none());
        assert_eq!(candidate.geometry, GeometryQuality::NativeCorners);
        assert!(candidate.corners.iter().all(|(x, y)| x.is_finite()
            && y.is_finite()
            && (0.0..=1.0).contains(x)
            && (0.0..=1.0).contains(y)));
        assert!(candidate.corners[0].0 < candidate.corners[1].0);
        assert!(candidate.corners[0].1 < candidate.corners[3].1);
        let center = candidate.corners.iter().map(|v| v.0).sum::<f64>() / 4.;
        assert!((center - (0.25 + index as f64 * 0.5)).abs() < 0.04);
    }
    let counters = analysis.counters().snapshot();
    assert_eq!(counters.preview.bytes, 0);
    assert_eq!(counters.analysis.bytes, 0);
    drop(analysis);
    let empty = gate
        .begin(
            frame(&vec![255; 128 * 128], 128, 128, 2),
            Duration::from_secs(1),
        )
        .unwrap();
    assert!(recognize_on_worker(&empty).unwrap().is_empty());
    gate.stop();
    assert!(matches!(
        recognize_on_worker(&empty),
        Err(QrError::Cancelled)
    ));
}
