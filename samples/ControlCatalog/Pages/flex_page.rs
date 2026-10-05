//! Port of `Pages/FlexPage.xaml.cs`: the class of the document
//! `Pages/FlexPage.xaml`.

use crate::markup::{content_page_class, xaml_class};
use crate::view_models::{FlexItemViewModel, FlexViewModel};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::metadata::from_markup_value;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref};
use ferroui_controls::{ContentPage, ListBoxItem};
use std::rc::Rc;

#[repr(C)]
pub struct FlexPage {
    base: ContentPage,
}

content_page_class!(FlexPage);
ferro_class_info!(FlexPage {
    new: FlexPage::new,
    markup: {
        methods: [
            fn OnItemTapped(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<FlexPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.on_item_tapped(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(FlexPage, "/Pages/FlexPage.xaml");

impl FlexPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(FlexViewModel::new() as BoxedValue));
        this
    }

    fn on_item_tapped(&self, sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let control = sender
            .as_ref()
            .and_then(|sender| ValueTypes::as_object(&**sender))
            .and_then(|sender| sender.cast::<ListBoxItem>());
        let vm = from_markup_value::<Rc<FlexViewModel>>(&self.data_context());
        let item = control.and_then(|control| from_markup_value::<Rc<FlexItemViewModel>>(&control.data_context()));
        if let (Some(vm), Some(item)) = (vm, item) {
            if let Some(selected_item) = vm.selected_item() {
                selected_item.set_is_selected(false);
            }

            if vm.selected_item().is_some_and(|selected_item| Rc::ptr_eq(&selected_item, &item)) {
                vm.set_selected_item(None);
            } else {
                vm.set_selected_item(Some(item.clone()));
                item.set_is_selected(true);
            }
        }
    }
}
