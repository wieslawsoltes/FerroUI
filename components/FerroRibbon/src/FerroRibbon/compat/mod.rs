//! Native FerroUI adapters for the ribbon's icon and input contracts.
//! These are not aliases for framework types or claims of complete platform parity.
mod font_icon;
pub use font_icon::FontIcon;
mod image_icon;
pub use image_icon::ImageIcon;
mod icon_source;
pub use icon_source::IconSource;
mod font_icon_source;
pub use font_icon_source::FontIconSource;
mod path_icon_source;
pub use path_icon_source::PathIconSource;
mod image_icon_source;
pub use image_icon_source::ImageIconSource;
mod symbol;
pub use symbol::Symbol;
mod symbol_glyphs;
mod symbol_icon;
pub use symbol_icon::{SymbolIcon, SymbolIconError};
mod symbol_icon_source;
pub use symbol_icon_source::SymbolIconSource;

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
pub(crate) use data_template_selector::register as register_template_selector;
pub use data_template_selector::{
    DataTemplateSelector, DataTemplateSelectorExt, DataTemplateSelectorImpl,
};

mod event_args;
pub(crate) use event_args::empty_event_args;

mod content_dialog_button;
pub use content_dialog_button::ContentDialogButton;
mod orientation;
pub use orientation::Orientation;
mod bitmap_icon;
mod bitmap_icon_source;
pub use bitmap_icon::BitmapIcon;
pub use bitmap_icon_source::BitmapIconSource;
