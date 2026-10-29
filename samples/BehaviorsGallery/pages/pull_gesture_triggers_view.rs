//! Port of `Views/Pages/PullGestureTriggersView` of the reference sample: the class of `pages/PullGestureTriggersView.xaml`,
//! which logs the pull gesture triggers of its surface (the page is its own data context).

use crate::markup::{user_control_class, xaml_class};
use crate::pages::PageInfo;
use crate::view_models::items::{text_list, ItemList};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::utilities::DateTimeOffset;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, ElementRef, Ref, TypeInfo};
use ferroui_controls::{Control, ItemsSource, UserControl};
use std::rc::Rc;

#[repr(C)]
pub struct PullGestureTriggersView {
    base: UserControl,
    events: ItemList,
}

user_control_class!(PullGestureTriggersView);
ferro_class_info!(PullGestureTriggersView {
    new: PullGestureTriggersView::new,
    markup: {
        properties: [
            Events: ItemsSource { get: |this: &Ref<PullGestureTriggersView>| ItemsSource::from(this.events()) },
        ],
        methods: [
            fn OnPull(Option<BoxedValue>, Option<BoxedValue>) =>
                |this: &Ref<PullGestureTriggersView>, sender: Option<BoxedValue>, parameter: Option<BoxedValue>| {
                    this.on_pull(sender, parameter)
                },
            fn OnPullEnded(Option<BoxedValue>, Option<BoxedValue>) =>
                |this: &Ref<PullGestureTriggersView>, sender: Option<BoxedValue>, parameter: Option<BoxedValue>| {
                    this.on_pull_ended(sender, parameter)
                },
            fn Clear() => |this: &Ref<PullGestureTriggersView>| this.clear(),
        ],
    },
});
xaml_class!(PullGestureTriggersView, "/pages/PullGestureTriggersView.xaml");

impl PullGestureTriggersView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            events: text_list([]),
        }
    }

    /// `PullGestureTriggersView()`: the page is its own data context.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this.set_data_context(Some(Rc::new(ElementRef::of(&this)) as BoxedValue));
        this
    }

    /// `Events`.
    pub fn events(&self) -> ItemList {
        self.events.clone()
    }

    /// `OnPull(object?, object?)`.
    pub fn on_pull(&self, _sender: Option<BoxedValue>, _parameter: Option<BoxedValue>) {
        self.add_entry("Pull gesture started");
    }

    /// `OnPullEnded(object?, object?)`.
    pub fn on_pull_ended(&self, _sender: Option<BoxedValue>, _parameter: Option<BoxedValue>) {
        self.add_entry("Pull gesture ended");
    }

    /// `Clear()`.
    pub fn clear(&self) {
        self.events.clear();
    }

    /// `AddEntry(string)`: the newest entry first, at most 50.
    fn add_entry(&self, message: &str) {
        let entry = format!("{} - {message}", DateTimeOffset::now().to_string_with("T"));
        self.events.insert(0, Rc::new(entry) as BoxedValue);
        if self.events.count() > 50 {
            self.events.remove_at(self.events.count() - 1);
        }
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    PullGestureTriggersView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[PullGestureTriggersView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Pull Gesture Triggers",
    reference: "Views/Pages/PullGestureTriggersView",
    create: Some(create),
    types: TYPES,
    classes: &[&PullGestureTriggersView::XAML_CLASS],
    markup_types: &[],
    register_value_types,
};

/// The page is its own data context (a reference to the element).
fn register_value_types() {
    ValueTypes::register_element_ref::<PullGestureTriggersView>();
}
