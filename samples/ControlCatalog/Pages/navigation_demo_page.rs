//! Port of `Pages/NavigationDemoPage.xaml.cs`: the class of the document
//! `Pages/NavigationDemoPage.xaml`.

use super::navigation_demo_helper::{Demo, NavigationDemoHelper};
use super::{
    FerroFlixAppPage, LAvenirAppPage, NavigationPageAppearancePage, NavigationPageAttachedMethodsPage,
    NavigationPageBackButtonPage, NavigationPageCurvedHeaderPage, NavigationPageEventsPage, NavigationPageFirstLookPage,
    NavigationPageGesturePage, NavigationPageInteractiveHeaderPage, NavigationPageModalPage,
    NavigationPageModalTransitionsPage, NavigationPageMvvmPage, NavigationPagePassDataPage,
    NavigationPagePerformancePage, NavigationPageScrollAwarePage, NavigationPageStackPage, NavigationPageTitlePage,
    NavigationPageToolbarPage, NavigationPageTransitionsPage, PulseAppPage, RetroGamingAppPage,
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
        "Basic NavigationPage with push/pop navigation and back button support.",
        || NavigationPageFirstLookPage::new().upcast(),
    ),
    (
        "Overview",
        "Modal Navigation",
        "Push and pop modal pages that appear on top of the navigation stack.",
        || NavigationPageModalPage::new().upcast(),
    ),
    (
        "Overview",
        "Navigation Events",
        "Subscribe to Pushed, Popped, PoppedToRoot, ModalPushed, and ModalPopped events.",
        || NavigationPageEventsPage::new().upcast(),
    ),
    // Appearance
    (
        "Appearance",
        "Bar Customization",
        "Customize the navigation bar background, foreground, shadow, and visibility.",
        || NavigationPageAppearancePage::new().upcast(),
    ),
    (
        "Appearance",
        "Header",
        "Set page header content: a string, icon, or any custom control in the navigation bar.",
        || NavigationPageTitlePage::new().upcast(),
    ),
    // Data
    (
        "Data",
        "Pass Data",
        "Pass data during navigation via constructor arguments or DataContext.",
        || NavigationPagePassDataPage::new().upcast(),
    ),
    (
        "Data",
        "MVVM Navigation",
        "Keep navigation decisions in view models by routing NavigationPage push and pop operations through a small INavigationService.",
        || NavigationPageMvvmPage::new().upcast(),
    ),
    // Features
    (
        "Features",
        "Attached Methods",
        "Per-page navigation bar and back button control via static attached methods.",
        || NavigationPageAttachedMethodsPage::new().upcast(),
    ),
    (
        "Features",
        "Back Button",
        "Customize, hide, or intercept the back button.",
        || NavigationPageBackButtonPage::new().upcast(),
    ),
    (
        "Features",
        "CommandBar",
        "Add, remove and position CommandBar items inside the navigation bar or as a bottom bar.",
        || NavigationPageToolbarPage::new().upcast(),
    ),
    (
        "Features",
        "Transitions",
        "Configure page transitions: PageSlide, Parallax Slide, CrossFade, Fade Through, and more.",
        || NavigationPageTransitionsPage::new().upcast(),
    ),
    (
        "Features",
        "Modal Transitions",
        "Configure modal transition: PageSlide from bottom, CrossFade, or None.",
        || NavigationPageModalTransitionsPage::new().upcast(),
    ),
    (
        "Features",
        "Stack Management",
        "Remove or insert pages anywhere in the navigation stack at runtime.",
        || NavigationPageStackPage::new().upcast(),
    ),
    (
        "Features",
        "Interactive Header",
        "Build a header with a title and live search box that filters page content in real time.",
        || NavigationPageInteractiveHeaderPage::new().upcast(),
    ),
    (
        "Features",
        "Back Swipe Gesture",
        "Swipe from the left edge to interactively pop the current page.",
        || NavigationPageGesturePage::new().upcast(),
    ),
    (
        "Features",
        "Scroll-Aware Bar",
        "Hide the navigation bar on downward scroll and reveal it on upward scroll.",
        || NavigationPageScrollAwarePage::new().upcast(),
    ),
    // Performance
    (
        "Performance",
        "Performance Monitor",
        "Track stack depth and live page instances. Observe how pages are released after popping them.",
        || NavigationPagePerformancePage::new().upcast(),
    ),
    // Showcases
    (
        "Showcases",
        "Pulse Fitness",
        "Login flow with RemovePage, TabbedPage dashboard with bottom tabs, and NavigationPage push for workout detail.",
        || PulseAppPage::new().upcast(),
    ),
    (
        "Showcases",
        "L'Avenir",
        "Restaurant app with DrawerPage flyout menu, TabbedPage bottom tabs, and NavigationPage push for dish detail.",
        || LAvenirAppPage::new().upcast(),
    ),
    (
        "Showcases",
        "FerroFlix",
        "Streaming app with dark NavigationPage, hidden nav bar on home, and custom bar tint on movie detail pages.",
        || FerroFlixAppPage::new().upcast(),
    ),
    (
        "Showcases",
        "Retro Gaming",
        "Arcade-style app with NavigationPage header, TabbedPage bottom tabs with CenteredTabPanel, and game detail push.",
        || RetroGamingAppPage::new().upcast(),
    ),
    (
        "Showcases",
        "Curved Header",
        "Shop app with dome-bottomed white header on home (nav bar hidden) and blue curved header on detail (BarLayoutBehavior.Overlay).",
        || NavigationPageCurvedHeaderPage::new().upcast(),
    ),
];

#[repr(C)]
pub struct NavigationDemoPage {
    base: ContentPage,
}

content_page_class!(NavigationDemoPage);
ferro_class_info!(NavigationDemoPage { new: NavigationDemoPage::new });
xaml_class!(NavigationDemoPage, "/Pages/NavigationDemoPage.xaml");

impl NavigationDemoPage {
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
