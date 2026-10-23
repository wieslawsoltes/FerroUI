//! The WinUI geometries of the shapes: `CompositionGeometry` and its
//! subclasses `CompositionRectangleGeometry`,
//! `CompositionRoundedRectangleGeometry`, `CompositionEllipseGeometry`,
//! `CompositionLineGeometry` and `CompositionPathGeometry`, and the
//! `CompositionPath` a path geometry shows.
//!
//! As for the clips, every geometry is one [`CompositionGeometry`] behind an
//! `Rc`, which holds what the concrete class adds in a kind; the concrete
//! classes are typed handles around the same `Rc`. The server builds the
//! platform geometry from the members and caches it until a member changes.

use super::composition_object_impls::composition_object_impls;
use super::generated::{
    CompositionEllipseGeometryHooks, CompositionEllipseGeometryProps, CompositionGeometryHooks,
    CompositionGeometryProps, CompositionLineGeometryHooks, CompositionLineGeometryProps,
    CompositionPathGeometryHooks, CompositionPathGeometryProps, CompositionRectangleGeometryHooks,
    CompositionRectangleGeometryProps, CompositionRoundedRectangleGeometryHooks,
    CompositionRoundedRectangleGeometryProps, ServerCompositionEllipseGeometryProps,
    ServerCompositionLineGeometryProps, ServerCompositionPathGeometryProps, ServerCompositionRectangleGeometryProps,
    ServerCompositionRoundedRectangleGeometryProps,
};
use super::server::{
    IServerObject, ServerCompositionEllipseGeometry, ServerCompositionLineGeometry, ServerCompositionPathGeometry,
    ServerCompositionRectangleGeometry, ServerCompositionRoundedRectangleGeometry, ServerCompositor, ServerObjectId,
};
use super::{CompositionObject, Compositor};
use crate::media::Geometry;
use crate::platform::IGeometryImpl;
use crate::utilities::FormatError;
use crate::{Rect, Vector};
use std::ops::Deref;
use std::rc::{Rc, Weak};
use std::sync::Arc;

/// The path of a [`CompositionPathGeometry`] (WinUI
/// `Microsoft.UI.Composition.CompositionPath`): an immutable platform
/// geometry, made from a FerroUI [`Geometry`] (a `StreamGeometry`, a
/// `PathGeometry`, ...) or from a string of path data.
///
/// Deviation (winui-composition.md): WinUI makes a path from an
/// `IGeometrySource2D` (a Win2D geometry); here the source is a FerroUI
/// geometry, and the path keeps the platform geometry it had when the path
/// was made.
#[derive(Clone)]
pub struct CompositionPath {
    geometry: Arc<dyn IGeometryImpl>,
}

impl CompositionPath {
    /// A path over a platform geometry.
    pub fn new(geometry: Arc<dyn IGeometryImpl>) -> CompositionPath {
        CompositionPath { geometry }
    }

    /// The path of `geometry`, as it is now; `None` when the geometry has no
    /// platform geometry.
    pub fn from_geometry(geometry: &Geometry) -> Option<CompositionPath> {
        geometry.platform_impl().map(CompositionPath::new)
    }

    /// The path described by a string of path data (the mini-language of
    /// `Geometry::parse`).
    pub fn parse(data: &str) -> Result<CompositionPath, FormatError> {
        let geometry = Geometry::parse(data)?;
        let platform = geometry.platform_impl().expect("a parsed stream geometry has a platform geometry");
        Ok(CompositionPath::new(platform))
    }

    /// The platform geometry of the path.
    pub fn geometry(&self) -> &Arc<dyn IGeometryImpl> {
        &self.geometry
    }

    /// The bounds of the path.
    pub fn bounds(&self) -> Rect {
        self.geometry.bounds()
    }
}

impl std::fmt::Debug for CompositionPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositionPath").field("bounds", &self.geometry.bounds()).finish()
    }
}

/// What the concrete class of a geometry adds to [`CompositionGeometry`].
pub(crate) enum CompositionGeometryKind {
    Rectangle(CompositionRectangleGeometryProps),
    RoundedRectangle(CompositionRoundedRectangleGeometryProps),
    Ellipse(CompositionEllipseGeometryProps),
    Line(CompositionLineGeometryProps),
    Path(CompositionPathGeometryProps),
}

/// The geometry of a sprite shape or a geometric clip (WinUI
/// `Microsoft.UI.Composition.CompositionGeometry`), with the animatable
/// trimming of its outline: `TrimStart` and `TrimEnd` (fractions of the
/// length of the outline, default 0 and 1) and `TrimOffset` (added to both,
/// wrapping around the outline).
pub struct CompositionGeometry {
    this: Weak<CompositionGeometry>,
    object: CompositionObject,
    kind: CompositionGeometryKind,
}

impl CompositionGeometry {
    fn create(
        compositor: &Rc<Compositor>,
        kind: CompositionGeometryKind,
        server: impl FnOnce(&Rc<ServerCompositor>) -> Rc<dyn IServerObject> + Send + 'static,
    ) -> Rc<CompositionGeometry> {
        let server = compositor.create_server_object(move |compositor, _| server(compositor));
        let geometry = Rc::new_cyclic(|this: &Weak<CompositionGeometry>| CompositionGeometry {
            this: this.clone(),
            object: CompositionObject::new(compositor, Some(server)),
            kind,
        });
        match &geometry.kind {
            CompositionGeometryKind::Rectangle(props) => props.initialize_defaults(&*geometry),
            CompositionGeometryKind::RoundedRectangle(props) => props.initialize_defaults(&*geometry),
            CompositionGeometryKind::Ellipse(props) => props.initialize_defaults(&*geometry),
            CompositionGeometryKind::Line(props) => props.initialize_defaults(&*geometry),
            CompositionGeometryKind::Path(props) => props.initialize_defaults(&*geometry),
        }
        geometry
    }

    /// The generated properties of `CompositionGeometry`.
    pub fn geometry_props(&self) -> &CompositionGeometryProps {
        match &self.kind {
            CompositionGeometryKind::Rectangle(props) => props.base(),
            CompositionGeometryKind::RoundedRectangle(props) => props.base(),
            CompositionGeometryKind::Ellipse(props) => props.base(),
            CompositionGeometryKind::Line(props) => props.base(),
            CompositionGeometryKind::Path(props) => props.base(),
        }
    }

    pub fn compositor(&self) -> &Rc<Compositor> {
        self.object.compositor()
    }

    /// The embedded `CompositionObject`.
    pub fn object(&self) -> &CompositionObject {
        &self.object
    }

    /// The id of the server-side geometry.
    pub fn server(&self) -> ServerObjectId {
        self.object.required_server()
    }

    pub fn is_disposed(&self) -> bool {
        self.object.is_disposed()
    }

    pub fn dispose(&self) {
        self.object.dispose();
    }

    /// The name of the concrete class, for messages.
    pub fn type_name(&self) -> &'static str {
        match &self.kind {
            CompositionGeometryKind::Rectangle(_) => "CompositionRectangleGeometry",
            CompositionGeometryKind::RoundedRectangle(_) => "CompositionRoundedRectangleGeometry",
            CompositionGeometryKind::Ellipse(_) => "CompositionEllipseGeometry",
            CompositionGeometryKind::Line(_) => "CompositionLineGeometry",
            CompositionGeometryKind::Path(_) => "CompositionPathGeometry",
        }
    }

    /// Where the drawn part of the outline ends, as a fraction of its length
    /// (default 1).
    pub fn trim_end(&self) -> f32 {
        self.geometry_props().trim_end()
    }

    pub fn set_trim_end(&self, value: f32) {
        self.geometry_props().set_trim_end(self, value)
    }

    /// How far along the outline both trim points are moved, as a fraction
    /// of its length; the drawn part wraps around the start of the outline.
    pub fn trim_offset(&self) -> f32 {
        self.geometry_props().trim_offset()
    }

    pub fn set_trim_offset(&self, value: f32) {
        self.geometry_props().set_trim_offset(self, value)
    }

    /// Where the drawn part of the outline starts, as a fraction of its
    /// length (default 0).
    pub fn trim_start(&self) -> f32 {
        self.geometry_props().trim_start()
    }

    pub fn set_trim_start(&self, value: f32) {
        self.geometry_props().set_trim_start(self, value)
    }
}

impl CompositionGeometryHooks for CompositionGeometry {}

impl CompositionRectangleGeometryHooks for CompositionGeometry {}

impl CompositionRoundedRectangleGeometryHooks for CompositionGeometry {}

impl CompositionEllipseGeometryHooks for CompositionGeometry {}

impl CompositionLineGeometryHooks for CompositionGeometry {}

impl CompositionPathGeometryHooks for CompositionGeometry {}

composition_object_impls!(
    CompositionGeometry, "CompositionGeometry",
    serialize: |this, writer| match &this.kind {
        CompositionGeometryKind::Rectangle(props) => props.serialize_changes_core(this, writer),
        CompositionGeometryKind::RoundedRectangle(props) => props.serialize_changes_core(this, writer),
        CompositionGeometryKind::Ellipse(props) => props.serialize_changes_core(this, writer),
        CompositionGeometryKind::Line(props) => props.serialize_changes_core(this, writer),
        CompositionGeometryKind::Path(props) => props.serialize_changes_core(this, writer),
    },
    start_animation: |this, name, animation, final_value| match &this.kind {
        CompositionGeometryKind::Rectangle(props) => props.start_animation(this, name, animation, final_value),
        CompositionGeometryKind::RoundedRectangle(props) => props.start_animation(this, name, animation, final_value),
        CompositionGeometryKind::Ellipse(props) => props.start_animation(this, name, animation, final_value),
        CompositionGeometryKind::Line(props) => props.start_animation(this, name, animation, final_value),
        CompositionGeometryKind::Path(props) => props.start_animation(this, name, animation, final_value),
    },
    composition_property: |this, name| match &this.kind {
        CompositionGeometryKind::Rectangle(_) => ServerCompositionRectangleGeometryProps::get_composition_property(name),
        CompositionGeometryKind::RoundedRectangle(_) => {
            ServerCompositionRoundedRectangleGeometryProps::get_composition_property(name)
        }
        CompositionGeometryKind::Ellipse(_) => ServerCompositionEllipseGeometryProps::get_composition_property(name),
        CompositionGeometryKind::Line(_) => ServerCompositionLineGeometryProps::get_composition_property(name),
        CompositionGeometryKind::Path(_) => ServerCompositionPathGeometryProps::get_composition_property(name),
    },
);

/// Declares the typed handle of a geometry class: the struct, `Deref` to
/// the shared geometry, `from_geometry` and the accessor of its properties.
macro_rules! geometry_handle {
    ($(#[$meta:meta])* $name:ident, $variant:ident, $props:ident, $what:literal) => {
        $(#[$meta])*
        #[derive(Clone)]
        pub struct $name(Rc<CompositionGeometry>);

        impl Deref for $name {
            type Target = Rc<CompositionGeometry>;

            fn deref(&self) -> &Rc<CompositionGeometry> {
                &self.0
            }
        }

        impl $name {
            #[doc = concat!("The handle of `geometry` as ", $what, ", if it is one.")]
            pub fn from_geometry(geometry: &Rc<CompositionGeometry>) -> Option<$name> {
                matches!(geometry.kind, CompositionGeometryKind::$variant(_)).then(|| $name(geometry.clone()))
            }

            /// The geometry as a [`CompositionGeometry`].
            pub fn geometry(&self) -> &Rc<CompositionGeometry> {
                &self.0
            }

            fn props(&self) -> &$props {
                match &self.0.kind {
                    CompositionGeometryKind::$variant(props) => props,
                    _ => unreachable!(concat!("the handle wraps ", $what)),
                }
            }
        }
    };
}

geometry_handle!(
    /// A rectangle (WinUI `Microsoft.UI.Composition.CompositionRectangleGeometry`):
    /// `(offset, size)`, both animatable (`Offset`, `Size`).
    CompositionRectangleGeometry,
    Rectangle,
    CompositionRectangleGeometryProps,
    "a rectangle geometry"
);

impl CompositionRectangleGeometry {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> CompositionRectangleGeometry {
        CompositionRectangleGeometry(CompositionGeometry::create(
            compositor,
            CompositionGeometryKind::Rectangle(CompositionRectangleGeometryProps::new()),
            |c| ServerCompositionRectangleGeometry::new(c),
        ))
    }

    /// The top left corner of the rectangle.
    pub fn offset(&self) -> Vector {
        self.props().offset()
    }

    pub fn set_offset(&self, value: Vector) {
        self.props().set_offset(&*self.0, value)
    }

    /// The size of the rectangle.
    pub fn size(&self) -> Vector {
        self.props().size()
    }

    pub fn set_size(&self, value: Vector) {
        self.props().set_size(&*self.0, value)
    }
}

geometry_handle!(
    /// A rectangle with rounded corners (WinUI
    /// `Microsoft.UI.Composition.CompositionRoundedRectangleGeometry`):
    /// `(offset, size)` with the elliptical radii `corner_radius`, all
    /// animatable (`CornerRadius`, `Offset`, `Size`). The radii are limited to
    /// half the size.
    CompositionRoundedRectangleGeometry,
    RoundedRectangle,
    CompositionRoundedRectangleGeometryProps,
    "a rounded rectangle geometry"
);

impl CompositionRoundedRectangleGeometry {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> CompositionRoundedRectangleGeometry {
        CompositionRoundedRectangleGeometry(CompositionGeometry::create(
            compositor,
            CompositionGeometryKind::RoundedRectangle(CompositionRoundedRectangleGeometryProps::new()),
            |c| ServerCompositionRoundedRectangleGeometry::new(c),
        ))
    }

    /// The radii of the corners.
    pub fn corner_radius(&self) -> Vector {
        self.props().corner_radius()
    }

    pub fn set_corner_radius(&self, value: Vector) {
        self.props().set_corner_radius(&*self.0, value)
    }

    /// The top left corner of the rectangle.
    pub fn offset(&self) -> Vector {
        self.props().offset()
    }

    pub fn set_offset(&self, value: Vector) {
        self.props().set_offset(&*self.0, value)
    }

    /// The size of the rectangle.
    pub fn size(&self) -> Vector {
        self.props().size()
    }

    pub fn set_size(&self, value: Vector) {
        self.props().set_size(&*self.0, value)
    }
}

geometry_handle!(
    /// An ellipse (WinUI `Microsoft.UI.Composition.CompositionEllipseGeometry`):
    /// its `center` and its radii `radius`, both animatable (`Center`,
    /// `Radius`).
    CompositionEllipseGeometry,
    Ellipse,
    CompositionEllipseGeometryProps,
    "an ellipse geometry"
);

impl CompositionEllipseGeometry {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> CompositionEllipseGeometry {
        CompositionEllipseGeometry(CompositionGeometry::create(
            compositor,
            CompositionGeometryKind::Ellipse(CompositionEllipseGeometryProps::new()),
            |c| ServerCompositionEllipseGeometry::new(c),
        ))
    }

    /// The centre of the ellipse.
    pub fn center(&self) -> Vector {
        self.props().center()
    }

    pub fn set_center(&self, value: Vector) {
        self.props().set_center(&*self.0, value)
    }

    /// The radii of the ellipse.
    pub fn radius(&self) -> Vector {
        self.props().radius()
    }

    pub fn set_radius(&self, value: Vector) {
        self.props().set_radius(&*self.0, value)
    }
}

geometry_handle!(
    /// A line (WinUI `Microsoft.UI.Composition.CompositionLineGeometry`) from
    /// `start` to `end`, both animatable (`Start`, `End`).
    CompositionLineGeometry,
    Line,
    CompositionLineGeometryProps,
    "a line geometry"
);

impl CompositionLineGeometry {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> CompositionLineGeometry {
        CompositionLineGeometry(CompositionGeometry::create(
            compositor,
            CompositionGeometryKind::Line(CompositionLineGeometryProps::new()),
            |c| ServerCompositionLineGeometry::new(c),
        ))
    }

    /// The start point of the line.
    pub fn start(&self) -> Vector {
        self.props().start()
    }

    pub fn set_start(&self, value: Vector) {
        self.props().set_start(&*self.0, value)
    }

    /// The end point of the line.
    pub fn end(&self) -> Vector {
        self.props().end()
    }

    pub fn set_end(&self, value: Vector) {
        self.props().set_end(&*self.0, value)
    }
}

geometry_handle!(
    /// A geometry that shows a [`CompositionPath`] (WinUI
    /// `Microsoft.UI.Composition.CompositionPathGeometry`).
    CompositionPathGeometry,
    Path,
    CompositionPathGeometryProps,
    "a path geometry"
);

impl CompositionPathGeometry {
    pub(crate) fn new(compositor: &Rc<Compositor>) -> CompositionPathGeometry {
        CompositionPathGeometry(CompositionGeometry::create(
            compositor,
            CompositionGeometryKind::Path(CompositionPathGeometryProps::new()),
            |c| ServerCompositionPathGeometry::new(c),
        ))
    }

    pub(crate) fn with_path(compositor: &Rc<Compositor>, path: Option<CompositionPath>) -> CompositionPathGeometry {
        let geometry = Self::new(compositor);
        geometry.set_path(path);
        geometry
    }

    /// The path the geometry shows.
    pub fn path(&self) -> Option<CompositionPath> {
        self.props().path().map(|path| (*path).clone())
    }

    pub fn set_path(&self, value: Option<CompositionPath>) {
        self.props().set_path(&*self.0, value.map(Arc::new))
    }
}
