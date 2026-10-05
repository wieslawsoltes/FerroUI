//! The samples of the `CommandBar` gallery (directory `Pages/CommandBar`): one module per upstream
//! code-behind file, next to its document.

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

mod command_bar_customization_page;
mod command_bar_dynamic_overflow_page;
mod command_bar_events_page;
mod command_bar_first_look_page;
mod command_bar_keyboard_page;
mod command_bar_label_position_page;
mod command_bar_overflow_page;
mod command_bar_toggle_page;

pub use command_bar_customization_page::CommandBarCustomizationPage;
pub use command_bar_dynamic_overflow_page::CommandBarDynamicOverflowPage;
pub use command_bar_events_page::CommandBarEventsPage;
pub use command_bar_first_look_page::CommandBarFirstLookPage;
pub use command_bar_keyboard_page::CommandBarKeyboardPage;
pub use command_bar_label_position_page::CommandBarLabelPositionPage;
pub use command_bar_overflow_page::CommandBarOverflowPage;
pub use command_bar_toggle_page::CommandBarTogglePage;

pub(crate) const TYPES: &[&TypeInfo] = &[
    CommandBarCustomizationPage::TYPE,
    CommandBarDynamicOverflowPage::TYPE,
    CommandBarEventsPage::TYPE,
    CommandBarFirstLookPage::TYPE,
    CommandBarKeyboardPage::TYPE,
    CommandBarLabelPositionPage::TYPE,
    CommandBarOverflowPage::TYPE,
    CommandBarTogglePage::TYPE,
];

pub(crate) const CLASSES: &[&XamlClass] = &[
    &CommandBarCustomizationPage::XAML_CLASS,
    &CommandBarDynamicOverflowPage::XAML_CLASS,
    &CommandBarEventsPage::XAML_CLASS,
    &CommandBarFirstLookPage::XAML_CLASS,
    &CommandBarKeyboardPage::XAML_CLASS,
    &CommandBarLabelPositionPage::XAML_CLASS,
    &CommandBarOverflowPage::XAML_CLASS,
    &CommandBarTogglePage::XAML_CLASS,
];

pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[];
