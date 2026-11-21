//! Port of RibbonSpace.Uno/Theming/RibbonThemeResources.cs.
use ferroribbon_core::theming::{
    RibbonChromeStyle, RibbonColor, RibbonThemePalette, RibbonThemeStyle,
};
use ferroui_base::controls::{IResourceProvider, ResourceDictionary, ResourceProviderImpl};
use ferroui_base::media::{Color, FontFamily, SolidColorBrush};
use ferroui_base::platform::{IPlatformSettings, SystemColors};
use ferroui_base::reactive::IDisposable;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{
    CornerRadius, FerroLocator, FerroObjectImpl, FerroObjectImplExt, LocatorExtensions, Ref,
    Thickness, ferro_class, ferro_class_info, ferro_impl_classes, instantiate,
};
use ferroui_controls::Application;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

#[path = "resource_assignments.rs"]
mod assignments;
pub use assignments::{BRUSH_KEYS, SHAPE_KEYS};
type Brushes = HashMap<&'static str, Ref<SolidColorBrush>>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RibbonThemeResourceError {
    pub parameter: &'static str,
}
impl std::fmt::Display for RibbonThemeResourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Value cannot be null. (Parameter '{}')", self.parameter)
    }
}
impl std::error::Error for RibbonThemeResourceError {}

#[repr(C)]
pub struct RibbonThemeResources {
    base: ResourceDictionary,
    light: Brushes,
    dark: Brushes,
    high_contrast: Brushes,
    dictionaries: [Ref<ResourceDictionary>; 4],
    shared: RefCell<Option<Ref<ResourceDictionary>>>,
    palette: RefCell<Rc<RibbonThemePalette>>,
    chrome_style: Cell<RibbonChromeStyle>,
    style: Cell<RibbonThemeStyle>,
    accessibility: RefCell<Option<Rc<dyn IDisposable>>>,
}
ferro_class!(RibbonThemeResources:ResourceDictionary);
ferro_impl_classes!(RibbonThemeResources:ResourceProviderImpl);
ferro_class_info!(RibbonThemeResources {new:RibbonThemeResources::new,markup:{namespace:"FerroRibbon.Controls",
properties:[Palette:Rc<RibbonThemePalette>{get:RibbonThemeResources::palette},ChromeStyle:RibbonChromeStyle{get:RibbonThemeResources::chrome_style},Style:RibbonThemeStyle{get:RibbonThemeResources::style}],
methods:[
    try fn GetBrush(Option<String>)->Option<Ref<SolidColorBrush>> => |r:&Ref<RibbonThemeResources>,key:Option<String>|r.markup_get_brush(key,Some("Light".into())),
    try fn GetBrush(Option<String>,Option<String>)->Option<Ref<SolidColorBrush>> => |r:&Ref<RibbonThemeResources>,key:Option<String>,theme:Option<String>|r.markup_get_brush(key,theme),
    try fn SetColor(Option<String>,Color) => |r:&Ref<RibbonThemeResources>,key:Option<String>,color:Color|r.markup_set_color(key,color,Some("Light".into())),
    try fn SetColor(Option<String>,Color,Option<String>) => |r:&Ref<RibbonThemeResources>,key:Option<String>,color:Color,theme:Option<String>|r.markup_set_color(key,color,theme),
    try fn Apply(Option<Rc<RibbonThemePalette>>,RibbonChromeStyle) => RibbonThemeResources::apply_palette_and_chrome,
    try fn Apply(Option<Rc<RibbonThemePalette>>,RibbonChromeStyle,RibbonThemeStyle) => RibbonThemeResources::apply,
]}});

pub fn to_color(c: RibbonColor) -> Color {
    Color::from_argb(c.a, c.r, c.g, c.b)
}
pub fn to_ribbon_color(c: Color) -> RibbonColor {
    RibbonColor::new(c.a, c.r, c.g, c.b)
}
fn c(hex: &str) -> RibbonColor {
    RibbonColor::parse(Some(&hex.into())).expect("fixed source color")
}
fn new_brushes() -> Brushes {
    BRUSH_KEYS
        .iter()
        .map(|&key| {
            (
                key,
                SolidColorBrush::with_color(to_color(RibbonColor::TRANSPARENT)),
            )
        })
        .collect()
}
impl RibbonThemeResources {
    /// Logical assembly URI of the compiled port of the original Shared.xaml.
    pub const SHARED_URI: &'static str = "ferres://FerroRibbon/Themes/Shared.xaml";
    pub fn new() -> Ref<Self> {
        crate::register_types();
        Self::create(None)
    }
    pub fn with_shared_resources(shared: Ref<ResourceDictionary>) -> Ref<Self> {
        Self::create(Some(shared))
    }
    fn create(shared: Option<Ref<ResourceDictionary>>) -> Ref<Self> {
        instantiate(Self {
            base: ResourceDictionary::construct(),
            light: new_brushes(),
            dark: new_brushes(),
            high_contrast: new_brushes(),
            dictionaries: std::array::from_fn(|_| ResourceDictionary::new()),
            shared: RefCell::new(shared),
            palette: RefCell::new(RibbonThemePalette::word()),
            chrome_style: Cell::new(RibbonChromeStyle::Colorful),
            style: Cell::new(RibbonThemeStyle::Office),
            accessibility: RefCell::new(None),
        })
    }
    #[doc(hidden)]
    pub fn markup_get_brush(
        &self,
        key: Option<String>,
        theme: Option<String>,
    ) -> Result<Option<Ref<SolidColorBrush>>, RibbonThemeResourceError> {
        let key = key.ok_or(RibbonThemeResourceError { parameter: "key" })?;
        Ok(self.get_brush(&key, theme.as_deref()))
    }
    #[doc(hidden)]
    pub fn markup_set_color(
        &self,
        key: Option<String>,
        color: Color,
        theme: Option<String>,
    ) -> Result<(), RibbonThemeResourceError> {
        let key = key.ok_or(RibbonThemeResourceError { parameter: "key" })?;
        self.set_color(&key, color, theme.as_deref());
        Ok(())
    }
    pub fn brush_keys() -> &'static [&'static str] {
        BRUSH_KEYS
    }
    pub fn shape_keys() -> &'static [&'static str] {
        SHAPE_KEYS
    }
    pub fn palette(&self) -> Rc<RibbonThemePalette> {
        self.palette.borrow().clone()
    }
    pub fn chrome_style(&self) -> RibbonChromeStyle {
        self.chrome_style.get()
    }
    pub fn style(&self) -> RibbonThemeStyle {
        self.style.get()
    }
    pub fn high_contrast_variant() -> ThemeVariant {
        // Same value protocol as the selected native WinUITheme::HighContrast;
        // the theme package need not become a dependency of this control library.
        ThemeVariant::new("HighContrast", Some(ThemeVariant::dark()))
    }
    /// Null/unknown theme names select Light, exactly as the source switch.
    pub fn get_brush(&self, key: &str, theme: Option<&str>) -> Option<Ref<SolidColorBrush>> {
        let map = match theme {
            Some("Dark") => &self.dark,
            Some("HighContrast") => &self.high_contrast,
            _ => &self.light,
        };
        map.get(key).cloned()
    }
    pub fn set_color(&self, key: &str, color: Color, theme: Option<&str>) {
        if let Some(brush) = self.get_brush(key, theme) {
            brush.set_color(color);
        }
    }
    pub fn apply_palette_and_chrome(
        &self,
        palette: Option<Rc<RibbonThemePalette>>,
        chrome: RibbonChromeStyle,
    ) -> Result<(), RibbonThemeResourceError> {
        self.apply(palette, chrome, self.style())
    }
    pub fn apply(
        &self,
        palette: Option<Rc<RibbonThemePalette>>,
        chrome: RibbonChromeStyle,
        style: RibbonThemeStyle,
    ) -> Result<(), RibbonThemeResourceError> {
        let palette = palette.ok_or(RibbonThemeResourceError {
            parameter: "palette",
        })?;
        self.palette.replace(palette.clone());
        self.chrome_style.set(chrome);
        if self.style.replace(style) != style {
            self.apply_shapes(style);
        }
        self.apply_light(&palette, chrome);
        self.apply_dark(&palette, chrome);
        if style == RibbonThemeStyle::Cad {
            self.apply_cad_light(&palette, chrome);
            self.apply_cad_dark(&palette, chrome);
        }
        self.apply_high_contrast();
        Ok(())
    }
    fn set_brush(&self, map: &Brushes, key: &str, color: RibbonColor) {
        map[key].set_color(to_color(color));
    }
    fn apply_shapes(&self, style: RibbonThemeStyle) {
        let cad = style == RibbonThemeStyle::Cad;
        for dict in &self.dictionaries {
            for (key, value) in [
                (
                    "RibbonControlCornerRadius",
                    CornerRadius::uniform(if cad { 2. } else { 4. }),
                ),
                (
                    "RibbonCommandBarCornerRadius",
                    CornerRadius::uniform(if cad { 0. } else { 8. }),
                ),
                (
                    "RibbonPopupCornerRadius",
                    CornerRadius::uniform(if cad { 2. } else { 6. }),
                ),
                (
                    "RibbonTabCornerRadius",
                    if cad {
                        CornerRadius::new(2., 2., 0., 0.)
                    } else {
                        CornerRadius::uniform(4.)
                    },
                ),
                (
                    "RibbonGroupCaptionCornerRadius",
                    CornerRadius::uniform(if cad { 1. } else { 0. }),
                ),
            ] {
                dict.set(key, Some(Rc::new(value)));
            }
            for (key, value) in [
                (
                    "RibbonCommandBarMargin",
                    if cad {
                        Thickness::uniform(0.)
                    } else {
                        Thickness::new(6., 1., 6., 6.)
                    },
                ),
                (
                    "RibbonCommandBarBorderThickness",
                    if cad {
                        Thickness::new(0., 0., 0., 1.)
                    } else {
                        Thickness::uniform(1.)
                    },
                ),
                (
                    "RibbonTabMargin",
                    if cad {
                        Thickness::new(0., 3., 1., 0.)
                    } else {
                        Thickness::new(1., 3., 1., 1.)
                    },
                ),
                (
                    "RibbonTabRowPadding",
                    if cad {
                        Thickness::new(4., 0., 4., 0.)
                    } else {
                        Thickness::new(6., 0., 6., 0.)
                    },
                ),
                (
                    "RibbonGroupCaptionMargin",
                    Thickness::uniform(if cad { 1. } else { 0. }),
                ),
            ] {
                dict.set(key, Some(Rc::new(value)));
            }
        }
    }
    fn system_color(
        key: &str,
        fallback: RibbonColor,
        select: fn(&SystemColors) -> Color,
    ) -> RibbonColor {
        if let Some(values) = FerroLocator::current()
            .get_service::<dyn IPlatformSettings>()
            .map(|settings| settings.get_color_values())
            && values.is_high_contrast()
            && let Some(color) = values
                .system_colors()
                .as_ref()
                .map(select)
                .filter(|c| c.a != 0)
        {
            return to_ribbon_color(color);
        }
        Application::current()
            .and_then(|app| {
                app.resources()
                    .try_get_resource(&key.into(), None)
                    .flatten()
            })
            .and_then(|v| v.downcast_ref::<Color>().copied())
            .filter(|c| c.a != 0)
            .map(to_ribbon_color)
            .unwrap_or(fallback)
    }
    fn apply_high_contrast(&self) {
        let m = &self.high_contrast;
        let bg = Self::system_color("SystemColorWindowColor", c("#000000"), SystemColors::window);
        let fg = Self::system_color(
            "SystemColorWindowTextColor",
            c("#FFFFFF"),
            SystemColors::window_text,
        );
        let hl = Self::system_color(
            "SystemColorHighlightColor",
            c("#1AEBFF"),
            SystemColors::highlight,
        );
        let hl_text = Self::system_color(
            "SystemColorHighlightTextColor",
            c("#000000"),
            SystemColors::highlight_text,
        );
        let button_face =
            Self::system_color("SystemColorButtonFaceColor", bg, SystemColors::button_face);
        let disabled = Self::system_color(
            "SystemColorGrayTextColor",
            c("#3FF23F"),
            SystemColors::gray_text,
        );
        let hotlight = Self::system_color(
            "SystemColorHotlightColor",
            c("#FFFF00"),
            SystemColors::hotlight,
        );
        for &key in BRUSH_KEYS {
            self.set_brush(m, key, if key.contains("Background") { bg } else { fg });
        }
        for key in [
            "RibbonAccentBrush",
            "RibbonTabIndicatorBrush",
            "RibbonInputFocusBorderBrush",
            "RibbonGalleryItemSelectedBorderBrush",
            "RibbonItemCheckedBorderBrush",
            "RibbonTabSelectedForegroundBrush",
            "RibbonItemBorderHoverBrush",
            "RibbonFocusBrush",
        ] {
            self.set_brush(m, key, hl);
        }
        let mix = |b: RibbonColor, h: RibbonColor, amount: f64| {
            let lerp = |x: u8, y: u8| {
                (f64::from(x) + f64::from(i32::from(y) - i32::from(x)) * amount).round_ties_even()
                    as u8
            };
            RibbonColor::new(255, lerp(b.r, h.r), lerp(b.g, h.g), lerp(b.b, h.b))
        };
        let hover = mix(bg, hl, 0.3);
        let checked = mix(bg, hl, 0.4);
        let pressed = mix(bg, hl, 0.5);
        for (key, color) in [
            ("RibbonCommandBarBackgroundBrush", button_face),
            ("RibbonAccentForegroundBrush", hl_text),
            ("RibbonAccentTextBrush", hotlight),
            ("RibbonAccentSubtleBrush", checked),
            ("RibbonAccentSubtleStrongBrush", pressed),
            ("RibbonItemHoverBrush", hover),
            ("RibbonItemPressedBrush", pressed),
            ("RibbonItemCheckedBrush", checked),
            ("RibbonItemCheckedHoverBrush", pressed),
            ("RibbonTabHoverBrush", hover),
            ("RibbonTitleBarHoverBrush", hover),
            ("RibbonBackstagePaneHoverBrush", hover),
            ("RibbonBackstagePaneSelectedBrush", pressed),
            ("RibbonDisabledForegroundBrush", disabled),
            ("RibbonKeyTipBackgroundBrush", hl),
            ("RibbonKeyTipForegroundBrush", hl_text),
            ("RibbonKeyTipBorderBrush", fg),
            ("RibbonShadowBrush", c("#00000000")),
            ("RibbonSwatchBorderBrush", fg),
            ("RibbonScrollButtonBackgroundBrush", bg),
        ] {
            self.set_brush(m, key, color);
        }
        self.ensure_high_contrast_tracking();
    }
    fn ensure_high_contrast_tracking(&self) {
        if self.accessibility.borrow().is_some() {
            return;
        }
        if let Some(settings) = FerroLocator::current().get_service::<dyn IPlatformSettings>() {
            let weak = self.to_ref().downgrade();
            let old = Cell::new(settings.get_color_values());
            let token = settings.color_values_changed(Rc::new(move |values| {
                let previous = old.replace(*values);
                let changed = previous.contrast_preference() != values.contrast_preference()
                    || values.is_high_contrast()
                        && previous.system_colors() != values.system_colors();
                if changed && let Some(this) = weak.upgrade() {
                    this.apply_high_contrast();
                }
            }));
            self.accessibility.replace(Some(token));
        }
    }
}
impl FerroObjectImpl for RibbonThemeResources {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        let [light, fallback, dark, hc] = &this.dictionaries;
        for &key in BRUSH_KEYS {
            light.set(key, Some(Rc::new(this.light[key].clone())));
            fallback.set(key, Some(Rc::new(this.light[key].clone())));
            dark.set(key, Some(Rc::new(this.dark[key].clone())));
            hc.set(key, Some(Rc::new(this.high_contrast[key].clone())));
        }
        for dict in &this.dictionaries {
            dict.set(
                "RibbonFontFamily",
                Some(Rc::new(FontFamily::default_family())),
            );
        }
        this.apply_shapes(RibbonThemeStyle::Office);
        // Match source constructor order: fonts/shapes above, Shared load here,
        // themed dictionaries below, then the initial Apply.
        let shared_input = this.shared.borrow_mut().take();
        let shared = shared_input.unwrap_or_else(|| {
            let value = crate::compiled_shared::try_load(None, Self::SHARED_URI)
                .unwrap_or_else(|e| panic!("{e}"))
                .expect("compiled Shared.xaml document");
            ferroui_base::data::core::ValueTypes::as_object(&*value)
                .and_then(|value| value.cast::<ResourceDictionary>())
                .expect("Shared.xaml must return a ResourceDictionary")
        });
        let shared: Rc<dyn IResourceProvider> = shared.into();
        this.merged_dictionaries().add(shared);
        this.add_theme_dictionary(ThemeVariant::light(), light.clone());
        this.add_theme_dictionary(ThemeVariant::default(), fallback.clone());
        this.add_theme_dictionary(ThemeVariant::dark(), dark.clone());
        this.add_theme_dictionary(Self::high_contrast_variant(), hc.clone());
        this.apply_palette_and_chrome(Some(RibbonThemePalette::word()), RibbonChromeStyle::Neutral)
            .expect("non-null source preset");
    }
}
impl Drop for RibbonThemeResources {
    fn drop(&mut self) {
        if let Some(token) = self.accessibility.get_mut().take() {
            token.dispose();
        }
    }
}
