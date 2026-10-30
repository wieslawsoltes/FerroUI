// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Original MDI input, state, layout and automation using native Headless controls.
use super::*;
use crate::internal::managed_host_window_access::ManagedHostWindowAccess;
use ferrodock_model_reactiveui::{
    controls::{Document, DocumentDock, RootDock},
    core::DockWindow,
    Factory,
};
use ferroui_base::{
    input::{
        KeyModifiers, MouseButton, Pointer, PointerPointProperties, PointerType, PointerUpdateKind,
        RawInputModifiers,
    },
    threading::{CancellationToken, Dispatcher},
    PixelPoint,
};
use ferroui_controls::{
    automation::{
        peers::{AutomationControlType, ControlAutomationPeer},
        provider::IInvokeProvider,
        AutomationProperties,
    },
    templates::FuncControlTemplate,
    AppBuilder, Application, Border, Grid, Window,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use ferroui_system::component_model::INotifyPropertyChanged;
use std::any::TypeId;
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "mdi-document-window",
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
struct Scene {
    window: Ref<Window>,
    panel: Ref<MdiLayoutPanel>,
    control: Ref<MdiDocumentWindow>,
    header: Ref<Border>,
    content: Ref<Border>,
    drag_handle: Ref<Border>,
    minimize: Ref<Button>,
    maximize: Ref<Button>,
    resize: Ref<Border>,
}
impl Scene {
    fn new(document: Rc<dyn IMdiDocument>) -> Self {
        let control = MdiDocumentWindow::new();
        control.set_data_context(Some(model_projection::to_mdi_document_value(document)));
        let panel = MdiLayoutPanel::new();
        panel.children().add(control.clone().upcast::<Control>());
        let (header, content, drag_handle, minimize, maximize, resize) = (
            Border::new(),
            Border::new(),
            Border::new(),
            Button::new(),
            Button::new(),
            Border::new(),
        );
        let parts = (
            header.clone(),
            content.clone(),
            drag_handle.clone(),
            minimize.clone(),
            maximize.clone(),
            resize.clone(),
        );
        control.set_template(Some(FuncControlTemplate::for_type::<MdiDocumentWindow>(
            move |_, scope| {
                let grid = Grid::new();
                scope.register("PART_Header", parts.0.clone().upcast());
                scope.register("PART_ContentBorder", parts.1.clone().upcast());
                scope.register("PART_DragHandle", parts.2.clone().upcast());
                scope.register("PART_MinimizeButton", parts.3.clone().upcast());
                scope.register("PART_MaximizeRestoreButton", parts.4.clone().upcast());
                scope.register("PART_ResizeBottomRight", parts.5.clone().upcast());
                for c in [
                    parts.0.clone().upcast::<Control>(),
                    parts.1.clone().upcast(),
                    parts.2.clone().upcast(),
                    parts.3.clone().upcast(),
                    parts.4.clone().upcast(),
                    parts.5.clone().upcast(),
                ] {
                    grid.children().add(c);
                }
                grid.upcast()
            },
        )));
        let window = Window::new();
        window.set_width(800.0);
        window.set_height(600.0);
        window.set_content(Some(Control::boxed(panel.clone())));
        window.show();
        control.apply_template();
        window.update_layout();
        Self {
            window,
            panel,
            control,
            header,
            content,
            drag_handle,
            minimize,
            maximize,
            resize,
        }
    }
    fn point(&self, local: Point) -> Point {
        self.panel
            .translate_point(local, &self.panel.visual_root().unwrap())
            .unwrap()
    }
    fn press(
        &self,
        source: &Control,
        pointer: Rc<dyn IPointer>,
        position: Point,
        left: bool,
    ) -> PointerPressedEventArgs {
        let e = PointerPressedEventArgs::new(
            source.to_ref(),
            pointer,
            &self.panel.visual_root().unwrap(),
            self.point(position),
            0,
            PointerPointProperties::new(
                if left {
                    RawInputModifiers::LEFT_MOUSE_BUTTON
                } else {
                    RawInputModifiers::RIGHT_MOUSE_BUTTON
                },
                if left {
                    PointerUpdateKind::LeftButtonPressed
                } else {
                    PointerUpdateKind::RightButtonPressed
                },
            ),
            KeyModifiers::NONE,
            1,
        );
        source.raise_event(&e);
        e
    }
    fn moved(&self, pointer: Rc<dyn IPointer>, position: Point) -> PointerEventArgs {
        let e = PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            self.control.clone(),
            pointer,
            Some(&self.panel.visual_root().unwrap()),
            self.point(position),
            1,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::Other,
            ),
            KeyModifiers::NONE,
        );
        self.control.raise_event(&e);
        e
    }
    fn released(&self, pointer: Rc<dyn IPointer>, position: Point) -> PointerReleasedEventArgs {
        let e = PointerReleasedEventArgs::new(
            self.control.clone(),
            pointer,
            &self.panel.visual_root().unwrap(),
            self.point(position),
            2,
            PointerPointProperties::new(
                RawInputModifiers::NONE,
                PointerUpdateKind::LeftButtonReleased,
            ),
            KeyModifiers::NONE,
            MouseButton::Left,
        );
        self.control.raise_event(&e);
        e
    }
}
impl Drop for Scene {
    fn drop(&mut self) {
        self.window.close();
    }
}
fn document() -> Rc<Document> {
    let d = Document::new();
    d.set_title("Original document".into());
    d.set_mdi_bounds(DockRect::new(20.0, 30.0, 280.0, 220.0));
    d
}
fn context() -> (Rc<Factory>, Rc<RootDock>, Rc<DocumentDock>, Rc<Document>) {
    let f = Factory::new();
    let root = RootDock::new();
    let dock = DocumentDock::new();
    let d = document();
    root.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(dock.clone())])));
    dock.set_visible_dockables(Some(ListHandle::from_plain(vec![Some(d.clone())])));
    f.init_layout(root.clone());
    dock.set_active_dockable(Some(d.clone()));
    (f, root, dock, d)
}
fn click(button: &Button) {
    let e = RoutedEventArgs::new();
    e.set_routed_event(Some(Button::click_event()));
    e.set_source(button.to_ref());
    button.raise_event(&e);
}
#[test]
fn original_properties_defaults_pseudoclasses_and_native_registration() {
    run(|| {
        assert_eq!(EventType::default(), EventType::Pressed);
        for (index, phase) in [
            EventType::Pressed,
            EventType::Released,
            EventType::Moved,
            EventType::Enter,
            EventType::Leave,
            EventType::CaptureLost,
            EventType::WheelChanged,
        ]
        .into_iter()
        .enumerate()
        {
            assert_eq!(phase as i32, index as i32);
        }
        let w = MdiDocumentWindow::new();
        assert!(!w.is_active());
        assert_eq!(w.mdi_state(), MdiWindowState::Normal);
        assert!(
            w.icon_template().is_none()
                && w.header_template().is_none()
                && w.modified_template().is_none()
                && w.close_template().is_none()
                && w.close_button_theme().is_none()
                && w.document_context_menu().is_none()
        );
        assert_eq!(w.get_type().namespace(), "FerroDock.Ferro.Controls");
        let icon: BoxedValue = Rc::new("icon".to_owned());
        w.set_icon_template(Some(icon.clone()));
        assert!(Rc::ptr_eq(&w.icon_template().unwrap(), &icon));
        let template = ferroui_controls::templates::FuncDataTemplate::new(
            |_| true,
            |_, _| Some(Border::new().upcast()),
            true,
        );
        w.set_header_template(Some(template.clone()));
        w.set_modified_template(Some(template.clone()));
        w.set_close_template(Some(template.clone()));
        assert!(Rc::ptr_eq(
            &w.header_template().unwrap(),
            &(template.clone() as Rc<dyn IDataTemplate>)
        ));
        assert!(Rc::ptr_eq(
            &w.modified_template().unwrap(),
            &(template.clone() as Rc<dyn IDataTemplate>)
        ));
        assert!(Rc::ptr_eq(
            &w.close_template().unwrap(),
            &(template as Rc<dyn IDataTemplate>)
        ));
        let theme = ControlTheme::new();
        w.set_close_button_theme(Some(theme.clone()));
        assert_eq!(w.close_button_theme(), Some(theme));
        let menu = ContextMenu::new();
        w.set_document_context_menu(Some(menu.clone()));
        assert_eq!(w.document_context_menu(), Some(menu));
        w.set_is_active(true);
        w.set_mdi_state(MdiWindowState::Maximized);
        assert!(w.pseudo_classes().contains(":active"));
        assert!(w.pseudo_classes().contains(":maximized"));
        w.set_mdi_state(MdiWindowState::Minimized);
        assert!(w.pseudo_classes().contains(":minimized"));
        assert!(!w.pseudo_classes().contains(":maximized"));
        assert_eq!(w.last_non_minimized_state.get(), MdiWindowState::Maximized);
        w.set_is_active(false);
        assert!(!w.pseudo_classes().contains(":active"));
        assert_eq!(w.try_get_content_offset(), None);
    });
}
#[test]
fn source_document_and_dock_subscriptions_detach_rebind_and_invalidate_layout() {
    run(|| {
        let (f, _root, dock, d) = context();
        let scene = Scene::new(d.clone());
        assert!(scene.control.is_active());
        dock.set_active_dockable(None);
        assert!(!scene.control.is_active());
        dock.set_active_dockable(Some(d.clone()));
        assert!(scene.control.is_active());
        scene.window.update_layout();
        assert!(scene.panel.is_arrange_valid());
        d.set_mdi_z_index(30);
        assert!(!scene.panel.is_arrange_valid());
        scene.window.update_layout();
        d.property_changed().raise("");
        assert!(!scene.panel.is_arrange_valid());
        scene.window.update_layout();
        d.set_title("New".into());
        assert!(scene.panel.is_arrange_valid());
        let replacement = document();
        replacement.set_mdi_state(MdiWindowState::Minimized);
        scene
            .control
            .set_data_context(Some(model_projection::to_dockable_value(
                replacement.clone(),
            )));
        assert_eq!(
            scene.control.last_non_minimized_state.get(),
            MdiWindowState::Normal
        );
        assert!(!scene.control.is_active());
        scene.window.update_layout();
        d.set_mdi_bounds(DockRect::new(0.0, 0.0, 300.0, 300.0));
        assert!(scene.panel.is_arrange_valid());
        scene.control.set_data_context(None);
        assert!(scene.control.document().is_none());
        assert!(scene.control.dock().is_none());
        scene.window.close();
        drop(f);
    });
}
#[test]
fn detach_releases_original_notifications_while_models_factory_and_window_stay_alive() {
    run(|| {
        let (f, root, dock, d) = context();
        let doc_count = d.property_changed().handler_count();
        let dock_count = dock.property_changed().handler_count();
        let c = MdiDocumentWindow::new();
        c.set_data_context(Some(model_projection::to_mdi_document_value(d.clone())));
        let window = Window::new();
        window.set_content(Some(Control::boxed(c.clone())));
        window.show();
        assert_eq!(d.property_changed().handler_count(), doc_count + 1);
        assert_eq!(dock.property_changed().handler_count(), dock_count + 1);
        let weak = c.downgrade();
        window.set_content(None);
        assert!(c.document().is_none());
        assert!(c.data_context_subscription.borrow().is_none());
        assert_eq!(d.property_changed().handler_count(), doc_count);
        assert_eq!(dock.property_changed().handler_count(), dock_count);
        drop(c);
        Dispatcher::ui_thread().run_jobs(None);
        window.update_layout();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(weak.upgrade().is_none());
        window.close();
        drop((f, root, dock, d));
    });
}
#[test]
fn source_buttons_restore_last_nonminimized_state_and_doubletap_toggles() {
    run(|| {
        let d = document();
        d.set_mdi_state(MdiWindowState::Maximized);
        let s = Scene::new(d.clone());
        click(&s.minimize);
        assert_eq!(d.mdi_state(), MdiWindowState::Minimized);
        click(&s.minimize);
        assert_eq!(d.mdi_state(), MdiWindowState::Maximized);
        click(&s.maximize);
        assert_eq!(d.mdi_state(), MdiWindowState::Normal);
        click(&s.maximize);
        assert_eq!(d.mdi_state(), MdiWindowState::Maximized);
        let pointer = Pointer::new(10, PointerType::Mouse, true);
        let args = PointerEventArgs::new(
            Some(InputElement::pointer_released_event()),
            s.header.clone(),
            pointer,
            Some(&s.panel.visual_root().unwrap()),
            Point::new(40.0, 50.0),
            0,
            PointerPointProperties::new(
                RawInputModifiers::NONE,
                PointerUpdateKind::LeftButtonReleased,
            ),
            KeyModifiers::NONE,
        );
        let e = ferroui_base::input::TappedEventArgs::new(
            Some(InputElement::double_tapped_event()),
            &args,
        );
        e.set_source(s.header.clone());
        s.header.raise_event(&e);
        // DoubleTapped itself bubbles; the source also requests the tunnel route.
        assert_eq!(d.mdi_state(), MdiWindowState::Normal);
        d.set_mdi_state(MdiWindowState::Minimized);
        click(&s.maximize);
        assert_eq!(d.mdi_state(), MdiWindowState::Normal);
        s.control.set_data_context(None);
        click(&s.minimize);
        click(&s.maximize);
        assert_eq!(d.mdi_state(), MdiWindowState::Normal);
    });
}
#[test]
fn source_header_and_content_input_activate_via_factory_and_update_zorder() {
    run(|| {
        let (_f, root, dock, d) = context();
        let s = Scene::new(d.clone());
        dock.set_active_dockable(None);
        let pointer = Pointer::new(1, PointerType::Mouse, true);
        s.press(&s.content, pointer.clone(), Point::new(40.0, 50.0), false);
        assert!(dock.active_dockable().is_none());
        s.press(&s.content, pointer.clone(), Point::new(40.0, 50.0), true);
        assert!(Rc::ptr_eq(
            &dock.active_dockable().unwrap(),
            &(d.clone() as Rc<dyn IDockable>)
        ));
        assert!(Rc::ptr_eq(
            &root.focused_dockable().unwrap(),
            &(d.clone() as Rc<dyn IDockable>)
        ));
        let other = document();
        other.set_mdi_z_index(100);
        let child = Border::new();
        child.set_data_context(Some(model_projection::to_mdi_document_value(other.clone())));
        s.panel.children().add(child.upcast::<Control>());
        s.press(&s.header, pointer.clone(), Point::new(40.0, 50.0), true);
        assert!(s.control.is_dragging.get());
        assert!(d.mdi_z_index() > other.mdi_z_index());
        assert_eq!(pointer.captured(), Some(s.control.clone().upcast()));
        s.released(pointer.clone(), Point::new(40.0, 50.0));
        assert!(!s.control.is_dragging.get());
        assert!(pointer.captured().is_none());
    });
}
#[test]
fn begin_drag_preserves_source_enabled_capability_state_and_exclusion_guards() {
    run(|| {
        let d = document();
        let s = Scene::new(d.clone());
        let p = Pointer::new(2, PointerType::Mouse, true);
        DockProperties::set_is_drag_enabled(&s.control, false);
        let e = s.press(&s.header, p.clone(), Point::new(40.0, 50.0), true);
        assert!(!e.handled());
        assert!(!s.control.is_dragging.get());
        DockProperties::set_is_drag_enabled(&s.control, true);
        d.set_can_drag(false);
        s.press(&s.header, p.clone(), Point::new(40.0, 50.0), true);
        assert!(!s.control.is_dragging.get());
        d.set_can_drag(true);
        d.set_mdi_state(MdiWindowState::Maximized);
        s.press(&s.header, p.clone(), Point::new(40.0, 50.0), true);
        assert!(!s.control.is_dragging.get());
        d.set_mdi_state(MdiWindowState::Normal);
        let button = Button::new();
        s.header.set_child(Some(button.clone().upcast()));
        let e = s.press(&button, p.clone(), Point::new(40.0, 50.0), true);
        assert!(!s.control.is_dragging.get());
        assert!(e.handled()); // Native Button handles its own press.
        let inside = Border::new();
        button.set_content(Some(Control::boxed(inside.clone())));
        assert!(WindowDragHelper::is_child_of_type::<Button>(
            &s.header, &inside
        ));
        assert!(!WindowDragHelper::is_child_of_type::<Border>(
            &s.header, &s.header
        ));
        assert!(is_descendant_of(&s.header, Some(&inside)));
        assert!(!is_descendant_of(&s.header, None));
        s.header.set_child(None);
        s.drag_handle
            .parent()
            .unwrap()
            .cast::<Grid>()
            .unwrap()
            .children()
            .remove(s.drag_handle.clone().upcast::<Control>());
        s.header.set_child(Some(s.drag_handle.clone().upcast()));
        s.press(&s.drag_handle, p.clone(), Point::new(40.0, 50.0), true);
        assert!(!s.control.is_dragging.get());
        s.header.set_child(None);
        s.control.set_data_context(None);
        let e = s.press(&s.header, p, Point::new(40.0, 50.0), true);
        assert!(!e.handled());
    });
}
#[test]
fn native_drag_resize_capture_release_and_state_change_follow_source_layout_manager() {
    run(|| {
        let d = document();
        let s = Scene::new(d.clone());
        let p = Pointer::new(3, PointerType::Mouse, true);
        let e = s.press(&s.header, p.clone(), Point::new(40.0, 50.0), true);
        assert!(e.handled());
        assert_eq!(s.control.drag_start_point.get(), Point::new(40.0, 50.0));
        let e = s.moved(p.clone(), Point::new(75.0, 80.0));
        assert!(e.handled());
        assert_eq!(d.mdi_bounds(), DockRect::new(55.0, 60.0, 280.0, 220.0));
        d.set_mdi_state(MdiWindowState::Maximized);
        let e = s.moved(p.clone(), Point::new(100.0, 100.0));
        assert!(!e.handled());
        assert_eq!(d.mdi_bounds(), DockRect::new(55.0, 60.0, 280.0, 220.0));
        s.released(p.clone(), Point::new(100.0, 100.0));
        d.set_mdi_state(MdiWindowState::Normal);
        let e = s.press(&s.resize, p.clone(), Point::new(75.0, 80.0), true);
        assert!(e.handled());
        assert!(s.control.is_resizing.get());
        let e = s.moved(p.clone(), Point::new(105.0, 100.0));
        assert!(e.handled());
        assert_eq!(d.mdi_bounds(), DockRect::new(55.0, 60.0, 310.0, 240.0));
        p.capture(None);
        assert!(s.control.is_resizing.get()); // Original tunnel subscription does not observe the direct event.
        s.control
            .pointer_capture_lost_handler(&PointerCaptureLostEventArgs::new(
                s.control.clone(),
                p.clone(),
            ));
        assert!(!s.control.is_resizing.get());
        assert_eq!(s.control.resize_direction.get(), MdiResizeDirection::NONE);
    });
}
#[test]
fn original_magnetism_uses_adjacent_normal_valid_windows_and_last_matching_edge() {
    run(|| {
        let previous = DockSettings::window_magnet_distance();
        DockSettings::set_window_magnet_distance(10.0);
        let current: Rc<dyn IMdiDocument> = document();
        let other = document();
        other.set_mdi_bounds(DockRect::new(150.0, 0.0, 50.0, 50.0));
        let entries = ListHandle::from_plain(vec![
            MdiLayoutEntry::new(Control::new(), current.clone()),
            MdiLayoutEntry::new(Control::new(), other.clone()),
        ]);
        assert_eq!(
            apply_window_magnetism(Rect::new(95.0, 0.0, 50.0, 50.0), &current, &entries).x,
            100.0
        );
        assert_eq!(
            apply_window_magnetism(Rect::new(205.0, 0.0, 50.0, 50.0), &current, &entries).x,
            200.0
        );
        assert_eq!(
            apply_window_magnetism(Rect::new(150.0, 55.0, 50.0, 50.0), &current, &entries).y,
            50.0
        );
        assert_eq!(
            apply_window_magnetism(Rect::new(150.0, -55.0, 50.0, 50.0), &current, &entries).y,
            -50.0
        );
        let unchanged = Rect::new(95.0, 50.0, 50.0, 50.0);
        assert_eq!(
            apply_window_magnetism(unchanged, &current, &entries),
            unchanged
        );
        other.set_mdi_state(MdiWindowState::Maximized);
        assert_eq!(
            apply_window_magnetism(Rect::new(95.0, 0.0, 50.0, 50.0), &current, &entries).x,
            95.0
        );
        other.set_mdi_state(MdiWindowState::Normal);
        for width in [f64::NAN, 0.0, -1.0] {
            other.set_mdi_bounds(DockRect::new(150.0, 0.0, width, 50.0));
            assert_eq!(
                apply_window_magnetism(Rect::new(95.0, 0.0, 50.0, 50.0), &current, &entries).x,
                95.0
            );
        }
        DockSettings::set_window_magnet_distance(0.0);
        assert_eq!(
            apply_window_magnetism(unchanged, &current, &entries),
            unchanged
        );
        DockSettings::set_window_magnet_distance(previous);
    });
}
#[test]
fn original_minimized_strip_clamp_and_fallback_manager_are_preserved() {
    run(|| {
        let d: Rc<dyn IMdiDocument> = document();
        let minimized = document();
        minimized.set_mdi_state(MdiWindowState::Minimized);
        let entries = ListHandle::from_plain(vec![MdiLayoutEntry::new(Control::new(), minimized)]);
        assert_eq!(
            get_available_size(&entries, Size::new(800.0, 600.0)),
            Size::new(800.0, 568.0)
        );
        assert_eq!(
            clamp_to_available_size(
                Rect::new(900.0, 900.0, 280.0, 220.0),
                &entries,
                Size::new(800.0, 600.0)
            ),
            Rect::new(520.0, 348.0, 280.0, 220.0)
        );
        assert_eq!(
            clamp_to_available_size(
                Rect::new(-10.0, -20.0, 900.0, 700.0),
                &entries,
                Size::new(800.0, 600.0)
            ),
            Rect::new(0.0, 0.0, 900.0, 700.0)
        );
        let c = MdiDocumentWindow::new();
        let (_, entries, _) = c.get_layout_manager();
        assert!(entries.is_empty());
        c.on_data_context_changed(Some(model_projection::to_mdi_document_value(d)));
        let (_, entries, _) = c.get_layout_manager();
        assert_eq!(entries.count(), 1);
        assert_eq!(entries.get(0).control(), c.clone().upcast::<Control>());
        c.detach_dock_subscription();
    });
}
#[test]
fn source_automation_invoke_activates_original_document_and_exposes_live_window_state() {
    run(|| {
        let (_f, _root, dock, d) = context();
        d.set_mdi_state(MdiWindowState::Maximized);
        let s = Scene::new(d.clone());
        dock.set_active_dockable(None);
        let peer = ControlAutomationPeer::create_peer_for_element(&s.control);
        assert_eq!(peer.get_class_name(), "MdiDocumentWindow");
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::Window
        );
        assert_eq!(peer.get_name(), "Original document");
        assert!(peer.get_help_text().contains("MdiState=Maximized"));
        d.set_id("original-id".into());
        assert_eq!(peer.get_automation_id().as_deref(), Some("original-id"));
        AutomationProperties::set_name(&s.control, Some("Explicit"));
        assert_eq!(peer.get_name(), "Explicit");
        let invoke = peer.get_provider::<dyn IInvokeProvider>().unwrap();
        invoke.invoke().unwrap();
        assert!(Rc::ptr_eq(
            &dock.active_dockable().unwrap(),
            &(d.clone() as Rc<dyn IDockable>)
        ));
        d.set_can_drag(false);
        assert!(peer.get_help_text().contains("CanDrag=false"));
        s.control.set_data_context(None);
        s.control.set_focusable(true);
        invoke.invoke().unwrap();
        assert!(s.control.is_focused());
    });
}

// Test-only observation of the original managed-host private members. The
// production concrete managed host and its state engine remain separate ports.
struct HostProbe {
    native: Option<Ref<Window>>,
    z: Cell<i32>,
    name: &'static str,
    events: Rc<RefCell<Vec<String>>>,
    phases: RefCell<Vec<(PixelPoint, EventType)>>,
    window: RefCell<Option<Rc<dyn IDockWindow>>>,
}
impl HostProbe {
    fn new(
        name: &'static str,
        z: i32,
        native: Option<Ref<Window>>,
        events: Rc<RefCell<Vec<String>>>,
    ) -> Rc<Self> {
        Rc::new(Self {
            native,
            z: Cell::new(z),
            name,
            events,
            phases: RefCell::new(Vec::new()),
            window: RefCell::new(None),
        })
    }
}
impl ManagedHostWindowAccess for HostProbe {
    fn managed_z_index(&self) -> i32 {
        self.z.get()
    }
    fn process_drag(&self, point: PixelPoint, phase: EventType) {
        self.phases.borrow_mut().push((point, phase));
    }
    fn last_close_canceled(&self) -> bool {
        false
    }
    fn update_bounds_from_document(&self, _: DockRect) {}
}
impl IHostWindow for HostProbe {
    fn boxed_interface(self: Rc<Self>, target: TypeId) -> Option<ferrodock_model::BoxedValue> {
        if self.native.is_none() && target == TypeId::of::<Rc<dyn ManagedHostWindowAccess>>() {
            let host: Rc<dyn ManagedHostWindowAccess> = self;
            return Some(Rc::new(host));
        }
        if target == TypeId::of::<Ref<Window>>() {
            return self
                .native
                .clone()
                .map(|w| Rc::new(w) as ferrodock_model::BoxedValue);
        }
        None
    }
    fn host_window_state(&self) -> Option<Rc<dyn IHostWindowState>> {
        None
    }
    fn is_tracked(&self) -> bool {
        false
    }
    fn set_is_tracked(&self, _: bool) {}
    fn window(&self) -> Option<Rc<dyn IDockWindow>> {
        self.window.borrow().clone()
    }
    fn set_window(&self, w: Option<Rc<dyn IDockWindow>>) {
        self.window.replace(w);
    }
    fn present(&self, _: bool) {}
    fn exit(&self) {}
    fn set_position(&self, _: f64, _: f64) {}
    fn get_position(&self) -> (f64, f64) {
        (0.0, 0.0)
    }
    fn set_size(&self, _: f64, _: f64) {}
    fn get_size(&self) -> (f64, f64) {
        (0.0, 0.0)
    }
    fn set_window_state(&self, _: DockWindowState) {}
    fn get_window_state(&self) -> DockWindowState {
        DockWindowState::Normal
    }
    fn set_title(&self, _: Option<String>) {}
    fn set_layout(&self, _: Rc<dyn IDock>) {}
    fn set_active(&self) {
        self.events.borrow_mut().push(self.name.into());
    }
}
#[test]
fn original_managed_drag_cancellation_notifications_and_screen_dispatch_follow_factory() {
    run(|| {
        let factory = Factory::new();
        let window = DockWindow::new();
        window.set_factory(Some(factory.clone()));
        let host = HostProbe::new("managed", 1, None, Rc::new(RefCell::new(Vec::new())));
        window.set_host(Some(host.clone()));
        let doc = ManagedDockWindowDocument::new(window.clone());
        doc.set_mdi_bounds(DockRect::new(20.0, 30.0, 280.0, 220.0));
        let s = Scene::new(doc.clone());
        let pointer = Pointer::new(4, PointerType::Mouse, true);
        let log = Rc::new(RefCell::new(Vec::new()));
        let cancel = Rc::new(Cell::new(true));
        let events = log.clone();
        let canceled = cancel.clone();
        let _begin = factory.window_move_drag_begin(Rc::new(move |e| {
            events.borrow_mut().push("begin");
            e.set_cancel(canceled.get());
        }));
        let events = log.clone();
        let _move = factory.window_move_drag(Rc::new(move |_| events.borrow_mut().push("move")));
        let events = log.clone();
        let _end = factory.window_move_drag_end(Rc::new(move |_| events.borrow_mut().push("end")));
        let e = s.press(&s.header, pointer.clone(), Point::new(40.0, 50.0), true);
        assert!(!e.handled());
        assert!(!s.control.is_dragging.get());
        assert!(host.phases.borrow().is_empty());
        assert_eq!(&*log.borrow(), &["begin"]);
        cancel.set(false);
        log.borrow_mut().clear();
        let old = DockSettings::bring_windows_to_front_on_drag();
        DockSettings::set_bring_windows_to_front_on_drag(false);
        let e = s.press(&s.header, pointer.clone(), Point::new(40.0, 50.0), true);
        assert!(e.handled());
        s.moved(pointer.clone(), Point::new(50.0, 60.0));
        s.released(pointer.clone(), Point::new(55.0, 65.0));
        assert_eq!(&*log.borrow(), &["begin", "move", "end"]);
        let phases = host.phases.borrow();
        assert_eq!(
            phases.iter().map(|(_, p)| *p).collect::<Vec<_>>(),
            vec![EventType::Pressed, EventType::Moved, EventType::Released]
        );
        assert_eq!(phases[0].0, s.panel.point_to_screen(Point::new(40.0, 50.0)));
        drop(phases);
        log.borrow_mut().clear();
        s.press(&s.header, pointer.clone(), Point::new(40.0, 50.0), true);
        pointer.capture(None);
        assert!(s.control.is_dragging.get());
        s.control
            .pointer_capture_lost_handler(&PointerCaptureLostEventArgs::new(
                s.control.clone(),
                pointer.clone(),
            ));
        assert_eq!(&*log.borrow(), &["begin", "end"]);
        assert_eq!(
            host.phases.borrow().last().unwrap().1,
            EventType::CaptureLost
        );
        assert!(!s.control.managed_window_drag_active.get());
        assert!(s.control.managed_drag_window.borrow().is_none());
        DockSettings::set_bring_windows_to_front_on_drag(old);
        s.control.set_data_context(None);
        doc.dispose();
        window.set_host(None);
    });
}
#[test]
fn managed_drag_without_window_factory_and_changed_factory_keep_original_notifications() {
    run(|| {
        let window = DockWindow::new();
        let doc = ManagedDockWindowDocument::new(window.clone());
        let c = MdiDocumentWindow::new();
        c.on_data_context_changed(Some(model_projection::to_mdi_document_value(doc.clone())));
        assert!(c.try_begin_managed_window_drag());
        assert!(!c.managed_window_drag_active.get());
        let first = Factory::new();
        let second = Factory::new();
        window.set_factory(Some(first.clone()));
        let log = Rc::new(RefCell::new(Vec::new()));
        let events = log.clone();
        let _move =
            second.window_move_drag(Rc::new(move |_| events.borrow_mut().push("second move")));
        let events = log.clone();
        let _end =
            second.window_move_drag_end(Rc::new(move |_| events.borrow_mut().push("second end")));
        let old = DockSettings::bring_windows_to_front_on_drag();
        DockSettings::set_bring_windows_to_front_on_drag(false);
        assert!(c.try_begin_managed_window_drag());
        window.set_factory(Some(second.clone()));
        c.notify_managed_window_drag();
        c.notify_managed_window_drag_end();
        assert_eq!(&*log.borrow(), &["second move", "second end"]);
        assert!(c.try_begin_managed_window_drag());
        window.set_factory(None);
        c.notify_managed_window_drag();
        c.notify_managed_window_drag_end();
        assert!(!c.managed_window_drag_active.get());
        DockSettings::set_bring_windows_to_front_on_drag(old);
        c.detach_dock_subscription();
        doc.dispose();
    });
}
#[test]
fn window_activation_preserves_hidden_native_window_guard_and_stable_managed_zorder() {
    run(|| {
        for hidden_initiator in [false, true] {
            let factory = Factory::new();
            let hidden = Window::new();
            let visible = Window::new();
            visible.show();
            hidden.show();
            hidden.hide();
            Dispatcher::ui_thread().run_jobs(None);
            let activations = Rc::new(Cell::new(0));
            let n = activations.clone();
            let _sub = hidden.activated(move || n.set(n.get() + 1));
            let log = Rc::new(RefCell::new(Vec::new()));
            let first = HostProbe::new("native", 0, Some(hidden.clone()), log.clone());
            let high = HostProbe::new("high", 9, None, log.clone());
            let second = HostProbe::new("low", 1, None, log.clone());
            let third = HostProbe::new("equal", 1, None, log.clone());
            factory.host_windows().add(first.clone());
            factory.host_windows().add(high);
            factory.host_windows().add(second);
            factory.host_windows().add(third);
            WindowActivationHelper::activate_all_windows(
                &*factory,
                Some(if hidden_initiator { &hidden } else { &visible }),
            );
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(activations.get(), 0);
            assert!(!hidden.is_visible());
            assert_eq!(&*log.borrow(), &["low", "equal", "high"]);
            hidden.show();
            Dispatcher::ui_thread().run_jobs(None);
            activations.set(0);
            WindowActivationHelper::activate_all_windows(
                &*factory,
                Some(if hidden_initiator { &hidden } else { &visible }),
            );
            Dispatcher::ui_thread().run_jobs(None);
            assert_eq!(activations.get(), 1);
            hidden.close();
            visible.close();
            factory.host_windows().clear();
        }
    });
}

#[test]
fn all_eight_source_resize_handles_route_their_original_direction() {
    run(|| {
        for (name, direction) in [
            ("Top", MdiResizeDirection::TOP),
            ("Bottom", MdiResizeDirection::BOTTOM),
            ("Left", MdiResizeDirection::LEFT),
            ("Right", MdiResizeDirection::RIGHT),
            (
                "TopLeft",
                MdiResizeDirection::TOP | MdiResizeDirection::LEFT,
            ),
            (
                "TopRight",
                MdiResizeDirection::TOP | MdiResizeDirection::RIGHT,
            ),
            (
                "BottomLeft",
                MdiResizeDirection::BOTTOM | MdiResizeDirection::LEFT,
            ),
            (
                "BottomRight",
                MdiResizeDirection::BOTTOM | MdiResizeDirection::RIGHT,
            ),
        ] {
            let d = document();
            let scene = Scene::new(d.clone());
            let handle = Border::new();
            let part_name = format!("PART_Resize{name}");
            let template_handle = handle.clone();
            scene
                .control
                .set_template(Some(FuncControlTemplate::for_type::<MdiDocumentWindow>(
                    move |_, scope| {
                        scope.register(&part_name, template_handle.clone().upcast());
                        template_handle.clone().upcast()
                    },
                )));
            scene.control.apply_template();
            scene.window.update_layout();
            let pointer = Pointer::new(11, PointerType::Mouse, true);
            let right = scene.press(&handle, pointer.clone(), Point::new(40.0, 50.0), false);
            assert!(!right.handled());
            assert!(!scene.control.is_resizing.get());
            let e = scene.press(&handle, pointer.clone(), Point::new(40.0, 50.0), true);
            assert!(e.handled());
            assert_eq!(scene.control.resize_direction.get(), direction);
            let (manager, entries, size) = scene.control.get_layout_manager();
            let expected = manager.get_resize_bounds(
                d.clone(),
                scene.control.drag_start_bounds.get(),
                ferroui_base::Vector::new(10.0, 15.0),
                direction,
                size,
                &entries,
            );
            scene.moved(pointer.clone(), Point::new(50.0, 65.0));
            assert_eq!(d.mdi_bounds(), to_dock_rect(expected));
            scene.released(pointer, Point::new(50.0, 65.0));
            assert!(!scene.control.is_resizing.get());
        }
    });
}
#[test]
fn template_replacement_removes_only_original_content_press_handler_and_resolves_content_offset() {
    run(|| {
        let (_factory, _root, dock, d) = context();
        let scene = Scene::new(d.clone());
        assert!(scene.control.try_get_content_offset().is_some());
        let old_content = scene.content.clone();
        let new_content = Border::new();
        let template_content = new_content.clone();
        scene
            .control
            .set_template(Some(FuncControlTemplate::for_type::<MdiDocumentWindow>(
                move |_, scope| {
                    scope.register("PART_ContentBorder", template_content.clone().upcast());
                    template_content.clone().upcast()
                },
            )));
        scene.control.apply_template();
        scene.window.update_layout();
        assert_eq!(
            scene.control.try_get_content_offset(),
            new_content.translate_point(Point::default(), &scene.control)
        );
        dock.set_active_dockable(None);
        let pointer = Pointer::new(12, PointerType::Mouse, true);
        scene.press(&old_content, pointer.clone(), Point::new(40.0, 50.0), true);
        assert!(dock.active_dockable().is_none());
        scene.press(&new_content, pointer, Point::new(40.0, 50.0), true);
        assert!(dock.active_dockable().is_some());
        scene
            .control
            .set_template(Some(FuncControlTemplate::for_type::<MdiDocumentWindow>(
                |_, _| Border::new().upcast(),
            )));
        scene.control.apply_template();
        assert!(scene.control.try_get_content_offset().is_none());
    });
}
struct ManagerProbe {
    calls: RefCell<Vec<String>>,
    drag: Rect,
    resize: Rect,
}
impl IMdiLayoutManager for ManagerProbe {
    fn arrange(&self, _: &ListHandle<MdiLayoutEntry>, _: Size) {}
    fn get_drag_bounds(
        &self,
        _: Rc<dyn IMdiDocument>,
        _: Rect,
        _: ferroui_base::Vector,
        _: Size,
        entries: &ListHandle<MdiLayoutEntry>,
    ) -> Rect {
        self.calls
            .borrow_mut()
            .push(format!("drag {}", entries.count()));
        self.drag
    }
    fn get_resize_bounds(
        &self,
        _: Rc<dyn IMdiDocument>,
        _: Rect,
        _: ferroui_base::Vector,
        direction: MdiResizeDirection,
        _: Size,
        _: &ListHandle<MdiLayoutEntry>,
    ) -> Rect {
        self.calls
            .borrow_mut()
            .push(format!("resize {}", direction.bits()));
        self.resize
    }
    fn update_z_order(
        &self,
        documents: &ListHandle<Rc<dyn IMdiDocument>>,
        active: Option<Rc<dyn IMdiDocument>>,
    ) {
        assert!(active.is_some());
        self.calls
            .borrow_mut()
            .push(format!("z {}", documents.count()));
    }
}
#[test]
fn original_custom_layout_manager_controls_drag_resize_zorder_and_ignores_non_document_children() {
    run(|| {
        let (_factory, _root, _dock, d) = context();
        let scene = Scene::new(d.clone());
        scene.panel.children().add(Border::new());
        let custom = Rc::new(ManagerProbe {
            calls: RefCell::new(Vec::new()),
            drag: Rect::new(123.0, 124.0, 281.0, 221.0),
            resize: Rect::new(123.0, 124.0, 301.0, 241.0),
        });
        scene.panel.set_layout_manager(Some(custom.clone()));
        let old = DockSettings::enable_window_magnetism();
        DockSettings::set_enable_window_magnetism(false);
        let pointer = Pointer::new(13, PointerType::Mouse, true);
        scene.press(&scene.header, pointer.clone(), Point::new(40.0, 50.0), true);
        scene.moved(pointer.clone(), Point::new(60.0, 70.0));
        assert_eq!(d.mdi_bounds(), to_dock_rect(custom.drag));
        scene.released(pointer.clone(), Point::new(60.0, 70.0));
        scene.press(&scene.resize, pointer.clone(), Point::new(40.0, 50.0), true);
        scene.moved(pointer.clone(), Point::new(60.0, 70.0));
        assert_eq!(d.mdi_bounds(), to_dock_rect(custom.resize));
        scene.released(pointer, Point::new(60.0, 70.0));
        assert_eq!(
            &*custom.calls.borrow(),
            &["z 1", "drag 1", "z 1", "resize 12"]
        );
        scene.panel.set_layout_manager(None);
        let (manager, _, _) = scene.control.get_layout_manager();
        let expected: Rc<dyn IMdiLayoutManager> = ClassicMdiLayoutManager::instance();
        assert!(Rc::ptr_eq(&manager, &expected));
        DockSettings::set_enable_window_magnetism(old);
    });
}
