//! Port of `Pages/CommandBar/CommandBarFirstLookPage.xaml.cs`: the class of the document
//! `Pages/CommandBar/CommandBarFirstLookPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, ObjectType, Ref};
use ferroui_base::data::core::ValueTypes;
use ferroui_controls::{CommandBarButton, CommandBarToggleButton, TextBlock, UserControl};
use std::rc::Rc;

/// `sender as T`.
fn sender_as<T: ObjectType>(sender: &Option<BoxedValue>) -> Option<Ref<T>> {
    sender.as_ref().and_then(|sender| ValueTypes::as_object(&**sender)).and_then(|sender| sender.cast::<T>())
}

#[repr(C)]
pub struct CommandBarFirstLookPage {
    base: UserControl,
}

user_control_class!(CommandBarFirstLookPage);
ferro_class_info!(CommandBarFirstLookPage {
    new: CommandBarFirstLookPage::new,
    markup: {
        methods: [
            fn OnButtonClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarFirstLookPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_button_click(&sender, e.as_routed_event_args())
                },
            fn OnToggleChanged(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<CommandBarFirstLookPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_toggle_changed(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(CommandBarFirstLookPage, "/Pages/CommandBar/CommandBarFirstLookPage.xaml");

impl CommandBarFirstLookPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn status_text(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("StatusText")
    }

    fn on_button_click(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(btn) = sender_as::<CommandBarButton>(sender) {
            self.status_text().set_text(Some(&format!("{} clicked", btn.label().unwrap_or_default())));
        }
    }

    fn on_toggle_changed(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        if let Some(btn) = sender_as::<CommandBarToggleButton>(sender) {
            let label = btn.label().unwrap_or_default();
            self.status_text().set_text(Some(&if btn.is_checked() == Some(true) {
                format!("{label} enabled")
            } else {
                format!("{label} disabled")
            }));
        }
    }
}
