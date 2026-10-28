//! The compiled-XAML macOS 26 theme and its live appearance preferences.
//! See `docs/porting/macos26-theme.md` for the calibrated metrics and native
//! comparison procedure. The control foundation is copied from the WinUI theme.

use crate::register_types::register_types;
use ferroui_base::controls::{
    IResourceNode, IResourceProvider, ResourceDictionary, ResourceHostRef, ResourceKey, ResourceValue,
    ResourcesChangedEventArgs,
};
use ferroui_base::metadata::{from_markup_value, IServiceProvider};
use ferroui_base::media::Color;
use ferroui_base::platform::{ColorContrastPreference, IPlatformSettings};
use ferroui_base::reactive::IDisposable;
use ferroui_base::styling::{IStyle, Styles, ThemeVariant};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_markup_enum, ferro_properties, instantiate, BoxedValue, DirectProperty,
    FerroObject, FerroObjectImpl, FerroObjectImplExt, FerroProperty, FerroPropertyChangedEventArgs, Ref,
};
use ferroui_markup_xaml::XamlLoadException;
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    /// The optional dark high-contrast variant. IncreaseContrast also works in Light.
    static HIGH_CONTRAST: ThemeVariant = ThemeVariant::new("HighContrast", Some(ThemeVariant::dark()));
}

/// Legacy density selection. Compact maps the standard controls to the Small size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DensityStyle {
    #[default]
    Normal,
    Compact,
}

ferro_markup_enum!(DensityStyle { Normal, Compact }, { namespace: "FerroUI.Themes.MacOS" });

/// macOS 26 control sizes. Values are in device-independent points.
/// Large and extra-large controls use capsule outlines; the smaller sizes
/// retain rounded rectangular outlines. Metrics are calibrated theme values.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ControlSize {
    Mini,
    Small,
    #[default]
    Regular,
    Large,
    ExtraLarge,
}

ferro_markup_enum!(ControlSize { Mini, Small, Regular, Large, ExtraLarge }, { namespace: "FerroUI.Themes.MacOS" });

impl ControlSize {
    /// Height of a standard single-line control in points.
    pub const fn height(self) -> f64 {
        match self { Self::Mini => 18.0, Self::Small => 22.0, Self::Regular => 26.0, Self::Large => 34.0, Self::ExtraLarge => 42.0 }
    }

    pub const fn font_size(self) -> f64 {
        match self { Self::Mini => 9.0, Self::Small => 11.0, Self::Regular => 13.0, Self::Large => 15.0, Self::ExtraLarge => 17.0 }
    }

    pub const fn corner_radius(self) -> f64 {
        match self { Self::Mini => 4.0, Self::Small => 5.0, Self::Regular => 6.0, Self::Large => 17.0, Self::ExtraLarge => 21.0 }
    }
}

/// Includes the macOS 26 theme in an application.
#[repr(C)]
pub struct MacOSTheme {
    base: Styles,
    compact_styles: OnceCell<Ref<ResourceDictionary>>,
    density_style: Cell<DensityStyle>,
    control_size: Cell<ControlSize>,
    reduce_motion: Cell<bool>,
    reduce_transparency: Cell<bool>,
    increase_contrast: Cell<bool>,
    window_active: Cell<bool>,
    accent_color: Cell<Option<Color>>,
    platform_settings: RefCell<Option<Rc<dyn IPlatformSettings>>>,
    accessibility_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(MacOSTheme: Styles);
ferro_class_info!(MacOSTheme {
    new: MacOSTheme::new,
    interfaces: [
        Rc<dyn IStyle> => MacOSTheme::style_of,
        Rc<dyn IResourceProvider> => MacOSTheme::resource_provider_of,
    ],
    markup: {
        constructors: [(Option<Rc<dyn IServiceProvider>>) => MacOSTheme::with_service_provider],
        fields: [HighContrast: ThemeVariant => MacOSTheme::high_contrast],
    },
});

impl FerroObjectImpl for MacOSTheme {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        if [
            Self::density_style_property().as_property(),
            Self::control_size_property().as_property(),
            Self::reduce_motion_property().as_property(),
            Self::reduce_transparency_property().as_property(),
            Self::increase_contrast_property().as_property(),
            Self::window_active_property().as_property(),
            Self::accent_color_property().as_property(),
        ].contains(&change.property()) {
            if let Some(owner) = this.owner() {
                owner.notify_hosted_resources_changed(ResourcesChangedEventArgs::create());
            }
        }
    }
}

ferro_properties! {
    impl MacOSTheme {
        pub fn control_size_property() -> DirectProperty<MacOSTheme, ControlSize> {
            FerroProperty::register_direct::<MacOSTheme, _>("ControlSize", |o| o.control_size(), Some(|o, v| o.set_control_size(v)), ControlSize::Regular)
        }
        pub fn reduce_motion_property() -> DirectProperty<MacOSTheme, bool> {
            FerroProperty::register_direct::<MacOSTheme, _>("ReduceMotion", |o| o.reduce_motion(), Some(|o, v| o.set_reduce_motion(v)), false)
        }
        pub fn reduce_transparency_property() -> DirectProperty<MacOSTheme, bool> {
            FerroProperty::register_direct::<MacOSTheme, _>("ReduceTransparency", |o| o.reduce_transparency(), Some(|o, v| o.set_reduce_transparency(v)), false)
        }
        pub fn increase_contrast_property() -> DirectProperty<MacOSTheme, bool> {
            FerroProperty::register_direct::<MacOSTheme, _>("IncreaseContrast", |o| o.increase_contrast(), Some(|o, v| o.set_increase_contrast(v)), false)
        }
        pub fn window_active_property() -> DirectProperty<MacOSTheme, bool> {
            FerroProperty::register_direct::<MacOSTheme, _>("WindowActive", |o| o.window_active(), Some(|o, v| o.set_window_active(v)), true)
        }
        pub fn accent_color_property() -> DirectProperty<MacOSTheme, Option<Color>> {
            FerroProperty::register_direct::<MacOSTheme, _>("AccentColor", |o| o.accent_color(), Some(|o, v| o.set_accent_color(v)), None)
        }
        /// Defines the `DensityStyle` property.
        pub fn density_style_property() -> DirectProperty<MacOSTheme, DensityStyle> {
            FerroProperty::register_direct::<MacOSTheme, _>(
                "DensityStyle",
                |o| o.density_style(),
                Some(|o, v| o.set_density_style(v)),
                DensityStyle::Normal,
            )
        }
    }
}

impl MacOSTheme {
    /// The URI of the document of the class.
    pub const DOCUMENT_URI: &'static str = "ferres://FerroUI.Themes.MacOS/MacOSTheme.xaml";

    /// Optional dark high-contrast variant. For increased contrast without changing
    /// appearance, set `IncreaseContrast` and keep the light/dark requested variant.
    pub fn high_contrast() -> ThemeVariant {
        HIGH_CONTRAST.with(Clone::clone)
    }

    /// Field initialisation; see [`Styles::construct`].
    pub fn construct() -> Self {
        Self {
            base: Styles::construct(), compact_styles: OnceCell::new(), density_style: Cell::new(DensityStyle::Normal),
            control_size: Cell::new(ControlSize::Regular), reduce_motion: Cell::new(false),
            reduce_transparency: Cell::new(false), increase_contrast: Cell::new(false),
            window_active: Cell::new(true), accent_color: Cell::new(None),
            platform_settings: RefCell::new(None), accessibility_subscription: RefCell::new(None),
        }
    }

    /// Creates the theme.
    ///
    /// # Panics
    /// Panics if the documents of the theme fail to load.
    pub fn new() -> Ref<Self> {
        Self::with_service_provider(None)
    }

    /// Creates the theme; `sp` is the parent's service provider.
    ///
    /// # Panics
    /// Panics if the documents of the theme fail to load, or if the loaded
    /// document lacks the compact styles.
    pub fn with_service_provider(sp: Option<Rc<dyn IServiceProvider>>) -> Ref<Self> {
        let this = instantiate(Self::construct());
        if let Err(error) = Self::load(sp, &this) {
            panic!("{error}");
        }

        let compact_styles = this.get_and_remove("CompactStyles");
        let compact_styles = from_markup_value::<Ref<ResourceDictionary>>(&Some(compact_styles.clone()))
            .unwrap_or_else(|| {
                panic!("Unable to cast object of type '{}' to type 'ResourceDictionary'.", (*compact_styles).type_name())
            });
        let _ = this.compact_styles.set(compact_styles);

        this
    }

    fn get_and_remove(&self, key: &str) -> BoxedValue {
        let resources = self.resources();
        let key = ResourceKey::from(key);
        let value = resources
            .try_get_value(&key)
            .flatten()
            .unwrap_or_else(|| panic!("Key {} was not found in the resources", key.as_str().unwrap_or_default()));
        resources.remove(&key);
        value
    }

    /// The density style of the theme (normal, compact).
    pub fn density_style(&self) -> DensityStyle {
        self.density_style.get()
    }

    pub fn set_density_style(&self, value: DensityStyle) {
        self.set_and_raise_cell(Self::density_style_property(), &self.density_style, value);
    }

    pub fn control_size(&self) -> ControlSize { self.control_size.get() }
    pub fn set_control_size(&self, value: ControlSize) { self.set_and_raise_cell(Self::control_size_property(), &self.control_size, value); }
    pub fn reduce_motion(&self) -> bool { self.reduce_motion.get() }
    pub fn set_reduce_motion(&self, value: bool) { self.set_and_raise_cell(Self::reduce_motion_property(), &self.reduce_motion, value); }
    pub fn reduce_transparency(&self) -> bool { self.reduce_transparency.get() }
    pub fn set_reduce_transparency(&self, value: bool) { self.set_and_raise_cell(Self::reduce_transparency_property(), &self.reduce_transparency, value); }
    pub fn increase_contrast(&self) -> bool { self.increase_contrast.get() }
    pub fn set_increase_contrast(&self, value: bool) { self.set_and_raise_cell(Self::increase_contrast_property(), &self.increase_contrast, value); }
    /// Applications hosting more than one window can install a theme per window
    /// and bind this property to the window's activation state.
    pub fn window_active(&self) -> bool { self.window_active.get() }
    pub fn set_window_active(&self, value: bool) { self.set_and_raise_cell(Self::window_active_property(), &self.window_active, value); }
    /// `None` follows the platform accent; setting a color overrides it for this theme.
    pub fn accent_color(&self) -> Option<Color> { self.accent_color.get() }
    pub fn set_accent_color(&self, value: Option<Color>) { self.set_and_raise_cell(Self::accent_color_property(), &self.accent_color, value); }

    /// Whether nonessential motion is disabled by this theme or the platform.
    pub fn effective_reduce_motion(&self) -> bool {
        self.reduce_motion() || self.platform_settings.borrow().as_ref().is_some_and(|settings| !settings.animations_enabled())
    }

    /// Whether glass must use the opaque fallback, including the system preference.
    pub fn effective_reduce_transparency(&self) -> bool {
        self.reduce_transparency() || self.platform_settings.borrow().as_ref().is_some_and(|settings| !settings.transparency_enabled())
    }

    /// Increased contrast preserves the selected light or dark appearance.
    pub fn effective_increase_contrast(&self) -> bool {
        self.increase_contrast() || self.platform_settings.borrow().as_ref().is_some_and(|settings| settings.get_color_values().contrast_preference() == ColorContrastPreference::High)
    }

    fn subscribe_to_platform(&self, owner: &ResourceHostRef) {
        self.unsubscribe_from_platform();
        let settings = crate::accents::SystemAccentColors::get_from_owner(Some(owner));
        *self.platform_settings.borrow_mut() = settings.clone();
        if let Some(settings) = settings {
            let weak = self.to_ref().downgrade();
            let subscription = settings.accessibility_settings_changed(Rc::new(move || {
                if let Some(theme) = weak.upgrade() {
                    if let Some(owner) = theme.owner() {
                        owner.notify_hosted_resources_changed(ResourcesChangedEventArgs::create());
                    }
                }
            }));
            *self.accessibility_subscription.borrow_mut() = Some(subscription);
        }
    }

    fn unsubscribe_from_platform(&self) {
        let subscription = self.accessibility_subscription.borrow_mut().take();
        *self.platform_settings.borrow_mut() = None;
        if let Some(subscription) = subscription { subscription.dispose(); }
    }

    /// Tries to find a resource within the theme: the compact resources are
    /// checked first when the density style is compact.
    pub fn try_get_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        if let Some(value) = crate::settings_resources::resolve(self, key, theme) {
            return Some(value);
        }
        if self.density_style.get() == DensityStyle::Compact {
            if let Some(value) = self.compact_styles.get().and_then(|styles| styles.try_get_resource(key, theme)) {
                return Some(value);
            }
        }

        let styles: &Styles = self;
        styles.try_get_resource(key, theme)
    }

    /// The theme as a style, for the style collections of applications and
    /// elements (`application.styles().add(theme.as_style())`).
    pub fn as_style(&self) -> Rc<dyn IStyle> {
        Rc::new(MacOSThemeHandle(self.to_ref()))
    }

    /// The theme as a resource provider.
    pub fn as_resource_provider(&self) -> Rc<dyn IResourceProvider> {
        Rc::new(MacOSThemeHandle(self.to_ref()))
    }

    fn style_of(this: Ref<Self>) -> Rc<dyn IStyle> {
        Rc::new(MacOSThemeHandle(this))
    }

    fn resource_provider_of(this: Ref<Self>) -> Rc<dyn IResourceProvider> {
        Rc::new(MacOSThemeHandle(this))
    }

    /// Populates `this` from the document of the class with the compiled
    /// markup (`compiled_xaml.rs`): the document compiled with every document
    /// it includes as one group, exactly as the run-time loader loads it.
    fn load(sp: Option<Rc<dyn IServiceProvider>>, this: &Ref<Self>) -> Result<(), XamlLoadException> {
        register_types();
        crate::compiled_xaml::populate(sp, this)
    }
}

impl Drop for MacOSTheme {
    fn drop(&mut self) {
        if let Some(subscription) = self.accessibility_subscription.get_mut().take() {
            subscription.dispose();
        }
    }
}

/// The style and resource provider interfaces of the theme: those of a
/// styles collection, with the resource lookup of the theme.
struct MacOSThemeHandle(Ref<MacOSTheme>);

impl IResourceNode for MacOSThemeHandle {
    fn reference_id(&self) -> *const () {
        let object: &FerroObject = &self.0;
        object as *const FerroObject as *const ()
    }

    fn has_resources(&self) -> bool {
        self.0.has_resources()
    }

    fn try_get_resource(&self, key: &ResourceKey, theme: Option<&ThemeVariant>) -> Option<ResourceValue> {
        self.0.try_get_resource(key, theme)
    }
}

impl IResourceProvider for MacOSThemeHandle {
    fn owner(&self) -> Option<ResourceHostRef> {
        self.0.owner()
    }

    fn owner_changed(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.0.owner_changed(move || handler())
    }

    fn add_owner(&self, owner: &ResourceHostRef) {
        self.0.add_owner(owner);
        self.0.subscribe_to_platform(owner);
    }

    fn remove_owner(&self, owner: &ResourceHostRef) {
        self.0.remove_owner(owner);
        self.0.unsubscribe_from_platform();
    }

    fn as_object(&self) -> Option<&FerroObject> {
        Some(&self.0)
    }
}

impl IStyle for MacOSThemeHandle {
    fn children(&self) -> Rc<Vec<Rc<dyn IStyle>>> {
        self.0.snapshot()
    }

    fn as_object(&self) -> Option<&FerroObject> {
        Some(&self.0)
    }

    fn as_resource_provider(&self) -> Option<&dyn IResourceProvider> {
        Some(self)
    }
}
