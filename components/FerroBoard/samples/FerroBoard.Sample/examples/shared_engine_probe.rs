//! Opt-in native control-lifecycle diagnostic. Layout is passive compiled XAML;
//! callbacks exercise public library APIs rather than implementing keyboard behavior.
//! No settings, learning, clipboard or document files are read or written.
use ferroboard_controls::OnScreenKeyboard;
use ferroboard_core::editor::ITextInputTarget;
use ferroboard_sample::App;
use ferroui_base::{
    FerroObjectImpl, Ref,
    controls::NameScope,
    threading::{Dispatcher, DispatcherPriority},
};
use ferroui_controls::{
    AppBuilder, ApplicationImpl, ApplicationImplExt, Button, ContentControl, Control,
    NewApplication, TextBlock, TextBox, Window,
    application_lifetimes::ClassicDesktopStyleApplicationLifetime,
};
use ferroui_desktop::AppBuilderDesktopExtensions;
use ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers;
use std::{cell::Cell, error::Error, rc::Rc};

#[repr(C)]
struct ProbeApp {
    base: App,
}
ferroui_base::ferro_class!(ProbeApp: App);
ferroui_base::ferro_impl_classes!(ProbeApp: FerroObjectImpl);
impl NewApplication for ProbeApp {
    fn new_application() -> Ref<Self> {
        let app = ferroui_base::instantiate(Self {
            base: App::construct(),
        });
        app.set_name(Some("FerroBoard Shared Engine Check".into()));
        app
    }
}
impl ApplicationImpl for ProbeApp {
    fn initialize(this: &Self) {
        Self::parent_initialize(this);
    }
    fn on_framework_initialization_completed(_this: &Self) {}
}

struct Probe {
    window: Ref<Window>,
    first: Ref<OnScreenKeyboard>,
    second: Ref<OnScreenKeyboard>,
    field_a: Ref<TextBox>,
    field_b: Ref<TextBox>,
    active: Rc<dyn ITextInputTarget>,
    first_host: Ref<ContentControl>,
    status: Ref<TextBlock>,
    releases_a: Cell<u32>,
    released_b: Cell<bool>,
    failed: Cell<bool>,
}
impl Probe {
    fn report(&self) {
        let target = self.second.engine().target();
        let target_ok = if self.released_b.get() {
            target.is_none()
        } else {
            target.is_some_and(|t| Rc::ptr_eq(&t, &self.active))
        };
        let first_ok = self.releases_a.get() == 0 || self.first.target_element().is_none();
        let field_a_empty = self.field_a.text().unwrap_or_default().is_empty();
        let focus = self.field_b.is_focused();
        if !(target_ok && first_ok && field_a_empty && focus) {
            self.failed.set(true);
        }
        let result = if self.failed.get() { "FAILED" } else { "PASS" };
        let message = format!(
            "{result} | release A={} B={} | target={} focus={} | B={:?}",
            self.releases_a.get(),
            self.released_b.get(),
            target_ok,
            focus,
            self.field_b.text().unwrap_or_default()
        );
        self.status.set_text(Some(&message));
        self.window
            .set_title(Some(format!("FerroBoard Shared Engine Check | {message}")));
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let lifetime = ClassicDesktopStyleApplicationLifetime::new();
    AppBuilder::configure::<ProbeApp>()
        .use_platform_detect()
        .setup_with_lifetime(lifetime.clone());
    let window = ferroboard_sample::compiled_shared_engine::build_examples_sharedengine_xaml(
        Some(XamlIlRuntimeHelpers::create_root_service_provider_v3(None)),
    )?;
    let scope = NameScope::get_name_scope(&window).ok_or("probe name scope missing")?;
    macro_rules! named {
        ($name:literal, $ty:ty) => {
            scope
                .find($name)
                .and_then(|x| x.cast::<$ty>())
                .ok_or(concat!("missing ", $name))?
        };
    }
    let first = OnScreenKeyboard::new(None)?;
    let second = OnScreenKeyboard::new(Some(first.engine()))?;
    first.set_auto_attach(false);
    second.set_auto_attach(false);
    let settings = first.settings();
    settings.set_auto_capitalization(false);
    settings.set_auto_correct(false);
    settings.set_learn_words(false);
    settings.set_incognito_mode(true);
    settings.set_show_suggestions(false);
    settings.set_emoji_suggestions(false);
    settings.set_emoji_history_enabled(false);
    settings.set_clipboard_history_enabled(false);
    settings.set_clipboard_suggestion(false);
    settings.set_haptic_feedback(false);
    settings.set_key_height(40.0);
    let field_a = named!("FieldA", TextBox);
    let field_b = named!("FieldB", TextBox);
    first.attach_to(Some(field_a.clone().upcast()))?;
    second.attach_to(Some(field_b.clone().upcast()))?;
    let first_host = named!("ViewA", ContentControl);
    first_host.set_content(Some(Control::boxed(first.clone())));
    named!("ViewB", ContentControl).set_content(Some(Control::boxed(second.clone())));
    let active = second.engine().target().ok_or("active adapter missing")?;
    let state = Rc::new(Probe {
        window: window.clone(),
        first,
        second,
        active,
        field_a,
        field_b,
        first_host,
        status: named!("Status", TextBlock),
        releases_a: Cell::new(0),
        released_b: Cell::new(false),
        failed: Cell::new(false),
    });
    let release_a = named!("ReleaseA", Button);
    let weak = Rc::downgrade(&state);
    let a_token = release_a.click(move |_, _| {
        let state = weak.upgrade().expect("live shared-engine probe");
        state
            .first
            .release_native_subscriptions()
            .expect("release view A");
        state.first_host.set_content(None);
        state.releases_a.set(state.releases_a.get() + 1);
        state.report();
    });
    let release_b = named!("ReleaseB", Button);
    let weak = Rc::downgrade(&state);
    let b_token = release_b.click(move |_, _| {
        let state = weak.upgrade().expect("live shared-engine probe");
        state
            .second
            .release_native_subscriptions()
            .expect("release view B");
        state.released_b.set(true);
        state.report();
    });
    let weak = Rc::downgrade(&state);
    let token = state
        .second
        .engine()
        .text_committed()
        .add(Rc::new(move |_| {
            let weak = weak.clone();
            Dispatcher::ui_thread().post_local(
                move || {
                    if let Some(state) = weak.upgrade() {
                        state.report();
                    }
                },
                DispatcherPriority::BACKGROUND,
            );
        }));
    window.show();
    let safety_close = window.downgrade();
    ferroui_base::threading::DispatcherTimer::run_once(
        move || {
            if let Some(window) = safety_close.upgrade() {
                window.close();
            }
        },
        std::time::Duration::from_secs(600),
        DispatcherPriority::NORMAL,
    )
    .detach();
    let initial = state.clone();
    Dispatcher::ui_thread().post_local(
        move || {
            initial.field_b.focus();
            initial
                .status
                .set_text(Some("Ready: type a using keyboard B, then release view A"));
        },
        DispatcherPriority::BACKGROUND,
    );
    let exit = lifetime.start(&[]);
    release_a.remove_handler(Button::click_event(), a_token);
    release_b.remove_handler(Button::click_event(), b_token);
    state.second.engine().text_committed().remove(token);
    state.first.release_native_subscriptions()?;
    state.second.release_native_subscriptions()?;
    state.second.engine().dispose()?;
    lifetime.dispose();
    if exit != 0 || state.failed.get() || state.releases_a.get() < 2 || !state.released_b.get() {
        return Err("shared-engine native checks failed or incomplete".into());
    }
    Ok(())
}
