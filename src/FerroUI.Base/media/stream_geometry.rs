use crate::media::{Geometry, GeometryImpl, PathMarkupParser, StreamGeometryContext};
use crate::platform::{self, IGeometryContext, IGeometryImpl, IStreamGeometryImpl};
use crate::utilities::FormatError;
use crate::{ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, Ref};
use std::cell::RefCell;
use std::rc::Rc;

/// Represents the geometry of an arbitrarily complex shape.
#[repr(C)]
pub struct StreamGeometry {
    base: Geometry,
    impl_: RefCell<Option<Rc<dyn IStreamGeometryImpl>>>,
}

ferro_class!(StreamGeometry: Geometry);
crate::ferro_class_info!(StreamGeometry { new: StreamGeometry::new });
ferro_impl_classes!(StreamGeometry: FerroObjectImpl);

impl GeometryImpl for StreamGeometry {
    fn clone_geometry(this: &Self) -> Ref<Geometry> {
        let platform_impl = this.platform_impl().expect("stream geometry has a platform implementation");
        let stream = platform_impl.as_stream_geometry().expect("platform implementation is a stream geometry");
        StreamGeometry::with_impl(stream.clone_geometry()).upcast()
    }

    fn create_defining_geometry(this: &Self) -> Option<Rc<dyn IGeometryImpl>> {
        let mut impl_ = this.impl_.borrow_mut();
        let geometry = impl_.get_or_insert_with(|| platform::render_interface().create_stream_geometry()).clone();
        Some(geometry)
    }
}

impl StreamGeometry {
    /// Creates the class data.
    pub fn construct() -> Self {
        Self { base: Geometry::construct(), impl_: RefCell::new(None) }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates a stream geometry over a platform implementation.
    fn with_impl(impl_: Rc<dyn IStreamGeometryImpl>) -> Ref<Self> {
        instantiate(Self { base: Geometry::construct(), impl_: RefCell::new(Some(impl_)) })
    }

    /// Creates a stream geometry from a string of path data.
    pub fn parse(s: &str) -> Result<Ref<StreamGeometry>, FormatError> {
        let stream_geometry = StreamGeometry::new();

        let mut context = stream_geometry.open();
        let result = PathMarkupParser::new(&mut context).parse(s);
        context.dispose();
        result?;

        Ok(stream_geometry)
    }

    /// Opens the geometry to start defining it.
    ///
    /// Returns a [`StreamGeometryContext`] which can be used to define the
    /// geometry.
    pub fn open(&self) -> StreamGeometryContext {
        let platform_impl = self.platform_impl().expect("stream geometry has a platform implementation");
        let stream = platform_impl.as_stream_geometry().expect("platform implementation is a stream geometry");
        StreamGeometryContext::new(stream.open())
    }
}
