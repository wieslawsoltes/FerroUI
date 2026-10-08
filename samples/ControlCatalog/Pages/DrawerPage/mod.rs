//! The samples of the `DrawerPage` gallery (directory `Pages/DrawerPage`): one
//! module per upstream code-behind file, next to its document.

use crate::markup::XamlClass;
use ferroui_base::metadata::MarkupType;
use ferroui_base::TypeInfo;

mod controls_gallery_app_page;
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
mod eco_tracker_app_page;
mod eco_tracker_community_view;
mod eco_tracker_habits_view;
mod eco_tracker_home_view;
mod eco_tracker_stats_view;
mod modern_app_page;
mod modern_discover_view;
mod modern_my_trips_view;
mod modern_profile_view;
mod modern_settings_view;

pub use controls_gallery_app_page::ControlsGalleryAppPage;
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
pub use eco_tracker_app_page::EcoTrackerAppPage;
pub use eco_tracker_community_view::EcoTrackerCommunityView;
pub use eco_tracker_habits_view::EcoTrackerHabitsView;
pub use eco_tracker_home_view::EcoTrackerHomeView;
pub use eco_tracker_stats_view::EcoTrackerStatsView;
pub use modern_app_page::ModernAppPage;
pub use modern_discover_view::ModernDiscoverView;
pub use modern_my_trips_view::ModernMyTripsView;
pub use modern_profile_view::ModernProfileView;
pub use modern_settings_view::ModernSettingsView;

pub(crate) const TYPES: &[&TypeInfo] = &[
    ControlsGalleryAppPage::TYPE,
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
    EcoTrackerAppPage::TYPE,
    EcoTrackerCommunityView::TYPE,
    EcoTrackerHabitsView::TYPE,
    EcoTrackerHomeView::TYPE,
    EcoTrackerStatsView::TYPE,
    ModernAppPage::TYPE,
    ModernDiscoverView::TYPE,
    ModernMyTripsView::TYPE,
    ModernProfileView::TYPE,
    ModernSettingsView::TYPE,
];

pub(crate) const CLASSES: &[&XamlClass] = &[
    &ControlsGalleryAppPage::XAML_CLASS,
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
    &EcoTrackerAppPage::XAML_CLASS,
    &EcoTrackerCommunityView::XAML_CLASS,
    &EcoTrackerHabitsView::XAML_CLASS,
    &EcoTrackerHomeView::XAML_CLASS,
    &EcoTrackerStatsView::XAML_CLASS,
    &ModernAppPage::XAML_CLASS,
    &ModernDiscoverView::XAML_CLASS,
    &ModernMyTripsView::XAML_CLASS,
    &ModernProfileView::XAML_CLASS,
    &ModernSettingsView::XAML_CLASS,
];

pub(crate) const MARKUP_TYPES: &[&MarkupType] = &[];
