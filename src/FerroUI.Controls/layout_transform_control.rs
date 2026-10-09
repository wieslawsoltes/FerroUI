// The idea and the algorithm come from the layout transformer of the
// Silverlight toolkit.

use crate::{ControlImpl, Control, Decorator};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, LayoutableImplExt};
use ferroui_base::media::{ITransform, MatrixTransform};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::math_utilities::min;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_property, instantiate, FerroObjectImpl, FerroProperty,
    FerroPropertyChangedEventArgs, Matrix, Rect, Ref, RelativePoint, RelativeUnit, Size, StyledElementImpl,
    StyledProperty, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use std::cell::{Cell, OnceCell, RefCell};
use std::rc::Rc;

// About ACCEPTABLE_DELTA and DECIMALS_AFTER_ROUND - the original comment from
// the Silverlight code explains:
// Note: AcceptableDelta and DecimalsAfterRound work around double arithmetic
// rounding issues on Silverlight.
//
// DECIMALS_AFTER_ROUND was increased from the original 4 to 8 because of
// issues when zooming in and out at the end of very large objects.

const ACCEPTABLE_DELTA: f64 = 0.0001;

const DECIMALS_AFTER_ROUND: i32 = 8;

/// Control that implements support for transformations as if applied by a
/// layout transform.
#[repr(C)]
pub struct LayoutTransformControl {
    base: Decorator,
    render_transform_changed_event: RefCell<Option<Rc<dyn IDisposable>>>,
    /// Actual desired size of the child when arranged.
    child_actual_size: Cell<Size>,
    /// Render transform / matrix transform applied to the transform root.
    matrix_transform: OnceCell<Ref<MatrixTransform>>,
    /// Transformation matrix corresponding to the matrix transform.
    transformation: Cell<Matrix>,
    /// The subscription to the changed notification of the layout transform.
    layout_transform_changed: RefCell<Option<Rc<dyn IDisposable>>>,
}

ferro_class!(LayoutTransformControl: Decorator);
ferroui_base::ferro_class_info!(LayoutTransformControl { new: LayoutTransformControl::new });
ferro_impl_classes!(LayoutTransformControl: StyledElementImpl, InteractiveImpl, InputElementImpl, ControlImpl);

ferroui_base::ferro_impl_classes!(LayoutTransformControl: FerroObjectImpl);

impl VisualImpl for LayoutTransformControl {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        this.subscribe_layout_transform(this.layout_transform());
        this.apply_layout_transform();
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);
        this.unsubscribe_layout_transform();
    }
}

impl LayoutableImpl for LayoutTransformControl {
    /// Provides the behaviour for the "arrange" pass of layout.
    fn arrange_override(this: &Self, final_size: Size) -> Size {
        let (Some(transform_root), Some(_)) = (this.transform_root(), this.layout_transform()) else {
            this.set_current_value(Self::layout_transform_property(), this.render_transform());
            return Self::parent_arrange_override(this, final_size);
        };

        // Determine the largest available size after the transformation
        let mut final_size_transformed = this.compute_largest_transformed_size(final_size);
        if Self::is_size_smaller(final_size_transformed, transform_root.desired_size()) {
            // Some elements do not like being given less space than they
            // asked for (ex: text blocks). Bump the working size up to do the
            // right thing by them.
            final_size_transformed = transform_root.desired_size();
        }

        // Transform the working size to find its width/height
        let transformed_rect = Rect::new(0.0, 0.0, final_size_transformed.width, final_size_transformed.height)
            .transform_to_aabb(this.transformation.get());
        // Create the arrange rect to center the transformed content
        let final_rect = Rect::new(
            -transformed_rect.x + ((final_size.width - transformed_rect.width) / 2.0),
            -transformed_rect.y + ((final_size.height - transformed_rect.height) / 2.0),
            final_size_transformed.width,
            final_size_transformed.height,
        );

        // Perform an arrange on the transform root (containing the child)
        transform_root.arrange(final_rect);
        let arranged_size = transform_root.bounds().size();

        // This is the first opportunity to find out the child's true desired
        // size. The reference implementation leaves the "measure again" step
        // disabled, so only the flag is maintained.
        if !(Self::is_size_smaller(final_size_transformed, arranged_size)
            && this.child_actual_size.get() == Size::default())
        {
            // Clear the "need to measure/arrange again" flag
            this.child_actual_size.set(Size::default());
        }

        // Return result to perform the transformation
        final_size
    }

    /// Provides the behaviour for the "measure" pass of layout.
    fn measure_override(this: &Self, available_size: Size) -> Size {
        let (Some(transform_root), Some(_)) = (this.transform_root(), this.layout_transform()) else {
            return Self::parent_measure_override(this, available_size);
        };

        let measure_size = if this.child_actual_size.get() == Size::default() {
            // Determine the largest size after the transformation
            this.compute_largest_transformed_size(available_size)
        } else {
            // Previous measure/arrange pass determined that the child's
            // desired size was larger than believed
            this.child_actual_size.get()
        };

        // Perform a measure on the transform root (containing the child)
        transform_root.measure(measure_size);

        let desired_size = transform_root.desired_size();

        // Transform the desired size to find its width/height
        let transformed_desired_rect =
            Rect::new(0.0, 0.0, desired_size.width, desired_size.height).transform_to_aabb(this.transformation.get());

        // Return result to allocate enough space for the transformation
        Size::new(transformed_desired_rect.width, transformed_desired_rect.height)
    }
}

ferroui_base::ferro_properties! { impl LayoutTransformControl {
    ferro_property!(
        /// Defines the `LayoutTransform` property.
        pub fn layout_transform_property() -> StyledProperty<Option<Rc<dyn ITransform>>> {
            FerroProperty::register::<LayoutTransformControl, _>("LayoutTransform", None)
        }
    );

    ferro_property!(
        /// Defines the `UseRenderTransform` property.
        pub fn use_render_transform_property() -> StyledProperty<bool> {
            FerroProperty::register::<LayoutTransformControl, _>("UseRenderTransform", false)
        }
    );
} }

impl LayoutTransformControl {
    fn static_constructor() {
        Visual::clip_to_bounds_property().override_default_value::<LayoutTransformControl>(true);

        Self::layout_transform_property()
            .changed()
            .add_class_handler::<LayoutTransformControl>(|x, e| x.on_layout_transform_changed(e));

        Decorator::child_property().changed().add_class_handler::<LayoutTransformControl>(|x, _| x.on_child_changed());

        Self::use_render_transform_property()
            .changed()
            .add_class_handler::<LayoutTransformControl>(|x, e| x.on_use_render_transform_property_changed(e));
    }

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: Decorator::construct(),
            render_transform_changed_event: RefCell::new(None),
            child_actual_size: Cell::new(Size::default()),
            matrix_transform: OnceCell::new(),
            transformation: Cell::new(Matrix::IDENTITY),
            layout_transform_changed: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// A graphics transformation that should apply to this element when
    /// layout is performed.
    pub fn layout_transform(&self) -> Option<Rc<dyn ITransform>> {
        self.get_value(Self::layout_transform_property())
    }

    pub fn set_layout_transform(&self, value: Option<Rc<dyn ITransform>>) {
        self.set_value(Self::layout_transform_property(), value)
    }

    /// Whether the layout transform follows the render transform of the
    /// control.
    pub fn use_render_transform(&self) -> bool {
        self.get_value(Self::use_render_transform_property())
    }

    pub fn set_use_render_transform(&self, value: bool) {
        self.set_value(Self::use_render_transform_property(), value)
    }

    /// The control the transformation is applied to: the child.
    pub fn transform_root(&self) -> Option<Ref<Control>> {
        self.child()
    }

    fn matrix_transform(&self) -> &Ref<MatrixTransform> {
        self.matrix_transform.get_or_init(MatrixTransform::new)
    }

    fn on_use_render_transform_property_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        // HACK: In theory, this method and the `UseRenderTransform` property
        //       shouldn't exist but it's hard to animate this particular
        //       control with style animations without property paths.
        //
        //       So until we get that implemented, we'll stick on this
        //       not-so-good workaround.

        let should_use_render_transform = e.get_new_value::<bool>();
        if should_use_render_transform {
            let subscription = Visual::render_transform_property().changed().subscribe(|x| {
                if let Some(target) = x.sender().downcast_ref::<LayoutTransformControl>() {
                    target.set_layout_transform(target.render_transform());
                }
            });
            *self.render_transform_changed_event.borrow_mut() = Some(subscription);
        } else {
            let subscription = self.render_transform_changed_event.borrow().clone();
            if let Some(subscription) = subscription {
                subscription.dispose();
            }
            self.clear_value(Self::layout_transform_property());
        }
    }

    fn on_child_changed(&self) {
        if let Some(transform_root) = self.transform_root() {
            transform_root.set_render_transform(Some(self.matrix_transform().into()));
            transform_root.set_render_transform_origin(RelativePoint::new(0.0, 0.0, RelativeUnit::Absolute));
        }

        self.apply_layout_transform();
    }

    /// Tests if the first size is significantly smaller than the second.
    fn is_size_smaller(a: Size, b: Size) -> bool {
        (a.width + ACCEPTABLE_DELTA < b.width) || (a.height + ACCEPTABLE_DELTA < b.height)
    }

    /// Rounds the non-offset elements of a matrix to avoid issues due to
    /// floating point imprecision.
    fn round_matrix(matrix: Matrix, decimals: i32) -> Matrix {
        Matrix::new(
            round(matrix.m11, decimals),
            round(matrix.m12, decimals),
            round(matrix.m21, decimals),
            round(matrix.m22, decimals),
            matrix.m31,
            matrix.m32,
        )
    }

    /// Applies the layout transform on the content.
    fn apply_layout_transform(&self) {
        // Get the transform matrix and apply it
        let matrix = match self.layout_transform() {
            None => Matrix::IDENTITY,
            Some(layout_transform) => Self::round_matrix(layout_transform.value(), DECIMALS_AFTER_ROUND),
        };

        if self.transformation.get() == matrix {
            return;
        }

        self.transformation.set(matrix);
        self.matrix_transform().set_matrix(matrix);

        // New transform means re-layout is necessary
        self.invalidate_measure();
    }

    /// Computes the largest usable size after applying the transformation to
    /// the specified bounds.
    fn compute_largest_transformed_size(&self, mut arrange_bounds: Size) -> Size {
        // Computed largest transformed size
        let mut computed_size = Size::default();
        let transformation = self.transformation.get();

        // Detect infinite bounds and constrain the scenario
        let infinite_width = arrange_bounds.width.is_infinite();
        if infinite_width {
            arrange_bounds = arrange_bounds.with_width(arrange_bounds.height);
        }
        let infinite_height = arrange_bounds.height.is_infinite();
        if infinite_height {
            arrange_bounds = arrange_bounds.with_height(arrange_bounds.width);
        }

        // Capture the matrix parameters
        let a = transformation.m11;
        let b = transformation.m12;
        let c = transformation.m21;
        let d = transformation.m22;

        // Compute maximum possible transformed width/height based on starting
        // width/height. These constraints define two lines in the positive
        // x/y quadrant.
        let max_width_from_width = (arrange_bounds.width / a).abs();
        let max_height_from_width = (arrange_bounds.width / c).abs();
        let max_width_from_height = (arrange_bounds.height / b).abs();
        let max_height_from_height = (arrange_bounds.height / d).abs();

        // The transformed width/height that maximize the area under each
        // segment is its midpoint. At most one of the two midpoints will
        // satisfy both constraints.
        let ideal_width_from_width = max_width_from_width / 2.0;
        let ideal_height_from_width = max_height_from_width / 2.0;
        let ideal_width_from_height = max_width_from_height / 2.0;
        let ideal_height_from_height = max_height_from_height / 2.0;

        // Compute slope of both constraint lines
        let slope_from_width = -(max_height_from_width / max_width_from_width);
        let slope_from_height = -(max_height_from_height / max_width_from_height);

        if (0.0 == arrange_bounds.width) || (0.0 == arrange_bounds.height) {
            // Check for empty bounds
            computed_size = Size::new(arrange_bounds.width, arrange_bounds.height);
        } else if infinite_width && infinite_height {
            // Check for completely unbound scenario
            computed_size = Size::new(f64::INFINITY, f64::INFINITY);
        } else if !transformation.has_inverse() {
            // Check for singular matrix
            computed_size = Size::new(0.0, 0.0);
        } else if (0.0 == b) || (0.0 == c) {
            // Check for 0/180 degree special cases
            let max_height = if infinite_height { f64::INFINITY } else { max_height_from_height };
            let max_width = if infinite_width { f64::INFINITY } else { max_width_from_width };
            if (0.0 == b) && (0.0 == c) {
                // No constraints
                computed_size = Size::new(max_width, max_height);
            } else if 0.0 == b {
                // Constrained by width
                let computed_height = min(ideal_height_from_width, max_height);
                computed_size = Size::new(max_width - ((c * computed_height) / a).abs(), computed_height);
            } else if 0.0 == c {
                // Constrained by height
                let computed_width = min(ideal_width_from_height, max_width);
                computed_size = Size::new(computed_width, max_height - ((b * computed_width) / d).abs());
            }
        } else if (0.0 == a) || (0.0 == d) {
            // Check for 90/270 degree special cases
            let max_width = if infinite_height { f64::INFINITY } else { max_width_from_height };
            let max_height = if infinite_width { f64::INFINITY } else { max_height_from_width };
            if (0.0 == a) && (0.0 == d) {
                // No constraints
                computed_size = Size::new(max_width, max_height);
            } else if 0.0 == a {
                // Constrained by width
                let computed_height = min(ideal_height_from_height, max_height);
                computed_size = Size::new(max_width - ((d * computed_height) / b).abs(), computed_height);
            } else if 0.0 == d {
                // Constrained by height
                let computed_width = min(ideal_width_from_width, max_width);
                computed_size = Size::new(computed_width, max_height - ((a * computed_width) / c).abs());
            }
        } else if ideal_height_from_width <= ((slope_from_height * ideal_width_from_width) + max_height_from_height) {
            // Check the width midpoint for viability (by being below the
            // height constraint line)
            computed_size = Size::new(ideal_width_from_width, ideal_height_from_width);
        } else if ideal_height_from_height <= ((slope_from_width * ideal_width_from_height) + max_height_from_width) {
            // Check the height midpoint for viability (by being below the
            // width constraint line)
            computed_size = Size::new(ideal_width_from_height, ideal_height_from_height);
        } else {
            // Neither midpoint is viable; use the intersection of the two
            // constraint lines instead.
            // Compute width by setting heights equal (m1*x+c1=m2*x+c2)
            let computed_width =
                (max_height_from_height - max_height_from_width) / (slope_from_width - slope_from_height);
            // Compute height from width constraint line (y=m*x+c; using
            // height would give same result)
            computed_size = Size::new(computed_width, (slope_from_width * computed_width) + max_height_from_width);
        }

        // Return result
        computed_size
    }

    fn on_layout_transform_changed(&self, e: &FerroPropertyChangedEventArgs<'_>) {
        if self.is_attached_to_visual_tree() {
            self.unsubscribe_layout_transform();
            self.subscribe_layout_transform(e.get_new_value::<Option<Rc<dyn ITransform>>>());
        }

        self.apply_layout_transform();
    }

    /// Subscribes to the changed notification of a mutable transform.
    fn subscribe_layout_transform(&self, transform: Option<Rc<dyn ITransform>>) {
        let Some(transform) = transform else { return };
        let Some(transform) = transform.as_mutable_transform() else { return };

        let weak = self.to_ref().downgrade();
        let subscription = transform.changed(Rc::new(move || {
            if let Some(this) = weak.upgrade() {
                this.apply_layout_transform();
            }
        }));
        *self.layout_transform_changed.borrow_mut() = Some(subscription);
    }

    /// Ends the subscription made by `subscribe_layout_transform`.
    fn unsubscribe_layout_transform(&self) {
        let subscription = self.layout_transform_changed.borrow_mut().take();
        if let Some(subscription) = subscription {
            subscription.dispose();
        }
    }
}

/// Rounds a value to the given number of fractional digits, midpoints to the
/// nearest even digit.
fn round(value: f64, decimals: i32) -> f64 {
    if value.abs() < 1e16 {
        let power10 = 10f64.powi(decimals);
        (value * power10).round_ties_even() / power10
    } else {
        value
    }
}
