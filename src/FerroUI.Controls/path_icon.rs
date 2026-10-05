use crate::primitives::TemplatedControlImpl;
use crate::{ControlImpl, IconElement};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::Geometry;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty,
    Nullable, Ref, StyledElementImpl, StyledProperty, Visual, VisualImpl,
};

/// Represents an icon that uses a vector path as its content.
#[repr(C)]
pub struct PathIcon {
    base: IconElement,
}

ferro_class!(PathIcon: IconElement);
ferroui_base::ferro_class_info!(PathIcon { new: PathIcon::new });
ferro_impl_classes!(
    PathIcon: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl
);

impl FerroObjectImpl for PathIcon {}

ferroui_base::ferro_properties! { impl PathIcon {
    ferro_property!(
        /// Defines the `Data` property.
        pub fn data_property() -> StyledProperty<Option<Ref<Geometry>>> {
            FerroProperty::register::<PathIcon, _>("Data", None)
        }
    );
} }

impl PathIcon {
    fn static_constructor() {
        Visual::affects_render::<PathIcon>(&[Self::data_property().as_property()]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: IconElement::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets the geometry drawn by the icon.
    pub fn data(&self) -> Option<Ref<Geometry>> {
        self.get_value(Self::data_property())
    }

    /// Sets the geometry drawn by the icon.
    pub fn set_data(&self, value: impl Into<Nullable<Geometry>>) {
        self.set_value(Self::data_property(), value.into().0)
    }
}
