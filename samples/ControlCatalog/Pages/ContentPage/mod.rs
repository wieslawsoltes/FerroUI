//! The samples of the `ContentPage` gallery (directory `Pages/ContentPage`): one module per upstream
//! code-behind file, next to its document.

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

mod content_page_command_bar_page;
mod content_page_customization_page;
mod content_page_events_page;
mod content_page_first_look_page;
mod content_page_performance_page;
mod content_page_safe_area_page;

pub use content_page_command_bar_page::ContentPageCommandBarPage;
pub use content_page_customization_page::ContentPageCustomizationPage;
pub use content_page_events_page::ContentPageEventsPage;
pub use content_page_first_look_page::ContentPageFirstLookPage;
pub use content_page_performance_page::ContentPagePerformancePage;
pub use content_page_safe_area_page::ContentPageSafeAreaPage;

pub(crate) const TYPES: &[&TypeInfo] = &[
    ContentPageCommandBarPage::TYPE,
    ContentPageCustomizationPage::TYPE,
    ContentPageEventsPage::TYPE,
    ContentPageFirstLookPage::TYPE,
    ContentPagePerformancePage::TYPE,
    ContentPageSafeAreaPage::TYPE,
];

pub(crate) const CLASSES: &[&XamlClass] = &[
    &ContentPageCommandBarPage::XAML_CLASS,
    &ContentPageCustomizationPage::XAML_CLASS,
    &ContentPageEventsPage::XAML_CLASS,
    &ContentPageFirstLookPage::XAML_CLASS,
    &ContentPagePerformancePage::XAML_CLASS,
    &ContentPageSafeAreaPage::XAML_CLASS,
];

pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[];
