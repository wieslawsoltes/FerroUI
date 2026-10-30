//! The window of the gallery (the class of `MainWindow.xaml`): the toolbar (theme variant,
//! density, flow direction, accent colour), the list of the pages and the selected page in its
//! host. The choices and the list are filled here from the tables of the gallery
//! ([`Variant::ALL`], [`Density::ALL`], [`FLOWS`], [`ACCENTS`], [`PAGES`]).

use crate::capture::{self, Setting};
use crate::capture_host::CaptureHost;
use crate::markup::xaml_class;
use crate::pages::PAGES;
use crate::theme::{self, Density, Variant};
use crate::App;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::Color;
use ferroui_base::threading::{DispatcherPriority, DispatcherTimer};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, Thickness, VisualImpl,
};
use ferroui_controls::platform::PlatformManager;
use ferroui_controls::primitives::{SelectingItemsControl, TemplatedControlImpl};
use ferroui_controls::{
    Application, ComboBox, ContentControlImpl, ControlImpl, ListBox, TextBlock, TopLevelImpl, Window, WindowBaseImpl,
    WindowImpl,
};
use ferroui_themes_material::MaterialMotionScheme;
use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

/// The flow directions of the toolbar: the name and whether it is right to left.
pub const FLOWS: [(&str, bool); 2] = [("Left to right", false), ("Right to left", true)];

/// Seed choices: None restores the AndroidX Expressive baseline; other choices
/// generate complete HCT palettes.
pub const ACCENTS: [(&str, Option<Color>); 6] = [
    ("Baseline", None),
    ("Blue", Some(Color::from_rgb(0x00, 0x78, 0xD4))),
    ("Green", Some(Color::from_rgb(0x10, 0x7C, 0x10))),
    ("Purple", Some(Color::from_rgb(0x87, 0x64, 0xB8))),
    ("Orange", Some(Color::from_rgb(0xCA, 0x50, 0x10))),
    ("Red", Some(Color::from_rgb(0xC4, 0x2B, 0x1C))),
];

pub const MOTIONS: [(&str, MaterialMotionScheme, bool); 3] = [
    ("Expressive", MaterialMotionScheme::Expressive, false),
    ("Standard", MaterialMotionScheme::Standard, false),
    ("Reduced", MaterialMotionScheme::Expressive, true),
];

#[repr(C)]
pub struct MainWindow {
    base: Window,
    setting: Cell<Setting>,
}

ferro_class!(MainWindow: Window);
ferro_impl_classes!(
    MainWindow: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl,
    WindowBaseImpl,
    WindowImpl
);
ferro_class_info!(MainWindow { new: MainWindow::new });
xaml_class!(MainWindow, "/MainWindow.xaml");

/// A text as an item of a list.
fn text_item(text: &str) -> Option<BoxedValue> {
    Some(Rc::new(text.to_string()) as BoxedValue)
}

/// The entry of a table at the selected index of a list, if one is selected.
fn selected<T: Copy>(table: &[T], index: i32) -> Option<T> {
    usize::try_from(index).ok().and_then(|index| table.get(index)).copied()
}

impl MainWindow {
    pub fn construct() -> Self {
        Self {
            base: Window::construct(PlatformManager::create_window()),
            setting: Cell::new(Setting { variant: Variant::Light, density: Density::Normal, right_to_left: false }),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        for variant in Variant::ALL {
            this.variant_choice().items().add(text_item(variant.name()));
        }
        for density in Density::ALL {
            this.density_choice().items().add(text_item(density.name()));
        }
        for (name, _) in FLOWS {
            this.flow_choice().items().add(text_item(name));
        }
        for (name, _) in ACCENTS {
            this.accent_choice().items().add(text_item(name));
        }
        for (name, _, _) in MOTIONS {
            this.motion_choice().items().add(text_item(name));
        }
        for page in PAGES {
            this.page_list().items().add(text_item(page.title));
        }

        // The handlers hold the window weakly: the window holds the controls, which hold their
        // handlers.
        let selected_index: &'static FerroProperty = SelectingItemsControl::selected_index_property().as_property();
        for (control, on_change) in [
            (this.variant_choice().upcast::<SelectingItemsControl>(), MainWindow::on_variant_changed as fn(&MainWindow)),
            (this.density_choice().upcast(), MainWindow::on_density_changed),
            (this.flow_choice().upcast(), MainWindow::on_flow_changed),
            (this.accent_choice().upcast(), MainWindow::on_accent_changed),
            (this.motion_choice().upcast(), MainWindow::on_motion_changed),
            (this.page_list().upcast(), MainWindow::show_selected_page),
        ] {
            let weak = this.downgrade();
            control.property_changed(move |e| {
                if e.property() == selected_index {
                    if let Some(this) = weak.upgrade() {
                        on_change(&this);
                    }
                }
            });
        }

        this.variant_choice().set_selected_index(0);
        this.density_choice().set_selected_index(0);
        this.flow_choice().set_selected_index(0);
        this.accent_choice().set_selected_index(0);
        this.motion_choice().set_selected_index(0);
        this.page_list().set_selected_index(0);
        this.select_start_options();
        this
    }

    /// `FERROUI_MATERIAL_GALLERY_START_PAGE=<page id>` and
    /// `FERROUI_MATERIAL_GALLERY_START_VARIANT=<Light|Dark|HighContrast>` choose the page and the
    /// theme variant the window opens with (for screenshots of the window); unknown values are
    /// reported and ignored.
    fn select_start_options(&self) {
        if let Some(name) = std::env::var("FERROUI_MATERIAL_GALLERY_START_VARIANT").ok().filter(|v| !v.trim().is_empty()) {
            match Variant::parse(&name).and_then(|variant| Variant::ALL.iter().position(|v| *v == variant)) {
                Some(index) => self.variant_choice().set_selected_index(index as i32),
                None => println!("FERROUI_MATERIAL_GALLERY_START_VARIANT: unknown variant {name:?}"),
            }
        }
        if let Some(id) = std::env::var("FERROUI_MATERIAL_GALLERY_START_PAGE").ok().filter(|v| !v.trim().is_empty()) {
            match PAGES.iter().position(|page| page.id.eq_ignore_ascii_case(id.trim())) {
                Some(index) => self.page_list().set_selected_index(index as i32),
                None => println!("FERROUI_MATERIAL_GALLERY_START_PAGE: unknown page {id:?}"),
            }
        }
    }

    fn variant_choice(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("VariantChoice")
    }

    fn density_choice(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("DensityChoice")
    }

    fn flow_choice(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("FlowChoice")
    }

    fn accent_choice(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("AccentChoice")
    }

    fn motion_choice(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("MotionChoice")
    }

    fn page_list(&self) -> Ref<ListBox> {
        self.get_control::<ListBox>("PageList")
    }

    /// The host of the selected page.
    pub fn page_host(&self) -> Ref<CaptureHost> {
        self.get_control::<CaptureHost>("PageHost")
    }

    fn on_variant_changed(&self) {
        let Some(variant) = selected(&Variant::ALL, self.variant_choice().selected_index()) else { return };
        self.setting.set(Setting { variant, ..self.setting.get() });
        // The window follows the variant of the page.
        if let Some(application) = Application::current() {
            application.set_requested_theme_variant(Some(variant.theme_variant()));
        }
        self.page_host().set_variant(variant);
    }

    fn on_density_changed(&self) {
        let Some(density) = selected(&Density::ALL, self.density_choice().selected_index()) else { return };
        self.setting.set(Setting { density, ..self.setting.get() });
        if let Some(gallery_theme) = App::current_theme() {
            theme::set_density(&gallery_theme, density);
        }
    }

    fn on_flow_changed(&self) {
        let Some((_, right_to_left)) = selected(&FLOWS, self.flow_choice().selected_index()) else { return };
        let setting = Setting { right_to_left, ..self.setting.get() };
        self.setting.set(setting);
        self.page_host().set_page_flow_direction(setting.flow_direction());
    }

    fn on_accent_changed(&self) {
        let Some((_, accent)) = selected(&ACCENTS, self.accent_choice().selected_index()) else { return };
        if let Some(gallery_theme) = App::current_theme() {
            theme::set_accent(&gallery_theme, accent);
        }
    }

    fn on_motion_changed(&self) {
        if let (Some((_, scheme, reduced)), Some(theme)) = (
            selected(&MOTIONS, self.motion_choice().selected_index()), App::current_theme(),
        ) {
            theme.set_motion_scheme(scheme);
            theme.set_reduced_motion(reduced);
        }
    }

    /// Shows the selected page in the current setting; its pointer-over and pressed states are
    /// forced once it is laid out.
    fn show_selected_page(&self) {
        let Some(page) = usize::try_from(self.page_list().selected_index()).ok().and_then(|index| PAGES.get(index)) else {
            return;
        };
        let host = self.page_host();
        match capture::create_page(page) {
            Ok(control) => capture::show(&host, Some(control), self.setting.get()),
            Err(error) => {
                let message = TextBlock::new();
                message.set_text(Some(&format!("The page {} does not load: {error}", page.title)));
                message.set_margin(Thickness::uniform(24.0));
                host.set_page(Some(message.upcast()));
            }
        }
        self.set_title(Some(format!("Material theme gallery: {}", page.title)));

        let weak = self.to_ref().downgrade();
        let _timer = DispatcherTimer::run_once(
            move || {
                if let Some(this) = weak.upgrade() {
                    capture::settle(&this.page_host());
                }
            },
            Duration::from_millis(100),
            DispatcherPriority::BACKGROUND,
        );
    }
}
