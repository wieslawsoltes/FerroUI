use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObject,
    FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};
use std::cell::Cell;

/// Shared identity and presentation properties of a chart axis.
#[repr(C)]
pub struct Axis {
    base: FerroObject,
    owner: Cell<Option<u64>>,
}

ferro_class!(Axis: FerroObject);
ferro_class_info!(Axis { new: Axis::new });
ferro_impl_classes!(Axis: FerroObjectImpl);

ferro_properties! { impl Axis {
    pub fn is_visible_property() -> StyledProperty<bool> {
        FerroProperty::register::<Axis, _>("IsVisible", true)
    }

    pub fn label_property() -> StyledProperty<String> {
        FerroProperty::register::<Axis, _>("Label", String::new())
    }
} }

impl Axis {
    /// Creates the fields of an axis, for use by deriving classes.
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            owner: Cell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn is_visible(&self) -> bool {
        self.get_value(Self::is_visible_property())
    }

    pub fn set_is_visible(&self, value: bool) {
        self.set_value(Self::is_visible_property(), value);
    }

    pub fn label(&self) -> String {
        self.get_value(Self::label_property())
    }

    pub fn set_label(&self, value: impl Into<String>) {
        self.set_value(Self::label_property(), value.into());
    }

    /// The nullable source API normalizes a null label to an empty string.
    pub fn set_optional_label(&self, value: Option<&str>) {
        self.set_label(value.unwrap_or_default());
    }

    pub(crate) fn owner_id(&self) -> Option<u64> {
        self.owner.get()
    }

    pub(crate) fn set_owner_id(&self, value: Option<u64>) {
        self.owner.set(value);
    }
}
