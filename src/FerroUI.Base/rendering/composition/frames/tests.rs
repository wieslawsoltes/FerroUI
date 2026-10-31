use super::*;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
pub(super) fn metadata(sequence: u64) -> FrameMetadata {
    FrameMetadata {
        identity: FrameIdentity {
            source: 1,
            generation: 1,
            sequence,
        },
        width: 2,
        height: 2,
        visible: FrameRegion {
            x: 0,
            y: 0,
            width: 2,
            height: 2,
        },
        format: FrameFormat::Bgra8,
        color: FrameColor::Srgb,
        alpha: FrameAlpha::Opaque,
        rotation: FrameRotation::None,
        mirrored: false,
        top_left_origin: true,
        timestamp: Duration::ZERO,
        device: Some(DeviceIdentity::Context(1)),
        device_generation: 1,
        planes: vec![FramePlane {
            offset: 0,
            stride: 8,
            length: 16,
        }],
        retained_bytes: 16,
    }
}
struct Resource(Arc<AtomicUsize>);
impl Drop for Resource {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
#[test]
fn dropping_a_mailbox_invalidates_queued_views_without_releasing_gpu_storage() {
    let budget = FrameBudget::new(Default::default());
    let mailbox = FrameMailbox::new(1, 1);
    mailbox
        .publish(budget.lease(metadata(1), ()).unwrap())
        .unwrap();
    let queued = mailbox.latest().unwrap();
    let completion = GpuCompletion::new();
    completion.retain(queued.clone()).unwrap();
    drop(mailbox);
    assert!(!queued.is_current());
    drop(queued);
    assert_eq!(budget.usage().frames, 1);
    completion.complete();
    assert_eq!(budget.usage().frames, 0);
}
#[test]
fn each_gpu_reader_retires_independently_after_ui_and_source_disappear() {
    let budget = FrameBudget::new(FrameBudgetLimits::default());
    let drops = Arc::new(AtomicUsize::new(0));
    let frame = budget.lease(metadata(1), Resource(drops.clone())).unwrap();
    let first = GpuCompletion::new();
    let second = GpuCompletion::new();
    first.retain(frame.clone()).unwrap();
    second.retain(frame.clone()).unwrap();
    first.retain(frame.clone()).unwrap();
    assert_eq!(first.retained_frames(), 1);
    drop(frame);
    first.complete();
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert_eq!(budget.usage().frames, 1);
    second.complete();
    assert_eq!(drops.load(Ordering::SeqCst), 1);
    assert_eq!(budget.usage(), FrameBudgetUsage::default());
}
#[test]
fn abandoned_completion_is_quarantined_and_budget_remains_charged() {
    let budget = FrameBudget::new(FrameBudgetLimits {
        frames: 1,
        ..FrameBudgetLimits::default()
    });
    let frame = budget.lease(metadata(1), ()).unwrap();
    let completion = GpuCompletion::new();
    let backend_callback = completion.clone();
    completion.retain(frame).unwrap();
    completion.mark_indeterminate();
    drop(completion);
    assert!(matches!(
        budget.lease(metadata(2), ()),
        Err(FrameError::BudgetExceeded)
    ));
    backend_callback.complete();
    assert!(budget.lease(metadata(3), ()).is_ok());
}
#[test]
fn replacement_keeps_retiring_generations_in_the_same_budget() {
    let budget = FrameBudget::new(FrameBudgetLimits {
        frames: 1,
        ..FrameBudgetLimits::default()
    });
    let mailbox = FrameMailbox::new(1, 1);
    let frame = budget.lease(metadata(1), ()).unwrap();
    let done = GpuCompletion::new();
    done.retain(frame.clone()).unwrap();
    mailbox.publish(frame).unwrap();
    mailbox.replace_generation(2).unwrap();
    assert!(mailbox.latest().is_none());
    assert!(matches!(
        budget.lease(metadata(2), ()),
        Err(FrameError::BudgetExceeded)
    ));
    done.complete();
    let stale = budget.lease(metadata(3), ()).unwrap();
    assert_eq!(mailbox.publish(stale), Err(FrameError::StaleGeneration));
}
#[test]
fn malformed_planes_and_overflow_are_rejected_before_native_access() {
    let mut m = metadata(1);
    m.planes[0].offset = u64::MAX;
    assert_eq!(m.validate(8192), Err(FrameError::InvalidPlanes));
    m = metadata(1);
    m.planes[0].stride = u64::MAX;
    assert_eq!(m.validate(8192), Err(FrameError::InvalidPlanes));
    m = metadata(1);
    m.visible.x = u32::MAX;
    assert_eq!(m.validate(8192), Err(FrameError::InvalidRegion));
}
#[test]
fn admission_prefers_shared_gpu_and_strict_policy_never_falls_back() {
    let mut capabilities = FrameImportCapabilities {
        device: DeviceIdentity::Context(1),
        generation: 1,
        graphics: Acceleration::Hardware,
        formats: vec![FrameFormat::Bgra8],
        direct_sampling: true,
        gpu_conversion: true,
        synchronized: true,
    };
    assert_eq!(
        capabilities
            .select(&metadata(1), PresentationPolicy::GpuPreferred, true)
            .unwrap()
            .route,
        PresentationRoute::SharedTexture
    );
    capabilities.graphics = Acceleration::Software;
    assert_eq!(
        capabilities.select(&metadata(1), PresentationPolicy::GpuOnly, true),
        Err(FrameError::SoftwareGraphics)
    );
    let fallback = capabilities
        .select(&metadata(1), PresentationPolicy::GpuPreferred, true)
        .unwrap();
    assert_eq!(fallback.route, PresentationRoute::CpuFallback);
    assert_eq!(fallback.fallback_reason, Some(FrameError::SoftwareGraphics));
}
#[test]
fn counters_trap_preview_transfers_without_hiding_diagnostics_or_analysis() {
    let counters = FrameTransferCounters::default();
    assert_eq!(
        counters.cpu_transfer(
            PixelTransferPurpose::Preview,
            16,
            PresentationRoute::SharedTexture
        ),
        Err(FrameError::CpuTransferForbidden)
    );
    counters
        .cpu_transfer(
            PixelTransferPurpose::Diagnostic,
            16,
            PresentationRoute::SharedTexture,
        )
        .unwrap();
    counters
        .cpu_transfer(
            PixelTransferPurpose::Analysis,
            4,
            PresentationRoute::SharedTexture,
        )
        .unwrap();
    let snapshot = counters.snapshot();
    assert_eq!(
        snapshot.preview,
        PixelTransferCount {
            attempts: 1,
            bytes: 0
        }
    );
    assert_eq!(snapshot.diagnostic.bytes, 16);
    assert_eq!(snapshot.analysis.bytes, 4);
}
#[test]
fn latest_frame_coalesces_and_old_sequence_does_not_replace_it() {
    let budget = FrameBudget::new(FrameBudgetLimits::default());
    let mailbox = FrameMailbox::new(1, 1);
    mailbox
        .publish(budget.lease(metadata(3), ()).unwrap())
        .unwrap();
    assert_eq!(
        mailbox.publish(budget.lease(metadata(2), ()).unwrap()),
        Err(FrameError::StaleGeneration)
    );
    assert_eq!(mailbox.latest().unwrap().metadata().identity.sequence, 3);
}
#[test]
fn rotation_and_mirror_apply_once() {
    let mut m = metadata(1);
    m.rotation = FrameRotation::Clockwise90;
    m.mirrored = true;
    assert_eq!(m.orient(0.25, 0.75), Some((0.75, 0.25)));
    assert_eq!(m.orient(f64::NAN, 0.), None);
}

#[test]
fn indeterminate_submission_quarantines_late_output_until_actual_completion() {
    let budget = FrameBudget::new(Default::default());
    let completion = GpuCompletion::new();
    completion.mark_indeterminate();
    completion
        .retain(budget.lease(metadata(1), ()).unwrap())
        .unwrap();
    assert_eq!(budget.usage().frames, 1);
    completion.complete();
    assert_eq!(budget.usage().frames, 0);
}
#[test]
fn replacing_source_does_not_change_the_graphics_generation() {
    let mut frame = metadata(1);
    frame.identity.generation = 42;
    let caps = FrameImportCapabilities {
        device: DeviceIdentity::Context(1),
        generation: 1,
        graphics: Acceleration::Hardware,
        formats: vec![FrameFormat::Bgra8],
        direct_sampling: true,
        gpu_conversion: false,
        synchronized: true,
    };
    assert!(caps
        .select(&frame, PresentationPolicy::GpuOnly, false)
        .is_ok());
    frame.device_generation = 2;
    assert_eq!(
        caps.select(&frame, PresentationPolicy::GpuOnly, false),
        Err(FrameError::StaleGeneration)
    );
}

#[test]
fn clearing_source_invalidates_queued_views_without_releasing_pending_gpu_reads() {
    let budget = FrameBudget::new(Default::default());
    let source = FrameMailbox::new(1, 1);
    source
        .publish(budget.lease(metadata(1), ()).unwrap())
        .unwrap();
    let queued = source.latest().unwrap();
    assert!(queued.is_current());
    let submitted = GpuCompletion::new();
    submitted.retain(queued.clone()).unwrap();
    source.clear();
    assert!(!queued.is_current());
    assert_eq!(budget.usage().frames, 1);
    drop(queued);
    assert_eq!(budget.usage().frames, 1);
    submitted.complete();
    assert_eq!(budget.usage().frames, 0);
}

#[test]
fn repeated_gpu_reads_of_one_frame_are_bounded_and_quarantine_is_inspectable() {
    let budget = FrameBudget::new(FrameBudgetLimits {
        gpu_reads: 1,
        ..Default::default()
    });
    let frame = budget.lease(metadata(1), ()).unwrap();
    let first = GpuCompletion::new();
    let second = GpuCompletion::new();
    first.retain(frame.clone()).unwrap();
    assert_eq!(budget.gpu_reads(), 1);
    assert_eq!(
        second.retain(frame.clone()),
        Err(FrameError::BudgetExceeded)
    );
    first.mark_indeterminate();
    assert_eq!(budget.quarantined_reads(), 1);
    first.complete();
    assert_eq!(budget.quarantined_reads(), 0);
    assert_eq!(budget.gpu_reads(), 0);
    second.retain(frame).unwrap();
    second.complete();
}
