use crate::media::{Geometry, GeometryImpl};
use crate::platform::IGeometryImpl;
use crate::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use std::rc::Rc;

/// A geometry that wraps a platform implementation.
#[repr(C)]
pub struct PlatformGeometry {
    base: Geometry,
    geometry_impl: Rc<dyn IGeometryImpl>,
}

ferro_class!(PlatformGeometry: Geometry);
ferro_impl_classes!(PlatformGeometry: FerroObjectImpl);

impl GeometryImpl for PlatformGeometry {
    fn clone_geometry(this: &Self) -> Ref<Geometry> {
        PlatformGeometry::new(this.geometry_impl.clone()).upcast()
    }

    fn create_defining_geometry(this: &Self) -> Option<Rc<dyn IGeometryImpl>> {
        Some(this.geometry_impl.clone())
    }
}

impl PlatformGeometry {
    pub fn new(geometry_impl: Rc<dyn IGeometryImpl>) -> Ref<Self> {
        instantiate(Self { base: Geometry::construct(), geometry_impl })
    }
}
