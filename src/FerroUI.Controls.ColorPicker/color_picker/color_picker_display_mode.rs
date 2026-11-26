//! Not from upstream: how a [`ColorPicker`](crate::ColorPicker) presents its editors (#1266).

use ferroui_base::data::core::ValueTypes;
use ferroui_base::ferro_markup_enum;
use ferroui_base::metadata::{MarkupType, MarkupTyped};

/// Defines how a [`ColorPicker`](crate::ColorPicker) presents its editors.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum ColorPickerDisplayMode {
    /// The editors are in a drop down; only the preview content area is shown until it opens.
    #[default]
    DropDown = 0,

    /// The editors are shown in place, laid out as WinUI's `ColorPicker`: the spectrum with the
    /// color preview, the color and alpha sliders, the optional More button, and the color
    /// model, channel, alpha and hex inputs.
    Inline = 1,
}

ferro_markup_enum!(ColorPickerDisplayMode { DropDown, Inline }, { namespace: "FerroUI.Controls" });

/// The markup types declared in this file.
pub(crate) const TYPES: &[&MarkupType] = &[<ColorPickerDisplayMode as MarkupTyped>::MARKUP];

/// Registers the type of this file with the untyped value conversions when the tables of
/// types register at start-up.
pub(crate) fn register_value_type_table() {
    ValueTypes::register_used::<ColorPickerDisplayMode>();
}

impl ferroui_base::data::core::ValueTypeRegistration for ColorPickerDisplayMode {
    fn register_value_conversions() {
        ValueTypes::register_nullable::<ColorPickerDisplayMode>();
    }
}
