//! Port of `Pages/CarouselPage/CarouselPageTransitionsPage.xaml.cs`: the class of the document
//! `Pages/CarouselPage/CarouselPageTransitionsPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::transitions::{CardStackPageTransition, WaveRevealPageTransition};
use ferroui_base::animation::{CrossFade, IPageTransition, PageSlide, SlideAxis, TimeSpan};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{
    CarouselPage, ComboBox, PageSelectionChangedEventArgs, SelectionChangedEventArgs, TextBlock, UserControl,
};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct CarouselPageTransitionsPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    component_initialized: Cell<bool>,
    /// The name of the mode of the transition this page set (`UpdateStatus` of the original
    /// reads it from the type of the transition).
    mode_name: Cell<Option<&'static str>>,
}

user_control_class!(CarouselPageTransitionsPage);
ferro_class_info!(CarouselPageTransitionsPage {
    new: CarouselPageTransitionsPage::new,
    markup: {
        methods: [
            fn OnTransitionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageTransitionsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_transition_changed(&sender, e)
                    }
                },
            fn OnPrevious(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageTransitionsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_previous(&sender, e.as_routed_event_args())
                },
            fn OnNext(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageTransitionsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_next(&sender, e.as_routed_event_args())
                },
            fn OnSelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageTransitionsPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PageSelectionChangedEventArgs>() {
                        this.on_selection_changed(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(CarouselPageTransitionsPage, "/Pages/CarouselPage/CarouselPageTransitionsPage.xaml");

impl CarouselPageTransitionsPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), component_initialized: Cell::new(false), mode_name: Cell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.component_initialized.set(true);
        this
    }

    fn carousel(&self) -> Ref<CarouselPage> {
        self.get_control::<CarouselPage>("DemoCarousel")
    }

    /// `(DemoCarousel.Pages as IList)?.Count ?? 0`.
    fn page_count(&self) -> i32 {
        self.carousel().pages().map_or(0, |pages| pages.count() as i32)
    }

    /// The field `StatusText`: null until `InitializeComponent()` has returned.
    fn status_text(&self) -> Option<Ref<TextBlock>> {
        self.component_initialized.get().then(|| self.get_control::<TextBlock>("StatusText"))
    }

    fn on_transition_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        if !self.component_initialized.get() {
            return;
        }

        let duration = TimeSpan::from_milliseconds(300.0);
        let (transition, mode_name): (Option<Rc<dyn IPageTransition>>, &'static str) =
            match self.get_control::<ComboBox>("TransitionCombo").selected_index() {
                0 => (None, "None"),
                1 => (Some(Rc::new(CrossFade::with_duration(duration))), "CrossFade"),
                2 => (Some(Rc::new(PageSlide::with_duration(duration, SlideAxis::Horizontal))), "PageSlide"),
                3 => (Some(Rc::new(PageSlide::with_duration(duration, SlideAxis::Vertical))), "PageSlide"),
                4 => (
                    Some(Rc::new(CardStackPageTransition::with_duration(
                        TimeSpan::from_milliseconds(400.0),
                        SlideAxis::Horizontal,
                    ))),
                    "Card Stack",
                ),
                5 => (
                    Some(Rc::new(WaveRevealPageTransition::with_duration(
                        TimeSpan::from_milliseconds(600.0),
                        SlideAxis::Horizontal,
                    ))),
                    "Wave Reveal",
                ),
                _ => (None, "None"),
            };
        self.carousel().set_page_transition(transition);
        self.mode_name.set(Some(mode_name));
        self.update_status();
    }

    fn on_previous(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let demo_carousel = self.carousel();
        if demo_carousel.selected_index() > 0 {
            demo_carousel.set_selected_index(demo_carousel.selected_index() - 1);
        }
    }

    fn on_next(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let demo_carousel = self.carousel();
        if demo_carousel.selected_index() < self.page_count() - 1 {
            demo_carousel.set_selected_index(demo_carousel.selected_index() + 1);
        }
    }

    fn on_selection_changed(&self, _sender: &Option<BoxedValue>, _e: &PageSelectionChangedEventArgs) {
        self.update_status();
    }

    fn update_status(&self) {
        let Some(status_text) = self.status_text() else {
            return;
        };
        let demo_carousel = self.carousel();
        // The name of the type of a transition this page did not set cannot be read from the
        // transition: the kinds the transition answers for are named instead.
        let mode_name = match (demo_carousel.page_transition(), self.mode_name.get()) {
            (None, _) => "None",
            (Some(_), Some(mode_name)) => mode_name,
            (Some(transition), None) if transition.as_composite_page_transition().is_some() => {
                "CompositePageTransition"
            }
            (Some(transition), None) if transition.as_page_slide().is_some() => "PageSlide",
            (Some(_), None) => "IPageTransition",
        };
        status_text.set_text(Some(&format!(
            "Page {} of {} | Transition: {mode_name}",
            demo_carousel.selected_index() + 1,
            self.page_count()
        )));
    }
}
