use crate::primitives::TemplatedControlImpl;
use crate::{ComboBox, ContentControlImpl, ControlImpl, ListBoxItem};
use ferroui_base::input::{InputElement, InputElementImpl};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::reactive::ObservableExt;
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObject, FerroObjectExtensions, FerroObjectImpl,
    FerroObjectImplExt, Ref, StyledElementImpl, VisualImpl,
};

/// A selectable item in a [`ComboBox`].
#[repr(C)]
pub struct ComboBoxItem {
    base: ListBoxItem,
}

ferro_class!(ComboBoxItem: ListBoxItem);
ferroui_base::ferro_class_info!(ComboBoxItem { new: ComboBoxItem::new });
ferro_impl_classes!(
    ComboBoxItem: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);

impl FerroObjectImpl for ComboBoxItem {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);

        let weak = this.to_ref().downgrade();
        let object: &FerroObject = this;
        FerroObjectExtensions::get_observable(object, InputElement::is_focused_property()).subscribe_fn(move |focused| {
            if focused {
                let Some(this) = weak.upgrade() else { return };
                if let Some(parent) = this.parent().and_then(|parent| parent.cast::<ComboBox>()) {
                    parent.item_focused(&this);
                }
            }
        });
    }
}

impl ComboBoxItem {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: ListBoxItem::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    fn static_constructor() {
        crate::automation::AutomationProperties::control_type_override_property()
            .override_default_value::<ComboBoxItem>(Some(crate::automation::peers::AutomationControlType::ComboBoxItem));
    }
}
