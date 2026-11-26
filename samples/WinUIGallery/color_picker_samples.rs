//! The pinned Gallery ColorPicker page with the inline ColorPicker (#1266).
//!
//! WinUI's ColorPicker is FerroUI's `ColorPicker` with `DisplayMode="Inline"`: the
//! spectrum, the sliders, the More button and the text inputs are the control's own.
//! The page's code wires the source's `x:Bind` expressions (the option check boxes, the
//! Rectangle preview) and its `ColorSpectrumShapeRadioButtons_SelectionChanged` handler
//! with weak callbacks.

use ferroui_base::media::{Color, SolidColorBrush};
use ferroui_base::Ref;
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{CheckBox, Control, RadioButtons};
use ferroui_controls_color_picker::{ColorPicker, ColorSpectrumShape};

/// The option check boxes of the page, by name.
const OPTIONS: [&str; 7] =
    ["moreBtn", "colorSlider", "colorChannelInput", "hexInput", "alpha", "alphaSlider", "alphaTextInput"];

pub(crate) fn wire(root: &Ref<Control>) {
    let Some(picker) = root.find_control::<Control>("colorPicker").and_then(|control| control.cast::<ColorPicker>())
    else {
        return;
    };

    // `<SolidColorBrush Color="{x:Bind colorPicker.Color, Mode=OneWay}" />`
    let weak_root = root.downgrade();
    picker.color_changed(move |args| {
        if let Some(root) = weak_root.upgrade() {
            show_color(&root, args.new_color());
        }
    });

    for name in OPTIONS {
        let option = root.get_control::<CheckBox>(name);
        let weak_root = root.downgrade();
        option.property_changed(move |change| {
            if change.property() == ToggleButton::is_checked_property().as_property() {
                if let Some(root) = weak_root.upgrade() {
                    update_options(&root);
                }
            }
        });
    }

    // ColorSpectrumShapeRadioButtons_SelectionChanged
    let shape = root.get_control::<RadioButtons>("ColorSpectrumShapeRadioButtons");
    let weak_shape = shape.downgrade();
    let weak_picker = picker.downgrade();
    shape.selection_changed(move |_, _| {
        if let (Some(shape), Some(picker)) = (weak_shape.upgrade(), weak_picker.upgrade()) {
            picker.set_color_spectrum_shape(if shape.selected_index() == 0 {
                ColorSpectrumShape::Box
            } else {
                ColorSpectrumShape::Ring
            });
        }
    });

    update_options(root);
    show_color(root, picker.color());
}

fn checked(root: &Ref<Control>, name: &str) -> bool {
    root.get_control::<CheckBox>(name).is_checked() == Some(true)
}

/// The source's `x:Bind` of each option: `IsColorSliderVisible` is `IsColorSpectrumSliderVisible`
/// and `IsColorChannelTextInputVisible` is `IsComponentTextInputVisible`. WinUI's
/// `IsAlphaEnabled` hides the alpha editors and keeps `Color.A`, which is `IsAlphaVisible`
/// here: the picker keeps alpha enabled, because a disabled alpha coerces `Color.A` to 255.
fn update_options(root: &Ref<Control>) {
    let picker = root.get_control::<ColorPicker>("colorPicker");
    let alpha = checked(root, "alpha");
    picker.set_is_more_button_visible(checked(root, "moreBtn"));
    picker.set_is_color_spectrum_slider_visible(checked(root, "colorSlider"));
    picker.set_is_component_text_input_visible(checked(root, "colorChannelInput"));
    picker.set_is_hex_input_visible(checked(root, "hexInput"));
    picker.set_is_alpha_visible(alpha);
    picker.set_is_alpha_slider_visible(checked(root, "alphaSlider"));
    picker.set_is_alpha_text_input_visible(checked(root, "alphaTextInput"));
    // `IsEnabled="{x:Bind alpha.IsChecked.Value, Mode=OneWay}"`
    for name in ["alphaSlider", "alphaTextInput"] {
        root.get_control::<CheckBox>(name).set_is_enabled(alpha);
    }
}

fn show_color(root: &Ref<Control>, color: Color) {
    if let Some(rectangle) = root.find_control::<Rectangle>("previewRect") {
        rectangle.set_fill(Some(SolidColorBrush::with_color(color).into()));
    }
}
