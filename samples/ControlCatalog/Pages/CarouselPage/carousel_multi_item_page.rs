//! Port of `Pages/CarouselPage/CarouselMultiItemPage.xaml.cs`: the class of the document
//! `Pages/CarouselPage/CarouselMultiItemPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::primitives::RangeBaseValueChangedEventArgs;
use ferroui_controls::{Button, Carousel, CheckBox, SelectionChangedEventArgs, TextBlock, UserControl};
use std::cell::Cell;
use std::rc::Rc;

/// `value.ToString("0.#")`: at most one decimal, without a trailing zero.
fn format_up_to_one_decimal(value: f64) -> String {
    let text = format!("{value:.1}");
    text.trim_end_matches('0').trim_end_matches('.').to_string()
}

#[repr(C)]
pub struct CarouselMultiItemPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    component_initialized: Cell<bool>,
}

user_control_class!(CarouselMultiItemPage);
ferro_class_info!(CarouselMultiItemPage {
    new: CarouselMultiItemPage::new,
    markup: {
        methods: [
            fn OnViewportFractionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselMultiItemPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<RangeBaseValueChangedEventArgs>() {
                        this.on_viewport_fraction_changed(&sender, e)
                    }
                },
            fn OnWrapChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselMultiItemPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_wrap_changed(&sender, e.as_routed_event_args())
                },
            fn OnSwipeChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselMultiItemPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_swipe_changed(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(CarouselMultiItemPage, "/Pages/CarouselPage/CarouselMultiItemPage.xaml");

impl CarouselMultiItemPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), component_initialized: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.component_initialized.set(true);

        // The handlers belong to children of the page: they hold the page weakly.
        let weak = this.downgrade();
        this.get_control::<Button>("PreviousButton").click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.carousel().previous();
            }
        });
        let weak = this.downgrade();
        this.get_control::<Button>("NextButton").click(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.carousel().next();
            }
        });
        let weak = this.downgrade();
        this.carousel().selection_changed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_selection_changed(sender, e);
            }
        });
        this
    }

    fn carousel(&self) -> Ref<Carousel> {
        self.get_control::<Carousel>("DemoCarousel")
    }

    /// The field `DemoCarousel`: null until `InitializeComponent()` has returned.
    fn demo_carousel(&self) -> Option<Ref<Carousel>> {
        self.component_initialized.get().then(|| self.carousel())
    }

    fn text(&self, name: &str) -> Ref<TextBlock> {
        self.get_control::<TextBlock>(name)
    }

    fn on_viewport_fraction_changed(&self, _sender: &Option<BoxedValue>, e: &RangeBaseValueChangedEventArgs) {
        let Some(demo_carousel) = self.demo_carousel() else {
            return;
        };

        // `Math.Round(value, 2)`: midpoints to even.
        let value = (e.new_value() * 100.0).round_ties_even() / 100.0;
        demo_carousel.set_viewport_fraction(value);
        self.text("ViewportLabel").set_text(Some(&format!("{value:.2}")));
        self.text("ViewportHint").set_text(Some(&if value >= 1.0 {
            String::from("1.00 \u{2014} single full item.")
        } else {
            format!("~{} items visible.", format_up_to_one_decimal(1.0 / value))
        }));
    }

    fn on_wrap_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(demo_carousel) = self.demo_carousel() else {
            return;
        };
        demo_carousel.set_wrap_selection(self.get_control::<CheckBox>("WrapCheck").is_checked() == Some(true));
    }

    fn on_swipe_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(demo_carousel) = self.demo_carousel() else {
            return;
        };
        demo_carousel.set_is_swipe_enabled(self.get_control::<CheckBox>("SwipeCheck").is_checked() == Some(true));
    }

    fn on_selection_changed(&self, _sender: &Interactive, _e: &SelectionChangedEventArgs) {
        let carousel = self.carousel();
        self.text("StatusText")
            .set_text(Some(&format!("Item: {} / {}", carousel.selected_index() + 1, carousel.item_count())));
    }
}
