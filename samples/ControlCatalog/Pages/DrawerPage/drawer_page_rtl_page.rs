//! Port of `Pages/DrawerPage/DrawerPageRtlPage.xaml.cs`: the class of the document
//! `Pages/DrawerPage/DrawerPageRtlPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::navigation_demo_helper::value_text;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::media::FlowDirection;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{
    Button, CheckBox, ComboBox, DrawerPage, DrawerPlacement, SelectionChangedEventArgs, TextBlock, UserControl,
};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct DrawerPageRtlPage {
    base: UserControl,
    /// Whether `InitializeComponent()` has returned: the fields of the named elements are
    /// null until then, which the handlers that run while the document loads test for.
    component_initialized: Cell<bool>,
}

user_control_class!(DrawerPageRtlPage);
ferro_class_info!(DrawerPageRtlPage {
    new: DrawerPageRtlPage::new,
    markup: {
        methods: [
            fn OnRtlToggled(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageRtlPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_rtl_toggled(&sender, e.as_routed_event_args())
                },
            fn OnPlacementChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageRtlPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    if let Some(e) = e.downcast_ref::<SelectionChangedEventArgs>() {
                        this.on_placement_changed(&sender, e)
                    }
                },
            fn OnToggleDrawer(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageRtlPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_toggle_drawer(&sender, e.as_routed_event_args())
                },
            fn OnMenuItemClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<DrawerPageRtlPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_menu_item_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(DrawerPageRtlPage, "/Pages/DrawerPage/DrawerPageRtlPage.xaml");

/// `sender as Button`.
fn as_button(sender: &Option<BoxedValue>) -> Option<Ref<Button>> {
    sender.as_ref().and_then(|sender| ValueTypes::as_object(&**sender)).and_then(|sender| sender.cast::<Button>())
}

impl DrawerPageRtlPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct(), component_initialized: Cell::new(false) }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.component_initialized.set(true);
        this
    }

    /// The field `DemoDrawer`: null until `InitializeComponent()` has returned.
    fn demo_drawer(&self) -> Option<Ref<DrawerPage>> {
        self.component_initialized.get().then(|| self.get_control::<DrawerPage>("DemoDrawer"))
    }

    fn rtl_check_box(&self) -> Ref<CheckBox> {
        self.get_control::<CheckBox>("RtlCheckBox")
    }

    fn placement_combo(&self) -> Ref<ComboBox> {
        self.get_control::<ComboBox>("PlacementCombo")
    }

    fn detail_title_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("DetailTitleText")
    }

    fn detail_description_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("DetailDescriptionText")
    }

    fn on_rtl_toggled(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(demo_drawer) = self.demo_drawer() else {
            return;
        };
        demo_drawer.set_flow_direction(if self.rtl_check_box().is_checked() == Some(true) {
            FlowDirection::RightToLeft
        } else {
            FlowDirection::LeftToRight
        });
    }

    fn on_placement_changed(&self, _sender: &Option<BoxedValue>, _e: &SelectionChangedEventArgs) {
        let Some(demo_drawer) = self.demo_drawer() else {
            return;
        };
        demo_drawer.set_drawer_placement(match self.placement_combo().selected_index() {
            0 => DrawerPlacement::Left,
            1 => DrawerPlacement::Right,
            _ => DrawerPlacement::Left,
        });
    }

    fn on_toggle_drawer(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(demo_drawer) = self.demo_drawer() else {
            return;
        };
        demo_drawer.set_is_open(!demo_drawer.is_open());
    }

    /// # Panics
    /// Panics if it runs before the document is loaded (a null reference in the managed
    /// original).
    fn on_menu_item_click(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let Some(button) = as_button(sender) else {
            return;
        };
        let item = value_text(&button.tag()).unwrap_or_else(|| String::from("Home"));
        self.detail_title_text().set_text(Some(&item));
        self.detail_description_text().set_text(Some(&match item.as_str() {
            "Home" => String::from(
                "Toggle RTL to see the drawer flip to the right edge.\nGestures are mirrored: drag from right edge to open, drag right to close.",
            ),
            "Profile" => String::from("View and edit your profile information here."),
            "Messages" => String::from("Your messages and notifications appear here."),
            "Settings" => String::from("Configure application preferences and options."),
            _ => format!("Content for {item}"),
        }));
        self.demo_drawer().expect("the drawer of the page").set_is_open(false);
    }
}
