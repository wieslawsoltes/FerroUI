//! Port of `Pages/PipsPagerPage.xaml.cs`: the class of the document
//! `Pages/PipsPagerPage.xaml`.

use super::navigation_demo_helper::{Demo, NavigationDemoHelper};
use super::{
    CareCompanionAppPage, PipsPagerCarouselPage, PipsPagerCustomButtonThemesPage, PipsPagerCustomColorsPage,
    PipsPagerCustomTemplatesPage, PipsPagerEventsPage, PipsPagerGettingStartedPage, PipsPagerLargeCollectionPage,
    SanctuaryShowcasePage,
};
use crate::markup::{content_page_class, xaml_class};
use ferroui_base::interactivity::{Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{ContentPage, NavigationPage};

/// The registry of the samples of the page.
const DEMOS: &[Demo] = &[
    (
        "Getting Started",
        "First Look",
        "Default PipsPager with horizontal and vertical orientation, with and without navigation buttons.",
        || PipsPagerGettingStartedPage::new().upcast(),
    ),
    (
        "Features",
        "Carousel Integration",
        "Bind SelectedPageIndex to a Carousel's SelectedIndex for two-way synchronized page navigation.",
        || PipsPagerCarouselPage::new().upcast(),
    ),
    (
        "Features",
        "Large Collections",
        "Use MaxVisiblePips to limit visible indicators when the page count is large. Pips scroll automatically.",
        || PipsPagerLargeCollectionPage::new().upcast(),
    ),
    (
        "Features",
        "Events",
        "Monitor SelectedPageIndex changes to react to user navigation.",
        || PipsPagerEventsPage::new().upcast(),
    ),
    (
        "Appearance",
        "Custom Colors",
        "Override pip indicator colors using resource keys for normal, selected, and hover states.",
        || PipsPagerCustomColorsPage::new().upcast(),
    ),
    (
        "Appearance",
        "Custom Button Themes",
        "Replace the default chevron navigation buttons with custom button themes.",
        || PipsPagerCustomButtonThemesPage::new().upcast(),
    ),
    (
        "Appearance",
        "Custom Templates",
        "Override pip item templates to create squares, pills, numbers, or any custom shape.",
        || PipsPagerCustomTemplatesPage::new().upcast(),
    ),
    (
        "Showcases",
        "Care Companion",
        "A health care onboarding flow using PipsPager as the page indicator for a CarouselPage.",
        || CareCompanionAppPage::new().upcast(),
    ),
    (
        "Showcases",
        "Sanctuary",
        "A travel discovery app using PipsPager as the page indicator for a CarouselPage.",
        || SanctuaryShowcasePage::new().upcast(),
    ),
];

#[repr(C)]
pub struct PipsPagerPage {
    base: ContentPage,
}

content_page_class!(PipsPagerPage);
ferro_class_info!(PipsPagerPage { new: PipsPagerPage::new });
xaml_class!(PipsPagerPage, "/Pages/PipsPagerPage.xaml");

impl PipsPagerPage {
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
