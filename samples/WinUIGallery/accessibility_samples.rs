//! The code behind of the pinned AccessibilityKeyboard page in Rust: the handlers of
//! `AccessibilityKeyboardPage.xaml.cs`, which color the rectangle of the accelerators example.
//! The tab order, arrow keys and access keys of the page are the markup's. The
//! AccessibilityScreenReader page has no code-behind upstream: the images of its examples are
//! set here (the markup has no bitmap converter for the gallery's assets). The
//! AccessibilityColorContrast page's code behind and its two InlineColorPickers are here too; the
//! calculation is in `contrast.rs`.

use ferroui_base::media::immutable::ImmutableSolidColorBrush;
use ferroui_base::media::{Color, Colors, IBrush};
use ferroui_base::Ref;
use ferroui_controls::shapes::{Ellipse, Rectangle};
use ferroui_controls::{Button, Control, FontIcon, Image, SplitButton, TextBlock, TextBox};
use ferroui_controls_color_picker::ColorView;
use std::rc::Rc;

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    match page_id {
        "AccessibilityKeyboard" => wire_keyboard(root),
        "AccessibilityScreenReader" => wire_screen_reader(root),
        "AccessibilityColorContrast" => wire_color_contrast(root),
        _ => {}
    }
}

/// The buttons of `MakeRedButton_Click`, `MakeBlueButton_Click` and `MakeChartreuseButton_Click`
/// with the color each sets.
pub(crate) const ACCELERATOR_COLORS: [(&str, Color); 3] = [
    ("MakeRedButton", Colors::RED),
    ("MakeBlueButton", Colors::BLUE),
    ("MakeChartreuseButton", Colors::CHARTREUSE),
];

fn wire_keyboard(root: &Ref<Control>) {
    let rectangle = root.get_control::<Rectangle>("ColorRectangle");
    for (name, color) in ACCELERATOR_COLORS {
        let weak_rectangle = rectangle.downgrade();
        root.get_control::<Button>(name).click(move |_, _| {
            if let Some(rectangle) = weak_rectangle.upgrade() {
                rectangle.set_fill(Some(Rc::new(ImmutableSolidColorBrush::new(color)) as Rc<dyn IBrush>));
            }
        });
    }
}

/// The images of the AccessibilityScreenReader page: the source's grapes.jpg, named Grapes, and
/// the decorative treetops.jpg in the Raw view.
pub(crate) const SCREEN_READER_IMAGES: [(&str, &str); 2] = [
    ("GrapesImage", "ferres://WinUIGallery/Assets/SampleMedia/grapes.jpg"),
    ("TreetopsImage", "ferres://WinUIGallery/Assets/SampleMedia/treetops.jpg"),
];

fn wire_screen_reader(root: &Ref<Control>) {
    for (name, uri) in SCREEN_READER_IMAGES {
        if let Ok(bitmap) = crate::assets::load_bitmap(uri) {
            root.get_control::<Image>(name).set_source(Some(Rc::new(bitmap)));
        }
    }
}

// --- AccessibilityColorContrast -----------------------------------------------------------------

fn solid(color: Color) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::new(color))
}

/// `Color.ToString().Replace("#FF", "#")` of an opaque color: `#RRGGBB`.
pub(crate) fn hex(color: Color) -> String {
    format!("#{:02X}{:02X}{:02X}", color.r, color.g, color.b)
}

/// The ratio of `RecalculateContrastRatio`: `Math.Round(ratio, 2).ToString() + ":1"`.
pub(crate) fn ratio_text(ratio: f64) -> String {
    format!("{}:1", (ratio * 100.0).round() / 100.0)
}

/// The color of the InlineColorPicker `prefix` (Text or Background): the fill of its preview.
pub(crate) fn picker_color(root: &Ref<Control>, prefix: &str) -> Option<Color> {
    root.get_control::<Rectangle>(&format!("{prefix}ColorPreview"))
        .fill()
        .and_then(|brush| brush.as_solid_color_brush().map(|brush| brush.color()))
}

/// `SetCheckState`: the check's ellipse, glyph and result for `passed`.
fn set_check_state(root: &Ref<Control>, name: &str, passed: bool) {
    let (color, glyph, result) = if passed {
        (Colors::DARK_GREEN, "\u{E73E}", "Pass")
    } else {
        (Colors::DARK_RED, "\u{E711}", "Fail")
    };
    root.get_control::<Ellipse>(&format!("{name}CheckEllipse")).set_fill(Some(solid(color)));
    root.get_control::<FontIcon>(&format!("{name}CheckIcon")).set_glyph(glyph);
    root.get_control::<TextBlock>(&format!("{name}CheckResult")).set_text(Some(result));
}

/// `RecalculateContrastRatio`, and the x:Bind of the pickers' ColorBrush to the preview.
fn recalculate_contrast_ratio(root: &Ref<Control>) {
    let (Some(text), Some(background)) = (picker_color(root, "Text"), picker_color(root, "Background")) else {
        return;
    };
    let result = crate::contrast::ContrastResult::calculate(text, background);
    root.get_control::<TextBlock>("ContrastRatioPresenter")
        .set_text(Some(&ratio_text(result.ratio)));
    set_check_state(root, "NormalText", result.normal_text);
    set_check_state(root, "LargeText", result.large_text);
    set_check_state(root, "Components", result.components);
    root.get_control::<ferroui_controls::Grid>("ContrastPreview")
        .set_background(Some(solid(background)));
    for name in ["PreviewText", "PreviewBoldText", "PreviewLargeText"] {
        root.get_control::<TextBlock>(name).set_foreground(Some(solid(text)));
    }
    for name in ["PreviewCheckBox", "PreviewToggle"] {
        root.get_control::<Rectangle>(name).set_fill(Some(solid(text)));
    }
    root.get_control::<FontIcon>("PreviewIcon").set_foreground(Some(solid(text)));
}

/// The gallery's InlineColorPicker `prefix` with `initial` as its Color: `Picker_ColorChanged`
/// (the preview, the hex box and ColorChanged), `PickerFlyout_Opened` (the picker starts at the
/// preview's color) and `ColorHex_TextChanged` (a six-digit hex sets the color; other text is
/// ignored, as the source's caught exception).
fn wire_inline_color_picker(root: &Ref<Control>, prefix: &'static str, initial: Color) {
    let preview = root.get_control::<Rectangle>(&format!("{prefix}ColorPreview"));
    let hex_box = root.get_control::<TextBox>(&format!("{prefix}ColorHex"));
    let view = root.get_control::<ColorView>(&format!("{prefix}ColorView"));
    preview.set_fill(Some(solid(initial)));
    view.set_color(initial);
    let weak_root = root.downgrade();
    view.color_changed(move |args| {
        if let Some(root) = weak_root.upgrade() {
            let color = args.new_color();
            root.get_control::<Rectangle>(&format!("{prefix}ColorPreview")).set_fill(Some(solid(color)));
            root.get_control::<TextBox>(&format!("{prefix}ColorHex")).set_text(Some(&hex(color)));
            recalculate_contrast_ratio(&root);
        }
    });
    if let Some(flyout) = root.get_control::<SplitButton>(&format!("{prefix}ColorButton")).flyout() {
        let weak_root = root.downgrade();
        flyout
            .opened(move || {
                if let Some(root) = weak_root.upgrade() {
                    if let Some(color) = picker_color(&root, prefix) {
                        root.get_control::<ColorView>(&format!("{prefix}ColorView")).set_color(color);
                    }
                }
            })
            .detach();
    }
    let weak_root = root.downgrade();
    let weak_box = hex_box.downgrade();
    hex_box.property_changed(move |change| {
        if change.property() != TextBox::text_property().as_property() {
            return;
        }
        let (Some(root), Some(hex_box)) = (weak_root.upgrade(), weak_box.upgrade()) else {
            return;
        };
        if let Some(color) = crate::contrast::parse_rgb(&hex_box.text().unwrap_or_default()) {
            root.get_control::<Rectangle>(&format!("{prefix}ColorPreview")).set_fill(Some(solid(color)));
            recalculate_contrast_ratio(&root);
        }
    });
    // InlineColorPicker_Loaded writes the color's hex.
    hex_box.set_text(Some(&hex(initial)));
}

/// The AccessibilityColorContrast page: the text picker starts Black, the background picker at
/// the InlineColorPicker's default, White.
fn wire_color_contrast(root: &Ref<Control>) {
    wire_inline_color_picker(root, "Text", Colors::BLACK);
    wire_inline_color_picker(root, "Background", Colors::WHITE);
    recalculate_contrast_ratio(root);
}
