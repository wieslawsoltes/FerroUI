//! Native FerroUI adapters for the ribbon's icon and input contracts.
//! These are not aliases for framework types or claims of complete platform parity.

// The icons, `Orientation` and `ContentDialogButton` are the framework's: one type per name in the
// default XML namespace.
pub(crate) use ferroui_base::layout::Orientation;
#[cfg(test)]
pub(crate) use ferroui_controls::Symbol;
pub(crate) use ferroui_controls::{
    ContentDialogButton, FontIcon, FontIconSource, IconSource, PathIconSource, SymbolIcon,
    SymbolIconSource,
};
pub(crate) use ferroui_controls_icons::{BitmapIcon, BitmapIconSource, ImageIcon, ImageIconSource};

mod menu_items;
pub use menu_items::{
    MenuFlyoutItem, MenuFlyoutItemImpl, MenuFlyoutItemImplExt, MenuFlyoutSeparator,
    MenuFlyoutSubItem, ToggleMenuFlyoutItem,
};

mod radio_menu_item;
pub use radio_menu_item::RadioMenuFlyoutItem;
mod visibility;
pub use visibility::Visibility;
mod ribbon_input_text_box;
pub use ribbon_input_text_box::RibbonInputTextBox;
mod ribbon_input_repeat_button;
pub use ribbon_input_repeat_button::RibbonInputRepeatButton;

pub(crate) mod slider_math;

mod ribbon_input_slider;
pub use ribbon_input_slider::RibbonInputSlider;

pub mod data_template_selector;
pub mod delegates;
pub(crate) use data_template_selector::register as register_template_selector;
pub use data_template_selector::{
    DataTemplateSelector, DataTemplateSelectorExt, DataTemplateSelectorImpl,
};

mod event_args;
pub(crate) use event_args::empty_event_args;
