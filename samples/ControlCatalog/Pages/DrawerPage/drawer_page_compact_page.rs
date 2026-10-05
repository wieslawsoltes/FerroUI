//! Port of `Pages/DrawerPage/DrawerPageCompactPage.xaml.cs`: the class of the document
//! `Pages/DrawerPage/DrawerPageCompactPage.xaml`.

use crate::markup::xaml_class;
use crate::pages::navigation_demo_helper::value_text;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, InteractiveImpl, RoutedEventArgs, RoutedEventHandlerToken};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::primitives::{RangeBaseValueChangedEventArgs, TemplatedControlImpl};
use ferroui_controls::{
    Button, ComboBox, ContentControlImpl, ContentPage, ControlImpl, ControlImplExt, DrawerLayoutBehavior, DrawerPage,
    SelectionChangedEventArgs, TextBlock, UserControl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
pub struct DrawerPageCompactPage {
    base: UserControl,
    is_loaded: Cell<bool>,
    /// The subscriptions `OnLoaded` adds and `OnUnloaded` removes: the opened and the closed
    /// event of the drawer.
    drawer_handlers: RefCell<Vec<[RoutedEventHandlerToken; 2]>>,
}

ferro_class!(DrawerPageCompactPage: UserControl);
ferro_impl_classes!(
    DrawerPageCompactPage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(DrawerPageCompactPage {
    new: DrawerPageCompactPage::new,
    markup: {
        methods: [
            fn OnLayoutChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageCompactPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_layout_changed(&sender, e)
                    }
                },
            fn OnCompactLengthChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageCompactPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<RangeBaseValueChangedEventArgs>() {
                        this.on_compact_length_changed(&sender, e)
                    }
                },
            fn OnDrawerLengthChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageCompactPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<RangeBaseValueChangedEventArgs>() {
                        this.on_drawer_length_changed(&sender, e)
                    }
                },
            fn OnMenuItemClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageCompactPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_menu_item_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(DrawerPageCompactPage, "/Pages/DrawerPage/DrawerPageCompactPage.xaml");

impl ControlImpl for DrawerPageCompactPage {
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);
        this.is_loaded.set(true);

        // The handlers belong to a child of the page: they hold the page weakly.
        let demo_drawer = this.demo_drawer();
        let handler = || {
            let weak = this.to_ref().downgrade();
            move |sender: &Interactive, e: &RoutedEventArgs| {
                if let Some(this) = weak.upgrade() {
                    this.on_drawer_status_changed(sender, e);
                }
            }
        };
        this.drawer_handlers.borrow_mut().push([demo_drawer.opened(handler()), demo_drawer.closed(handler())]);
    }

    fn on_unloaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_unloaded(this, e);

        // One subscription of each handler is removed, as `-=` removes one.
        let handlers = this.drawer_handlers.borrow_mut().pop();
        if let Some([opened, closed]) = handlers {
            let demo_drawer = this.demo_drawer();
            demo_drawer.remove_handler(DrawerPage::opened_event(), opened);
            demo_drawer.remove_handler(DrawerPage::closed_event(), closed);
        }
    }
}

/// `sender as Button`.
fn as_button(sender: &Option<BoxedValue>) -> Option<Ref<Button>> {
    sender.as_ref().and_then(|sender| ValueTypes::as_object(&**sender)).and_then(|sender| sender.cast::<Button>())
}

impl DrawerPageCompactPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), is_loaded: Cell::new(false), drawer_handlers: RefCell::new(Vec::new()) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn layout_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("LayoutCombo")
    }

    fn compact_length_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("CompactLengthText")
    }

    fn drawer_length_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("DrawerLengthText")
    }

    fn status_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("StatusText")
    }

    fn demo_drawer(&self) -> Ref<DrawerPage> {
        self.get_control::<DrawerPage>("DemoDrawer")
    }

    fn detail_page(&self) -> Ref<ContentPage> {
        self.get_control::<ContentPage>("DetailPage")
    }

    fn detail_title_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("DetailTitleText")
    }

    fn on_drawer_status_changed(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        self.update_status();
    }

    fn on_layout_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        if !self.is_loaded.get() {
            return;
        }
        self.demo_drawer().set_drawer_layout_behavior(match self.layout_combo().selected_index() {
            0 => DrawerLayoutBehavior::CompactOverlay,
            1 => DrawerLayoutBehavior::CompactInline,
            _ => DrawerLayoutBehavior::CompactOverlay,
        });
    }

    fn on_compact_length_changed(&self, _sender: &Option<BoxedValue>, e: &RangeBaseValueChangedEventArgs) {
        if !self.is_loaded.get() {
            return;
        }
        self.demo_drawer().set_compact_drawer_length(e.new_value());
        self.compact_length_text().set_text(Some(&(e.new_value() as i32).to_string()));
    }

    fn on_drawer_length_changed(&self, _sender: &Option<BoxedValue>, e: &RangeBaseValueChangedEventArgs) {
        if !self.is_loaded.get() {
            return;
        }
        self.demo_drawer().set_drawer_length(e.new_value());
        self.drawer_length_text().set_text(Some(&(e.new_value() as i32).to_string()));
    }

    fn on_menu_item_click(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if !self.is_loaded.get() {
            return;
        }
        let Some(button) = as_button(sender) else {
            return;
        };
        let item = value_text(&button.tag()).unwrap_or_else(|| String::from("Home"));
        self.detail_title_text().set_text(Some(&item));
        self.detail_page().set_header(Some(Rc::new(item) as BoxedValue));
        self.demo_drawer().set_is_open(false);
    }

    fn update_status(&self) {
        self.status_text()
            .set_text(Some(&format!("Drawer: {}", if self.demo_drawer().is_open() { "Open" } else { "Closed" })));
    }
}
