//! Original command partial/traversal corpus plus real native owner, popup and
//! catalog integration. Test checkable item is a native RibbonButton subclass.
use crate::controls::*;
use ferroribbon_core::{
    RibbonString,
    commands::{RibbonCommandCatalog, RibbonCommandDescriptor, RibbonRelayCommand},
};
use ferroui_base::{
    BoxedValue, FerroObject, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl,
    data::core::{ValueType, ValueTypes},
    ferro_class, ferro_class_info, ferro_impl_classes,
    input::InputElementImpl,
    instantiate,
    interactivity::InteractiveImpl,
    layout::LayoutableImpl,
};
use ferroui_controls::{
    ButtonImpl, ContentControlImpl, Control, ControlImpl, primitives::TemplatedControlImpl,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
#[repr(C)]
struct CheckedItem {
    base: RibbonButton,
    checked: Cell<Option<bool>>,
}
ferro_class!(CheckedItem:RibbonButton);
ferro_class_info!(CheckedItem{interfaces:[Rc<dyn IRibbonCheckable> => |v|Rc::new(Checked(v)) as Rc<dyn IRibbonCheckable>]});
ferro_impl_classes!(CheckedItem:FerroObjectImpl,StyledElementImpl,VisualImpl,InteractiveImpl,InputElementImpl,ControlImpl,ContentControlImpl,LayoutableImpl,TemplatedControlImpl,ButtonImpl,RibbonButtonImpl);
struct Checked(Ref<CheckedItem>);
impl IRibbonCheckable for Checked {
    fn as_object(&self) -> Ref<FerroObject> {
        self.0.clone().upcast()
    }
    fn is_checked(&self) -> Option<bool> {
        self.0.checked.get()
    }
    fn set_is_checked(&self, value: Option<bool>) {
        self.0.checked.set(value)
    }
}
fn item(name: &str, command: Option<&str>) -> Ref<CheckedItem> {
    let i = instantiate(CheckedItem {
        base: RibbonButton::construct(),
        checked: Cell::new(None),
    });
    i.set_name(Some(name.into()));
    i.set_command_id(command.map(Into::into));
    i
}
fn make() -> Ref<Ribbon> {
    let root = Ribbon::new();
    let tab = RibbonTab::new();
    let group = RibbonGroup::new();
    let plain = item("plain", Some("paste"));
    plain.set_id(Some("plain-id".into()));
    group.items().add(Some(plain.upcast()));
    let hidden = item("hidden-top", Some("PASTE"));
    hidden.set_is_visible(false);
    group.items().add(Some(hidden.upcast()));
    let container = RibbonStackPanel::new();
    for (name, id, visible) in [
        ("sigma", "σ", true),
        ("hidden-child", "ς", false),
        ("long-s", "ſ", true),
        ("deseret", "𐐨", true),
    ] {
        let i = item(name, Some(id));
        i.set_is_visible(visible);
        container.items().add(Some(i.upcast()));
    }
    group.items().add(Some(container.upcast()));
    let label = RibbonLabel::new();
    label.set_name(Some("label".into()));
    group.items().add(Some(label.upcast()));
    let sep = RibbonSeparator::new();
    sep.set_name(Some("separator".into()));
    group.items().add(Some(sep.upcast()));
    let text = ferroui_controls::TextBlock::new();
    text.set_name(Some("text".into()));
    group.items().add(Some(text.upcast()));
    let control = ferroui_controls::Button::new();
    control.set_name(Some("control".into()));
    group.items().add(Some(control.upcast()));
    group
        .slide_out_items()
        .add(Some(item("expanded", Some("paste")).upcast()));
    tab.groups().add(Some(group));
    root.tabs().add(Some(tab));
    root.tab_strip_items()
        .add(Some(item("strip", None).upcast()));
    root
}
fn names(query: ferroribbon_core::model::RibbonItemsSource<Ref<Control>>) -> Vec<String> {
    query
        .iter()
        .map(|i| i.name().unwrap().to_string())
        .collect()
}
#[test]
fn unchanged_command_partial_and_traversal_corpus() {
    crate::group_tests::run(|| {
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/ribbon-commands.json"))
                .unwrap();
        for c in corpus["corpus"]["cases"].as_array().unwrap() {
            let r = make();
            let id = c["id"].as_str().map(RibbonString::from);
            assert_eq!(serde_json::json!(names(r.get_all_items())), c["all"]);
            assert_eq!(
                serde_json::json!(names(r.find_items_by_command(id.clone()))),
                c["matches"]
            );
            let enabled = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                r.set_command_enabled(id.clone(), false)
            }));
            let checked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                r.set_command_checked(id.clone(), true)
            }));
            assert_eq!(
                usize::from(enabled.is_err()) + usize::from(checked.is_err()),
                c["errors"].as_array().unwrap().len()
            );
            let before=r.get_all_items().iter().filter_map(|v|v.cast::<CheckedItem>()).map(|v|serde_json::json!({"Name":v.name(),"IsEnabled":v.is_enabled(),"IsChecked":v.checked.get()})).collect::<Vec<_>>();
            assert_eq!(serde_json::json!(before), c["before"], "case {:?}", id);
            let later = item("later", c["id"].as_str());
            later.set_id(Some("later-id".into()));
            let group = r.tabs().get(0).unwrap().groups().get(0).unwrap();
            group.items().add(Some(later.clone().upcast()));
            r.apply_command_state(&later);
            assert_eq!(
                serde_json::json!({"IsEnabled":later.is_enabled(),"IsChecked":later.checked.get()}),
                c["later"]
            );
            assert_eq!(
                r.find_item(None)
                    .and_then(|v| v.name().map(|s| s.to_string())),
                c["foundNull"].as_str().map(String::from)
            );
            assert_eq!(
                r.find_item(Some("plain-id".into()))
                    .unwrap()
                    .name()
                    .as_deref(),
                c["found"].as_str()
            );
            assert!(r.find_location(&later).1.unwrap().ptr_eq(&group));
        }
        let r = make();
        let mut it = r.get_all_items().iter();
        assert!(it.next().is_some());
        r.tabs()
            .get(0)
            .unwrap()
            .groups()
            .get(0)
            .unwrap()
            .items()
            .add(Some(item("mutated", Some("paste")).upcast()));
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| it.next())).is_err());
        drop(it);
        let container = RibbonStackPanel::new();
        container.items().add(None);
        let mut it = RibbonGroup::flatten(Some(container.upcast())).iter();
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| it.next())).is_err());
        drop(it);
    });
}
#[test]
fn command_fallback_catalog_replacement_preserves_user_commands() {
    crate::group_tests::run(|| {
        let root = make();
        root.set_command_enabled(Some("paste".into()), false);
        let button = root
            .find_item(Some("plain-id".into()))
            .unwrap()
            .cast::<RibbonButton>()
            .unwrap();
        RibbonItemHelper::resolve_command(&button).unwrap();
        assert!(!button.is_enabled());
        let catalog = RibbonCommandCatalog::new();
        let runs = Rc::new(Cell::new(0));
        let n = runs.clone();
        let command = RibbonRelayCommand::new(move || n.set(n.get() + 1));
        let d =
            RibbonCommandDescriptor::new(Some("paste".into()), Some("Paste".into()), Some(command));
        catalog.register(Some(d.clone()));
        root.set_command_catalog(Some(catalog.clone()));
        assert!(button.command().is_some());
        assert!(button.is_enabled());
        button.perform_click();
        assert_eq!(runs.get(), 1);
        let user = RibbonRelayCommand::new(|| {});
        let native = crate::commands::adapt_core_command(user);
        button.set_command(Some(native.clone()));
        catalog.unregister(Some(&"paste".into()));
        root.set_command_catalog(None);
        assert!(Rc::ptr_eq(&button.command().unwrap(), &native));
    });
}
#[test]
fn native_owner_property_accepts_self_and_ancestor_and_weakly_releases_dropped_owner() {
    crate::group_tests::run(|| {
        let child = RibbonButton::new();
        let weak;
        {
            let root = Ribbon::new();
            weak = root.downgrade();
            RibbonItemHelper::set_owner(&root, Some(root.item_owner())).unwrap();
            assert!(
                RibbonItemHelper::get_owner(&root)
                    .unwrap()
                    .as_object()
                    .ptr_eq(&root)
            );
            RibbonItemHelper::set_owner(&child, Some(root.item_owner())).unwrap();
            let value = Control::boxed(root.clone());
            let owner =
                ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn IRibbonItemOwner>>()).unwrap();
            assert!(
                owner
                    .downcast_ref::<Rc<dyn IRibbonItemOwner>>()
                    .unwrap()
                    .owner_ribbon()
                    .unwrap()
                    .ptr_eq(&root)
            );
        }
        assert!(
            weak.upgrade().is_none(),
            "native weak backedge must not create a strong self-cycle"
        );
        assert!(
            RibbonItemHelper::get_owner(&child).is_none(),
            "documented ElementRef lifetime difference from CLR"
        );
    });
}
type ItemEventFn = dyn Fn(Rc<RibbonItemInvokedEventArgs>);
struct ItemHandler(Box<ItemEventFn>);
impl PartialEq for ItemHandler {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class ItemHandler{this:Rc<ItemHandler>,handles:[Rc<ItemHandler>],methods:[fn Invoke(Option<BoxedValue>,Rc<RibbonItemInvokedEventArgs>) => |h:&Rc<ItemHandler>,_sender,args|(h.0)(args)]});
pub(crate) fn invocation(
    f: impl Fn(Rc<RibbonItemInvokedEventArgs>) + 'static,
) -> ferroui_base::metadata::MarkupDelegate {
    use ferroui_base::metadata::MarkupTyped;
    ValueTypes::register_reference::<ItemHandler>();
    ferroui_base::metadata::MarkupDelegate::for_method(
        Some(Rc::new(ItemHandler(Box::new(f)))),
        ItemHandler::MARKUP,
        &ItemHandler::MARKUP.methods[0],
    )
}
#[test]
fn item_invoked_keeps_the_source_id_snapshot_through_handlers_and_later_changes() {
    use ferroui_base::metadata::{MarkupDelegate, MarkupTyped, from_markup_value};

    fn exercise(add: impl Fn(MarkupDelegate), raise: impl Fn(Ref<Control>)) {
        let source = RibbonButton::new();
        source.set_id(Some("start".into()));
        let read = RibbonItemInvokedEventArgs::MARKUP
            .properties
            .iter()
            .find(|property| property.name == "ItemId")
            .unwrap()
            .get
            .unwrap();
        let markup_id = move |args: &Rc<RibbonItemInvokedEventArgs>| {
            let value = read(&[Some(Rc::new(args.clone()))]).unwrap();
            from_markup_value::<Option<String>>(&value).unwrap()
        };
        let retained = Rc::new(RefCell::new(None));
        add(invocation(move |args| {
            assert_eq!(markup_id(&args).as_deref(), Some("start"));
            args.item
                .cast::<RibbonButton>()
                .unwrap()
                .set_id(Some("renamed".into()));
        }));
        add(invocation({
            let retained = retained.clone();
            move |args| {
                assert_eq!(markup_id(&args).as_deref(), Some("start"));
                assert_eq!(args.item_id, Some("start".into()));
                assert_eq!(
                    args.item.cast::<RibbonButton>().unwrap().id(),
                    Some("renamed".into())
                );
                retained.replace(Some(args));
            }
        }));
        raise(source.clone().upcast());
        let args = retained.take().unwrap();
        assert!(args.item.ptr_eq(&source));
        assert_eq!(args.command_id, Some("command".into()));
        assert_eq!(
            args.parameter
                .as_ref()
                .and_then(|p| p.downcast_ref::<i32>()),
            Some(&42)
        );
        source.set_id(None);
        assert_eq!(markup_id(&args).as_deref(), Some("start"));
    }

    crate::group_tests::run(|| {
        let ribbon = Ribbon::new();
        exercise(
            |handler| ribbon.add_item_invoked(Some(handler)),
            |item| ribbon.on_item_invoked(item, Some("command".into()), Some(Rc::new(42_i32))),
        );
        let toolbar = RibbonToolBar::new();
        exercise(
            |handler| toolbar.add_item_invoked(Some(handler)),
            |item| toolbar.on_item_invoked(item, Some("command".into()), Some(Rc::new(42_i32))),
        );
        let status = RibbonStatusBar::new();
        exercise(
            |handler| status.add_item_invoked(Some(handler)),
            |item| status.on_item_invoked(item, Some("command".into()), Some(Rc::new(42_i32))),
        );
    });
}
#[test]
fn item_invoked_uses_group_id_and_leaves_plain_controls_unidentified() {
    crate::group_tests::run(|| {
        let ribbon = Ribbon::new();
        let retained = Rc::new(RefCell::new(Vec::new()));
        ribbon.add_item_invoked(Some(invocation({
            let retained = retained.clone();
            move |args| retained.borrow_mut().push(args)
        })));

        let group = RibbonGroup::new();
        group.set_id(Some("group".into()));
        ribbon.on_item_invoked(group.clone().upcast(), None, None);
        group.set_id(Some("renamed-group".into()));
        let plain = ferroui_controls::Button::new();
        ribbon.on_item_invoked(plain.clone().upcast(), None, None);

        let args = retained.borrow();
        assert!(args[0].item.ptr_eq(&group));
        assert_eq!(args[0].item_id, Some("group".into()));
        assert!(args[1].item.ptr_eq(&plain));
        assert_eq!(args[1].item_id, None);
        assert!(ribbon.search_engine().recent().is_empty());
    });
}
fn root_group() -> (Ref<Ribbon>, Ref<RibbonGroup>) {
    let r = Ribbon::new();
    r.set_is_application_button_visible(false);
    r.set_is_display_options_button_visible(false);
    let tab = RibbonTab::new();
    tab.set_id(Some("home".into()));
    tab.set_header(Some("Home".into()));
    let group = RibbonGroup::new();
    group.set_header(Some("Tools".into()));
    tab.groups().add(Some(group.clone()));
    r.tabs().add(Some(tab));
    (r, group)
}
#[test]
fn queued_catalog_change_survives_unsubscribe_and_resolves_current_items() {
    crate::group_tests::run(|| {
        let (r, g) = root_group();
        let b = item("early", Some("paste"));
        g.items().add(Some(b.clone().upcast()));
        let catalog = RibbonCommandCatalog::new();
        r.set_command_catalog(Some(catalog.clone()));
        assert_eq!(catalog.commands_changed().handler_count(), 0);
        let w = crate::ribbon_tests::mount(&r);
        assert_eq!(catalog.commands_changed().handler_count(), 1);
        let d = RibbonCommandDescriptor::new(
            Some("paste".into()),
            Some("Paste".into()),
            Some(RibbonRelayCommand::new(|| {})),
        );
        catalog.register(Some(d));
        assert!(b.command().is_none());
        let late = item("late", Some("paste"));
        g.items().add(Some(late.clone().upcast()));
        r.detach_command_catalog();
        assert_eq!(catalog.commands_changed().handler_count(), 0);
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(b.command().is_some() && late.command().is_some());
        r.attach_command_catalog();
        assert_eq!(catalog.commands_changed().handler_count(), 1);
        w.close();
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert_eq!(catalog.commands_changed().handler_count(), 0);
    });
}
#[test]
fn actual_root_linked_item_and_collapsed_popup_report_original_once() {
    crate::group_tests::run(|| {
        let (r, g) = root_group();
        let source = RibbonButton::new();
        source.set_id(Some("original".into()));
        source.set_command_id(Some("action".into()));
        source.set_label(Some("Run".into()));
        let trace = Rc::new(RefCell::new(Vec::new()));
        let log = trace.clone();
        let command = RibbonRelayCommand::new_with_parameter(
            move |p| {
                assert_eq!(p.unwrap().downcast_ref::<i32>(), Some(&7));
                log.borrow_mut().push("command");
            },
            None,
        );
        source.set_command(Some(crate::commands::adapt_core_command(command)));
        source.set_command_parameter(Some(Rc::new(7_i32)));
        g.items().add(Some(source.clone().upcast()));
        let log = trace.clone();
        let weak = source.downgrade();
        r.add_item_invoked(Some(invocation(move |args| {
            assert!(args.item.ptr_eq(&weak.upgrade().unwrap()));
            assert_eq!(args.item_id, Some("original".into()));
            assert_eq!(args.command_id, Some("action".into()));
            assert_eq!(
                args.parameter.as_ref().unwrap().downcast_ref::<i32>(),
                Some(&7)
            );
            log.borrow_mut().push("event");
        })));
        let w = crate::ribbon_tests::mount(&r);
        let copy = source
            .create_linked_copy_core()
            .unwrap()
            .cast::<RibbonButton>()
            .unwrap();
        copy.perform_click();
        assert_eq!(&*trace.borrow(), &["event", "command"]);
        assert_eq!(r.search_engine().recent().get(0), Some("original".into()));
        trace.borrow_mut().clear();
        g.apply_state(
            ferroribbon_core::RibbonGroupState::Collapsed,
            false,
            crate::primitives::RibbonMetrics::comfortable(),
            None,
        );
        g.open_popup();
        assert!(g.is_popup_open());
        source.perform_click();
        assert!(!g.is_popup_open());
        assert_eq!(&*trace.borrow(), &["event", "command"]);
        w.close();
    });
}
#[test]
fn expanded_panel_reparents_same_controls_pins_and_survives_collapsed_popup() {
    crate::group_tests::run(|| {
        use ferroui_base::{
            Rect, Size,
            input::{InputElement, Key, KeyEventArgs},
        };
        let (r, g) = root_group();
        let ordinary = RibbonButton::new();
        ordinary.set_label(Some("Ordinary".into()));
        g.items().add(Some(ordinary.upcast()));
        let extra = RibbonButton::new();
        extra.set_label(Some("Advanced".into()));
        extra.set_size(ferroribbon_core::RibbonItemSize::Large);
        g.slide_out_items().add(Some(extra.clone().upcast()));
        let w = crate::ribbon_tests::mount(&r);
        assert!(g.slide_out_button().unwrap().is_visible());
        g.open_slide_out();
        assert!(g.is_slide_out_open());
        assert_eq!(
            extra.current_size(),
            ferroribbon_core::RibbonItemSize::Medium
        );
        let panel = extra.visual_parent().unwrap();
        assert!(extra.ptr_eq(&g.slide_out_items().get(0).unwrap()));
        g.set_is_slide_out_pinned(true);
        g.close_slide_out();
        assert!(!g.is_slide_out_open() && g.is_slide_out_pinned());
        g.open_slide_out();
        let mut e = KeyEventArgs::new();
        e.key = Key::Escape;
        e.set_routed_event(Some(InputElement::key_down_event()));
        extra.raise_event(&e);
        assert!(e.handled());
        assert!(!g.is_slide_out_open() && !g.is_slide_out_pinned());
        assert!(g.slide_out_button().unwrap().is_focused());
        g.apply_state(
            ferroribbon_core::RibbonGroupState::Collapsed,
            false,
            crate::primitives::RibbonMetrics::comfortable(),
            None,
        );
        g.open_popup();
        w.measure(Size::new(1400., 300.));
        w.arrange(Rect::new(0., 0., 1400., 300.));
        assert!(g.is_popup_open());
        assert!(extra.visual_parent().unwrap().ptr_eq(&panel));
        assert!(panel.get_logical_parent().is_some());
        g.close_popup();
        assert!(panel.get_logical_parent().is_none());
        assert!(extra.ptr_eq(&g.slide_out_items().get(0).unwrap()));
        w.close();
    });
}
#[test]
fn replacing_group_template_moves_slide_out_click_to_the_new_part() {
    use ferroui_base::{interactivity::RoutedEventArgs, threading::Dispatcher};
    use ferroui_controls::Button;

    crate::group_tests::run(|| {
        let (ribbon, group) = root_group();
        group
            .slide_out_items()
            .add(Some(RibbonButton::new().upcast()));
        let window = crate::ribbon_tests::mount(&ribbon);
        let click = |button: &Button| {
            let args = RoutedEventArgs::new();
            args.set_routed_event(Some(Button::click_event()));
            button.raise_event(&args);
        };

        let template = group.template();
        let old_button = group.slide_out_button().unwrap();
        click(&old_button);
        assert!(!group.is_slide_out_open());
        Dispatcher::ui_thread().run_jobs(None);
        assert!(group.is_slide_out_open());
        group.set_is_slide_out_pinned(true);

        group.set_template(None);
        group.apply_template();
        assert!(group.slide_out_button().is_none());
        group.set_template(template);
        group.apply_template();
        let new_button = group.slide_out_button().unwrap();
        assert!(!new_button.ptr_eq(&old_button));

        click(&old_button);
        assert!(group.is_slide_out_open() && group.is_slide_out_pinned());
        click(&new_button);
        assert!(!group.is_slide_out_open() && !group.is_slide_out_pinned());
        click(&new_button);
        assert!(!group.is_slide_out_open());
        Dispatcher::ui_thread().run_jobs(None);
        assert!(group.is_slide_out_open());
        window.close();
    });
}

#[cfg(windows)]
#[test]
fn slide_out_uses_opening_pin_state_for_outside_pointer_dismissal() {
    use ferroui_base::{
        Point,
        input::{
            KeyModifiers, Pointer, PointerPointProperties, PointerPressedEventArgs, PointerType,
        },
    };
    use ferroui_controls::primitives::LightDismissOverlayLayer;

    crate::group_tests::run(|| {
        let (ribbon, group) = root_group();
        group
            .slide_out_items()
            .add(Some(RibbonButton::new().upcast()));
        let window = crate::ribbon_tests::mount(&ribbon);
        let overlay = LightDismissOverlayLayer::get_light_dismiss_overlay_layer(&window).unwrap();
        let click_outside = || {
            let args = PointerPressedEventArgs::new(
                window.clone(),
                Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true),
                &window,
                Point::new(10., 10.),
                0,
                PointerPointProperties {
                    is_left_button_pressed: true,
                    ..PointerPointProperties::NONE
                },
                KeyModifiers::NONE,
                1,
            );
            overlay.raise_event(&args);
        };

        group.open_slide_out();
        group.set_is_slide_out_pinned(true);
        assert!(group.is_slide_out_open() && overlay.is_visible());
        click_outside();
        assert!(!group.is_slide_out_open() && group.is_slide_out_pinned());

        group.open_slide_out();
        assert!(group.is_slide_out_open() && !overlay.is_visible());
        group.set_is_slide_out_pinned(false);
        click_outside();
        assert!(group.is_slide_out_open() && !overlay.is_visible());
        group.close_slide_out();
        window.close();
    });
}

#[test]
fn header_arrows_home_end_and_down_use_source_focus_and_rtl_order() {
    crate::group_tests::run(|| {
        use ferroui_base::{
            input::{InputElement, Key, KeyEventArgs},
            media::FlowDirection,
        };
        let (r, g) = root_group();
        let b = RibbonButton::new();
        g.items().add(Some(b.clone().upcast()));
        let second = RibbonTab::new();
        second.set_id(Some("second".into()));
        second.set_header(Some("Second".into()));
        r.tabs().add(Some(second));
        let w = crate::ribbon_tests::mount(&r);
        let key = |header: &RibbonTabHeader, key| {
            let mut e = KeyEventArgs::new();
            e.key = key;
            e.set_routed_event(Some(InputElement::key_down_event()));
            header.raise_event(&e);
            assert!(e.handled());
        };
        let first = r.tab_headers()[0].clone();
        key(&first, Key::Right);
        assert_eq!(r.selected_tab_id().as_deref(), Some("second"));
        assert!(r.tab_headers()[1].is_focused());
        key(&r.tab_headers()[1], Key::Home);
        assert_eq!(r.selected_tab_id().as_deref(), Some("home"));
        key(&first, Key::Down);
        assert!(b.is_focused());
        r.set_flow_direction(FlowDirection::RightToLeft);
        key(&first, Key::Left);
        assert_eq!(r.selected_tab_id().as_deref(), Some("second"));
        key(&r.tab_headers()[1], Key::End);
        assert_eq!(r.selected_tab_id().as_deref(), Some("second"));
        w.close();
    });
}
#[test]
fn root_lifetime_baseline_and_popup_matrix() {
    crate::group_tests::run(|| {
        let bare = {
            let r = Ribbon::new();
            r.downgrade()
        };
        assert!(bare.upgrade().is_none(), "bare root");
        let unmounted = {
            let (r, _g) = root_group();
            r.downgrade()
        };
        assert!(unmounted.upgrade().is_none(), "unmounted root");
        let mounted = {
            let (r, _g) = root_group();
            let weak = r.downgrade();
            let w = crate::ribbon_tests::mount(&r);
            w.close();
            w.set_content(None);
            weak
        };
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(mounted.upgrade().is_none(), "mounted root");
        let expanded = {
            let (r, g) = root_group();
            g.slide_out_items().add(Some(RibbonButton::new().upcast()));
            let weak = r.downgrade();
            let w = crate::ribbon_tests::mount(&r);
            g.open_slide_out();
            g.close_slide_out();
            w.close();
            w.set_content(None);
            weak
        };
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(expanded.upgrade().is_none(), "expanded popup root");
    });
}

#[cfg(feature = "source-button-semantics-reproducer")]
#[test]
fn source_explicit_owner_roots_an_externally_retained_item_owner() {
    crate::group_tests::run(|| {
        let corpus: serde_json::Value =
            serde_json::from_str(include_str!("../../tests/fixtures/ribbon-commands.json"))
                .unwrap();
        assert_eq!(corpus["corpus"]["ownerLifetime"]["kept"], true);
        let item = RibbonButton::new();
        let weak = {
            let root = Ribbon::new();
            let weak = root.downgrade();
            let owner = ValueTypes::try_cast(
                &Control::boxed(root),
                ValueType::of::<Rc<dyn IRibbonItemOwner>>(),
            )
            .unwrap();
            RibbonItemHelper::set_owner(
                &item,
                Some(
                    owner
                        .downcast_ref::<Rc<dyn IRibbonItemOwner>>()
                        .unwrap()
                        .clone(),
                ),
            )
            .unwrap();
            weak
        };
        assert!(
            weak.upgrade().is_some(),
            "source Owner is a strong CLR property; native ElementRef drops an owner held only by an external item"
        );
        RibbonItemHelper::set_owner(&item, None).unwrap();
        assert!(weak.upgrade().is_none());
    });
}

#[test]
fn launcher_root_invocation_preserves_null_empty_and_nonempty_group_ids() {
    crate::group_tests::run(|| {
        for id in [None, Some(""), Some("tools")] {
            let (root, group) = root_group();
            group.set_id(id.map(Into::into));
            let observed = Rc::new(RefCell::new(Vec::new()));
            let log = observed.clone();
            let weak = group.downgrade();
            root.add_item_invoked(Some(invocation(move |args| {
                assert!(args.item.ptr_eq(&weak.upgrade().unwrap()));
                log.borrow_mut()
                    .push((args.item_id.clone(), args.command_id.clone()));
            })));
            group.open_dialog_launcher().unwrap();
            assert_eq!(
                &*observed.borrow(),
                &[(
                    id.map(Into::into),
                    id.map(|id| format!("{id}.launcher").into())
                )]
            );
        }
    });
}
