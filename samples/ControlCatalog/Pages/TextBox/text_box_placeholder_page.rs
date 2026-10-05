//! Port of `Pages/TextBox/TextBoxPlaceholderPage.xaml.cs`: the class of the
//! document `Pages/TextBox/TextBoxPlaceholderPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{TextBox, UserControl};
use std::rc::Rc;

#[repr(C)]
pub struct TextBoxPlaceholderPage {
    base: UserControl,
}

user_control_class!(TextBoxPlaceholderPage);
ferro_class_info!(TextBoxPlaceholderPage {
    new: TextBoxPlaceholderPage::new,
    markup: {
        methods: [
            fn OnClearInnerContentBox(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<TextBoxPlaceholderPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_clear_inner_content_box(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(TextBoxPlaceholderPage, "/Pages/TextBox/TextBoxPlaceholderPage.xaml");

impl TextBoxPlaceholderPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn inner_content_box(&self) -> Ref<TextBox> {
        self.get_control::<TextBox>("InnerContentBox")
    }

    fn on_clear_inner_content_box(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.inner_content_box().clear();
        self.inner_content_box().focus();
    }
}
