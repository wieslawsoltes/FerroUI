//! Port of `Pages/DrawerPage/DrawerPageBreakpointPage.xaml.cs`: the class of the document
//! `Pages/DrawerPage/DrawerPageBreakpointPage.xaml`.

use crate::markup::xaml_class;
use crate::pages::navigation_demo_helper::value_text;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl,
    FerroPropertyChangedEventArgs, Ref, StyledElementImpl, Visual, VisualImpl,
};
use ferroui_controls::primitives::{RangeBaseValueChangedEventArgs, TemplatedControlImpl};
use ferroui_controls::{
    Button, ComboBox, ContentControlImpl, ContentPage, ControlImpl, ControlImplExt, DrawerLayoutBehavior, DrawerPage,
    DrawerPlacement, SelectionChangedEventArgs, TextBlock, UserControl,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[repr(C)]
pub struct DrawerPageBreakpointPage {
    base: UserControl,
    is_loaded: Cell<bool>,
    /// The subscriptions `OnLoaded` adds and `OnUnloaded` removes.
    drawer_property_changed: RefCell<Vec<Rc<dyn IDisposable>>>,
}

ferro_class!(DrawerPageBreakpointPage: UserControl);
ferro_impl_classes!(
    DrawerPageBreakpointPage: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(DrawerPageBreakpointPage {
    new: DrawerPageBreakpointPage::new,
    markup: {
        methods: [
            fn OnBreakpointChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageBreakpointPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<RangeBaseValueChangedEventArgs>() {
                        this.on_breakpoint_changed(&sender, e)
                    }
                },
            fn OnLayoutChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageBreakpointPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_layout_changed(&sender, e)
                    }
                },
            fn OnMenuItemClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageBreakpointPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_menu_item_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(DrawerPageBreakpointPage, "/Pages/DrawerPage/DrawerPageBreakpointPage.xaml");

impl ControlImpl for DrawerPageBreakpointPage {
    fn on_loaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_loaded(this, e);
        this.is_loaded.set(true);
        // The handler belongs to a child of the page: it holds the page weakly.
        let weak = this.to_ref().downgrade();
        this.drawer_property_changed.borrow_mut().push(this.demo_drawer().property_changed(move |e| {
            if let Some(this) = weak.upgrade() {
                this.on_drawer_property_changed(e);
            }
        }));
        this.update_status();
    }

    fn on_unloaded(this: &Self, e: &RoutedEventArgs) {
        Self::parent_on_unloaded(this, e);
        // One subscription is removed, as `-=` removes one.
        let property_changed = this.drawer_property_changed.borrow_mut().pop();
        if let Some(property_changed) = property_changed {
            property_changed.dispose();
        }
    }
}

/// `sender as Button`.
fn as_button(sender: &Option<BoxedValue>) -> Option<Ref<Button>> {
    sender.as_ref().and_then(|sender| ValueTypes::as_object(&**sender)).and_then(|sender| sender.cast::<Button>())
}

impl DrawerPageBreakpointPage {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            is_loaded: Cell::new(false),
            drawer_property_changed: RefCell::new(Vec::new()),
        }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn breakpoint_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("BreakpointText")
    }

    fn layout_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("LayoutCombo")
    }

    fn width_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("WidthText")
    }

    fn mode_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("ModeText")
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

    fn on_drawer_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if e.property() == Visual::bounds_property().as_property() {
            self.update_status();
        }
    }

    fn on_breakpoint_changed(&self, _sender: &Option<BoxedValue>, e: &RangeBaseValueChangedEventArgs) {
        if !self.is_loaded.get() {
            return;
        }
        let value = e.new_value() as i32;
        self.demo_drawer().set_drawer_breakpoint_length(f64::from(value));
        self.breakpoint_text().set_text(Some(&value.to_string()));
        self.update_status();
    }

    fn on_layout_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        if !self.is_loaded.get() {
            return;
        }
        self.demo_drawer().set_drawer_layout_behavior(match self.layout_combo().selected_index() {
            0 => DrawerLayoutBehavior::Split,
            1 => DrawerLayoutBehavior::CompactInline,
            2 => DrawerLayoutBehavior::CompactOverlay,
            _ => DrawerLayoutBehavior::Split,
        });
        self.update_status();
    }

    fn on_menu_item_click(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(button) = as_button(sender).filter(|_| self.is_loaded.get()) else {
            return;
        };
        let item = value_text(&button.tag()).unwrap_or_else(|| String::from("Home"));
        self.detail_title_text().set_text(Some(&item));
        self.detail_page().set_header(Some(Rc::new(item) as BoxedValue));
        let demo_drawer = self.demo_drawer();
        if demo_drawer.drawer_layout_behavior() != DrawerLayoutBehavior::Split {
            demo_drawer.set_is_open(false);
        }
    }

    fn update_status(&self) {
        let demo_drawer = self.demo_drawer();
        let is_vertical = demo_drawer.drawer_placement() == DrawerPlacement::Top
            || demo_drawer.drawer_placement() == DrawerPlacement::Bottom;
        let length = if is_vertical { demo_drawer.bounds().height } else { demo_drawer.bounds().width };
        let breakpoint = demo_drawer.drawer_breakpoint_length();
        self.width_text().set_text(Some(&format!(
            "{}: {} px",
            if is_vertical { "Height" } else { "Width" },
            length as i32
        )));
        let is_overlay = breakpoint > 0.0 && length > 0.0 && length < breakpoint;
        self.mode_text().set_text(Some(&if is_overlay {
            String::from("Mode: Overlay (below breakpoint)")
        } else {
            format!("Mode: {:?} (above breakpoint)", demo_drawer.drawer_layout_behavior())
        }));
    }
}
