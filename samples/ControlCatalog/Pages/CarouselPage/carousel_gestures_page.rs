//! Port of `Pages/CarouselPage/CarouselGesturesPage.xaml.cs`: the class of the document
//! `Pages/CarouselPage/CarouselGesturesPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::input::{InputElement, Key, KeyEventArgs};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{Carousel, CheckBox, SelectionChangedEventArgs, TextBlock, UserControl};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct CarouselGesturesPage {
    base: UserControl,
    keyboard_enabled: Cell<bool>,
}

user_control_class!(CarouselGesturesPage);
ferro_class_info!(CarouselGesturesPage {
    new: CarouselGesturesPage::new,
    markup: {
        methods: [
            fn OnSwipeEnabledChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselGesturesPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_swipe_enabled_changed(&sender, e.as_routed_event_args())
                },
            fn OnWrapSelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselGesturesPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_wrap_selection_changed(&sender, e.as_routed_event_args())
                },
            fn OnKeyboardEnabledChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselGesturesPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_keyboard_enabled_changed(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(CarouselGesturesPage, "/Pages/CarouselPage/CarouselGesturesPage.xaml");

impl CarouselGesturesPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), keyboard_enabled: Cell::new(true) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        // The handlers belong to a child of the page: they hold the page weakly.
        let demo_carousel = this.demo_carousel();
        let weak = this.downgrade();
        demo_carousel.add_handler_with(
            InputElement::key_down_event(),
            move |sender, e| {
                if let Some(this) = weak.upgrade() {
                    this.on_key_down(sender, e);
                }
            },
            Interactive::DEFAULT_ROUTES,
            true,
        );
        let weak = this.downgrade();
        demo_carousel.selection_changed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_selection_changed(sender, e);
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

    fn check(&self, name: &str) -> Ref<CheckBox> {
        self.get_control::<CheckBox>(name)
    }

    fn last_action_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("LastActionText")
    }

    fn on_key_down(&self, _sender: &Interactive, e: &KeyEventArgs) {
        if !self.keyboard_enabled.get() {
            return;
        }

        match e.key {
            Key::Left | Key::Up => {
                self.last_action_text().set_text(Some(&format!("Action: Key {:?} (Previous)", e.key)));
            }
            Key::Right | Key::Down => {
                self.last_action_text().set_text(Some(&format!("Action: Key {:?} (Next)", e.key)));
            }
            _ => {}
        }
    }

    fn on_selection_changed(&self, _sender: &Interactive, _e: &SelectionChangedEventArgs) {
        let demo_carousel = self.demo_carousel();
        self.get_control::<TextBlock>("StatusText").set_text(Some(&format!(
            "Item: {} / {}",
            demo_carousel.selected_index() + 1,
            demo_carousel.item_count()
        )));
        if demo_carousel.is_swiping() {
            self.last_action_text().set_text(Some("Action: Swipe"));
        }
    }

    fn on_swipe_enabled_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.demo_carousel().set_is_swipe_enabled(self.check("SwipeCheck").is_checked() == Some(true));
    }

    fn on_wrap_selection_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.demo_carousel().set_wrap_selection(self.check("WrapCheck").is_checked() == Some(true));
    }

    fn on_keyboard_enabled_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.keyboard_enabled.set(self.check("KeyboardCheck").is_checked() == Some(true));
    }
}
