use crate::media::immutable::ImmutableDashStyle;
use crate::media::DashStyle;
use crate::media::ref_adapter::RefAdapter;
use crate::{FerroObject, ObjectType, Ref, Upcast};
use std::any::Any;
use std::rc::Rc;

/// Represents the sequence of dashes and gaps that will be applied by a pen.
///
/// Implemented by [`ImmutableDashStyle`] and, through an adapter, by handles ([`Ref`]) of
/// [`DashStyle`].
pub trait IDashStyle: 'static {
    /// The length of alternating dashes and gaps.
    fn dashes(&self) -> Option<Vec<f64>>;

    /// How far in the dash sequence the stroke will start.
    fn offset(&self) -> f64;

    /// The implementing value, for downcasts to [`ImmutableDashStyle`].
    fn as_any(&self) -> &dyn Any;

    /// The object behind the dash style when it is a mutable [`DashStyle`].
    fn as_object(&self) -> Option<&FerroObject> {
        None
    }

    /// Converts the dash style to an immutable dash style; an immutable dash
    /// style returns itself.
    fn into_immutable_dash_style(self: Rc<Self>) -> Rc<ImmutableDashStyle>;

    /// The identity of the dash style, used for reference equality.
    #[doc(hidden)]
    fn reference_id(&self) -> *const () {
        self as *const Self as *const ()
    }

    /// Whether the dash style equals `other`. Mutable dash styles compare by
    /// reference, immutable ones structurally.
    fn equals(&self, other: &dyn IDashStyle) -> bool {
        self.reference_id() == other.reference_id()
    }
}

impl PartialEq for dyn IDashStyle {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.equals(other)
    }
}

impl std::fmt::Debug for dyn IDashStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IDashStyle").field("dashes", &self.dashes()).field("offset", &self.offset()).finish()
    }
}

impl<T: ObjectType + Upcast<DashStyle>> IDashStyle for RefAdapter<T> {
    fn dashes(&self) -> Option<Vec<f64>> {
        Upcast::<DashStyle>::upcast(&*self.0).dashes().map(|dashes| dashes.to_vec())
    }

    #[inline]
    fn offset(&self) -> f64 {
        Upcast::<DashStyle>::upcast(&*self.0).offset()
    }

    fn as_any(&self) -> &dyn Any {
        Upcast::<DashStyle>::upcast(&*self.0)
    }

    fn as_object(&self) -> Option<&FerroObject> {
        Some(self.object())
    }

    fn into_immutable_dash_style(self: Rc<Self>) -> Rc<ImmutableDashStyle> {
        Rc::new(Upcast::<DashStyle>::upcast(&*self.0).to_immutable())
    }

    fn reference_id(&self) -> *const () {
        RefAdapter::reference_id(self)
    }
}

impl<T: ObjectType + Upcast<DashStyle>> From<Ref<T>> for Rc<dyn IDashStyle> {
    #[inline]
    fn from(value: Ref<T>) -> Self {
        Rc::new(RefAdapter(value))
    }
}

impl<T: ObjectType + Upcast<DashStyle>> From<&Ref<T>> for Rc<dyn IDashStyle> {
    #[inline]
    fn from(value: &Ref<T>) -> Self {
        Rc::new(RefAdapter(value.clone()))
    }
}
