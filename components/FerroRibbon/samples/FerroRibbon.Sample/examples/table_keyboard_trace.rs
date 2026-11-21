//! Development-only observer of the real compiled sample; no focus/input workaround.
//! The Table button's accessibility Help contains the last eight trace records.
use ferroribbon::controls::{Ribbon, RibbonDropDownButton, RibbonGridPicker};
use ferroribbon_sample::{pages::WordPage, shell_page::ShellPage};
use ferroui_base::{
    FerroObjectImpl, Ref, WeakRef,
    controls::NameScopeExtensions,
    ferro_class, ferro_impl_classes,
    input::{InputElement, InputManager, KeyboardDevice, raw::RawKeyEventArgs},
    instantiate,
    interactivity::{Interactive, RoutingStrategies},
    metadata::MarkupDelegate,
    reactive::{IDisposable, ObservableExt},
    threading::{Dispatcher, DispatcherPriority},
};
use ferroui_controls::{
    AppBuilder, Application, ApplicationImpl, ApplicationImplExt, Button, Control, NewApplication,
    TopLevel, Window, automation::AutomationProperties, primitives::PopupFlyoutBase,
};
use ferroui_desktop::AppBuilderDesktopExtensions;
use std::{
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
};

struct Trace {
    window: WeakRef<Window>,
    table: WeakRef<RibbonDropDownButton>,
    picker: WeakRef<RibbonGridPicker>,
    ribbon: WeakRef<Ribbon>,
    last_key: RefCell<String>,
    mode_changes: Cell<usize>,
    lines: RefCell<VecDeque<String>>,
    sequence: Cell<usize>,
    cleanup: RefCell<Vec<Box<dyn FnOnce()>>>,
}
impl Trace {
    fn sample_later(this: &Rc<Self>, label: &'static str) {
        let weak = Rc::downgrade(this);
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(trace) = weak.upgrade() {
                    trace.record(label);
                }
            },
            DispatcherPriority::BACKGROUND,
        );
    }
    fn record(&self, message: &str) {
        let Some(window) = self.window.upgrade() else {
            return;
        };
        let focus = KeyboardDevice::instance().and_then(|k| k.focused_element());
        let focus_name = focus
            .as_ref()
            .map(|f| f.name().unwrap_or_else(|| f.get_type().name().into()))
            .unwrap_or_else(|| "<unnamed/none>".into());
        let table = self.table.upgrade();
        let picker = self.picker.upgrade();
        let ribbon = self.ribbon.upgrade();
        let main_origin = TopLevel::platform_impl(&window)
            .map(|p| p.point_to_screen(ferroui_base::Point::default()));
        let badge_layout = ribbon
            .as_ref()
            .and_then(|r| r.active_key_tips().iter().next().flatten())
            .map(|badge| {
                let root = TopLevel::get_top_level(Some(&badge));
                let root_info = root
                    .map(|root| {
                        format!(
                            "{} visible={} client={:?} origin={:?}",
                            root.get_type().name(),
                            root.is_visible(),
                            root.client_size(),
                            root.platform_impl()
                                .map(|p| p.point_to_screen(ferroui_base::Point::default()))
                        )
                    })
                    .unwrap_or_else(|| "detached".into());
                let text_count = badge
                    .get_visual_descendants()
                    .filter(|v| v.is::<ferroui_controls::TextBlock>())
                    .count();
                format!(
                    "{root_info} first={:?} desired={:?} text-nodes={text_count}",
                    badge.bounds(),
                    badge.desired_size()
                )
            })
            .unwrap_or_else(|| "none".into());
        self.sequence.set(self.sequence.get() + 1);
        let line = format!(
            "{} {message}; active={} enabled={} mode={} badges={} tips={} changes={} last=[{}] open={} table-focus={} picker-focus={} focus={focus_name} main-origin={main_origin:?} overlay=[{badge_layout}]",
            self.sequence.get(),
            window.is_active(),
            ribbon.as_ref().is_some_and(|r| r.is_key_tips_enabled()),
            ribbon.as_ref().is_some_and(|r| r.is_key_tip_mode()),
            ribbon.as_ref().map_or(-1, |r| r.active_key_tips().count()),
            ribbon.as_ref().map_or(-1, |r| r.current_key_tips().count()),
            self.mode_changes.get(),
            self.last_key.borrow(),
            table
                .as_ref()
                .and_then(|b| b.flyout())
                .is_some_and(|f| f.is_open()),
            table.as_ref().is_some_and(|b| b.is_focused()),
            picker.as_ref().is_some_and(|p| p.is_focused())
        );
        eprintln!("{line}");
        let help = {
            let mut lines = self.lines.borrow_mut();
            lines.push_back(line.clone());
            while lines.len() > 64 {
                lines.pop_front();
            }
            lines
                .iter()
                .skip(lines.len().saturating_sub(8))
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        };
        // No new controls, renaming of styled elements, focus calls or event handling.
        AutomationProperties::set_help_text(&window, Some(&help));
        if let Some(table) = table {
            AutomationProperties::set_help_text(&table, Some(&help));
        }
        window.set_title(Some(format!("Input trace | {line}")));
    }
    fn subscription(&self, value: Rc<dyn IDisposable>) {
        self.cleanup
            .borrow_mut()
            .push(Box::new(move || value.dispose()));
    }
    fn observe(this: &Rc<Self>, target: Ref<Control>, name: &'static str) {
        for (event, label) in [
            (InputElement::key_down_event(), "down"),
            (InputElement::key_up_event(), "up"),
        ] {
            let weak = Rc::downgrade(this);
            let token = target.add_handler_with(
                event,
                move |_, e| {
                    if let Some(trace) = weak.upgrade() {
                        let message = format!(
                            "{name} {label} {:?} modifiers={:?} physical={:?} symbol={:?} handled={}",
                            e.key,
                            e.key_modifiers,
                            e.physical_key,
                            e.key_symbol,
                            e.handled()
                        );
                        trace.last_key.replace(message.clone());
                        trace.record(&message);
                    }
                },
                RoutingStrategies::TUNNEL | RoutingStrategies::BUBBLE,
                true,
            );
            let weak = target.downgrade();
            this.cleanup.borrow_mut().push(Box::new(move || {
                if let Some(target) = weak.upgrade() {
                    target.remove_handler(event, token);
                }
            }));
        }
        for (event, label) in [
            (InputElement::got_focus_event(), "got-focus"),
            (InputElement::lost_focus_event(), "lost-focus"),
        ] {
            let weak = Rc::downgrade(this);
            let token = target.add_handler_with(
                event,
                move |_, _| {
                    if let Some(trace) = weak.upgrade() {
                        trace.record(&format!("{name} {label}"));
                    }
                },
                RoutingStrategies::BUBBLE,
                true,
            );
            let weak = target.downgrade();
            this.cleanup.borrow_mut().push(Box::new(move || {
                if let Some(target) = weak.upgrade() {
                    target.remove_handler(event, token);
                }
            }));
        }
        for (event, label) in [
            (Control::loaded_event(), "loaded"),
            (Control::unloaded_event(), "unloaded"),
        ] {
            let weak = Rc::downgrade(this);
            let token = target.add_handler(event, move |_, _| {
                if let Some(trace) = weak.upgrade() {
                    trace.record(&format!("{name} {label}"));
                }
            });
            let weak = target.downgrade();
            this.cleanup.borrow_mut().push(Box::new(move || {
                if let Some(target) = weak.upgrade() {
                    target.remove_handler(event, token);
                }
            }));
        }
    }
    fn install(window: &Ref<Window>) -> Rc<Self> {
        let shell = window
            .content()
            .as_ref()
            .and_then(Control::from_boxed)
            .and_then(|c| c.cast::<ShellPage>())
            .expect("real sample ShellPage");
        let page = shell
            .current_content()
            .expect("current page")
            .and_then(|p| p.cast::<WordPage>())
            .expect("loaded WordPage");
        let table = NameScopeExtensions::find::<RibbonDropDownButton>(&page, "InsertTable")
            .expect("compiled InsertTable");
        let picker = NameScopeExtensions::find::<RibbonGridPicker>(&page, "TablePicker")
            .expect("original TablePicker instance");
        let trace = Rc::new(Self {
            window: window.downgrade(),
            table: table.downgrade(),
            picker: picker.downgrade(),
            ribbon: page.ribbon().downgrade(),
            last_key: RefCell::new("none".into()),
            mode_changes: Cell::new(0),
            lines: RefCell::new(VecDeque::new()),
            sequence: Cell::new(0),
            cleanup: RefCell::new(Vec::new()),
        });
        let manager = InputManager::instance().expect("native input manager");
        for (observable, label) in [
            (manager.pre_process(), "raw-pre"),
            (manager.post_process(), "raw-post"),
        ] {
            let weak = Rc::downgrade(&trace);
            trace.subscription(observable.subscribe_fn(move |event| {
                let Some(trace) = weak.upgrade() else {
                    return;
                };
                let Some(window) = trace.window.upgrade() else {
                    return;
                };
                if event.root().try_root_element().is_none()
                    || !event.root().focus_root().ptr_eq(&window)
                {
                    return;
                }
                if let Some(key) = event.downcast_ref::<RawKeyEventArgs>() {
                    let message = format!(
                        "{label} {:?} {:?} modifiers={:?} physical={:?} handled={} timestamp={}",
                        key.type_(),
                        key.key(),
                        key.modifiers(),
                        key.physical_key(),
                        key.handled(),
                        key.timestamp()
                    );
                    trace.last_key.replace(message.clone());
                    trace.record(&message);
                    if label == "raw-post" {
                        Self::sample_later(&trace, "settled raw input layout sample");
                    }
                }
            }));
        }
        Self::observe(&trace, window.clone().upcast(), "window");
        Self::observe(&trace, table.clone().upcast(), "table");
        Self::observe(&trace, picker.clone().upcast(), "picker");
        Self::observe(&trace, page.ribbon().upcast(), "ribbon");
        let weak = Rc::downgrade(&trace);
        let mode_changed = MarkupDelegate::new(move |_| {
            if let Some(trace) = weak.upgrade() {
                trace.mode_changes.set(trace.mode_changes.get() + 1);
                trace.record("KeyTip mode changed");
                Self::sample_later(&trace, "KeyTip background layout sample");
            }
            None
        });
        let ribbon = page.ribbon();
        ribbon.add_key_tip_mode_changed(Some(mode_changed.clone()));
        let weak_ribbon = ribbon.downgrade();
        trace.cleanup.borrow_mut().push(Box::new(move || {
            if let Some(ribbon) = weak_ribbon.upgrade() {
                ribbon.remove_key_tip_mode_changed(Some(mode_changed));
            }
        }));
        let weak = Rc::downgrade(&trace);
        let token = table.add_handler_with(
            Button::click_event(),
            move |_: &Interactive, _| {
                if let Some(trace) = weak.upgrade() {
                    trace.record("table Click");
                }
            },
            RoutingStrategies::BUBBLE,
            true,
        );
        let weak = table.downgrade();
        trace.cleanup.borrow_mut().push(Box::new(move || {
            if let Some(table) = weak.upgrade() {
                table.remove_handler(Button::click_event(), token);
            }
        }));
        let flyout = table.flyout().expect("source table flyout");
        let popup = flyout
            .cast::<PopupFlyoutBase>()
            .expect("native popup flyout");
        let weak = Rc::downgrade(&trace);
        trace.subscription(
            popup
                .opening(move |_| {
                    if let Some(t) = weak.upgrade() {
                        t.record("flyout opening");
                    }
                })
                .detach(),
        );
        let weak = Rc::downgrade(&trace);
        trace.subscription(
            popup
                .closing(move |_| {
                    if let Some(t) = weak.upgrade() {
                        t.record("flyout closing");
                    }
                })
                .detach(),
        );
        let weak = Rc::downgrade(&trace);
        trace.subscription(
            flyout
                .opened(move || {
                    if let Some(t) = weak.upgrade() {
                        t.record("flyout opened");
                    }
                })
                .detach(),
        );
        let weak = Rc::downgrade(&trace);
        trace.subscription(
            flyout
                .closed(move || {
                    if let Some(t) = weak.upgrade() {
                        t.record("flyout closed");
                    }
                })
                .detach(),
        );
        let weak = Rc::downgrade(&trace);
        trace.subscription(
            window
                .activated(move || {
                    if let Some(t) = weak.upgrade() {
                        t.record("window activated");
                    }
                })
                .detach(),
        );
        let weak = Rc::downgrade(&trace);
        trace.subscription(
            window
                .deactivated(move || {
                    if let Some(t) = weak.upgrade() {
                        t.record("window deactivated");
                    }
                })
                .detach(),
        );
        trace.record("installed");
        trace
    }
}
impl Drop for Trace {
    fn drop(&mut self) {
        for cleanup in self.cleanup.get_mut().drain(..) {
            cleanup();
        }
    }
}

#[repr(C)]
struct App {
    base: Application,
}
ferro_class!(App:Application);
ferro_impl_classes!(App:FerroObjectImpl);
impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        instantiate(Self {
            base: Application::construct(),
        })
    }
}
impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        ferroribbon::localization::initialize_ui_culture_from_platform()
            .expect("native UI culture");
        ferroribbon_sample::register_types();
        this.resources().set(
            "SymbolThemeFontFamily",
            Some(Rc::new(ferroui_base::media::FontFamily::new(
                "ferres://FerroRibbon/Assets/Fonts/uno-fluentui-assets.ttf#Symbols",
            ))),
        );
        this.set_name(Some("FerroRibbon Input Trace".into()));
        this.styles().add(
            ferroui_themes_simple::SimpleTheme::new().upcast::<ferroui_base::styling::Styles>(),
        );
    }
    fn on_framework_initialization_completed(this: &Self) {
        if let Some(desktop) = this
            .application_lifetime()
            .as_ref()
            .and_then(|l| l.as_classic_desktop_style_application_lifetime())
        {
            let window = ferroribbon_sample::create_shell_window();
            let state = Rc::new(RefCell::new(None));
            let out = state.clone();
            let weak = window.downgrade();
            window
                .opened(move || {
                    let out = out.clone();
                    let weak = weak.clone();
                    Dispatcher::ui_thread().post_local(
                        move || {
                            if let Some(window) = weak.upgrade() {
                                out.replace(Some(Trace::install(&window)));
                            }
                        },
                        DispatcherPriority::BACKGROUND,
                    );
                })
                .detach();
            window
                .closed(move || {
                    state.take();
                })
                .detach();
            desktop.set_main_window(Some(window));
        }
        Self::parent_on_framework_initialization_completed(this);
    }
}
fn main() {
    if std::env::args().skip(1).eq(["--print-build-fingerprint"]) {
        println!(
            "sample={}",
            option_env!("FERRORIBBON_BUILD_FINGERPRINT").unwrap_or("development")
        );
        println!(
            "library={}",
            ferroribbon::build_source_fingerprint().unwrap_or("development")
        );
        return;
    }
    AppBuilder::configure::<App>()
        .with(Rc::new(ferroui_base::media::FontManagerOptions {
            font_family_mappings: Some(std::collections::HashMap::from([
                (
                    "Segoe Fluent Icons".to_owned(),
                    ferroui_base::media::FontFamily::new(
                        "ferres://FerroRibbon/Assets/Fonts/uno-fluentui-assets.ttf#Symbols",
                    ),
                ),
                (
                    "Segoe MDL2 Assets".to_owned(),
                    ferroui_base::media::FontFamily::new(
                        "ferres://FerroRibbon/Assets/Fonts/uno-fluentui-assets.ttf#Symbols",
                    ),
                ),
            ])),
            ..Default::default()
        }))
        .use_platform_detect()
        .start_with_classic_desktop_lifetime(&[]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::{
        input::{Key, PhysicalKey, RawInputModifiers},
        threading::CancellationToken,
    };
    use ferroui_controls::TopLevel;
    use ferroui_headless::{
        FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, HeadlessUnitTestSession,
        HeadlessWindowExtensions,
    };

    #[test]
    fn observer_keeps_actual_sample_keyboard_route_and_retires_without_owning_controls() {
        let session = HeadlessUnitTestSession::start_new(|| {
            AppBuilder::configure::<App>()
                .use_platform_detect()
                .use_headless(FerroHeadlessPlatformOptions {
                    use_headless_drawing: false,
                    ..Default::default()
                })
        });
        let result = session
            .dispatch(
                || {
                    let window = ferroribbon_sample::create_shell_window();
                    window.show();
                    Dispatcher::ui_thread().run_jobs(None);
                    let trace = Trace::install(&window);
                    let shell = window
                        .content()
                        .as_ref()
                        .and_then(Control::from_boxed)
                        .and_then(|c| c.cast::<ShellPage>())
                        .unwrap();
                    let page = shell
                        .current_content()
                        .unwrap()
                        .unwrap()
                        .cast::<WordPage>()
                        .unwrap();
                    let table = trace.table.upgrade().unwrap();
                    let picker = trace.picker.upgrade().unwrap();
                    let flyout = table.flyout().unwrap();
                    let ribbon = page.ribbon();
                    let home = ribbon
                        .tab_headers()
                        .iter()
                        .find(|h| h.tab().id().as_deref() == Some("home"))
                        .unwrap()
                        .clone();
                    assert!(home.focus());
                    window.key_press(Key::F10, RawInputModifiers::NONE, PhysicalKey::F10, None);
                    window.key_release(Key::F10, RawInputModifiers::NONE, PhysicalKey::F10, None);
                    Dispatcher::ui_thread().run_jobs(None);
                    assert!(ribbon.is_key_tip_mode());
                    assert!(ribbon.active_key_tips().count() > 0);
                    assert!(trace.last_key.borrow().contains("F10"));
                    assert_eq!(trace.mode_changes.get(), 1);
                    window.key_press(Key::F10, RawInputModifiers::NONE, PhysicalKey::F10, None);
                    window.key_release(Key::F10, RawInputModifiers::NONE, PhysicalKey::F10, None);
                    Dispatcher::ui_thread().run_jobs(None);
                    assert!(!ribbon.is_key_tip_mode());
                    assert_eq!(trace.mode_changes.get(), 2);
                    window.key_press(Key::F10, RawInputModifiers::NONE, PhysicalKey::F10, None);
                    window.key_release(Key::F10, RawInputModifiers::NONE, PhysicalKey::F10, None);
                    Dispatcher::ui_thread().run_jobs(None);
                    assert!(ribbon.is_key_tip_mode());
                    assert_eq!(trace.mode_changes.get(), 3);
                    window.key_press(
                        Key::Escape,
                        RawInputModifiers::NONE,
                        PhysicalKey::None,
                        None,
                    );
                    window.key_release(
                        Key::Escape,
                        RawInputModifiers::NONE,
                        PhysicalKey::None,
                        None,
                    );
                    Dispatcher::ui_thread().run_jobs(None);
                    assert!(!ribbon.is_key_tip_mode());
                    // Original PopKeyTipLevel pops the last navigator scope before
                    // HideKeyTips checks wasActive: root Escape emits no false event.
                    assert_eq!(trace.mode_changes.get(), 3);
                    assert!(
                        trace
                            .lines
                            .borrow()
                            .iter()
                            .any(|line| line.contains("raw-pre") && line.contains("F10"))
                    );
                    assert!(trace.last_key.borrow().contains("raw-post"));
                    assert!(
                        trace
                            .lines
                            .borrow()
                            .iter()
                            .any(|line| line.contains("mode=true") && line.contains("F10"))
                    );
                    // Source SetActiveContextualGroups only changes visibility.
                    // Reapplying an already-visible group does not select it again.
                    for (opening, selected) in [(Key::Space, "tableDesign"), (Key::Enter, "insert")]
                    {
                        page.ribbon().set_selected_tab_id(Some("insert".into()));
                        Dispatcher::ui_thread().run_jobs(None);
                        assert!(table.focus());
                        window.key_press(opening, RawInputModifiers::NONE, PhysicalKey::None, None);
                        window.key_release(
                            opening,
                            RawInputModifiers::NONE,
                            PhysicalKey::None,
                            None,
                        );
                        Dispatcher::ui_thread().run_jobs(None);
                        assert!(flyout.is_open());
                        assert!(picker.is_focused());
                        for key in [Key::Right, Key::Down] {
                            let top = TopLevel::get_top_level(Some(&picker)).unwrap();
                            top.key_press(key, RawInputModifiers::NONE, PhysicalKey::None, None);
                            top.key_release(key, RawInputModifiers::NONE, PhysicalKey::None, None);
                        }
                        assert_eq!(
                            picker.highlighted_size(),
                            ferroribbon_core::model::RibbonGridSize::new(2, 2)
                        );
                        let top = TopLevel::get_top_level(Some(&picker)).unwrap();
                        top.key_press(Key::Enter, RawInputModifiers::NONE, PhysicalKey::None, None);
                        window.key_release(
                            Key::Enter,
                            RawInputModifiers::NONE,
                            PhysicalKey::None,
                            None,
                        );
                        Dispatcher::ui_thread().run_jobs(None);
                        assert!(!flyout.is_open());
                        assert_eq!(page.ribbon().selected_tab_id().as_deref(), Some(selected));
                    }
                    assert!(
                        trace
                            .lines
                            .borrow()
                            .iter()
                            .any(|l| l.contains("picker down"))
                    );
                    assert!(trace.lines.borrow().len() <= 64);
                    assert!(
                        AutomationProperties::get_help_text(&table)
                            .unwrap()
                            .contains("open=false")
                    );
                    assert!(window.title().unwrap().contains("active=true"));
                    let weak = Rc::downgrade(&trace);
                    drop(trace);
                    assert!(weak.upgrade().is_none(), "observer callbacks must be weak");
                    assert!(table.focus());
                    window.close();
                    window.set_content(None);
                },
                CancellationToken::none(),
            )
            .wait();
        session.dispose();
        result.unwrap();
    }
}
