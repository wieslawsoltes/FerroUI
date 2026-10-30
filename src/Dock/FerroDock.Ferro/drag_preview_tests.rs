//! Native original drag preview state, inheritance and accessibility behavior.
use crate::controls::{DragPreviewControl, DragPreviewWindow};
use ferrodock_model::core::IDockable;
use ferrodock_model_reactiveui::controls::Document;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl,
    VisualImpl,
};
use ferroui_base::{
    input::InputElementImpl, interactivity::InteractiveImpl, layout::LayoutableImpl,
    threading::CancellationToken,
};
use ferroui_controls::automation::peers::{AutomationControlType, ControlAutomationPeer};
use ferroui_controls::automation::provider::IValueProvider;
use ferroui_controls::automation::{AutomationProperties, ValuePatternIdentifiers};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    AppBuilder, Application, Border, ContentControlImpl, ControlImpl, TopLevelImpl, WindowBaseImpl,
    WindowImpl,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::{cell::RefCell, rc::Rc};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-drag-preview",
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
#[repr(C)]
struct DerivedPreview {
    base: DragPreviewControl,
}
ferro_class!(DerivedPreview:DragPreviewControl);
ferroui_base::ferro_class_info!(DerivedPreview {});
ferro_impl_classes!(DerivedPreview:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl);
#[repr(C)]
struct DerivedWindow {
    base: DragPreviewWindow,
}
ferro_class!(DerivedWindow:DragPreviewWindow);
ferroui_base::ferro_class_info!(DerivedWindow {});
ferro_impl_classes!(DerivedWindow:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl,TemplatedControlImpl,ContentControlImpl,TopLevelImpl,WindowBaseImpl,WindowImpl);
#[test]
fn preview_source_defaults_nullable_properties_unvalidated_dimensions_and_own_real_recycling_cache()
{
    run(|| {
        let a = DragPreviewControl::new();
        let b = DragPreviewControl::new();
        assert!(a.content_template().is_none());
        assert!(a.status().is_none());
        assert!(!a.show_content());
        assert!(a.preview_content_width().is_nan());
        assert!(a.preview_content_height().is_nan());
        assert!(a.preview_content().is_none());
        let cache = a.control_recycling().unwrap();
        let other = b.control_recycling().unwrap();
        assert!(!Rc::ptr_eq(&cache, &other));
        let key: Rc<dyn std::any::Any> = Rc::new("original key".to_owned());
        let control = Border::new();
        let boxed: Rc<dyn std::any::Any> = Rc::new(control.clone());
        cache.add(key.clone(), boxed);
        let cached = cache.try_get_value(Some(key.clone())).unwrap();
        let native = ferrodock_controls_recycling::from_recycling_value(&cached);
        assert_eq!(
            ferroui_base::data::core::ValueTypes::as_object(&*native)
                .unwrap()
                .cast::<Border>(),
            Some(control.clone())
        );
        assert!(other.try_get_value(Some(key)).is_none());
        a.set_control_recycling(None);
        assert!(a.control_recycling().is_none());
        a.set_control_recycling(Some(other.clone()));
        assert!(Rc::ptr_eq(&a.control_recycling().unwrap(), &other));
        a.set_preview_content(Some(control.upcast()));
        assert!(a.preview_content().is_some());
        a.set_preview_content(None);
        for v in [-1.0, 0.0, f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            a.set_preview_content_width(v);
            a.set_preview_content_height(v);
            assert!(a.preview_content_width() == v || v.is_nan());
            assert!(a.preview_content_height() == v || v.is_nan());
        }
    });
}
#[test]
fn derived_preview_and_window_keep_source_constructor_peer_and_window_style_key() {
    run(|| {
        let a: Ref<DerivedPreview> = instantiate(DerivedPreview {
            base: DragPreviewControl::construct(),
        });
        let b = DragPreviewControl::new();
        assert!(a.control_recycling().is_some());
        assert!(!Rc::ptr_eq(
            &a.control_recycling().unwrap(),
            &b.control_recycling().unwrap()
        ));
        assert_eq!(
            ControlAutomationPeer::create_peer_for_element(&a).get_class_name(),
            "DragPreviewControl"
        );
        let w: Ref<DerivedWindow> = instantiate(DerivedWindow {
            base: DragPreviewWindow::construct(),
        });
        assert_eq!(w.style_key(), DragPreviewWindow::TYPE);
        assert_eq!(
            ControlAutomationPeer::create_peer_for_element(&w).get_class_name(),
            "DragPreviewWindow"
        );
        w.close();
    });
}
#[test]
fn status_changes_do_not_create_peers_and_existing_peer_emits_only_ordinal_normalized_value_changes(
) {
    run(|| {
        let preview = DragPreviewControl::new();
        preview.set_status(Some("early".into()));
        assert!(ControlAutomationPeer::from_element(&preview).is_none());
        preview.set_status(None);
        let peer = ControlAutomationPeer::create_peer_for_element(&preview);
        let events = Rc::new(RefCell::new(Vec::new()));
        let output = events.clone();
        let subscription = peer.property_changed(move |e| {
            assert!(std::ptr::eq(
                e.property(),
                ValuePatternIdentifiers::value_property()
            ));
            output.borrow_mut().push((
                e.old_value()
                    .unwrap()
                    .downcast_ref::<String>()
                    .unwrap()
                    .clone(),
                e.new_value()
                    .unwrap()
                    .downcast_ref::<String>()
                    .unwrap()
                    .clone(),
            ));
        });
        preview.set_status(Some(String::new()));
        preview.set_status(None);
        assert!(events.borrow().is_empty());
        preview.set_status(Some("Dock".into()));
        preview.set_status(Some("Dock".into()));
        preview.set_status(Some("dock".into()));
        preview.set_status(Some(" dock ".into()));
        preview.set_show_content(true);
        preview.set_preview_content_width(320.0);
        preview.set_status(None);
        assert_eq!(
            &*events.borrow(),
            &[
                ("".into(), "Dock".into()),
                ("Dock".into(), "dock".into()),
                ("dock".into(), " dock ".into()),
                (" dock ".into(), "".into())
            ]
        );
        subscription.dispose();
        preview.set_status(Some("Float".into()));
        assert_eq!(events.borrow().len(), 4);
    });
}
#[test]
fn preview_peer_retains_name_precedence_help_fields_and_read_only_value_contract() {
    run(|| {
        let p = DragPreviewControl::new();
        let peer = ControlAutomationPeer::create_peer_for_element(&p);
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::Pane
        );
        assert_eq!(peer.get_name(), "Drag preview");
        assert_eq!(
            peer.get_help_text(),
            "Status=null; ShowContent=false; PreviewWidth=NaN; PreviewHeight=NaN"
        );
        let document = Document::new();
        document.set_title("Original document".into());
        p.set_data_context(Some(crate::model_projection::to_dockable_value(document)));
        assert_eq!(peer.get_name(), "Original document");
        AutomationProperties::set_name(&p, Some("Explicit"));
        assert_eq!(peer.get_name(), "Explicit");
        let value = peer.get_provider::<dyn IValueProvider>().unwrap();
        assert!(value.is_read_only());
        assert_eq!(value.peer(), peer);
        assert_eq!(value.value(), Some(String::new()));
        p.set_status(Some("Dock".into()));
        p.set_show_content(true);
        p.set_preview_content_width(100.0);
        p.set_preview_content_height(200.0);
        assert_eq!(value.value().as_deref(), Some("Dock"));
        assert_eq!(
            peer.get_help_text(),
            "Status=Dock; ShowContent=true; PreviewWidth=100; PreviewHeight=200"
        );
        p.set_is_enabled(false);
        for requested in [None, Some("Float"), Some("")] {
            assert_eq!(
                value.set_value(requested).unwrap_err().to_string(),
                "Drag preview status is read-only."
            );
            assert_eq!(p.status().as_deref(), Some("Dock"));
        }
    });
}
#[test]
fn preview_status_event_callbacks_observe_committed_state_and_can_reenter() {
    run(|| {
        let p = DragPreviewControl::new();
        let peer = ControlAutomationPeer::create_peer_for_element(&p);
        let weak = p.downgrade();
        let calls = Rc::new(RefCell::new(Vec::new()));
        let observed = calls.clone();
        let token = peer.property_changed(move |e| {
            let p = weak.upgrade().unwrap();
            let value = e.new_value().unwrap().downcast_ref::<String>().unwrap();
            assert_eq!(p.status().as_deref(), Some(value.as_str()));
            observed.borrow_mut().push(value.clone());
            if value == "Dock" {
                p.set_status(Some("Float".into()));
            }
        });
        p.set_status(Some("Dock".into()));
        assert_eq!(&*calls.borrow(), &["Dock", "Float"]);
        assert_eq!(p.status().as_deref(), Some("Float"));
        token.dispose();
    });
}
#[test]
fn transient_window_peer_reports_original_state_but_excludes_control_and_content_roles() {
    run(|| {
        let window = DragPreviewWindow::new();
        let peer = ControlAutomationPeer::create_peer_for_element(&window);
        assert_eq!(peer.get_class_name(), "DragPreviewWindow");
        assert_eq!(peer.get_name(), window.title().unwrap());
        window.set_title(Some(String::new()));
        assert_eq!(peer.get_name(), "Drag preview window");
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::Pane
        );
        assert!(!peer.is_control_element());
        assert!(!peer.is_content_element());
        window.set_title(Some(String::new()));
        AutomationProperties::set_name(&window, Some("Named preview"));
        assert_eq!(peer.get_name(), "Named preview");
        window.show();
        assert!(peer.get_help_text().contains("Visible=true"));
        assert!(!peer.is_control_element());
        assert!(!peer.is_content_element());
        window.close();
        assert!(peer.get_help_text().contains("Visible=false"));
    });
}

#[test]
fn source_drag_preview_and_adorner_windows_are_noninteractive_panes() {
    run(|| {
        let preview = DragPreviewWindow::new();
        let adorner = crate::controls::DockAdornerWindow::new();
        for (window, text) in [
            (
                preview.clone().upcast::<ferroui_controls::Window>(),
                "Preview",
            ),
            (
                adorner.clone().upcast::<ferroui_controls::Window>(),
                "Adorner",
            ),
        ] {
            window.set_width(320.0);
            window.set_height(180.0);
            let label = ferroui_controls::TextBlock::new();
            label.set_text(Some(text));
            window.set_content(Some(Rc::new(label)));
            window.show();
            window.apply_template();
            window.update_layout();
        }
        for (window, class_name) in [
            (
                preview.clone().upcast::<ferroui_controls::Window>(),
                "DragPreviewWindow",
            ),
            (
                adorner.clone().upcast::<ferroui_controls::Window>(),
                "DockAdornerWindow",
            ),
        ] {
            let peer = ControlAutomationPeer::create_peer_for_element(&window);
            assert_eq!(peer.get_class_name(), class_name);
            assert_eq!(
                peer.get_automation_control_type(),
                AutomationControlType::Pane
            );
            assert!(!peer.is_control_element());
            assert!(!peer.is_content_element());
        }
        adorner.close();
        preview.close();
    });
}
#[test]
fn source_drag_preview_control_exposes_readonly_value_provider_and_change_event() {
    run(|| {
        let control = DragPreviewControl::new();
        control.set_status(Some("Dock".into()));
        control.set_show_content(true);
        let window = ferroui_controls::Window::new();
        window.set_width(420.0);
        window.set_height(240.0);
        window.set_content(Some(Rc::new(control.clone())));
        window.show();
        control.apply_template();
        window.update_layout();
        let peer = ControlAutomationPeer::create_peer_for_element(&control);
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::Pane
        );
        assert_eq!(peer.get_class_name(), "DragPreviewControl");
        let provider = peer.get_provider::<dyn IValueProvider>().unwrap();
        assert!(provider.is_read_only());
        assert_eq!(provider.value().as_deref(), Some("Dock"));
        let changes = Rc::new(RefCell::new(Vec::new()));
        let observed = changes.clone();
        peer.property_changed(move |args| observed.borrow_mut().push(args.property()));
        control.set_status(Some("Float".into()));
        assert_eq!(provider.value().as_deref(), Some("Float"));
        assert!(changes
            .borrow()
            .iter()
            .any(|p| std::ptr::eq(*p, ValuePatternIdentifiers::value_property())));
        window.close();
    });
}
