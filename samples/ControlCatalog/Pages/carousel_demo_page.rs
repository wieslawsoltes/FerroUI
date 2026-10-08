//! Port of `Pages/CarouselDemoPage.xaml.cs`: the class of the document
//! `Pages/CarouselDemoPage.xaml`.

use super::navigation_demo_helper::{Demo, NavigationDemoHelper};
use super::{
    CareCompanionAppPage, CarouselCustomizationPage, CarouselDataBindingPage, CarouselGalleryAppPage,
    CarouselGesturesPage, CarouselGettingStartedPage, CarouselMultiItemPage, CarouselPageCustomizationPage,
    CarouselPageDataTemplatePage, CarouselPageEventsPage, CarouselPageFirstLookPage, CarouselPageGesturePage,
    CarouselPagePerformancePage, CarouselPageSelectionPage, CarouselPageTransitionsPage, CarouselTransitionsPage,
    CarouselVerticalPage, SanctuaryShowcasePage,
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
        "Basic CarouselPage with three pages and page indicator.",
        || CarouselPageFirstLookPage::new().upcast(),
    ),
    // Populate
    (
        "Populate",
        "Data Templates",
        "Bind CarouselPage to an ObservableCollection, add or remove pages at runtime, and switch the page template.",
        || CarouselPageDataTemplatePage::new().upcast(),
    ),
    // Appearance
    (
        "Appearance",
        "Customization",
        "Switch slide direction between horizontal and vertical with PageSlide. Page indicator dots update on each selection.",
        || CarouselPageCustomizationPage::new().upcast(),
    ),
    // Features
    (
        "Features",
        "Page Transitions",
        "Animate page switches with CrossFade or PageSlide.",
        || CarouselPageTransitionsPage::new().upcast(),
    ),
    (
        "Features",
        "Programmatic Selection",
        "Jump to any page programmatically with SelectedIndex and respond to SelectionChanged events.",
        || CarouselPageSelectionPage::new().upcast(),
    ),
    (
        "Features",
        "Gesture & Keyboard",
        "Swipe left/right to navigate pages. Toggle IsGestureEnabled and IsKeyboardNavigationEnabled.",
        || CarouselPageGesturePage::new().upcast(),
    ),
    (
        "Features",
        "Events",
        "SelectionChanged, NavigatedTo, and NavigatedFrom events. Swipe or navigate to see the live event log.",
        || CarouselPageEventsPage::new().upcast(),
    ),
    // Performance
    (
        "Performance",
        "Performance Monitor",
        "Track page count and live page instances. Observe how pages are released after removing them.",
        || CarouselPagePerformancePage::new().upcast(),
    ),
    // Showcases
    (
        "Showcases",
        "Sanctuary",
        "Travel discovery app with 3 full-screen immersive pages. Each page has a real background photo, gradient overlay, and themed content. Built as a 1:1 replica of a Stitch design.",
        || SanctuaryShowcasePage::new().upcast(),
    ),
    (
        "Showcases",
        "Care Companion",
        "Healthcare onboarding with CarouselPage (3 pages), then a TabbedPage patient dashboard. Skip or complete onboarding to navigate to the dashboard via RemovePage.",
        || CareCompanionAppPage::new().upcast(),
    ),
    // Carousel (ItemsControl) demos
    (
        "Carousel",
        "Getting Started",
        "Basic Carousel with image items and previous/next navigation buttons.",
        || CarouselGettingStartedPage::new().upcast(),
    ),
    (
        "Carousel",
        "Transitions",
        "Configure page transitions: PageSlide, CrossFade, 3D Rotation, or None.",
        || CarouselTransitionsPage::new().upcast(),
    ),
    (
        "Carousel",
        "Customization",
        "Adjust orientation and transition type to tailor the carousel layout.",
        || CarouselCustomizationPage::new().upcast(),
    ),
    (
        "Carousel",
        "Gestures & Keyboard",
        "Navigate items via swipe gesture and arrow keys. Toggle each input mode on and off.",
        || CarouselGesturesPage::new().upcast(),
    ),
    (
        "Carousel",
        "Vertical Orientation",
        "Carousel with Orientation set to Vertical, navigated with Up/Down keys, swipe, or buttons.",
        || CarouselVerticalPage::new().upcast(),
    ),
    (
        "Carousel",
        "Multi-Item Peek",
        "Adjust ViewportFraction to show multiple items simultaneously with adjacent cards peeking.",
        || CarouselMultiItemPage::new().upcast(),
    ),
    (
        "Carousel",
        "Data Binding",
        "Bind Carousel to an ObservableCollection and add, remove, or shuffle items at runtime.",
        || CarouselDataBindingPage::new().upcast(),
    ),
    (
        "Carousel",
        "Curated Gallery",
        "Editorial art gallery app with DrawerPage navigation, hero Carousel with PipsPager dots, and a horizontal peek carousel for collection highlights.",
        || CarouselGalleryAppPage::new().upcast(),
    ),
];

#[repr(C)]
pub struct CarouselDemoPage {
    base: ContentPage,
}

content_page_class!(CarouselDemoPage);
ferro_class_info!(CarouselDemoPage { new: CarouselDemoPage::new });
xaml_class!(CarouselDemoPage, "/Pages/CarouselDemoPage.xaml");

impl CarouselDemoPage {
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
