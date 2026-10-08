use crate::media::immutable::ImmutablePen;
use crate::media::{IBrush, IDashStyle, Pen, PenLineCap, PenLineJoin};
use crate::media::ref_adapter::RefAdapter;
use crate::{FerroObject, ObjectType, Ref, Upcast};
use std::any::Any;
use std::rc::Rc;

/// Describes how a stroke is drawn.
///
/// Implemented by [`ImmutablePen`] and, through an adapter, by handles ([`Ref`]) of [`Pen`].
pub trait IPen: 'static {
    /// The brush used to draw the stroke.
    fn brush(&self) -> Option<Rc<dyn IBrush>>;

    /// The style of dashed lines drawn with the pen.
    fn dash_style(&self) -> Option<Rc<dyn IDashStyle>>;

    /// The type of shape to use on both ends of a line.
    fn line_cap(&self) -> PenLineCap;

    /// A value describing how to join consecutive line or curve segments.
    fn line_join(&self) -> PenLineJoin;

    /// The limit of the ratio of the miter length to half this pen's
    /// thickness.
    fn miter_limit(&self) -> f64;

    /// The stroke thickness.
    fn thickness(&self) -> f64;

    /// The implementing value, for downcasts to [`ImmutablePen`].
    fn as_any(&self) -> &dyn Any;

    /// The pen viewed as an [`ImmutablePen`], when it is one (the `is
    /// ImmutablePen` test of upstream, which a class deriving from it
    /// passes).
    fn as_immutable_pen(&self) -> Option<&ImmutablePen> {
        self.as_any().downcast_ref::<ImmutablePen>()
    }

    /// The object behind the pen when it is a mutable [`Pen`].
    fn as_object(&self) -> Option<&FerroObject> {
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

    /// Converts the pen to an immutable pen; an immutable pen returns itself.
    fn into_immutable_pen(self: Rc<Self>) -> Rc<ImmutablePen>;

    /// The form of the pen that is sent to the render thread, when the
    /// type keeps one: see [`SharedPen`](crate::media::SharedPen).
    fn to_shared(&self) -> Option<crate::media::SharedPen> {
        None
    }

    /// The identity of the pen, used for reference equality.
    #[doc(hidden)]
    fn reference_id(&self) -> *const () {
        self as *const Self as *const ()
    }

    /// Whether the pen equals `other`. Mutable pens compare by reference,
    /// immutable ones structurally (against any pen).
    fn equals(&self, other: &dyn IPen) -> bool {
        self.reference_id() == other.reference_id()
    }
}

impl PartialEq for dyn IPen {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.equals(other)
    }
}

impl std::fmt::Debug for dyn IPen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IPen")
            .field("brush", &self.brush())
            .field("thickness", &self.thickness())
            .field("dash_style", &self.dash_style())
            .field("line_cap", &self.line_cap())
            .field("line_join", &self.line_join())
            .field("miter_limit", &self.miter_limit())
            .finish()
    }
}

impl<T: ObjectType + Upcast<Pen>> IPen for RefAdapter<T> {
    fn brush(&self) -> Option<Rc<dyn IBrush>> {
        Upcast::<Pen>::upcast(&*self.0).brush()
    }

    fn dash_style(&self) -> Option<Rc<dyn IDashStyle>> {
        Upcast::<Pen>::upcast(&*self.0).dash_style()
    }

    #[inline]
    fn line_cap(&self) -> PenLineCap {
        Upcast::<Pen>::upcast(&*self.0).line_cap()
    }

    #[inline]
    fn line_join(&self) -> PenLineJoin {
        Upcast::<Pen>::upcast(&*self.0).line_join()
    }

    #[inline]
    fn miter_limit(&self) -> f64 {
        Upcast::<Pen>::upcast(&*self.0).miter_limit()
    }

    #[inline]
    fn thickness(&self) -> f64 {
        Upcast::<Pen>::upcast(&*self.0).thickness()
    }

    fn as_any(&self) -> &dyn Any {
        Upcast::<Pen>::upcast(&*self.0)
    }

    fn as_object(&self) -> Option<&FerroObject> {
        Some(self.object())
    }

    fn as_composition_render_resource(
        &self,
    ) -> Option<&dyn crate::rendering::composition::drawing::ICompositionRenderResource> {
        Some(Upcast::<Pen>::upcast(&*self.0))
    }

    fn into_immutable_pen(self: Rc<Self>) -> Rc<ImmutablePen> {
        Rc::new(Upcast::<Pen>::upcast(&*self.0).to_immutable())
    }

    fn reference_id(&self) -> *const () {
        RefAdapter::reference_id(self)
    }
}

impl<T: ObjectType + Upcast<Pen>> From<Ref<T>> for Rc<dyn IPen> {
    #[inline]
    fn from(value: Ref<T>) -> Self {
        Rc::new(RefAdapter(value))
    }
}

impl<T: ObjectType + Upcast<Pen>> From<&Ref<T>> for Rc<dyn IPen> {
    #[inline]
    fn from(value: &Ref<T>) -> Self {
        Rc::new(RefAdapter(value.clone()))
    }
}
