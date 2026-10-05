//! Port of `Pages/CarouselPage/CarouselTransitionsPage.xaml.cs`: the class of the document
//! `Pages/CarouselPage/CarouselTransitionsPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::transitions::{CardStackPageTransition, WaveRevealPageTransition};
use ferroui_base::animation::{
    CompositePageTransition, CrossFade, IPageTransition, PageSlide, Rotate3DTransition, SlideAxis, TimeSpan,
};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{Button, Carousel, ComboBox, TextBlock, UserControl};
use std::rc::Rc;

#[repr(C)]
pub struct CarouselTransitionsPage {
    base: UserControl,
}

user_control_class!(CarouselTransitionsPage);
ferro_class_info!(CarouselTransitionsPage { new: CarouselTransitionsPage::new });
xaml_class!(CarouselTransitionsPage, "/Pages/CarouselPage/CarouselTransitionsPage.xaml");

impl CarouselTransitionsPage {
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
        for name in ["TransitionCombo", "OrientationCombo"] {
            let weak = this.downgrade();
            this.get_control::<ComboBox>(name).selection_changed(move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.apply_transition();
                }
            });
        }
        this
    }

    fn demo_carousel(&self) -> Ref<Carousel> {
        self.get_control::<Carousel>("DemoCarousel")
    }

    fn apply_transition(&self) {
        let axis = if self.get_control::<ComboBox>("OrientationCombo").selected_index() == 0 {
            SlideAxis::Horizontal
        } else {
            SlideAxis::Vertical
        };
        let label = if axis == SlideAxis::Horizontal { "Horizontal" } else { "Vertical" };

        let (transition, status): (Option<Rc<dyn IPageTransition>>, String) =
            match self.get_control::<ComboBox>("TransitionCombo").selected_index() {
                0 => (None, String::from("Transition: None")),
                1 => (
                    Some(Rc::new(PageSlide::with_duration(TimeSpan::from_seconds(0.25), axis))),
                    format!("Transition: Page Slide ({label})"),
                ),
                2 => (
                    Some(Rc::new(CrossFade::with_duration(TimeSpan::from_seconds(0.25)))),
                    String::from("Transition: Cross Fade"),
                ),
                3 => (
                    Some(Rc::new(Rotate3DTransition::with_duration(TimeSpan::from_seconds(0.5), axis, None))),
                    format!("Transition: Rotate 3D ({label})"),
                ),
                4 => (
                    Some(Rc::new(CardStackPageTransition::with_duration(TimeSpan::from_seconds(0.5), axis))),
                    format!("Transition: Card Stack ({label})"),
                ),
                5 => (
                    Some(Rc::new(WaveRevealPageTransition::with_duration(TimeSpan::from_seconds(0.8), axis))),
                    format!("Transition: Wave Reveal ({label})"),
                ),
                6 => {
                    let composite = CompositePageTransition::new();
                    composite.add(Rc::new(PageSlide::with_duration(TimeSpan::from_seconds(0.25), axis)));
                    composite.add(Rc::new(CrossFade::with_duration(TimeSpan::from_seconds(0.25))));
                    (Some(Rc::new(composite)), String::from("Transition: Composite (Slide + Fade)"))
                }
                _ => return,
            };

        self.demo_carousel().set_page_transition(transition);
        self.get_control::<TextBlock>("StatusText").set_text(Some(&status));
    }
}
