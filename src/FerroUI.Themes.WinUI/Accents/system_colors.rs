//! The system colours, as the resources `SystemColorWindowColor`, `SystemColorWindowTextColor`,
//! `SystemColorButtonFaceColor`, `SystemColorButtonTextColor`, `SystemColorHighlightColor`,
//! `SystemColorHighlightTextColor`, `SystemColorHotlightColor` and `SystemColorGrayTextColor`
//! (docs/porting/winui-theme.md, section 3.4; docs/porting/winui-calculator-gaps.md, section 7.3).
//!
//! WinUI 3 builds them from `GetSysColor` in a dictionary of the framework
//! (`FrameworkTheming::RebuildColorAndBrushResources` of microsoft-ui-xaml); the theme resources
//! name them, chiefly in their `HighContrast` dictionaries, and define none of them. This class
//! is the resource provider of the theme for them, beside `SystemAccentColors`:
//!
//! - in the high contrast variant ([`WinUITheme::high_contrast`](crate::WinUITheme::high_contrast))
//!   the system colours the platform reports ([`PlatformColorValues::system_colors`]: Windows
//!   while high contrast is on, macOS under "Increase contrast"), else the colours of "High
//!   Contrast Black" when the platform's variant is dark and of "High Contrast White" when it is
//!   light;
//! - in every other variant the standard colours of Windows, as the light and dark dictionaries
//!   of WinUI resolve them outside high contrast.
//!
//! It follows `color_values_changed` of the platform settings of its owner and raises a change of
//! the resources only when the colours of the high contrast variant change, so that the
//! `{DynamicResource}` references of the theme update live.

use ferroui_base::controls::{
    ResourceHostRef, ResourceKey, ResourceProvider, ResourceProviderImpl, ResourceValue, ResourcesChangedEventArgs,
};
use ferroui_base::media::Color;
use ferroui_base::metadata::into_markup_value;
use ferroui_base::platform::{IPlatformSettings, PlatformColorValues, PlatformThemeVariant, SystemColors as Scheme};
use ferroui_base::reactive::IDisposable;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, Visual};
use ferroui_controls::Application;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The system colours, as resources.
#[repr(C)]
pub struct SystemColors {
    base: ResourceProvider,
    /// The colours of the high contrast variant, once resolved: `None` until the first lookup
    /// and after a change of the owner.
    high_contrast: Cell<Option<Scheme>>,
    /// The subscription to the colour values of the platform settings of the owner.
    color_values_changed: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(SystemColors: ResourceProvider);
ferro_impl_classes!(SystemColors: FerroObjectImpl);
ferro_class_info!(SystemColors { new: SystemColors::new });

impl ResourceProviderImpl for SystemColors {
    fn has_resources(_this: &Self) -> bool {
        true
    }

    fn try_get_resource(this: &Self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        let name = key.as_str()?;
        if !name.starts_with("SystemColor") {
            return None;
        }
        let colors = if theme.is_some_and(|theme| *theme == crate::WinUITheme::high_contrast()) {
            this.high_contrast_colors()
        } else {
            Self::STANDARD
        };
        let color = Self::color_of(&colors, name)?;
        Some(into_markup_value(color))
    }

    fn on_add_owner(this: &Self, owner: &ResourceHostRef) {
        if let Some(platform_settings) = Self::get_from_owner(Some(owner)) {
            this.subscribe(&platform_settings);
        }

        this.high_contrast.set(None);
    }

    fn on_remove_owner(this: &Self, _owner: &ResourceHostRef) {
        let subscription = this.color_values_changed.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }

        this.high_contrast.set(None);
    }
}

impl SystemColors {
    pub const WINDOW_KEY: &'static str = "SystemColorWindowColor";
    pub const WINDOW_TEXT_KEY: &'static str = "SystemColorWindowTextColor";
    pub const BUTTON_FACE_KEY: &'static str = "SystemColorButtonFaceColor";
    pub const BUTTON_TEXT_KEY: &'static str = "SystemColorButtonTextColor";
    pub const HIGHLIGHT_KEY: &'static str = "SystemColorHighlightColor";
    pub const HIGHLIGHT_TEXT_KEY: &'static str = "SystemColorHighlightTextColor";
    pub const HOTLIGHT_KEY: &'static str = "SystemColorHotlightColor";
    pub const GRAY_TEXT_KEY: &'static str = "SystemColorGrayTextColor";

    /// The keys of the resources of the class.
    pub const KEYS: [&'static str; 8] = [
        Self::WINDOW_KEY,
        Self::WINDOW_TEXT_KEY,
        Self::BUTTON_FACE_KEY,
        Self::BUTTON_TEXT_KEY,
        Self::HIGHLIGHT_KEY,
        Self::HIGHLIGHT_TEXT_KEY,
        Self::HOTLIGHT_KEY,
        Self::GRAY_TEXT_KEY,
    ];

    /// The standard colours of Windows outside high contrast (`GetSysColor` with the default
    /// colours): the values of the light and dark variants.
    const STANDARD: Scheme = Scheme::new(Color::from_rgb(0xFF, 0xFF, 0xFF), Color::from_rgb(0x00, 0x00, 0x00))
        .with_button_face(Color::from_rgb(0xF0, 0xF0, 0xF0))
        .with_highlight(Color::from_rgb(0x00, 0x78, 0xD7))
        .with_highlight_text(Color::from_rgb(0xFF, 0xFF, 0xFF))
        .with_hotlight(Color::from_rgb(0x00, 0x66, 0xCC))
        .with_gray_text(Color::from_rgb(0x6D, 0x6D, 0x6D));

    /// Field initialisation; see [`ResourceProvider::construct`].
    pub fn construct() -> Self {
        Self { base: ResourceProvider::construct(), high_contrast: Cell::new(None), color_values_changed: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The colours of the high contrast variant for the colour values of the platform: the
    /// system colours the platform reports, else "High Contrast Black" when the platform's
    /// variant is dark and "High Contrast White" when it is light; "High Contrast Black" without
    /// platform settings.
    pub fn high_contrast_colors_of(color_values: Option<&PlatformColorValues>) -> Scheme {
        match color_values {
            Some(values) => values.system_colors().unwrap_or(match values.theme_variant() {
                PlatformThemeVariant::Dark => Scheme::high_contrast_black(),
                PlatformThemeVariant::Light => Scheme::high_contrast_white(),
            }),
            None => Scheme::high_contrast_black(),
        }
    }

    /// The colours of the high contrast variant for the platform settings of the owner. They are
    /// kept once the owner has platform settings (an element owner gets them when it is attached
    /// to a top level, and the subscription is made then).
    fn high_contrast_colors(&self) -> Scheme {
        if let Some(colors) = self.high_contrast.get() {
            return colors;
        }
        let Some(platform_settings) = Self::get_from_owner(self.owner().as_ref()) else {
            return Self::high_contrast_colors_of(None);
        };
        if self.color_values_changed.borrow().is_none() {
            self.subscribe(&platform_settings);
        }
        let colors = Self::high_contrast_colors_of(Some(&platform_settings.get_color_values()));
        self.high_contrast.set(Some(colors));
        colors
    }

    /// Follows the colour values of `platform_settings`.
    fn subscribe(&self, platform_settings: &Rc<dyn IPlatformSettings>) {
        let weak = self.to_ref().downgrade();
        let subscription = platform_settings.color_values_changed(Rc::new(move |e: &PlatformColorValues| {
            if let Some(this) = weak.upgrade() {
                this.platform_settings_on_color_values_changed(e);
            }
        }));
        if let Some(previous) = self.color_values_changed.borrow_mut().replace(subscription) {
            previous.dispose();
        }
    }

    /// The colour of the resource `name` in `colors`.
    fn color_of(colors: &Scheme, name: &str) -> Option<Color> {
        Some(match name {
            Self::WINDOW_KEY => colors.window(),
            Self::WINDOW_TEXT_KEY => colors.window_text(),
            Self::BUTTON_FACE_KEY => colors.button_face(),
            Self::BUTTON_TEXT_KEY => colors.button_text(),
            Self::HIGHLIGHT_KEY => colors.highlight(),
            Self::HIGHLIGHT_TEXT_KEY => colors.highlight_text(),
            Self::HOTLIGHT_KEY => colors.hotlight(),
            Self::GRAY_TEXT_KEY => colors.gray_text(),
            _ => return None,
        })
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

    /// A change of the colour values: the resources change when the colours of the high
    /// contrast variant do (the contrast preference alone changes the variant, not these
    /// resources).
    fn platform_settings_on_color_values_changed(&self, e: &PlatformColorValues) {
        let colors = Self::high_contrast_colors_of(Some(e));
        if self.high_contrast.replace(Some(colors)) == Some(colors) {
            return;
        }
        if let Some(owner) = self.owner() {
            owner.notify_hosted_resources_changed(ResourcesChangedEventArgs::create());
        }
    }
}
