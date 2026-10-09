use super::{Shape, ShapeImpl};
use crate::ControlImpl;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::Geometry;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty,
    Nullable, Ref, StyledElementImpl, StyledProperty, VisualImpl,
};

/// Draws a [`Geometry`].
#[repr(C)]
pub struct Path {
    base: Shape,
}

ferro_class!(Path: Shape);
ferroui_base::ferro_class_info!(Path { new: Path::new });
ferro_impl_classes!(
    Path: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

ferroui_base::ferro_impl_classes!(Path: FerroObjectImpl);

impl ShapeImpl for Path {
    fn create_defining_geometry(this: &Self) -> Option<Ref<Geometry>> {
        this.data()
    }
}

ferroui_base::ferro_properties! { impl Path {
    ferro_property!(
        /// Defines the `Data` property.
        pub fn data_property() -> StyledProperty<Option<Ref<Geometry>>> {
            FerroProperty::register::<Path, _>("Data", None)
        }
    );
} }

impl Path {
    fn static_constructor() {
        Shape::affects_geometry::<Path>(&[Self::data_property().as_property()]);
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: Shape::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The geometry drawn by the path.
    pub fn data(&self) -> Option<Ref<Geometry>> {
        self.get_value(Self::data_property())
    }

    pub fn set_data(&self, value: impl Into<Nullable<Geometry>>) {
        self.set_value(Self::data_property(), value.into().0)
    }
}
