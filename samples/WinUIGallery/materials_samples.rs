//! The ThemeShadow and Acrylic examples of the pinned WinUI Gallery and their C# handlers in
//! Rust. The shadow and the acrylic material are the framework's (`ferroui_base::media`); the
//! sample owns only what its page's code-behind does.

use ferroui_base::controls::ResourceKey;
use ferroui_base::media::{AcrylicBrush, Color};
use ferroui_base::{BoxedValue, ObjectType, Ref, Vector3D};
use ferroui_controls::primitives::{RangeBase, SelectingItemsControl};
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{Border, ComboBox, Control, Grid, Slider, TextBlock};
use std::rc::Rc;

pub(crate) fn wire(root: &Ref<Control>) {
    wire_theme_shadow(root);
    wire_acrylic(root);
}

fn find<T: ObjectType>(root: &Ref<Control>, name: &str) -> Option<Ref<T>> {
    root.find_control::<Control>(name).and_then(|control| control.cast())
}

/// `ShadowRect_Loaded` adds the receiver; `TranslationSliderInApp_ValueChanged` sets the Z of the
/// Border's `Translation`, the elevation the shadow is drawn at.
fn wire_theme_shadow(root: &Ref<Control>) {
    let Some(rect) = find::<Border>(root, "ShadowRect") else {
        return;
    };
    let receiver = root.get_control::<Grid>("ShadowCastGrid");
    if let Some(shadow) = rect.shadow() {
        if let Some(theme_shadow) = shadow.as_theme_shadow() {
            // WinUI 3 keeps the receivers (projected shadows are off, CThemeShadow::IsDropShadowMode);
            // the caster is not an ancestor of the grid, so the append cannot fail.
            let _ = theme_shadow.receivers().append(&receiver);
        }
    }
    let slider = root.get_control::<Slider>("TranslationSliderInApp");
    let output = root.get_control::<TextBlock>("ShadowTranslationOutput");
    let apply = {
        let rect = rect.downgrade();
        let output = output.downgrade();
        move |z: f64| {
            if let Some(rect) = rect.upgrade() {
                rect.set_translation(Vector3D::new(0.0, 0.0, z));
            }
            if let Some(output) = output.upgrade() {
                output.set_text(Some(&format!("Translation.Z: {z}")));
            }
        }
    };
    // The slider's Value="32" raises ValueChanged while the source page loads.
    apply(slider.value());
    let weak_slider = slider.downgrade();
    slider.property_changed(move |change| {
        if change.property() != RangeBase::value_property().as_property() {
            return;
        }
        if let Some(slider) = weak_slider.upgrade() {
            apply(slider.value());
        }
    });
}

/// The tint colours of `ColorSelectorInApp` (its SolidColorBrush items: Black, Red, Blue).
pub(crate) const TINT_COLORS: [Color; 3] = [
    Color::from_rgb(0x00, 0x00, 0x00),
    Color::from_rgb(0xFF, 0x00, 0x00),
    Color::from_rgb(0x00, 0x00, 0xFF),
];

/// The fallback colours of `FallbackColorSelectorInApp` (Green, Yellow).
pub(crate) const FALLBACK_COLORS: [Color; 2] = [
    Color::from_rgb(0x00, 0x80, 0x00),
    Color::from_rgb(0xFF, 0xFF, 0x00),
];

/// The keys of the page's two AcrylicBrush resources.
pub(crate) const IN_APP_BRUSH_KEY: &str = "CustomAcrylicInAppBrush";
pub(crate) const LUMINOSITY_BRUSH_KEY: &str = "CustomAcrylicInAppLuminosity";

/// The page's `CustomAcrylicInAppBrush` resource.
pub(crate) fn custom_in_app_brush() -> Ref<AcrylicBrush> {
    let brush = AcrylicBrush::new();
    brush.set_fallback_color(FALLBACK_COLORS[0]);
    brush.set_tint_color(TINT_COLORS[0]);
    brush.set_tint_opacity(0.8);
    brush
}

/// The page's `CustomAcrylicInAppLuminosity` resource (SkyBlue fallback and tint).
pub(crate) fn custom_luminosity_brush() -> Ref<AcrylicBrush> {
    let sky_blue = Color::from_rgb(0x87, 0xCE, 0xEB);
    let brush = AcrylicBrush::new();
    brush.set_fallback_color(sky_blue);
    brush.set_tint_color(sky_blue);
    brush.set_tint_opacity(0.8);
    brush
}

fn on_value(slider: &Ref<Slider>, handler: impl Fn(f64) + 'static) {
    let weak_slider = slider.downgrade();
    slider.property_changed(move |change| {
        if change.property() != RangeBase::value_property().as_property() {
            return;
        }
        if let Some(slider) = weak_slider.upgrade() {
            handler(slider.value());
        }
    });
}

fn on_selection(combo: &Ref<ComboBox>, handler: impl Fn(i32) + 'static) {
    let weak_combo = combo.downgrade();
    combo.property_changed(move |change| {
        if change.property() != SelectingItemsControl::selected_index_property().as_property() {
            return;
        }
        if let Some(combo) = weak_combo.upgrade() {
            handler(combo.selected_index());
        }
    });
}

/// The two custom brushes are the fills of their rectangles, as the resources are in the source;
/// the handlers change the brush the rectangle is filled with (`(AcrylicBrush)shape.Fill`).
fn wire_acrylic(root: &Ref<Control>) {
    let Some(in_app_shape) = find::<Rectangle>(root, "CustomAcrylicShapeInApp") else {
        return;
    };
    let lumin_shape = root.get_control::<Rectangle>("CustomAcrylicShapeLumin");
    let in_app = custom_in_app_brush();
    let lumin = custom_luminosity_brush();
    in_app_shape.set_fill(Some(in_app.clone().into()));
    lumin_shape.set_fill(Some(lumin.clone().into()));
    // The page's resources, under the source's keys.
    for (key, brush) in [(IN_APP_BRUSH_KEY, &in_app), (LUMINOSITY_BRUSH_KEY, &lumin)] {
        root.resources().add(ResourceKey::from(key), Some(Rc::new(brush.clone()) as BoxedValue));
    }

    let opacity_in_app = root.get_control::<Slider>("OpacitySliderInApp");
    let opacity_lumin = root.get_control::<Slider>("OpacitySliderLumin");
    let luminosity = root.get_control::<Slider>("LuminositySlider");
    let tint = root.get_control::<ComboBox>("ColorSelectorInApp");
    let fallback = root.get_control::<ComboBox>("FallbackColorSelectorInApp");

    // Slider_ValueChanged: the slider of each example sets the tint opacity of its brush.
    for (slider, brush) in [(&opacity_in_app, in_app.downgrade()), (&opacity_lumin, lumin.downgrade())] {
        on_value(slider, move |value| {
            if let Some(brush) = brush.upgrade() {
                brush.set_tint_opacity(value);
            }
        });
    }
    // LuminositySlider_ValueChanged.
    let weak_lumin = lumin.downgrade();
    on_value(&luminosity, move |value| {
        if let Some(brush) = weak_lumin.upgrade() {
            brush.set_tint_luminosity_opacity(Some(value));
        }
    });
    // ColorSelector_SelectionChanged and FallbackColorSelector_SelectionChanged.
    let weak_in_app = in_app.downgrade();
    on_selection(&tint, move |index| {
        let color = usize::try_from(index).ok().and_then(|index| TINT_COLORS.get(index));
        if let (Some(brush), Some(color)) = (weak_in_app.upgrade(), color) {
            brush.set_tint_color(*color);
        }
    });
    let weak_in_app = in_app.downgrade();
    on_selection(&fallback, move |index| {
        let color = usize::try_from(index).ok().and_then(|index| FALLBACK_COLORS.get(index));
        if let (Some(brush), Some(color)) = (weak_in_app.upgrade(), color) {
            brush.set_fallback_color(*color);
        }
    });

    // AcrylicPage_Loaded.
    tint.set_selected_index(0);
    fallback.set_selected_index(0);
    opacity_in_app.set_range_value(0.8);
    opacity_lumin.set_range_value(0.8);
    luminosity.set_range_value(0.8);
}
