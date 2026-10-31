use super::*;
use ferroui_media::frames::*;
#[cfg(feature = "rust-decoder")]
use rxing::Writer;
fn frame(width: u32, height: u32, sequence: u64) -> FrameLease {
    FrameBudget::new(FrameBudgetLimits::default())
        .lease(
            FrameMetadata {
                identity: FrameIdentity {
                    source: 1,
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
                    stride: width as u64 * 4,
                    length: width as u64 * height as u64 * 4,
                }],
                retained_bytes: width as u64 * height as u64 * 4,
            },
            (),
        )
        .unwrap()
}
#[test]
fn slow_analysis_never_takes_a_second_slot_even_after_generation_change() {
    let gate = QrAnalysisGate::new(QrOptions::default());
    gate.activate(1, 1);
    let active = gate.begin(frame(128, 128, 1), Duration::ZERO).unwrap();
    gate.activate(1, 1);
    assert!(matches!(
        gate.begin(frame(128, 128, 2), Duration::from_secs(1)),
        Err(QrError::Busy)
    ));
    assert!(!active.is_current(Duration::ZERO));
    drop(active);
    assert!(gate
        .begin(frame(128, 128, 3), Duration::from_secs(1))
        .is_ok());
}
#[test]
fn overlay_expires_and_acceptance_can_suppress_pending_publication() {
    let gate = QrAnalysisGate::new(QrOptions::default());
    gate.activate(1, 1);
    let active = gate.begin(frame(128, 128, 1), Duration::ZERO).unwrap();
    assert!(active.is_current(Duration::from_millis(249)));
    assert!(!active.is_current(Duration::from_millis(251)));
    gate.stop();
    assert!(!active.is_current(Duration::ZERO));
}

#[test]
fn analysis_expiry_uses_acquisition_age_and_the_scheduler_clock_not_media_position() {
    let gate = QrAnalysisGate::new(Default::default());
    gate.activate(1, 1);
    // Source PTS is zero while the host scheduler is already an hour old.
    let frame = frame(128, 128, 1);
    let now = Duration::from_secs(3600);
    let active = gate.begin(frame.clone(), now).unwrap();
    assert!(active.is_current(now + Duration::from_millis(100)));
    assert!(!active.is_current(now + Duration::from_millis(251)));
    drop(active);
    assert!(matches!(
        gate.begin(frame, now + Duration::from_millis(200)),
        Err(QrError::StaleFrame)
    ));
}
#[cfg(feature = "rust-decoder")]
#[test]
fn actual_qr_decode_borrows_packed_pixels_and_counts_strided_repacking() {
    let encoded = rxing::qrcode::QRCodeWriter::default()
        .encode(
            "ferroui-shared-texture",
            &rxing::BarcodeFormat::QR_CODE,
            128,
            128,
        )
        .unwrap();
    for padding in [0usize, 17] {
        let stride = 128 + padding;
        let mut pixels = vec![255u8; stride * 128];
        for y in 0..128 {
            for x in 0..128 {
                pixels[y * stride + x] = if encoded.get(x as u32, y as u32) {
                    0
                } else {
                    255
                };
            }
        }
        let gate = QrAnalysisGate::new(QrOptions::default());
        gate.activate(1, 1);
        let active = gate.begin(frame(128, 128, 1), Duration::ZERO).unwrap();
        let results = decode(
            &active,
            AnalysisPlane {
                data: &pixels,
                width: 128,
                height: 128,
                row_stride: stride,
                pixel_stride: 1,
            },
        )
        .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].text.as_deref(), Some("ferroui-shared-texture"));
        assert!(results[0]
            .corners
            .iter()
            .all(|(x, y)| (0.0..=1.0).contains(x) && (0.0..=1.0).contains(y)));
        let counts = active.counters().snapshot();
        assert_eq!(counts.preview.bytes, 0);
        if padding == 0 {
            assert_eq!(counts.analysis.bytes, 0);
        } else {
            assert!(counts.analysis.bytes >= 128 * 128);
        }
    }
}
#[test]
fn malformed_strides_and_oversize_are_rejected_before_decode() {
    let plane = AnalysisPlane {
        data: &[0; 16],
        width: 4,
        height: 4,
        row_stride: usize::MAX,
        pixel_stride: 1,
    };
    assert_eq!(
        plane.validate(QrOptions::default()),
        Err(QrError::InvalidPlane)
    );
    let plane = AnalysisPlane {
        data: &[],
        width: 1920,
        height: 1080,
        row_stride: 1920,
        pixel_stride: 1,
    };
    assert_eq!(
        plane.validate(QrOptions::default()),
        Err(QrError::InputTooLarge)
    );
}
