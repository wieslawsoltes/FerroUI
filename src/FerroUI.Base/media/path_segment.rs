use crate::media::StreamGeometryContext;
use crate::{ferro_class, ferro_property, FerroObject, FerroObjectImpl, FerroProperty, StyledProperty};

/// Represents a segment of a [`PathFigure`](crate::media::PathFigure).
#[repr(C)]
pub struct PathSegment {
    base: FerroObject,
}

ferro_class! {
    PathSegment: FerroObject, virtuals PathSegmentImpl: FerroObjectImpl {
        /// Draws the segment into `ctx`.
        fn apply_to(this, ctx: &mut StreamGeometryContext);

        /// The path markup of the segment.
        fn to_string(this) -> String;
    }
}

crate::ferro_impl_classes!(PathSegment: FerroObjectImpl);

impl PathSegmentImpl for PathSegment {
    fn apply_to(_this: &Self, _ctx: &mut StreamGeometryContext) {
        panic!("PathSegment is abstract: 'apply_to' must be implemented by the deriving class")
    }

    fn to_string(this: &Self) -> String {
        this.get_type().name().to_string()
    }
}

crate::ferro_properties! { impl PathSegment {
    ferro_property!(pub fn is_stroked_property() -> StyledProperty<bool> {
        FerroProperty::register::<PathSegment, _>("IsStroked", true)
    });
} }

impl PathSegment {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    /// Whether the segment is stroked.
    pub fn is_stroked(&self) -> bool {
        self.get_value(Self::is_stroked_property())
    }

    pub fn set_is_stroked(&self, value: bool) {
        self.set_value(Self::is_stroked_property(), value)
    }
}
