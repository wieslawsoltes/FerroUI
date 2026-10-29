//! Port of `Controls/EditableItem.xaml.cs` (the class of `Controls/EditableItem.xaml`).

use crate::markup::xaml_class;
use ferroui_base::data::BindingMode;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate,
    FerroObjectImpl, Ref, StyledElementImpl, StyledProperty, StyledPropertyMetadata, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, ControlImpl, TextBlock, UserControl};

/// An item of the editable lists and trees: its text, shown in a text block and edited in a text
/// box on a double tap (`InlineEditBehavior`).
#[repr(C)]
pub struct EditableItem {
    base: UserControl,
}

ferro_class!(EditableItem: UserControl);
ferro_class_info!(EditableItem {
    new: EditableItem::new
});
ferro_impl_classes!(
    EditableItem: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
xaml_class!(EditableItem, "/Controls/EditableItem.xaml");

ferro_properties! {
    impl EditableItem {
        /// Defines the `Text` property (the text of a text block, bound both ways by default).
        pub fn text_property() -> StyledProperty<Option<String>> {
            TextBlock::text_property()
                .add_owner_with::<EditableItem>(StyledPropertyMetadata::new(None).with_default_binding_mode(BindingMode::TwoWay))
        }
    }
}

impl EditableItem {
    pub fn construct() -> Self {
        Self {
            base: UserControl::construct(),
        }
    }

    /// `EditableItem()`.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }

    /// `Text`.
    pub fn text(&self) -> Option<String> {
        self.get_value(Self::text_property())
    }

    /// Sets `Text`.
    pub fn set_text(&self, value: Option<String>) {
        self.set_value(Self::text_property(), value)
    }
}
