//! Original selector model, state and accessibility scenarios.
use crate::{
    controls::DockSelectorOverlay,
    selectors::{DockSelectorItem, DockSelectorMode},
};
use ferrodock_model::{collections::ReadOnlyListHandle, core::IDockable};
use ferrodock_model_reactiveui::controls::Document;
use ferroui_base::threading::CancellationToken;
use ferroui_controls::{
    automation::{
        peers::{AutomationControlType, ControlAutomationPeer},
        provider::{
            IExpandCollapseProvider, IScrollProvider, ISelectionProvider, IValueProvider,
            ScrollAmount,
        },
        AutomationElementIdentifiers, AutomationProperties, ExpandCollapsePatternIdentifiers,
        ExpandCollapseState, SelectionPatternIdentifiers, ValuePatternIdentifiers,
    },
    AppBuilder, Application,
};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ferrodock-selector",
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
fn item(title: &str) -> Rc<DockSelectorItem> {
    let model = Document::new();
    model.set_title(title.into());
    DockSelectorItem::new(model, 1, true, false, false, false, false)
}
#[test]
fn selector_entry_captures_original_title_and_flags_without_copying_or_observing_the_model() {
    let model = Document::new();
    model.set_title("Original".into());
    for selector in [None, Some("".to_owned()), Some(" \t\r\n\u{a0}".to_owned())] {
        model.set_selector_title(selector);
        let entry = DockSelectorItem::new(model.clone(), i64::MIN, true, false, true, false, true);
        let expected: Rc<dyn IDockable> = model.clone();
        assert!(Rc::ptr_eq(&entry.dockable(), &expected));
        assert_eq!(entry.title(), "Original");
        assert_eq!(entry.activation_order(), i64::MIN);
        assert!(entry.is_document());
        assert!(!entry.is_tool());
        assert!(entry.is_pinned());
        assert!(!entry.is_hidden());
        assert!(entry.is_floating());
    }
    model.set_selector_title(Some("  Selector title  ".into()));
    let entry = DockSelectorItem::new(model.clone(), i64::MAX, false, true, false, true, false);
    assert_eq!(entry.title(), "  Selector title  ");
    model.set_title("Changed".into());
    model.set_selector_title(Some("Changed selector".into()));
    assert_eq!(entry.title(), "  Selector title  ");
    assert!(entry.is_tool());
    assert!(entry.is_hidden());
    assert!(!entry.is_document());
    assert!(!entry.is_pinned());
    assert!(!entry.is_floating());
}
#[test]
fn original_selector_list_role_expand_collapse_value_and_missing_part_scroll_defaults() {
    run(|| {
        let overlay = DockSelectorOverlay::new();
        assert!(!overlay.is_visible());
        assert!(!overlay.is_hit_test_visible());
        assert!(!overlay.is_open());
        assert!(overlay.items().is_none());
        assert!(overlay.selected_item().is_none());
        assert_eq!(overlay.mode(), DockSelectorMode::Documents);
        let a = item("Document A");
        let b = item("Document B");
        overlay.set_items(Some(ReadOnlyListHandle::from_items(vec![a.clone(), b])));
        overlay.set_selected_item(Some(a));
        assert!(ControlAutomationPeer::from_element(&overlay).is_none());
        let peer = ControlAutomationPeer::create_peer_for_element(&overlay);
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::List
        );
        assert_eq!(peer.get_class_name(), "DockSelectorOverlay");
        assert_eq!(peer.get_name(), "Documents selector");
        assert_eq!(
            peer.get_help_text(),
            "Open=false; Mode=Documents; ItemCount=2; Selected=Document A"
        );
        let selection = peer.get_provider::<dyn ISelectionProvider>().unwrap();
        assert!(!selection.can_select_multiple());
        assert!(!selection.is_selection_required());
        assert!(selection.get_selection().is_empty());
        let value = peer.get_provider::<dyn IValueProvider>().unwrap();
        assert!(value.is_read_only());
        assert_eq!(value.value().as_deref(), Some("Document A"));
        assert!(value.set_value(None).is_err());
        let scroll = peer.get_provider::<dyn IScrollProvider>().unwrap();
        assert!(!scroll.horizontally_scrollable());
        assert!(!scroll.vertically_scrollable());
        assert_eq!(scroll.horizontal_scroll_percent(), -1.0);
        assert_eq!(scroll.vertical_scroll_percent(), -1.0);
        assert_eq!(scroll.horizontal_view_size(), 100.0);
        assert_eq!(scroll.vertical_view_size(), 100.0);
        scroll
            .scroll(ScrollAmount::LargeIncrement, ScrollAmount::LargeIncrement)
            .unwrap();
        scroll.set_scroll_percent(50.0, 50.0).unwrap();
        let expand = peer.get_provider::<dyn IExpandCollapseProvider>().unwrap();
        assert!(!expand.shows_menu());
        assert_eq!(
            expand.expand_collapse_state(),
            ExpandCollapseState::Collapsed
        );
        expand.expand().unwrap();
        assert!(overlay.is_visible());
        assert!(overlay.is_hit_test_visible());
        assert!(overlay.classes().contains(":open"));
        assert_eq!(
            expand.expand_collapse_state(),
            ExpandCollapseState::Expanded
        );
        expand.collapse().unwrap();
        assert!(!overlay.is_visible());
        assert!(!overlay.is_hit_test_visible());
        assert!(!overlay.classes().contains(":open"));
    });
}
#[test]
fn original_selector_notifications_preserve_selection_then_value_and_suppress_unchanged_names() {
    run(|| {
        let overlay = DockSelectorOverlay::new();
        let a = item("A");
        let b = item("B");
        overlay.set_selected_item(Some(a));
        let peer = ControlAutomationPeer::create_peer_for_element(&overlay);
        let events = Rc::new(RefCell::new(Vec::new()));
        let out = events.clone();
        let subscription = peer.property_changed(move |e| {
            let name = if std::ptr::eq(
                e.property(),
                SelectionPatternIdentifiers::selection_property(),
            ) {
                "Selection"
            } else if std::ptr::eq(e.property(), ValuePatternIdentifiers::value_property()) {
                "Value"
            } else if std::ptr::eq(e.property(), AutomationElementIdentifiers::name_property()) {
                "Name"
            } else if std::ptr::eq(
                e.property(),
                ExpandCollapsePatternIdentifiers::expand_collapse_state_property(),
            ) {
                "Open"
            } else {
                "Other"
            };
            out.borrow_mut().push(name);
        });
        let children = Rc::new(Cell::new(0));
        let out = children.clone();
        let child_subscription = peer.children_changed(move || out.set(out.get() + 1));
        overlay.set_selected_item(Some(b.clone()));
        overlay.set_mode(DockSelectorMode::Tools);
        overlay.set_items(Some(ReadOnlyListHandle::from_items(vec![b])));
        overlay.set_is_open(true);
        overlay.set_is_open(false);
        assert_eq!(
            &*events.borrow(),
            &["Selection", "Value", "Name", "Open", "Open"]
        );
        assert_eq!(children.get(), 1);
        events.borrow_mut().clear();
        overlay.set_selected_item(Some(item("B")));
        assert_eq!(&*events.borrow(), &["Selection"]);
        AutomationProperties::set_name(&overlay, Some("Explicit selector"));
        events.borrow_mut().clear();
        overlay.set_mode(DockSelectorMode::All);
        assert!(events.borrow().is_empty());
        assert_eq!(peer.get_name(), "Explicit selector");
        overlay.set_selected_item(None);
        assert_eq!(
            peer.get_provider::<dyn IValueProvider>()
                .unwrap()
                .value()
                .as_deref(),
            Some("")
        );
        subscription.dispose();
        child_subscription.dispose();
    });
}
#[test]
fn selector_mode_values_and_service_markup_dispatch_to_the_supplied_contract() {
    use crate::selectors::IDockSelectorService;
    use ferroui_base::{metadata::MarkupTyped, BoxedValue};
    struct Service {
        open: Cell<bool>,
        mode: Cell<DockSelectorMode>,
    }
    impl IDockSelectorService for Service {
        fn is_open(&self) -> bool {
            self.open.get()
        }
        fn show_selector(&self, mode: DockSelectorMode) {
            self.mode.set(mode);
            self.open.set(true);
        }
        fn hide_selector(&self) {
            self.open.set(false);
        }
    }
    crate::register_types();
    let service = Rc::new(Service {
        open: Cell::new(false),
        mode: Cell::new(DockSelectorMode::Documents),
    });
    let contract: Rc<dyn IDockSelectorService> = service.clone();
    let boxed: BoxedValue = Rc::new(contract);
    let metadata = <dyn IDockSelectorService>::MARKUP;
    let show = metadata.find_methods("ShowSelector").next().unwrap();
    let hide = metadata.find_methods("HideSelector").next().unwrap();
    let get = metadata.find_property("IsOpen").unwrap().get.unwrap();
    assert_eq!(
        (get)(&[Some(boxed.clone())])
            .unwrap()
            .unwrap()
            .downcast_ref::<bool>(),
        Some(&false)
    );
    (show.invoke)(&[Some(boxed.clone()), Some(Rc::new(DockSelectorMode::Tools))]).unwrap();
    assert!(service.open.get());
    assert_eq!(service.mode.get(), DockSelectorMode::Tools);
    (hide.invoke)(&[Some(boxed)]).unwrap();
    assert!(!service.open.get());
    for (member, expected) in DockSelectorMode::MARKUP.enum_members.iter().zip([0, 1, 2]) {
        assert_eq!(member.value, expected);
    }
}

#[test]
fn selector_native_receiver_identity_and_value_cast_survive_independent_interface_handles() {
    use crate::selectors::IDockSelectorService;
    use ferroui_base::{data::core::ValueTypes, BoxedValue, FerroObject, Ref};
    use ferroui_controls::Control;
    struct NativeService(Ref<DockSelectorOverlay>);
    impl IDockSelectorService for NativeService {
        fn as_object(&self) -> Option<Ref<FerroObject>> {
            Some(self.0.clone().upcast())
        }
        fn is_open(&self) -> bool {
            self.0.is_open()
        }
        fn show_selector(&self, mode: DockSelectorMode) {
            self.0.set_mode(mode);
            self.0.set_is_open(true);
        }
        fn hide_selector(&self) {
            self.0.set_is_open(false);
        }
    }
    run(|| {
        let native = DockSelectorOverlay::new();
        let weak = native.downgrade();
        let first: Rc<dyn IDockSelectorService> = Rc::new(NativeService(native.clone()));
        let second: Rc<dyn IDockSelectorService> = Rc::new(NativeService(native.clone()));
        let other: Rc<dyn IDockSelectorService> =
            Rc::new(NativeService(DockSelectorOverlay::new()));
        assert!(!Rc::ptr_eq(&first, &second));
        assert!(first.as_ref() == second.as_ref());
        assert!(first.as_ref() != other.as_ref());
        let boxed: BoxedValue = Rc::new(first.clone());
        let recovered = ValueTypes::as_object(&*boxed)
            .unwrap()
            .cast::<DockSelectorOverlay>()
            .unwrap();
        assert!(recovered.ptr_eq(&native));
        assert!(Control::from_boxed(&boxed).unwrap().ptr_eq(&native));
        first.show_selector(DockSelectorMode::Tools);
        assert!(second.is_open());
        assert_eq!(native.mode(), DockSelectorMode::Tools);
        second.hide_selector();
        assert!(!first.is_open());
        drop(recovered);
        drop(boxed);
        drop(first);
        drop(native);
        assert!(weak.upgrade().is_some());
        drop(second);
        assert!(weak.upgrade().is_none());
    });
}
