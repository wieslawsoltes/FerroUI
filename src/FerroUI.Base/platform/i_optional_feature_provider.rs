use std::any::{Any, TypeId};
use std::rc::Rc;
use std::sync::Arc;

/// Something that can expose optional, backend-specific features by type.
///
/// Features are looked up by the `TypeId` of the feature's type, which may be
/// a trait object type (`TypeId::of::<dyn IFoo>()`); the returned value then
/// holds an `Rc<dyn IFoo>`, or an `Arc<dyn IFoo>` for a feature that is
/// shared by threads.
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

    /// Queries for an optional feature registered as `Arc<T>`: one that is
    /// shared by threads.
    pub fn try_get_shared<T: ?Sized + 'static>(&self) -> Option<Arc<T>> {
        let feature = self.try_get_feature(TypeId::of::<T>())?;
        feature.downcast_ref::<Arc<T>>().cloned()
    }
}
