//! Port of `Pages/ListBoxPage.xaml.cs`: the class of the document `Pages/ListBoxPage.xaml`.

use crate::markup::{user_control_class, xaml_class};
use ferroui_base::collections::FerroList;
use ferroui_base::data::core::ValueTypes;
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, ElementRef, Ref};
use ferroui_controls::{ListBox, UserControl};
use std::rc::Rc;

// The list of the items of the page (`List<string>`), with its item type.
ferroui_controls::ferro_markup_list!(pub ListBoxItemList: String);

#[repr(C)]
pub struct ListBoxPage {
    base: UserControl,
    list_box_items: FerroList<String>,
}

user_control_class!(ListBoxPage);
ferro_class_info!(ListBoxPage {
    new: ListBoxPage::new,
    markup: {
        properties: [
            ListBoxItems: FerroList<String> { get: |this: &Ref<ListBoxPage>| this.list_box_items() },
        ],
        methods: [
            fn ListBoxSelectionClear_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<ListBoxPage>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.list_box_selection_clear_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(ListBoxPage, "/Pages/ListBoxPage.xaml");

impl ListBoxPage {
    pub fn construct() -> Self {
        let list_box_items = FerroList::new();
        for x in 0..100 {
            list_box_items.add(format!("Item {x}"));
        }
        Self { base: UserControl::construct(), list_box_items }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        // `DataContext = this;` in the managed original, where the collector frees a page
        // that holds itself. Here the page is its data context as an element reference, which
        // holds it weakly: the bindings of the document read the page through it.
        ValueTypes::register_element_ref::<ListBoxPage>();
        this.set_data_context(Some(Rc::new(ElementRef::of(&this)) as BoxedValue));
        this
    }

    pub fn list_box_items(&self) -> FerroList<String> {
        self.list_box_items.clone()
    }

    fn basic_list_box(&self) -> Ref<ListBox> {
        self.get_control::<ListBox>("BasicListBox")
    }

    fn list_box_selection_clear_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.basic_list_box().set_selected_index(-1);
    }
}
