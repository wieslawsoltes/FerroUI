use super::TemplatedControlImpl;
use crate::{ContentControlImpl, ControlImpl, ListBoxItem};
use ferroui_base::input::{FocusChangedEventArgs, InputElementImpl, InputElementImplExt};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl,
};

/// Represents a tab in a `TabStrip`.
#[repr(C)]
pub struct TabStripItem {
    base: ListBoxItem,
}

ferro_class!(TabStripItem: ListBoxItem);
ferroui_base::ferro_class_info!(TabStripItem { new: TabStripItem::new });
ferro_impl_classes!(
    TabStripItem: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl InputElementImpl for TabStripItem {
    fn on_got_focus(this: &Self, e: &FocusChangedEventArgs) {
        Self::parent_on_got_focus(this, e);
        this.update_selection_from_event(e);
    }
}

impl TabStripItem {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ListBoxItem::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
