//! Port of `Pages/GesturePage.xaml.cs`: the class of the document
//! `Pages/GesturePage.xaml`.

use super::navigation_demo_helper::{Demo, NavigationDemoHelper};
use super::{GesturePinchRotationPage, GesturePinchZoomPage};
use crate::markup::{content_page_class, xaml_class};
use ferroui_base::interactivity::{Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{ContentPage, NavigationPage};

/// The registry of the samples of the page.
///
/// Two entries of the original are not listed, because their pages are not ported (see
/// `excluded.txt`): "Pull Gesture" of group "Touch / Pen" (`GesturePullPage`) and
/// "Swipe Gesture" of group "Touch / Pen / Mouse" (`GestureSwipePage`).
const DEMOS: &[Demo] = &[
    (
        "Multi Touch",
        "Pinch / Zoom",
        "Pinch to scale an image using composition visuals. Scroll to pan when zoomed in.",
        || GesturePinchZoomPage::new().upcast(),
    ),
    (
        "Multi Touch",
        "Pinch / Rotation",
        "Pinch to rotate a rectangle. The Angle property from the pinch event drives a RotateTransform.",
        || GesturePinchRotationPage::new().upcast(),
    ),
];

#[repr(C)]
pub struct GesturePage {
    base: ContentPage,
}

content_page_class!(GesturePage);
ferro_class_info!(GesturePage { new: GesturePage::new });
xaml_class!(GesturePage, "/Pages/GesturePage.xaml");

impl GesturePage {
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
