use crate::media::{Geometry, GeometryImpl};
use crate::platform::IGeometryImpl;
use crate::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use std::rc::Rc;

/// A geometry with a fixed platform implementation.
#[repr(C)]
pub struct ImmutableGeometry {
    base: Geometry,
}

ferro_class!(ImmutableGeometry: Geometry);
ferro_impl_classes!(ImmutableGeometry: FerroObjectImpl);

impl GeometryImpl for ImmutableGeometry {
    fn clone_geometry(this: &Self) -> Ref<Geometry> {
        ImmutableGeometry::new(this.platform_impl()).upcast()
    }

    fn create_defining_geometry(this: &Self) -> Option<Rc<dyn IGeometryImpl>> {
        this.platform_impl()
    }
}

impl ImmutableGeometry {
    pub fn new(platform_impl: Option<Rc<dyn IGeometryImpl>>) -> Ref<Self> {
        instantiate(Self { base: Geometry::construct_with_impl(platform_impl) })
    }
}
