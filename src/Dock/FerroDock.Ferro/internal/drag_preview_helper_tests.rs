// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Source preview mechanics on actual native windows, overlays and production model identities.
use super::*;
use crate::controls::DocumentContentControl;
use ferrodock_model::core::{DockFloatingWindowHostMode, DockRect, IFactory};
use ferrodock_model_reactiveui::{
    controls::{Document, RootDock, Tool},
    core::DockWindow,
    Factory,
};
use ferroui_base::threading::CancellationToken;
use ferroui_controls::{
    templates::FuncControlTemplate, AppBuilder, Application, Button, Canvas, ContentControl,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession, HeadlessWindowExtensions,
};
use std::panic::{catch_unwind, AssertUnwindSafe};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "drag-preview-helper",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
struct Cleanup;
impl Drop for Cleanup {
    fn drop(&mut self) {
        DragPreviewHelper::new().hide();
        DragPreviewHelper::new().hide();
        Dispatcher::ui_thread().run_jobs(None);
    }
}
fn run(f: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                let _cleanup = Cleanup;
                DragPreviewHelper::new().hide();
                DragPreviewHelper::new().hide();
                Dispatcher::ui_thread().run_jobs(None);
                f();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
struct Scene(Ref<Window>);
impl Scene {
    fn new(control: Ref<Control>) -> Self {
        let w = Window::new();
        w.set_width(400.0);
        w.set_height(300.0);
        w.set_content(Some(Control::boxed(control)));
        w.show();
        w.update_layout();
        Self(w)
    }
}
impl Drop for Scene {
    fn drop(&mut self) {
        self.0.close();
        self.0.set_content(None);
    }
}
fn layer() -> (Ref<ManagedWindowLayer>, Ref<Canvas>, Scene) {
    let layer = ManagedWindowLayer::new();
    layer.set_template(Some(FuncControlTemplate::for_type::<ManagedWindowLayer>(
        |_, scope| {
            let canvas = Canvas::new();
            canvas.set_name(Some("PART_OverlayCanvas".into()));
            scope.register("PART_OverlayCanvas", canvas.clone().upcast());
            canvas.upcast()
        },
    )));
    let scene = Scene::new(layer.clone().upcast());
    layer.apply_template();
    let canvas = layer
        .get_visual_descendants()
        .find_map(|v| v.cast::<Canvas>())
        .unwrap();
    (layer, canvas, scene)
}
fn install_window() -> (Ref<DragPreviewWindow>, Ref<DragPreviewControl>) {
    let window = DragPreviewWindow::new();
    let control = DragPreviewControl::new();
    let state = DragPreviewHelper::state();
    state.window.replace(Some(window.clone()));
    state.control.replace(Some(control.clone()));
    (window, control)
}
#[test]
fn original_native_preview_uses_registered_live_content_as_visual_brush_proxy_for_tool_and_document(
) {
    run(|| {
        let factory = Factory::new();
        for use_tool in [false, true] {
            let item: Rc<dyn IDockable> = if use_tool {
                factory.create_tool()
            } else {
                factory.create_document()
            };
            item.set_factory(Some(factory.clone()));
            let content = ContentControl::new();
            content.set_content(Some(Control::boxed(Button::new())));
            let _scene = Scene::new(content.clone().upcast());
            let map = if use_tool {
                factory.tool_controls()
            } else {
                factory.document_controls()
            };
            map.borrow_mut()
                .insert(item.clone(), Rc::new(content.clone()));
            let root = content.visual_root().unwrap();
            let parent = content.parent();
            let preview = DragPreviewHelper::build_dockable_preview_content(item.clone(), true)
                .unwrap()
                .cast::<Border>()
                .unwrap();
            let brush = preview
                .background()
                .unwrap()
                .as_object()
                .unwrap()
                .to_ref()
                .cast::<VisualBrush>()
                .unwrap();
            assert_eq!(brush.visual(), Some(content.clone().upcast::<Visual>()));
            assert_eq!(brush.stretch(), Stretch::Uniform);
            assert!(preview.clip_to_bounds());
            assert_eq!(content.visual_root(), Some(root));
            assert_eq!(content.parent(), parent);
            assert!(
                DragPreviewHelper::build_dockable_preview_content(item.clone(), false).is_none()
            );
            map.borrow_mut().remove(&item);
        }
    });
}
#[test]
fn preview_content_guards_tool_registry_priority_native_handle_transport_and_detach_are_preserved()
{
    run(|| {
        let factory = Factory::new();
        let tool = Tool::new();
        tool.set_factory(Some(factory.clone()));
        let item: Rc<dyn IDockable> = tool;
        let document = Document::new();
        assert!(DragPreviewHelper::build_dockable_preview_content(document, true).is_none());
        assert!(DragPreviewHelper::build_dockable_preview_content(item.clone(), true).is_none());
        let tool_control = Border::new();
        let doc_control = DocumentContentControl::new();
        let panel = ferroui_controls::Panel::new();
        panel
            .children()
            .add(tool_control.clone().upcast::<Control>());
        panel
            .children()
            .add(doc_control.clone().upcast::<Control>());
        let scene = Scene::new(panel.upcast());
        factory
            .tool_controls()
            .borrow_mut()
            .insert(item.clone(), Rc::new(tool_control.clone()));
        factory
            .document_controls()
            .borrow_mut()
            .insert(item.clone(), Rc::new(doc_control));
        let preview = DragPreviewHelper::build_dockable_preview_content(item.clone(), true)
            .unwrap()
            .cast::<Border>()
            .unwrap();
        let brush = preview
            .background()
            .unwrap()
            .as_object()
            .unwrap()
            .to_ref()
            .cast::<VisualBrush>()
            .unwrap();
        assert_eq!(
            brush.visual(),
            Some(tool_control.clone().upcast::<Visual>())
        );
        factory
            .tool_controls()
            .borrow_mut()
            .insert(item.clone(), Rc::new("wrong type".to_owned()));
        assert!(DragPreviewHelper::build_dockable_preview_content(item.clone(), true).is_none());
        factory
            .tool_controls()
            .borrow_mut()
            .insert(item.clone(), Rc::new(tool_control));
        drop(scene);
        assert!(DragPreviewHelper::build_dockable_preview_content(item, true).is_none());
    });
}
#[test]
fn source_preview_size_uses_complete_managed_bounds_then_individual_visible_owner_and_default_dimensions(
) {
    run(|| {
        let doc = Document::new();
        let owner = RootDock::new();
        doc.set_owner(Some(owner.clone()));
        owner.set_visible_bounds(0.0, 0.0, 610.0, 720.0);
        for (w, h, expected) in [
            (20.0, 30.0, Size::new(20.0, 30.0)),
            (f64::NAN, 30.0, Size::new(610.0, 30.0)),
            (20.0, -1.0, Size::new(20.0, 720.0)),
            (0.0, f64::NAN, Size::new(610.0, 720.0)),
            (f64::INFINITY, 30.0, Size::new(f64::INFINITY, 30.0)),
        ] {
            doc.set_visible_bounds(0.0, 0.0, w, h);
            assert_eq!(DragPreviewHelper::get_preview_size(doc.clone()), expected);
        }
        owner.set_visible_bounds(0.0, 0.0, f64::NAN, -1.0);
        doc.set_visible_bounds(0.0, 0.0, 0.0, 0.0);
        assert_eq!(
            DragPreviewHelper::get_preview_size(doc.clone()),
            Size::new(300.0, 400.0)
        );
        let nondock = Document::new();
        nondock.set_visible_bounds(0.0, 0.0, 900.0, 800.0);
        doc.set_owner(Some(nondock.clone()));
        assert_eq!(
            DragPreviewHelper::get_preview_size(doc),
            Size::new(300.0, 400.0)
        );
        let window = DockWindow::new();
        let managed = ManagedDockWindowDocument::new(window.clone());
        managed.set_mdi_bounds(DockRect::new(0.0, 0.0, 345.0, 234.0));
        assert_eq!(
            DragPreviewHelper::get_preview_size(managed.clone()),
            Size::new(345.0, 234.0)
        );
        managed.set_mdi_bounds(DockRect::new(0.0, 0.0, f64::NAN, 234.0));
        managed.set_visible_bounds(0.0, 0.0, 50.0, 60.0);
        assert_eq!(
            DragPreviewHelper::get_preview_size(managed.clone()),
            Size::new(50.0, 60.0)
        );
        managed.dispose();
    });
}
#[test]
fn original_opacity_clamping_preserves_nan_default_infinity_and_signed_zero() {
    for (value, expected) in [
        (f64::NAN, 1.0_f64),
        (f64::NEG_INFINITY, 0.0),
        (-0.2, 0.0),
        (-0.0, -0.0),
        (0.4, 0.4),
        (1.2, 1.0),
        (f64::INFINITY, 1.0),
    ] {
        assert_eq!(
            DragPreviewHelper::clamp_opacity(value).to_bits(),
            expected.to_bits()
        );
    }
}
#[test]
fn source_default_templates_initialize_once_and_failed_add_leaves_partial_list_for_retry() {
    run(|| {
        let control = DragPreviewControl::new();
        let initialized = Cell::new(false);
        DragPreviewHelper::ensure_data_templates(&control, &initialized);
        assert!(initialized.get());
        assert_eq!(control.data_templates().count(), 14);
        let first = control.data_templates().get(0);
        DragPreviewHelper::ensure_data_templates(&control, &initialized);
        assert_eq!(control.data_templates().count(), 14);
        assert!(Rc::ptr_eq(&first, &control.data_templates().get(0)));
        let control = DragPreviewControl::new();
        let initialized = Cell::new(false);
        let templates = control.data_templates();
        let token =
            templates.add_collection_changed(Rc::new(|_| panic!("source add observer failed")));
        assert!(catch_unwind(AssertUnwindSafe(|| {
            DragPreviewHelper::ensure_data_templates(&control, &initialized)
        }))
        .is_err());
        assert!(!initialized.get());
        assert_eq!(templates.count(), 1);
        assert!(templates.remove_collection_changed(token));
        DragPreviewHelper::ensure_data_templates(&control, &initialized);
        assert_eq!(templates.count(), 15);
        assert!(initialized.get());
    });
}
#[test]
fn source_floating_show_continuation_keeps_model_identity_offsets_size_templates_and_reuse_until_hide(
) {
    run(|| {
        let model: Rc<dyn IDockable> = Document::new();
        DragPreviewHelper::show_floating(
            model.clone(),
            PixelPoint::new(100, 200),
            PixelPoint::new(-4, 7),
            false,
            Some(Size::new(170.0, 90.0)),
        );
        let state = DragPreviewHelper::state();
        let window = state.expect_window();
        let control = state.expect_control();
        assert!(window.is_visible());
        assert_eq!(window.position(), PixelPoint::new(96, 207));
        assert_eq!((window.width(), window.height()), (170.0, 90.0));
        assert_eq!(window.size_to_content(), SizeToContent::MANUAL);
        assert!(Rc::ptr_eq(
            &model,
            &model_projection::dockable(window.data_context().as_ref()).unwrap()
        ));
        assert!(!control.show_content());
        assert!(
            control.preview_content_width().is_nan() && control.preview_content_height().is_nan()
        );
        assert_eq!(control.data_templates().count(), 14);
        let session = state.window_move_session_id.get();
        DragPreviewHelper::show_floating(
            model.clone(),
            PixelPoint::new(15, 16),
            PixelPoint::default(),
            true,
            Some(Size::new(120.0, 80.0)),
        );
        assert_eq!(state.window(), Some(window.clone()));
        assert_eq!(state.control(), Some(control.clone()));
        assert_eq!(
            (
                control.preview_content_width(),
                control.preview_content_height()
            ),
            (120.0, 80.0)
        );
        assert_eq!(control.data_templates().count(), 14);
        assert_eq!(state.window_move_session_id.get(), session.wrapping_add(1));
        DragPreviewContext::set_dockable(Some(model.clone()));
        DragPreviewContext::set_is_active(true);
        DragPreviewHelper::new().hide();
        assert!(state.window().is_none() && state.control().is_none());
        assert!(!state.window_templates_initialized.get());
        assert!(!window.is_visible());
        assert!(DragPreviewContext::dockable().is_none());
        DragPreviewHelper::show_floating(
            model,
            PixelPoint::default(),
            PixelPoint::default(),
            false,
            Some(Size::new(40.0, 50.0)),
        );
        assert_ne!(state.window(), Some(window));
    });
}
#[test]
fn original_abandoned_size_freeze_does_not_reschedule_on_later_move() {
    run(|| {
        let (window, control) = install_window();
        let state = DragPreviewHelper::state();
        state.window_size_frozen.set(false);
        state.window_size_freeze_scheduled.set(false);
        state.window_size_freeze_abandoned.set(true);
        state.window_size_freeze_retry_count.set(4);
        DragPreviewHelper::apply_window_move(
            &window,
            &control,
            PixelPoint::new(24, 42),
            Some("Float"),
        );
        assert_eq!(control.status().as_deref(), Some("Float"));
        assert_eq!(window.position(), PixelPoint::new(24, 42));
        assert!(!state.window_size_freeze_scheduled.get());
        assert!(state.window_size_freeze_abandoned.get());
    });
}
#[test]
fn source_move_coalesces_latest_pending_status_and_ignores_stale_session_and_sequence() {
    run(|| {
        let (window, control) = install_window();
        let helper = DragPreviewHelper::new();
        let state = DragPreviewHelper::state();
        let original = window.position();
        helper.move_(PixelPoint::new(30, 40), PixelPoint::new(1, 2), "Float");
        let sequence = state.window_move_post_sequence.get();
        helper.move_(PixelPoint::new(30, 40), PixelPoint::new(1, 2), "Float");
        assert_eq!(state.window_move_post_sequence.get(), sequence);
        helper.move_(PixelPoint::new(80, 90), PixelPoint::new(2, 3), "Dock");
        if USE_WINDOW_MOVE_COALESCING {
            assert_eq!(window.position(), original);
            assert!(state.has_pending_window_move.get());
            let session = state.window_move_session_id.get();
            DragPreviewHelper::flush_pending_window_move(session.wrapping_sub(1), sequence);
            assert!(state.window_move_flush_scheduled.get());
            assert_eq!(window.position(), original);
            DragPreviewHelper::flush_pending_window_move(session, sequence.wrapping_sub(1));
            assert!(state.has_pending_window_move.get());
            DragPreviewHelper::flush_pending_window_move(session, sequence);
            assert!(!state.has_pending_window_move.get());
        }
        assert_eq!(window.position(), PixelPoint::new(82, 93));
        assert_eq!(control.status().as_deref(), Some("Dock"));
        Dispatcher::ui_thread().run_jobs(None);
        let sequence = state.window_move_post_sequence.get();
        helper.move_(PixelPoint::new(82, 93), PixelPoint::default(), "Dock");
        assert_eq!(state.window_move_post_sequence.get(), sequence);
        assert!(!state.has_pending_window_move.get());
    });
}
#[test]
fn source_flush_retains_reentrant_pending_overwrite_instead_of_inventing_a_followup_move() {
    run(|| {
        let (window, control) = install_window();
        let state = DragPreviewHelper::state();
        let first = PixelPoint::new(21, 31);
        let later = PixelPoint::new(81, 91);
        let w = window.clone();
        let c = control.clone();
        let entered = Rc::new(Cell::new(false));
        let seen = entered.clone();
        let subscription = control.property_changed(move |e| {
            if e.property() == DragPreviewControl::status_property().as_property()
                && !seen.replace(true)
            {
                DragPreviewHelper::queue_window_move(&w, &c, later, Some("Dock"));
            }
        });
        DragPreviewHelper::queue_window_move(&window, &control, first, Some("Float"));
        if USE_WINDOW_MOVE_COALESCING {
            DragPreviewHelper::flush_pending_window_move(
                state.window_move_session_id.get(),
                state.window_move_post_sequence.get(),
            );
            assert_eq!(state.pending_window_position.get(), later);
            assert!(!state.has_pending_window_move.get());
        }
        Dispatcher::ui_thread().run_jobs(None);
        assert!(entered.get());
        assert_eq!(window.position(), first);
        assert_eq!(
            control.status().as_deref(),
            Some(if USE_WINDOW_MOVE_COALESCING {
                "Float"
            } else {
                "Dock"
            })
        );
        assert!(!state.window_move_flush_scheduled.get());
        subscription.dispose();
    });
}
#[test]
fn source_freeze_retains_pixel_sizes_across_real_native_scaling_changes_and_skips_same_scale_repair(
) {
    run(|| {
        let (window, control) = install_window();
        window.set_content(Some(Control::boxed(control.clone())));
        window.set_width(160.0);
        window.set_height(80.0);
        control.set_preview_content_width(100.0);
        control.set_preview_content_height(40.0);
        window.show();
        window.update_layout();
        let state = DragPreviewHelper::state();
        DragPreviewHelper::freeze_window_size_if_needed();
        assert!(state.window_size_frozen.get());
        let width_pixels = state.frozen_window_width_pixels.get();
        let height_pixels = state.frozen_window_height_pixels.get();
        assert_eq!(
            state.frozen_content_width_pixels.get(),
            100.0 * window.render_scaling()
        );
        window.set_width(999.0);
        DragPreviewHelper::maintain_frozen_window_size(&window, &control);
        assert_eq!(window.width(), 999.0);
        window.set_render_scaling(2.0);
        DragPreviewHelper::maintain_frozen_window_size(&window, &control);
        assert_eq!(window.width(), width_pixels / 2.0);
        assert_eq!(window.height(), height_pixels / 2.0);
        assert_eq!(control.preview_content_width(), 50.0);
        assert_eq!(control.preview_content_height(), 20.0);
        control.set_preview_content_width(f64::NAN);
        state.frozen_content_height_pixels.set(f64::NAN);
        window.set_render_scaling(1.0);
        DragPreviewHelper::maintain_frozen_window_size(&window, &control);
        assert!(control.preview_content_width().is_nan());
        assert_eq!(control.preview_content_height(), 20.0);
    });
}
#[test]
fn original_managed_overlay_movement_and_hide_keep_source_control_and_model_without_reparenting_live_content(
) {
    run(|| {
        let (layer, canvas, _scene) = layer();
        let root = RootDock::new();
        root.set_floating_window_host_mode(DockFloatingWindowHostMode::Managed);
        let doc = Document::new();
        doc.set_owner(Some(root.clone()));
        let item: Rc<dyn IDockable> = doc;
        DragPreviewHelper::show_managed(
            &layer,
            item.clone(),
            PixelPoint::new(20, 30),
            PixelPoint::new(4, 5),
            true,
            Some(Size::new(120.0, 80.0)),
        );
        let state = DragPreviewHelper::state();
        let control = state.expect_managed_control();
        assert_eq!(canvas.children().count(), 1);
        assert_eq!(
            canvas.children().get(0),
            control.clone().upcast::<Control>()
        );
        assert!(!control.is_hit_test_visible());
        assert_eq!((control.width(), control.height()), (120.0, 80.0));
        assert_eq!(control.data_templates().count(), 14);
        assert!(Rc::ptr_eq(
            &item,
            &model_projection::dockable(control.data_context().as_ref()).unwrap()
        ));
        // Preserve the source direct TopLevel root cast. This backend's root is TopLevelHost.
        assert!(layer.visual_root().unwrap().cast::<TopLevel>().is_none());
        assert_eq!(
            DragPreviewHelper::get_managed_bounds(
                &layer,
                PixelPoint::new(80, 90),
                PixelPoint::default(),
                Size::new(5.0, 6.0)
            ),
            Rect::new(0.0, 0.0, 5.0, 6.0)
        );
        DragPreviewContext::set_dockable(Some(item));
        DragPreviewHelper::new().move_(PixelPoint::new(80, 90), PixelPoint::default(), "Dock");
        assert_eq!(state.managed_control(), Some(control.clone()));
        assert_eq!(control.status().as_deref(), Some("Dock"));
        assert_eq!(Canvas::get_left(&control), 0.0);
        DragPreviewHelper::new().hide();
        assert_eq!(canvas.children().count(), 0);
        assert!(state.managed_control().is_none() && state.managed_layer().is_none());
        assert!(control.preview_content().is_none());
        assert!(!state.managed_templates_initialized.get());
        assert!(DragPreviewContext::dockable().is_none());
    });
}
#[test]
fn source_managed_hide_returns_before_clearing_an_existing_floating_preview() {
    run(|| {
        let model: Rc<dyn IDockable> = Document::new();
        DragPreviewHelper::show_floating(
            model.clone(),
            PixelPoint::default(),
            PixelPoint::default(),
            false,
            Some(Size::new(40.0, 50.0)),
        );
        let state = DragPreviewHelper::state();
        let floating = state.expect_window();
        let (layer, _, _scene) = layer();
        DragPreviewHelper::show_managed(
            &layer,
            model,
            PixelPoint::default(),
            PixelPoint::default(),
            false,
            None,
        );
        DragPreviewHelper::new().hide();
        assert!(state.managed_layer().is_none());
        assert_eq!(state.window(), Some(floating.clone()));
        assert!(floating.is_visible());
        DragPreviewHelper::new().hide();
        assert!(state.window().is_none());
    });
}
#[test]
fn factory_search_and_layer_ready_root_guards_preserve_source_precedence_and_shallow_visibility() {
    run(|| {
        let first = Factory::new();
        let second = Factory::new();
        let root = RootDock::new();
        root.set_factory(Some(first.clone()));
        let item = Document::new();
        item.set_owner(Some(root.clone()));
        assert!(Rc::ptr_eq(
            &DragPreviewHelper::find_factory(Some(item.clone())).unwrap(),
            &(first as Rc<dyn IFactory>)
        ));
        item.set_factory(Some(second.clone()));
        assert!(Rc::ptr_eq(
            &DragPreviewHelper::find_factory(Some(item.clone())).unwrap(),
            &(second as Rc<dyn IFactory>)
        ));
        item.set_factory(None);
        item.set_owner(None);
        assert!(DragPreviewHelper::find_factory(Some(item.clone())).is_none());
        assert!(DragPreviewHelper::find_factory(None).is_none());
        assert_eq!(
            DragPreviewHelper::should_show_preview_content(&*item),
            DockSettings::show_dockable_preview_on_drag()
        );
        let detached = ManagedWindowLayer::new();
        assert!(!DragPreviewHelper::is_layer_ready(Some(&detached)));
        assert!(DragPreviewHelper::is_layer_in_root(Some(&detached), None));
        assert!(!DragPreviewHelper::is_layer_in_root(None, None));
        let (layer, _, scene) = layer();
        let root = layer.visual_root().unwrap();
        assert!(DragPreviewHelper::is_layer_ready(Some(&layer)));
        assert!(DragPreviewHelper::is_layer_in_root(
            Some(&layer),
            Some(&root)
        ));
        let other = Scene::new(Border::new().upcast());
        assert!(!DragPreviewHelper::is_layer_in_root(
            Some(&layer),
            other.0.visual_root().as_deref()
        ));
        scene.0.hide();
        assert!(layer.is_visible());
        assert!(DragPreviewHelper::is_layer_ready(Some(&layer)));
    });
}

#[test]
fn floating_show_premeasures_custom_template_and_keeps_original_partial_preferred_size_behavior() {
    run(|| {
        let (window, control) = install_window();
        window.set_content(Some(Control::boxed(control.clone())));
        control.set_template(Some(FuncControlTemplate::for_type::<DragPreviewControl>(
            |_, _| {
                let border = Border::new();
                border.set_width(80.0);
                border.set_height(40.0);
                border.upcast()
            },
        )));
        let model: Rc<dyn IDockable> = Document::new();
        DragPreviewHelper::show_floating(
            model,
            PixelPoint::default(),
            PixelPoint::default(),
            true,
            Some(Size::new(150.0, f64::NAN)),
        );
        // The source's premeasure continuation replaces a one-axis preferred outer size.
        assert_eq!((window.width(), window.height()), (80.0, 40.0));
        assert_eq!(
            (
                control.preview_content_width(),
                control.preview_content_height()
            ),
            (300.0, 400.0)
        );
        assert_eq!(window.size_to_content(), SizeToContent::MANUAL);
    });
}
#[test]
fn size_freeze_retries_are_bounded_for_real_visible_zero_size_window() {
    run(|| {
        let (window, control) = install_window();
        window.set_width(0.0);
        window.set_height(0.0);
        window.show();
        window.update_layout();
        assert!(window.is_visible());
        assert_eq!(window.bounds().size(), Size::new(0.0, 0.0));
        DragPreviewHelper::freeze_window_size_if_needed();
        Dispatcher::ui_thread().run_jobs(None);
        let state = DragPreviewHelper::state();
        assert_eq!(
            state.window_size_freeze_retry_count.get(),
            MAX_WINDOW_SIZE_FREEZE_RETRIES
        );
        assert!(state.window_size_freeze_abandoned.get());
        assert!(!state.window_size_freeze_scheduled.get());
        assert!(!state.window_size_frozen.get());
        DragPreviewHelper::apply_window_move(
            &window,
            &control,
            PixelPoint::new(10, 20),
            Some("Float"),
        );
        assert!(!state.window_size_freeze_scheduled.get());
    });
}
#[test]
fn source_managed_preview_delegates_actual_document_build_and_preserves_chrome_overage_adjustment()
{
    run(|| {
        let content = Border::new();
        let _content_scene = Scene::new(content.clone().upcast());
        let parent = content.parent();
        let window = DockWindow::new();
        let document = ManagedDockWindowDocument::new(window.clone());
        document.set_content(Some(Rc::new(content.clone())));
        document.set_mdi_bounds(DockRect::new(0.0, 0.0, 100.0, 100.0));
        let model: Rc<dyn IDockable> = document.clone();
        DragPreviewContext::set_dockable(Some(model.clone()));
        DragPreviewContext::set_is_active(true);
        assert!(DragPreviewHelper::build_managed_preview_content(model.clone(), false).is_none());
        let proxy = DragPreviewHelper::build_managed_preview_content(model.clone(), true)
            .unwrap()
            .cast::<Border>()
            .unwrap();
        assert_ne!(proxy, content);
        assert_eq!(content.parent(), parent);
        let control = DragPreviewControl::new();
        control.set_template(Some(FuncControlTemplate::for_type::<DragPreviewControl>(
            |_, _| {
                let chrome = Border::new();
                chrome.set_width(100.0);
                chrome.set_height(140.0);
                chrome.upcast()
            },
        )));
        let state = DragPreviewHelper::state();
        state.managed_control.replace(Some(control.clone()));
        let (layer, _, _scene) = layer();
        DragPreviewHelper::show_managed(
            &layer,
            model,
            PixelPoint::default(),
            PixelPoint::default(),
            true,
            None,
        );
        assert_eq!(control.preview_content_height(), 60.0);
        assert_eq!(control.height(), 140.0);
        assert_eq!(content.parent(), parent);
        document.dispose();
    });
}

#[test]
fn pinned_original_headless_coordinate_reference_matches_native_roots_and_fallback_at_both_scales()
{
    run(|| {
        for line in include_str!("../../../../tests/Dock/data/preview-root.tsv")
            .lines()
            .skip(1)
        {
            let fields: Vec<_> = line.split('\t').collect();
            let scale: f64 = fields[1].parse().unwrap();
            let expected = Point::new(fields[3].parse().unwrap(), fields[4].parse().unwrap());
            if fields[0] == "detached" {
                let detached = ManagedWindowLayer::new();
                assert!(detached.visual_root().is_none());
                assert_eq!(
                    DragPreviewHelper::get_managed_position(
                        &detached,
                        PixelPoint::new(44, 55),
                        PixelPoint::new(3, -5)
                    ),
                    expected
                );
                continue;
            }
            let (layer, _, scene) = layer();
            layer.set_width(300.0);
            layer.set_height(200.0);
            layer.set_margin(ferroui_base::Thickness::new(17.0, 23.0, 0.0, 0.0));
            layer.set_horizontal_alignment(ferroui_base::layout::HorizontalAlignment::Left);
            layer.set_vertical_alignment(ferroui_base::layout::VerticalAlignment::Top);
            scene.0.set_render_scaling(scale);
            scene.0.update_layout();
            if fields[0] == "hidden" {
                scene.0.hide();
            }
            assert_eq!(scene.0.render_scaling(), scale);
            assert_eq!(layer.visual_root().unwrap().get_type().name(), fields[2]);
            let pointer = layer.point_to_screen(Point::new(41.0, 52.0));
            assert_eq!(
                DragPreviewHelper::get_managed_position(&layer, pointer, PixelPoint::new(3, -5)),
                expected
            );
        }
    });
}
