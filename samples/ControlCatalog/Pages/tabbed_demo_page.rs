//! Port of `Pages/TabbedDemoPage.xaml.cs`: the class of the document
//! `Pages/TabbedDemoPage.xaml`.

use super::navigation_demo_helper::{Demo, NavigationDemoHelper};
use super::{
    TabbedPageCollectionPage, TabbedPageCustomTabBarPage, TabbedPageCustomizationPage, TabbedPageDataTemplatePage,
    TabbedPageDisabledTabsPage, TabbedPageEventsPage, TabbedPageFabPage, TabbedPageFirstLookPage,
    TabbedPageFluidNavPage, TabbedPageGesturePage, TabbedPageKeyboardPage, TabbedPagePlacementPage,
    TabbedPageProgrammaticPage, TabbedPageTransitionsPage, TabbedPageWithDrawerPage, TabbedPageWithNavigationPage,
};
use crate::markup::{content_page_class, xaml_class};
use ferroui_base::interactivity::{Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{ContentPage, NavigationPage};

/// The registry of the samples of the page.
///
/// Four entries of the original are not listed, because their pages are not ported:
/// "Performance Monitor" of group "Performance" (`TabbedPagePerformancePage`) and the group
/// "Showcases": "Pulse Fitness" (`PulseAppPage`), "L'Avenir Restaurant" (`LAvenirAppPage`) and
/// "Retro Gaming" (`RetroGamingAppPage`), all three of the directory `Pages/NavigationPage`.
const DEMOS: &[Demo] = &[
    // Overview
    (
        "Overview",
        "First Look",
        "Basic TabbedPage with three tabs, tab placement selector, and selection status.",
        || TabbedPageFirstLookPage::new().upcast(),
    ),
    // Populate
    (
        "Populate",
        "Page Collection",
        "Populate a TabbedPage by adding ContentPage objects directly to the Pages collection.",
        || TabbedPageCollectionPage::new().upcast(),
    ),
    (
        "Populate",
        "Data Templates",
        "Bind TabbedPage to an ObservableCollection, add or remove tabs at runtime, and switch the page template.",
        || TabbedPageDataTemplatePage::new().upcast(),
    ),
    // Appearance
    (
        "Appearance",
        "Tab Customization",
        "Customize tab placement, bar background, selected and unselected tab colors.",
        || TabbedPageCustomizationPage::new().upcast(),
    ),
    (
        "Appearance",
        "Custom Tab Bar",
        "VYNTRA-style custom tab bar with floating pill, brand colours, and system-adaptive theme using only resource overrides and styles.",
        || TabbedPageCustomTabBarPage::new().upcast(),
    ),
    (
        "Appearance",
        "FAB Tab Bar",
        "Social-media-style bottom nav with a central floating action button that triggers a command, not a tab.",
        || TabbedPageFabPage::new().upcast(),
    ),
    (
        "Appearance",
        "Fluid Nav Bar",
        "Inspired by the Flutter fluid_nav_bar vignette. Color themes with animated indicator and icons.",
        || TabbedPageFluidNavPage::new().upcast(),
    ),
    // Features
    (
        "Features",
        "Programmatic Selection",
        "Preset the initial tab with SelectedIndex, jump to any tab programmatically, and respond to SelectionChanged events.",
        || TabbedPageProgrammaticPage::new().upcast(),
    ),
    (
        "Features",
        "Placement",
        "Switch the tab bar between Top, Bottom, Left, and Right placements.",
        || TabbedPagePlacementPage::new().upcast(),
    ),
    (
        "Features",
        "Page Transitions",
        "Animate tab switches with CrossFade, PageSlide, or composite transitions.",
        || TabbedPageTransitionsPage::new().upcast(),
    ),
    (
        "Features",
        "Keyboard Navigation",
        "Keyboard shortcuts to navigate between tabs, with a toggle to enable or disable.",
        || TabbedPageKeyboardPage::new().upcast(),
    ),
    (
        "Features",
        "Swipe Gestures",
        "Swipe left/right (Top/Bottom) or up/down (Left/Right) to navigate. Toggle IsGestureEnabled.",
        || TabbedPageGesturePage::new().upcast(),
    ),
    (
        "Features",
        "Events",
        "SelectionChanged, NavigatedTo, and NavigatedFrom events. Switch tabs to see the live event log.",
        || TabbedPageEventsPage::new().upcast(),
    ),
    (
        "Features",
        "Disabled Tabs",
        "IsTabEnabled attached property: disable individual tabs so they cannot be selected.",
        || TabbedPageDisabledTabsPage::new().upcast(),
    ),
    // Composition
    (
        "Composition",
        "With NavigationPage",
        "Embed a NavigationPage inside each TabbedPage tab for drill-down navigation.",
        || TabbedPageWithNavigationPage::new().upcast(),
    ),
    (
        "Composition",
        "With DrawerPage",
        "Combine TabbedPage with DrawerPage: a global navigation drawer sits over tabbed content.",
        || TabbedPageWithDrawerPage::new().upcast(),
    ),
];

#[repr(C)]
pub struct TabbedDemoPage {
    base: ContentPage,
}

content_page_class!(TabbedDemoPage);
ferro_class_info!(TabbedDemoPage { new: TabbedDemoPage::new });
xaml_class!(TabbedDemoPage, "/Pages/TabbedDemoPage.xaml");

impl TabbedDemoPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The handler of an event of the page itself holds it weakly.
        let weak = this.downgrade();
        this.loaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_loaded(sender, e);
            }
        });
        this
    }

    fn sample_nav(&self) -> Ref<NavigationPage> {
        self.get_control::<NavigationPage>("SampleNav")
    }

    /// `async void`: nothing follows the push.
    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        let sample_nav = self.sample_nav();
        let home_page = NavigationDemoHelper::create_gallery_home_page(&sample_nav, DEMOS);
        drop(sample_nav.push_async_with_transition(home_page, None));
    }
}
