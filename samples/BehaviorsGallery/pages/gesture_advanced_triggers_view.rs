//! Port of `Views/Pages/GestureAdvancedTriggersView` of the reference sample: the class of `pages/GestureAdvancedTriggersView.xaml`,
//! which logs the pinch gesture triggers of its surface (the page is its own data context).

use crate::markup::{user_control_class, xaml_class};
use crate::pages::PageInfo;
use crate::view_models::items::{text_list, ItemList};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::utilities::DateTimeOffset;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, ElementRef, Ref, TypeInfo};
use ferroui_controls::{Control, ItemsSource, UserControl};
use std::rc::Rc;

#[repr(C)]
pub struct GestureAdvancedTriggersView {
    base: UserControl,
    events: ItemList,
}

user_control_class!(GestureAdvancedTriggersView);
ferro_class_info!(GestureAdvancedTriggersView {
    new: GestureAdvancedTriggersView::new,
    markup: {
        properties: [
            Events: ItemsSource { get: |this: &Ref<GestureAdvancedTriggersView>| ItemsSource::from(this.events()) },
        ],
        methods: [
            fn OnPinch(Option<BoxedValue>, Option<BoxedValue>) =>
                |this: &Ref<GestureAdvancedTriggersView>, sender: Option<BoxedValue>, parameter: Option<BoxedValue>| {
                    this.on_pinch(sender, parameter)
                },
            fn OnPinchEnded(Option<BoxedValue>, Option<BoxedValue>) =>
                |this: &Ref<GestureAdvancedTriggersView>, sender: Option<BoxedValue>, parameter: Option<BoxedValue>| {
                    this.on_pinch_ended(sender, parameter)
                },
            fn Clear() => |this: &Ref<GestureAdvancedTriggersView>| this.clear(),
        ],
    },
});
xaml_class!(GestureAdvancedTriggersView, "/pages/GestureAdvancedTriggersView.xaml");

impl GestureAdvancedTriggersView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            events: text_list([]),
        }
    }

    /// `GestureAdvancedTriggersView()`: the page is its own data context.
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

    /// `OnPinch(object?, object?)`.
    pub fn on_pinch(&self, _sender: Option<BoxedValue>, _parameter: Option<BoxedValue>) {
        self.add_entry("Pinch gesture detected");
    }

    /// `OnPinchEnded(object?, object?)`.
    pub fn on_pinch_ended(&self, _sender: Option<BoxedValue>, _parameter: Option<BoxedValue>) {
        self.add_entry("Pinch ended");
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
    GestureAdvancedTriggersView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[GestureAdvancedTriggersView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Gesture Advanced Triggers",
    reference: "Views/Pages/GestureAdvancedTriggersView",
    create: Some(create),
    types: TYPES,
    classes: &[&GestureAdvancedTriggersView::XAML_CLASS],
    markup_types: &[],
    register_value_types,
};

/// The page is its own data context (a reference to the element).
fn register_value_types() {
    ValueTypes::register_element_ref::<GestureAdvancedTriggersView>();
}
