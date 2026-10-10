//! Port of `Pages/AutomationPage.xaml.cs`: the class of the document
//! `Pages/AutomationPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{TextBlock, UserControl};
use std::rc::Rc;

#[repr(C)]
pub struct AutomationPage {
    base: UserControl,
}

user_control_class!(AutomationPage);
ferro_class_info!(AutomationPage {
    new: AutomationPage::new,
    markup: {
        methods: [
            fn OnButtonAddSomeText(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<AutomationPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_button_add_some_text(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(AutomationPage, "/Pages/AutomationPage.xaml");

impl AutomationPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    fn text_live_region(&self) -> Ref<TextBlock> {
        self.get_control::<TextBlock>("textLiveRegion")
    }

    fn on_button_add_some_text(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let text_live_region = self.text_live_region();
        text_live_region.set_text(Some(&format!("{} Lorem ipsum.", text_live_region.text().unwrap_or_default())));
    }
}
