//! The pinned Gallery ColorPicker page expressed with existing native color controls.
//!
//! WinUI's inline ColorPicker maps to ColorView. Two views keep the spectrum and
//! component inputs visible together; the existing native ColorPicker retains its
//! dropdown contract. The separate native alpha slider/text input and More button
//! preserve the source options without adding compatibility properties to ColorView.

use ferroui_base::input::{FocusChangedEventArgs, InputElement};
use ferroui_base::media::{Color, SolidColorBrush};
use ferroui_base::Ref;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{CheckBox, Control, RadioButtons, StackPanel, TextBlock, TextBox};
use ferroui_controls_color_picker::converters::ColorToHexConverter;
use ferroui_controls_color_picker::primitives::ColorSlider;
use ferroui_controls_color_picker::{
    AlphaComponentPosition, ColorModel, ColorSpectrumShape, ColorView,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub(crate) fn wire(root: &Ref<Control>) {
    let Some(picker) = root
        .find_control::<Control>("colorPicker")
        .and_then(|control| control.cast::<ColorView>())
    else {
        return;
    };
    let updating = Rc::new(Cell::new(false));

    for view in [picker.clone(), root.get_control::<ColorView>("ColorInputs")] {
        let weak_root = root.downgrade();
        let updating = updating.clone();
        view.color_changed(move |args| {
            if let Some(root) = weak_root.upgrade() {
                synchronize_color(&root, &updating, args.new_color(), None);
            }
        });
    }

    let slider = root.get_control::<ColorSlider>("AlphaColorSlider");
    let weak_root = root.downgrade();
    let changing = updating.clone();
    slider.color_changed(move |args| {
        if let Some(root) = weak_root.upgrade() {
            synchronize_color(&root, &changing, args.new_color(), None);
        }
    });

    wire_text_input(root, &updating, "AlphaInput", "AlphaInputError");
    wire_text_input(root, &updating, "HexInput", "HexInputError");

    for name in [
        "moreBtn",
        "colorSlider",
        "colorChannelInput",
        "hexInput",
        "alpha",
        "alphaSlider",
        "alphaTextInput",
    ] {
        let option = root.get_control::<CheckBox>(name);
        let weak_root = root.downgrade();
        let updating = updating.clone();
        option.property_changed(move |change| {
            if change.property() == ToggleButton::is_checked_property().as_property() {
                if let Some(root) = weak_root.upgrade() {
                    update_options(&root, &updating);
                }
            }
        });
    }

    let more = root.get_control::<ToggleButton>("MoreButton");
    let weak_root = root.downgrade();
    let changing = updating.clone();
    more.property_changed(move |change| {
        if change.property() == ToggleButton::is_checked_property().as_property() {
            if let Some(root) = weak_root.upgrade() {
                update_options(&root, &changing);
            }
        }
    });

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

    // Fluent's internal model buttons size themselves from its hex TextBox.
    // Our source-compatible external hex editor hides that native grid, so the
    // model selector is composed explicitly instead of changing native styles.
    let model = root.get_control::<RadioButtons>("ColorModelRadioButtons");
    let weak_model = model.downgrade();
    let weak_inputs = root.get_control::<ColorView>("ColorInputs").downgrade();
    model.selection_changed(move |_, _| {
        if let (Some(model), Some(inputs)) = (weak_model.upgrade(), weak_inputs.upgrade()) {
            inputs.set_color_model(if model.selected_index() == 1 {
                ColorModel::Hsva
            } else {
                ColorModel::Rgba
            });
        }
    });

    update_options(root, &updating);
}

fn checked(root: &Ref<Control>, name: &str) -> bool {
    root.get_control::<CheckBox>(name).is_checked() == Some(true)
}

fn update_options(root: &Ref<Control>, updating: &Cell<bool>) {
    if updating.replace(true) {
        return;
    }
    let picker = root.get_control::<ColorView>("colorPicker");
    let inputs = root.get_control::<ColorView>("ColorInputs");
    let alpha_enabled = checked(root, "alpha");
    // The source only hides alpha editors; it does not discard Color.A. Native
    // ColorView intentionally coerces alpha when IsAlphaEnabled=false, so both
    // internal views retain alpha support and the sample controls visibility.
    picker.set_is_color_spectrum_slider_visible(checked(root, "colorSlider"));
    let channel_input = checked(root, "colorChannelInput");
    inputs.set_is_component_text_input_visible(channel_input);
    inputs.set_is_visible(channel_input);
    root.get_control::<RadioButtons>("ColorModelRadioButtons")
        .set_is_visible(channel_input);
    root.get_control::<StackPanel>("HexInputPanel")
        .set_is_visible(checked(root, "hexInput"));
    let hex = root.get_control::<TextBox>("HexInput");
    hex.set_max_length(if alpha_enabled { 9 } else { 7 });
    AutomationProperties::set_name(
        &hex,
        Some(if alpha_enabled {
            "ARGB hexadecimal color"
        } else {
            "RGB hexadecimal color"
        }),
    );

    let more = root.get_control::<ToggleButton>("MoreButton");
    let more_visible = checked(root, "moreBtn");
    let expanded = more.is_checked() == Some(true);
    more.set_is_visible(more_visible);
    more.set_content(Some(Rc::new(
        if expanded { "Less" } else { "More" }.to_string(),
    )));
    AutomationProperties::set_name(
        &more,
        Some(if expanded {
            "Hide color text inputs"
        } else {
            "Show color text inputs"
        }),
    );
    // The source retains the More expansion state when the option is hidden.
    root.get_control::<StackPanel>("ColorTextEntry")
        .set_is_visible(!more_visible || expanded);

    for name in ["alphaSlider", "alphaTextInput"] {
        root.get_control::<CheckBox>(name)
            .set_is_enabled(alpha_enabled);
    }
    root.get_control::<StackPanel>("AlphaSliderPanel")
        .set_is_visible(alpha_enabled && checked(root, "alphaSlider"));
    root.get_control::<StackPanel>("AlphaTextPanel")
        .set_is_visible(alpha_enabled && checked(root, "alphaTextInput"));
    updating.set(false);
    synchronize_color(root, updating, picker.color(), None);
}

fn synchronize_color(
    root: &Ref<Control>,
    updating: &Cell<bool>,
    color: Color,
    editing: Option<&str>,
) {
    if updating.replace(true) {
        return;
    }
    let picker = root.get_control::<ColorView>("colorPicker");
    picker.set_color(color);
    let color = picker.color();
    root.get_control::<ColorView>("ColorInputs")
        .set_color(color);
    root.get_control::<ColorSlider>("AlphaColorSlider")
        .set_color(color);
    let percent = (f64::from(color.a) * 100.0 / 255.0).round();
    if editing != Some("AlphaInput") {
        root.get_control::<TextBox>("AlphaInput")
            .set_text(Some(&format!("{percent:.0}%")));
        root.get_control::<TextBlock>("AlphaInputError")
            .set_is_visible(false);
    }
    if editing != Some("HexInput") {
        let hex = ColorToHexConverter::to_hex_string(
            color,
            AlphaComponentPosition::Leading,
            checked(root, "alpha"),
            true,
        );
        root.get_control::<TextBox>("HexInput").set_text(Some(&hex));
        root.get_control::<TextBlock>("HexInputError")
            .set_is_visible(false);
    }
    root.get_control::<Rectangle>("previewRect")
        .set_fill(Some(SolidColorBrush::with_color(color).into()));
    updating.set(false);
}

fn wire_text_input(
    root: &Ref<Control>,
    updating: &Rc<Cell<bool>>,
    name: &'static str,
    error_name: &'static str,
) {
    let input = root.get_control::<TextBox>(name);
    let weak_root = root.downgrade();
    let changing = updating.clone();
    input.property_changed(move |change| {
        if change.property() != TextBox::text_property().as_property() || changing.get() {
            return;
        }
        let Some(root) = weak_root.upgrade() else {
            return;
        };
        let input = root.get_control::<TextBox>(name);
        let value = input.text().unwrap_or_default();
        let value = if name == "AlphaInput" && !value.ends_with('%') {
            format!("{value}%")
        } else if name == "HexInput" && !value.starts_with('#') {
            format!("#{value}")
        } else {
            value
        };
        if input.text().as_deref() != Some(value.as_str()) {
            let cursor = input.selection_start().max(input.selection_end());
            changing.set(true);
            input.set_text(Some(&value));
            let cursor = if name == "AlphaInput" {
                cursor
            } else {
                value.chars().count() as i32
            };
            input.set_selection_start(cursor);
            input.set_selection_end(cursor);
            changing.set(false);
        }
        let color = root.get_control::<ColorView>("colorPicker").color();
        let parsed = if name == "AlphaInput" {
            parse_alpha(&value).map(|alpha| Color::new(alpha, color.r, color.g, color.b))
        } else {
            parse_hex(&value, checked(&root, "alpha"))
        };
        let error = root.get_control::<TextBlock>(error_name);
        error.set_is_visible(parsed.is_none());
        if let Some(color) = parsed {
            // Keep the input that originated a change, as the source's
            // AlphaTextBoxChanged / HexTextBoxChanged reasons do. Reformatting
            // here would move the caret and erase intermediate editing text.
            synchronize_color(&root, &changing, color, Some(name));
        }
    });

    let previous = Rc::new(RefCell::new(input.text().unwrap_or_default()));
    let weak_input = input.downgrade();
    let previous_input = previous.clone();
    input.add_disposable_handler(
        InputElement::got_focus_event(),
        move |_, _: &FocusChangedEventArgs| {
            if let Some(input) = weak_input.upgrade() {
                *previous_input.borrow_mut() = input.text().unwrap_or_default();
            }
        },
    );
    let weak_root = root.downgrade();
    let changing = updating.clone();
    input.add_disposable_handler(
        InputElement::lost_focus_event(),
        move |_, _: &FocusChangedEventArgs| {
            if let Some(root) = weak_root.upgrade() {
                if root.get_control::<TextBlock>(error_name).is_visible() {
                    // Upstream restores the value from focus entry, then refreshes
                    // every editor. Setting text invokes the same validated handler.
                    let previous = previous.borrow().clone();
                    root.get_control::<TextBox>(name).set_text(Some(&previous));
                }
                let color = root.get_control::<ColorView>("colorPicker").color();
                synchronize_color(&root, &changing, color, None);
            }
        },
    );
}

fn parse_alpha(text: &str) -> Option<u8> {
    let percent = text.strip_suffix('%')?.trim_start().parse::<i64>().ok()?;
    (0..=100)
        .contains(&percent)
        .then(|| (percent as f64 * 255.0 / 100.0).round() as u8)
}

fn parse_hex(text: &str, alpha_enabled: bool) -> Option<Color> {
    let digits = text.strip_prefix('#')?.trim_start();
    let digits = digits.strip_prefix('+').unwrap_or(digits);
    let digits = digits
        .strip_prefix("0x")
        .or_else(|| digits.strip_prefix("0X"))
        .unwrap_or(digits);
    if digits.is_empty() || !digits.bytes().all(|digit| digit.is_ascii_hexdigit()) {
        return None;
    }
    let color = Color::from_uint32(u32::from_str_radix(digits, 16).ok()?);
    // The source treats incomplete hex input as an integer, not CSS shorthand:
    // #F is blue=15; with alpha enabled its missing high byte means alpha=0.
    Some(if alpha_enabled {
        color
    } else {
        Color::new(255, color.r, color.g, color.b)
    })
}
