//! Port of `Pages/CarouselPage/CarouselVerticalPage.xaml.cs`: the class of the document
//! `Pages/CarouselPage/CarouselVerticalPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::animation::{CrossFade, IPageTransition, PageSlide, SlideAxis, TimeSpan};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{Button, Carousel, CheckBox, ComboBox, SelectionChangedEventArgs, TextBlock, UserControl};
use std::rc::Rc;

#[repr(C)]
pub struct CarouselVerticalPage {
    base: UserControl,
}

user_control_class!(CarouselVerticalPage);
ferro_class_info!(CarouselVerticalPage {
    new: CarouselVerticalPage::new,
    markup: {
        methods: [
            fn OnWrapSelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselVerticalPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_wrap_selection_changed(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(CarouselVerticalPage, "/Pages/CarouselPage/CarouselVerticalPage.xaml");

impl CarouselVerticalPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The handlers belong to children of the page: they hold the page weakly.
        let weak = this.downgrade();
        this.get_control::<Button>("PreviousButton").click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.demo_carousel().previous();
            }
        });
        let weak = this.downgrade();
        this.get_control::<Button>("NextButton").click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.demo_carousel().next();
            }
        });
        let demo_carousel = this.demo_carousel();
        let weak = this.downgrade();
        demo_carousel.selection_changed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_selection_changed(sender, e);
            }
        });
        let weak = this.downgrade();
        this.transition_combo().selection_changed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_transition_changed(sender, e);
            }
        });
        // The handler of an event of the carousel itself holds it weakly.
        let weak_carousel = demo_carousel.downgrade();
        demo_carousel.loaded(move |_, _| {
            if let Some(demo_carousel) = weak_carousel.upgrade() {
                demo_carousel.focus();
            }
        });
        this
    }

    fn demo_carousel(&self) -> Ref<Carousel> {
        self.get_control::<Carousel>("DemoCarousel")
    }

    fn transition_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("TransitionCombo")
    }

    fn on_selection_changed(&self, _sender: &Interactive, _e: &SelectionChangedEventArgs) {
        let demo_carousel = self.demo_carousel();
        self.get_control::<TextBlock>("StatusText").set_text(Some(&format!(
            "Item: {} / {}",
            demo_carousel.selected_index() + 1,
            demo_carousel.item_count()
        )));
    }

    fn on_transition_changed(&self, _sender: &Interactive, _e: &SelectionChangedEventArgs) {
        let transition: Option<Rc<dyn IPageTransition>> = match self.transition_combo().selected_index() {
            1 => Some(Rc::new(CrossFade::with_duration(TimeSpan::from_seconds(0.3)))),
            2 => None,
            _ => Some(Rc::new(PageSlide::with_duration(TimeSpan::from_seconds(0.3), SlideAxis::Vertical))),
        };
        self.demo_carousel().set_page_transition(transition);
    }

    fn on_wrap_selection_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.demo_carousel()
            .set_wrap_selection(self.get_control::<CheckBox>("WrapCheck").is_checked() == Some(true));
    }
}
