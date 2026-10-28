//! Live preference resources, shared by compiled and runtime XAML.
//!
//! Colors and metrics are theme values, not private AppKit constants. Glass is
//! expressed through FerroUI's acrylic compositor contract, including an opaque
//! fallback. This does not assert equivalence to AppKit's optical refraction.

use crate::{ControlSize, DensityStyle, MacOSTheme};
use ferroui_base::animation::{IterationCount, PageSlide, SlideAxis, TimeSpan, Transitions};
use ferroui_base::animation::DoubleTransition;
use ferroui_base::animation::easings::Easing;
use ferroui_base::controls::{ResourceKey, ResourceValue};
use ferroui_base::media::{AcrylicBackgroundSource, BoxShadows, Color, ExperimentalAcrylicMaterial, SolidColorBrush};
use ferroui_base::metadata::{from_markup_value, into_markup_value};
use ferroui_base::styling::{Styles, ThemeVariant};
use ferroui_base::{CornerRadius, Rect, Thickness, Visual};
use ferroui_controls::{Canvas, WindowTransparencyLevel, WindowTransparencyLevelCollection};
use std::rc::Rc;

fn is_dark(variant: Option<&ThemeVariant>) -> bool {
    let Some(variant) = variant else { return true; };
    if *variant == ThemeVariant::light() { return false; }
    if *variant == ThemeVariant::dark() { return true; }
    variant.inherit_variant().as_ref().map_or(true, |parent| is_dark(Some(parent)))
}

fn is_high_contrast(variant: Option<&ThemeVariant>) -> bool {
    let mut current = variant.cloned();
    while let Some(value) = current {
        if value == MacOSTheme::high_contrast() { return true; }
        current = value.inherit_variant();
    }
    false
}

fn accent(theme: &MacOSTheme, variant: Option<&ThemeVariant>) -> Color {
    if let Some(value) = theme.accent_color() { return value; }
    // Bypass the preference layer: the original provider owns the subscription
    // to platform color changes and notifies the same resource host.
    let styles: &Styles = theme;
    styles.try_get_resource(&ResourceKey::from("SystemAccentColor"), variant)
        .and_then(|value| from_markup_value::<Color>(&value))
        .unwrap_or(Color::from_rgb(0, 122, 255))
}

// Explicit keys, not a suffix wildcard: application-specific shadows retain
// their own policy. Every consumer must use DynamicResource for live updates.
const DECORATIVE_SHADOW_KEYS: &[&str] = &[
    "MacOSWindowShadow", "MacOSSegmentShadow", "MacOSMenuShadow",
    "MacOSControlShadow", "MacOSSwitchKnobShadow", "MacOSToolTipShadow",
    "WindowDrawnDecorationsWindowBorderBoxShadow",
    "ThemeShadowFlyoutBoxShadow", "ThemeShadowSubMenuBoxShadow", "ThemeShadowToolTipBoxShadow",
];

fn shadow_margin(shadows: &BoxShadows) -> Thickness {
    // Use the renderer's bound calculation, including multiple/inset shadows,
    // spread and offsets. Round outwards rather than clipping fractional bounds.
    let content = Rect::new(0.0, 0.0, 1.0, 1.0);
    let bounds = shadows.transform_bounds(content);
    Thickness::new(
        (-bounds.x).max(0.0).ceil(),
        (-bounds.y).max(0.0).ceil(),
        (bounds.right() - content.right()).max(0.0).ceil(),
        (bounds.bottom() - content.bottom()).max(0.0).ceil(),
    )
}

fn solid(value: Color) -> ResourceValue { into_markup_value(SolidColorBrush::with_color(value)) }
fn color(value: u32) -> ResourceValue { into_markup_value(Color::from_uint32(value)) }

pub(crate) fn resolve(theme: &MacOSTheme, key: &ResourceKey, variant: Option<&ThemeVariant>) -> Option<ResourceValue> {
    let name = key.as_str()?;
    let dark = is_dark(variant);
    let high_contrast_variant = is_high_contrast(variant);
    let contrast = theme.effective_increase_contrast() || high_contrast_variant;
    let active = theme.window_active();
    let size = if theme.density_style() == DensityStyle::Compact { ControlSize::Small } else { theme.control_size() };

    if contrast && DECORATIVE_SHADOW_KEYS.contains(&name) {
        return Some(into_markup_value(BoxShadows::default()));
    }
    let shadow_key = match name {
        "MacOSMenuShadowMargin" => Some("MacOSMenuShadow"),
        "MacOSWindowShadowMargin" => Some("WindowDrawnDecorationsWindowBorderBoxShadow"),
        "MacOSToolTipShadowMargin" => Some("MacOSToolTipShadow"),
        _ => None,
    };
    if let Some(shadow_key) = shadow_key {
        // A different key, so this does not recurse back into the margin lookup.
        // Consult the live provider too: contrast can replace the shadow by none.
        let value = theme.try_get_resource(&ResourceKey::from(shadow_key), variant)?;
        let shadows = from_markup_value::<BoxShadows>(&value)?;
        return Some(into_markup_value(shadow_margin(&shadows)));
    }

    let duration = match name {
        "MacOSMotionFastDuration" => Some(100.0),
        "MacOSMotionStandardDuration" => Some(180.0),
        "MacOSMotionSlowDuration" => Some(280.0),
        _ => None,
    };
    if let Some(milliseconds) = duration {
        return Some(into_markup_value(if theme.effective_reduce_motion() { TimeSpan::ZERO } else { TimeSpan::from_milliseconds(milliseconds) }));
    }

    match name {
        "MacOSToolTipTransitions" => {
            let transition = DoubleTransition::new();
            transition.set_property(Some(Visual::opacity_property().as_property()));
            transition.set_duration(if theme.effective_reduce_motion() {
                TimeSpan::ZERO
            } else {
                TimeSpan::from_milliseconds(100.0)
            });
            // Preserve the original opacity transition's default easing.
            return Some(into_markup_value(Transitions::from_items([transition.into()])));
        }
        "MacOSSwitchKnobTransitions" => {
            // The control observes this resource; a transition buried in a setter
            // value cannot inherit that control's dynamic-resource context. Assign
            // an already configured collection, as for the page transitions below.
            let transition = DoubleTransition::new();
            transition.set_property(Some(Canvas::left_property().as_property()));
            transition.set_duration(if theme.effective_reduce_motion() {
                TimeSpan::ZERO
            } else {
                TimeSpan::from_milliseconds(180.0)
            });
            transition.set_easing(Easing::parse("CubicEaseOut").expect("built-in easing"));
            return Some(into_markup_value(Transitions::from_items([transition.into()])));
        }
        "MacOSNavigationTransition" | "MacOSModalTransition" => {
            // PageSlide is a plain transition object, so its Duration cannot host
            // DynamicResource. Replace the whole typed transition at its styled
            // consumer instead. The control retains its cancellation/gesture logic.
            let transition = Rc::new(PageSlide::with_duration(
                if theme.effective_reduce_motion() { TimeSpan::ZERO } else { TimeSpan::from_milliseconds(180.0) },
                if name == "MacOSModalTransition" { SlideAxis::Vertical } else { SlideAxis::Horizontal },
            ));
            transition.set_slide_in_easing(Easing::parse("CubicEaseOut").expect("built-in easing"));
            transition.set_slide_out_easing(Easing::parse("CubicEaseOut").expect("built-in easing"));
            return Some(into_markup_value(transition));
        }
        "MacOSWindowTransparencyLevels" => return Some(into_markup_value(WindowTransparencyLevelCollection::new(
            if theme.effective_reduce_transparency() || contrast {
                vec![WindowTransparencyLevel::none()]
            } else {
                vec![WindowTransparencyLevel::acrylic_blur(), WindowTransparencyLevel::blur(), WindowTransparencyLevel::none()]
            }
        ))),
        "MacOSAnimationsEnabled" => return Some(into_markup_value(!theme.effective_reduce_motion())),
        "MacOSReduceMotion" => return Some(into_markup_value(theme.effective_reduce_motion())),
        // The fixed pulse period lives in MacOSMaterials.xaml and is resolved
        // eagerly. Retain the iteration resource for existing application users;
        // the theme's progress template gates the animation through its selector.
        "MacOSProgressPulseIterations" => return Some(into_markup_value(if theme.effective_reduce_motion() { IterationCount::new(0) } else { IterationCount::INFINITE })),
        "MacOSControlHeight" | "ButtonMinHeight" | "TextControlThemeMinHeight" | "ComboBoxMinHeight" | "SplitButtonMinHeight" => return Some(into_markup_value(size.height())),
        "MacOSControlFontSize" | "ControlContentThemeFontSize" | "ButtonFontSize" | "TextControlThemeFontSize" => return Some(into_markup_value(size.font_size())),
        "MacOSControlCornerRadius" | "ControlCornerRadius" | "ButtonCornerRadius" | "TextControlCornerRadius" | "ComboBoxCornerRadius" => return Some(into_markup_value(CornerRadius::uniform(size.corner_radius()))),
        "MacOSControlPadding" | "ButtonPadding" => {
            let x = match size { ControlSize::Mini => 6.0, ControlSize::Small => 8.0, ControlSize::Regular => 10.0, ControlSize::Large => 14.0, ControlSize::ExtraLarge => 18.0 };
            return Some(into_markup_value(Thickness::new(x, 2.0, x, 2.0)));
        }
        "MacOSFocusBrush" | "SystemControlFocusVisualPrimaryBrush" | "FocusStrokeColorOuterBrush" => {
            let a = accent(theme, variant);
            return Some(solid(Color::new(if contrast { 255 } else { 153 }, a.r, a.g, a.b)));
        }
        "SystemControlFocusVisualSecondaryBrush" | "FocusStrokeColorInnerBrush" => return Some(solid(Color::from_uint32(0))),
        "MacOSAccentBrush" => return Some(solid(accent(theme, variant))),
        "MacOSAccentColor" => return Some(into_markup_value(accent(theme, variant))),
        "MacOSFocusThickness" => return Some(into_markup_value(Thickness::uniform(if contrast { 3.0 } else { 2.0 }))),
        "MacOSGlassBorderBrush" => return Some(solid(Color::from_uint32(if contrast { if dark { 0xFFDDDDDD } else { 0xFF555555 } } else if dark { 0x55FFFFFF } else { 0x88FFFFFF }))),
        "MacOSTrafficLightCloseBrush" | "MacOSTrafficLightMinimizeBrush" | "MacOSTrafficLightZoomBrush" => {
            let value = if !active { if dark { 0xFF5A5A5A } else { 0xFFD0D0D0 } } else {
                match name { "MacOSTrafficLightCloseBrush" => 0xFFFF5F57, "MacOSTrafficLightMinimizeBrush" => 0xFFFEBC2E, _ => 0xFF28C840 }
            };
            return Some(solid(Color::from_uint32(value)));
        }
        "MacOSRegularGlassMaterial" | "MacOSSidebarGlassMaterial" | "MacOSMenuGlassMaterial" | "MacOSToolbarGlassMaterial" | "MacOSClearGlassMaterial" => {
            let opaque = theme.effective_reduce_transparency() || contrast || !active;
            let (light_tint, dark_tint, material_opacity) = match name {
                "MacOSSidebarGlassMaterial" => (0.72, 0.78, 0.70),
                "MacOSMenuGlassMaterial" => (0.86, 0.88, 0.72),
                "MacOSToolbarGlassMaterial" => (0.62, 0.70, 0.58),
                "MacOSClearGlassMaterial" => (0.10, 0.10, 0.18),
                _ => (0.68, 0.74, 0.62),
            };
            let tint = Color::from_uint32(if dark { 0xFF2C2C2E } else { 0xFFF5F5F7 });
            // Each surface owns its material: ExperimentalAcrylicBorder adjusts
            // platform transparency compensation per host, so sharing one mutable
            // material across windows would let one host alter another's surface.
            let material = ExperimentalAcrylicMaterial::new();
            material.set_tint_color(tint);
            material.set_fallback_color(tint);
            material.set_background_source(if opaque { AcrylicBackgroundSource::None } else { AcrylicBackgroundSource::Digger });
            material.set_tint_opacity(if opaque { 1.0 } else if dark { dark_tint } else { light_tint });
            material.set_material_opacity(if opaque { 1.0 } else { material_opacity });
            return Some(into_markup_value(material));
        }
        _ => {}
    }

    // Honor an application override without altering the shared system provider.
    // Every derived accent brush uses DynamicResource, so resetting to None also
    // restores the latest platform accent on controls that already exist.
    if name.starts_with("SystemAccentColor") {
        if let Some(value) = theme.accent_color() {
            let shades = crate::accents::SystemAccentColors::calculate_accent_shades(value);
            return Some(into_markup_value(match name {
                "SystemAccentColor" => value,
                "SystemAccentColorDark1" => shades.0, "SystemAccentColorDark2" => shades.1, "SystemAccentColorDark3" => shades.2,
                "SystemAccentColorLight1" => shades.3, "SystemAccentColorLight2" => shades.4, "SystemAccentColorLight3" => shades.5,
                _ => return None,
            }));
        }
    }

    // A brush realized from the inherited dark dictionary can otherwise retain
    // the dark background when its host requests the named contrast variant.
    // Resolve both forms coherently at the consuming host, including derived
    // contrast variants. IncreaseContrast alone keeps the chosen appearance.
    if high_contrast_variant && matches!(name, "SolidBackgroundFillColorBase" | "SolidBackgroundFillColorBaseBrush") {
        let styles: &Styles = theme;
        let background = styles.try_get_resource(&ResourceKey::from("SystemColorWindowColor"), variant)
            .and_then(|value| from_markup_value::<Color>(&value))
            .unwrap_or(Color::from_uint32(0xFF1C1C1E));
        return Some(if name.ends_with("Brush") { solid(background) } else { into_markup_value(background) });
    }

    if contrast {
        // The inherited high-contrast dictionary defines its system window color,
        // but the normal solid-background brush can already have been realized in
        // the dark dictionary. Resolve both public forms here so existing windows,
        // their opaque fallback and their system-bar color change together.
        let background = if dark { 0xFF1C1C1E } else { 0xFFFFFFFF };
        match name {
            "SolidBackgroundFillColorBase" => return Some(color(background)),
            "SolidBackgroundFillColorBaseBrush" => return Some(solid(Color::from_uint32(background))),
            _ => {}
        }
        let value = match name {
            "TextFillColorPrimary" => Some(if dark { 0xFFFFFFFF } else { 0xFF000000 }),
            "TextFillColorSecondary" => Some(if dark { 0xFFE5E5E5 } else { 0xFF262626 }),
            "TextFillColorTertiary" | "TextFillColorDisabled" => Some(if dark { 0xFFB0B0B0 } else { 0xFF595959 }),
            "ControlStrokeColorDefault" | "ControlStrokeColorSecondary" | "ControlStrongStrokeColorDefault" | "SurfaceStrokeColorDefault" | "SurfaceStrokeColorFlyout" | "DividerStrokeColorDefault" | "CardStrokeColorDefault" => Some(if dark { 0xFFCACACA } else { 0xFF606060 }),
            _ => None,
        };
        if let Some(value) = value { return Some(color(value)); }
    }

    if !active {
        // Inactive primary buttons lose accent prominence without changing their
        // default-action semantics. Preserve disabled resources and let the
        // existing disabled selectors win over hover and pressed states.
        let neutral = match name {
            "AccentButtonBackground" => Some("ButtonBackground"),
            "AccentButtonBackgroundPointerOver" => Some("ButtonBackgroundPointerOver"),
            "AccentButtonBackgroundPressed" => Some("ButtonBackgroundPressed"),
            "AccentButtonForeground" => Some("ButtonForeground"),
            "AccentButtonForegroundPointerOver" => Some("ButtonForegroundPointerOver"),
            "AccentButtonForegroundPressed" => Some("ButtonForegroundPressed"),
            "AccentButtonBorderBrush" => Some("ButtonBorderBrush"),
            "AccentButtonBorderBrushPointerOver" => Some("ButtonBorderBrushPointerOver"),
            "AccentButtonBorderBrushPressed" => Some("ButtonBorderBrushPressed"),
            _ => None,
        };
        if let Some(neutral) = neutral {
            let styles: &Styles = theme;
            return styles.try_get_resource(&ResourceKey::from(neutral), variant);
        }
        // Update foreground/background pairs for every selected interaction
        // state, not just the resting state; otherwise white hover text can
        // survive after its selection background has become pale gray.
        match name {
            "MacOSSelectionBrush"
            | "ListViewItemBackgroundSelected"
            | "ListViewItemBackgroundSelectedPointerOver"
            | "ListViewItemBackgroundSelectedPressed"
            | "TreeViewItemBackgroundSelected"
            | "TreeViewItemBackgroundSelectedPointerOver"
            | "TreeViewItemBackgroundSelectedPressed"
            | "TabularSurfaceRowBackgroundSelectedBrush"
            | "TabularSurfaceRowBackgroundSelectedPointerOverBrush"
            | "TabularSurfaceRowBackgroundSelectedPressedBrush" =>
                return Some(solid(Color::from_uint32(if dark { 0xFF555557 } else { 0xFFDCDCDD }))),
            "MacOSSelectionForegroundBrush"
            | "ListViewItemForegroundSelected"
            | "ListViewItemForegroundSelectedPointerOver"
            | "ListViewItemForegroundSelectedPressed"
            | "TreeViewItemForegroundSelected"
            | "TreeViewItemForegroundSelectedPointerOver"
            | "TreeViewItemForegroundSelectedPressed"
            | "TabularSurfaceRowForegroundSelectedBrush"
            | "TabularSurfaceRowForegroundSelectedPointerOverBrush"
            | "TabularSurfaceRowForegroundSelectedPressedBrush" =>
                return Some(solid(Color::from_uint32(if dark { 0xFFF2F2F2 } else { 0xFF262626 }))),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
#[path = "tests/recovery_regressions.rs"]
mod recovery_regressions;

#[cfg(test)]
#[path = "tests/theme_review_regressions.rs"]
mod theme_review_regressions;
