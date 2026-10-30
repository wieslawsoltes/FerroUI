// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Source pinned sizing/control assertions on the actual native implementation.
use super::*;
use crate::controls::PinnedDockHostPanel;
use ferrodock_model::{collections::ListHandle, core::IDock};
use ferrodock_model_reactiveui::{
    controls::{ProportionalDockSplitter, RootDock, Tool, ToolDock},
    Factory,
};
use ferroui_base::{input::VectorEventArgs, threading::CancellationToken};
use ferroui_controls::{templates::FuncControlTemplate, AppBuilder, Application, Border};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "source-pinned-host",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
struct Setting(bool);
impl Drop for Setting {
    fn drop(&mut self) {
        DockSettings::set_use_pinned_dock_window(self.0);
    }
}
fn run(f: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                let _setting = Setting(DockSettings::use_pinned_dock_window());
                DockSettings::set_use_pinned_dock_window(false);
                f();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
struct Scene {
    window: Ref<Window>,
    control: Ref<PinnedDockControl>,
    tool: Rc<Tool>,
    root: Rc<RootDock>,
    pinned: Rc<ToolDock>,
    _factory: Rc<Factory>,
}
impl Scene {
    fn new(alignment: Alignment, mode: PinnedDockDisplayMode, custom_thumb: bool) -> Self {
        let factory = Factory::new();
        let tool = Tool::new();
        tool.set_pinned_bounds(0.0, 0.0, 200.0, 150.0);
        let pinned = ToolDock::new();
        pinned.set_alignment(alignment);
        pinned.set_is_empty(false);
        let list = ListHandle::<Option<Rc<dyn IDockable>>>::new();
        list.add(Some(tool.clone()));
        pinned.set_visible_dockables(Some(list));
        pinned.set_active_dockable(Some(tool.clone()));
        let root = RootDock::new();
        root.set_factory(Some(factory.clone()));
        root.set_pinned_dock(Some(pinned.clone()));
        root.set_pinned_dock_display_mode(mode);
        let control = PinnedDockControl::new();
        control.set_pinned_dock_alignment(alignment);
        control.set_pinned_dock_display_mode(mode);
        control.set_data_context(Some(model_projection::to_dockable_value(root.clone())));
        control.set_template(Some(FuncControlTemplate::for_type::<PinnedDockControl>(
            move |_, scope| {
                let grid = Grid::new();
                grid.set_name(Some("PART_PinnedDockGrid".into()));
                let content = ContentControl::new();
                content.set_name(Some("PART_PinnedDock".into()));
                content.set_content(Some(Rc::new("Preview".to_owned())));
                let splitter = if custom_thumb {
                    Thumb::new()
                } else {
                    GridSplitter::new().upcast::<Thumb>()
                };
                splitter.set_name(Some("PART_PinnedDockSplitter".into()));
                splitter.set_width(6.0);
                scope.register("PART_PinnedDockGrid", grid.clone().upcast());
                scope.register("PART_PinnedDock", content.clone().upcast());
                scope.register("PART_PinnedDockSplitter", splitter.clone().upcast());
                grid.children().add(content);
                grid.children().add(splitter);
                grid.upcast()
            },
        )));
        let host = PinnedDockHostPanel::new();
        host.set_data_context(control.data_context());
        host.set_pinned_dock_alignment(alignment);
        host.set_pinned_dock_display_mode(mode);
        host.children().add(Border::new());
        host.children().add(control.clone());
        let window = Window::new();
        window.set_width(800.0);
        window.set_height(600.0);
        window.set_content(Some(Rc::new(host)));
        window.show();
        control.apply_template();
        window.update_layout();
        control.update_layout();
        Self {
            window,
            control,
            tool,
            root,
            pinned,
            _factory: factory,
        }
    }
}
impl Drop for Scene {
    fn drop(&mut self) {
        self.window.close();
    }
}
#[test]
fn original_stored_width_reapplies_after_external_column_change() {
    run(|| {
        let scene = Scene::new(Alignment::Left, PinnedDockDisplayMode::Overlay, false);
        let grid = scene.control.grid();
        assert_eq!(grid.column_definitions().get(0).width().value(), 200.0);
        grid.column_definitions()
            .get(0)
            .set_width(GridLength::new(100.0, GridUnitType::Pixel));
        grid.invalidate_measure();
        scene.control.update_layout();
        assert_eq!(grid.column_definitions().get(0).width().value(), 200.0);
    });
}
#[test]
fn original_drag_resize_commits_actual_content_bounds() {
    run(|| {
        let scene = Scene::new(Alignment::Left, PinnedDockDisplayMode::Overlay, false);
        scene.control.on_pinned_dock_splitter_drag_started();
        scene
            .control
            .grid()
            .column_definitions()
            .get(0)
            .set_width(GridLength::new(300.0, GridUnitType::Pixel));
        scene.control.grid().invalidate_measure();
        scene.control.update_layout();
        scene.control.on_pinned_dock_splitter_drag_completed();
        assert_eq!(scene.tool.get_pinned_bounds().2, 300.0);
    });
}
#[test]
fn original_override_selects_inline_grid_and_tracks_later_mode_changes() {
    run(|| {
        let scene = Scene::new(Alignment::Left, PinnedDockDisplayMode::Overlay, false);
        scene
            .tool
            .set_pinned_dock_display_mode_override(Some(PinnedDockDisplayMode::Inline));
        scene.window.update_layout();
        assert_eq!(scene.control.grid().column_definitions().count(), 2);
        scene
            .tool
            .set_pinned_dock_display_mode_override(Some(PinnedDockDisplayMode::Overlay));
        assert_eq!(scene.control.grid().column_definitions().count(), 3);
        scene.window.close();
        assert!(scene.control.tracked_dockable.borrow().is_none());
    });
}
#[test]
fn original_inline_resize_updates_stored_bounds_and_native_definition() {
    run(|| {
        let scene = Scene::new(Alignment::Left, PinnedDockDisplayMode::Inline, false);
        assert!(scene
            .control
            .try_resize_inline_pinned_dock(Vector::new(40.0, 0.0)));
        assert_eq!(scene.tool.get_pinned_bounds().2, 240.0);
        assert_eq!(
            scene
                .control
                .grid()
                .column_definitions()
                .get(0)
                .width()
                .value(),
            240.0
        );
    });
}
#[test]
fn original_configured_minima_layout_and_resize_matrix() {
    run(|| {
        for mode in [
            PinnedDockDisplayMode::Inline,
            PinnedDockDisplayMode::Overlay,
        ] {
            for alignment in [
                Alignment::Left,
                Alignment::Right,
                Alignment::Top,
                Alignment::Bottom,
            ] {
                let scene = Scene::new(alignment, mode, false);
                assert_eq!(scene.control.minimum_pinned_width(), 50.0);
                assert_eq!(scene.control.minimum_pinned_height(), 50.0);
                scene.control.set_minimum_pinned_width(240.0);
                scene.control.set_minimum_pinned_height(180.0);
                scene.window.update_layout();
                let horizontal = matches!(alignment, Alignment::Left | Alignment::Right);
                let bounds = scene.control.dock().bounds();
                assert_eq!(
                    if horizontal {
                        bounds.width
                    } else {
                        bounds.height
                    },
                    if horizontal { 240.0 } else { 180.0 }
                );
                scene.control.set_minimum_pinned_width(310.0);
                scene.control.set_minimum_pinned_height(270.0);
                scene.window.update_layout();
                if mode == PinnedDockDisplayMode::Inline {
                    let delta = if matches!(alignment, Alignment::Right | Alignment::Bottom) {
                        1000.0
                    } else {
                        -1000.0
                    };
                    assert!(scene.control.try_resize_inline_pinned_dock(if horizontal {
                        Vector::new(delta, 0.0)
                    } else {
                        Vector::new(0.0, delta)
                    }));
                    let (_, _, w, h) = scene.tool.get_pinned_bounds();
                    assert_eq!(
                        if horizontal { w } else { h },
                        if horizontal { 310.0 } else { 270.0 }
                    );
                }
            }
        }
    });
}
#[test]
fn original_minima_validation_rejects_nan_infinite_negative_and_accepts_zero() {
    run(|| {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        let control = PinnedDockControl::new();
        for value in [f64::NAN, f64::INFINITY, -1.0] {
            assert!(
                catch_unwind(AssertUnwindSafe(|| control.set_minimum_pinned_width(value))).is_err()
            );
            assert!(
                catch_unwind(AssertUnwindSafe(|| control.set_minimum_pinned_height(value)))
                    .is_err()
            );
        }
        control.set_minimum_pinned_width(0.0);
        control.set_minimum_pinned_height(0.0);
        assert_eq!(
            (
                control.minimum_pinned_width(),
                control.minimum_pinned_height()
            ),
            (0.0, 0.0)
        );
    });
}
#[test]
fn original_custom_thumb_is_positioned_without_grid_splitter_handlers() {
    run(|| {
        let scene = Scene::new(Alignment::Right, PinnedDockDisplayMode::Inline, true);
        let thumb = scene.control.splitter().unwrap();
        assert_eq!(Grid::get_column(&thumb), 0);
        for event in [
            Thumb::drag_started_event(),
            Thumb::drag_delta_event(),
            Thumb::drag_completed_event(),
        ] {
            let mut args = VectorEventArgs::new();
            args.set_routed_event(Some(event));
            args.vector = Vector::new(35.0, 0.0);
            thumb.raise_event(&args);
        }
        assert_eq!(scene.tool.get_pinned_bounds().2, 200.0);
        assert!(!scene.control.is_resizing_pinned_dock.get());
        scene.tool.set_pinned_bounds(0.0, 0.0, 260.0, 100.0);
        scene.control.invalidate_measure();
        scene.window.update_layout();
        assert_eq!(scene.control.dock().bounds().width, 260.0);
    });
}
#[test]
fn source_pinned_resolution_excludes_splitters_and_stale_active_and_uses_original_list() {
    run(|| {
        let scene = Scene::new(Alignment::Left, PinnedDockDisplayMode::Inline, false);
        let list = scene.pinned.visible_dockables().unwrap();
        let stale = Tool::new();
        scene.pinned.set_active_dockable(Some(stale));
        assert!(Rc::ptr_eq(
            &PinnedDockControl::get_pinned_dockable(&*scene.root).unwrap(),
            &(scene.tool.clone() as Rc<dyn IDockable>)
        ));
        list.insert(0, Some(ProportionalDockSplitter::new()));
        scene.pinned.set_active_dockable(list.get(0));
        assert!(Rc::ptr_eq(
            &PinnedDockControl::get_pinned_dockable(&*scene.root).unwrap(),
            &(scene.tool.clone() as Rc<dyn IDockable>)
        ));
        list.clear();
        assert!(PinnedDockControl::get_pinned_dockable(&*scene.root).is_none());
    });
}
#[test]
fn source_peer_reports_pinned_state_and_expands_original_model() {
    run(|| {
        use ferroui_controls::automation::{
            peers::{AutomationControlType, ControlAutomationPeer},
            provider::IExpandCollapseProvider,
            ExpandCollapseState,
        };
        let scene = Scene::new(Alignment::Right, PinnedDockDisplayMode::Inline, false);
        scene.tool.set_title("Pinned tool".into());
        let peer = ControlAutomationPeer::create_peer_for_element(&scene.control);
        assert_eq!(peer.get_class_name(), "PinnedDockControl");
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::Pane
        );
        assert_eq!(peer.get_name(), "Pinned tool");
        let provider = peer.get_provider::<dyn IExpandCollapseProvider>().unwrap();
        assert!(!provider.shows_menu());
        provider.expand().unwrap();
        assert!(scene.pinned.is_expanded());
        assert_eq!(
            provider.expand_collapse_state(),
            ExpandCollapseState::Expanded
        );
        provider.collapse().unwrap();
        assert!(!scene.pinned.is_expanded());
        scene.pinned.visible_dockables().unwrap().clear();
        provider.expand().unwrap();
        assert_eq!(
            provider.expand_collapse_state(),
            ExpandCollapseState::Collapsed
        );
    });
}
#[test]
fn source_native_window_branch_uses_original_content_and_clears_overlay_properties_on_close() {
    run(|| {
        let scene = Scene::new(Alignment::Left, PinnedDockDisplayMode::Overlay, false);
        DockSettings::set_use_pinned_dock_window(true);
        scene.control.update_window();
        assert!(scene.control.window.borrow().is_some());
        assert_eq!(scene.control.dock().opacity(), 0.0);
        assert!(!scene.control.dock().is_hit_test_visible());
        let window = scene.control.window();
        let content = window.content().unwrap();
        let native = ferroui_base::data::core::ValueTypes::as_object(&*content)
            .unwrap()
            .cast::<ToolDockControl>()
            .unwrap();
        let dock = model_projection::dockable(native.data_context().as_ref()).unwrap();
        assert!(Rc::ptr_eq(
            &dock,
            &(scene.pinned.clone() as Rc<dyn IDockable>)
        ));
        DockSettings::set_use_pinned_dock_window(false);
        scene.control.update_window();
        assert!(scene.control.window.borrow().is_some());
        scene.control.close_window();
        assert!(scene.control.window.borrow().is_none());
        assert_eq!(scene.control.dock().opacity(), 1.0);
        assert!(scene.control.dock().is_hit_test_visible());
    });
}
#[test]
fn source_size_helpers_keep_half_pixel_tolerance_and_nan_rules() {
    run(|| {
        assert!(PinnedDockControl::are_close(10.0, 10.5));
        assert!(!PinnedDockControl::are_close(10.0, 10.500001));
        for value in [0.0, -0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(!PinnedDockControl::is_valid_size(value));
        }
        assert!(max(1.0, f64::NAN).is_nan());
        assert_eq!(max(-0.0, 0.0).to_bits(), 0.0_f64.to_bits());
        let defs = ColumnDefinitions::new();
        PinnedDockControl::set_column_size(&defs, -1, 10.0);
        assert!(!PinnedDockControl::is_column_size_applied(&defs, 0, 10.0));
    });
}
