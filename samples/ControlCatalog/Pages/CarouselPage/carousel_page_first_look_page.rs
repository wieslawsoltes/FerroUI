//! Port of `Pages/CarouselPage/CarouselPageFirstLookPage.xaml.cs`: the class of the document
//! `Pages/CarouselPage/CarouselPageFirstLookPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{CarouselPage, PageSelectionChangedEventArgs, TextBlock, UserControl};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct CarouselPageFirstLookPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    component_initialized: Cell<bool>,
}

user_control_class!(CarouselPageFirstLookPage);
ferro_class_info!(CarouselPageFirstLookPage {
    new: CarouselPageFirstLookPage::new,
    markup: {
        methods: [
            fn OnPrevious(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageFirstLookPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_previous(&sender, e.as_routed_event_args())
                },
            fn OnNext(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageFirstLookPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_next(&sender, e.as_routed_event_args())
                },
            fn OnSelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageFirstLookPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PageSelectionChangedEventArgs>() {
                        this.on_selection_changed(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(CarouselPageFirstLookPage, "/Pages/CarouselPage/CarouselPageFirstLookPage.xaml");

impl CarouselPageFirstLookPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), component_initialized: Cell::new(false) }
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
