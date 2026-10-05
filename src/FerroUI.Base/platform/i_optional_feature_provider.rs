use std::any::{Any, TypeId};
use std::rc::Rc;

/// Something that can expose optional, backend-specific features by type.
///
/// Features are looked up by the `TypeId` of the feature's type, which may be
/// a trait object type (`TypeId::of::<dyn IFoo>()`); the returned value then
/// holds an `Rc<dyn IFoo>`.
pub trait IOptionalFeatureProvider {
    /// Queries for an optional feature.
    fn try_get_feature(&self, feature_type: TypeId) -> Option<Rc<dyn Any>>;
}

impl dyn IOptionalFeatureProvider + '_ {
    /// Queries for an optional feature registered as `Rc<T>`.
    pub fn try_get<T: ?Sized + 'static>(&self) -> Option<Rc<T>> {
        let feature = self.try_get_feature(TypeId::of::<T>())?;
        feature.downcast_ref::<Rc<T>>().cloned()
    }
}
