//! Native host/control tests against unchanged Invoke-body reference cases.
use super::*;
use ferroui_base::{input::ICommand, reactive::Disposable, threading::Dispatcher};
use ferroui_controls::{PlacementMode, Window};
use serde_json::{Value, json};
use std::cell::Cell;
type Can = Box<dyn Fn(Option<&BoxedValue>) -> bool>;
type Execute = Box<dyn Fn(Option<&BoxedValue>)>;
struct Command {
    can: Can,
    execute: Execute,
}
impl ICommand for Command {
    fn can_execute(&self, p: Option<&BoxedValue>) -> bool {
        (self.can)(p)
    }
    fn execute(&self, p: Option<&BoxedValue>) {
        (self.execute)(p)
    }
    fn can_execute_changed(&self, _: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}
fn command(
    can: impl Fn(Option<&BoxedValue>) -> bool + 'static,
    execute: impl Fn(Option<&BoxedValue>) + 'static,
) -> Rc<dyn ICommand> {
    Rc::new(Command {
        can: Box::new(can),
        execute: Box::new(execute),
    })
}
fn parameter(value: Option<&BoxedValue>) -> String {
    value
        .and_then(|v| v.downcast_ref::<String>())
        .cloned()
        .unwrap_or_default()
}
fn set_parameter(item: &Item, value: &str) {
    item.set_command_parameter(Some(Rc::new(value.to_owned())));
}
fn mount() -> (Ref<RibbonApplicationMenu>, Ref<Popup>, Ref<Window>) {
    let menu = RibbonApplicationMenu::new();
    let window = Window::new();
    let target = Border::new();
    window.set_content(Some(Control::boxed(target.clone())));
    window.show();
    let popup = Popup::new();
    popup.set_placement_target(target.clone().upcast::<Control>());
    popup.set_placement(PlacementMode::Bottom);
    popup.set_parent(target);
    popup.set_child(menu.clone().upcast::<Control>());
    popup.set_is_open(true);
    window.update_layout();
    Dispatcher::ui_thread().run_jobs(None);
    assert!(popup.host().is_some());
    assert!(popup.is_inside_popup(&menu));
    assert!(ferroui_controls::TopLevel::get_top_level(Some(&menu)).is_some());
    (menu, popup, window)
}
#[test]
fn application_menu_parent_reference_does_not_form_an_owned_flyout_cycle() {
    crate::group_tests::run(|| {
        let ribbon = Ribbon::new();
        let menu = RibbonApplicationMenu::new();
        let flyout = ferroui_controls::Flyout::new();
        flyout.set_content(Some(Control::boxed(menu.clone())));
        ribbon.set_application_menu(Some(flyout.clone().upcast()));
        menu.set_ribbon(Some(ribbon.clone()));
        assert!(menu.ribbon().unwrap().ptr_eq(&ribbon));
        let weak_ribbon = ribbon.downgrade();
        let weak_menu = menu.downgrade();
        let weak_flyout = flyout.downgrade();
        drop(ribbon);
        assert!(
            weak_ribbon.upgrade().is_none(),
            "An application menu is a child, not a strong owner of its ribbon"
        );
        assert!(menu.ribbon().is_none());
        drop(menu);
        drop(flyout);
        assert!(weak_menu.upgrade().is_none());
        assert!(weak_flyout.upgrade().is_none());
    });
}
#[test]
fn application_menu_native_popup_invocation_matches_unchanged_source_order_and_reentry() {
    crate::group_tests::run(|| {
        let mut cases = Vec::new();
        {
            let (menu, popup, window) = mount();
            let item = Item::new();
            item.set_id(Some("replace".into()));
            set_parameter(&item, "initial");
            let trace = Rc::new(RefCell::new(Vec::<String>::new()));
            let ignored = Rc::new(Cell::new(0));
            let c = ignored.clone();
            let x = ignored.clone();
            let last = command(
                move |_| {
                    c.set(c.get() + 1);
                    true
                },
                move |_| x.set(x.get() + 1),
            );
            item.set_command(Some(last.clone()));
            let t = trace.clone();
            let weak = item.downgrade();
            let ignored_command = last.clone();
            let close = popup.closed(move || {
                t.borrow_mut().push("close".into());
                let item = weak.upgrade().unwrap();
                set_parameter(&item, "closed");
                item.set_command(Some(ignored_command.clone()));
            });
            let t = trace.clone();
            let click = item.click(move |sender, e| {
                t.borrow_mut()
                    .push(format!("click:{}", parameter(e.parameter().as_ref())));
                set_parameter(sender, "clicked");
                sender.set_is_enabled(false);
                let weak = sender.to_ref().downgrade();
                let weak2 = weak.clone();
                let can_trace = t.clone();
                let execute_trace = t.clone();
                let last = last.clone();
                sender.set_command(Some(command(
                    move |p| {
                        can_trace.borrow_mut().push(format!("can:{}", parameter(p)));
                        let item = weak.upgrade().unwrap();
                        set_parameter(&item, "can");
                        item.set_command(Some(last.clone()));
                        true
                    },
                    move |p| {
                        execute_trace
                            .borrow_mut()
                            .push(format!("execute:{}", parameter(p)));
                        set_parameter(&weak2.upgrade().unwrap(), "executed");
                    },
                )));
            });
            let t = trace.clone();
            let invoked = menu.item_invoked(move |_, e| {
                t.borrow_mut()
                    .push(format!("invoked:{}", parameter(e.parameter().as_ref())))
            });
            menu.invoke(item.clone());
            assert!(!popup.is_open());
            cases.push(json!({"name":"replace-reenter","trace":*trace.borrow(),"ignored":ignored.get(),"IsEnabled":item.is_enabled()}));
            close.dispose();
            click.dispose();
            invoked.dispose();
            window.close();
        }
        {
            let (menu, popup, window) = mount();
            let item = Item::new();
            set_parameter(&item, "p");
            let trace = Rc::new(RefCell::new(Vec::<String>::new()));
            let t = trace.clone();
            let close = popup.closed(move || t.borrow_mut().push("close".into()));
            let t = trace.clone();
            let click = item.click(move |_, _| t.borrow_mut().push("click".into()));
            let t = trace.clone();
            let x = trace.clone();
            item.set_command(Some(command(
                move |_| {
                    t.borrow_mut().push("can".into());
                    false
                },
                move |_| x.borrow_mut().push("execute".into()),
            )));
            let t = trace.clone();
            let invoked = menu.item_invoked(move |_, _| t.borrow_mut().push("invoked".into()));
            menu.invoke(item);
            cases.push(json!({"name":"denied","trace":*trace.borrow()}));
            close.dispose();
            click.dispose();
            invoked.dispose();
            window.close();
        }
        {
            let (menu, popup, window) = mount();
            let item = Item::new();
            item.set_is_enabled(false);
            item.items().add(Some(Item::new()));
            let calls = Rc::new(Cell::new(0));
            let c = calls.clone();
            let close = popup.closed(move || c.set(c.get() + 1));
            let c = calls.clone();
            let click = item.click(move |_, _| c.set(c.get() + 1));
            let c = calls.clone();
            let invoked = menu.item_invoked(move |_, _| c.set(c.get() + 1));
            menu.invoke(item.clone());
            let disabled_ignored = menu.shown_item().is_none();
            item.set_is_enabled(true);
            menu.invoke(item.clone());
            cases.push(json!({"name":"parent","disabledIgnored":disabled_ignored,"shown":menu.shown_item().is_some_and(|shown|shown.ptr_eq(&item)),"calls":calls.get(),"HostOpen":popup.is_open()}));
            close.dispose();
            click.dispose();
            invoked.dispose();
            popup.set_is_open(false);
            window.close();
        }
        {
            let (menu, popup, window) = mount();
            let item = Recent::new();
            let trace = Rc::new(RefCell::new(Vec::<String>::new()));
            let identity = Rc::new(Cell::new(true));
            let ignored = Rc::new(Cell::new(0));
            let t = trace.clone();
            let close = popup.closed(move || t.borrow_mut().push("close".into()));
            let t = trace.clone();
            let same = identity.clone();
            let weak = item.downgrade();
            let ignored2 = ignored.clone();
            let click = item.click(move |sender| {
                same.set(same.get() && weak.upgrade().is_some_and(|v| v.ptr_eq(&sender.to_ref())));
                t.borrow_mut().push("click".into());
                let w = sender.to_ref().downgrade();
                let w2 = w.clone();
                let c = ignored2.clone();
                let x = ignored2.clone();
                let last = command(
                    move |_| {
                        c.set(c.get() + 1);
                        true
                    },
                    move |_| x.set(x.get() + 1),
                );
                let can_trace = t.clone();
                let execute_trace = t.clone();
                let same = same.clone();
                let same2 = same.clone();
                sender.set_command(Some(command(
                    move |p| {
                        let arg = p
                            .and_then(|p| ferroui_base::data::core::ValueTypes::as_object(&**p))
                            .and_then(|v| v.cast::<Recent>());
                        let item = w.upgrade().unwrap();
                        same.set(same.get() && arg.is_some_and(|v| v.ptr_eq(&item)));
                        can_trace.borrow_mut().push("can".into());
                        item.set_command(Some(last.clone()));
                        true
                    },
                    move |p| {
                        let arg = p
                            .and_then(|p| ferroui_base::data::core::ValueTypes::as_object(&**p))
                            .and_then(|v| v.cast::<Recent>());
                        same2.set(
                            same2.get() && arg.is_some_and(|v| v.ptr_eq(&w2.upgrade().unwrap())),
                        );
                        execute_trace.borrow_mut().push("execute".into());
                    },
                )));
            });
            let t = trace.clone();
            let same = identity.clone();
            let weak = item.downgrade();
            let invoked = menu.recent_item_invoked(move |_, item| {
                same.set(same.get() && weak.upgrade().is_some_and(|v| v.ptr_eq(item)));
                t.borrow_mut().push("invoked".into());
            });
            menu.invoke_recent(item);
            cases.push(json!({"name":"recent","trace":*trace.borrow(),"identity":identity.get(),"ignored":ignored.get()}));
            close.dispose();
            click.dispose();
            invoked.dispose();
            window.close();
        }
        {
            let (menu, popup, window) = mount();
            let item = Item::new();
            set_parameter(&item, "initial");
            let trace = Rc::new(RefCell::new(Vec::<String>::new()));
            let t = trace.clone();
            let close = popup.closed(move || t.borrow_mut().push("close".into()));
            let t = trace.clone();
            let nested = Cell::new(false);
            let weak = menu.downgrade();
            let click = item.click(move |sender, e| {
                t.borrow_mut()
                    .push(format!("click:{}", parameter(e.parameter().as_ref())));
                if !nested.replace(true) {
                    set_parameter(sender, "inner");
                    weak.upgrade().unwrap().invoke(sender.to_ref());
                }
            });
            let t = trace.clone();
            let x = trace.clone();
            let weak = item.downgrade();
            item.set_command(Some(command(
                move |p| {
                    t.borrow_mut().push(format!("can:{}", parameter(p)));
                    true
                },
                move |p| {
                    x.borrow_mut().push(format!("execute:{}", parameter(p)));
                    set_parameter(&weak.upgrade().unwrap(), "after");
                },
            )));
            let t = trace.clone();
            let invoked = menu.item_invoked(move |_, e| {
                t.borrow_mut()
                    .push(format!("invoked:{}", parameter(e.parameter().as_ref())))
            });
            menu.invoke(item);
            cases.push(json!({"name":"nested","trace":*trace.borrow()}));
            close.dispose();
            click.dispose();
            invoked.dispose();
            window.close();
        }
        let expected: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/application-menu-invoke.json"
        ))
        .unwrap();
        assert_eq!(json!(cases), expected);
    });
}

fn menu_item(id: Option<&str>, label: &str) -> Ref<Item> {
    let item = Item::new();
    item.set_id(id.map(Into::into));
    item.set_label(Some(label.into()));
    item
}
fn search_dump(menu: &RibbonApplicationMenu, query: &str) -> Value {
    json!(menu.search_menu(query).iter().map(|result|{let entry=result.entry().unwrap();let target=crate::metadata::model_objects::to_native(entry.target()).unwrap().and_then(|v|ferroui_base::data::core::ValueTypes::as_object(&*v)).and_then(|v|v.cast::<Item>());json!({"Id":localized(entry.id()),"Label":localized(entry.label()),"Path":localized(entry.path()),"Description":localized(entry.description()),"IsEnabled":entry.is_enabled(),"ScoreBits":format!("{:016x}",result.score().to_bits()),"TargetId":target.and_then(|item|item.id())})}).collect::<Vec<_>>())
}
struct EnglishStrings(std::sync::Arc<RibbonStrings>);
impl EnglishStrings {
    fn new() -> Self {
        let old = RibbonStrings::current();
        let english = RibbonStrings::for_culture(Some(
            ferroribbon_core::localization::RibbonCulture::new("en-US".into(), "en".into()),
        ))
        .unwrap();
        RibbonStrings::set_current(Some(english)).unwrap();
        Self(old)
    }
}
impl Drop for EnglishStrings {
    fn drop(&mut self) {
        RibbonStrings::set_current(Some(self.0.clone())).unwrap();
    }
}
#[test]
fn application_menu_search_matches_unchanged_tree_paths_limits_ties_and_numeric_scores() {
    if crate::test_process::isolated_case(concat!(
        module_path!(),
        "::application_menu_search_matches_unchanged_tree_paths_limits_ties_and_numeric_scores"
    )) {
        return;
    }
    crate::group_tests::run(|| {
        let _strings = EnglishStrings::new();
        let mut cases = Vec::new();
        {
            let menu = RibbonApplicationMenu::new();
            let drawing = menu_item(Some("dup"), "Open Drawing");
            drawing.set_description(Some("Write drawing".into()));
            menu.items().add(Some(drawing));
            menu.items()
                .add(Some(menu_item(Some("dup"), "Open Template")));
            let denied = menu_item(Some("denied"), "Open Denied");
            denied.set_is_enabled(false);
            menu.items().add(Some(denied));
            menu.items().add(Some(menu_item(None, "")));
            menu.items().add(Some(menu_item(None, "   ")));
            let save = menu_item(None, "Save");
            save.set_is_enabled(false);
            let template = menu_item(None, "Template");
            template
                .items()
                .add(Some(menu_item(None, "Export Document")));
            save.items().add(Some(template));
            menu.items().add(Some(save));
            for query in ["open", "export", "write", ""] {
                cases.push(json!({"name":"tree","query":query,"results":search_dump(&menu,query)}));
            }
        }
        {
            let menu = RibbonApplicationMenu::new();
            let ribbon = Ribbon::new();
            menu.set_ribbon(Some(ribbon.clone()));
            for i in 0..20 {
                menu.items()
                    .add(Some(menu_item(Some(&format!("m{i:02}")), "Alpha")));
                ribbon
                    .additional_search_entries()
                    .add(Some(RibbonSearchEntry::new(
                        Some(format!("r{i:02}").into()),
                        Some("Alpha".into()),
                    )));
            }
            cases.push(
                json!({"name":"limits","query":"alpha","results":search_dump(&menu,"alpha")}),
            );
        }
        {
            let menu = RibbonApplicationMenu::new();
            let item = menu_item(Some("resume"), "Résumé");
            item.set_description(Some("café".into()));
            menu.items().add(Some(item));
            for query in ["resume", "cafe"] {
                cases.push(
                    json!({"name":"unicode","query":query,"results":search_dump(&menu,query)}),
                );
            }
        }
        let expected: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/application-menu-search.json"
        ))
        .unwrap();
        assert_eq!(json!(cases), expected);
    });
}
fn key(control: &Control, key: Key) -> bool {
    let mut e = KeyEventArgs::new();
    e.key = key;
    e.set_routed_event(Some(ferroui_base::input::InputElement::key_down_event()));
    control.raise_event(&e);
    e.handled()
}
fn recent_title(row: &Control) -> String {
    let row = row.to_ref().cast::<Grid>().unwrap();
    let button = row.children().get(0).cast::<Button>().unwrap();
    let grid = ferroui_base::data::core::ValueTypes::as_object(&*button.content().unwrap())
        .unwrap()
        .cast::<Grid>()
        .unwrap();
    let text = grid.children().get(1).cast::<StackPanel>().unwrap();
    text.children()
        .get(0)
        .cast::<TextBlock>()
        .unwrap()
        .text()
        .unwrap()
}
#[test]
fn application_menu_native_keyboard_search_pins_and_removed_item_callbacks() {
    crate::group_tests::run(|| {
        let _strings = EnglishStrings::new();
        let (menu, popup, window) = mount();
        let parent = menu_item(Some("save"), "Save As");
        let leaf = menu_item(Some("drawing"), "Drawing");
        parent.items().add(Some(leaf.clone()));
        menu.items().add(Some(parent.clone()));
        let normal = Recent::new();
        normal.set_title(Some("Normal".into()));
        let pinned = Recent::new();
        pinned.set_title(Some("Pinned".into()));
        pinned.set_is_pinned(true);
        menu.recent_items().add(Some(normal));
        menu.recent_items().add(Some(pinned.clone()));
        use ferroui_base::input::navigation::{XYFocus, XYFocusNavigationModes};
        assert_eq!(
            XYFocus::get_navigation_modes(&menu.parts.commands),
            XYFocusNavigationModes::ENABLED
        );
        assert_eq!(
            XYFocus::get_navigation_modes(&menu.parts.pane),
            XYFocusNavigationModes::ENABLED
        );
        let parent_button = menu
            .parts
            .commands
            .children()
            .get(0)
            .cast::<Button>()
            .unwrap();
        assert_eq!(
            AutomationProperties::get_automation_id(&parent_button).as_deref(),
            Some("AppMenu_save")
        );
        assert_eq!(
            AutomationProperties::get_name(&parent_button).as_deref(),
            Some("Save As")
        );
        assert_eq!(recent_title(&menu.parts.pane.children().get(0)), "Pinned");
        let pin = menu
            .parts
            .pane
            .children()
            .get(0)
            .cast::<Grid>()
            .unwrap()
            .children()
            .get(1)
            .cast::<ToggleButton>()
            .unwrap();
        ferroui_controls::automation::peers::ButtonAutomationPeer::new(&pin)
            .invoke()
            .unwrap();
        assert!(!pinned.is_pinned());
        assert_eq!(recent_title(&menu.parts.pane.children().get(0)), "Normal");
        assert!(key(&parent_button, Key::Right));
        assert!(menu.shown_item().unwrap().ptr_eq(&parent));
        let sub = menu.parts.pane.children().get(0);
        assert!(
            ferroui_controls::TopLevel::get_top_level(Some(&menu))
                .unwrap()
                .focus_manager()
                .get_focused_element()
                .unwrap()
                .ptr_eq(&sub)
        );
        assert!(key(&sub, Key::Left));
        assert!(parent_button.is_focused());
        menu.parts.search.set_text(Some(" "));
        assert!(menu.shown_item().is_none());
        let subfocus = ferroui_base::input::FocusChangedEventArgs::new(
            ferroui_base::input::InputElement::got_focus_event(),
        );
        parent_button.raise_event(&subfocus);
        assert!(
            menu.shown_item().is_none(),
            "whitespace displays recents but still suppresses hover"
        );
        menu.parts.search.set_text(Some(""));
        menu.recent_items().remove(&Some(pinned.clone()));
        let before = menu.parts.pane.children().get(0);
        pinned.set_title(Some("Removed changed".into()));
        assert!(
            !before.ptr_eq(&menu.parts.pane.children().get(0)),
            "source keeps removed recent callbacks while the model is alive"
        );
        menu.set_is_search_visible(false);
        assert!(!menu.parts.search_row.is_visible());
        menu.set_is_search_visible(true);
        assert!(menu.parts.search_row.is_visible());
        let clicks = Rc::new(Cell::new(0));
        let c = clicks.clone();
        let subscription = leaf.click(move |_, _| c.set(c.get() + 1));
        menu.parts.search.set_text(Some("drawing"));
        assert_eq!(menu.parts.pane.children().count(), 1);
        assert!(key(&menu.parts.search, Key::Enter));
        assert_eq!(clicks.get(), 1);
        assert!(!popup.is_open());
        subscription.dispose();
        window.close();
    });
}
#[test]
fn application_menu_footer_reparents_same_control_and_closes_after_existing_click_handlers() {
    crate::group_tests::run(|| {
        let (menu, popup, window) = mount();
        let previous = StackPanel::new();
        let button = Button::new();
        previous.children().add(button.clone().upcast::<Control>());
        let trace = Rc::new(RefCell::new(Vec::new()));
        let t = trace.clone();
        button.click(move |_, _| t.borrow_mut().push("user"));
        let t = trace.clone();
        let closed = popup.closed(move || t.borrow_mut().push("close"));
        menu.footer_items()
            .add(Some(button.clone().upcast::<Control>()));
        assert!(previous.children().is_empty());
        assert!(menu.parts.footer.children().get(0).ptr_eq(&button));
        ferroui_controls::automation::peers::ButtonAutomationPeer::new(&button)
            .invoke()
            .unwrap();
        assert_eq!(*trace.borrow(), ["user", "close"]);
        assert!(!popup.is_open());
        assert!(menu.parts.footer.children().get(0).ptr_eq(&button));
        closed.dispose();
        window.close();
    });
}
