//! Port of `Views/Pages/PointerEventsTriggerView` of the reference sample: the class of
//! `pages/PointerEventsTriggerView.xaml`, which logs the pointer events of its surface (the page
//! is its own data context).

use crate::markup::{user_control_class, xaml_class};
use crate::pages::PageInfo;
use crate::view_models::items::{text_list, ItemList};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::input::{PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs};
use ferroui_base::interactivity::IRoutedEventArgs;
use ferroui_base::{ferro_class_info, instantiate, BoxedValue, ElementRef, Point, Ref, TypeInfo};
use ferroui_controls::{Control, ItemsSource, UserControl};
use std::rc::Rc;

#[repr(C)]
pub struct PointerEventsTriggerView {
    base: UserControl,
    events: ItemList,
}

user_control_class!(PointerEventsTriggerView);
ferro_class_info!(PointerEventsTriggerView {
    new: PointerEventsTriggerView::new,
    markup: {
        properties: [
            Events: ItemsSource { get: |this: &Ref<PointerEventsTriggerView>| ItemsSource::from(this.events()) },
        ],
        methods: [
            fn OnPointerEvent(Option<BoxedValue>, Option<BoxedValue>) =>
                |this: &Ref<PointerEventsTriggerView>, sender: Option<BoxedValue>, args: Option<BoxedValue>| {
                    this.on_pointer_event(sender, args)
                },
        ],
    },
});
xaml_class!(
    PointerEventsTriggerView,
    "/pages/PointerEventsTriggerView.xaml"
);

/// A position as the reference formats it with `F0` (`x, y`, without decimals).
fn format_position(position: Point) -> String {
    format!("{:.0}, {:.0}", position.x.round(), position.y.round())
}

impl PointerEventsTriggerView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            events: text_list([]),
        }
    }

    /// `PointerEventsTriggerView()`: the page is its own data context.
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

    /// `OnPointerEvent(object?, object?)`: the newest event first, at most 50.
    pub fn on_pointer_event(&self, _sender: Option<BoxedValue>, args: Option<BoxedValue>) {
        let description = Self::describe_pointer_event(args.as_ref());
        self.events.insert(0, Rc::new(description) as BoxedValue);
        if self.events.count() > 50 {
            self.events.remove_at(self.events.count() - 1);
        }
    }

    /// `DescribePointerEvent(object?)`.
    fn describe_pointer_event(args: Option<&BoxedValue>) -> String {
        let Some(args) = args else {
            return "Unknown".to_string();
        };
        let Some(routed) = args.downcast_ref::<Rc<dyn IRoutedEventArgs>>() else {
            let name = ValueType::of_value(&**args).name();
            return name.rsplit("::").next().unwrap_or(name).to_string();
        };
        if let Some(pressed) = routed.downcast_ref::<PointerPressedEventArgs>() {
            format!("Pressed at {}", format_position(pressed.get_position(None)))
        } else if let Some(released) = routed.downcast_ref::<PointerReleasedEventArgs>() {
            format!("Released at {}", format_position(released.get_position(None)))
        } else if let Some(moved) = routed.downcast_ref::<PointerEventArgs>() {
            format!("Moved at {}", format_position(moved.get_position(None)))
        } else {
            // The name of the type of the args: the name of their routed event.
            routed
                .as_routed_event_args()
                .routed_event()
                .map(|routed_event| format!("{}EventArgs", routed_event.name()))
                .unwrap_or_else(|| "RoutedEventArgs".to_string())
        }
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    PointerEventsTriggerView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[PointerEventsTriggerView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "PointerEventsTrigger",
    reference: "Views/Pages/PointerEventsTriggerView",
    create: Some(create),
    types: TYPES,
    classes: &[&PointerEventsTriggerView::XAML_CLASS],
    markup_types: &[],
    register_value_types,
};

/// The page is its own data context (a reference to the element).
fn register_value_types() {
    ValueTypes::register_element_ref::<PointerEventsTriggerView>();
}
