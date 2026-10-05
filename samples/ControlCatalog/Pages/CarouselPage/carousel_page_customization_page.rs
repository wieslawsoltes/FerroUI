//! Port of `Pages/CarouselPage/CarouselPageCustomizationPage.xaml.cs`: the class of the document
//! `Pages/CarouselPage/CarouselPageCustomizationPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::animation::{PageSlide, SlideAxis, TimeSpan};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, RoutedEventArgs, RoutedEventHandlerToken};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{
    CarouselPage, ComboBox, Control, PageSelectionChangedEventArgs, SelectingMultiPage, SelectionChangedEventArgs,
    TextBlock, UserControl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
pub struct CarouselPageCustomizationPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    component_initialized: Cell<bool>,
    /// The subscriptions `OnLoaded` adds and `OnUnloaded` removes.
    selection_changed: RefCell<Vec<RoutedEventHandlerToken>>,
}

user_control_class!(CarouselPageCustomizationPage);
ferro_class_info!(CarouselPageCustomizationPage {
    new: CarouselPageCustomizationPage::new,
    markup: {
        methods: [
            fn OnOrientationChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_orientation_changed(&sender, e)
                    }
                },
            fn OnPrevious(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_previous(&sender, e.as_routed_event_args())
                },
            fn OnNext(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselPageCustomizationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_next(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(CarouselPageCustomizationPage, "/Pages/CarouselPage/CarouselPageCustomizationPage.xaml");

impl CarouselPageCustomizationPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            component_initialized: Cell::new(false),
            selection_changed: RefCell::new(Vec::new()),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.component_initialized.set(true);

        // The handlers of the events of the page itself hold it weakly.
        let weak = this.downgrade();
        this.loaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_loaded(sender, e);
            }
        });
        let weak = this.downgrade();
        this.unloaded(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_unloaded(sender, e);
            }
        });
        this
    }

    fn carousel(&self) -> Ref<CarouselPage> {
        self.get_control::<CarouselPage>("DemoCarousel")
    }

    fn orientation_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("OrientationCombo")
    }

    /// `(DemoCarousel.Pages as IList)?.Count ?? 0`.
    fn page_count(&self) -> i32 {
        self.carousel().pages().map_or(0, |pages| pages.count() as i32)
    }

    fn on_loaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        let demo_carousel = self.carousel();
        demo_carousel.set_page_transition(Some(Rc::new(PageSlide::with_duration(
            TimeSpan::from_milliseconds(300.0),
            SlideAxis::Horizontal,
        ))));
        // The handler belongs to a child of the page: it holds the page weakly.
        let weak = self.to_ref().downgrade();
        self.selection_changed.borrow_mut().push(demo_carousel.selection_changed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_selection_changed(sender, e);
            }
        }));
        self.update_dots(demo_carousel.selected_index());
        self.update_status();
    }

    fn on_unloaded(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        // One subscription is removed, as `-=` removes one.
        let token = self.selection_changed.borrow_mut().pop();
        if let Some(token) = token {
            self.carousel().remove_handler(SelectingMultiPage::selection_changed_event(), token);
        }
    }

    fn on_selection_changed(&self, _sender: &Interactive, _e: &PageSelectionChangedEventArgs) {
        self.update_dots(self.carousel().selected_index());
        self.update_status();
    }

    fn on_orientation_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        if !self.component_initialized.get() {
            return;
        }

        let axis = if self.orientation_combo().selected_index() == 1 { SlideAxis::Vertical } else { SlideAxis::Horizontal };
        self.carousel()
            .set_page_transition(Some(Rc::new(PageSlide::with_duration(TimeSpan::from_milliseconds(300.0), axis))));
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

    fn update_dots(&self, selected_index: i32) {
        for (index, name) in ["Dot0", "Dot1", "Dot2", "Dot3"].into_iter().enumerate() {
            self.get_control::<Control>(name).set_opacity(if selected_index == index as i32 { 1.0 } else { 0.4 });
        }
    }

    fn update_status(&self) {
        if !self.component_initialized.get() {
            return;
        }
        let axis = if self.orientation_combo().selected_index() == 1 { "Vertical" } else { "Horizontal" };
        self.get_control::<TextBlock>("StatusText").set_text(Some(&format!(
            "Page {} of {} | {axis}",
            self.carousel().selected_index() + 1,
            self.page_count()
        )));
    }
}
