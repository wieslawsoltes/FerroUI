// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original fixed-size host panel scenarios using native derived controls.
use super::*;
use ferroui_base::{layout::LayoutableImpl, threading::CancellationToken};
use ferroui_controls::{primitives::TemplatedControlImpl, AppBuilder, Application};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
#[repr(C)]
struct FixedSizeControl {
    base: Control,
    size: Size,
}
ferro_class!(FixedSizeControl:Control);
ferro_class_info!(FixedSizeControl {});
ferro_impl_classes!(FixedSizeControl:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl LayoutableImpl for FixedSizeControl {
    fn measure_override(this: &Self, _: Size) -> Size {
        this.size
    }
}
#[repr(C)]
struct FixedSizePinnedDockControl {
    base: PinnedDockControl,
    size: Size,
}
ferro_class!(FixedSizePinnedDockControl:PinnedDockControl);
ferro_class_info!(FixedSizePinnedDockControl {});
ferro_impl_classes!(FixedSizePinnedDockControl:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
impl LayoutableImpl for FixedSizePinnedDockControl {
    fn measure_override(this: &Self, _: Size) -> Size {
        this.size
    }
}
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "source-pinned-panel",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(f: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                f();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
fn scene(
    mode: PinnedDockDisplayMode,
) -> (
    Ref<PinnedDockHostPanel>,
    Ref<FixedSizeControl>,
    Ref<FixedSizePinnedDockControl>,
) {
    let panel = PinnedDockHostPanel::new();
    panel.set_pinned_dock_display_mode(mode);
    panel.set_pinned_dock_alignment(Alignment::Left);
    let main = instantiate(FixedSizeControl {
        base: Control::construct(),
        size: Size::new(300.0, 200.0),
    });
    let pinned = instantiate(FixedSizePinnedDockControl {
        base: PinnedDockControl::construct(),
        size: Size::new(100.0, 200.0),
    });
    panel.children().add(main.clone());
    panel.children().add(pinned.clone());
    (panel, main, pinned)
}
fn layout(panel: &PinnedDockHostPanel) {
    panel.measure(Size::new(400.0, 200.0));
    panel.arrange(Rect::new(0.0, 0.0, 400.0, 200.0));
}
#[test]
fn original_inline_left_allocates_space() {
    run(|| {
        let (panel, main, pinned) = scene(PinnedDockDisplayMode::Inline);
        layout(&panel);
        assert_eq!(pinned.bounds(), Rect::new(0.0, 0.0, 100.0, 200.0));
        assert_eq!(main.bounds(), Rect::new(100.0, 0.0, 300.0, 200.0));
    });
}
#[test]
fn original_overlay_overlaps_children() {
    run(|| {
        let (panel, main, pinned) = scene(PinnedDockDisplayMode::Overlay);
        layout(&panel);
        let expected = Rect::new(0.0, 0.0, 400.0, 200.0);
        assert_eq!(pinned.bounds(), expected);
        assert_eq!(main.bounds(), expected);
    });
}
#[test]
fn original_inline_without_pinned_dockables_does_not_reserve_space() {
    run(|| {
        use ferrodock_model::{collections::ListHandle, core::IDock};
        use ferrodock_model_reactiveui::controls::{RootDock, ToolDock};
        let (panel, main, pinned) = scene(PinnedDockDisplayMode::Inline);
        let root = RootDock::new();
        let dock = ToolDock::new();
        dock.set_visible_dockables(Some(ListHandle::new()));
        root.set_pinned_dock(Some(dock));
        panel.set_data_context(Some(model_projection::to_dockable_value(root)));
        layout(&panel);
        let expected = Rect::new(0.0, 0.0, 400.0, 200.0);
        assert_eq!(pinned.bounds(), expected);
        assert_eq!(main.bounds(), expected);
    });
}
#[test]
fn original_inline_uses_pinned_control_alignment() {
    run(|| {
        let (panel, main, pinned) = scene(PinnedDockDisplayMode::Inline);
        pinned.set_pinned_dock_alignment(Alignment::Right);
        layout(&panel);
        assert_eq!(pinned.bounds(), Rect::new(300.0, 0.0, 100.0, 200.0));
        assert_eq!(main.bounds(), Rect::new(0.0, 0.0, 300.0, 200.0));
    });
}
