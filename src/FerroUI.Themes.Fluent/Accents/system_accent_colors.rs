//! Port of `Accents/SystemAccentColors.cs`.

use ferroui_base::controls::{
    ResourceHostRef, ResourceKey, ResourceProvider, ResourceProviderImpl, ResourceValue, ResourcesChangedEventArgs,
};
use ferroui_base::media::{Color, HslColor};
use ferroui_base::metadata::into_markup_value;
use ferroui_base::platform::{IPlatformSettings, PlatformColorValues};
use ferroui_base::reactive::IDisposable;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, Visual};
use ferroui_controls::Application;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The accent colour of the system and its shades, as resources.
#[repr(C)]
pub struct SystemAccentColors {
    base: ResourceProvider,
    invalidate_colors: Cell<bool>,
    system_accent_color: Cell<Color>,
    system_accent_color_dark1: Cell<Color>,
    system_accent_color_dark2: Cell<Color>,
    system_accent_color_dark3: Cell<Color>,
    system_accent_color_light1: Cell<Color>,
    system_accent_color_light2: Cell<Color>,
    system_accent_color_light3: Cell<Color>,
    /// The subscription to the colour values of the platform settings of
    /// the owner.
    color_values_changed: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(SystemAccentColors: ResourceProvider);
ferro_impl_classes!(SystemAccentColors: FerroObjectImpl);
ferro_class_info!(SystemAccentColors { new: SystemAccentColors::new });

impl ResourceProviderImpl for SystemAccentColors {
    fn has_resources(_this: &Self) -> bool {
        true
    }

    fn try_get_resource(this: &Self, key: &ResourceKey, _theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        let color = match key.as_str()? {
            Self::ACCENT_KEY => &this.system_accent_color,
            Self::ACCENT_DARK1_KEY => &this.system_accent_color_dark1,
            Self::ACCENT_DARK2_KEY => &this.system_accent_color_dark2,
            Self::ACCENT_DARK3_KEY => &this.system_accent_color_dark3,
            Self::ACCENT_LIGHT1_KEY => &this.system_accent_color_light1,
            Self::ACCENT_LIGHT2_KEY => &this.system_accent_color_light2,
            Self::ACCENT_LIGHT3_KEY => &this.system_accent_color_light3,
            _ => return None,
        };
        this.ensure_colors();
        Some(into_markup_value(color.get()))
    }

    fn on_add_owner(this: &Self, owner: &ResourceHostRef) {
        if let Some(platform_settings) = Self::get_from_owner(Some(owner)) {
            let weak = this.to_ref().downgrade();
            let subscription = platform_settings.color_values_changed(Rc::new(move |e: &PlatformColorValues| {
                if let Some(this) = weak.upgrade() {
                    this.platform_settings_on_color_values_changed(e);
                }
            }));
            *this.color_values_changed.borrow_mut() = Some(subscription);
        }

        this.invalidate_colors.set(true);
    }

    fn on_remove_owner(this: &Self, _owner: &ResourceHostRef) {
        let subscription = this.color_values_changed.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }

        this.invalidate_colors.set(true);
    }
}

impl SystemAccentColors {
    pub const ACCENT_KEY: &'static str = "SystemAccentColor";
    pub const ACCENT_DARK1_KEY: &'static str = "SystemAccentColorDark1";
    pub const ACCENT_DARK2_KEY: &'static str = "SystemAccentColorDark2";
    pub const ACCENT_DARK3_KEY: &'static str = "SystemAccentColorDark3";
    pub const ACCENT_LIGHT1_KEY: &'static str = "SystemAccentColorLight1";
    pub const ACCENT_LIGHT2_KEY: &'static str = "SystemAccentColorLight2";
    pub const ACCENT_LIGHT3_KEY: &'static str = "SystemAccentColorLight3";

    const DEFAULT_SYSTEM_ACCENT_COLOR: Color = Color::from_rgb(0, 120, 215);

    /// Field initialisation; see [`ResourceProvider::construct`].
    pub fn construct() -> Self {
        Self {
            base: ResourceProvider::construct(),
            invalidate_colors: Cell::new(true),
            system_accent_color: Cell::new(Color::default()),
            system_accent_color_dark1: Cell::new(Color::default()),
            system_accent_color_dark2: Cell::new(Color::default()),
            system_accent_color_dark3: Cell::new(Color::default()),
            system_accent_color_light1: Cell::new(Color::default()),
            system_accent_color_light2: Cell::new(Color::default()),
            system_accent_color_light3: Cell::new(Color::default()),
            color_values_changed: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn ensure_colors(&self) {
        if self.invalidate_colors.get() {
            self.invalidate_colors.set(false);

            let platform_settings = Self::get_from_owner(self.owner().as_ref());

            let accent = platform_settings
                .map(|settings| settings.get_color_values().accent_color1())
                .unwrap_or(Self::DEFAULT_SYSTEM_ACCENT_COLOR);
            self.system_accent_color.set(accent);
            let (dark1, dark2, dark3, light1, light2, light3) = Self::calculate_accent_shades(accent);
            self.system_accent_color_dark1.set(dark1);
            self.system_accent_color_dark2.set(dark2);
            self.system_accent_color_dark3.set(dark3);
            self.system_accent_color_light1.set(light1);
            self.system_accent_color_light2.set(light2);
            self.system_accent_color_light3.set(light3);
        }
    }

    /// The platform settings of the owner: those of the application when
    /// the owner is the application, those of the visual when it is one.
    fn get_from_owner(owner: Option<&ResourceHostRef>) -> Option<Rc<dyn IPlatformSettings>> {
        match owner? {
            ResourceHostRef::Element(element) => element.cast::<Visual>().and_then(|visual| visual.get_platform_settings()),
            other => {
                // A host that is not an element: the application, if it is the current one.
                let application = Application::current()?;
                (ResourceHostRef::from(application.as_resource_host()) == *other)
                    .then(|| application.platform_settings())
                    .flatten()
            }
        }
    }

    /// The shades of an accent colour: three darker and three lighter ones.
    pub fn calculate_accent_shades(accent_color: Color) -> (Color, Color, Color, Color, Color, Color) {
        // dark1step = (hslAccent.L - SystemAccentColorDark1.L) * 255
        const DARK1_STEP: f64 = 28.5 / 255.0;
        const DARK2_STEP: f64 = 49.0 / 255.0;
        const DARK3_STEP: f64 = 74.5 / 255.0;
        // light1step = (SystemAccentColorLight1.L - hslAccent.L) * 255
        const LIGHT1_STEP: f64 = 39.0 / 255.0;
        const LIGHT2_STEP: f64 = 70.0 / 255.0;
        const LIGHT3_STEP: f64 = 103.0 / 255.0;

        let hsl_accent = accent_color.to_hsl();
        let shade = |lightness: f64| HslColor::new(hsl_accent.a, hsl_accent.h, hsl_accent.s, lightness).to_rgb();

        (
            // Darker shades
            shade(hsl_accent.l - DARK1_STEP),
            shade(hsl_accent.l - DARK2_STEP),
            shade(hsl_accent.l - DARK3_STEP),
            // Lighter shades
            shade(hsl_accent.l + LIGHT1_STEP),
            shade(hsl_accent.l + LIGHT2_STEP),
            shade(hsl_accent.l + LIGHT3_STEP),
        )
    }

    fn platform_settings_on_color_values_changed(&self, _e: &PlatformColorValues) {
        self.invalidate_colors.set(true);
        if let Some(owner) = self.owner() {
            owner.notify_hosted_resources_changed(ResourcesChangedEventArgs::create());
        }
    }
}
