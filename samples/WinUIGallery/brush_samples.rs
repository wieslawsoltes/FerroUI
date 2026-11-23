//! The RadialGradientBrush, IconElement, Spacing and Typography pages of the pinned WinUI Gallery in
//! Rust: the code behind of `RadialGradientBrushPage.xaml.cs`, `IconElementPage.xaml.cs` and
//! `TypographyPage.xaml.cs` with the copy buttons of the gallery's `TypographyControl`, and the
//! theme images of the Spacing and Typography pages.
//!
//! FerroUI's RadialGradientBrush has no `MappingMode`: each point and radius has a unit, Relative
//! for RelativeToBoundingBox and Absolute for Absolute. FerroUI has no BitmapIcon: the bitmap icon
//! is an Image of the source's bitmap, and its `ShowAsMonochrome` a rectangle of the foreground
//! brush masked by the bitmap (the icon drawn in one color, as WinUI's monochrome bitmap icon).

use ferroui_base::media::imaging::Bitmap;
use ferroui_base::media::{GradientSpreadMethod, IImage, IImageBrushSource, ImageBrush, RadialGradientBrush, Stretch};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{Ref, RelativePoint, RelativeScalar, RelativeUnit};
use ferroui_controls::primitives::{SelectingItemsControl, ToggleButton};
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{ComboBox, Control, Image, Slider};
use std::rc::Rc;

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    match page_id {
        "RadialGradientBrush" => wire_radial_gradient_brush(root),
        "IconElement" => wire_icon_element(root),
        "Spacing" => wire_spacing(root),
        "Typography" => wire_typography(root),
        _ => {}
    }
}

fn load_bitmap(uri: &str) -> Option<Rc<Bitmap>> {
    crate::assets::load_bitmap(uri).ok().map(Rc::new)
}

// --- RadialGradientBrush ----------------------------------------------------------------------

/// The brush of the example: `RadialGradientBrushExample`, the fill of `Rect`.
pub(crate) fn radial_brush(root: &Ref<Control>) -> Option<Ref<RadialGradientBrush>> {
    let fill = root.get_control::<Rectangle>("Rect").fill()?;
    fill.as_object()
        .and_then(|object| object.to_ref().cast::<RadialGradientBrush>())
}

const CENTER_X: [&str; 3] = ["CenterXSlider", "RadiusXSlider", "OriginXSlider"];
const CENTER_Y: [&str; 3] = ["CenterYSlider", "RadiusYSlider", "OriginYSlider"];

/// The unit of the brush's points and radii: `MappingMode`.
fn mapping_unit(root: &Ref<Control>) -> RelativeUnit {
    if root.get_control::<ComboBox>("MappingModeComboBox").selected_index() == 1 {
        RelativeUnit::Absolute
    } else {
        RelativeUnit::Relative
    }
}

/// `InitializeSliders`: the sliders' ranges, values and steps of the mapping mode (in pixels of
/// the rectangle for Absolute, fractions of it otherwise).
fn initialize_sliders(root: &Ref<Control>) {
    let rect = root.get_control::<Rectangle>("Rect");
    let (width, height) = (rect.width(), rect.height());
    let absolute = mapping_unit(root) == RelativeUnit::Absolute;
    for (names, step) in [(CENTER_X, width / 50.0), (CENTER_Y, height / 50.0)] {
        for name in names {
            let slider = root.get_control::<Slider>(name);
            // The source's Y sliders take the width for their range and value, as it does.
            let (maximum, value, step, small) = if absolute {
                (width, width / 2.0, step, 10.0)
            } else {
                (1.0, 0.5, 0.02, 0.05)
            };
            slider.set_maximum(maximum);
            slider.set_tick_frequency(step);
            slider.set_is_snap_to_tick_enabled(true);
            slider.set_small_change(small);
            slider.set_range_value(value);
        }
    }
    on_slider_value_changed(root);
}

/// `OnSliderValueChanged`: the brush's center, radii and gradient origin from the sliders.
fn on_slider_value_changed(root: &Ref<Control>) {
    let Some(brush) = radial_brush(root) else {
        return;
    };
    let unit = mapping_unit(root);
    let value = |name: &str| root.get_control::<Slider>(name).value();
    brush.set_center(RelativePoint::new(value("CenterXSlider"), value("CenterYSlider"), unit));
    brush.set_radius_x(RelativeScalar::new(value("RadiusXSlider"), unit));
    brush.set_radius_y(RelativeScalar::new(value("RadiusYSlider"), unit));
    brush.set_gradient_origin(RelativePoint::new(value("OriginXSlider"), value("OriginYSlider"), unit));
}

/// The spread methods of the SpreadMethod box, in its order.
pub(crate) const SPREAD_METHODS: [GradientSpreadMethod; 3] = [
    GradientSpreadMethod::Pad,
    GradientSpreadMethod::Reflect,
    GradientSpreadMethod::Repeat,
];

fn on_selection(root: &Ref<Control>, name: &str, handler: fn(&Ref<Control>, i32)) {
    let weak_root = root.downgrade();
    let weak_combo_box = root.get_control::<ComboBox>(name).downgrade();
    root.get_control::<ComboBox>(name).property_changed(move |change| {
        if change.property() != SelectingItemsControl::selected_index_property().as_property() {
            return;
        }
        if let (Some(root), Some(combo_box)) = (weak_root.upgrade(), weak_combo_box.upgrade()) {
            handler(&root, combo_box.selected_index());
        }
    });
}

/// `OnPageLoaded`: the handlers of the two boxes and the sliders, then `InitializeSliders`.
fn wire_radial_gradient_brush(root: &Ref<Control>) {
    on_selection(root, "MappingModeComboBox", |root, _| initialize_sliders(root));
    on_selection(root, "SpreadMethodComboBox", |root, index| {
        if let (Some(brush), Some(method)) = (
            radial_brush(root),
            usize::try_from(index).ok().and_then(|index| SPREAD_METHODS.get(index)),
        ) {
            brush.set_spread_method(*method);
        }
    });
    for name in CENTER_X.into_iter().chain(CENTER_Y) {
        let weak_root = root.downgrade();
        root.get_control::<Slider>(name).value_changed(move |_, _| {
            if let Some(root) = weak_root.upgrade() {
                on_slider_value_changed(&root);
            }
        });
    }
    initialize_sliders(root);
}

// --- IconElement -------------------------------------------------------------------------------

const SLICES: &str = "ferres://WinUIGallery/Assets/SampleMedia/Slices.png";

/// `MonochromeButton_CheckedChanged`: `ShowAsMonochrome` of the bitmap icon, which shows the
/// bitmap again.
fn show_as_monochrome(root: &Ref<Control>, monochrome: bool) {
    let bitmap = load_bitmap(SLICES);
    let image = root.get_control::<Image>("SlicesImage");
    image.set_source(bitmap.clone().map(|bitmap| bitmap as Rc<dyn IImage>));
    // The image keeps the icon's size; the monochrome icon is drawn over it.
    image.set_opacity(if monochrome { 0.0 } else { 1.0 });
    let mask = root.get_control::<Rectangle>("SlicesMonochrome");
    let brush = ImageBrush::new();
    brush.set_stretch(Stretch::Uniform);
    brush.set_source(bitmap.map(|bitmap| bitmap as Rc<dyn IImageBrushSource>));
    mask.set_opacity_mask(Some(brush.into()));
    mask.set_is_visible(monochrome);
}

/// Whether the bitmap icon of the IconElement page is drawn in one color.
pub(crate) fn is_monochrome(root: &Ref<Control>) -> bool {
    root.get_control::<Rectangle>("SlicesMonochrome").is_visible()
}

fn wire_icon_element(root: &Ref<Control>) {
    // The markup's ShowAsMonochrome="False".
    show_as_monochrome(root, false);
    let check = root.get_control::<ToggleButton>("MonochromeButton");
    let weak_check = check.downgrade();
    let weak_root = root.downgrade();
    check.property_changed(move |change| {
        if change.property() != ToggleButton::is_checked_property().as_property() {
            return;
        }
        if let (Some(check), Some(root)) = (weak_check.upgrade(), weak_root.upgrade()) {
            show_as_monochrome(&root, check.is_checked() == Some(true));
        }
    });
    // The ImageIcon with the source's PNG.
    root.get_control::<Image>("ImageExample1Icon")
        .set_source(load_bitmap(SLICES).map(|bitmap| bitmap as Rc<dyn IImage>));
}

// --- Spacing -----------------------------------------------------------------------------------

/// An image of the page's theme dictionaries: `<name>.dark.png` in the Default (dark) theme and
/// `<name>.light.png` in the Light theme.
pub(crate) fn design_image_uri(name: &str, theme: Option<&ThemeVariant>) -> String {
    let light = theme
        .is_some_and(|theme| *theme == ThemeVariant::light() || theme.inherit_variant() == Some(ThemeVariant::light()));
    let variant = if light { "light" } else { "dark" };
    format!("ferres://WinUIGallery/Assets/Design/{name}.{variant}.png")
}

/// `CardsImage` and `DialogImage` of the theme (the gallery's SampleThemeListener).
fn update_spacing_images(root: &Ref<Control>) {
    let theme = root.actual_theme_variant();
    for (image, name) in [("CardsImage", "Cards"), ("DialogImage", "Dialog")] {
        root.get_control::<Image>(image)
            .set_source(load_bitmap(&design_image_uri(name, theme.as_ref())).map(|bitmap| bitmap as Rc<dyn IImage>));
    }
}

fn wire_spacing(root: &Ref<Control>) {
    update_spacing_images(root);
    let weak_root = root.downgrade();
    root.actual_theme_variant_changed(move || {
        if let Some(root) = weak_root.upgrade() {
            update_spacing_images(&root);
        }
    })
    .detach();
}

// --- Typography --------------------------------------------------------------------------------

/// The rows of the type ramp, the source's TypographyControls: the example, the resource name of
/// its style (a ControlTheme of the WinUI theme), the size and line height, and the variable font.
pub(crate) const TYPE_RAMP: [(&str, &str, &str, &str); 9] = [
    ("Caption", "CaptionTextBlockStyle", "12/16 epx", "Small, Regular"),
    ("Body", "BodyTextBlockStyle", "14/20 epx", "Text, Regular"),
    ("Body Strong", "BodyStrongTextBlockStyle", "14/20 epx", "Text, SemiBold"),
    ("Body Large", "BodyLargeTextBlockStyle", "18/24 epx", "Text, Regular"),
    ("Body Large Strong", "BodyLargeStrongTextBlockStyle", "18/24 epx", "Text, SemiBold"),
    ("Subtitle", "SubtitleTextBlockStyle", "20/28 epx", "Display, SemiBold"),
    ("Title", "TitleTextBlockStyle", "28/36 epx", "Display, SemiBold"),
    ("Title Large", "TitleLargeTextBlockStyle", "40/52 epx", "Display, SemiBold"),
    ("Display", "DisplayTextBlockStyle", "68/92 epx", "Display, SemiBold"),
];

/// The copy buttons of the type ramp: `CopyToClipboardButton_Click` of each TypographyControl puts
/// its ResourceName on the clipboard.
const TYPE_RAMP_COPY_BUTTONS: [(&str, &str); 9] = [
    ("CopyCaptionTextBlockStyle", "CaptionTextBlockStyle"),
    ("CopyBodyTextBlockStyle", "BodyTextBlockStyle"),
    ("CopyBodyStrongTextBlockStyle", "BodyStrongTextBlockStyle"),
    ("CopyBodyLargeTextBlockStyle", "BodyLargeTextBlockStyle"),
    ("CopyBodyLargeStrongTextBlockStyle", "BodyLargeStrongTextBlockStyle"),
    ("CopySubtitleTextBlockStyle", "SubtitleTextBlockStyle"),
    ("CopyTitleTextBlockStyle", "TitleTextBlockStyle"),
    ("CopyTitleLargeTextBlockStyle", "TitleLargeTextBlockStyle"),
    ("CopyDisplayTextBlockStyle", "DisplayTextBlockStyle"),
];

/// `HeaderImage` of the theme dictionaries (the gallery's SampleThemeListener).
fn update_typography_image(root: &Ref<Control>) {
    let theme = root.actual_theme_variant();
    root.get_control::<Image>("HeaderImage").set_source(
        load_bitmap(&design_image_uri("Typography", theme.as_ref())).map(|bitmap| bitmap as Rc<dyn IImage>),
    );
}

/// The code behind of `TypographyPage.xaml.cs`: the header image of the theme, and the copy
/// buttons of the rows. The ShowTypographyButtonClick handlers toggle the tips, which are the
/// Flyouts of their buttons (FerroUI has no TeachingTip): the button's click opens and closes it.
fn wire_typography(root: &Ref<Control>) {
    update_typography_image(root);
    let weak_root = root.downgrade();
    root.actual_theme_variant_changed(move || {
        if let Some(root) = weak_root.upgrade() {
            update_typography_image(&root);
        }
    })
    .detach();
    crate::shape_samples::wire_copy_buttons(root, &TYPE_RAMP_COPY_BUTTONS);
}
