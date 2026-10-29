//! Port of `Views/Pages/DebounceThrottleActionView` of the reference sample: the class of
//! `pages/DebounceThrottleActionView.xaml`, whose method the throttled action calls.

use crate::markup::{user_control_class, xaml_class};
use crate::pages::{no_value_types, PageInfo};
use ferroui_base::{ferro_class_info, instantiate, Ref, TypeInfo};
use ferroui_controls::{Control, TextBlock, UserControl};
use std::cell::Cell;

#[repr(C)]
pub struct DebounceThrottleActionView {
    base: UserControl,
    throttle_count: Cell<i32>,
}

user_control_class!(DebounceThrottleActionView);
ferro_class_info!(DebounceThrottleActionView {
    new: DebounceThrottleActionView::new,
    markup: {
        methods: [
            fn IncrementThrottleCount() =>
                |this: &Ref<DebounceThrottleActionView>| this.increment_throttle_count(),
        ],
    },
});
xaml_class!(
    DebounceThrottleActionView,
    "/pages/DebounceThrottleActionView.xaml"
);

impl DebounceThrottleActionView {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
            throttle_count: Cell::new(0),
        }
    }

    /// `DebounceThrottleActionView()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    /// `IncrementThrottleCount()`: counts a click the throttle let through and shows the count.
    pub fn increment_throttle_count(&self) {
        let count = self.throttle_count.get() + 1;
        self.throttle_count.set(count);
        if let Some(text_block) = self.find_control::<TextBlock>("ThrottleCountText") {
            text_block.set_text(Some(&format!("Clicks Processed: {count}")));
        }
    }
}

/// Creates the page.
fn create() -> Ref<Control> {
    DebounceThrottleActionView::new().upcast()
}

/// The classes of the page (`X::TYPE`).
pub(crate) const TYPES: &[&TypeInfo] = &[DebounceThrottleActionView::TYPE];

/// The entry of the page in the page table.
pub(crate) const PAGE: PageInfo = PageInfo {
    header: "Debounce & Throttle",
    reference: "Views/Pages/DebounceThrottleActionView",
    create: Some(create),
    types: TYPES,
    classes: &[&DebounceThrottleActionView::XAML_CLASS],
    markup_types: &[],
    register_value_types: no_value_types,
};
