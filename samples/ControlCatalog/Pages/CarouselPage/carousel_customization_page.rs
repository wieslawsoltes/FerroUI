//! Port of `Pages/CarouselPage/CarouselCustomizationPage.xaml.cs`: the class of the document
//! `Pages/CarouselPage/CarouselCustomizationPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::animation::{PageSlide, SlideAxis, TimeSpan};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::primitives::RangeBaseValueChangedEventArgs;
use ferroui_controls::{Button, Carousel, CheckBox, ComboBox, Slider, TextBlock, UserControl};
use std::rc::Rc;

/// `value.ToString("0.##")`: at most two decimals, without trailing zeros.
fn format_up_to_two_decimals(value: f64) -> String {
    let text = format!("{value:.2}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[repr(C)]
pub struct CarouselCustomizationPage {
    base: UserControl,
}

user_control_class!(CarouselCustomizationPage);
ferro_class_info!(CarouselCustomizationPage {
    new: CarouselCustomizationPage::new,
    markup: {
        methods: [
            fn OnWrapSelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_wrap_selection_changed(&sender, e.as_routed_event_args())
                },
            fn OnSwipeEnabledChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_swipe_enabled_changed(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(CarouselCustomizationPage, "/Pages/CarouselPage/CarouselCustomizationPage.xaml");

impl CarouselCustomizationPage {
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
        let weak = this.downgrade();
        this.orientation_combo().selection_changed(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.apply_orientation();
            }
        });
        let weak = this.downgrade();
        this.get_control::<Slider>("ViewportSlider").value_changed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_viewport_fraction_changed(sender, e);
            }
        });
        this
    }

    fn demo_carousel(&self) -> Ref<Carousel> {
        self.get_control::<Carousel>("DemoCarousel")
    }

    fn orientation_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("OrientationCombo")
    }

    fn text(&self, name: &str) -> Ref<TextBlock> {
        self.get_control::<TextBlock>(name)
    }

    fn apply_orientation(&self) {
        let horizontal = self.orientation_combo().selected_index() == 0;
        let axis = if horizontal { SlideAxis::Horizontal } else { SlideAxis::Vertical };

        self.demo_carousel()
            .set_page_transition(Some(Rc::new(PageSlide::with_duration(TimeSpan::from_seconds(0.25), axis))));
        self.text("StatusText")
            .set_text(Some(&format!("Orientation: {}", if horizontal { "Horizontal" } else { "Vertical" })));
    }

    fn on_viewport_fraction_changed(&self, _sender: &Interactive, e: &RangeBaseValueChangedEventArgs) {
        // `Math.Round(value, 2)`: midpoints to even.
        let value = (e.new_value() * 100.0).round_ties_even() / 100.0;
        self.demo_carousel().set_viewport_fraction(value);
        self.text("ViewportLabel").set_text(Some(&format!("{value:.2}")));
        self.text("ViewportHint").set_text(Some(&if value >= 1.0 {
            String::from("1.00 shows a single full page.")
        } else {
            format!("{} pages fit in view. Try 0.80 for peeking.", format_up_to_two_decimals(1.0 / value))
        }));
    }

    fn on_wrap_selection_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.demo_carousel()
            .set_wrap_selection(self.get_control::<CheckBox>("WrapSelectionCheck").is_checked() == Some(true));
    }

    fn on_swipe_enabled_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.demo_carousel()
            .set_is_swipe_enabled(self.get_control::<CheckBox>("SwipeEnabledCheck").is_checked() == Some(true));
    }
}
