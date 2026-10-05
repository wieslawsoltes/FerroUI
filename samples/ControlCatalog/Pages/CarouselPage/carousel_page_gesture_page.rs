//! Port of `Pages/CarouselPage/CarouselPageGesturePage.xaml.cs`: the class of the document
//! `Pages/CarouselPage/CarouselPageGesturePage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{CarouselPage, CheckBox, PageSelectionChangedEventArgs, TextBlock, UserControl};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct CarouselPageGesturePage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    component_initialized: Cell<bool>,
}

user_control_class!(CarouselPageGesturePage);
ferro_class_info!(CarouselPageGesturePage {
    new: CarouselPageGesturePage::new,
    markup: {
        methods: [
            fn OnGestureChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageGesturePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_gesture_changed(&sender, e.as_routed_event_args())
                },
            fn OnKeyboardChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageGesturePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_keyboard_changed(&sender, e.as_routed_event_args())
                },
            fn OnSelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageGesturePage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PageSelectionChangedEventArgs>() {
                        this.on_selection_changed(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(CarouselPageGesturePage, "/Pages/CarouselPage/CarouselPageGesturePage.xaml");

impl CarouselPageGesturePage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), component_initialized: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.component_initialized.set(true);

        // The handler of an event of the page itself holds it weakly.
        let weak = this.downgrade();
        this.loaded(move |_, _| {
            if let Some(this) = weak.upgrade() {
                this.update_status();
            }
        });
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

    fn on_gesture_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if !self.component_initialized.get() {
            return;
        }
        self.carousel()
            .set_is_gesture_enabled(self.get_control::<CheckBox>("GestureCheck").is_checked() == Some(true));
    }

    fn on_keyboard_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if !self.component_initialized.get() {
            return;
        }
        self.carousel().set_is_keyboard_navigation_enabled(
            self.get_control::<CheckBox>("KeyboardCheck").is_checked() == Some(true),
        );
    }

    fn on_selection_changed(&self, _sender: &Option<BoxedValue>, _e: &PageSelectionChangedEventArgs) {
        self.update_status();
    }

    fn update_status(&self) {
        let Some(status_text) = self.status_text() else {
            return;
        };
        status_text.set_text(Some(&format!(
            "Page {} of {}",
            self.carousel().selected_index() + 1,
            self.page_count()
        )));
    }
}
