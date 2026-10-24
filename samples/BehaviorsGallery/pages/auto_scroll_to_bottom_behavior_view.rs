//! Port of `Views/Pages/AutoScrollToBottomBehaviorView` of the reference sample: the class of
//! `pages/AutoScrollToBottomBehaviorView.xaml`, which fills the list box and adds an item on each
//! click of its button.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use crate::view_models::items::{text_list, ItemList};
use ferroui_base::interactivity::{IRoutedEventArgs, RoutedEventArgs};
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, Ref, TypeInfo};
use ferroui_controls::{Control, ItemsSource, ListBox, UserControl};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct AutoScrollToBottomBehaviorView {
    base: UserControl,
    items: ItemList,
    count: Cell<i32>,
}

user_control_class!(AutoScrollToBottomBehaviorView);
ferro_class_info!(AutoScrollToBottomBehaviorView {
    new: AutoScrollToBottomBehaviorView::new,
    markup: {
        methods: [
            fn AddItemButton_Click(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<AutoScrollToBottomBehaviorView>, sender: Option<BoxedValue>, e: Rc<dyn IRoutedEventArgs>| {
                    this.add_item_button_click(&sender, e.as_routed_event_args())
                },
        ],
    },
});
xaml_class!(
    AutoScrollToBottomBehaviorView,
    "/pages/AutoScrollToBottomBehaviorView.xaml"
);

impl AutoScrollToBottomBehaviorView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            items: text_list([]),
            count: Cell::new(0),
        }
    }

    /// `AutoScrollToBottomBehaviorView()`: twenty items, the items of the list box.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        for _ in 0..20 {
            this.add_item();
        }

        if let Some(items_list_box) = this.find_control::<ListBox>("ItemsListBox") {
            items_list_box.set_items_source(Some(ItemsSource::from(this.items.clone())));
        }
        this
    }

    /// `_items.Add($"Item {_count++}")`.
    fn add_item(&self) {
        let count = self.count.get();
        self.count.set(count + 1);
        self.items
            .add(Rc::new(format!("Item {count}")) as BoxedValue);
    }

    /// `AddItemButton_Click`: adds an item.
    pub fn add_item_button_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        self.add_item();
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    AutoScrollToBottomBehaviorView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[AutoScrollToBottomBehaviorView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "AutoScrollToBottomBehavior",
    reference: "Views/Pages/AutoScrollToBottomBehaviorView",
    create: Some(create),
    types: TYPES,
    classes: &[&AutoScrollToBottomBehaviorView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
