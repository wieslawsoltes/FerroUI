//! The samples of the `DrawerPage` gallery (directory `Pages/DrawerPage`): one
//! module per upstream code-behind file, next to its document.

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

mod drawer_page_breakpoint_page;
mod drawer_page_compact_page;
mod drawer_page_custom_flyout_page;
mod drawer_page_customization_page;
mod drawer_page_events_page;
mod drawer_page_first_look_page;
mod drawer_page_navigation_page;
mod drawer_page_performance_page;
mod drawer_page_rtl_page;
mod drawer_page_transitions_page;

pub use drawer_page_breakpoint_page::DrawerPageBreakpointPage;
pub use drawer_page_compact_page::DrawerPageCompactPage;
pub use drawer_page_custom_flyout_page::DrawerPageCustomFlyoutPage;
pub use drawer_page_customization_page::DrawerPageCustomizationPage;
pub use drawer_page_events_page::DrawerPageEventsPage;
pub use drawer_page_first_look_page::DrawerPageFirstLookPage;
pub use drawer_page_navigation_page::DrawerPageNavigationPage;
pub use drawer_page_performance_page::DrawerPagePerformancePage;
pub use drawer_page_rtl_page::DrawerPageRtlPage;
pub use drawer_page_transitions_page::DrawerPageTransitionsPage;

pub(crate) const TYPES: &[&TypeInfo] = &[
    DrawerPageBreakpointPage::TYPE,
    DrawerPageCompactPage::TYPE,
    DrawerPageCustomFlyoutPage::TYPE,
    DrawerPageCustomizationPage::TYPE,
    DrawerPageEventsPage::TYPE,
    DrawerPageFirstLookPage::TYPE,
    DrawerPageNavigationPage::TYPE,
    DrawerPagePerformancePage::TYPE,
    DrawerPageRtlPage::TYPE,
    DrawerPageTransitionsPage::TYPE,
];

pub(crate) const CLASSES: &[&XamlClass] = &[
    &DrawerPageBreakpointPage::XAML_CLASS,
    &DrawerPageCompactPage::XAML_CLASS,
    &DrawerPageCustomFlyoutPage::XAML_CLASS,
    &DrawerPageCustomizationPage::XAML_CLASS,
    &DrawerPageEventsPage::XAML_CLASS,
    &DrawerPageFirstLookPage::XAML_CLASS,
    &DrawerPageNavigationPage::XAML_CLASS,
    &DrawerPagePerformancePage::XAML_CLASS,
    &DrawerPageRtlPage::XAML_CLASS,
    &DrawerPageTransitionsPage::XAML_CLASS,
];

pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[];
