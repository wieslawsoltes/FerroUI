//! Port of `Pages/MenuPage.xaml.cs`: the class of the document `Pages/MenuPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{MenuItem, TextBlock, UserControl};
use std::rc::Rc;

#[repr(C)]
pub struct MenuPage {
    base: UserControl,
}

user_control_class!(MenuPage);
ferro_class_info!(MenuPage {
    new: MenuPage::new,
    markup: {
        methods: [
            fn MenuClicked(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<MenuPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.menu_clicked(&sender, e.as_routed_event_args())
                },
            fn MenuClickedMenuItemReset_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<MenuPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.menu_clicked_menu_item_reset_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(MenuPage, "/Pages/MenuPage.xaml");

impl MenuPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn clicked_menu_item(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("ClickedMenuItem")
    }

    fn menu_clicked(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let clicked_menu_item_text_block = self.clicked_menu_item();
        // `(sender as MenuItem)?.Header?.ToString()`: the headers of the items of the page are
        // texts.
        let header = sender
            .as_ref()
            .and_then(|sender| ValueTypes::as_object(&**sender))
            .and_then(|sender| sender.cast::<MenuItem>())
            .and_then(|menu_item| menu_item.header())
            .and_then(|header| from_markup_value::<String>(&Some(header)));
        clicked_menu_item_text_block.set_text(header.as_deref());
    }

    fn menu_clicked_menu_item_reset_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.clicked_menu_item().set_text(Some("None"));
    }
}
