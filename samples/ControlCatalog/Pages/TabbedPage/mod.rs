//! The samples of the `TabbedPage` gallery (directory `Pages/TabbedPage`): one
//! module per upstream file, next to its document.

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

mod centered_tab_panel;
#[path = "FluidNavBar/fluid_nav_bar.rs"]
mod fluid_nav_bar;
#[path = "FluidNavBar/fluid_nav_item.rs"]
mod fluid_nav_item;
mod tabbed_page_collection_page;
mod tabbed_page_custom_tab_bar_page;
mod tabbed_page_customization_page;
mod tabbed_page_data_template_page;
mod tabbed_page_disabled_tabs_page;
mod tabbed_page_events_page;
mod tabbed_page_fab_page;
mod tabbed_page_first_look_page;
mod tabbed_page_fluid_nav_page;
mod tabbed_page_gesture_page;
mod tabbed_page_keyboard_page;
mod tabbed_page_performance_page;
mod tabbed_page_placement_page;
mod tabbed_page_programmatic_page;
mod tabbed_page_transitions_page;
mod tabbed_page_with_drawer_page;
mod tabbed_page_with_navigation_page;

pub use centered_tab_panel::CenteredTabPanel;
pub use fluid_nav_bar::FluidNavBar;
pub use fluid_nav_item::FluidNavItem;
pub use tabbed_page_collection_page::TabbedPageCollectionPage;
pub use tabbed_page_custom_tab_bar_page::TabbedPageCustomTabBarPage;
pub use tabbed_page_customization_page::TabbedPageCustomizationPage;
pub use tabbed_page_data_template_page::TabbedPageDataTemplatePage;
pub use tabbed_page_disabled_tabs_page::TabbedPageDisabledTabsPage;
pub use tabbed_page_events_page::TabbedPageEventsPage;
pub use tabbed_page_fab_page::TabbedPageFabPage;
pub use tabbed_page_first_look_page::TabbedPageFirstLookPage;
pub use tabbed_page_fluid_nav_page::TabbedPageFluidNavPage;
pub use tabbed_page_gesture_page::TabbedPageGesturePage;
pub use tabbed_page_keyboard_page::TabbedPageKeyboardPage;
pub use tabbed_page_performance_page::TabbedPagePerformancePage;
pub use tabbed_page_placement_page::TabbedPagePlacementPage;
pub use tabbed_page_programmatic_page::TabbedPageProgrammaticPage;
pub use tabbed_page_transitions_page::TabbedPageTransitionsPage;
pub use tabbed_page_with_drawer_page::TabbedPageWithDrawerPage;
pub use tabbed_page_with_navigation_page::TabbedPageWithNavigationPage;

pub(crate) const TYPES: &[&TypeInfo] = &[
    CenteredTabPanel::TYPE,
    FluidNavBar::TYPE,
    TabbedPageCollectionPage::TYPE,
    TabbedPageCustomTabBarPage::TYPE,
    TabbedPageCustomizationPage::TYPE,
    TabbedPageDataTemplatePage::TYPE,
    TabbedPageDisabledTabsPage::TYPE,
    TabbedPageEventsPage::TYPE,
    TabbedPageFabPage::TYPE,
    TabbedPageFirstLookPage::TYPE,
    TabbedPageFluidNavPage::TYPE,
    TabbedPageGesturePage::TYPE,
    TabbedPageKeyboardPage::TYPE,
    TabbedPagePerformancePage::TYPE,
    TabbedPagePlacementPage::TYPE,
    TabbedPageProgrammaticPage::TYPE,
    TabbedPageTransitionsPage::TYPE,
    TabbedPageWithDrawerPage::TYPE,
    TabbedPageWithNavigationPage::TYPE,
];

pub(crate) const CLASSES: &[&XamlClass] = &[
    &TabbedPageCollectionPage::XAML_CLASS,
    &TabbedPageCustomTabBarPage::XAML_CLASS,
    &TabbedPageCustomizationPage::XAML_CLASS,
    &TabbedPageDataTemplatePage::XAML_CLASS,
    &TabbedPageDisabledTabsPage::XAML_CLASS,
    &TabbedPageEventsPage::XAML_CLASS,
    &TabbedPageFabPage::XAML_CLASS,
    &TabbedPageFirstLookPage::XAML_CLASS,
    &TabbedPageFluidNavPage::XAML_CLASS,
    &TabbedPageGesturePage::XAML_CLASS,
    &TabbedPageKeyboardPage::XAML_CLASS,
    &TabbedPagePerformancePage::XAML_CLASS,
    &TabbedPagePlacementPage::XAML_CLASS,
    &TabbedPageProgrammaticPage::XAML_CLASS,
    &TabbedPageTransitionsPage::XAML_CLASS,
    &TabbedPageWithDrawerPage::XAML_CLASS,
    &TabbedPageWithNavigationPage::XAML_CLASS,
];

pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[];
