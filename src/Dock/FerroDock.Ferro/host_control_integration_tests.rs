//! Source host-method cases and native class/contract integration on the real controls.
use crate::controls::{
    DockControl, DocumentTabStrip, DocumentTabStripItem, HostWindow, ToolChromeControl,
    ToolPinItemControl, ToolPinnedControl, ToolTabStrip, ToolTabStripItem,
};
use ferrodock_model::{
    core::{DockWindowState, IHostWindowCloseState},
    BoxedValue as ModelValue,
};
use ferrodock_model_reactiveui::{core::DockWindow, Factory};
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    layout::Orientation,
    threading::CancellationToken,
    BoxedValue, Ref,
};
use ferroui_controls::{AppBuilder, Application, Control, Window, WindowState};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::rc::Rc;
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-native-host-stack",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
fn run(test: impl FnOnce() + Send + 'static) {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY))
        .dispatch(
            move || {
                crate::register_types();
                test();
            },
            CancellationToken::none(),
        )
        .wait()
        .unwrap();
}
#[test]
fn source_host_position_and_size_setters_getters_and_window_state() {
    run(|| {
        let host = HostWindow::new();
        host.set_position_xy(100.0, 50.0);
        assert_eq!(host.get_position(), (100.0, 50.0));
        host.set_size(800.0, 600.0);
        assert_eq!(host.get_size(), (800.0, 600.0));
        host.set_dock_window_state(DockWindowState::Maximized);
        assert_eq!(host.get_window_state(), DockWindowState::Maximized);
        host.close();
    });
}
#[test]
fn source_host_ignores_nan_coordinates_and_each_nan_size_dimension() {
    run(|| {
        let host = HostWindow::new();
        host.set_position_xy(f64::NAN, f64::NAN);
        assert_eq!(host.get_position(), (0.0, 0.0));
        host.set_size(f64::NAN, f64::NAN);
        let (w, h) = host.get_size();
        assert!(w.is_nan() && h.is_nan());
        host.set_position_xy(10.0, 20.0);
        host.set_position_xy(200.0, f64::NAN);
        assert_eq!(host.get_position(), (10.0, 20.0));
        host.set_size(300.0, f64::NAN);
        assert_eq!(host.width(), 300.0);
        assert!(host.height().is_nan());
        host.set_size(f64::NAN, 240.0);
        assert_eq!(host.get_size(), (300.0, 240.0));
        host.close();
    });
}
#[test]
fn normal_bounds_preserve_fractional_source_values_for_non_normal_states() {
    run(|| {
        let host = HostWindow::new();
        host.set_position_xy(101.75, 55.25);
        host.set_size(800.5, 600.25);
        assert_eq!(host.get_position(), (101.0, 55.0));
        host.set_window_state(WindowState::Maximized);
        assert_eq!(host.get_position(), (101.75, 55.25));
        assert_eq!(host.get_size(), (800.5, 600.25));
        host.close();
    });
}
#[test]
fn source_reopening_hidden_host_does_not_duplicate_native_factory_registration() {
    run(|| {
        let factory = Factory::new();
        let model = DockWindow::new();
        model.set_factory(Some(factory.clone()));
        let host = HostWindow::new();
        host.set_window(Some(model));
        for _ in 0..3 {
            host.present(false);
            assert_eq!(factory.host_windows().count(), 1);
            let registered = factory.host_windows().get(0);
            let native = registered
                .boxed_interface(std::any::TypeId::of::<Ref<Window>>())
                .unwrap();
            assert!(native.downcast_ref::<Ref<Window>>().unwrap().ptr_eq(&host));
            host.hide();
            assert_eq!(factory.host_windows().count(), 1);
        }
        host.close();
        assert_eq!(factory.host_windows().count(), 0);
    });
}
#[test]
fn native_host_foreign_interfaces_share_receiver_and_report_actual_close_completion() {
    run(|| {
        let host = HostWindow::new();
        let first = HostWindow::as_host_window(host.clone());
        let second = HostWindow::as_host_window(host.clone());
        assert!(Rc::ptr_eq(&first, &second));
        let boxed: BoxedValue = Rc::new(host.clone());
        let close =
            ValueTypes::try_cast(&boxed, ValueType::of::<Rc<dyn IHostWindowCloseState>>()).unwrap();
        let close = close
            .downcast_ref::<Rc<dyn IHostWindowCloseState>>()
            .unwrap();
        assert!(!close.is_closed());
        let boxed: BoxedValue = Rc::new(first.clone());
        assert!(Control::from_boxed(&boxed).unwrap().ptr_eq(&host));
        host.close();
        assert!(close.is_closed());
        assert!(first.as_host_window_close_state().unwrap().is_closed());
    });
}
#[test]
fn main_control_native_contract_context_and_engine_options_use_original_receiver() {
    run(|| {
        let control = DockControl::new();
        let first = DockControl::as_dock_control(control.clone());
        let second = DockControl::as_dock_control(control.clone());
        assert!(Rc::ptr_eq(&first, &second));
        assert!(control.is_docking_enabled());
        assert!(first.dock_manager().is_docking_enabled());
        control.set_is_docking_enabled(false);
        assert!(!first.dock_manager().is_docking_enabled());
        let original: BoxedValue = Rc::new(String::from("native context"));
        control.set_default_context(Some(original.clone()));
        let model: ModelValue = first.default_context().unwrap();
        let restored = ferrodock_controls_recycling::from_recycling_value(&model);
        assert!(Rc::ptr_eq(&restored, &original));
        first.set_default_context(Some(ferrodock_controls_recycling::to_recycling_value(
            original.clone(),
        )));
        assert!(Rc::ptr_eq(&control.default_context().unwrap(), &original));
        let boxed: BoxedValue = Rc::new(first);
        assert!(Control::from_boxed(&boxed).unwrap().ptr_eq(&control));
    });
}
#[test]
fn actual_tab_chrome_and_pinned_classes_keep_source_property_defaults_and_pseudo_classes() {
    run(|| {
        let document = DocumentTabStrip::new();
        assert!(!document.can_create_item());
        assert!(!document.enable_window_drag());
        assert_eq!(document.orientation(), Orientation::Horizontal);
        document.set_can_create_item(true);
        assert!(document.classes().contains(":create"));
        let item = DocumentTabStripItem::new();
        item.set_is_active(true);
        assert!(item.classes().contains(":active"));
        let tool = ToolTabStrip::new();
        tool.set_can_create_item(true);
        assert!(tool.classes().contains(":create"));
        let _ = ToolTabStripItem::new();
        let chrome = ToolChromeControl::new();
        assert!(chrome.title().is_none());
        chrome.set_is_floating(true);
        chrome.set_is_pinned(true);
        assert!(chrome.classes().contains(":floating") && chrome.classes().contains(":pinned"));
        assert_eq!(
            ToolPinItemControl::new().orientation(),
            Orientation::Vertical
        );
        assert_eq!(
            ToolPinnedControl::new().orientation(),
            Orientation::Vertical
        );
    });
}
#[test]
fn source_tab_item_peer_shared_specializations_use_actual_owner_state_and_selection() {
    run(|| {
        use ferroui_controls::automation::{
            peers::{AutomationControlType, ControlAutomationPeer},
            provider::{IInvokeProvider, ISelectionItemProvider},
        };
        let document = DocumentTabStripItem::new();
        let peer = ControlAutomationPeer::create_peer_for_element(&document);
        assert_eq!(peer.get_class_name(), "DocumentTabStripItem");
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::TabItem
        );
        let invoke = peer.get_provider::<dyn IInvokeProvider>().unwrap();
        invoke.invoke().unwrap();
        assert!(document.is_selected());
        let selection = peer.get_provider::<dyn ISelectionItemProvider>().unwrap();
        assert!(selection.is_selected());
        selection.remove_from_selection().unwrap();
        assert!(!document.is_selected());
        let tool = ToolTabStripItem::new();
        let peer = ControlAutomationPeer::create_peer_for_element(&tool);
        assert_eq!(peer.get_class_name(), "ToolTabStripItem");
        peer.get_provider::<dyn IInvokeProvider>()
            .unwrap()
            .invoke()
            .unwrap();
        assert!(tool.is_selected());
    });
}

fn explicit_move_drag(cancel: bool) {
    run(move || {
        use ferrodock_model::core::IFactoryEvents;
        use ferrodock_settings::WindowDragDockScope;
        use ferroui_base::{
            input::{InputElement, MouseButton, RawInputModifiers},
            Point,
        };
        use ferroui_headless::HeadlessWindowExtensions;
        use std::cell::Cell;
        let factory = Factory::new();
        let model = DockWindow::new();
        model.set_factory(Some(factory.clone()));
        let host = HostWindow::new();
        host.set_window(Some(model));
        host.set_size(400.0, 300.0);
        let began = Rc::new(Cell::new(0));
        let ended = Rc::new(Cell::new(0));
        let accepted = Rc::new(Cell::new(None));
        let count = began.clone();
        let begin = factory.window_move_drag_begin(Rc::new(move |args| {
            count.set(count.get() + 1);
            args.set_cancel(cancel);
        }));
        let count = ended.clone();
        let end = factory.window_move_drag_end(Rc::new(move |_| count.set(count.get() + 1)));
        let result = accepted.clone();
        let native = host.clone();
        let pressed = host.pointer_pressed(move |_, args| {
            result.set(Some(
                native.try_begin_move_drag(args, WindowDragDockScope::FullWindow),
            ))
        });
        host.show();
        host.mouse_down(
            Point::new(10.0, 10.0),
            MouseButton::Left,
            RawInputModifiers::NONE,
        );
        assert_eq!(accepted.get(), Some(!cancel));
        assert_eq!(began.get(), 1);
        assert_eq!(host.classes().contains(":dragging"), !cancel);
        host.mouse_up(
            Point::new(10.0, 10.0),
            MouseButton::Left,
            RawInputModifiers::NONE,
        );
        assert!(!host.classes().contains(":dragging"));
        assert_eq!(ended.get(), if cancel { 0 } else { 1 });
        host.remove_handler(InputElement::pointer_pressed_event(), pressed);
        host.close();
        begin.dispose();
        end.dispose();
    });
}
#[test]
fn original_explicit_move_drag_accepts_and_ends_once_on_pointer_release() {
    explicit_move_drag(false);
}
#[test]
fn original_explicit_move_drag_observes_factory_cancellation() {
    explicit_move_drag(true);
}

#[test]
fn original_factory_defaults_transfer_between_actual_controls_and_release_after_last_detach() {
    run(|| {
        use ferrodock_model::core::IFactory;
        use ferrodock_model_reactiveui::controls::RootDock;
        let factory = Factory::new();
        let root = RootDock::new();
        let first = DockControl::new();
        first.set_initialize_factory(true);
        first.set_factory(Some(factory.clone()));
        first.set_default_context(Some(Rc::new(String::from("first"))));
        first.set_layout(Some(root.clone()));
        let second = DockControl::new();
        second.set_initialize_factory(true);
        second.set_factory(Some(factory.clone()));
        second.set_default_context(Some(Rc::new(String::from("second"))));
        second.set_layout(Some(root));
        assert_eq!(factory.dock_controls().count(), 2);
        let read = || {
            let callback = IFactory::default_context_locator(&*factory).unwrap();
            let value = callback().unwrap();
            ferrodock_controls_recycling::from_recycling_value(&value)
                .downcast_ref::<String>()
                .unwrap()
                .clone()
        };
        assert_eq!(read(), "first");
        first.set_layout(None);
        assert_eq!(factory.dock_controls().count(), 1);
        assert_eq!(read(), "second");
        second.set_layout(None);
        assert_eq!(factory.dock_controls().count(), 0);
        assert!(IFactory::default_context_locator(&*factory).is_none());
        assert!(IFactory::default_host_window_locator(&*factory).is_none());
        assert!(IFactory::host_window_locator(&*factory).is_none());
    });
}

#[test]
fn source_tracking_updates_pinned_bounds_outside_pin_item() {
    run(|| {
        use crate::controls::DockableControl;
        use ferrodock_model::core::{IDockable, TrackingMode};
        use ferrodock_model_reactiveui::controls::Tool;
        use ferroui_controls::Grid;
        let tool = Tool::new();
        let tracking = DockableControl::new();
        tracking.set_tracking_mode(TrackingMode::Pinned);
        tracking.set_width(120.0);
        tracking.set_height(80.0);
        tracking.set_data_context(Some(crate::model_projection::to_dockable_value(
            tool.clone(),
        )));
        let grid = Grid::new();
        grid.set_width(200.0);
        grid.set_height(200.0);
        grid.children().add(tracking.clone().upcast::<Control>());
        let window = Window::new();
        window.set_width(300.0);
        window.set_height(300.0);
        window.set_content(Some(Rc::new(grid)));
        window.show();
        window.update_layout();
        tracking.invalidate_measure();
        window.update_layout();
        let (_, _, width, height) = tool.get_pinned_bounds();
        assert_eq!((width, height), (120.0, 80.0));
        window.close();
    });
}
#[test]
fn source_tracking_skips_pinned_bounds_inside_actual_pin_item() {
    run(|| {
        use crate::controls::DockableControl;
        use ferrodock_model::core::{IDockable, TrackingMode};
        use ferrodock_model_reactiveui::controls::Tool;
        use ferroui_controls::{templates::FuncControlTemplate, Grid};
        let tool = Tool::new();
        let tracking = DockableControl::new();
        tracking.set_tracking_mode(TrackingMode::Pinned);
        tracking.set_width(120.0);
        tracking.set_height(80.0);
        tracking.set_data_context(Some(crate::model_projection::to_dockable_value(
            tool.clone(),
        )));
        let pin = ToolPinItemControl::new();
        pin.set_width(200.0);
        pin.set_height(200.0);
        let child = tracking.clone();
        pin.set_template(Some(FuncControlTemplate::for_type::<ToolPinItemControl>(
            move |_, _| {
                let grid = Grid::new();
                grid.set_width(200.0);
                grid.set_height(200.0);
                grid.children().add(child.clone().upcast::<Control>());
                grid.upcast()
            },
        )));
        let window = Window::new();
        window.set_width(300.0);
        window.set_height(300.0);
        window.set_content(Some(Rc::new(pin.clone())));
        window.show();
        pin.apply_template();
        window.update_layout();
        tracking.invalidate_measure();
        window.update_layout();
        let (_, _, width, height) = tool.get_pinned_bounds();
        assert!(width.is_nan() && height.is_nan());
        window.close();
    });
}
#[test]
fn tracking_keeps_original_factory_identity_and_unregisters_old_context_and_detach() {
    run(|| {
        use crate::controls::DockableControl;
        use ferrodock_model::core::{IDockable, IFactory, TrackingMode};
        use ferrodock_model_reactiveui::controls::Tool;
        for mode in [
            TrackingMode::Visible,
            TrackingMode::Pinned,
            TrackingMode::Tab,
        ] {
            let factory = Factory::new();
            let first = Tool::new();
            let second = Tool::new();
            first.set_factory(Some(factory.clone()));
            second.set_factory(Some(factory.clone()));
            let tracking = DockableControl::new();
            tracking.set_tracking_mode(mode);
            tracking.set_data_context(Some(crate::model_projection::to_dockable_value(
                first.clone(),
            )));
            let window = Window::new();
            window.set_content(Some(Rc::new(tracking.clone())));
            window.show();
            let (controls, roots) = match mode {
                TrackingMode::Visible => (
                    IFactory::visible_dockable_controls(&*factory),
                    IFactory::visible_root_controls(&*factory),
                ),
                TrackingMode::Pinned => (
                    IFactory::pinned_dockable_controls(&*factory),
                    IFactory::pinned_root_controls(&*factory),
                ),
                TrackingMode::Tab => (
                    IFactory::tab_dockable_controls(&*factory),
                    IFactory::tab_root_controls(&*factory),
                ),
            };
            let first: Rc<dyn IDockable> = first;
            let second: Rc<dyn IDockable> = second;
            let recorded = controls.borrow().get(&first).unwrap().clone();
            assert!(Rc::ptr_eq(
                &recorded,
                &DockableControl::as_dockable_control(tracking.clone())
            ));
            let root = roots.borrow().get(&first).unwrap().clone();
            assert!(root
                .downcast_ref::<Ref<ferroui_base::FerroObject>>()
                .unwrap()
                .ptr_eq(&tracking));
            tracking.set_data_context(Some(crate::model_projection::to_dockable_value(
                second.clone(),
            )));
            assert!(!controls.borrow().contains_key(&first));
            assert!(controls.borrow().contains_key(&second));
            window.close();
            assert!(controls.borrow().is_empty());
            assert!(roots.borrow().is_empty());
        }
    });
}
#[test]
fn document_host_tracks_original_visible_list_replacement_and_detaches_all_events() {
    run(|| {
        use crate::controls::DocumentControl;
        use ferrodock_model::{
            collections::ListHandle,
            core::{IDock, IDockable},
        };
        use ferrodock_model_reactiveui::controls::{Document, DocumentDock};
        let dock = DocumentDock::new();
        let list = ListHandle::<Option<Rc<dyn IDockable>>>::new();
        dock.set_visible_dockables(Some(list.clone()));
        let host = DocumentControl::new();
        host.set_data_context(Some(crate::model_projection::to_dockable_value(
            dock.clone(),
        )));
        let window = Window::new();
        window.set_content(Some(Rc::new(host.clone())));
        window.show();
        assert!(!host.has_visible_dockables());
        list.add(Some(Document::new()));
        assert!(host.has_visible_dockables());
        let replacement = ListHandle::<Option<Rc<dyn IDockable>>>::new();
        dock.set_visible_dockables(Some(replacement.clone()));
        assert!(!host.has_visible_dockables());
        list.add(Some(Document::new()));
        assert!(!host.has_visible_dockables());
        replacement.add(Some(Document::new()));
        assert!(host.has_visible_dockables());
        host.set_is_active(true);
        assert!(host.classes().contains(":active"));
        window.close();
        assert!(!host.has_visible_dockables());
        replacement.add(Some(Document::new()));
        assert!(!host.has_visible_dockables());
    });
}

#[test]
fn document_and_tool_host_peers_use_source_state_and_activate_first_original_item() {
    run(|| {
        use crate::controls::{DocumentControl, ToolControl};
        use ferrodock_model::{
            collections::ListHandle,
            core::{IDock, IDockable},
        };
        use ferrodock_model_reactiveui::controls::{Document, DocumentDock, Tool, ToolDock};
        use ferroui_controls::automation::{
            peers::{AutomationControlType, ControlAutomationPeer},
            provider::IInvokeProvider,
        };
        let factory = Factory::new();
        let document_dock = DocumentDock::new();
        document_dock.set_factory(Some(factory.clone()));
        let document = Document::new();
        document.set_owner(Some(document_dock.clone()));
        document.set_title("First document".into());
        let documents = ListHandle::<Option<Rc<dyn IDockable>>>::new();
        documents.add(Some(document.clone()));
        document_dock.set_visible_dockables(Some(documents));
        let document_host = DocumentControl::new();
        document_host.set_data_context(Some(crate::model_projection::to_dockable_value(
            document_dock.clone(),
        )));
        let peer = ControlAutomationPeer::create_peer_for_element(&document_host);
        assert_eq!(peer.get_class_name(), "DocumentControl");
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::Pane
        );
        assert_eq!(peer.get_help_text(),"Active=false; TabsLayout=Top; HasVisibleDockables=false; VisibleDockables=1; CanCreateDocument=false; LayoutMode=Tabbed; ActiveDockable=none");
        peer.get_provider::<dyn IInvokeProvider>()
            .unwrap()
            .invoke()
            .unwrap();
        assert!(Rc::ptr_eq(
            &document_dock.active_dockable().unwrap(),
            &(document as Rc<dyn IDockable>)
        ));
        let tool_dock = ToolDock::new();
        tool_dock.set_factory(Some(factory.clone()));
        let tool = Tool::new();
        tool.set_owner(Some(tool_dock.clone()));
        let tools = ListHandle::<Option<Rc<dyn IDockable>>>::new();
        tools.add(Some(tool.clone()));
        tool_dock.set_visible_dockables(Some(tools));
        let tool_host = ToolControl::new();
        assert!(
            tool_host.icon_template().is_none()
                && tool_host.header_template().is_none()
                && tool_host.modified_template().is_none()
        );
        tool_host.set_data_context(Some(crate::model_projection::to_dockable_value(
            tool_dock.clone(),
        )));
        let peer = ControlAutomationPeer::create_peer_for_element(&tool_host);
        assert_eq!(peer.get_class_name(), "ToolControl");
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::Pane
        );
        peer.get_provider::<dyn IInvokeProvider>()
            .unwrap()
            .invoke()
            .unwrap();
        assert!(Rc::ptr_eq(
            &tool_dock.active_dockable().unwrap(),
            &(tool as Rc<dyn IDockable>)
        ));
    });
}
