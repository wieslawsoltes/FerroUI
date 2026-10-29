//! Port of `Controls/SingleSelectionTabControl.cs`.

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
    StaticType, StyledElementImpl, TypeInfo, VisualImpl,
};
use ferroui_controls::primitives::{
    SelectingItemsControl, SelectingItemsControlImpl, TemplatedControlImpl,
};
use ferroui_controls::{ControlImpl, ItemsControlImpl, SelectionMode, TabControl, TabControlImpl};

/// A tab control that selects one tab at a time and may have none selected (the selection mode
/// `Single`; a tab control is `AlwaysSelected` by default), styled as a tab control: the tab
/// control of the pages, whose search hides tabs.
#[repr(C)]
pub struct SingleSelectionTabControl {
    base: TabControl,
}

ferro_class!(SingleSelectionTabControl: TabControl);
ferro_class_info!(SingleSelectionTabControl {
    new: SingleSelectionTabControl::new
});
ferro_impl_classes!(
    SingleSelectionTabControl: FerroObjectImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ItemsControlImpl,
    SelectingItemsControlImpl,
    TabControlImpl
);

impl StyledElementImpl for SingleSelectionTabControl {
    /// `StyleKeyOverride => typeof(TabControl)`.
    fn style_key_override(_this: &Self) -> &'static TypeInfo {
        <TabControl as StaticType>::TYPE
    }
}

impl SingleSelectionTabControl {
    /// The static constructor: the selection mode defaults to `Single`.
    fn static_constructor() {
        SelectingItemsControl::selection_mode_property()
            .override_default_value::<SingleSelectionTabControl>(SelectionMode::SINGLE);
    }

    pub fn construct() -> Self {
        Self {
            base: TabControl::construct(),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
