//! Port of `Pages/DrawerPage/DrawerPageFirstLookPage.xaml.cs`: the class of the document
//! `Pages/DrawerPage/DrawerPageFirstLookPage.xaml`.

use crate::markup::xaml_class;
use crate::pages::navigation_demo_helper::value_text;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::gesture_recognizers::SwipeGestureRecognizer;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, Interactive, InteractiveImpl, RoutedEventArgs, RoutedEventHandlerToken};
use ferroui_base::layout::{HorizontalAlignment, LayoutableImpl, VerticalAlignment};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    CheckBox, ContentControlImpl, ContentPage, Control, ControlImpl, ControlImplExt, DrawerPage, ListBox, ListBoxItem,
    SelectionChangedEventArgs, TextBlock, UserControl,
};
use std::cell::RefCell;
use std::rc::Rc;

#[repr(C)]
pub struct DrawerPageFirstLookPage {
    base: UserControl,
    /// The subscriptions `OnLoaded` adds and `OnUnloaded` removes: the opened and the closed
    /// event of the drawer.
    drawer_handlers: RefCell<Vec<[RoutedEventHandlerToken; 2]>>,
}

ferro_class!(DrawerPageFirstLookPage: UserControl);
ferro_impl_classes!(
    DrawerPageFirstLookPage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(DrawerPageFirstLookPage {
    new: DrawerPageFirstLookPage::new,
    markup: {
        methods: [
            fn OnToggleDrawer(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageFirstLookPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_toggle_drawer(&sender, e.as_routed_event_args())
                },
            fn OnGestureChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageFirstLookPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_gesture_changed(&sender, e.as_routed_event_args())
                },
            fn OnMenuSelectionChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageFirstLookPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_menu_selection_changed(&sender, e)
                    }
                },
        ],
    },
});
xaml_class!(DrawerPageFirstLookPage, "/Pages/DrawerPage/DrawerPageFirstLookPage.xaml");

impl ControlImpl for DrawerPageFirstLookPage {
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);

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

impl DrawerPageFirstLookPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), drawer_handlers: RefCell::new(Vec::new()) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        Self::enable_mouse_swipe_gesture(&this.demo_drawer());
        this
    }

    fn gesture_check(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("GestureCheck")
    }

    fn status_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("StatusText")
    }

    fn demo_drawer(&self) -> Ref<DrawerPage> {
        self.get_control::<DrawerPage>("DemoDrawer")
    }

    fn drawer_menu(&self) -> Ref<ListBox> {
        self.get_control::<ListBox>("DrawerMenu")
    }

    fn on_drawer_status_changed(&self, _sender: &Interactive, _e: &RoutedEventArgs) {
        self.update_status();
    }

    fn on_toggle_drawer(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let demo_drawer = self.demo_drawer();
        demo_drawer.set_is_open(!demo_drawer.is_open());
    }

    fn on_gesture_changed(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.demo_drawer().set_is_gesture_enabled(self.gesture_check().is_checked() == Some(true));
    }

    fn on_menu_selection_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        let item = self
            .drawer_menu()
            .selected_item()
            .and_then(|item| ValueTypes::as_object(&*item))
            .and_then(|item| item.cast::<ListBoxItem>());
        if let Some(item) = item {
            let content = value_text(&item.content());

            let text = TextBlock::new();
            text.set_text(Some(&format!("{} page content", content.clone().unwrap_or_default())));
            text.set_font_size(16.0);
            text.set_horizontal_alignment(HorizontalAlignment::Center);
            text.set_vertical_alignment(VerticalAlignment::Center);

            let page = ContentPage::new();
            page.set_header(content.map(|content| Rc::new(content) as BoxedValue));
            page.set_content(Some(Control::boxed(text)));
            page.set_horizontal_content_alignment(HorizontalAlignment::Stretch);
            page.set_vertical_content_alignment(VerticalAlignment::Stretch);

            let demo_drawer = self.demo_drawer();
            demo_drawer.set_content(Some(Control::boxed(page)));
            demo_drawer.set_is_open(false);
        }
    }

    fn update_status(&self) {
        self.status_text()
            .set_text(Some(&format!("Drawer: {}", if self.demo_drawer().is_open() { "Open" } else { "Closed" })));
    }

    fn enable_mouse_swipe_gesture(control: &Control) {
        let recognizer = control
            .gesture_recognizers()
            .to_vec()
            .into_iter()
            .find_map(|recognizer| recognizer.cast::<SwipeGestureRecognizer>());
        if let Some(recognizer) = recognizer {
            recognizer.set_is_mouse_enabled(true);
        }
    }
}
