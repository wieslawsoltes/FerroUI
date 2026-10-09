use crate::media::{
    CombinedGeometry, GeometryCombineMode, IPen, ImmutableGeometry, IntersectionResult, PlatformGeometry,
    RectangleGeometry, StreamGeometry, Transform,
};
use crate::media::ref_adapter::RefAdapter;
use crate::platform::IGeometryImpl;
use crate::reactive::{Disposable, IDisposable};
use crate::rendering::composition::drawing::{CompositorResourceHolder, ICompositionRenderResource};
use crate::rendering::composition::generated::ServerCompositionSimpleGeometryProps;
use crate::rendering::composition::server::{IServerObject, ServerCompositionSimpleGeometry, ServerObjectId};
use crate::rendering::composition::transport::BatchStreamWriter;
use crate::rendering::composition::{Compositor, ICompositorSerializable};
use crate::utilities::{FormatError, HandlerList};
use crate::{
    ferro_class, ferro_property, FerroObject, FerroObjectImpl, FerroProperty,
    FerroPropertyChangedEventArgs, IntoRef, Matrix, Nullable, Point, Rect, Ref, StyledProperty, Upcast,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

/// Defines a geometric shape.
#[repr(C)]
pub struct Geometry {
    base: FerroObject,
    is_dirty: Cell<bool>,
    can_invalidate: bool,
    platform_impl: RefCell<Option<Arc<dyn IGeometryImpl>>>,
    changed: HandlerList<dyn Fn()>,
    transform_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
    resource: CompositorResourceHolder,
}

ferro_class! {
    Geometry: FerroObject, virtuals GeometryImpl: FerroObjectImpl {
        /// Clones the geometry.
        fn clone_geometry(this) -> Ref<Geometry>;

        /// Creates the platform implementation of the geometry, without the
        /// transform applied.
        fn create_defining_geometry(this) -> Option<Arc<dyn IGeometryImpl>>;
    }
}

crate::ferro_impl_classes!(Geometry: FerroObjectImpl);

impl GeometryImpl for Geometry {
    fn clone_geometry(_this: &Self) -> Ref<Geometry> {
        panic!("Geometry is abstract: 'clone_geometry' must be implemented by the deriving class")
    }

    fn create_defining_geometry(_this: &Self) -> Option<Arc<dyn IGeometryImpl>> {
        panic!("Geometry is abstract: 'create_defining_geometry' must be implemented by the deriving class")
    }
}

crate::ferro_properties! { impl Geometry {
    ferro_property!(pub fn transform_property() -> StyledProperty<Option<Ref<Transform>>> {
        FerroProperty::register::<Geometry, _>("Transform", None)
    });
} }

impl Geometry {
    fn static_constructor() {
        Self::transform_property().changed().add_class_handler::<Geometry>(|x, e| x.transform_property_changed(e));
    }

    /// Creates the class data of a geometry whose platform implementation is
    /// created on demand.
    pub fn construct() -> Self {
        Self {
            base: FerroObject::construct(),
            is_dirty: Cell::new(true),
            can_invalidate: true,
            platform_impl: RefCell::new(None),
            changed: HandlerList::new(),
            transform_subscription: RefCell::new(None),
            resource: CompositorResourceHolder::new(),
        }
    }

    /// Creates the class data of a geometry with a fixed platform
    /// implementation, which is never invalidated.
    pub fn construct_with_impl(platform_impl: Option<Arc<dyn IGeometryImpl>>) -> Self {
        Self {
            base: FerroObject::construct(),
            is_dirty: Cell::new(false),
            can_invalidate: false,
            platform_impl: RefCell::new(platform_impl),
            changed: HandlerList::new(),
            transform_subscription: RefCell::new(None),
            resource: CompositorResourceHolder::new(),
        }
    }

    /// The number of subscribers to the changes of the geometry (a
    /// diagnostics member, used by tests to verify that subscriptions are
    /// released).
    pub fn changed_subscriber_count(&self) -> usize {
        self.changed.len()
    }

    /// Subscribes to changes of the geometry. Disposing the returned handle
    /// unsubscribes.
    pub fn changed(&self, handler: impl Fn() + 'static) -> Rc<dyn IDisposable> {
        let token = self.changed.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.changed.remove(token);
            }
        })
    }

    fn raise_changed(&self) {
        if self.changed.is_empty() {
            return;
        }
        for (_, handler) in self.changed.snapshot().iter() {
            handler();
        }
    }

    /// The geometry's bounding rectangle.
    pub fn bounds(&self) -> Rect {
        self.platform_impl().map(|g| g.bounds()).unwrap_or_default()
    }

    /// The platform-specific implementation of the geometry.
    pub fn platform_impl(&self) -> Option<Arc<dyn IGeometryImpl>> {
        if self.is_dirty.get() {
            let mut geometry = self.create_defining_geometry();
            let transform = self.transform();

            if let (Some(defining), Some(transform)) = (&geometry, &transform) {
                let value = transform.value();
                if value != Matrix::IDENTITY {
                    let transformed: Arc<dyn IGeometryImpl> = defining.with_transform(value);
                    geometry = Some(transformed);
                }
            }

            *self.platform_impl.borrow_mut() = geometry;
            self.is_dirty.set(false);
        }

        self.platform_impl.borrow().clone()
    }

    /// A transform to apply to the geometry.
    pub fn transform(&self) -> Option<Ref<Transform>> {
        self.get_value(Self::transform_property())
    }

    pub fn set_transform(&self, value: impl Into<Nullable<Transform>>) {
        self.set_value(Self::transform_property(), value.into().0)
    }

    /// Creates a geometry from a string of path data.
    pub fn parse(s: &str) -> Result<Ref<Geometry>, FormatError> {
        Ok(StreamGeometry::parse(s)?.upcast())
    }

    /// Gets the geometry's bounding rectangle with the specified pen.
    pub fn get_render_bounds(&self, pen: &dyn IPen) -> Rect {
        self.platform_impl().map(|g| g.get_render_bounds(Some(pen))).unwrap_or_default()
    }

    /// Indicates whether the geometry's fill contains the specified point.
    pub fn fill_contains(&self, point: Point) -> bool {
        self.platform_impl().is_some_and(|g| g.fill_contains(point))
    }

    /// Gets the relation between this geometry's fill and another one's.
    pub fn get_fill_intersection_result(&self, geometry: &Geometry) -> Option<IntersectionResult> {
        match geometry.platform_impl() {
            None => Some(IntersectionResult::Empty),
            Some(other) => self.platform_impl().map(|g| g.get_fill_intersection_result(&*other)),
        }
    }

    /// Indicates whether the geometry's stroke contains the specified point.
    pub fn stroke_contains(&self, pen: &dyn IPen, point: Point) -> bool {
        self.platform_impl().is_some_and(|g| g.stroke_contains(Some(pen), point))
    }

    /// Gets a geometry that is the shape defined by the stroke on the
    /// geometry produced by the specified pen.
    pub fn get_widened_geometry(&self, pen: &dyn IPen) -> Ref<Geometry> {
        ImmutableGeometry::new(self.platform_impl().map(|g| g.get_widened_geometry(pen))).upcast()
    }

    /// Marks the given properties as affecting the geometry's platform
    /// implementation: after a change to any of them, the implementation is
    /// recreated and the changed notification is raised.
    pub fn affects_geometry(properties: &[&'static FerroProperty]) {
        for property in properties {
            property.changed().subscribe(Self::affects_geometry_invalidate);
        }
    }

    /// Invalidates the platform implementation of the geometry.
    pub fn invalidate_geometry(&self) {
        if !self.can_invalidate {
            return;
        }
        self.is_dirty.set(true);
        *self.platform_impl.borrow_mut() = None;
        self.register_for_serialization();
        self.raise_changed();
    }

    fn transform_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        let new_value = e.get_new_value::<Option<Ref<Transform>>>();

        if let Some(subscription) = self.transform_subscription.take() {
            subscription.dispose();
        }

        if let Some(new_value) = &new_value {
            let weak = self.to_ref().downgrade();
            let weak_transform = new_value.downgrade();
            let subscription = new_value.changed(move || {
                if let (Some(this), Some(transform)) = (weak.upgrade(), weak_transform.upgrade()) {
                    this.transform_changed(Some(&transform));
                }
            });
            *self.transform_subscription.borrow_mut() = Some(subscription);
        }

        self.transform_changed(new_value.as_ref());
    }

    fn transform_changed(&self, sender: Option<&Ref<Transform>>) {
        let transform = sender.map(|t| t.value());
        let current = self.platform_impl.borrow().clone();

        if let Some(current) = current {
            let replacement: Option<Arc<dyn IGeometryImpl>> = match current.as_transformed_geometry() {
                Some(t) => match transform {
                    None => Some(t.source_geometry()),
                    Some(transform) if transform == Matrix::IDENTITY => Some(t.source_geometry()),
                    Some(transform) if transform != t.transform() => {
                        let transformed: Arc<dyn IGeometryImpl> = t.source_geometry().with_transform(transform);
                        Some(transformed)
                    }
                    Some(_) => None,
                },
                None => match transform {
                    Some(transform) if transform != Matrix::IDENTITY => {
                        let transformed: Arc<dyn IGeometryImpl> = current.with_transform(transform);
                        Some(transformed)
                    }
                    _ => None,
                },
            };

            if let Some(replacement) = replacement {
                *self.platform_impl.borrow_mut() = Some(replacement);
            }
        }

        self.register_for_serialization();
        self.raise_changed();
    }

    /// The object as something a compositor serializes: the adapter keeps
    /// the object alive while it is queued.
    pub(crate) fn as_compositor_serializable(&self) -> Rc<dyn ICompositorSerializable> {
        Rc::new(RefAdapter(self.to_ref()))
    }

    fn register_for_serialization(&self) {
        if self.resource.is_attached() {
            let serializable = self.as_compositor_serializable();
            self.resource.register_for_invalidation_on_all_compositors(&serializable);
        }
    }

    fn affects_geometry_invalidate(e: &FerroPropertyChangedEventArgs<'_>) {
        if let Some(control) = e.sender().downcast_ref::<Geometry>() {
            control.invalidate_geometry();
        }
    }

    /// The geometry's total length as if all its contours are placed in a
    /// straight line.
    pub fn contour_length(&self) -> f64 {
        self.platform_impl().map(|g| g.contour_length()).unwrap_or(0.0)
    }

    /// Combines the two geometries using the specified combine mode and
    /// applies the specified transform to the resulting geometry.
    pub fn combine(
        geometry1: impl IntoRef<Geometry>,
        geometry2: &Ref<RectangleGeometry>,
        combine_mode: GeometryCombineMode,
        transform: impl Into<Nullable<Transform>>,
    ) -> Ref<Geometry> {
        CombinedGeometry::with_mode_and_transform(
            combine_mode,
            Some(geometry1.into_ref()),
            Some(geometry2.clone().upcast()),
            transform,
        )
        .upcast()
    }

    /// Attempts to get the corresponding point at the specified distance.
    pub fn try_get_point_at_distance(&self, distance: f64) -> Option<Point> {
        self.platform_impl()?.try_get_point_at_distance(distance)
    }

    /// Attempts to get the corresponding point and tangent from the
    /// specified distance along the contour of the geometry.
    pub fn try_get_point_and_tangent_at_distance(&self, distance: f64) -> Option<(Point, Point)> {
        self.platform_impl()?.try_get_point_and_tangent_at_distance(distance)
    }

    /// Attempts to get the corresponding path segment given by the two
    /// distances specified. Imagine it like snipping a part of the current
    /// geometry.
    pub fn try_get_segment(
        &self,
        start_distance: f64,
        stop_distance: f64,
        start_on_begin_figure: bool,
    ) -> Option<Ref<Geometry>> {
        let segment = self.platform_impl()?.try_get_segment(start_distance, stop_distance, start_on_begin_figure)?;
        Some(PlatformGeometry::new(segment).upcast())
    }

    #[allow(dead_code)]
    #[inline]
    fn object(&self) -> &FerroObject {
        self.upcast()
    }
}

impl ICompositionRenderResource for Geometry {
    fn add_ref_on_compositor(&self, c: &Rc<Compositor>) {
        let owner = self.as_compositor_serializable();
        self.resource.create_or_add_ref(c, Some(owner), |c| {
            c.create_server_object(|server, _| ServerCompositionSimpleGeometry::new(server) as Rc<dyn IServerObject>)
        });
    }

    fn release_on_compositor(&self, c: &Rc<Compositor>) {
        self.resource.release(c);
    }

    fn get_for_compositor(&self, c: &Compositor) -> ServerObjectId {
        self.resource.get_for_compositor(c)
    }
}

impl ICompositorSerializable for RefAdapter<Geometry> {
    fn try_get_server(&self, c: &Compositor) -> Option<ServerObjectId> {
        self.0.resource.try_get_for_compositor(c)
    }

    fn serialization_key(&self) -> *const () {
        RefAdapter::reference_id(self)
    }

    fn serialize_changes(&self, _c: &Compositor, writer: &mut BatchStreamWriter<'_>) {
        ServerCompositionSimpleGeometryProps::serialize_all_changes(writer, self.0.platform_impl());
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::media::{RotateTransform, GeometryImplExt};
    use crate::platform::{IStreamGeometryImpl, ITransformedGeometryImpl};
    use crate::{ferro_impl_classes, instantiate};

    /// A platform geometry that does nothing but track transforms.
    pub(crate) struct MockGeometryImpl;

    impl MockGeometryImpl {
        pub fn create() -> Arc<dyn IGeometryImpl> {
            Arc::new(MockGeometryImpl)
        }
    }

    pub(crate) struct MockTransformedGeometryImpl {
        source: Arc<dyn IGeometryImpl>,
        transform: Matrix,
    }

    macro_rules! mock_geometry_members {
        () => {
            fn bounds(&self) -> Rect {
                Rect::default()
            }
            fn contour_length(&self) -> f64 {
                0.0
            }
            fn get_render_bounds(&self, _pen: Option<&dyn IPen>) -> Rect {
                Rect::default()
            }
            fn get_widened_geometry(&self, _pen: &dyn IPen) -> Arc<dyn IGeometryImpl> {
                MockGeometryImpl::create()
            }
            fn fill_contains(&self, _point: Point) -> bool {
                false
            }
            fn get_fill_intersection_result(&self, _geometry: &dyn IGeometryImpl) -> IntersectionResult {
                IntersectionResult::Empty
            }
            fn intersect(&self, _geometry: &dyn IGeometryImpl) -> Option<Arc<dyn IGeometryImpl>> {
                None
            }
            fn stroke_contains(&self, _pen: Option<&dyn IPen>, _point: Point) -> bool {
                false
            }
            fn try_get_point_at_distance(&self, _distance: f64) -> Option<Point> {
                None
            }
            fn try_get_point_and_tangent_at_distance(&self, _distance: f64) -> Option<(Point, Point)> {
                None
            }
            fn as_any(&self) -> &dyn std::any::Any {
                self
            }
            fn try_get_segment(&self, _start: f64, _stop: f64, _begin: bool) -> Option<Arc<dyn IGeometryImpl>> {
                None
            }
        };
    }
    pub(crate) use mock_geometry_members;

    impl IGeometryImpl for MockGeometryImpl {
        mock_geometry_members!();

        fn with_transform(&self, transform: Matrix) -> Arc<dyn ITransformedGeometryImpl> {
            // The source of a transformed mock is a fresh mock: the tests
            // only look at the kind of the implementation.
            Arc::new(MockTransformedGeometryImpl { source: MockGeometryImpl::create(), transform })
        }

        fn as_stream_geometry(&self) -> Option<&dyn IStreamGeometryImpl> {
            None
        }
    }

    impl IGeometryImpl for MockTransformedGeometryImpl {
        mock_geometry_members!();

        fn with_transform(&self, transform: Matrix) -> Arc<dyn ITransformedGeometryImpl> {
            Arc::new(MockTransformedGeometryImpl { source: self.source.clone(), transform })
        }

        fn as_transformed_geometry(&self) -> Option<&dyn ITransformedGeometryImpl> {
            Some(self)
        }
    }

    impl ITransformedGeometryImpl for MockTransformedGeometryImpl {
        fn source_geometry(&self) -> Arc<dyn IGeometryImpl> {
            self.source.clone()
        }

        fn transform(&self) -> Matrix {
            self.transform
        }
    }

    pub(crate) type CommandLog = Arc<std::sync::Mutex<Vec<String>>>;

    /// A stream geometry that records the drawing commands it receives.
    pub(crate) struct MockStreamGeometryImpl {
        pub log: CommandLog,
    }

    struct MockStreamGeometryContext {
        log: CommandLog,
    }

    impl crate::platform::IGeometryContext for MockStreamGeometryContext {
        fn arc_to(
            &mut self,
            point: Point,
            size: crate::Size,
            rotation_angle: f64,
            is_large_arc: bool,
            sweep_direction: crate::media::SweepDirection,
            is_stroked: bool,
        ) {
            self.log.lock().unwrap().push(format!(
                "arc {point} {size} {rotation_angle} {is_large_arc} {sweep_direction:?} {is_stroked}"
            ));
        }
        fn begin_figure(&mut self, start_point: Point, is_filled: bool) {
            self.log.lock().unwrap().push(format!("begin {start_point} {is_filled}"));
        }
        fn cubic_bezier_to(&mut self, p1: Point, p2: Point, p3: Point, is_stroked: bool) {
            self.log.lock().unwrap().push(format!("cubic {p1} {p2} {p3} {is_stroked}"));
        }
        fn quadratic_bezier_to(&mut self, p1: Point, p2: Point, is_stroked: bool) {
            self.log.lock().unwrap().push(format!("quad {p1} {p2} {is_stroked}"));
        }
        fn line_to(&mut self, point: Point, is_stroked: bool) {
            self.log.lock().unwrap().push(format!("line {point} {is_stroked}"));
        }
        fn end_figure(&mut self, is_closed: bool) {
            self.log.lock().unwrap().push(format!("end {is_closed}"));
        }
        fn set_fill_rule(&mut self, fill_rule: crate::media::FillRule) {
            self.log.lock().unwrap().push(format!("fill {fill_rule:?}"));
        }
        fn dispose(&mut self) {
            self.log.lock().unwrap().push("dispose".to_string());
        }
    }

    impl crate::platform::IStreamGeometryContextImpl for MockStreamGeometryContext {}

    impl IGeometryImpl for MockStreamGeometryImpl {
        mock_geometry_members!();

        fn with_transform(&self, transform: Matrix) -> Arc<dyn ITransformedGeometryImpl> {
            Arc::new(MockTransformedGeometryImpl { source: MockGeometryImpl::create(), transform })
        }

        fn as_stream_geometry(&self) -> Option<&dyn IStreamGeometryImpl> {
            Some(self)
        }
    }

    impl IStreamGeometryImpl for MockStreamGeometryImpl {
        fn clone_geometry(&self) -> Arc<dyn IStreamGeometryImpl> {
            Arc::new(MockStreamGeometryImpl { log: Arc::new(std::sync::Mutex::new(self.log.lock().unwrap().clone())) })
        }

        fn open(&self) -> Box<dyn crate::platform::IStreamGeometryContextImpl> {
            Box::new(MockStreamGeometryContext { log: self.log.clone() })
        }
    }

    /// A geometry factory that records what it was asked to create.
    #[derive(Default)]
    pub(crate) struct MockFactory {
        pub created: RefCell<Vec<String>>,
        pub streams: RefCell<Vec<CommandLog>>,
    }

    impl crate::platform::IPlatformRenderInterface for MockFactory {
        fn build_glyph_run_geometry(&self, _glyph_run: &crate::media::GlyphRun) -> Arc<dyn IGeometryImpl> {
            unimplemented!()
        }
        fn create_glyph_run(
            &self,
            _glyph_typeface: &Rc<crate::media::GlyphTypeface>,
            _font_rendering_em_size: f64,
            _glyph_infos: &[crate::media::text_formatting::GlyphInfo],
            _baseline_origin: Point,
        ) -> std::sync::Arc<dyn crate::platform::IGlyphRunImpl> {
            unimplemented!()
        }
        fn create_ellipse_geometry(&self, rect: Rect) -> Arc<dyn IGeometryImpl> {
            self.created.borrow_mut().push(format!("ellipse {rect}"));
            MockGeometryImpl::create()
        }
        fn create_line_geometry(&self, p1: Point, p2: Point) -> Arc<dyn IGeometryImpl> {
            self.created.borrow_mut().push(format!("line {p1} {p2}"));
            MockGeometryImpl::create()
        }
        fn create_rectangle_geometry(&self, rect: Rect) -> Arc<dyn IGeometryImpl> {
            self.created.borrow_mut().push(format!("rectangle {rect}"));
            MockGeometryImpl::create()
        }
        fn create_stream_geometry(&self) -> Arc<dyn IStreamGeometryImpl> {
            self.created.borrow_mut().push("stream".to_string());
            let log = Arc::new(std::sync::Mutex::new(Vec::new()));
            self.streams.borrow_mut().push(log.clone());
            Arc::new(MockStreamGeometryImpl { log })
        }
        fn create_geometry_group(
            &self,
            fill_rule: crate::media::FillRule,
            children: &[Arc<dyn IGeometryImpl>],
        ) -> Arc<dyn IGeometryImpl> {
            self.created.borrow_mut().push(format!("group {fill_rule:?} {}", children.len()));
            MockGeometryImpl::create()
        }
        fn create_combined_geometry(
            &self,
            combine_mode: GeometryCombineMode,
            _g1: Arc<dyn IGeometryImpl>,
            _g2: Arc<dyn IGeometryImpl>,
        ) -> Arc<dyn IGeometryImpl> {
            self.created.borrow_mut().push(format!("combined {combine_mode:?}"));
            MockGeometryImpl::create()
        }

        // The remaining members are not exercised by the geometry tests.
        fn create_render_target_bitmap(
            &self,
            _: crate::PixelSize,
            _: crate::Vector,
        ) -> std::sync::Arc<dyn crate::platform::IRenderTargetBitmapImpl> {
            unimplemented!()
        }
        fn create_writeable_bitmap(
            &self,
            _: crate::PixelSize,
            _: crate::Vector,
            _: crate::platform::PixelFormat,
            _: crate::platform::AlphaFormat,
        ) -> std::sync::Arc<dyn crate::platform::IWriteableBitmapImpl> {
            unimplemented!()
        }
        fn load_bitmap_from_file(&self, _: &str) -> std::io::Result<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
            unimplemented!()
        }
        fn load_bitmap(&self, _: &mut dyn std::io::Read) -> std::io::Result<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
            unimplemented!()
        }
        fn load_writeable_bitmap_to_width(
            &self,
            _: &mut dyn std::io::Read,
            _: i32,
            _: crate::media::imaging::BitmapInterpolationMode,
        ) -> std::io::Result<std::sync::Arc<dyn crate::platform::IWriteableBitmapImpl>> {
            unimplemented!()
        }
        fn load_writeable_bitmap_to_height(
            &self,
            _: &mut dyn std::io::Read,
            _: i32,
            _: crate::media::imaging::BitmapInterpolationMode,
        ) -> std::io::Result<std::sync::Arc<dyn crate::platform::IWriteableBitmapImpl>> {
            unimplemented!()
        }
        fn load_writeable_bitmap_from_file(
            &self,
            _: &str,
        ) -> std::io::Result<std::sync::Arc<dyn crate::platform::IWriteableBitmapImpl>> {
            unimplemented!()
        }
        fn load_writeable_bitmap(
            &self,
            _: &mut dyn std::io::Read,
        ) -> std::io::Result<std::sync::Arc<dyn crate::platform::IWriteableBitmapImpl>> {
            unimplemented!()
        }
        fn load_bitmap_to_width(
            &self,
            _: &mut dyn std::io::Read,
            _: i32,
            _: crate::media::imaging::BitmapInterpolationMode,
        ) -> std::io::Result<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
            unimplemented!()
        }
        fn load_bitmap_to_height(
            &self,
            _: &mut dyn std::io::Read,
            _: i32,
            _: crate::media::imaging::BitmapInterpolationMode,
        ) -> std::io::Result<std::sync::Arc<crate::platform::SharedBitmapImpl>> {
            unimplemented!()
        }
        fn resize_bitmap(
            &self,
            _: &dyn crate::platform::IBitmapImpl,
            _: crate::PixelSize,
            _: crate::media::imaging::BitmapInterpolationMode,
        ) -> std::sync::Arc<crate::platform::SharedBitmapImpl> {
            unimplemented!()
        }
        fn load_bitmap_from_pixels(
            &self,
            _: crate::platform::PixelFormat,
            _: crate::platform::AlphaFormat,
            _: &[u8],
            _: crate::PixelSize,
            _: crate::Vector,
            _: i32,
        ) -> std::sync::Arc<crate::platform::SharedBitmapImpl> {
            unimplemented!()
        }
        fn create_backend_context(
            &self,
            _: Option<Rc<dyn crate::platform::IPlatformGraphicsContext>>,
        ) -> Rc<dyn crate::platform::IPlatformRenderInterfaceContext> {
            unimplemented!()
        }
        fn supports_individual_round_rects(&self) -> bool {
            false
        }
        fn default_alpha_format(&self) -> crate::platform::AlphaFormat {
            crate::platform::AlphaFormat::Premul
        }
        fn default_pixel_format(&self) -> crate::platform::PixelFormat {
            crate::platform::PixelFormats::RGBA8888
        }
        fn is_supported_bitmap_pixel_format(&self, _: crate::platform::PixelFormat) -> bool {
            true
        }
        fn supports_regions(&self) -> bool {
            false
        }
        fn create_region(&self) -> Rc<dyn crate::platform::IPlatformRenderInterfaceRegion> {
            unimplemented!()
        }
    }

    /// Runs `f` with a mock render interface registered in a locator scope.
    pub(crate) fn with_mock_render_interface(f: impl FnOnce(&MockFactory)) {
        let scope = crate::FerroLocator::enter_scope();
        let factory = Rc::new(MockFactory::default());
        crate::FerroLocator::current_mutable()
            .bind::<dyn crate::platform::IPlatformRenderInterface>()
            .to_constant(factory.clone());
        f(&factory);
        scope.dispose();
    }

    #[repr(C)]
    struct TestGeometry {
        base: Geometry,
    }

    ferro_class!(TestGeometry: Geometry);
    ferro_impl_classes!(TestGeometry: FerroObjectImpl);

    impl GeometryImpl for TestGeometry {
        fn clone_geometry(this: &Self) -> Ref<Geometry> {
            Self::parent_clone_geometry(this)
        }

        fn create_defining_geometry(_this: &Self) -> Option<Arc<dyn IGeometryImpl>> {
            Some(MockGeometryImpl::create())
        }
    }

    impl TestGeometry {
        ferro_property!(pub fn foo_property() -> StyledProperty<bool> {
            FerroProperty::register::<TestGeometry, _>("Foo", false)
        });

        fn new() -> Ref<Self> {
            thread_local! {
                static DONE: Cell<bool> = const { Cell::new(false) };
            }
            if !DONE.replace(true) {
                Geometry::affects_geometry(&[Self::foo_property().as_property()]);
            }
            instantiate(Self { base: Geometry::construct() })
        }

        fn set_foo(&self, value: bool) {
            self.set_value(Self::foo_property(), value)
        }
    }

    fn is_transformed(target: &TestGeometry) -> bool {
        assert!(target.to_ref().get_type() == TestGeometry::TYPE);
        target.platform_impl().unwrap().as_transformed_geometry().is_some()
    }

    fn track_changed(target: &Geometry) -> Rc<Cell<bool>> {
        let raised = Rc::new(Cell::new(false));
        let r = raised.clone();
        target.changed(move || r.set(true));
        raised
    }

    #[test]
    fn changing_affects_geometry_property_causes_platform_impl_to_be_updated() {
        let target = TestGeometry::new();
        let platform_impl = target.platform_impl().unwrap();

        target.set_foo(true);

        assert!(!Arc::ptr_eq(&platform_impl, &target.platform_impl().unwrap()));
    }

    #[test]
    fn changing_affects_geometry_property_causes_changed_to_be_raised() {
        let target = TestGeometry::new();
        let raised = track_changed(&target);

        target.set_foo(true);

        assert!(raised.get());
    }

    #[test]
    fn setting_transform_causes_changed_to_be_raised() {
        let target = TestGeometry::new();
        let raised = track_changed(&target);

        target.set_transform(RotateTransform::with_angle(45.0));

        assert!(raised.get());
    }

    #[test]
    fn changing_transform_causes_changed_to_be_raised() {
        let transform = RotateTransform::with_angle(45.0);
        let target = TestGeometry::new();
        target.set_transform(&transform);
        let raised = track_changed(&target);

        transform.set_angle(90.0);

        assert!(raised.get());
    }

    #[test]
    fn removing_transform_causes_changed_to_be_raised() {
        let transform = RotateTransform::with_angle(45.0);
        let target = TestGeometry::new();
        target.set_transform(&transform);
        let raised = track_changed(&target);

        target.set_transform(None);

        assert!(raised.get());
    }

    #[test]
    fn transform_produces_transformed_platform_impl() {
        let target = TestGeometry::new();
        let rotate = RotateTransform::with_angle(45.0);

        assert!(!is_transformed(&target));

        target.set_transform(&rotate);

        assert!(is_transformed(&target));

        rotate.set_angle(0.0);

        assert!(!is_transformed(&target));
    }

    #[test]
    fn geometries_create_their_platform_implementation() {
        use crate::media::{
            EllipseGeometry, FillRule, GeometryGroup, LineGeometry, PathGeometry, PolylineGeometry, StreamGeometry,
        };
        use crate::platform::IGeometryContext;

        with_mock_render_interface(|factory| {
            let ellipse = EllipseGeometry::with_rect(Rect::new(1.0, 2.0, 3.0, 4.0));
            assert!(ellipse.platform_impl().is_some());
            let ellipse = EllipseGeometry::new();
            ellipse.set_center(Point::new(10.0, 10.0));
            ellipse.set_radius_x(2.0);
            ellipse.set_radius_y(3.0);
            assert!(ellipse.platform_impl().is_some());
            let clone = ellipse.clone_geometry().cast::<EllipseGeometry>().unwrap();
            assert_eq!((Point::new(10.0, 10.0), 2.0, 3.0), (clone.center(), clone.radius_x(), clone.radius_y()));

            let line = LineGeometry::with_points(Point::new(0.0, 0.0), Point::new(5.0, 5.0));
            assert!(line.platform_impl().is_some());
            assert!(line.clone_geometry().is::<LineGeometry>());

            let polyline = PolylineGeometry::with_points_and_fill_rule(
                [Point::new(0.0, 0.0), Point::new(1.0, 0.0), Point::new(1.0, 1.0)],
                true,
                FillRule::NonZero,
            );
            assert!(polyline.platform_impl().is_some());
            let clone = polyline.clone_geometry().cast::<PolylineGeometry>().unwrap();
            assert_eq!((3, true, FillRule::NonZero), (clone.points().len(), clone.is_filled(), clone.fill_rule()));

            let path = PathGeometry::parse("F1 M0,0 L1,1 Q1,2 3,4 Z").unwrap();
            assert!(path.platform_impl().is_some());

            let group = GeometryGroup::new();
            group.children().add(line.clone().upcast());
            group.children().add(GeometryGroup::new().upcast());
            assert!(group.platform_impl().is_some());
            let clone = group.clone_geometry().cast::<GeometryGroup>().unwrap();
            assert_eq!(2, clone.children().len());

            let combined = CombinedGeometry::with_geometries(line.upcast(), ellipse.upcast());
            assert!(combined.platform_impl().is_some());

            let stream = StreamGeometry::new();
            {
                let mut context = stream.open();
                context.begin_figure(Point::new(1.0, 1.0), true);
                context.line_to(Point::new(2.0, 2.0), true);
                context.end_figure(false);
            }
            assert!(stream.clone_geometry().is::<StreamGeometry>());
            let parsed = StreamGeometry::parse("M1,1 L2,2").unwrap();
            assert!(parsed.platform_impl().is_some());
            assert!(StreamGeometry::parse("M1,1 X").is_err());

            assert_eq!(
                vec![
                    "ellipse 1, 2, 3, 4",
                    "ellipse 8, 7, 4, 6",
                    "line 0, 0 5, 5",
                    "stream",
                    "stream",
                    "group EvenOdd 1",
                    "combined Union",
                    "stream",
                    "stream",
                    "stream",
                ],
                *factory.created.borrow()
            );
            let streams = factory.streams.borrow();
            assert_eq!(
                vec!["fill NonZero", "begin 0, 0 true", "line 1, 0 true", "line 1, 1 true", "end true", "dispose"],
                *streams[0].lock().unwrap()
            );
            assert_eq!(
                vec![
                    "fill NonZero",
                    "begin 0, 0 true",
                    "line 1, 1 true",
                    "quad 1, 2 3, 4 true",
                    "end true",
                    "dispose"
                ],
                *streams[1].lock().unwrap()
            );
            assert_eq!(vec!["begin 1, 1 true", "line 2, 2 true", "end false", "dispose"], *streams[2].lock().unwrap());
            assert_eq!(vec!["begin 1, 1 true", "line 2, 2 true", "end false", "dispose"], *streams[3].lock().unwrap());
        });
    }

    #[test]
    fn replaced_transform_no_longer_raises_changed() {
        let transform = RotateTransform::with_angle(45.0);
        let target = TestGeometry::new();
        target.set_transform(&transform);
        target.set_transform(None);
        let raised = track_changed(&target);

        transform.set_angle(90.0);

        assert!(!raised.get());
    }
}
