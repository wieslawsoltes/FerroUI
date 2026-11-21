//! Original C# menu snapshot/state corpus, plus native ownership/input regressions.
use crate::{compat::*, controls::*};
use ferroribbon_core::model::RibbonItemsSource;
use ferroui_base::{
    Ref,
    data::core::ValueTypes,
    metadata::{MarkupDelegate, MarkupTyped},
};
use ferroui_controls::{Control, MenuItem};
use std::{cell::RefCell, rc::Rc};

#[test]
fn image_uri_failures_leave_overflow_commands_usable() {
    use crate::commands::adapt_core_command;
    use crate::primitives::{RibbonDeferredImage, RibbonImageLoadError};
    use ferroribbon_core::commands::RibbonRelayCommand;
    use ferroui_base::data::core::ValueType;
    use std::cell::Cell;

    crate::group_tests::run(|| {
        let calls = Rc::new(Cell::new(0));
        let observed = calls.clone();
        let button = RibbonButton::new();
        button.set_icon(Some(Rc::new(
            "ferres://FerroRibbon/Assets/missing-overflow-icon.png".to_owned(),
        )));
        button.set_command(Some(adapt_core_command(RibbonRelayCommand::new(
            move || observed.set(observed.get() + 1),
        ))));
        let item = ValueTypes::try_convert(
            Some(&Control::boxed(&button)),
            ValueType::of::<Rc<dyn IRibbonItem>>(),
        )
        .flatten()
        .unwrap();
        let entry = item
            .downcast_ref::<Rc<dyn IRibbonItem>>()
            .unwrap()
            .create_overflow_menu_items()
            .iter()
            .next()
            .unwrap()
            .unwrap()
            .cast::<MenuFlyoutItem>()
            .unwrap();
        assert!(entry.is_enabled());
        let load_error = || {
            let icon = Control::from_boxed(&entry.icon().unwrap())
                .unwrap()
                .cast::<ImageIcon>()
                .unwrap();
            let source = icon.source().unwrap();
            let failed = source
                .as_object()
                .unwrap()
                .downcast_ref::<RibbonDeferredImage>()
                .unwrap();
            failed.load_result().unwrap().unwrap_err()
        };
        assert!(matches!(
            load_error().as_ref(),
            RibbonImageLoadError::Load(_)
        ));
        RibbonMenu::set_icon(
            &entry,
            Some(Rc::new("ftp://example.invalid/icon.png".to_owned())),
        );
        assert!(matches!(
            load_error().as_ref(),
            RibbonImageLoadError::UnsupportedScheme(scheme) if scheme == "ftp"
        ));
        assert!(entry.is_enabled());
        entry.invoke_menu_core();
        assert_eq!(calls.get(), 1);
    });
}

#[test]
fn menu_item_peers_expose_literal_labels_and_native_activation() {
    use ferroui_controls::automation::{
        AutomationProperties,
        peers::{AutomationControlType, ControlAutomationPeer},
        provider::{IExpandCollapseProvider, IInvokeProvider},
    };
    use std::cell::Cell;

    crate::group_tests::run(|| {
        let item = MenuFlyoutItem::new();
        item.set_text(Some("_Paste".to_owned()));
        let peer = ControlAutomationPeer::create_peer_for_element(&item);
        assert_eq!(
            peer.get_automation_control_type(),
            AutomationControlType::MenuItem
        );
        assert_eq!(peer.get_name(), "_Paste");
        item.set_text(Some("Paste options".to_owned()));
        assert_eq!(peer.get_name(), "Paste options");
        AutomationProperties::set_name(&item, Some("Custom name"));
        assert_eq!(peer.get_name(), "Custom name");

        let clicks = Rc::new(Cell::new(0));
        let count = clicks.clone();
        item.add_click_action(move |_, _| count.set(count.get() + 1));
        peer.get_provider::<dyn IInvokeProvider>()
            .unwrap()
            .invoke()
            .unwrap();
        assert_eq!(clicks.get(), 1);

        let submenu = MenuFlyoutSubItem::new();
        submenu.set_text(Some("Document actions".to_owned()));
        submenu.items().add(Some(item.upcast()));
        let peer = ControlAutomationPeer::create_peer_for_element(&submenu);
        assert_eq!(peer.get_name(), "Document actions");
        assert!(peer.get_provider::<dyn IExpandCollapseProvider>().is_some());
        assert!(peer.get_provider::<dyn IInvokeProvider>().is_none());
    });
}

struct Action(Box<dyn Fn()>);
type ConverterFn = dyn Fn(Option<ferroui_base::BoxedValue>) -> Option<ferroui_base::BoxedValue>;
struct Converter(Box<ConverterFn>);
impl PartialEq for Converter {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::eq(self, o)
    }
}
ferroui_base::ferro_markup_type!(class Converter{this:Rc<Converter>,handles:[Rc<Converter>],methods:[fn Invoke(Option<ferroui_base::BoxedValue>)->Option<ferroui_base::BoxedValue> => |this:&Rc<Converter>,v|(this.0)(v)]});
fn converter(
    f: impl Fn(Option<ferroui_base::BoxedValue>) -> Option<ferroui_base::BoxedValue> + 'static,
) -> MarkupDelegate {
    ValueTypes::register_reference::<Converter>();
    MarkupDelegate::for_method(
        Some(Rc::new(Converter(Box::new(f)))),
        Converter::MARKUP,
        &Converter::MARKUP.methods[0],
    )
}
impl PartialEq for Action {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::eq(self, o)
    }
}
ferroui_base::ferro_markup_type!(class Action{this:Rc<Action>,handles:[Rc<Action>],methods:[fn Invoke()=>|this:&Rc<Action>|(this.0)()]});
fn action(f: impl Fn() + 'static) -> MarkupDelegate {
    ValueTypes::register_reference::<Action>();
    MarkupDelegate::for_method(
        Some(Rc::new(Action(Box::new(f)))),
        Action::MARKUP,
        &Action::MARKUP.methods[0],
    )
}
fn clone_one(item: &Control) -> Ref<Control> {
    RibbonMenuCloner::clone(Some(RibbonItemsSource::from_values(vec![Some(
        item.to_ref(),
    )])))
    .iter()
    .next()
    .unwrap()
    .unwrap()
}
fn checked(item: &MenuFlyoutItem) -> bool {
    item.downcast_ref::<RadioMenuFlyoutItem>()
        .map(|r| r.is_checked())
        .unwrap_or_else(|| item.is_checked())
}
fn snapshot(source: &MenuFlyoutItem, target: &MenuFlyoutItem) -> serde_json::Value {
    serde_json::json!({"source":checked(source),"target":checked(target),"text":target.text(),"enabled":target.is_enabled(),"tag":target.tag().and_then(|v|v.downcast_ref::<String>().cloned()),"shortcut":target.keyboard_accelerator_text_override(),"hasInvoker":RibbonMenu::get_invoke(target).is_some(),"commandSame":Rc::ptr_eq(&source.command().unwrap(),&target.command().unwrap()),"parameter":target.command_parameter().and_then(|v|v.downcast_ref::<String>().cloned())})
}
#[test]
fn original_menu_branch_snapshot_invocation_and_radio_corpus() {
    crate::group_tests::run(|| {
        let fixture: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/menus.json")).unwrap();
        for (index, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
            if case["name"] != "clone" {
                continue;
            }
            let log = Rc::new(RefCell::new(Vec::<String>::new()));
            let old = log.clone();
            let source: Ref<MenuFlyoutItem> = match case["kind"].as_str().unwrap() {
                "toggle" => ToggleMenuFlyoutItem::new().upcast(),
                "radio" => {
                    let r = RadioMenuFlyoutItem::new();
                    r.set_group_name(Some(format!("corpus{index}")));
                    r.upcast()
                }
                _ => RibbonMenu::item(
                    Some("factory".into()),
                    Some(action(move || old.borrow_mut().push("old".into()))),
                    None,
                    None,
                )
                .unwrap(),
            };
            source.set_text(Some("_Literal-".into()));
            source.set_is_enabled(!case["disabled"].as_bool().unwrap());
            source.set_tag(Some(Rc::new("tag".to_owned())));
            source.set_keyboard_accelerator_text_override(Some("Localized shortcut".into()));
            let trace = log.clone();
            source.set_command(Some(crate::commands::adapt_core_command(
                ferroribbon_core::commands::RibbonRelayCommand::new(move || {
                    trace.borrow_mut().push("command".into())
                }),
            )));
            source.set_command_parameter(Some(Rc::new("parameter".to_owned())));
            let initial = case["initial"].as_bool().unwrap();
            if let Some(r) = source.downcast_ref::<RadioMenuFlyoutItem>() {
                r.set_is_checked(initial)
            } else if source.is::<ToggleMenuFlyoutItem>() {
                source.set_is_checked(initial)
            }
            let trace = log.clone();
            RibbonMenu::set_invoke(
                &source,
                Some(action(move || trace.borrow_mut().push("new".into()))),
            );
            let trace = log.clone();
            source.add_click_action(move |_, e| {
                e.set_handled(true);
                trace.borrow_mut().push("sourceClick".into());
            });
            let first = clone_one(&source);
            let target = if case["twice"] == true {
                clone_one(&first)
            } else {
                first.clone()
            }
            .cast::<MenuFlyoutItem>()
            .unwrap();
            assert_eq!(snapshot(&source, &target), case["before"], "{case}");
            target.invoke_menu_core();
            assert_eq!(snapshot(&source, &target), case["after"], "{case}");
            source.invoke_menu_core();
            assert_eq!(
                serde_json::json!(checked(&source)),
                case["sourceAfterPress"],
                "{case}"
            );
            assert_eq!(serde_json::json!(&*log.borrow()), case["trace"], "{case}");
        }
    });
}
#[test]
fn menu_clone_defers_input_checks_first_error_and_restarts_with_a_fresh_iterator() {
    crate::group_tests::run(|| {
        let q = RibbonMenuCloner::clone(None);
        let mut cursor = q.iter();
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cursor.next())).is_err());
        // A panic ends this native Rust iteration operation.
        drop(cursor);
        let sub = MenuFlyoutSubItem::new();
        sub.set_text(Some("parent".into()));
        sub.set_is_enabled(false);
        sub.set_tag(Some(Rc::new("tag".to_owned())));
        let leaf = MenuFlyoutItem::new();
        leaf.set_text(Some("leaf".into()));
        sub.items().add(Some(leaf.clone().upcast()));
        let sep = MenuFlyoutSeparator::new();
        sep.set_is_enabled(false);
        sep.set_tag(Some(Rc::new("discard".to_owned())));
        let input = crate::metadata::control_items::RibbonControlItems::new();
        input.add(None);
        input.add(Some(Control::new()));
        input.add(Some(sub.clone().upcast()));
        input.add(Some(sep.upcast()));
        let query =
            RibbonMenuCloner::clone(Some(RibbonItemsSource::from_collection(input.clone())));
        let mut cursor = query.iter();
        input.add(Some(MenuFlyoutItem::new().upcast()));
        let first = cursor
            .next()
            .unwrap()
            .unwrap()
            .cast::<MenuFlyoutSubItem>()
            .unwrap();
        assert_eq!(first.text().as_deref(), Some("parent"));
        assert!(first.is_enabled());
        assert!(first.tag().is_none());
        let child = first.items().get(0).unwrap();
        assert!(!child.ptr_eq(&leaf));
        assert_eq!(
            child.cast::<MenuFlyoutItem>().unwrap().text().as_deref(),
            Some("leaf")
        );
        input.add(Some(MenuFlyoutItem::new().upcast()));
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| cursor.next())).is_err());
        // A panic ends this native Rust iteration operation.
        drop(cursor);
        let again: Vec<_> = query.iter().flatten().collect();
        assert_eq!(again.len(), 4);
        assert!(!again[0].ptr_eq(&first));
        assert!(again[1].is::<MenuFlyoutSeparator>());
        assert!(again[1].is_enabled());
        assert!(again[1].tag().is_none());
    });
}
#[test]
fn source_radio_two_slots_global_rekey_and_weak_lifetime() {
    crate::group_tests::run(|| {
        let a = RadioMenuFlyoutItem::new();
        a.set_is_checked(true);
        a.set_group_name(Some("rekey".into()));
        use ferroui_base::interactivity::RoutedEventArgs;
        a.raise_event(&RoutedEventArgs::with_event(Control::loaded_event()));
        let b = RadioMenuFlyoutItem::new();
        b.set_is_checked(true);
        assert!(a.is_checked());
        assert!(b.is_checked());
        MenuItem::set_is_checked(&a, false);
        assert!(a.is_checked());
        assert!(MenuItem::is_checked(&a));
        a.raise_event(&RoutedEventArgs::with_event(Control::unloaded_event()));
        let c = RadioMenuFlyoutItem::new();
        c.set_group_name(Some("rekey".into()));
        c.set_is_checked(true);
        assert!(a.is_checked());
        assert!(c.is_checked());
        let sub = MenuFlyoutSubItem::new();
        sub.items().add(Some(c.upcast()));
        RadioMenuFlyoutItem::set_are_check_states_enabled(&sub, true);
        RadioMenuFlyoutItem::set_are_check_states_enabled(&sub, false);
        sub.raise_event(&RoutedEventArgs::with_event(Control::loaded_event()));
        assert!(sub.classes().contains(":checked"));
        b.set_is_checked(false);
        assert!(!MenuItem::is_checked(&b));
        let weak = b.downgrade();
        drop(b);
        assert!(weak.upgrade().is_none());
        let again = RadioMenuFlyoutItem::new();
        again.set_group_name(Some("rekey".into()));
        again.set_is_checked(true);
        assert!(again.is_checked());
    });
}
#[test]
fn menu_click_snapshot_handles_current_command_replacement_and_native_route() {
    crate::group_tests::run(|| {
        let menu = MenuFlyoutItem::new();
        let log = Rc::new(RefCell::new(Vec::new()));
        let trace = log.clone();
        menu.set_command(Some(crate::commands::adapt_core_command(
            ferroribbon_core::commands::RibbonRelayCommand::new(move || {
                trace.borrow_mut().push("old")
            }),
        )));
        let trace = log.clone();
        menu.add_click_action(move |this, e| {
            e.set_handled(true);
            trace.borrow_mut().push("click");
            let command_trace = trace.clone();
            this.set_command(Some(crate::commands::adapt_core_command(
                ferroribbon_core::commands::RibbonRelayCommand::new(move || {
                    command_trace.borrow_mut().push("new")
                }),
            )));
            let trace = trace.clone();
            this.add_click_action(move |_, _| trace.borrow_mut().push("late"));
        });
        let trace = log.clone();
        menu.add_click_action(move |_, _| trace.borrow_mut().push("second"));
        ferroui_controls::automation::peers::MenuItemAutomationPeer::new(&menu)
            .invoke()
            .unwrap();
        assert_eq!(&*log.borrow(), &["click", "second", "new"]);
        log.borrow_mut().clear();
        menu.invoke_menu_core();
        assert_eq!(&*log.borrow(), &["click", "second", "late", "new"]);
    });
}
#[test]
fn actual_overflow_entries_snapshot_fields_keep_source_and_release_without_self_cycle() {
    crate::group_tests::run(|| {
        let source = RibbonButton::new();
        source.set_label(Some("Copy".into()));
        source.set_shortcut(Some("  ".into()));
        let count = Rc::new(std::cell::Cell::new(0));
        let calls = count.clone();
        source.set_command(Some(crate::commands::adapt_core_command(
            ferroribbon_core::commands::RibbonRelayCommand::new(move || calls.set(calls.get() + 1)),
        )));
        let q = source.create_overflow_menu_items_core();
        let entry = q
            .iter()
            .next()
            .unwrap()
            .unwrap()
            .cast::<MenuFlyoutItem>()
            .unwrap();
        assert!(q.iter().next().unwrap().unwrap().ptr_eq(&entry));
        source.set_label(Some("Changed".into()));
        assert_eq!(entry.text().as_deref(), Some("Copy"));
        assert_eq!(entry.keyboard_accelerator_text_override(), "  ");
        let source_weak = source.downgrade();
        let entry_weak = entry.downgrade();
        drop(source);
        assert!(source_weak.upgrade().is_some());
        entry.invoke_menu_core();
        assert_eq!(count.get(), 1);
        drop(q);
        drop(entry);
        assert!(entry_weak.upgrade().is_none());
        assert!(source_weak.upgrade().is_none());
        let toggle = RibbonToggleButton::new();
        let q = toggle.create_overflow_menu_items_core();
        let entry = q
            .iter()
            .next()
            .unwrap()
            .unwrap()
            .cast::<MenuFlyoutItem>()
            .unwrap();
        assert!(entry.is::<ToggleMenuFlyoutItem>());
        assert!(!entry.is_checked());
        entry.invoke_menu_core();
        assert_eq!(toggle.is_checked(), Some(true));
        assert!(entry.is_checked());
        let weak = entry.downgrade();
        drop(q);
        drop(entry);
        assert!(weak.upgrade().is_none());
    });
}
#[test]
fn menu_icons_convert_for_attached_non_menu_targets_and_clone_font_glyph_only() {
    crate::group_tests::run(|| {
        let icon = FontIcon::new();
        icon.set_glyph(Some("\u{e700}".into()));
        icon.set_font_size(42.);
        let converted = RibbonItemHelper::create_menu_icon(Some(Control::boxed(icon.clone())))
            .unwrap()
            .unwrap()
            .cast::<FontIcon>()
            .unwrap();
        assert_eq!(converted.glyph(), icon.glyph());
        assert_ne!(converted.font_size(), 42.);
        assert!(!converted.ptr_eq(&icon));
        let menu = MenuFlyoutItem::new();
        RibbonMenu::set_icon(&menu, Some(Rc::new("\u{e700}".to_owned())));
        assert!(
            menu.icon()
                .and_then(|v| ValueTypes::as_object(&*v))
                .unwrap()
                .is::<FontIcon>()
        );
        let calls = Rc::new(std::cell::Cell::new(0));
        let count = calls.clone();
        let result = icon.clone();
        RibbonItemHelper::set_menu_icon_converter(Some(converter(move |_| {
            count.set(count.get() + 1);
            Some(Control::boxed(result.clone()))
        })));
        RibbonMenu::set_icon(&Control::new(), Some(Rc::new("unknown".to_owned())));
        assert_eq!(calls.get(), 1);
        assert!(RibbonItemHelper::create_menu_icon(None).unwrap().is_none());
        assert_eq!(calls.get(), 1);
        let result = RibbonItemHelper::create_menu_icon(Some(Rc::new("unknown".to_owned())))
            .unwrap()
            .unwrap();
        assert!(result.ptr_eq(&icon));
        RibbonItemHelper::set_menu_icon_converter(Some(converter(|_| None)));
        assert!(
            RibbonItemHelper::create_menu_icon(Some(Rc::new("\u{e700}".to_owned())))
                .unwrap()
                .unwrap()
                .is::<FontIcon>()
        );
        RibbonItemHelper::set_menu_icon_converter(None);
        // Selected CadFeatureTests.MenuIcons source scenarios, excluding the
        // explicitly missing source image-loading service.
        assert!(
            RibbonItemHelper::create_menu_icon(Some(Rc::new("M0,0 L10,0 L10,10 Z".to_owned())))
                .unwrap()
                .unwrap()
                .is::<ferroui_controls::PathIcon>()
        );
        assert!(
            RibbonItemHelper::create_menu_icon(Some(Rc::new("[stroke=1.5]M0,0 L10,10".to_owned())))
                .unwrap()
                .is_none()
        );
    });
}
#[test]
fn null_action_is_stored_but_only_original_factory_handler_throws() {
    crate::group_tests::run(|| {
        let menu = RibbonMenu::item(None, None, None, None).unwrap();
        assert!(menu.text().is_none());
        RibbonMenu::set_invoke(&menu, None);
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| menu.invoke_menu_core()))
                .is_err()
        );
        let copy = clone_one(&menu).cast::<MenuFlyoutItem>().unwrap();
        assert!(RibbonMenu::get_invoke(&copy).is_none());
        copy.invoke_menu_core();
    });
}
#[test]
fn submenu_mutations_keep_unaffected_items_and_release_external_collection_subscription() {
    crate::group_tests::run(|| {
        use ferroui_controls::ItemsControl;
        let sub = MenuFlyoutSubItem::new();
        let collection = sub.items();
        let a = MenuFlyoutItem::new();
        let b = MenuFlyoutItem::new();
        collection.add(Some(a.clone().upcast()));
        collection.insert(0, Some(b.clone().upcast()));
        let native = ItemsControl::items(&sub);
        assert!(
            Control::from_boxed(&native.view().get_at(1).unwrap())
                .unwrap()
                .ptr_eq(&a)
        );
        collection.move_item(0, 1);
        assert!(
            Control::from_boxed(&native.view().get_at(1).unwrap())
                .unwrap()
                .ptr_eq(&b)
        );
        collection.set(0, None);
        assert!(native.view().get_at(0).is_none());
        collection.remove_at(0);
        assert_eq!(native.view().count(), 1);
        let weak = sub.downgrade();
        drop(native);
        drop(sub);
        assert!(weak.upgrade().is_none());
        collection.clear();
        assert!(collection.is_empty());
    });
}
#[test]
fn native_menu_theme_keeps_literal_labels_checked_visuals_and_shortcut_text() {
    crate::group_tests::run(|| {
        use ferroui_base::{Rect, Size};
        use ferroui_controls::{StackPanel, TextBlock, Window};
        let item = ToggleMenuFlyoutItem::new();
        item.set_text(Some("-".into()));
        item.set_is_checked(true);
        item.set_keyboard_accelerator_text_override(Some("Localized shortcut".into()));
        let panel = StackPanel::new();
        panel.children().add(item.clone().upcast::<Control>());
        let w = Window::new();
        w.set_content(Some(Control::boxed(panel)));
        w.show();
        w.measure(Size::new(350., 100.));
        w.arrange(Rect::new(0., 0., 350., 100.));
        assert!(!item.classes().contains(":separator"));
        assert!(item.classes().contains(":checked"));
        assert!(item.classes().contains(":toggle"));
        let header = Control::from_boxed(&item.header().unwrap())
            .unwrap()
            .cast::<TextBlock>()
            .unwrap();
        assert_eq!(header.text().as_deref(), Some("-"));
        let shortcut = item
            .get_visual_descendants()
            .find_map(|c| {
                c.cast::<TextBlock>()
                    .filter(|t| t.name().as_deref() == Some("PART_InputGestureText"))
            })
            .unwrap();
        assert_eq!(shortcut.text().as_deref(), Some("Localized shortcut"));
        item.set_keyboard_accelerator_text_override(Some("Changed".into()));
        assert_eq!(shortcut.text().as_deref(), Some("Changed"));
        item.set_text(Some("_Literal".into()));
        assert_eq!(header.text().as_deref(), Some("_Literal"));
        ferroui_controls::automation::peers::MenuItemAutomationPeer::new(&item)
            .invoke()
            .unwrap();
        assert!(!item.is_checked());
        w.close();
    });
}
