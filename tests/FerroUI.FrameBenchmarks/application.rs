//! The application of a scene in a window: the desktop program and the hosts of the devices
//! (`tests/FerroUI.FrameBenchmarks.Android`, `tests/FerroUI.FrameBenchmarks.iOS`) run the same one.

use crate::{report, scenes, FrameHost, Roboto};
use ferroui_base::media::FontManager;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::{Application, ApplicationImpl, ApplicationImplExt, Border, Control, NewApplication, Window};
use ferroui_themes_fluent::FluentTheme;
use std::rc::Rc;
use std::time::Duration;

/// Shows the scene `FRAMES_SCENE` names (or the host, [`crate::scene_from`]) in a [`FrameHost`]: in
/// a window of 1280 by 820 with a classic desktop lifetime, as the main view of an activity or of
/// a single view lifetime on a device. Prints `PERF scene <name>` when the scene is built.
#[repr(C)]
pub struct App {
    base: Application,
}

ferro_class!(App: Application);
ferro_impl_classes!(App: FerroObjectImpl);

impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        instantiate(Self { base: Application::construct() })
    }
}

impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        Self::parent_initialize(this);
        this.set_requested_theme_variant(Some(ThemeVariant::light()));
        this.styles().add(FluentTheme::new().as_style());
        FontManager::current().add_font_collection(Rc::new(Roboto::new()));
    }

    fn on_framework_initialization_completed(this: &Self) {
        let lifetime = this.application_lifetime();
        if let Some(desktop) = lifetime.as_ref().and_then(|l| l.as_classic_desktop_style_application_lifetime()) {
            let window = Window::new();
            window.set_title(Some("Frame benchmarks".to_string()));
            window.set_width(1280.0);
            window.set_height(820.0);
            window.set_can_resize(false);
            window.set_content(Some(Control::boxed(&stage())));
            desktop.set_main_window(Some(window));
        } else if let Some(activity) = lifetime.as_ref().and_then(|l| l.as_activity_application_lifetime()) {
            activity.set_main_view_factory(Some(Rc::new(|| -> Ref<Control> { stage().upcast() })));
        } else if let Some(single_view) = lifetime.as_ref().and_then(|l| l.as_single_view_application_lifetime()) {
            single_view.set_main_view(Some(stage().upcast()));
        }
        Self::parent_on_framework_initialization_completed(this);
    }
}

/// How often the source of the host is asked for the scene while it has no answer.
const SCENE_POLL: Duration = Duration::from_millis(4);

/// What the window or the main view shows: the scene in its [`FrameHost`], at once where its name
/// is known and as soon as the host names it otherwise.
fn stage() -> Ref<Border> {
    let stage = Border::new();
    if !show_scene(&stage) {
        let stage = stage.clone();
        let _timer = DispatcherTimer::run(move || !show_scene(&stage), SCENE_POLL, DispatcherPriority::NORMAL).detach();
    }
    stage
}

/// Builds the scene into `stage` when its name is known.
fn show_scene(stage: &Ref<Border>) -> bool {
    let Some(name) = report::scene_name() else { return false };
    let scene = scenes::scene(&name).unwrap_or_else(|| panic!("no scene {name}"));
    report::line(&format!("PERF scene {name}"));
    stage.set_child(&FrameHost::new((scene.build)()).upcast::<Control>());
    true
}
