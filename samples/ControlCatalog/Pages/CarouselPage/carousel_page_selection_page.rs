//! Port of `Pages/CarouselPage/CarouselPageSelectionPage.xaml.cs`: the class of the document
//! `Pages/CarouselPage/CarouselPageSelectionPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::value_text;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{CarouselPage, ContentPage, PageSelectionChangedEventArgs, TextBlock, UserControl};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct CarouselPageSelectionPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    component_initialized: Cell<bool>,
}

user_control_class!(CarouselPageSelectionPage);
ferro_class_info!(CarouselPageSelectionPage {
    new: CarouselPageSelectionPage::new,
    markup: {
        methods: [
            fn OnGoTo0(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageSelectionPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_go_to0(&sender, e.as_routed_event_args())
                },
            fn OnGoTo1(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageSelectionPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_go_to1(&sender, e.as_routed_event_args())
                },
            fn OnGoTo2(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageSelectionPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_go_to2(&sender, e.as_routed_event_args())
                },
            fn OnGoTo3(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageSelectionPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_go_to3(&sender, e.as_routed_event_args())
                },
            fn OnFirst(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageSelectionPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_first(&sender, e.as_routed_event_args())
                },
            fn OnPrevious(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageSelectionPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_previous(&sender, e.as_routed_event_args())
                },
            fn OnNext(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageSelectionPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_next(&sender, e.as_routed_event_args())
                },
            fn OnLast(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageSelectionPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_last(&sender, e.as_routed_event_args())
                },
            fn OnSelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageSelectionPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PageSelectionChangedEventArgs>() {
                        this.on_selection_changed(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(CarouselPageSelectionPage, "/Pages/CarouselPage/CarouselPageSelectionPage.xaml");

impl CarouselPageSelectionPage {
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

    fn on_go_to0(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.carousel().set_selected_index(0);
    }

    fn on_go_to1(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.carousel().set_selected_index(1);
    }

    fn on_go_to2(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.carousel().set_selected_index(2);
    }

    fn on_go_to3(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.carousel().set_selected_index(3);
    }

    fn on_first(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.carousel().set_selected_index(0);
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

    fn on_last(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let page_count = self.page_count();
        if page_count > 0 {
            self.carousel().set_selected_index(page_count - 1);
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
        let header = demo_carousel
            .selected_page()
            .and_then(|page| page.cast::<ContentPage>())
            .and_then(|page| value_text(&page.header()))
            .unwrap_or_else(|| String::from("\u{2014}"));
        status_text.set_text(Some(&format!(
            "Page {} of {}: {header}",
            demo_carousel.selected_index() + 1,
            self.page_count()
        )));
    }
}
