use crate::media::ref_adapter::RefAdapter;
use crate::media::{
    Brush, Color, ConicGradientBrush, DrawingBrush, GradientBrush, GradientSpreadMethod, IConicGradientBrush, IGradientBrush,
    IGradientStop, IImageBrush, IImmutableBrush, ILinearGradientBrush, IMutableBrush, IRadialGradientBrush,
    ISceneBrush, ISolidColorBrush, ITileBrush, ITransform, ImageBrush, LinearGradientBrush, RadialGradientBrush,
    SolidColorBrush, TileBrush, VisualBrush,
};
use crate::{FerroObject, ObjectType, Ref, RelativePoint, RelativeScalar, Upcast};
use std::any::Any;
use std::rc::Rc;

/// Describes how an area is painted.
///
/// Implemented by the immutable brush types and, through an adapter, by
/// every class deriving from [`Brush`]: a handle of such a class converts to
/// `Rc<dyn IBrush>` with `into()`, so a value of that type can hold either
/// kind of brush. The `as_*` members replace interface casts
/// (`brush as ISolidColorBrush`).
pub trait IBrush: 'static {
    /// The opacity of the brush.
    fn opacity(&self) -> f64;

    /// The transform of the brush.
    fn transform(&self) -> Option<Rc<dyn ITransform>>;

    /// The origin of the brush [`transform`](Self::transform).
    fn transform_origin(&self) -> RelativePoint;

    /// The transform of the brush, relative to the bounds of the painted
    /// area.
    fn relative_transform(&self) -> Option<Rc<dyn ITransform>>;

    /// The implementing value, for downcasts to immutable brush types. For
    /// mutable brushes use [`as_object`](Self::as_object).
    fn as_any(&self) -> &dyn Any;

    /// The object behind the brush when it is a mutable [`Brush`].
    fn as_object(&self) -> Option<&FerroObject> {
        None
    }

    /// The brush viewed as [`ISolidColorBrush`], when it is one.
    fn as_solid_color_brush(&self) -> Option<&dyn ISolidColorBrush> {
        None
    }

    /// The brush viewed as [`IGradientBrush`], when it is one.
    fn as_gradient_brush(&self) -> Option<&dyn IGradientBrush> {
        None
    }

    /// The brush viewed as [`ILinearGradientBrush`], when it is one.
    fn as_linear_gradient_brush(&self) -> Option<&dyn ILinearGradientBrush> {
        None
    }

    /// The brush viewed as [`IRadialGradientBrush`], when it is one.
    fn as_radial_gradient_brush(&self) -> Option<&dyn IRadialGradientBrush> {
        None
    }

    /// The brush viewed as [`IConicGradientBrush`], when it is one.
    fn as_conic_gradient_brush(&self) -> Option<&dyn IConicGradientBrush> {
        None
    }

    /// The brush viewed as [`ITileBrush`], when it is one.
    fn as_tile_brush(&self) -> Option<&dyn ITileBrush> {
        None
    }

    /// The brush viewed as [`IImageBrush`], when it is one.
    fn as_image_brush(&self) -> Option<&dyn IImageBrush> {
        None
    }

    /// The brush viewed as [`ISceneBrush`], when it is one.
    fn as_scene_brush(&self) -> Option<&dyn ISceneBrush> {
        None
    }

    /// The brush viewed as [`IMutableBrush`], when it is one.
    fn as_mutable_brush(&self) -> Option<&dyn IMutableBrush> {
        None
    }

    /// The brush viewed as [`IImmutableBrush`], when it is one (the `is
    /// IImmutableBrush` test of upstream).
    fn as_immutable_brush(&self) -> Option<&dyn IImmutableBrush> {
        None
    }

    /// The composition render resource behind the object, if it is a
    /// mutable object with server-side counterparts on the compositors it
    /// is drawn with.
    fn as_composition_render_resource(
        &self,
    ) -> Option<&dyn crate::rendering::composition::drawing::ICompositionRenderResource> {
        None
    }

    /// The brush as a composition brush, when it is one (the `is
    /// CompositionBrush` test of upstream).
    fn as_composition_brush(&self) -> Option<&crate::rendering::composition::CompositionBrush> {
        None
    }

    /// The brush as an [`IImmutableBrush`] handle, when it is immutable.
    fn into_immutable_brush(self: Rc<Self>) -> Option<Rc<dyn IImmutableBrush>> {
        None
    }

    /// The identity of the brush, used for reference equality.
    #[doc(hidden)]
    fn reference_id(&self) -> *const () {
        self as *const Self as *const ()
    }

    /// Whether the brush equals `other`. Brushes compare by reference unless
    /// the type defines structural equality (immutable solid color brushes).
    fn equals(&self, other: &dyn IBrush) -> bool {
        self.reference_id() == other.reference_id()
    }
}

impl PartialEq for dyn IBrush {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.equals(other)
    }
}

impl std::fmt::Debug for dyn IBrush {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.as_solid_color_brush() {
            Some(solid) => write!(f, "IBrush({})", solid.color()),
            None => f.write_str("IBrush"),
        }
    }
}

impl<T: ObjectType + Upcast<Brush>> RefAdapter<T> {
    #[inline]
    fn brush(&self) -> &Brush {
        (*self.0).upcast()
    }
}

impl<T: ObjectType + Upcast<Brush>> IBrush for RefAdapter<T> {
    #[inline]
    fn opacity(&self) -> f64 {
        self.brush().opacity()
    }

    #[inline]
    fn transform(&self) -> Option<Rc<dyn ITransform>> {
        self.brush().transform()
    }

    #[inline]
    fn transform_origin(&self) -> RelativePoint {
        self.brush().transform_origin()
    }

    #[inline]
    fn relative_transform(&self) -> Option<Rc<dyn ITransform>> {
        self.brush().relative_transform()
    }

    fn as_any(&self) -> &dyn Any {
        self.brush()
    }

    fn as_object(&self) -> Option<&FerroObject> {
        Some(self.object())
    }

    fn as_solid_color_brush(&self) -> Option<&dyn ISolidColorBrush> {
        if self.object().is::<SolidColorBrush>() {
            Some(self)
        } else {
            None
        }
    }

    fn as_gradient_brush(&self) -> Option<&dyn IGradientBrush> {
        if self.object().is::<GradientBrush>() {
            Some(self)
        } else {
            None
        }
    }

    fn as_linear_gradient_brush(&self) -> Option<&dyn ILinearGradientBrush> {
        if self.object().is::<LinearGradientBrush>() {
            Some(self)
        } else {
            None
        }
    }

    fn as_radial_gradient_brush(&self) -> Option<&dyn IRadialGradientBrush> {
        if self.object().is::<RadialGradientBrush>() {
            Some(self)
        } else {
            None
        }
    }

    fn as_conic_gradient_brush(&self) -> Option<&dyn IConicGradientBrush> {
        if self.object().is::<ConicGradientBrush>() {
            Some(self)
        } else {
            None
        }
    }

    fn as_tile_brush(&self) -> Option<&dyn ITileBrush> {
        if self.object().is::<TileBrush>() {
            Some(self)
        } else {
            None
        }
    }

    fn as_image_brush(&self) -> Option<&dyn IImageBrush> {
        if self.object().is::<ImageBrush>() {
            Some(self)
        } else {
            None
        }
    }

    fn as_scene_brush(&self) -> Option<&dyn ISceneBrush> {
        if self.object().is::<VisualBrush>() || self.object().is::<DrawingBrush>() {
            Some(self)
        } else {
            None
        }
    }

    fn as_mutable_brush(&self) -> Option<&dyn IMutableBrush> {
        if self.object().is::<SolidColorBrush>()
            || self.object().is::<GradientBrush>()
            || self.object().is::<ImageBrush>()
        {
            Some(self)
        } else {
            None
        }
    }

    fn as_composition_render_resource(
        &self,
    ) -> Option<&dyn crate::rendering::composition::drawing::ICompositionRenderResource> {
        // Every brush class that can be instantiated has a server-side
        // counterpart; only the abstract classes have no factory.
        let brush = self.brush();
        brush.factory().map(|_| brush as &dyn crate::rendering::composition::drawing::ICompositionRenderResource)
    }

    fn reference_id(&self) -> *const () {
        RefAdapter::reference_id(self)
    }
}

impl<T: ObjectType + Upcast<Brush>> ISolidColorBrush for RefAdapter<T> {
    #[inline]
    fn color(&self) -> Color {
        self.class::<SolidColorBrush>().color()
    }
}

impl<T: ObjectType + Upcast<Brush>> IGradientBrush for RefAdapter<T> {
    fn gradient_stops(&self) -> Vec<Rc<dyn IGradientStop>> {
        self.class::<GradientBrush>().gradient_stops().iter().map(Into::into).collect()
    }

    #[inline]
    fn spread_method(&self) -> GradientSpreadMethod {
        self.class::<GradientBrush>().spread_method()
    }
}

impl<T: ObjectType + Upcast<Brush>> ILinearGradientBrush for RefAdapter<T> {
    #[inline]
    fn start_point(&self) -> RelativePoint {
        self.class::<LinearGradientBrush>().start_point()
    }

    #[inline]
    fn end_point(&self) -> RelativePoint {
        self.class::<LinearGradientBrush>().end_point()
    }
}

impl<T: ObjectType + Upcast<Brush>> IRadialGradientBrush for RefAdapter<T> {
    #[inline]
    fn center(&self) -> RelativePoint {
        self.class::<RadialGradientBrush>().center()
    }

    #[inline]
    fn gradient_origin(&self) -> RelativePoint {
        self.class::<RadialGradientBrush>().gradient_origin()
    }

    #[inline]
    fn radius_x(&self) -> RelativeScalar {
        self.class::<RadialGradientBrush>().radius_x()
    }

    #[inline]
    fn radius_y(&self) -> RelativeScalar {
        self.class::<RadialGradientBrush>().radius_y()
    }
}

impl<T: ObjectType + Upcast<Brush>> IConicGradientBrush for RefAdapter<T> {
    #[inline]
    fn center(&self) -> RelativePoint {
        self.class::<ConicGradientBrush>().center()
    }

    #[inline]
    fn angle(&self) -> f64 {
        self.class::<ConicGradientBrush>().angle()
    }
}

impl<T: ObjectType + Upcast<Brush>> IMutableBrush for RefAdapter<T> {
    fn to_immutable(&self) -> Rc<dyn IImmutableBrush> {
        let object = self.object();
        if let Some(solid) = object.downcast_ref::<SolidColorBrush>() {
            solid.to_immutable()
        } else if let Some(image) = object.downcast_ref::<ImageBrush>() {
            image.to_immutable()
        } else {
            self.class::<GradientBrush>().to_immutable()
        }
    }
}

impl<T: ObjectType + Upcast<Brush>> From<Ref<T>> for Rc<dyn IBrush> {
    #[inline]
    fn from(value: Ref<T>) -> Self {
        Rc::new(RefAdapter(value))
    }
}

impl<T: ObjectType + Upcast<Brush>> From<&Ref<T>> for Rc<dyn IBrush> {
    #[inline]
    fn from(value: &Ref<T>) -> Self {
        Rc::new(RefAdapter(value.clone()))
    }
}
