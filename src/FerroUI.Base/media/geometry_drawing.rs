use crate::media::immutable::ImmutablePen;
use crate::media::{Brushes, Colors, Drawing, DrawingContext, DrawingImpl, Geometry, IBrush, IPen};
use crate::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty, Nullable, Rect, Ref,
    StyledProperty,
};
use std::rc::Rc;

/// Represents a drawing of a [`Geometry`] with a brush and a pen.
#[repr(C)]
pub struct GeometryDrawing {
    base: Drawing,
}

ferro_class!(GeometryDrawing: Drawing);
crate::ferro_class_info!(GeometryDrawing { new: GeometryDrawing::new });
ferro_impl_classes!(GeometryDrawing: FerroObjectImpl);

impl DrawingImpl for GeometryDrawing {
    fn draw_core(this: &Self, context: &mut DrawingContext) {
        if let Some(geometry) = this.geometry() {
            context.draw_geometry(this.brush().as_ref(), this.pen().as_ref(), &geometry);
        }
    }

    fn get_bounds(this: &Self) -> Rect {
        let Some(geometry) = this.geometry() else { return Rect::default() };
        match this.pen() {
            Some(pen) => geometry.get_render_bounds(&*pen),
            // Adding the pen's stroke thickness here could yield wrong
            // results due to transforms.
            None => geometry.get_render_bounds(&ImmutablePen::from_uint32(Colors::BLACK.to_uint32(), 0.0)),
        }
    }
}

crate::ferro_properties! { impl GeometryDrawing {
    ferro_property!(
        /// Defines the `Geometry` property.
        pub fn geometry_property() -> StyledProperty<Option<Ref<Geometry>>> {
            FerroProperty::register::<GeometryDrawing, _>("Geometry", None)
        }
    );

    ferro_property!(
        /// Defines the `Brush` property.
        pub fn brush_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<GeometryDrawing, _>("Brush", Some(Brushes::transparent() as Rc<dyn IBrush>))
        }
    );

    ferro_property!(
        /// Defines the `Pen` property.
        pub fn pen_property() -> StyledProperty<Option<Rc<dyn IPen>>> {
            FerroProperty::register::<GeometryDrawing, _>("Pen", None)
        }
    );
} }

impl GeometryDrawing {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Drawing::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The geometry that describes the shape of this drawing.
    pub fn geometry(&self) -> Option<Ref<Geometry>> {
        self.get_value(Self::geometry_property())
    }

    pub fn set_geometry(&self, value: impl Into<Nullable<Geometry>>) {
        self.set_value(Self::geometry_property(), value.into().0)
    }

    /// The brush used to fill the interior of the shape described by this
    /// drawing.
    pub fn brush(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::brush_property())
    }

    pub fn set_brush(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::brush_property(), value)
    }

    /// The pen used to stroke this drawing.
    pub fn pen(&self) -> Option<Rc<dyn IPen>> {
        self.get_value(Self::pen_property())
    }

    pub fn set_pen(&self, value: Option<Rc<dyn IPen>>) {
        self.set_value(Self::pen_property(), value)
    }
}
