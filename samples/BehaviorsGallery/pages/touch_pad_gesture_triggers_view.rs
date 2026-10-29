//! Port of `Views/Pages/TouchPadGestureTriggersView` of the reference sample: the class of `pages/TouchPadGestureTriggersView.xaml`,
//! which logs the touchpad gesture triggers of its surface (the page is its own data context).

use crate::markup::{user_control_class, xaml_class};
use crate::pages::PageInfo;
use crate::view_models::items::{text_list, ItemList};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::utilities::DateTimeOffset;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, ElementRef, Ref, TypeInfo};
use ferroui_controls::{Control, ItemsSource, UserControl};
use std::rc::Rc;

#[repr(C)]
pub struct TouchPadGestureTriggersView {
    base: UserControl,
    events: ItemList,
}

user_control_class!(TouchPadGestureTriggersView);
ferro_class_info!(TouchPadGestureTriggersView {
    new: TouchPadGestureTriggersView::new,
    markup: {
        properties: [
            Events: ItemsSource { get: |this: &Ref<TouchPadGestureTriggersView>| ItemsSource::from(this.events()) },
        ],
        methods: [
            fn OnMagnify(Option<BoxedValue>, Option<BoxedValue>) =>
                |this: &Ref<TouchPadGestureTriggersView>, sender: Option<BoxedValue>, parameter: Option<BoxedValue>| {
                    this.on_magnify(sender, parameter)
                },
            fn OnRotate(Option<BoxedValue>, Option<BoxedValue>) =>
                |this: &Ref<TouchPadGestureTriggersView>, sender: Option<BoxedValue>, parameter: Option<BoxedValue>| {
                    this.on_rotate(sender, parameter)
                },
            fn OnSwipe(Option<BoxedValue>, Option<BoxedValue>) =>
                |this: &Ref<TouchPadGestureTriggersView>, sender: Option<BoxedValue>, parameter: Option<BoxedValue>| {
                    this.on_swipe(sender, parameter)
                },
            fn Clear() => |this: &Ref<TouchPadGestureTriggersView>| this.clear(),
        ],
    },
});
xaml_class!(TouchPadGestureTriggersView, "/pages/TouchPadGestureTriggersView.xaml");

impl TouchPadGestureTriggersView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            events: text_list([]),
        }
    }

    /// `TouchPadGestureTriggersView()`: the page is its own data context.
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

    /// `OnMagnify(object?, object?)`.
    pub fn on_magnify(&self, _sender: Option<BoxedValue>, _parameter: Option<BoxedValue>) {
        self.add_entry("Touchpad magnify gesture");
    }

    /// `OnRotate(object?, object?)`.
    pub fn on_rotate(&self, _sender: Option<BoxedValue>, _parameter: Option<BoxedValue>) {
        self.add_entry("Touchpad rotate gesture");
    }

    /// `OnSwipe(object?, object?)`.
    pub fn on_swipe(&self, _sender: Option<BoxedValue>, _parameter: Option<BoxedValue>) {
        self.add_entry("Touchpad swipe gesture");
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
    TouchPadGestureTriggersView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[TouchPadGestureTriggersView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "TouchPad Gesture Triggers",
    reference: "Views/Pages/TouchPadGestureTriggersView",
    create: Some(create),
    types: TYPES,
    classes: &[&TouchPadGestureTriggersView::XAML_CLASS],
    markup_types: &[],
    register_value_types,
};

/// The page is its own data context (a reference to the element).
fn register_value_types() {
    ValueTypes::register_element_ref::<TouchPadGestureTriggersView>();
}
