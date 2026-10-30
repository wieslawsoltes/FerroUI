//! Port of `Views/Pages/ScrollGestureTriggersView` of the reference sample: the class of `pages/ScrollGestureTriggersView.xaml`,
//! which logs the scroll gesture triggers of its surface (the page is its own data context).

use crate::markup::{user_control_class, xaml_class};
use crate::pages::PageInfo;
use crate::view_models::items::{text_list, ItemList};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::utilities::DateTimeOffset;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, ElementRef, Ref, TypeInfo};
use ferroui_controls::{Control, ItemsSource, UserControl};
use std::rc::Rc;

#[repr(C)]
pub struct ScrollGestureTriggersView {
    base: UserControl,
    events: ItemList,
}

user_control_class!(ScrollGestureTriggersView);
ferro_class_info!(ScrollGestureTriggersView {
    new: ScrollGestureTriggersView::new,
    markup: {
        properties: [
            Events: ItemsSource { get: |this: &Ref<ScrollGestureTriggersView>| ItemsSource::from(this.events()) },
        ],
        methods: [
            fn OnScroll(Option<BoxedValue>, Option<BoxedValue>) =>
                |this: &Ref<ScrollGestureTriggersView>, sender: Option<BoxedValue>, parameter: Option<BoxedValue>| {
                    this.on_scroll(sender, parameter)
                },
            fn OnScrollEnded(Option<BoxedValue>, Option<BoxedValue>) =>
                |this: &Ref<ScrollGestureTriggersView>, sender: Option<BoxedValue>, parameter: Option<BoxedValue>| {
                    this.on_scroll_ended(sender, parameter)
                },
            fn OnScrollInertia(Option<BoxedValue>, Option<BoxedValue>) =>
                |this: &Ref<ScrollGestureTriggersView>, sender: Option<BoxedValue>, parameter: Option<BoxedValue>| {
                    this.on_scroll_inertia(sender, parameter)
                },
            fn Clear() => |this: &Ref<ScrollGestureTriggersView>| this.clear(),
        ],
    },
});
xaml_class!(ScrollGestureTriggersView, "/pages/ScrollGestureTriggersView.xaml");

impl ScrollGestureTriggersView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            events: text_list([]),
        }
    }

    /// `ScrollGestureTriggersView()`: the page is its own data context.
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

    /// `OnScroll(object?, object?)`.
    pub fn on_scroll(&self, _sender: Option<BoxedValue>, _parameter: Option<BoxedValue>) {
        self.add_entry("Scroll gesture");
    }

    /// `OnScrollEnded(object?, object?)`.
    pub fn on_scroll_ended(&self, _sender: Option<BoxedValue>, _parameter: Option<BoxedValue>) {
        self.add_entry("Scroll gesture ended");
    }

    /// `OnScrollInertia(object?, object?)`.
    pub fn on_scroll_inertia(&self, _sender: Option<BoxedValue>, _parameter: Option<BoxedValue>) {
        self.add_entry("Scroll inertia starting");
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
    ScrollGestureTriggersView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[ScrollGestureTriggersView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Scroll Gesture Triggers",
    reference: "Views/Pages/ScrollGestureTriggersView",
    create: Some(create),
    types: TYPES,
    classes: &[&ScrollGestureTriggersView::XAML_CLASS],
    markup_types: &[],
    register_value_types,
};

/// The page is its own data context (a reference to the element).
fn register_value_types() {
    ValueTypes::register_element_ref::<ScrollGestureTriggersView>();
}
