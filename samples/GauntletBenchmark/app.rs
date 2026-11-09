//! The application: the launch options the harness passes, the light Fluent theme, Roboto from
//! the files every app of the comparison loads, and the 1280 x 820 window of the desktop apps.

use crate::gauntlet::GauntletView;
use crate::wall::WallView;
use ferroui_base::media::fonts::{FontCollectionBase, FontCollectionBaseImpl};
use ferroui_base::media::FontManager;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::utilities::UriRef;
use ferroui_base::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use ferroui_controls::{Application, ApplicationImpl, ApplicationImplExt, Control, NewApplication, Window};
use ferroui_themes_fluent::FluentTheme;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::OnceLock;

/// What the launch asked of the gauntlet.
pub struct Launch {
    /// Load tier, 1 to 16 (the still wall: 1 to 20).
    pub tier: i32,
    /// Stop on this frame and hold still, for picture comparisons; 0 runs on.
    pub freeze: i32,
    /// Whether the screen is the still wall in place of the gauntlet.
    pub wall: bool,
}

static LAUNCH: OnceLock<Launch> = OnceLock::new();

impl Launch {
    /// Sets the options, once, before the application starts.
    pub fn set(launch: Launch) {
        if LAUNCH.set(launch).is_err() {
            panic!("the launch options are set once");
        }
    }

    /// The options of this launch: the defaults when none were set.
    pub fn get() -> &'static Launch {
        LAUNCH.get_or_init(|| Launch { tier: 5, freeze: 0, wall: false })
    }

    /// The screen asked for.
    pub fn screen() -> Ref<Control> {
        let launch = Self::get();
        if launch.wall {
            WallView::new(launch.tier, launch.freeze).upcast()
        } else {
            GauntletView::new(launch.tier, launch.freeze).upcast()
        }
    }

    /// Writes a `PERF` line where the harness reads it: standard output.
    pub fn log(line: &str) {
        println!("{line}");
    }

    /// The bytes of a Roboto file in the folder `PERF_FONTS` names (default `fonts`).
    pub fn read_font(file: &str) -> Vec<u8> {
        let folder = std::env::var_os("PERF_FONTS").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("fonts"));
        let path = folder.join(file);
        std::fs::read(&path).unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
    }
}

/// The window every desktop app opens for the gauntlet, in logical points.
const DESKTOP_WIDTH: f64 = 1280.0;
const DESKTOP_HEIGHT: f64 = 820.0;

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
        FontManager::current().add_font_collection(Rc::new(DeviceRoboto::new()));
    }

    fn on_framework_initialization_completed(this: &Self) {
        let lifetime = this.application_lifetime();
        if let Some(desktop) = lifetime.as_ref().and_then(|l| l.as_classic_desktop_style_application_lifetime()) {
            let window = Window::new();
            window.set_title(Some(if Launch::get().wall { "Still wall" } else { "Gauntlet" }.to_string()));
            window.set_width(DESKTOP_WIDTH);
            window.set_height(DESKTOP_HEIGHT);
            window.set_can_resize(false);
            window.set_content(Some(Control::boxed(Launch::screen())));
            desktop.set_main_window(Some(window));
        }
        Self::parent_on_framework_initialization_completed(this);
    }
}

/// Roboto from the files the other apps load, as [`Launch::read_font`] reads them.
pub struct DeviceRoboto {
    base: FontCollectionBase,
    key: url::Url,
}

impl DeviceRoboto {
    /// The family the screens name: Roboto in this collection.
    pub const FAMILY: &'static str = "fonts:Device#Roboto";

    pub fn new() -> Self {
        let collection = Self {
            base: FontCollectionBase::new(),
            key: UriRef::parse_absolute("fonts:Device").expect("the key of the collection"),
        };
        for file in ["Roboto-Regular.ttf", "Roboto-Bold.ttf"] {
            let mut stream = std::io::Cursor::new(Launch::read_font(file));
            FontCollectionBase::try_add_glyph_typeface_from_stream(&collection, &mut stream);
        }
        collection
    }
}

impl FontCollectionBaseImpl for DeviceRoboto {
    fn base(this: &Self) -> &FontCollectionBase {
        &this.base
    }

    fn key(this: &Self) -> url::Url {
        this.key.clone()
    }
}
