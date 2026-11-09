//! Port of `Pages/DrawerDemoPage.xaml.cs`: the class of the document
//! `Pages/DrawerDemoPage.xaml`.

use super::navigation_demo_helper::{Demo, NavigationDemoHelper};
use super::{
    DrawerPageBreakpointPage, DrawerPageCompactPage, DrawerPageCustomFlyoutPage, DrawerPageCustomizationPage,
    DrawerPageEventsPage, DrawerPageFirstLookPage, DrawerPageNavigationPage, DrawerPagePerformancePage,
    DrawerPageRtlPage, DrawerPageTransitionsPage,
};
use crate::markup::{content_page_class, xaml_class};
use ferroui_base::interactivity::{Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{ContentPage, NavigationPage};

/// The registry of the samples of the page.
///
/// The description of the entry "Performance Monitor" differs from the original's, which
/// promises the size of the managed heap and the effect of the garbage collector: the page
/// reports what can be measured here (see `navigation_performance_monitor_helper.rs`).
const DEMOS: &[Demo] = &[
    // Overview
    (
        "Overview",
        "First Look",
        "Basic DrawerPage with a navigation drawer, menu items, and detail content.",
        || DrawerPageFirstLookPage::new().upcast(),
    ),
    // Features
    (
        "Features",
        "Navigation",
        "Master-detail pattern: select a drawer menu item to navigate the detail via a NavigationPage with hamburger-to-back-button transition.",
        || DrawerPageNavigationPage::new().upcast(),
    ),
    (
        "Features",
        "Compact Rail",
        "CompactOverlay and CompactInline layout modes: a narrow icon rail is always visible and expands on open. Adjust rail width and open pane width.",
        || DrawerPageCompactPage::new().upcast(),
    ),
    (
        "Features",
        "Breakpoint",
        "DrawerBreakpointLength: below the threshold the drawer switches to Overlay mode automatically. Resize the window or adjust the slider to see the layout switch in real time.",
        || DrawerPageBreakpointPage::new().upcast(),
    ),
    (
        "Features",
        "Events",
        "Opened, Closing, and Closed drawer events plus NavigatedTo and NavigatedFrom page lifecycle events. Enable 'Cancel next close' to prevent the drawer from closing.",
        || DrawerPageEventsPage::new().upcast(),
    ),
    (
        "Features",
        "RTL Layout",
        "Right-to-left layout: drawer opens from the right edge with mirrored gestures.",
        || DrawerPageRtlPage::new().upcast(),
    ),
    // Appearance
    (
        "Appearance",
        "Customization",
        "Customize drawer behavior, layout mode, length, colors, header and footer.",
        || DrawerPageCustomizationPage::new().upcast(),
    ),
    (
        "Appearance",
        "Custom Flyout",
        "Dark overlay menu with staggered item animations and CrossFade page transitions on the detail NavigationPage.",
        || DrawerPageCustomFlyoutPage::new().upcast(),
    ),
    (
        "Appearance",
        "Transitions",
        "Configure the detail NavigationPage transition. Choose CrossFade, PageSlide, or CompositePageTransition to animate detail page changes.",
        || DrawerPageTransitionsPage::new().upcast(),
    ),
    // Performance
    (
        "Performance",
        "Performance Monitor",
        "Track detail page swaps and live page instances. Observe how pages are released after swapping them.",
        || DrawerPagePerformancePage::new().upcast(),
    ),
];

#[repr(C)]
pub struct DrawerDemoPage {
    base: ContentPage,
}

content_page_class!(DrawerDemoPage);
ferro_class_info!(DrawerDemoPage { new: DrawerDemoPage::new });
xaml_class!(DrawerDemoPage, "/Pages/DrawerDemoPage.xaml");

impl DrawerDemoPage {
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
