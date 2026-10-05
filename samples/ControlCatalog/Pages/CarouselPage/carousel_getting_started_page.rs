//! Port of `Pages/CarouselPage/CarouselGettingStartedPage.xaml.cs`: the class of the document
//! `Pages/CarouselPage/CarouselGettingStartedPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{Button, Carousel, SelectionChangedEventArgs, TextBlock, UserControl};

#[repr(C)]
pub struct CarouselGettingStartedPage {
    base: UserControl,
}

user_control_class!(CarouselGettingStartedPage);
ferro_class_info!(CarouselGettingStartedPage { new: CarouselGettingStartedPage::new });
xaml_class!(CarouselGettingStartedPage, "/Pages/CarouselPage/CarouselGettingStartedPage.xaml");

impl CarouselGettingStartedPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The handlers belong to children of the page: they hold the page weakly.
        let weak = this.downgrade();
        this.get_control::<Button>("PreviousButton").click(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_previous(sender, e);
            }
        });
        let weak = this.downgrade();
        this.get_control::<Button>("NextButton").click(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_next(sender, e);
            }
        });
        let weak = this.downgrade();
        this.demo_carousel().selection_changed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_selection_changed(sender, e);
            }
        });
        this
    }

    fn demo_carousel(&self) -> Ref<Carousel> {
        self.get_control::<Carousel>("DemoCarousel")
    }

    fn on_previous(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        self.demo_carousel().previous();
        self.update_status();
    }

    fn on_next(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        self.demo_carousel().next();
        self.update_status();
    }

    fn on_selection_changed(&self, _sender: &Interactive, _e: &SelectionChangedEventArgs) {
        self.update_status();
    }

    fn update_status(&self) {
        let demo_carousel = self.demo_carousel();
        let index = demo_carousel.selected_index() + 1;
        let count = demo_carousel.item_count();
        self.get_control::<TextBlock>("StatusText").set_text(Some(&format!("Item: {index} / {count}")));
    }
}
