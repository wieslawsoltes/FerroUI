//! Unchanged selected split algorithms and native part/flyout/provider integration.
use crate::{
    compat::*,
    controls::*,
    primitives::{RibbonItemContent, RibbonItemContentPart},
};
use ferroui_base::{
    Rect, Ref, Size, StaticType,
    controls::ResourceDictionary,
    data::core::{ValueType, ValueTypes},
    styling::ControlTheme,
};
use ferroui_controls::{
    Button, Control, Grid, MenuFlyout, Window,
    automation::{
        AutomationProperties, ExpandCollapseState,
        peers::{AutomationControlType, ControlAutomationPeer},
        provider::{IExpandCollapseProvider, IInvokeProvider},
    },
};
use std::{cell::RefCell, rc::Rc};
fn text(value: Option<&dyn std::any::Any>) -> String {
    value
        .map(|v| {
            if let Some(b) = v.downcast_ref::<bool>() {
                if *b { "True".into() } else { "False".into() }
            } else {
                v.downcast_ref::<String>().unwrap().clone()
            }
        })
        .unwrap_or_default()
}
fn owner(root: &Ribbon) -> Rc<dyn IRibbonItemOwner> {
    ValueTypes::try_cast(
        &Control::boxed(root.to_ref()),
        ValueType::of::<Rc<dyn IRibbonItemOwner>>(),
    )
    .unwrap()
    .downcast_ref::<Rc<dyn IRibbonItemOwner>>()
    .unwrap()
    .clone()
}
fn cases() -> serde_json::Value {
    serde_json::from_str(include_str!("../../tests/fixtures/split-button.json")).unwrap()
}
#[test]
fn original_split_invocation_and_linked_copy_counts_match_selected_source() {
    crate::group_tests::run(|| {
        for case in cases()["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["name"] == "invoke")
        {
            let root = Ribbon::new();
            let log = Rc::new(RefCell::new(Vec::<String>::new()));
            let trace = log.clone();
            root.add_item_invoked(Some(crate::root_command_tests::invocation(move |args| {
                let p = args.parameter.as_ref().map(|v| v.as_any());
                trace.borrow_mut().push(format!("notify:{}", text(p)));
            })));
            let source = RibbonSplitButton::new();
            source.set_label(Some("Own".into()));
            source.set_icon(Some(Rc::new("own".to_owned())));
            source.set_large_icon(Some(Rc::new("large".to_owned())));
            source.set_is_checkable(case["checkable"].as_bool().unwrap());
            source.set_is_checked(case["state"].as_bool());
            source.set_follow_last_choice(case["follow"].as_bool().unwrap());
            source.set_command_parameter(Some(Rc::new("ownParameter".to_owned())));
            RibbonItemHelper::set_owner(&source, Some(owner(&root))).unwrap();
            let trace = log.clone();
            source.set_command(Some(crate::commands::adapt_core_command(
                ferroribbon_core::commands::RibbonRelayCommand::new_with_parameter(
                    move |p| trace.borrow_mut().push(format!("own:{}", text(p))),
                    None,
                ),
            )));
            if case["last"] != "none" {
                let last = MenuFlyoutItem::new();
                last.set_text(Some("Choice".into()));
                last.set_command_parameter(Some(Rc::new("choiceParameter".to_owned())));
                let trace = log.clone();
                let enabled = case["last"] != "blocked";
                last.set_command(Some(crate::commands::adapt_core_command(
                    ferroribbon_core::commands::RibbonRelayCommand::new_with_parameter(
                        move |p| trace.borrow_mut().push(format!("choice:{}", text(p))),
                        Some(Rc::new(move |_| enabled)),
                    ),
                )));
                if case["last"] == "action" {
                    let trace = log.clone();
                    RibbonMenu::set_invoke(
                        &last,
                        Some(crate::controls::ribbon_menu::action_delegate(move || {
                            trace.borrow_mut().push("action".into())
                        })),
                    );
                }
                source.set_last_choice(Some(last));
            }
            let trace = log.clone();
            source.add_click(Some(crate::controls::ribbon_menu::click_delegate(
                move |_, _| trace.borrow_mut().push("sourceClick".into()),
            )));
            let target = if case["copied"] == true {
                let copy = source
                    .create_linked_copy_core()
                    .unwrap()
                    .cast::<RibbonSplitButton>()
                    .unwrap();
                let trace = log.clone();
                copy.add_click(Some(crate::controls::ribbon_menu::click_delegate(
                    move |_, _| trace.borrow_mut().push("copyClick".into()),
                )));
                copy
            } else {
                source.clone()
            };
            log.borrow_mut().clear();
            assert_eq!(
                serde_json::json!(target.invoke_core()),
                case["result"],
                "{case}"
            );
            assert_eq!(
                serde_json::json!(source.is_checked()),
                case["sourceChecked"],
                "{case}"
            );
            assert_eq!(
                serde_json::json!(target.is_checked()),
                case["targetChecked"],
                "{case}"
            );
            assert_eq!(serde_json::json!(&*log.borrow()), case["trace"], "{case}");
        }
    });
}
#[test]
fn original_primary_presentation_fallbacks_and_localized_options_format() {
    if crate::test_process::isolated_case(concat!(
        module_path!(),
        "::original_primary_presentation_fallbacks_and_localized_options_format"
    )) {
        return;
    }
    crate::group_tests::run(|| {
        use ferroribbon_core::localization::RibbonStrings;
        for case in cases()["cases"].as_array().unwrap() {
            if case["name"] == "presentation" {
                let choice = MenuFlyoutItem::new();
                choice.set_text(case["text"].as_str().map(Into::into));
                if case["icon"] == "font" {
                    let f = FontIcon::new();
                    f.set_glyph(Some("glyph".into()));
                    choice.set_icon(Some(Control::boxed(f)));
                } else if case["icon"] == "other" {
                    choice.set_icon(Some(Control::boxed(ferroui_controls::PathIcon::new())));
                }
                if case["tag"] == true {
                    choice.set_tag(Some(Rc::new("tag".to_owned())));
                }
                let split = RibbonSplitButton::new();
                split.set_label(Some("Own".into()));
                split.set_icon(Some(Rc::new("own".to_owned())));
                split.set_large_icon(Some(Rc::new("large".to_owned())));
                split.set_follow_last_choice(true);
                split.set_last_choice(Some(choice));
                let (label, icon, large) = split.primary_presentation();
                assert_eq!(serde_json::json!(label), case["label"]);
                assert_eq!(
                    serde_json::json!(icon.map(|v| v.downcast_ref::<String>().unwrap().clone())),
                    case["small"]
                );
                assert_eq!(
                    serde_json::json!(large.map(|v| v.downcast_ref::<String>().unwrap().clone())),
                    case["large"]
                );
            } else if case["name"] == "format" {
                let original = RibbonStrings::current();
                let strings = RibbonStrings::for_culture(Some(original.culture())).unwrap();
                RibbonStrings::set_current(Some(strings.clone())).unwrap();
                strings
                    .override_string(
                        Some("SplitButtonOptions".into()),
                        Some(case["format"].as_str().unwrap().into()),
                    )
                    .unwrap();
                let result =
                    std::panic::catch_unwind(|| RibbonSplitButton::format_options_name("Paste\nX"));
                RibbonStrings::set_current(Some(original)).unwrap();
                if case["error"].is_null() {
                    assert_eq!(result.unwrap(), case["result"].as_str().unwrap(), "{case}");
                } else {
                    assert!(result.is_err(), "{case}");
                }
            }
        }
    });
}
fn mount(split: &RibbonSplitButton) -> Ref<Window> {
    let dictionary = ValueTypes::as_object(
        &*crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/Split.xaml")
            .unwrap()
            .unwrap(),
    )
    .unwrap()
    .cast::<ResourceDictionary>()
    .unwrap();
    let theme = ValueTypes::as_object(
        &*dictionary
            .try_get_resource(&<RibbonSplitButton as StaticType>::TYPE.into(), None)
            .flatten()
            .unwrap(),
    )
    .unwrap()
    .cast::<ControlTheme>()
    .unwrap();
    split.set_resources(dictionary);
    split.set_theme(theme);
    let w = Window::new();
    w.set_content(Some(Control::boxed(split.to_ref())));
    w.show();
    w.measure(Size::new(600., 200.));
    w.arrange(Rect::new(0., 0., 600., 200.));
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    w
}
fn part<T: ferroui_base::ObjectType>(split: &RibbonSplitButton, name: &str) -> Ref<T> {
    split
        .get_visual_descendants()
        .find_map(|v| {
            v.cast::<Control>()
                .filter(|c| c.name().as_deref() == Some(name))
                .and_then(|c| c.cast())
        })
        .unwrap()
}
#[test]
fn source_template_parts_switch_layout_and_detach_clicks_on_template_replacement() {
    crate::group_tests::run(|| {
        let split = RibbonSplitButton::new();
        split.set_label(Some("Paste\nText".into()));
        let w = mount(&split);
        assert_eq!(split.size(), ferroribbon_core::RibbonItemSize::Large);
        split.apply_layout(RibbonItemLayout {
            size: ferroribbon_core::RibbonItemSize::Large,
            ..Default::default()
        });
        let grid = part::<Grid>(&split, "PART_Root");
        let primary = part::<Button>(&split, "PART_PrimaryButton");
        let secondary = part::<Button>(&split, "PART_SecondaryButton");
        let content = part::<RibbonItemContent>(&split, "PART_PrimaryContent");
        assert_eq!(grid.row_definitions().count(), 2);
        assert_eq!(Grid::get_row(&secondary), 1);
        assert_eq!(content.part(), RibbonItemContentPart::IconOnly);
        assert_eq!(
            AutomationProperties::get_name(&primary).as_deref(),
            Some("Paste Text")
        );
        assert_eq!(
            AutomationProperties::get_name(&secondary).as_deref(),
            Some("Paste Text options")
        );
        split.set_is_checkable(true);
        ferroui_controls::automation::peers::ButtonAutomationPeer::new(&primary)
            .invoke()
            .unwrap();
        assert_eq!(split.is_checked(), Some(true));
        split.apply_layout(RibbonItemLayout {
            size: ferroribbon_core::RibbonItemSize::Small,
            ..Default::default()
        });
        assert_eq!(grid.column_definitions().count(), 2);
        assert_eq!(Grid::get_column(&secondary), 1);
        assert_eq!(content.part(), RibbonItemContentPart::All);
        split.set_template(None);
        split.apply_template();
        assert_eq!(split.visual_children().count(), 0);
        w.measure(Size::new(600., 200.));
        w.arrange(Rect::new(0., 0., 600., 200.));
        primary.raise_event(&ferroui_base::interactivity::RoutedEventArgs::with_event(
            Button::click_event(),
        ));
        assert_eq!(split.is_checked(), Some(true));
        w.close();
    });
}
#[test]
fn nested_choice_hooks_reopen_unlink_shared_flyout_and_release_source() {
    crate::group_tests::run(|| {
        let source = RibbonSplitButton::new();
        source.set_follow_last_choice(true);
        let menu = MenuFlyout::new();
        let sub = MenuFlyoutSubItem::new();
        let choice = MenuFlyoutItem::new();
        choice.set_text(Some("Text Only".into()));
        sub.items().add(Some(choice.clone().upcast()));
        menu.items().add(Some(Control::boxed(sub)));
        source.set_flyout(Some(menu.clone().upcast()));
        let w = mount(&source);
        let copy = source
            .create_linked_copy_core()
            .unwrap()
            .cast::<RibbonSplitButton>()
            .unwrap();
        source.open_drop_down();
        assert!(source.is_drop_down_open());
        choice.invoke_menu_core();
        assert!(source.last_choice().unwrap().ptr_eq(&choice));
        assert!(copy.last_choice().unwrap().ptr_eq(&choice));
        source.close_drop_down();
        source.open_drop_down();
        choice.invoke_menu_core();
        source.close_drop_down();
        RibbonItemHelper::unlink(&copy);
        assert!(copy.flyout().is_none());
        assert!(source.flyout().is_some());
        let other = MenuFlyout::new();
        source.set_flyout(Some(other.upcast()));
        source.set_last_choice(None);
        choice.invoke_menu_core();
        assert!(source.last_choice().is_none());
        w.set_content(None);
        w.close();
        let weak = source.downgrade();
        drop(source);
        assert!(weak.upgrade().is_some());
        drop(copy);
        drop(w);
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(weak.upgrade().is_none());
    });
}
#[test]
fn native_automation_exposes_source_split_patterns_name_and_disabled_noop() {
    crate::group_tests::run(|| {
        let split = RibbonSplitButton::new();
        split.set_label(Some("Font\nColor".into()));
        split.set_is_checkable(true);
        let peer = ControlAutomationPeer::create_peer_for_element(&split);
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::SplitButton
        );
        assert_eq!(peer.get_class_name(), "RibbonSplitButton");
        assert_eq!(peer.get_name(), "Font Color");
        let invoke = peer.get_provider::<dyn IInvokeProvider>().unwrap();
        let expand = peer.get_provider::<dyn IExpandCollapseProvider>().unwrap();
        assert_eq!(
            expand.expand_collapse_state(),
            ExpandCollapseState::Collapsed
        );
        invoke.invoke().unwrap();
        assert_eq!(split.is_checked(), Some(true));
        split.set_is_enabled(false);
        invoke.invoke().unwrap();
        assert_eq!(split.is_checked(), Some(true));
        expand.expand().unwrap();
        expand.collapse().unwrap();
    });
}
#[test]
fn upstream_item_test_command_can_execute_combines_with_explicit_enabled_state() {
    crate::group_tests::run(|| {
        let enabled = Rc::new(std::cell::Cell::new(false));
        let predicate = enabled.clone();
        let command = ferroribbon_core::commands::RibbonRelayCommand::new_with_can_execute(
            || {},
            Some(Rc::new(move || predicate.get())),
        );
        let split = RibbonSplitButton::new();
        split.set_label(Some("Paste".into()));
        split.set_command(Some(crate::commands::adapt_core_command(command.clone())));
        let w = mount(&split);
        assert!(!split.is_enabled());
        enabled.set(true);
        command.notify_can_execute_changed();
        assert!(split.is_enabled());
        enabled.set(false);
        command.notify_can_execute_changed();
        split.set_command(None);
        assert!(split.is_enabled());
        split.set_is_enabled(false);
        split.set_command(Some(crate::commands::adapt_core_command(command.clone())));
        enabled.set(true);
        command.notify_can_execute_changed();
        assert!(!split.is_enabled());
        assert!(!split.invoke_core());
        w.close();
    });
}
#[test]
fn split_f4_alt_down_custom_placement_and_cloned_overflow_primary() {
    crate::group_tests::run(|| {
        use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
        use ferroui_controls::{PlacementMode, primitives::PopupFlyoutBase};
        let split = RibbonSplitButton::new();
        split.set_label(Some("Paste".into()));
        let menu = MenuFlyout::new();
        menu.set_placement(PlacementMode::Right);
        let item = MenuFlyoutItem::new();
        item.set_text(Some("Option".into()));
        menu.items().add(Some(Control::boxed(item)));
        split.set_flyout(Some(menu.clone().upcast()));
        assert_eq!(menu.placement(), PlacementMode::Right);
        assert!(
            menu.get_local_value(PopupFlyoutBase::placement_property())
                .is_some()
        );
        let w = mount(&split);
        for (name, key, modifiers) in [
            ("PART_PrimaryButton", Key::F4, KeyModifiers::NONE),
            ("PART_SecondaryButton", Key::Down, KeyModifiers::ALT),
        ] {
            let button = part::<Button>(&split, name);
            let mut e = KeyEventArgs::new();
            e.key = key;
            e.key_modifiers = modifiers;
            e.set_routed_event(Some(InputElement::key_down_event()));
            button.raise_event(&e);
            assert!(e.handled());
            assert!(split.is_drop_down_open());
            split.close_drop_down();
        }
        split.set_is_checkable(true);
        let entries = split.create_overflow_menu_items_core();
        let sub = entries
            .iter()
            .next()
            .unwrap()
            .unwrap()
            .cast::<MenuFlyoutSubItem>()
            .unwrap();
        assert_eq!(sub.items().count(), 3);
        let primary = sub.items().get(0).unwrap();
        assert!(sub.items().get(1).unwrap().is::<MenuFlyoutSeparator>());
        let cloned = RibbonMenuCloner::clone(Some(
            ferroribbon_core::model::RibbonItemsSource::from_values(vec![Some(primary)]),
        ))
        .iter()
        .next()
        .unwrap()
        .unwrap()
        .cast::<MenuFlyoutItem>()
        .unwrap();
        cloned.invoke_menu_core();
        assert_eq!(split.is_checked(), Some(true));
        w.close();
    });
}
