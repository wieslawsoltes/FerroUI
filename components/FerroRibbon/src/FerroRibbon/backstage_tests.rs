//! Original backstage state/invocation corpus and real native template/input cases.
use crate::controls::{Ribbon, RibbonBackstage, RibbonBackstageItem};
use ferroui_base::{
    BoxedValue, Rect, Ref, Size, StaticType,
    controls::ResourceDictionary,
    data::core::ValueTypes,
    input::ICommand,
    metadata::{MarkupDelegate, MarkupTyped},
    reactive::{Disposable, IDisposable},
    styling::ControlTheme,
};
use ferroui_controls::{Control, Panel, TextBlock, TopLevel, Window};
use serde_json::{Value, json};
use std::{cell::RefCell, rc::Rc};
type Trace = Rc<RefCell<Vec<String>>>;
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
type ItemCallback = Box<dyn Fn(Ref<RibbonBackstageItem>)>;
struct ItemHandler(ItemCallback);
impl PartialEq for ItemHandler {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::eq(self, o)
    }
}
ferroui_base::ferro_markup_type!(class ItemHandler{this:Rc<ItemHandler>,handles:[Rc<ItemHandler>],methods:[fn Invoke(Option<BoxedValue>,Ref<RibbonBackstageItem>)=>|h:&Rc<ItemHandler>,_,i|(h.0)(i)]});
fn on_item(b: &RibbonBackstage, f: impl Fn(Ref<RibbonBackstageItem>) + 'static) {
    ValueTypes::register_reference::<ItemHandler>();
    b.add_item_invoked(Some(MarkupDelegate::for_method(
        Some(Rc::new(ItemHandler(Box::new(f)))),
        ItemHandler::MARKUP,
        &ItemHandler::MARKUP.methods[0],
    )));
}
struct CloseHandler(Box<dyn Fn()>);
impl PartialEq for CloseHandler {
    fn eq(&self, o: &Self) -> bool {
        std::ptr::eq(self, o)
    }
}
ferroui_base::ferro_markup_type!(class CloseHandler{this:Rc<CloseHandler>,handles:[Rc<CloseHandler>],methods:[fn Invoke(Option<BoxedValue>,ferroui_base::utilities::EventArgs)=>|h:&Rc<CloseHandler>,_,_|(h.0)()]});
fn close_delegate(f: impl Fn() + 'static) -> MarkupDelegate {
    ValueTypes::register_reference::<CloseHandler>();
    MarkupDelegate::for_method(
        Some(Rc::new(CloseHandler(Box::new(f)))),
        CloseHandler::MARKUP,
        &CloseHandler::MARKUP.methods[0],
    )
}
fn log_close(b: &RibbonBackstage, trace: &Trace) {
    let t = trace.clone();
    b.add_close_requested(Some(close_delegate(move || {
        t.borrow_mut().push("close".into())
    })));
}
fn item(id: &str, page: bool) -> Ref<RibbonBackstageItem> {
    let i = RibbonBackstageItem::new();
    i.set_id(Some(id.into()));
    i.set_header(Some(id.into()));
    if page {
        i.set_content(Some(Rc::new("page".to_owned())));
    }
    i
}
fn parameter(p: Option<&BoxedValue>) -> String {
    p.and_then(|p| p.downcast_ref::<String>())
        .cloned()
        .unwrap_or_default()
}
fn selected(b: &RibbonBackstage) -> Option<String> {
    b.selected_item().and_then(|i| i.id())
}
#[test]
fn original_backstage_selection_and_reentrant_command_corpus() {
    crate::group_tests::run(|| {
        let mut cases = Vec::<Value>::new();
        {
            let b = RibbonBackstage::new();
            let a = item("a", true);
            a.set_is_enabled(false);
            let z = item("z", true);
            let h = item("h", true);
            h.set_is_visible(false);
            let mut trace = Vec::new();
            b.items().add(Some(a.clone()));
            b.items().add(Some(z.clone()));
            b.items().add(Some(h.clone()));
            trace.push(selected(&b));
            a.set_is_visible(false);
            trace.push(selected(&b));
            z.set_content(None);
            trace.push(selected(&b));
            h.set_is_visible(true);
            trace.push(selected(&b));
            b.set_selected_item(Some(a.clone()));
            b.ensure_selection();
            trace.push(selected(&b));
            b.items().remove(&Some(h));
            trace.push(selected(&b));
            a.set_is_visible(true);
            trace.push(selected(&b));
            a.set_header(Some("renamed".into()));
            let title = b.page_title();
            b.items().add(Some(a.clone()));
            b.items().remove(&Some(a.clone()));
            let same = b.selected_item() == Some(a.clone());
            b.items().clear();
            // Header mutation after removal must not repair a stale PageTitle.
            a.set_header(Some("removed".into()));
            cases.push(json!({"name":"selection","trace":trace,"title":title,"same":same,"empty":b.selected_item().is_none(),"ownerCleared":a.owner().is_none()}));
        }
        {
            let b = RibbonBackstage::new();
            let a = item("action", false);
            let trace: Trace = Rc::default();
            let t = trace.clone();
            on_item(&b, move |i| {
                t.borrow_mut().push(format!("item:{}", i.id().unwrap()))
            });
            log_close(&b, &trace);
            let t = trace.clone();
            let t2 = trace.clone();
            a.set_command(Some(Rc::new(Command {
                can: Box::new(move |p| {
                    t.borrow_mut().push(format!("can:{}", parameter(p)));
                    false
                }),
                execute: Box::new(move |p| {
                    t2.borrow_mut().push(format!("execute:{}", parameter(p)))
                }),
            })));
            a.set_command_parameter(Some(Rc::new("before".to_owned())));
            b.invoke(Some(a.clone()));
            a.set_closes_backstage(false);
            let t = trace.clone();
            let t2 = trace.clone();
            let weak = a.downgrade();
            a.set_command(Some(Rc::new(Command {
                can: Box::new(move |p| {
                    t.borrow_mut().push(format!("can:{}", parameter(p)));
                    weak.upgrade()
                        .unwrap()
                        .set_command_parameter(Some(Rc::new("after".to_owned())));
                    true
                }),
                execute: Box::new(move |p| {
                    t2.borrow_mut().push(format!("execute:{}", parameter(p)))
                }),
            })));
            let r = Ribbon::new();
            let t = trace.clone();
            r.add_item_invoked(Some(crate::root_command_tests::invocation(move |e| {
                t.borrow_mut().push(format!(
                    "root:{}:{}",
                    e.command_id.as_ref().unwrap().to_utf8().unwrap(),
                    parameter(e.parameter.as_ref())
                ))
            })));
            b.set_ribbon(Some(r.clone()));
            b.invoke(Some(a.clone()));
            a.set_is_enabled(false);
            b.invoke(Some(a.clone()));
            a.set_is_enabled(true);
            a.set_is_visible(false);
            b.invoke(Some(a));
            cases.push(json!({"name":"actions","trace":*trace.borrow()}));
        }
        {
            let b = RibbonBackstage::new();
            let i = item("page", true);
            let trace: Trace = Rc::default();
            log_close(&b, &trace);
            let t = trace.clone();
            on_item(&b, move |i| {
                t.borrow_mut().push("item".into());
                i.set_content(None);
            });
            let t = trace.clone();
            let t2 = trace.clone();
            i.set_command(Some(Rc::new(Command {
                can: Box::new(move |_| {
                    t.borrow_mut().push("can".into());
                    true
                }),
                execute: Box::new(move |_| t2.borrow_mut().push("execute".into())),
            })));
            b.invoke(Some(i));
            cases.push(json!({"name":"page_becomes_action","trace":*trace.borrow(),"selected":selected(&b)}));
        }
        {
            let b = RibbonBackstage::new();
            let i = item("outside", false);
            let trace: Trace = Rc::default();
            let t = trace.clone();
            on_item(&b, move |i| {
                t.borrow_mut().push("item".into());
                i.set_content(Some(Rc::new("page".to_owned())));
            });
            log_close(&b, &trace);
            i.set_command(Some(Rc::new(Command {
                can: Box::new(|_| panic!("page cannot query command")),
                execute: Box::new(|_| panic!("page cannot execute")),
            })));
            b.invoke(Some(i.clone()));
            let same = b.selected_item() == Some(i);
            let title = b.page_title();
            b.ensure_selection();
            cases.push(json!({"name":"action_becomes_external_page","trace":*trace.borrow(),"selected":same,"title":title,"ensured":b.selected_item().is_none()}));
        }
        {
            let b = RibbonBackstage::new();
            let i = item("replace", false);
            let trace: Trace = Rc::default();
            let t = trace.clone();
            let t2 = trace.clone();
            let weak = i.downgrade();
            i.set_command(Some(Rc::new(Command {
                can: Box::new(move |_| {
                    t.borrow_mut().push("old-can".into());
                    let item = weak.upgrade().unwrap();
                    item.set_command(Some(Rc::new(Command {
                        can: Box::new(|_| panic!("wrong")),
                        execute: Box::new(|_| panic!("wrong")),
                    })));
                    item.set_is_visible(false);
                    true
                }),
                execute: Box::new(move |_| t2.borrow_mut().push("old-execute".into())),
            })));
            let t = trace.clone();
            on_item(&b, move |_| t.borrow_mut().push("item".into()));
            log_close(&b, &trace);
            b.invoke(Some(i));
            cases.push(json!({"name":"captured_command","trace":*trace.borrow()}));
        }
        {
            let b = RibbonBackstage::new();
            let mut errors = Vec::new();
            let mut attempt = |action: &dyn Fn()| {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(action));
                errors.push(if result.is_err() {
                    "NullReferenceException"
                } else {
                    "none"
                });
            };
            attempt(&|| b.items().add(None));
            let added = b.items().count();
            attempt(&|| b.items().clear());
            let cleared = b.items().count();
            attempt(&|| b.items().add(Some(item("good", true))));
            cases.push(json!({"name":"null_items","errors":errors,"added":added,"cleared":cleared,"final":b.items().count(),"selected":selected(&b)}));
        }
        let expected: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/backstage.json")).unwrap();
        assert_eq!(json!(cases), expected["corpus"]["cases"]);
    });
}

fn mount(b: &RibbonBackstage) -> (Ref<Window>, Ref<ResourceDictionary>) {
    let d = crate::compiled_shared::try_load(None, "ferres://FerroRibbon/Themes/Backstage.xaml")
        .unwrap()
        .unwrap();
    let d = ValueTypes::as_object(&*d)
        .unwrap()
        .cast::<ResourceDictionary>()
        .unwrap();
    let theme = ValueTypes::as_object(
        &*d.try_get_resource(&<RibbonBackstage as StaticType>::TYPE.into(), None)
            .flatten()
            .unwrap(),
    )
    .unwrap()
    .cast::<ControlTheme>()
    .unwrap();
    // Programmatic navigation source consults Application.Resources, rather than the local template dictionary.
    ferroui_controls::Application::current()
        .unwrap()
        .resources()
        .add_merged_dictionary(d.clone());
    b.set_theme(theme);
    let w = Window::new();
    w.set_content(Some(Control::boxed(b.to_ref())));
    w.set_width(800.);
    w.set_height(500.);
    w.show();
    w.measure(Size::new(800., 500.));
    w.arrange(Rect::new(0., 0., 800., 500.));
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    (w, d)
}
fn part(b: &RibbonBackstage, name: &str) -> Ref<Control> {
    b.get_visual_descendants()
        .filter_map(|v| v.cast::<Control>())
        .find(|v| v.name().as_deref() == Some(name))
        .unwrap()
}
#[test]
fn original_backstage_navigation_refresh_identity_focus_and_template_parts() {
    crate::group_tests::run(|| {
        let b = RibbonBackstage::new();
        let a = item("a", true);
        a.set_is_enabled(false);
        let action = item("save", false);
        action.set_placement(ferroribbon_core::RibbonBackstagePlacement::Bottom);
        let z = item("z", true);
        b.items().add(Some(a.clone()));
        b.items().add(Some(action.clone()));
        b.items().add(Some(z.clone()));
        let (w, resources) = mount(&b);
        assert_eq!(b.page_title().as_deref(), Some("a"));
        let title = part(&b, "PART_PageTitle").cast::<TextBlock>().unwrap();
        assert_eq!(title.text().as_deref(), Some("a"));
        let title_peer =
            ferroui_controls::automation::peers::ControlAutomationPeer::create_peer_for_element(
                &title,
            );
        assert!(title_peer.is_control_element());
        assert!(title_peer.is_content_element());
        assert_eq!(title_peer.get_name(), "a");
        let button = a.navigation_button().unwrap();
        assert!(!button.is_enabled());
        assert!(button.pseudo_classes().contains(":ribbon-selected"));
        assert_eq!(
            ferroui_controls::automation::AutomationProperties::get_automation_id(&button)
                .as_deref(),
            Some("Backstage_a")
        );
        a.set_header(Some("Changed".into()));
        assert_eq!(b.page_title().as_deref(), Some("Changed"));
        assert_eq!(title.text().as_deref(), Some("Changed"));
        assert_eq!(title_peer.get_name(), "Changed");
        assert_eq!(a.navigation_button(), Some(button.clone()));
        a.set_id(Some("".into()));
        assert_eq!(
            ferroui_controls::automation::AutomationProperties::get_automation_id(&button)
                .as_deref(),
            Some("Backstage_")
        );
        a.set_id(None);
        assert!(
            ferroui_controls::automation::AutomationProperties::get_automation_id(&button)
                .is_none()
        );
        assert!(b.focus_selected_item());
        assert!(
            TopLevel::get_top_level(Some(&*b))
                .unwrap()
                .focus_manager()
                .get_focused_element()
                .unwrap()
                .ptr_eq(&z.navigation_button().unwrap())
        );
        let before = button.downgrade();
        drop(button);
        action.set_has_separator_before(true);
        assert!(
            before
                .upgrade()
                .is_none_or(|old| Some(old) != a.navigation_button())
        );
        assert_eq!(
            part(&b, "PART_TopItems")
                .cast::<Panel>()
                .unwrap()
                .children()
                .count(),
            2
        );
        assert_eq!(
            part(&b, "PART_BottomItems")
                .cast::<Panel>()
                .unwrap()
                .children()
                .count(),
            2
        );
        let content = TextBlock::new();
        content.set_text(Some("same control"));
        z.set_content(Some(Control::boxed(content.clone())));
        b.invoke(Some(z.clone()));
        w.update_layout();
        ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        assert!(content.is_loaded());
        assert!(b.selected_item() == Some(z.clone()));
        assert_eq!(title.text().as_deref(), Some("z"));
        assert_eq!(title_peer.get_name(), "z");
        b.set_show_page_title(false);
        assert!(!part(&b, "PART_PageTitle").is_visible());
        let targets = b.key_tip_targets();
        assert_eq!(targets.iter().count(), 4);
        action.set_is_visible(false);
        assert_eq!(targets.iter().count(), 3);
        z.navigation_button().unwrap().on_click();
        assert!(b.selected_item() == Some(z.clone()));
        b.items().remove(&Some(z.clone()));
        assert!(z.navigation_button().is_none());
        assert_eq!(b.selected_item(), Some(a));
        w.close();
        ferroui_controls::Application::current()
            .unwrap()
            .resources()
            .remove_merged_dictionary(&Rc::from(resources));
    });
}

#[test]
fn backstage_null_invocation_and_multicast_removal() {
    crate::group_tests::run(|| {
        let b = RibbonBackstage::new();
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| b.invoke(None))).is_err());
        let calls = Rc::new(RefCell::new(0));
        let c = calls.clone();
        let h = close_delegate(move || *c.borrow_mut() += 1);
        b.add_close_requested(Some(h.clone()));
        b.add_close_requested(Some(h.clone()));
        b.remove_close_requested(Some(h));
        b.invoke(Some(item("a", false)));
        assert_eq!(*calls.borrow(), 1);
    });
}

#[test]
fn backstage_retemplate_detaches_back_handler_and_releases_acyclic_native_graph() {
    crate::group_tests::run(|| {
        use ferroui_controls::{Button, StackPanel, templates::FuncControlTemplate};
        let b = RibbonBackstage::new();
        let page = item("page", true);
        b.items().add(Some(page.clone()));
        let calls = Rc::new(RefCell::new(0));
        let c = calls.clone();
        b.add_close_requested(Some(close_delegate(move || *c.borrow_mut() += 1)));
        let make = || {
            FuncControlTemplate::new(|_, scope| {
                let host = StackPanel::new();
                let back = Button::new();
                scope.register("PART_BackButton", back.clone().upcast());
                host.children().add(back);
                for name in ["PART_TopItems", "PART_BottomItems"] {
                    let panel = StackPanel::new();
                    scope.register(name, panel.clone().upcast());
                    host.children().add(panel);
                }
                host.upcast()
            })
        };
        b.set_template(Some(make()));
        b.apply_template();
        let old_back = b.back_button().unwrap();
        let old_navigation = page.navigation_button().unwrap();
        old_back.on_click();
        assert_eq!(*calls.borrow(), 1);
        let old_weak = old_navigation.downgrade();
        drop(old_navigation);
        b.set_template(Some(make()));
        b.apply_template();
        old_back.on_click();
        assert_eq!(*calls.borrow(), 1);
        assert!(old_weak.upgrade().is_none());
        b.back_button().unwrap().on_click();
        assert_eq!(*calls.borrow(), 2);
        let owner = b.downgrade();
        let nav = page.navigation_button().unwrap().downgrade();
        let entry = page.downgrade();
        drop(page);
        drop(b);
        assert!(owner.upgrade().is_none());
        assert!(nav.upgrade().is_none());
        assert!(entry.upgrade().is_none());
    });
}

#[test]
fn backstage_raw_keyboard_navigation_wraps_footer_and_preserves_page_arrows() {
    crate::group_tests::run(|| {
        use ferroui_base::input::{Key, PhysicalKey, RawInputModifiers};
        use ferroui_headless::HeadlessWindowExtensions;
        let b = RibbonBackstage::new();
        let a = item("a", true);
        let footer = item("footer", false);
        footer.set_placement(ferroribbon_core::RibbonBackstagePlacement::Bottom);
        let z = item("z", true);
        b.items().add(Some(a.clone()));
        b.items().add(Some(footer.clone()));
        b.items().add(Some(z.clone()));
        let (w, resources) = mount(&b);
        assert!(b.focus_selected_item());
        let focused = || {
            TopLevel::get_top_level(Some(&*b))
                .unwrap()
                .focus_manager()
                .get_focused_element()
                .unwrap()
        };
        for (key, expected) in [
            (Key::Down, z.navigation_button().unwrap()),
            (Key::Down, footer.navigation_button().unwrap()),
            (Key::Down, a.navigation_button().unwrap()),
            (Key::Up, footer.navigation_button().unwrap()),
        ] {
            w.key_press(key, RawInputModifiers::NONE, PhysicalKey::None, None);
            assert!(focused().ptr_eq(&expected));
            w.key_release(key, RawInputModifiers::NONE, PhysicalKey::None, None);
        }
        let text = ferroui_controls::TextBox::new();
        a.set_content(Some(Control::boxed(text.clone())));
        assert!(text.focus());
        w.key_press(Key::Down, RawInputModifiers::NONE, PhysicalKey::None, None);
        assert!(focused().ptr_eq(&text));
        w.key_release(Key::Down, RawInputModifiers::NONE, PhysicalKey::None, None);
        let closes = Rc::new(RefCell::new(0));
        let c = closes.clone();
        b.add_close_requested(Some(close_delegate(move || *c.borrow_mut() += 1)));
        w.key_press(
            Key::Escape,
            RawInputModifiers::NONE,
            PhysicalKey::None,
            None,
        );
        w.key_release(
            Key::Escape,
            RawInputModifiers::NONE,
            PhysicalKey::None,
            None,
        );
        assert_eq!(*closes.borrow(), 1);
        w.close();
        w.set_content(None);
        ferroui_controls::Application::current()
            .unwrap()
            .resources()
            .remove_merged_dictionary(&Rc::from(resources));
    });
}

struct Clock {
    ticks: ferroui_base::reactive::LightweightSubject<ferroui_base::animation::SignedDuration>,
}
impl ferroui_base::reactive::IObservable<ferroui_base::animation::SignedDuration> for Clock {
    fn subscribe(
        &self,
        observer: Rc<
            dyn ferroui_base::reactive::IObserver<ferroui_base::animation::SignedDuration>,
        >,
    ) -> Rc<dyn IDisposable> {
        self.ticks.subscribe(observer)
    }
}
impl ferroui_base::animation::IClock for Clock {
    fn play_state(&self) -> ferroui_base::animation::PlayState {
        ferroui_base::animation::PlayState::Run
    }
    fn set_play_state(&self, _: ferroui_base::animation::PlayState) {}
}
impl ferroui_base::animation::IGlobalClock for Clock {}
#[test]
fn backstage_native_animation_uses_source_durations_and_cubic_easing() {
    crate::group_tests::run(|| {
        use ferroui_base::{
            FerroLocator,
            animation::{IGlobalClock, SignedDuration},
            reactive::IObserver,
        };
        let _scope = FerroLocator::enter_scope();
        let clock = Rc::new(Clock {
            ticks: ferroui_base::reactive::LightweightSubject::new(),
        });
        FerroLocator::current_mutable()
            .bind::<dyn IGlobalClock>()
            .to_constant(clock.clone() as Rc<dyn IGlobalClock>);
        let b = RibbonBackstage::new();
        b.items().add(Some(item("page", true)));
        let (w, resources) = mount(&b);
        let root = part(&b, "PART_ContentRoot");
        let tick = |ms| {
            clock.ticks.on_next(SignedDuration::from_milliseconds(ms));
            ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
        };
        tick(0.);
        tick(500.);
        b.animate_content();
        assert_eq!(root.opacity(), 0.);
        let transform = root.render_transform().unwrap();
        tick(500.);
        tick(590.);
        assert!((root.opacity() - 0.5).abs() < 1e-9);
        let expected = 24. * (1. - 90_f64 / 220.).powi(3);
        assert!((transform.value().m31 - expected).abs() < 1e-9);
        tick(680.);
        assert_eq!(root.opacity(), 1.);
        assert!(transform.value().m31 > 0.);
        tick(720.);
        assert_eq!(transform.value().m31, 0.);
        assert_eq!(root.opacity(), 1.);
        w.close();
        w.set_content(None);
        ferroui_controls::Application::current()
            .unwrap()
            .resources()
            .remove_merged_dictionary(&Rc::from(resources));
    });
}

#[test]
fn backstage_tracks_current_ribbon_theme_only_while_loaded() {
    crate::group_tests::run(|| {
        use ferroui_base::styling::ThemeVariant;
        let b = RibbonBackstage::new();
        let r = Ribbon::new();
        b.set_ribbon(Some(r.clone()));
        let before = b.get_value(ThemeVariant::requested_theme_variant_property());
        r.set_value(
            ThemeVariant::requested_theme_variant_property(),
            Some(ThemeVariant::dark()),
        );
        assert_eq!(
            b.get_value(ThemeVariant::requested_theme_variant_property()),
            before
        );
        let (w, resources) = mount(&b);
        r.set_value(
            ThemeVariant::requested_theme_variant_property(),
            Some(ThemeVariant::light()),
        );
        assert_eq!(
            b.get_value(ThemeVariant::requested_theme_variant_property()),
            Some(ThemeVariant::light())
        );
        r.set_value(
            ThemeVariant::requested_theme_variant_property(),
            Some(ThemeVariant::dark()),
        );
        assert_eq!(
            b.get_value(ThemeVariant::requested_theme_variant_property()),
            Some(ThemeVariant::dark())
        );
        b.set_ribbon(None);
        r.set_value(
            ThemeVariant::requested_theme_variant_property(),
            Some(ThemeVariant::light()),
        );
        assert_eq!(
            b.get_value(ThemeVariant::requested_theme_variant_property()),
            Some(ThemeVariant::dark())
        );
        w.close();
        w.set_content(None);
        ferroui_controls::Application::current()
            .unwrap()
            .resources()
            .remove_merged_dictionary(&Rc::from(resources));
    });
}

#[test]
fn native_presenter_transfer_requires_explicit_outgoing_teardown_reproducer() {
    crate::tests::run(|| {
        use ferroui_controls::presenters::ContentPresenter;
        let first = ContentPresenter::new();
        let second = ContentPresenter::new();
        let page = TextBlock::new();
        first.set_content(Some(Control::boxed(page.clone())));
        first.update_child();
        second.set_content(Some(Control::boxed(page.clone())));
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| second.update_child()))
                .is_err(),
            "native single-parent enforcement must not be misreported as automatic source template handoff"
        );
        first.set_content(None);
        first.set_content_template(None);
        second.update_child();
        assert_eq!(second.child(), Some(page.upcast()));
        assert!(first.child().is_none());
        second.set_content(None);
    });
}
