//! Checked source/platform invocation corpus and actual native states/links.
use crate::controls::*;
use ferroui_base::{
    Ref,
    data::core::{ValueType, ValueTypes},
};
use ferroui_controls::{Control, primitives::ToggleButton};
use std::{cell::RefCell, rc::Rc};
fn item(c: &Control) -> Rc<dyn IRibbonItem> {
    ValueTypes::try_cast(
        &Control::boxed(c.to_ref()),
        ValueType::of::<Rc<dyn IRibbonItem>>(),
    )
    .unwrap()
    .downcast_ref::<Rc<dyn IRibbonItem>>()
    .unwrap()
    .clone()
}
#[test]
fn original_checked_control_and_pinned_uno_invocation_corpus() {
    crate::group_tests::run(|| {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/checked-controls.json"))
                .unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            let root = Ribbon::new();
            let trace = Rc::new(RefCell::new(Vec::new()));
            let log = trace.clone();
            root.add_item_invoked(Some(crate::root_command_tests::invocation(move |args| {
                let value = args
                    .parameter
                    .as_ref()
                    .map(|p| {
                        if let Some(b) = p.downcast_ref::<bool>() {
                            if *b { "True".into() } else { "False".into() }
                        } else {
                            p.downcast_ref::<String>().unwrap().clone()
                        }
                    })
                    .unwrap_or("null".into());
                log.borrow_mut().push(format!("notify:source:{value}"));
            })));
            let source: Ref<ToggleButton> = if case["kind"] == "toggle" {
                let b = RibbonToggleButton::new();
                b.set_group_name(case["grouped"].as_bool().unwrap().then(|| "g".into()));
                b.upcast()
            } else {
                RibbonCheckBox::new().upcast()
            };
            source.set_name(Some("source".into()));
            source.set_is_checked(case["state"].as_bool());
            source.set_is_three_state(case["three"].as_bool().unwrap());
            let owner = ValueTypes::try_cast(
                &Control::boxed(root),
                ValueType::of::<Rc<dyn IRibbonItemOwner>>(),
            )
            .unwrap();
            let owner = owner
                .downcast_ref::<Rc<dyn IRibbonItemOwner>>()
                .unwrap()
                .clone();
            RibbonItemHelper::set_owner(&source, Some(owner.clone())).unwrap();
            let log = trace.clone();
            let command = ferroribbon_core::commands::RibbonRelayCommand::new_with_parameter(
                move |p| {
                    log.borrow_mut().push(format!(
                        "command:{}",
                        p.and_then(|p| p.downcast_ref::<String>()).unwrap()
                    ));
                },
                None,
            );
            source.set_command(Some(crate::commands::adapt_core_command(command)));
            source.set_command_parameter(Some(Rc::new("payload".to_owned())));
            let target: Ref<ToggleButton> = if case["copied"] == true {
                item(&source).create_linked_copy().unwrap().cast().unwrap()
            } else {
                source.clone()
            };
            trace.borrow_mut().clear();
            if case["action"] == "click" {
                ferroui_controls::automation::peers::ToggleButtonAutomationPeer::new(&target)
                    .toggle()
                    .unwrap();
            } else {
                assert!(item(&target).invoke());
            }
            assert_eq!(
                serde_json::json!(source.is_checked()),
                case["sourceState"],
                "{case}"
            );
            assert_eq!(
                serde_json::json!(target.is_checked()),
                case["targetState"],
                "{case}"
            );
            assert_eq!(serde_json::json!(&*trace.borrow()), case["trace"], "{case}");
        }
    });
}
#[test]
fn checked_common_defaults_links_nullable_state_and_unlink_cleanup() {
    crate::group_tests::run(|| {
        let toggle = RibbonToggleButton::new();
        let check = RibbonCheckBox::new();
        assert_eq!(toggle.size(), ferroribbon_core::RibbonItemSize::Small);
        assert_eq!(check.size(), ferroribbon_core::RibbonItemSize::Medium);
        toggle.set_group_name(Some("one".into()));
        toggle.set_show_chevron(true);
        toggle.set_is_three_state(true);
        let copy = toggle
            .create_linked_copy_core()
            .unwrap()
            .cast::<RibbonToggleButton>()
            .unwrap();
        assert!(!copy.is_three_state());
        assert_eq!(copy.group_name().as_deref(), Some("one"));
        assert!(copy.show_chevron());
        copy.set_is_checked(None);
        assert_eq!(toggle.is_checked(), None);
        toggle.set_is_checked(Some(true));
        assert_eq!(copy.is_checked(), Some(true));
        toggle.set_group_name(Some("two".into()));
        assert_eq!(copy.group_name().as_deref(), Some("two"));
        copy.set_group_name(Some("local".into()));
        assert_eq!(toggle.group_name().as_deref(), Some("two"));
        RibbonItemHelper::unlink(&copy);
        toggle.set_is_checked(Some(false));
        assert_eq!(copy.is_checked(), Some(true));
        let weak = toggle.downgrade();
        drop(toggle);
        assert!(
            weak.upgrade().is_some(),
            "source Unlink retains SourceItem and its original object identity"
        );
        drop(copy);
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(
            weak.upgrade().is_none(),
            "dropping the last copy must release its source"
        );
    });
}
#[test]
fn source_radio_groups_exclude_in_visual_scope_and_linked_copies_mirror() {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        r.set_is_application_button_visible(false);
        r.set_is_display_options_button_visible(false);
        let tab = RibbonTab::new();
        let group = RibbonGroup::new();
        let other_group = RibbonGroup::new();
        let a = RibbonToggleButton::new();
        let b = RibbonToggleButton::new();
        let c = RibbonToggleButton::new();
        for v in [&a, &b, &c] {
            v.set_group_name(Some("same".into()));
        }
        group.items().add(Some(a.clone().upcast()));
        group.items().add(Some(b.clone().upcast()));
        other_group.items().add(Some(c.clone().upcast()));
        tab.groups().add(Some(group));
        tab.groups().add(Some(other_group));
        r.tabs().add(Some(tab));
        let w = crate::ribbon_tests::mount(&r);
        a.set_is_checked(Some(true));
        c.set_is_checked(Some(true));
        b.set_is_checked(Some(true));
        assert_eq!(a.is_checked(), Some(false));
        assert_eq!(c.is_checked(), Some(true));
        let copy = a
            .create_linked_copy_core()
            .unwrap()
            .cast::<RibbonToggleButton>()
            .unwrap();
        copy.set_is_checked(Some(true));
        assert_eq!(a.is_checked(), Some(true));
        assert_eq!(b.is_checked(), Some(false));
        ferroui_controls::automation::peers::ToggleButtonAutomationPeer::new(&copy)
            .toggle()
            .unwrap();
        assert_eq!(a.is_checked(), Some(true));
        w.close();
        w.set_content(None);
    });
}
type CheckedCallback = dyn Fn(&ferroui_base::interactivity::RoutedEventArgs);
struct CheckedHandler(Box<CheckedCallback>);
impl PartialEq for CheckedHandler {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class CheckedHandler{this:Rc<CheckedHandler>,handles:[Rc<CheckedHandler>],methods:[fn Invoke(Option<ferroui_base::BoxedValue>,ferroui_base::interactivity::RoutedEventArgs)=>|h:&Rc<CheckedHandler>,_,args|(h.0)(&args)]});
fn checked_handler(
    f: impl Fn(&ferroui_base::interactivity::RoutedEventArgs) + 'static,
) -> ferroui_base::metadata::MarkupDelegate {
    use ferroui_base::metadata::MarkupTyped;
    ValueTypes::register_reference::<CheckedHandler>();
    ferroui_base::metadata::MarkupDelegate::for_method(
        Some(Rc::new(CheckedHandler(Box::new(f)))),
        CheckedHandler::MARKUP,
        &CheckedHandler::MARKUP.methods[0],
    )
}
#[test]
fn checked_events_preserve_raw_multicast_source_and_reentrant_order() {
    crate::group_tests::run(|| {
        let b = RibbonToggleButton::new();
        let log = Rc::new(RefCell::new(Vec::new()));
        let weak = b.downgrade();
        let trace = log.clone();
        b.add_checked(Some(checked_handler(move |e| {
            assert!(e.source().unwrap().ptr_eq(&weak.upgrade().unwrap()));
            e.set_handled(true);
            trace.borrow_mut().push("checked-A");
            weak.upgrade().unwrap().set_is_checked(Some(false));
        })));
        let trace = log.clone();
        b.add_checked(Some(checked_handler(move |_| {
            trace.borrow_mut().push("checked-B")
        })));
        let trace = log.clone();
        b.add_unchecked(Some(checked_handler(move |_| {
            trace.borrow_mut().push("unchecked")
        })));
        b.set_is_checked(Some(true));
        assert_eq!(&*log.borrow(), &["checked-A", "unchecked", "checked-B"]);
    });
}
#[test]
fn native_effective_disablement_maps_source_coerced_invoke_guard() {
    crate::group_tests::run(|| {
        let parent = ferroui_controls::StackPanel::new();
        let toggle = RibbonToggleButton::new();
        let check = RibbonCheckBox::new();
        parent.children().add(toggle.clone());
        parent.children().add(check.clone());
        parent.set_is_enabled(false);
        for c in [toggle.upcast::<Control>(), check.upcast::<Control>()] {
            assert!(!item(&c).invoke());
            assert_eq!(c.cast::<ToggleButton>().unwrap().is_checked(), Some(false));
        }
    });
}
