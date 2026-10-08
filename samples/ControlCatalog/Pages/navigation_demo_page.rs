//! Port of `Pages/NavigationDemoPage.xaml.cs`: the class of the document
//! `Pages/NavigationDemoPage.xaml`.

use super::navigation_demo_helper::{Demo, NavigationDemoHelper};
use super::{
    NavigationPageAppearancePage, NavigationPageAttachedMethodsPage, NavigationPageBackButtonPage,
    NavigationPageEventsPage, NavigationPageFirstLookPage, NavigationPageGesturePage, NavigationPageModalPage,
    NavigationPageModalTransitionsPage, NavigationPageScrollAwarePage, NavigationPageStackPage,
    NavigationPageTitlePage, NavigationPageToolbarPage, NavigationPageTransitionsPage,
};
use crate::markup::{content_page_class, xaml_class};
use ferroui_base::interactivity::{Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{ContentPage, NavigationPage};

/// The registry of the samples of the page.
///
/// Nine entries of the original are not listed, because their pages are not ported (see
/// `excluded.txt`): the group "Data", "Pass Data" (`NavigationPagePassDataPage`) and "MVVM
/// Navigation" (`NavigationPageMvvmPage`); "Interactive Header" of group "Features"
/// (`NavigationPageInteractiveHeaderPage`); the group "Performance", "Performance Monitor"
/// (`NavigationPagePerformancePage`); and the group "Showcases": "Pulse Fitness"
/// (`PulseAppPage`), "L'Avenir" (`LAvenirAppPage`), the streaming app (`FerroFlixAppPage`),
/// "Retro Gaming" (`RetroGamingAppPage`) and "Curved Header" (`NavigationPageCurvedHeaderPage`).
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
