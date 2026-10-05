//! Port of `Pages/CarouselPage/CarouselGalleryAppPage.xaml.cs`: the class of the document
//! `Pages/CarouselPage/CarouselGalleryAppPage.xaml`.

use crate::markup::xaml_class;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::{InputElementImpl, PointerCaptureLostEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs};
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, Point, Ref, StyledElementImpl, Visual, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    Carousel, ContentControlImpl, ControlImpl, ControlImplExt, DrawerPage, ListBox, PipsPager,
    PipsPagerSelectedIndexChangedEventArgs, ScrollViewer, SelectionChangedEventArgs, UserControl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

const SWIPE_THRESHOLD: f64 = 50.0;

#[repr(C)]
pub struct CarouselGalleryAppPage {
    base: UserControl,
    syncing: Cell<bool>,
    drag_start: Cell<Point>,
    is_dragging: Cell<bool>,
    info_panel: RefCell<Option<Ref<ScrollViewer>>>,
}

ferro_class!(CarouselGalleryAppPage: UserControl);
ferro_impl_classes!(
    CarouselGalleryAppPage: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(CarouselGalleryAppPage {
    new: CarouselGalleryAppPage::new,
    markup: {
        methods: [
            fn OnHamburgerClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselGalleryAppPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_hamburger_click(&sender, e.as_routed_event_args())
                },
            fn OnDrawerMenuSelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselGalleryAppPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_drawer_menu_selection_changed(&sender, e)
                    }
                },
            fn OnHeroPointerPressed(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselGalleryAppPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerPressedEventArgs>() {
                        this.on_hero_pointer_pressed(&sender, e)
                    }
                },
            fn OnHeroPointerReleased(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselGalleryAppPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerReleasedEventArgs>() {
                        this.on_hero_pointer_released(&sender, e)
                    }
                },
            fn OnHeroPointerCaptureLost(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CarouselGalleryAppPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<PointerCaptureLostEventArgs>() {
                        this.on_hero_pointer_capture_lost(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(CarouselGalleryAppPage, "/Pages/CarouselPage/CarouselGalleryAppPage.xaml");

impl ControlImpl for CarouselGalleryAppPage {
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);
        this.update_info_panel_visibility();
    }
}

impl FerroObjectImpl for CarouselGalleryAppPage {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Visual::bounds_property().as_property() {
            this.update_info_panel_visibility();
        }
    }
}

/// `(Visual?)sender`.
///
/// # Panics
/// Panics if the sender is not a visual (the invalid cast of the managed original).
fn sender_visual(sender: &Option<BoxedValue>) -> Option<Ref<Visual>> {
    sender.as_ref().map(|sender| {
        ValueTypes::as_object(&**sender).and_then(|sender| sender.cast::<Visual>()).expect("the sender is a visual")
    })
}

impl CarouselGalleryAppPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            syncing: Cell::new(false),
            drag_start: Cell::new(Point::default()),
            is_dragging: Cell::new(false),
            info_panel: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        *this.info_panel.borrow_mut() = this.find_control::<ScrollViewer>("InfoPanel");

        // The handlers belong to children of the page: they hold the page weakly.
        let weak = this.downgrade();
        this.hero_carousel().selection_changed(move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_hero_selection_changed(sender, e);
            }
        });
        let weak = this.downgrade();
        this.hero_pager().add_handler(PipsPager::selected_index_changed_event(), move |sender, e| {
            if let Some(this) = weak.upgrade() {
                this.on_pager_index_changed(sender, e);
            }
        });
        this
    }

    fn root_drawer(&self) -> Ref<DrawerPage> {
        self.get_control::<DrawerPage>("RootDrawer")
    }

    fn hero_carousel(&self) -> Ref<Carousel> {
        self.get_control::<Carousel>("HeroCarousel")
    }

    fn hero_pager(&self) -> Ref<PipsPager> {
        self.get_control::<PipsPager>("HeroPager")
    }

    fn update_info_panel_visibility(&self) {
        if let Some(info_panel) = self.info_panel.borrow().as_ref() {
            info_panel.set_is_visible(self.bounds().width >= 640.0);
        }
    }

    fn on_hamburger_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let root_drawer = self.root_drawer();
        root_drawer.set_is_open(!root_drawer.is_open());
    }

    fn on_hero_selection_changed(&self, _sender: &Interactive, _e: &SelectionChangedEventArgs) {
        if self.syncing.get() {
            return;
        }
        self.syncing.set(true);
        self.hero_pager().set_selected_page_index(self.hero_carousel().selected_index());
        self.syncing.set(false);
    }

    fn on_pager_index_changed(&self, _sender: &Interactive, e: &PipsPagerSelectedIndexChangedEventArgs) {
        if self.syncing.get() {
            return;
        }
        self.syncing.set(true);
        self.hero_carousel().set_selected_index(e.new_index());
        self.syncing.set(false);
    }

    fn on_drawer_menu_selection_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        self.root_drawer().set_is_open(false);
        self.get_control::<ListBox>("DrawerMenu").set_selected_item(None);
    }

    fn on_hero_pointer_pressed(&self, sender: &Option<BoxedValue>, e: &PointerPressedEventArgs) {
        if !e.get_current_point(None).properties.is_left_button_pressed {
            return;
        }
        let sender = sender_visual(sender);
        self.drag_start.set(e.get_position(sender.as_deref()));
        self.is_dragging.set(true);
    }

    fn on_hero_pointer_released(&self, sender: &Option<BoxedValue>, e: &PointerReleasedEventArgs) {
        if !self.is_dragging.get() {
            return;
        }
        self.is_dragging.set(false);
        let sender = sender_visual(sender);
        let delta = e.get_position(sender.as_deref()).x - self.drag_start.get().x;
        if delta.abs() < SWIPE_THRESHOLD {
            return;
        }
        if delta < 0.0 {
            self.hero_carousel().next();
        } else {
            self.hero_carousel().previous();
        }
    }

    fn on_hero_pointer_capture_lost(&self, _sender: &Option<BoxedValue>, _e: &PointerCaptureLostEventArgs) {
        self.is_dragging.set(false);
    }
}
