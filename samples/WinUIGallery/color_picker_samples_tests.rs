//! The original ColorPicker page with the inline ColorPicker (#1266): its options, the
//! selection handler of the spectrum shape and the Rectangle preview.

use crate::{examples, tests::start};
use ferroui_base::media::{Color, Colors};
use ferroui_base::Ref;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{CheckBox, Control, RadioButtons};
use ferroui_controls_color_picker::{AlphaComponentPosition, ColorPicker, ColorPickerDisplayMode, ColorSpectrumShape};

fn page() -> Ref<Control> {
    examples::create(examples::find("ColorPicker").unwrap()).unwrap()
}

fn option(root: &Ref<Control>, name: &str, value: bool) {
    root.get_control::<CheckBox>(name).set_is_checked(Some(value));
}

fn preview(root: &Ref<Control>) -> Color {
    root.get_control::<Rectangle>("previewRect").fill().unwrap().as_solid_color_brush().unwrap().color()
}

#[test]
fn color_picker_page_preserves_source_option_defaults_with_the_inline_picker() {
    let _application = start();
    let root = page();
    let picker = root.get_control::<ColorPicker>("colorPicker");
    assert_eq!(picker.display_mode(), ColorPickerDisplayMode::Inline);
    assert_eq!(picker.color(), Colors::WHITE);
    // WinUI's hex input is #AARRGGBB.
    assert_eq!(picker.hex_input_alpha_position(), AlphaComponentPosition::Leading);
    assert!(!picker.is_more_button_visible());
    assert!(picker.is_color_spectrum_slider_visible());
    assert!(picker.is_component_text_input_visible());
    assert!(picker.is_hex_input_visible());
    assert!(picker.is_alpha_enabled(), "alpha stays enabled so that Color.A is kept");
    assert!(!picker.is_alpha_visible(), "the source's Alpha Enabled option starts unchecked");
    assert!(picker.is_alpha_slider_visible());
    assert!(picker.is_alpha_text_input_visible());
    assert_eq!(picker.color_spectrum_shape(), ColorSpectrumShape::Box);
    for name in ["alphaSlider", "alphaTextInput"] {
        assert!(!root.get_control::<CheckBox>(name).is_enabled(), "{name} follows the alpha option");
    }
    assert_eq!(preview(&root), Colors::WHITE);
}

#[test]
fn color_picker_page_maps_every_option_and_the_shape_handler() {
    let _application = start();
    let root = page();
    let picker = root.get_control::<ColorPicker>("colorPicker");

    option(&root, "moreBtn", true);
    assert!(picker.is_more_button_visible());
    option(&root, "colorSlider", false);
    assert!(!picker.is_color_spectrum_slider_visible());
    option(&root, "colorChannelInput", false);
    assert!(!picker.is_component_text_input_visible());
    option(&root, "hexInput", false);
    assert!(!picker.is_hex_input_visible());
    option(&root, "alpha", true);
    assert!(picker.is_alpha_visible());
    for name in ["alphaSlider", "alphaTextInput"] {
        assert!(root.get_control::<CheckBox>(name).is_enabled());
    }
    option(&root, "alphaSlider", false);
    assert!(!picker.is_alpha_slider_visible());
    option(&root, "alphaTextInput", false);
    assert!(!picker.is_alpha_text_input_visible());

    let shape = root.get_control::<RadioButtons>("ColorSpectrumShapeRadioButtons");
    shape.set_selected_index(1);
    assert_eq!(picker.color_spectrum_shape(), ColorSpectrumShape::Ring);
    shape.set_selected_index(0);
    assert_eq!(picker.color_spectrum_shape(), ColorSpectrumShape::Box);
}

#[test]
fn color_picker_page_keeps_alpha_when_the_alpha_option_is_off_and_previews_the_color() {
    let _application = start();
    let root = page();
    let picker = root.get_control::<ColorPicker>("colorPicker");
    option(&root, "alpha", true);
    let translucent = Color::from_argb(0x80, 0x12, 0x34, 0x56);
    picker.set_color(translucent);
    assert_eq!(preview(&root), translucent);
    // WinUI's IsAlphaEnabled=False hides the alpha editors and keeps Color.A.
    option(&root, "alpha", false);
    assert_eq!(picker.color(), translucent);
    picker.set_color(Colors::RED);
    assert_eq!(preview(&root), Colors::RED);
}

#[test]
fn color_picker_callbacks_do_not_retain_the_gallery_page() {
    let _application = start();
    let root = page();
    let picker = root.get_control::<ColorPicker>("colorPicker");
    let option = root.get_control::<CheckBox>("alpha");
    let shape = root.get_control::<RadioButtons>("ColorSpectrumShapeRadioButtons");
    let weak_root = root.downgrade();
    drop(root);
    assert!(weak_root.upgrade().is_none());
    // A retained child is allowed to change after removal of its page.
    picker.set_color(Colors::RED);
    option.set_is_checked(Some(true));
    shape.set_selected_index(1);
}

#[cfg(feature = "theme")]
mod themed {
    use super::*;
    use crate::tests::start_winui;
    use ferroui_controls::primitives::ToggleButton;
    use ferroui_controls::{TextBox, Window};

    fn part<T: ferroui_base::ObjectType>(picker: &Ref<ColorPicker>, name: &str) -> Ref<T> {
        picker
            .get_visual_descendants()
            .filter_map(|visual| visual.cast::<Control>())
            .find(|control| control.name().as_deref() == Some(name))
            .and_then(|control| control.cast::<T>())
            .unwrap_or_else(|| panic!("the inline part {name}"))
    }

    /// The page presents the inline template of the library's Fluent styles under the WinUI
    /// theme, and the options reach the parts.
    #[test]
    fn color_picker_page_presents_the_inline_picker_under_the_winui_theme() {
        let _application = start_winui();
        let root = page();
        let picker = root.get_control::<ColorPicker>("colorPicker");
        let window = Window::new();
        window.set_width(1280.0);
        window.set_height(900.0);
        window.set_content(Some(Control::boxed(&root)));
        window.show();
        window.update_layout();

        let spectrum = part::<Control>(&picker, "ColorSpectrum");
        assert!(spectrum.is_effectively_visible());
        assert!(spectrum.bounds().width >= 256.0, "the spectrum keeps WinUI's minimum size");
        let hex = part::<TextBox>(&picker, "PART_HexTextBox");
        assert_eq!(hex.text().as_deref(), Some("FFFFFF"), "without alpha the hex input is RGB");
        option(&root, "alpha", true);
        assert_eq!(hex.text().as_deref(), Some("FFFFFFFF"));
        assert!(part::<Control>(&picker, "AlphaSlider").is_visible());

        let more = part::<ToggleButton>(&picker, "PART_MoreButton");
        let text_entry = part::<Control>(&picker, "PART_TextEntryGrid");
        assert!(!more.is_visible());
        assert!(text_entry.is_visible());
        option(&root, "moreBtn", true);
        assert!(more.is_visible());
        assert!(!text_entry.is_visible());
        more.set_is_checked(Some(true));
        assert!(text_entry.is_visible());
        window.close();
    }
}
