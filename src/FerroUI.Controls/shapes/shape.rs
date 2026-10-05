use crate::{Control, ControlImpl};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{
    DrawingContext, Geometry, IBrush, IPen, MatrixTransform, MediaCollection, Pen, PenLineCap, PenLineJoin, Stretch,
    StrokeDashArray,
};
use ferroui_base::reactive::IDisposable;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, Matrix, ObjectType, Rect, Ref, Size, StyledElementImpl, StyledProperty, Upcast,
    Vector, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Provides a base class for shape elements, such as `Ellipse`, `Polygon`
/// and `Rectangle`.
#[repr(C)]
pub struct Shape {
    base: Control,
    transform: Cell<Matrix>,
    defining_geometry: RefCell<Option<Ref<Geometry>>>,
    rendered_geometry: RefCell<Option<Ref<Geometry>>>,
    stroke_pen: RefCell<Option<Rc<dyn IPen>>>,
    geometry_changed_subscription: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class! {
    Shape: Control, virtuals ShapeImpl: ControlImpl {
        /// Creates the shape's defining geometry.
        fn create_defining_geometry(this) -> Option<Ref<Geometry>>;

        /// Called when the underlying [`Geometry`] changed.
        fn on_geometry_changed(this);
    }
}

ferro_impl_classes!(Shape: StyledElementImpl, InteractiveImpl, InputElementImpl, ControlImpl);

impl ShapeImpl for Shape {
    fn create_defining_geometry(_this: &Self) -> Option<Ref<Geometry>> {
        panic!("Shape is abstract: 'create_defining_geometry' must be implemented by the deriving class")
    }

    fn on_geometry_changed(this: &Self) {
        *this.rendered_geometry.borrow_mut() = None;

        this.invalidate_measure();
    }
}

impl FerroObjectImpl for Shape {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);

        let property = change.property();
        let is_stroke = property == Self::stroke_property().as_property();
        let is_stroke_thickness = property == Self::stroke_thickness_property().as_property();

        if is_stroke
            || is_stroke_thickness
            || property == Self::stroke_dash_array_property().as_property()
            || property == Self::stroke_dash_offset_property().as_property()
            || property == Self::stroke_line_cap_property().as_property()
            || property == Self::stroke_join_property().as_property()
            || property == Self::stroke_miter_limit_property().as_property()
        {
            if is_stroke || is_stroke_thickness {
                this.invalidate_measure();
            }

            // The pen is taken out of its slot while it is updated: updating
            // a mutable pen raises its change notifications.
            let mut pen = this.stroke_pen.borrow().clone();
            let dash_array = this.stroke_dash_array();
            let changed = Pen::try_modify_or_create(
                &mut pen,
                this.stroke(),
                this.stroke_thickness(),
                dash_array.as_ref().map(StrokeDashArray::Observable),
                this.stroke_dash_offset(),
                this.stroke_line_cap(),
                this.stroke_join(),
                this.stroke_miter_limit(),
            );
            *this.stroke_pen.borrow_mut() = pen;

            if changed {
                this.invalidate_visual();
            }
        } else if property == Self::fill_property().as_property() {
            this.invalidate_visual();
        }
    }
}

impl VisualImpl for Shape {
    fn render(this: &Self, context: &mut DrawingContext) {
        let geometry = this.rendered_geometry();

        if let Some(geometry) = geometry {
            let fill = this.fill();
            let pen = this.stroke_pen.borrow().clone();
            context.draw_geometry(fill.as_ref(), pen.as_ref(), &geometry);
        }
    }

    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);

        let defining_geometry = this.defining_geometry.borrow().clone();
        if let Some(defining_geometry) = defining_geometry {
            this.subscribe_to_geometry(&defining_geometry);
        }
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        if this.defining_geometry.borrow().is_some() {
            this.unsubscribe_from_geometry();
        }
    }
}

impl LayoutableImpl for Shape {
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let Some(defining_geometry) = this.defining_geometry() else {
            return Size::default();
        };

        Self::calculate_size_and_transform(available_size, defining_geometry.bounds(), this.stretch()).0
    }

    fn arrange_override(this: &Self, final_size: Size) -> Size {
        if let Some(defining_geometry) = this.defining_geometry() {
            // This should probably use the render bounds for the stroke
            // thickness, but then the calculations would multiply the stroke
            // thickness as well, which isn't correct.
            let (_, transform) =
                Self::calculate_size_and_transform(final_size, defining_geometry.bounds(), this.stretch());

            if this.transform.get() != transform {
                this.transform.set(transform);
                *this.rendered_geometry.borrow_mut() = None;
            }

            return final_size;
        }

        Size::default()
    }
}

ferroui_base::ferro_properties! { impl Shape {
    ferro_property!(
        /// Defines the `Fill` property.
        pub fn fill_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<Shape, _>("Fill", None)
        }
    );

    ferro_property!(
        /// Defines the `Stretch` property.
        pub fn stretch_property() -> StyledProperty<Stretch> {
            FerroProperty::register::<Shape, _>("Stretch", Stretch::None)
        }
    );

    ferro_property!(
        /// Defines the `Stroke` property.
        pub fn stroke_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
            FerroProperty::register::<Shape, _>("Stroke", None)
        }
    );

    ferro_property!(
        /// Defines the `StrokeDashArray` property.
        pub fn stroke_dash_array_property() -> StyledProperty<Option<MediaCollection<f64>>> {
            FerroProperty::register::<Shape, _>("StrokeDashArray", None)
        }
    );

    ferro_property!(
        /// Defines the `StrokeDashOffset` property.
        pub fn stroke_dash_offset_property() -> StyledProperty<f64> {
            FerroProperty::register::<Shape, _>("StrokeDashOffset", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `StrokeThickness` property.
        pub fn stroke_thickness_property() -> StyledProperty<f64> {
            FerroProperty::register::<Shape, _>("StrokeThickness", 0.0)
        }
    );

    ferro_property!(
        /// Defines the `StrokeLineCap` property.
        pub fn stroke_line_cap_property() -> StyledProperty<PenLineCap> {
            FerroProperty::register::<Shape, _>("StrokeLineCap", PenLineCap::Flat)
        }
    );

    ferro_property!(
        /// Defines the `StrokeJoin` property.
        pub fn stroke_join_property() -> StyledProperty<PenLineJoin> {
            FerroProperty::register::<Shape, _>("StrokeJoin", PenLineJoin::Miter)
        }
    );

    ferro_property!(
        /// Defines the `StrokeMiterLimit` property.
        pub fn stroke_miter_limit_property() -> StyledProperty<f64> {
            FerroProperty::register::<Shape, _>("StrokeMiterLimit", 10.0)
        }
    );
} }

impl Shape {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Control::construct(),
            transform: Cell::new(Matrix::IDENTITY),
            defining_geometry: RefCell::new(None),
            rendered_geometry: RefCell::new(None),
            stroke_pen: RefCell::new(None),
            geometry_changed_subscription: RefCell::new(None),
        }
    }

    /// Gets a value that represents the [`Geometry`] of the shape.
    pub fn defining_geometry(&self) -> Option<Ref<Geometry>> {
        let current = self.defining_geometry.borrow().clone();
        if current.is_some() {
            return current;
        }

        let created = self.create_defining_geometry();
        *self.defining_geometry.borrow_mut() = created.clone();

        if let Some(geometry) = &created {
            if self.visual_root().is_some() {
                self.subscribe_to_geometry(geometry);
            }
        }

        created
    }

    /// Gets a value that represents the final rendered [`Geometry`] of the
    /// shape.
    pub fn rendered_geometry(&self) -> Option<Ref<Geometry>> {
        let current = self.rendered_geometry.borrow().clone();
        if current.is_some() {
            return current;
        }

        let defining_geometry = self.defining_geometry()?;
        let transform = self.transform.get();

        let rendered = if transform == Matrix::IDENTITY {
            defining_geometry
        } else {
            let rendered = defining_geometry.clone_geometry();

            match rendered.transform().map(|t| t.value()) {
                Some(value) if value != Matrix::IDENTITY => {
                    rendered.set_transform(MatrixTransform::with_matrix(value * transform));
                }
                _ => rendered.set_transform(MatrixTransform::with_matrix(transform)),
            }

            rendered
        };

        *self.rendered_geometry.borrow_mut() = Some(rendered.clone());
        Some(rendered)
    }

    /// Gets the brush that specifies how the shape's interior is painted.
    pub fn fill(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::fill_property())
    }

    /// Sets the brush that specifies how the shape's interior is painted.
    pub fn set_fill(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::fill_property(), value)
    }

    /// Gets a [`Stretch`] value that describes how the shape fills its
    /// allocated space.
    pub fn stretch(&self) -> Stretch {
        self.get_value(Self::stretch_property())
    }

    /// Sets a [`Stretch`] value that describes how the shape fills its
    /// allocated space.
    pub fn set_stretch(&self, value: Stretch) {
        self.set_value(Self::stretch_property(), value)
    }

    /// Gets the brush that specifies how the shape's outline is painted.
    pub fn stroke(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::stroke_property())
    }

    /// Sets the brush that specifies how the shape's outline is painted.
    pub fn set_stroke(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::stroke_property(), value)
    }

    /// Gets a collection of values that indicate the pattern of dashes and
    /// gaps that is used to outline shapes.
    pub fn stroke_dash_array(&self) -> Option<MediaCollection<f64>> {
        self.get_value(Self::stroke_dash_array_property())
    }

    /// Sets a collection of values that indicate the pattern of dashes and
    /// gaps that is used to outline shapes.
    pub fn set_stroke_dash_array(&self, value: Option<MediaCollection<f64>>) {
        self.set_value(Self::stroke_dash_array_property(), value)
    }

    /// Gets a value that specifies the distance within the dash pattern
    /// where a dash begins.
    pub fn stroke_dash_offset(&self) -> f64 {
        self.get_value(Self::stroke_dash_offset_property())
    }

    /// Sets a value that specifies the distance within the dash pattern
    /// where a dash begins.
    pub fn set_stroke_dash_offset(&self, value: f64) {
        self.set_value(Self::stroke_dash_offset_property(), value)
    }

    /// Gets the width of the shape outline.
    pub fn stroke_thickness(&self) -> f64 {
        self.get_value(Self::stroke_thickness_property())
    }

    /// Sets the width of the shape outline.
    pub fn set_stroke_thickness(&self, value: f64) {
        self.set_value(Self::stroke_thickness_property(), value)
    }

    /// Gets a [`PenLineCap`] value that describes the shape at the ends of a
    /// line.
    pub fn stroke_line_cap(&self) -> PenLineCap {
        self.get_value(Self::stroke_line_cap_property())
    }

    /// Sets a [`PenLineCap`] value that describes the shape at the ends of a
    /// line.
    pub fn set_stroke_line_cap(&self, value: PenLineCap) {
        self.set_value(Self::stroke_line_cap_property(), value)
    }

    /// Gets a [`PenLineJoin`] value that specifies the type of join that is
    /// used at the vertices of a shape.
    pub fn stroke_join(&self) -> PenLineJoin {
        self.get_value(Self::stroke_join_property())
    }

    /// Sets a [`PenLineJoin`] value that specifies the type of join that is
    /// used at the vertices of a shape.
    pub fn set_stroke_join(&self, value: PenLineJoin) {
        self.set_value(Self::stroke_join_property(), value)
    }

    /// Gets the limit on the ratio of the miter length to half the
    /// `StrokeThickness` of the pen.
    pub fn stroke_miter_limit(&self) -> f64 {
        self.get_value(Self::stroke_miter_limit_property())
    }

    /// Sets the limit on the ratio of the miter length to half the
    /// `StrokeThickness` of the pen.
    pub fn set_stroke_miter_limit(&self, value: f64) {
        self.set_value(Self::stroke_miter_limit_property(), value)
    }

    /// Marks a property as affecting the shape's geometry.
    ///
    /// After a call to this method in a control's class initialisation, any
    /// change to the property on an object of class `T` causes
    /// [`invalidate_geometry`](Self::invalidate_geometry) to be called on
    /// the element.
    pub fn affects_geometry<T: ObjectType + Upcast<Shape>>(properties: &[&'static FerroProperty]) {
        for property in properties {
            property.changed().subscribe(|e| {
                if let Some(sender) = e.sender().downcast_ref::<T>() {
                    let shape: &Shape = sender.upcast();
                    Shape::affects_geometry_invalidate(shape, e);
                }
            });
        }
    }

    /// Invalidates the geometry of this shape.
    pub fn invalidate_geometry(&self) {
        if self.defining_geometry.borrow().is_some() {
            self.unsubscribe_from_geometry();
        }

        *self.rendered_geometry.borrow_mut() = None;
        *self.defining_geometry.borrow_mut() = None;

        self.invalidate_measure();
    }

    /// Calculates the size of a shape with the given bounds in the available
    /// space, and the transform which stretches the shape to that size.
    pub(crate) fn calculate_size_and_transform(
        available_size: Size,
        shape_bounds: Rect,
        stretch: Stretch,
    ) -> (Size, Matrix) {
        let mut shape_size = Size::new(shape_bounds.right(), shape_bounds.bottom());
        let mut translate = Matrix::IDENTITY;
        let mut desired_x = available_size.width;
        let mut desired_y = available_size.height;
        let mut sx = 0.0;
        let mut sy = 0.0;

        if stretch != Stretch::None {
            shape_size = shape_bounds.size();
            translate = Matrix::create_translation_vector(-Vector::from(shape_bounds.position()));
        }

        if available_size.width.is_infinite() {
            desired_x = shape_size.width;
        }

        if available_size.height.is_infinite() {
            desired_y = shape_size.height;
        }

        if shape_bounds.width > 0.0 {
            sx = desired_x / shape_size.width;
        }

        if shape_bounds.height > 0.0 {
            sy = desired_y / shape_size.height;
        }

        if available_size.width.is_infinite() {
            sx = sy;
        }

        if available_size.height.is_infinite() {
            sy = sx;
        }

        match stretch {
            Stretch::Uniform => {
                sx = f64::min(sx, sy);
                sy = sx;
            }
            Stretch::UniformToFill => {
                sx = f64::max(sx, sy);
                sy = sx;
            }
            Stretch::Fill => {
                if available_size.width.is_infinite() {
                    sx = 1.0;
                }

                if available_size.height.is_infinite() {
                    sy = 1.0;
                }
            }
            Stretch::None => {
                sx = 1.0;
                sy = 1.0;
            }
        }

        let transform = translate * Matrix::create_scale(sx, sy);
        let size = Size::new(shape_size.width * sx, shape_size.height * sy);
        (size, transform)
    }

    fn affects_geometry_invalidate(control: &Shape, e: &FerroPropertyChangedEventArgs<'_>) {
        // If the geometry is invalidated when the bounds change, only
        // invalidate when the size portion changes.
        if e.property() == Visual::bounds_property().as_property() {
            let (old_bounds, new_bounds) = e.get_old_and_new_value::<Rect>();

            if old_bounds.size() == new_bounds.size() {
                return;
            }
        }

        control.invalidate_geometry();
    }

    /// Listens to changes of the defining geometry. At most one subscription
    /// is kept.
    fn subscribe_to_geometry(&self, geometry: &Ref<Geometry>) {
        let weak = self.to_ref().downgrade();
        let subscription = geometry.changed(move || {
            if let Some(this) = weak.upgrade() {
                this.on_geometry_changed();
            }
        });

        let previous = self.geometry_changed_subscription.replace(Some(subscription));
        if let Some(previous) = previous {
            previous.dispose();
        }
    }

    fn unsubscribe_from_geometry(&self) {
        let subscription = self.geometry_changed_subscription.take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }
}
