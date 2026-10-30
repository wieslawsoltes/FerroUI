// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Actual source-context state and reference transfer boundaries.
use super::*;
use ferroui_base::threading::CancellationToken;
use ferroui_controls::{AppBuilder, Application, Border};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "dock-drag-context",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(f: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(f, CancellationToken::none())
        .wait()
        .unwrap();
}
fn assert_cleared(context: &DockDragContext) {
    assert!(context.drag_control().is_none());
    assert_eq!(context.drag_start_point.get(), Point::default());
    assert!(!context.pointer_pressed.get());
    assert!(!context.do_drag_drop.get());
    assert_eq!(context.target_point.get(), Point::default());
    assert!(context.target_dock_control().is_none());
    assert_eq!(context.resolved_operation.get(), DockOperation::None);
    assert!(!context.use_global_operation.get());
    assert!(!context.has_resolved_operation.get());
}
#[test]
fn source_start_end_and_clear_resolution_keep_cached_drag_offset_and_exact_control_reference() {
    run(|| {
        let context = DockDragContext::new();
        assert_cleared(&context);
        assert_eq!(context.drag_offset.get(), PixelPoint::default());
        let drag = Border::new();
        let target = Border::new();
        context.drag_offset.set(PixelPoint::new(4, -8));
        context.target_point.set(Point::new(6.0, 7.0));
        context.set_target_dock_control(Some(target.upcast()));
        context.resolved_operation.set(DockOperation::OuterRight);
        context.use_global_operation.set(true);
        context.has_resolved_operation.set(true);
        context.start(drag.clone().upcast(), Point::new(2.0, 3.0));
        assert_eq!(
            context.drag_control(),
            Some(drag.clone().upcast::<Control>())
        );
        assert_eq!(context.drag_start_point.get(), Point::new(2.0, 3.0));
        assert!(context.pointer_pressed.get());
        assert!(!context.do_drag_drop.get());
        assert!(context.target_dock_control().is_none());
        assert_eq!(context.resolved_operation.get(), DockOperation::None);
        assert_eq!(context.drag_offset.get(), PixelPoint::new(4, -8));
        context.do_drag_drop.set(true);
        context.resolved_operation.set(DockOperation::Fill);
        context.use_global_operation.set(true);
        context.has_resolved_operation.set(true);
        context.clear_resolved_operation();
        assert!(context.pointer_pressed.get() && context.do_drag_drop.get());
        assert_eq!(context.drag_control(), Some(drag.upcast::<Control>()));
        assert_eq!(context.resolved_operation.get(), DockOperation::None);
        assert!(!context.use_global_operation.get() && !context.has_resolved_operation.get());
        context.end();
        assert_cleared(&context);
        assert_eq!(context.drag_offset.get(), PixelPoint::new(4, -8));
    });
}
#[test]
fn source_copy_retains_every_field_and_shared_visual_identity_before_source_reset_and_self_copy() {
    run(|| {
        let source = DockDragContext::new();
        let target = DockDragContext::new();
        let drag = Border::new();
        let visual = Border::new();
        source.start(drag.clone().upcast(), Point::new(4.0, 5.0));
        source.do_drag_drop.set(true);
        source.target_point.set(Point::new(14.0, 15.0));
        source.set_target_dock_control(Some(visual.clone().upcast()));
        source.resolved_operation.set(DockOperation::OuterBottom);
        source.use_global_operation.set(true);
        source.has_resolved_operation.set(true);
        source.drag_offset.set(PixelPoint::new(-9, 7));
        source.copy_to(&target);
        source.end();
        assert_eq!(
            target.drag_control(),
            Some(drag.clone().upcast::<Control>())
        );
        assert_eq!(
            target.target_dock_control(),
            Some(visual.upcast::<Visual>())
        );
        assert_eq!(target.drag_start_point.get(), Point::new(4.0, 5.0));
        assert_eq!(target.target_point.get(), Point::new(14.0, 15.0));
        assert!(
            target.pointer_pressed.get()
                && target.do_drag_drop.get()
                && target.use_global_operation.get()
                && target.has_resolved_operation.get()
        );
        assert_eq!(target.resolved_operation.get(), DockOperation::OuterBottom);
        assert_eq!(target.drag_offset.get(), PixelPoint::new(-9, 7));
        target.copy_to(&target);
        assert_eq!(target.drag_control(), Some(drag.upcast::<Control>()));
        assert_eq!(target.resolved_operation.get(), DockOperation::OuterBottom);
    });
}
#[test]
fn source_context_retains_detached_controls_until_explicit_end_and_copy_target_keeps_its_own_reference(
) {
    run(|| {
        let source = DockDragContext::new();
        let target = DockDragContext::new();
        let drag = Border::new();
        let visual = Border::new();
        let weak_drag = drag.downgrade();
        let weak_visual = visual.downgrade();
        source.start(drag.upcast(), Point::default());
        source.set_target_dock_control(Some(visual.upcast()));
        source.copy_to(&target);
        source.end();
        assert!(weak_drag.upgrade().is_some() && weak_visual.upgrade().is_some());
        target.end();
        assert!(weak_drag.upgrade().is_none() && weak_visual.upgrade().is_none());
    });
}

#[test]
fn source_net10_numeric_reference_preserves_even_rounding_saturation_nan_and_positive_max_zero() {
    for line in include_str!("../../../../tests/Dock/data/drag-state-numeric.tsv")
        .lines()
        .skip(1)
    {
        let columns: Vec<_> = line.split('\t').collect();
        let value = f64::from_bits(u64::from_str_radix(columns[0], 16).unwrap());
        let expected: i32 = columns[1].parse().unwrap();
        assert_eq!(
            round_screen_point(Point::new(value, value)),
            PixelPoint::new(expected, expected),
            "{}",
            columns[0]
        );
        assert_eq!(
            max_preview_dimension(value).to_bits(),
            u64::from_str_radix(columns[2], 16).unwrap(),
            "{}",
            columns[0]
        );
    }
}
