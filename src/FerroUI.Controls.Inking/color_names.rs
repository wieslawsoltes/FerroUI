//! Default palettes from the pinned WinUI InkToolbarPenButton and HighlighterButton sources.
use super::ink_toolbar_buttons::strings;
use super::InkHighContrastAdjustment;
use ferroui_controls::Control;
use ferroui_base::collections::FerroList;
use ferroui_base::controls::ResourceKey;
use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, IBrush};
use ferroui_base::platform::{ColorContrastPreference, PlatformThemeVariant};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::StyledElement;
use std::rc::Rc;

const PEN_COLORS: [u32; 30] = [
    0xff000000, 0xffffffff, 0xffd1d3d4, 0xffa7a9ac, 0xff808285, 0xff58595b, 0xffb31564, 0xffe61b1b,
    0xffff5500, 0xffffaa00, 0xffffce00, 0xffffe600, 0xffa2e61b, 0xff26e600, 0xff008055, 0xff00aacc,
    0xff004de6, 0xff3d00b8, 0xff6600cc, 0xff600080, 0xfff7d7c4, 0xffbb9167, 0xff8e562e, 0xff613d30,
    0xffff80ff, 0xffffc680, 0xffffff80, 0xff80ff9e, 0xff80d6ff, 0xffbcb3ff,
];
const HIGHLIGHTER_COLORS: [u32; 6] = [
    0xffffe600, 0xff26e600, 0xff44c8f5, 0xffec008c, 0xffff5500, 0xff6600cc,
];

pub(crate) fn default_palette(highlighter: bool) -> FerroList<Rc<dyn IBrush>> {
    let colors: &[u32] = if highlighter {
        &HIGHLIGHTER_COLORS
    } else {
        &PEN_COLORS
    };
    FerroList::from_items(colors.iter().map(|&value| {
        Rc::new(ImmutableSolidColorBrush::new(Color::from_uint32(value))) as Rc<dyn IBrush>
    }))
}

/// The five-bit lookup matches the source's color families, including nearby RGB values.
pub(crate) fn color_resource_key(color: Color) -> Option<&'static str> {
    let packed =
        |r: u8, g: u8, b: u8| ((r as u16 >> 3) << 10) | ((g as u16 >> 3) << 5) | (b as u16 >> 3);
    let key = packed(color.r, color.g, color.b);
    const COLORS: &[(u8, u8, u8, &str)] = &[
        (0x00, 0x00, 0x00, "InkToolbarColorNameBlack"),
        (0xFF, 0xFF, 0xFF, "InkToolbarColorNameWhite"),
        (0xD1, 0xD3, 0xD4, "InkToolbarColorNameSilver"),
        (0xA7, 0xA9, 0xAC, "InkToolbarColorNameGray"),
        (0x80, 0x82, 0x85, "InkToolbarColorNameDarkGray"),
        (0x58, 0x59, 0x5B, "InkToolbarColorNameCharcoal"),
        (0xB3, 0x15, 0x64, "InkToolbarColorNameMagenta"),
        (0xE6, 0x1B, 0x1B, "InkToolbarColorNameRed"),
        (0xFF, 0x55, 0x00, "InkToolbarColorNameRedOrange"),
        (0xFF, 0xAA, 0x00, "InkToolbarColorNameOrange"),
        (0xFF, 0xCE, 0x00, "InkToolbarColorNameGold"),
        (0xFF, 0xE6, 0x00, "InkToolbarColorNameYellow"),
        (0xA2, 0xE6, 0x1B, "InkToolbarColorNameGrassGreen"),
        (0x26, 0xE6, 0x00, "InkToolbarColorNameGreen"),
        (0x00, 0x80, 0x55, "InkToolbarColorNameDarkGreen"),
        (0x00, 0xAA, 0xCC, "InkToolbarColorNameTeal"),
        (0x00, 0x4D, 0xE6, "InkToolbarColorNameBlue"),
        (0x3D, 0x00, 0xB8, "InkToolbarColorNameIndigo"),
        (0x66, 0x00, 0xCC, "InkToolbarColorNameViolet"),
        (0x60, 0x00, 0x80, "InkToolbarColorNamePurple"),
        (0xF7, 0xD7, 0xC4, "InkToolbarColorNameBeige"),
        (0xBB, 0x91, 0x67, "InkToolbarColorNameLightBrown"),
        (0x8E, 0x56, 0x2E, "InkToolbarColorNameBrown"),
        (0x61, 0x3D, 0x30, "InkToolbarColorNameDarkBrown"),
        (0xFF, 0x80, 0xFF, "InkToolbarColorNamePastelPink"),
        (0xFF, 0xC6, 0x80, "InkToolbarColorNamePastelOrange"),
        (0xFF, 0xFF, 0x80, "InkToolbarColorNamePastelYellow"),
        (0x80, 0xFF, 0x9E, "InkToolbarColorNamePastelGreen"),
        (0x80, 0xD6, 0xFF, "InkToolbarColorNamePastelBlue"),
        (0xBC, 0xB3, 0xFF, "InkToolbarColorNamePastelPurple"),
        (0x44, 0xC8, 0xF5, "InkToolbarColorNameLightBlue"),
        (0xEC, 0x00, 0x8C, "InkToolbarColorNamePink"),
    ];
    COLORS
        .iter()
        .find_map(|&(r, g, b, name)| (packed(r, g, b) == key).then_some(name))
}

pub(crate) fn color_name(control: &StyledElement, color: Color) -> String {
    if let Some(key) = color_resource_key(color) {
        strings::text(control, key)
    } else {
        strings::text(control, "InkToolbarColorRgbFormat")
            .replace("%1!u!", &color.r.to_string())
            .replace("%2!u!", &color.g.to_string())
            .replace("%3!u!", &color.b.to_string())
    }
}

/// Accessible swatch names and tooltips update with application resource overrides.
pub(crate) fn bind_color_name(control: &Control, color: Option<Color>) {
    use ferroui_controls::automation::AutomationProperties;
    use ferroui_controls::ToolTip;
    use ferroui_base::data::BindingPriority;
    use ferroui_base::reactive::ObservableExt;
    use ferroui_base::BoxedValue;
    let key = color
        .map(|color| color_resource_key(color).unwrap_or("InkToolbarColorRgbFormat"))
        .unwrap_or("InkToolbarNonSolidColorName");
    let names = strings::observable(control, key).select(move |name| match color {
        Some(color) if key == "InkToolbarColorRgbFormat" => name
            .replace("%1!u!", &color.r.to_string())
            .replace("%2!u!", &color.g.to_string())
            .replace("%3!u!", &color.b.to_string()),
        _ => name,
    });
    control.bind(
        AutomationProperties::name_property(),
        names.clone().select(Some),
        BindingPriority::Style,
    );
    control.bind(
        ToolTip::tip_property(),
        names.select(|name| Some(Rc::new(name) as BoxedValue)),
        BindingPriority::Style,
    );
}

/// Source HighContrastHelper luminance/contrast math, shared by the toolbar and canvas.
pub(crate) fn contrast_ratio(a: Color, b: Color) -> f64 {
    fn luminance(color: Color) -> f64 {
        let linear = |byte: u8| {
            let value = f64::from(byte) / 255.0;
            if value <= 0.03928 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
    }
    let (a, b) = (luminance(a), luminance(b));
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

pub(crate) fn adjust_high_contrast_color(
    color: Color,
    background: Color,
    system: Color,
    mode: InkHighContrastAdjustment,
) -> Color {
    match mode {
        InkHighContrastAdjustment::UseOriginalColors => color,
        InkHighContrastAdjustment::UseSystemColors => system,
        InkHighContrastAdjustment::UseSystemColorsWhenNecessary
            if contrast_ratio(color, background) >= 7.0 =>
        {
            color
        }
        _ => system,
    }
}

fn resource_color(control: &Control, key: &str) -> Option<Color> {
    let value = control
        .try_find_resource(
            &ResourceKey::from(key),
            control.actual_theme_variant().as_ref(),
        )
        .flatten()?;
    if let Some(color) = value.downcast_ref::<Color>() {
        return Some(*color);
    }
    value
        .downcast_ref::<Rc<dyn IBrush>>()?
        .as_solid_color_brush()
        .map(|brush| brush.color())
}

/// Resolve actual theme/platform contrast using FerroUI; resources provide the system colors.
/// Platforms without a system palette use the corresponding light/dark text and background.
pub(crate) fn high_contrast_colors(control: &Control, highlighter: bool) -> Option<(Color, Color)> {
    let variant = control.actual_theme_variant();
    let theme_contrast = variant
        .as_ref()
        .is_some_and(|theme| theme.key().as_str() == Some("HighContrast"));
    let platform = control
        .presentation_source()
        .and_then(|source| source.platform_settings())
        .map(|settings| settings.get_color_values());
    let platform_contrast = platform
        .as_ref()
        .is_some_and(|values| values.contrast_preference() == ColorContrastPreference::High);
    if !theme_contrast && !platform_contrast {
        return None;
    }
    let dark = theme_contrast
        || ThemeVariant::to_platform_theme_variant(variant.as_ref())
            == Some(PlatformThemeVariant::Dark)
        || platform
            .as_ref()
            .is_some_and(|values| values.theme_variant() == PlatformThemeVariant::Dark);
    let background = resource_color(control, "SystemColorWindowColor")
        .unwrap_or_else(|| Color::from_uint32(if dark { 0xff000000 } else { 0xffffffff }));
    let system = resource_color(
        control,
        if highlighter {
            "SystemColorHighlightColor"
        } else {
            "SystemColorWindowTextColor"
        },
    )
    .unwrap_or_else(|| Color::from_uint32(if dark { 0xffffffff } else { 0xff000000 }));
    Some((background, system))
}
