//! Root backstage source corpus and native popup/keyboard/window integration.
use crate::controls::{
    Ribbon, RibbonBackstage, RibbonBackstageItem, RibbonHandledEventArgs, RibbonItemHelper,
    RibbonTab,
};
use ferroui_base::{
    BoxedValue, Point, Rect, Ref, Size, StaticType, StyledElement, Thickness,
    controls::ResourceDictionary,
    data::{BindingPriority, core::ValueTypes},
    input::{InputElement, Key, KeyEventArgs},
    metadata::{MarkupDelegate, MarkupTyped},
    styling::{ControlTheme, ThemeVariant},
    threading::Dispatcher,
    utilities::EventArgs,
};
use ferroui_controls::{
    Application, Border, Button, Control, Flyout, Panel, TextBlock, TopLevel, Window,
    WindowResizeReason,
};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
fn check(name: &str, mut actual: Value) {
    let expected: Value =
        serde_json::from_str(include_str!("../../tests/fixtures/root-backstage.json")).unwrap();
    let wanted = expected["corpus"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap();
    // Compare exact double values, independent of JSON's integral-number spelling.
    fn doubles(value: &mut Value) {
        match value {
            Value::Number(n) => *value = json!(n.as_f64().unwrap()),
            Value::Array(a) => a.iter_mut().for_each(doubles),
            Value::Object(o) => o.values_mut().for_each(doubles),
            _ => {}
        }
    }
    let mut wanted = wanted.clone();
    doubles(&mut wanted);
    doubles(&mut actual);
    assert_eq!(actual, wanted, "source root backstage case {name}");
}
struct Handler(Box<dyn Fn()>);
impl PartialEq for Handler {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class Handler{this:Rc<Handler>,handles:[Rc<Handler>],methods:[fn Invoke(Option<BoxedValue>,EventArgs)=>|h:&Rc<Handler>,_,_|(h.0)()]});
fn handler(f: impl Fn() + 'static) -> MarkupDelegate {
    ValueTypes::register_reference::<Handler>();
    MarkupDelegate::for_method(
        Some(Rc::new(Handler(Box::new(f)))),
        Handler::MARKUP,
        &Handler::MARKUP.methods[0],
    )
}
struct ClickHandler(Box<dyn Fn(Rc<RibbonHandledEventArgs>)>);
impl PartialEq for ClickHandler {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class ClickHandler{this:Rc<ClickHandler>,handles:[Rc<ClickHandler>],methods:[fn Invoke(Option<BoxedValue>,Rc<RibbonHandledEventArgs>)=>|h:&Rc<ClickHandler>,_,a|(h.0)(a)]});
fn click(f: impl Fn(Rc<RibbonHandledEventArgs>) + 'static) -> MarkupDelegate {
    ValueTypes::register_reference::<ClickHandler>();
    MarkupDelegate::for_method(
        Some(Rc::new(ClickHandler(Box::new(f)))),
        ClickHandler::MARKUP,
        &ClickHandler::MARKUP.methods[0],
    )
}
fn events(r: &Ribbon) -> Rc<RefCell<Vec<String>>> {
    let trace = Rc::new(RefCell::new(vec![]));
    let out = trace.clone();
    r.add_backstage_opened(Some(handler(move || {
        out.borrow_mut().push("opened".into())
    })));
    let out = trace.clone();
    r.add_backstage_closed(Some(handler(move || {
        out.borrow_mut().push("closed".into())
    })));
    trace
}
struct Themes(Vec<Ref<ResourceDictionary>>);
impl Themes {
    fn new(r: &Ribbon) -> Self {
        let mut dictionaries = vec![];
        for file in ["Ribbon", "Backstage"] {
            let value = crate::compiled_shared::try_load(
                None,
                &format!("ferres://FerroRibbon/Themes/{file}.xaml"),
            )
            .unwrap()
            .unwrap();
            let dictionary = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<ResourceDictionary>()
                .unwrap();
            Application::current()
                .unwrap()
                .resources()
                .add_merged_dictionary(dictionary.clone());
            if file == "Ribbon" {
                let value = dictionary
                    .try_get_resource(&<Ribbon as StaticType>::TYPE.into(), None)
                    .flatten()
                    .unwrap();
                r.set_theme(
                    ValueTypes::as_object(&*value)
                        .unwrap()
                        .cast::<ControlTheme>(),
                );
            }
            dictionaries.push(dictionary);
        }
        Self(dictionaries)
    }
}
impl Drop for Themes {
    fn drop(&mut self) {
        for d in self.0.drain(..) {
            Application::current()
                .unwrap()
                .resources()
                .remove_merged_dictionary(&Rc::from(d));
        }
    }
}
fn resized(w: &Window, width: f64, height: f64) {
    let top = w.to_ref().upcast::<TopLevel>();
    top.handle_resized(Size::new(width, height), WindowResizeReason::User);
    w.measure(Size::new(width, height));
    w.arrange(Rect::new(0., 0., width, height));
    Dispatcher::ui_thread().run_jobs(None);
}
fn mount(r: &Ribbon) -> Ref<Window> {
    let w = Window::new();
    w.set_width(900.5);
    w.set_height(600.25);
    w.set_content(Some(Control::boxed(r.to_ref())));
    w.show();
    resized(&w, 900.5, 600.25);
    w
}
fn unmount(w: &Window) {
    w.close();
    w.set_content(None);
    Dispatcher::ui_thread().run_jobs(None);
}
fn request_close(b: &RibbonBackstage) {
    let i = RibbonBackstageItem::new();
    b.invoke(Some(i));
}
#[test]
fn original_root_backstage_absent_and_deferred_repeated_open_events_and_ownership() {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        let trace = events(&r);
        r.set_is_backstage_open(true);
        check(
            "absent",
            json!({"name":"absent","open":r.is_backstage_open(),"popup":r.backstage_popup().is_some(),"events":*trace.borrow()}),
        );
        let b = RibbonBackstage::new();
        let r = Ribbon::new();
        r.set_backstage(Some(b.clone()));
        let trace = events(&r);
        r.set_is_backstage_open(true);
        let before = json!({"open":r.is_backstage_open(),"popup":r.backstage_popup().is_some()});
        let _themes = Themes::new(&r);
        let w = mount(&r);
        r.open_backstage_core();
        let popup = r.backstage_popup().unwrap();
        let loaded = json!({"open":r.is_backstage_open(),"popup":popup.is_open(),"same":popup.child().is_some_and(|c|c.ptr_eq(&b)),"Width":b.width(),"Height":b.height(),"owner":RibbonItemHelper::get_owner(&b).is_some_and(|o|o.owner_ribbon()==Some(r.clone())),"ribbon":b.ribbon()==Some(r.clone())});
        assert!(!popup.is_light_dismiss_enabled());
        r.set_is_backstage_open(false);
        check(
            "deferred",
            json!({"name":"deferred","before":before,"loaded":loaded,"closed":!popup.is_open(),"childNull":popup.child().is_none(),"events":*trace.borrow()}),
        );
        unmount(&w);
    });
}
#[test]
fn original_root_backstage_replacement_removes_close_handler_and_preserves_old_owner() {
    crate::group_tests::run(|| {
        let old = RibbonBackstage::new();
        let next = RibbonBackstage::new();
        let r = Ribbon::new();
        r.set_backstage(Some(old.clone()));
        let _themes = Themes::new(&r);
        let w = mount(&r);
        let trace = events(&r);
        r.set_is_backstage_open(true);
        r.set_backstage(Some(next.clone()));
        request_close(&old);
        r.set_is_backstage_open(true);
        request_close(&old);
        let old_ignored = r.is_backstage_open();
        request_close(&next);
        check(
            "replacement",
            json!({"name":"replacement","open":r.is_backstage_open(),"oldIgnored":old_ignored,"oldRibbonNull":old.ribbon().is_none(),"newRibbon":next.ribbon()==Some(r.clone()),"oldOwnerKept":RibbonItemHelper::get_owner(&old).is_some_and(|o|o.owner_ribbon()==Some(r.clone())),"newOwner":RibbonItemHelper::get_owner(&next).is_some_and(|o|o.owner_ribbon()==Some(r.clone())),"events":*trace.borrow()}),
        );
        unmount(&w);
    });
}
#[test]
fn original_root_backstage_local_context_theme_window_size_subscription_and_focus_restore() {
    crate::group_tests::run(|| {
        let context: BoxedValue = Rc::new("initial".to_owned());
        let b = RibbonBackstage::new();
        let r = Ribbon::new();
        r.set_backstage(Some(b.clone()));
        r.set_data_context(Some(context.clone()));
        r.set_value(
            ThemeVariant::requested_theme_variant_property(),
            Some(ThemeVariant::dark()),
        );
        let _themes = Themes::new(&r);
        let w = mount(&r);
        r.set_is_backstage_open(true);
        let first = b.data_context().is_some_and(|c| Rc::ptr_eq(&c, &context));
        // Drive the native observable viewport directly: Headless physical-window
        // resizing rounds to device pixels, whereas XamlRoot.Size is in DIPs.
        w.set_client_size(Size::new(777.25, 444.5));
        let sized = [b.width(), b.height()];
        r.set_is_backstage_open(false);
        let focused = TopLevel::get_top_level(Some(&r))
            .unwrap()
            .focus_manager()
            .get_focused_element()
            .is_some_and(|f| f.ptr_eq(&r.application_button().unwrap()));
        r.set_data_context(Some(Rc::new("changed".to_owned())));
        w.set_client_size(Size::new(600., 300.));
        let closed = [b.width(), b.height()];
        r.set_is_backstage_open(true);
        check(
            "context_size",
            json!({"name":"context_size","first":first,"sticky":b.data_context().is_some_and(|c|Rc::ptr_eq(&c,&context)),"theme":if b.actual_theme_variant()==Some(ThemeVariant::dark()){"Dark"}else{"Other"},"sized":sized,"closed":closed,"reopen":[b.width(),b.height()],"focused":focused}),
        );
        unmount(&w);
        let b = RibbonBackstage::new();
        b.set_data_context(None);
        let r = Ribbon::new();
        r.set_backstage(Some(b.clone()));
        r.set_data_context(Some(context));
        let _themes = Themes::new(&r);
        let w = mount(&r);
        r.set_is_backstage_open(true);
        check(
            "local_null",
            json!({"name":"local_null","kept":b.data_context().is_none()}),
        );
        unmount(&w);
    });
}
#[test]
fn original_root_theme_refresh_updates_backstage_before_later_observers() {
    crate::group_tests::run(|| {
        let ribbon = Ribbon::new();
        let backstage = RibbonBackstage::new();
        let property = ThemeVariant::requested_theme_variant_property();
        let requested_theme = move |backstage: &RibbonBackstage| match backstage.get_value(property)
        {
            Some(theme) if theme == ThemeVariant::light() => "Light",
            Some(theme) if theme == ThemeVariant::dark() => "Dark",
            _ => "Default",
        };
        ribbon.set_value(property, Some(ThemeVariant::light()));
        let observed = Rc::new(RefCell::new(Vec::new()));
        let _observer = ribbon.actual_theme_variant_changed({
            let backstage = backstage.clone();
            let observed = observed.clone();
            move || observed.borrow_mut().push(requested_theme(&backstage))
        });
        ribbon.set_backstage(Some(backstage.clone()));
        let events = events(&ribbon);
        let _themes = Themes::new(&ribbon);
        let window = mount(&ribbon);
        ribbon.set_is_backstage_open(true);
        let popup = ribbon.backstage_popup().unwrap();
        let before = requested_theme(&backstage);
        observed.borrow_mut().clear();

        ribbon.set_value(property, Some(ThemeVariant::dark()));
        let after_open_change = requested_theme(&backstage);
        let same = popup.is_open() && popup.child().is_some_and(|child| child.ptr_eq(&backstage));
        assert_eq!(backstage.actual_theme_variant(), Some(ThemeVariant::dark()));
        Dispatcher::ui_thread().run_jobs(None);
        assert!(backstage.is_loaded());
        ribbon.set_value(property, Some(ThemeVariant::light()));
        let after_loaded_change = requested_theme(&backstage);
        ribbon.set_is_backstage_open(false);
        Dispatcher::ui_thread().run_jobs(None);
        ribbon.set_value(property, Some(ThemeVariant::dark()));
        let closed = requested_theme(&backstage);
        ribbon.set_is_backstage_open(true);
        let reopened = requested_theme(&backstage);

        check(
            "live_theme_change",
            json!({
                "name": "live_theme_change",
                "before": before,
                "observed": *observed.borrow(),
                "afterOpenChange": after_open_change,
                "same": same,
                "afterLoadedChange": after_loaded_change,
                "closed": closed,
                "reopened": reopened,
                "events": *events.borrow()
            }),
        );
        unmount(&window);
    });
}
#[test]
fn original_root_backstage_opened_reentrant_close_clears_same_popup_child() {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        r.set_backstage(Some(RibbonBackstage::new()));
        let _themes = Themes::new(&r);
        let w = mount(&r);
        let trace = events(&r);
        let weak = r.downgrade();
        r.add_backstage_opened(Some(handler(move || {
            weak.upgrade().unwrap().set_is_backstage_open(false)
        })));
        r.set_is_backstage_open(true);
        let p = r.backstage_popup().unwrap();
        check(
            "reentrant_open",
            json!({"name":"reentrant_open","open":r.is_backstage_open(),"popup":p.is_open(),"childNull":p.child().is_none(),"events":*trace.borrow()}),
        );
        unmount(&w);
    });
}
#[test]
fn original_application_event_order_cancellation_removal_and_final_shared_handled_state() {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        r.set_backstage(Some(RibbonBackstage::new()));
        let trace = Rc::new(RefCell::new(Vec::new()));
        let t = trace.clone();
        let first = click(move |a| {
            t.borrow_mut().push(format!(
                "first:{}",
                if a.handled() { "True" } else { "False" }
            ));
            a.set_handled(true);
        });
        let t = trace.clone();
        let second = click(move |a| {
            t.borrow_mut().push(format!(
                "second:{}",
                if a.handled() { "True" } else { "False" }
            ));
            a.set_handled(false);
        });
        r.add_application_button_click(Some(first.clone()));
        r.invoke_application_button();
        let canceled = !r.is_backstage_open();
        r.add_application_button_click(Some(second.clone()));
        r.invoke_application_button();
        let restored = r.is_backstage_open();
        r.remove_application_button_click(Some(first));
        r.remove_application_button_click(Some(second));
        check(
            "application_cancel",
            json!({"name":"application_cancel","canceled":canceled,"restored":restored,"calls":*trace.borrow()}),
        );
        r.set_is_backstage_open(false);
        r.invoke_application_button();
        assert!(r.is_backstage_open());
        assert_eq!(
            trace.borrow().len(),
            3,
            "both removed handlers remain detached"
        );
    });
}
#[test]
fn native_application_flyout_explicit_anchor_fallback_backstage_precedence_and_missing_anchor() {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        let _themes = Themes::new(&r);
        let w = mount(&r);
        let menu = Flyout::new();
        menu.set_content(Some(Control::boxed(TextBlock::new())));
        r.set_application_menu(Some(menu.clone().upcast()));
        let anchor = Button::new();
        let host = Panel::new();
        w.set_content(None);
        host.children().add(r.clone().upcast::<Control>());
        host.children().add(anchor.clone().upcast::<Control>());
        w.set_content(Some(Control::boxed(host)));
        resized(&w, 900.5, 600.25);
        let shown = Rc::new(Cell::new(0));
        let out = shown.clone();
        menu.opened(move || out.set(out.get() + 1)).detach();
        r.invoke_application_button_at(Some(anchor.clone().upcast()));
        let explicit_anchor = menu.target() == Some(anchor.upcast());
        assert!(menu.flyout_presenter_theme().is_some());
        menu.hide();
        r.invoke_application_button();
        let fallback = menu.target() == r.application_button().map(|b| b.upcast());
        menu.hide();
        r.set_backstage(Some(RibbonBackstage::new()));
        r.invoke_application_button();
        check(
            "application_menu",
            json!({"name":"application_menu","explicitAnchor":explicit_anchor,"fallback":fallback,"Shown":shown.get(),"backstage":r.is_backstage_open()}),
        );
        unmount(&w);
        let r = Ribbon::new();
        let menu = Flyout::new();
        r.set_application_menu(Some(menu.clone().upcast()));
        let shown = Rc::new(Cell::new(0));
        let out = shown.clone();
        menu.opened(move || out.set(out.get() + 1)).detach();
        r.invoke_application_button();
        check("no_anchor", json!({"name":"no_anchor","Shown":shown.get()}));
        assert!(!menu.is_open());
    });
}
#[test]
fn native_backstage_window_origin_nested_ribbon_escape_suspend_unload_and_control_identity() {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        let b = RibbonBackstage::new();
        let page = RibbonBackstageItem::new();
        let content = TextBlock::new();
        content.set_text(Some("same page"));
        page.set_content(Some(Control::boxed(content.clone())));
        b.items().add(Some(page));
        r.set_backstage(Some(b.clone()));
        let _themes = Themes::new(&r);
        let nested = Border::new();
        nested.set_padding(Thickness::new(70., 40., 0., 0.));
        nested.set_child(Some(r.clone().upcast()));
        let w = Window::new();
        w.set_content(Some(Control::boxed(nested)));
        w.show();
        resized(&w, 900.5, 600.25);
        r.application_button().unwrap().on_click();
        Dispatcher::ui_thread().run_jobs(None);
        let popup = r.backstage_popup().unwrap();
        assert!(popup.is_open());
        assert!(popup.child().unwrap().ptr_eq(&b));
        assert_eq!(popup.horizontal_offset(), 0.);
        assert_eq!(popup.vertical_offset(), 0.);
        assert_eq!(popup.placement_target(), Some(w.clone().upcast()));
        assert!(content.is_loaded());
        let root = TopLevel::get_top_level(Some(&b)).unwrap();
        assert!(root.focus_manager().get_focused_element().is_some_and(|f|f.ptr_eq(&b.selected_item().unwrap().navigation_button().unwrap())), "source open focus moves to selected navigation item");
        assert_eq!(
            b.translate_point(Point::default(), &root).unwrap(),
            Point::default()
        );
        let mut key = KeyEventArgs::new();
        key.key = Key::Escape;
        key.set_routed_event(Some(InputElement::key_down_event()));
        b.raise_event(&key);
        assert!(key.handled());
        assert!(!r.is_backstage_open());
        r.invoke_application_button();
        Dispatcher::ui_thread().run_jobs(None);
        assert!(popup.is_open());
        assert!(content.is_loaded());
        r.suspend_popups();
        assert!(!popup.is_open());
        r.set_is_backstage_open(true);
        w.set_content(None);
        Dispatcher::ui_thread().run_jobs(None);
        assert!(!r.is_backstage_open());
        assert!(popup.child().is_none());
        unmount(&w);
    });
}
#[test]
fn native_backstage_keytips_file_nested_item_backtracking_and_escape() {
    crate::group_tests::run(|| {
        let r = Ribbon::new();
        let tab = RibbonTab::new();
        tab.set_header(Some("Home".into()));
        tab.set_key_tip(Some("H".into()));
        r.tabs().add(Some(tab));
        let b = RibbonBackstage::new();
        let page = RibbonBackstageItem::new();
        page.set_header(Some("Info".into()));
        page.set_key_tip(Some("I".into()));
        page.set_content(Some(Control::boxed(TextBlock::new())));
        let save = RibbonBackstageItem::new();
        save.set_header(Some("Save".into()));
        save.set_key_tip(Some("S".into()));
        b.items().add(Some(page.clone()));
        b.items().add(Some(save.clone()));
        r.set_backstage(Some(b.clone()));
        let _themes = Themes::new(&r);
        let w = mount(&r);
        r.show_key_tips();
        assert!(r.process_key_tip_input(u16::from(b'F')));
        Dispatcher::ui_thread().run_jobs(None);
        assert!(r.is_backstage_open());
        assert!(r.is_key_tip_mode());
        assert!(
            r.current_key_tips()
                .iter()
                .any(|t| t.tip.unwrap().to_utf8().unwrap() == "I")
        );
        assert!(r.process_key_tip_input(u16::from(b'I')));
        assert_eq!(b.selected_item(), Some(page));
        assert!(!r.is_key_tip_mode());
        assert!(r.is_backstage_open());
        r.show_key_tips();
        assert!(r.process_key_tip_input(u16::from(b'S')));
        assert!(!r.is_backstage_open());
        assert!(!r.is_key_tip_mode());
        unmount(&w);
    });
}
#[test]
fn native_backstage_explicit_local_context_and_style_context_are_distinct() {
    crate::group_tests::run(|| {
        let b = RibbonBackstage::new();
        b.set_value_with_priority(
            StyledElement::data_context_property(),
            Some(Rc::new("style".to_owned()) as BoxedValue),
            BindingPriority::Style,
        );
        let r = Ribbon::new();
        let vm: BoxedValue = Rc::new("owner".to_owned());
        r.set_data_context(Some(vm.clone()));
        r.set_backstage(Some(b.clone()));
        let _themes = Themes::new(&r);
        let w = mount(&r);
        r.invoke_application_button();
        assert!(Rc::ptr_eq(&b.data_context().unwrap(), &vm));
        unmount(&w);
    });
}
