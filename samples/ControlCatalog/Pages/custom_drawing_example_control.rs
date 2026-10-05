//! Port of `Pages/CustomDrawingExampleControl.cs`.

use ferroui_base::input::{
    InputElement, InputElementImpl, InputElementImplExt, PointerEventArgs, PointerPressedEventArgs, PointerReleasedEventArgs,
    PointerWheelEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{
    ArcSegment, BoxShadows, Brushes, Colors, DrawingContext, Geometry, IBrush, IPen, Pen, PenLineCap,
    PenLineJoin, SolidColorBrush, StreamGeometry, SweepDirection,
};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObject, FerroObjectImpl,
    FerroProperty, Matrix, Point, Rect, Ref, Size, StyledElementImpl, StyledProperty, StyledPropertyOptions, Vector,
    VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use std::cell::Cell;
use std::rc::Rc;

#[repr(C)]
pub struct CustomDrawingExampleControl {
    base: Control,
    cursor_point: Cell<Point>,
    smile_geometry: Ref<Geometry>,
    pen: Rc<dyn IPen>,
    /// The time the stopwatch of the control was started at, in the
    /// milliseconds of the clock of the dispatcher.
    time_keeper: i64,
    is_pointer_captured: Cell<bool>,
}

ferro_class!(CustomDrawingExampleControl: Control);
ferro_impl_classes!(
    CustomDrawingExampleControl: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl
);
ferro_class_info!(CustomDrawingExampleControl { new: CustomDrawingExampleControl::new });

ferro_properties! {
    impl CustomDrawingExampleControl {
        pub fn scale_property() -> StyledProperty<f64> {
            FerroProperty::register::<CustomDrawingExampleControl, _>("Scale", 1.0)
        }

        pub fn rotation_property() -> StyledProperty<f64> {
            FerroProperty::register_with::<CustomDrawingExampleControl, _>(
                "Rotation",
                StyledPropertyOptions::new(0.0).coerce(CustomDrawingExampleControl::coerce_rotation),
            )
        }

        pub fn viewport_center_y_property() -> StyledProperty<f64> {
            FerroProperty::register::<CustomDrawingExampleControl, _>("ViewportCenterY", 0.0)
        }

        pub fn viewport_center_x_property() -> StyledProperty<f64> {
            FerroProperty::register::<CustomDrawingExampleControl, _>("ViewportCenterX", 0.0)
        }
    }
}

impl InputElementImpl for CustomDrawingExampleControl {
    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        Self::parent_on_pointer_moved(this, e);

        let previous_point = this.cursor_point.get();

        this.cursor_point.set(e.get_position(Some(this)));

        if this.is_pointer_captured.get() {
            let old_world_point = this.ui_point_to_world_point(
                previous_point,
                this.viewport_center_x(),
                this.viewport_center_y(),
                this.scale(),
                this.rotation(),
            );
            let new_world_point = this.ui_point_to_world_point(
                this.cursor_point.get(),
                this.viewport_center_x(),
                this.viewport_center_y(),
                this.scale(),
                this.rotation(),
            );

            let diff = new_world_point - old_world_point;

            this.set_viewport_center_x(this.viewport_center_x() - diff.x);
            this.set_viewport_center_y(this.viewport_center_y() - diff.y);
        }
    }

    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        e.set_handled(true);
        e.pointer().capture(Some(&this.to_ref().upcast::<InputElement>()));
        this.is_pointer_captured.set(true);
        Self::parent_on_pointer_pressed(this, e);
    }

    fn on_pointer_wheel_changed(this: &Self, e: &PointerWheelEventArgs) {
        Self::parent_on_pointer_wheel_changed(this, e);
        let old_scale = this.scale();
        this.set_scale(this.scale() * (1.0 + e.delta().y / 12.0));

        let cursor_point = this.cursor_point.get();
        let old_world_point = this.ui_point_to_world_point(
            cursor_point,
            this.viewport_center_x(),
            this.viewport_center_y(),
            old_scale,
            this.rotation(),
        );
        let new_world_point = this.ui_point_to_world_point(
            cursor_point,
            this.viewport_center_x(),
            this.viewport_center_y(),
            this.scale(),
            this.rotation(),
        );

        let diff = new_world_point - old_world_point;

        this.set_viewport_center_x(this.viewport_center_x() - diff.x);
        this.set_viewport_center_y(this.viewport_center_y() - diff.y);
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        e.pointer().capture(None);
        this.is_pointer_captured.set(false);
        Self::parent_on_pointer_released(this, e);
    }
}

impl VisualImpl for CustomDrawingExampleControl {
    fn render(this: &Self, context: &mut DrawingContext) {
        let bounds = this.bounds();
        let white: Rc<dyn IBrush> = Brushes::white();
        let gray: Rc<dyn IBrush> = Brushes::gray();
        let pen = &this.pen;

        let local_bounds = Rect::from_size(Size::new(bounds.width, bounds.height));
        let clip = context.push_clip(bounds);
        context.draw_rectangle(Some(&white), Some(pen), local_bounds, 1.0, 0.0, &BoxShadows::default());

        let _half_max = (bounds.width / 2.0).max(bounds.height / 2.0) * 2.0_f64.sqrt();
        let _half_min = (bounds.width / 2.0).min(bounds.height / 2.0) / 1.3;
        let half_width = bounds.width / 2.0;
        let half_height = bounds.height / 2.0;

        // 0,0 refers to the top-left of the control now. It is not prime time to draw gui stuff because it'll be under the world

        let translate_modifier =
            context.push_transform(Matrix::create_translation_vector(Vector::new(half_width, half_height)));

        // now 0,0 refers to the ViewportCenter(X,Y).
        let rotation_matrix = Matrix::create_rotation(this.rotation());
        let rotation_modifier = context.push_transform(rotation_matrix);

        // everything is rotated but not scaled

        let scale_modifier = context.push_transform(Matrix::create_scale(this.scale(), -this.scale()));

        let map_position_modifier = context.push_transform(Matrix::create_translation_vector(Vector::new(
            -this.viewport_center_x(),
            -this.viewport_center_y(),
        )));

        // now everything is rotated and scaled, and at the right position, now we're drawing strictly in world coordinates

        context.draw_ellipse_at(Some(&white), Some(pen), Point::new(0.0, 0.0), 50.0, 50.0);
        context.draw_line(pen, Point::new(-25.0, -5.0), Point::new(-25.0, 15.0));
        context.draw_line(pen, Point::new(25.0, -5.0), Point::new(25.0, 15.0));
        context.draw_geometry(None, Some(pen), &this.smile_geometry);

        let cursor_in_world_point = this.ui_point_to_world_point(
            this.cursor_point.get(),
            this.viewport_center_x(),
            this.viewport_center_y(),
            this.scale(),
            this.rotation(),
        );
        context.draw_ellipse_at(Some(&gray), Some(pen), cursor_in_world_point, 20.0, 20.0);

        let elapsed_milliseconds = (Dispatcher::ui_thread().now() - this.time_keeper) as f64;
        for i in 0..10 {
            let orbit_radius = f64::from(i * 100 + 200);
            let mut orbit_input = ((elapsed_milliseconds + 987654.0) / orbit_radius) / 10.0;
            if i % 3 == 0 {
                orbit_input *= -1.0;
            }
            let orbit_position = Point::new(orbit_input.sin() * orbit_radius, orbit_input.cos() * orbit_radius);
            context.draw_ellipse_at(Some(&gray), Some(pen), orbit_position, 20.0, 20.0);
        }

        // end drawing the world

        context.pop(map_position_modifier);

        context.pop(scale_modifier);

        context.pop(rotation_modifier);
        context.pop(translate_modifier);

        // this is prime time to draw gui stuff

        let cursor_point = this.cursor_point.get();
        context.draw_line(pen, cursor_point + Vector::new(-20.0, 0.0), cursor_point + Vector::new(20.0, 0.0));
        context.draw_line(pen, cursor_point + Vector::new(0.0, -20.0), cursor_point + Vector::new(0.0, 20.0));

        context.pop(clip);

        // oh and draw again when you can, no rush, right?
        let this = this.to_ref();
        Dispatcher::ui_thread().post_local(move || this.invalidate_visual(), DispatcherPriority::BACKGROUND);
    }
}

impl CustomDrawingExampleControl {
    pub fn construct() -> Self {
        let pen: Rc<dyn IPen> = Pen::with_all(
            Some(SolidColorBrush::with_color(Colors::BLACK).into()),
            1.0,
            None,
            PenLineCap::Round,
            PenLineJoin::Miter,
            10.0,
        )
        .into();

        let arc = ArcSegment::new();
        arc.set_is_large_arc(false);
        arc.set_point(Point::new(0.0, 0.0));
        arc.set_rotation_angle(0.0);
        arc.set_size(Size::new(25.0, 25.0));
        arc.set_sweep_direction(SweepDirection::Clockwise);

        let sg = StreamGeometry::new();
        {
            let mut cntx = sg.open();
            cntx.begin_figure(Point::new(-25.0, -10.0), false);
            cntx.arc_to(Point::new(25.0, -10.0), Size::new(10.0, 10.0), 0.0, false, SweepDirection::Clockwise, true);
            cntx.end_figure(true);
            cntx.dispose();
        }
        let smile_geometry = sg.clone_geometry();

        Self {
            base: Control::construct(),
            cursor_point: Cell::new(Point::default()),
            smile_geometry,
            pen,
            time_keeper: Dispatcher::ui_thread().now(),
            is_pointer_captured: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn scale(&self) -> f64 {
        self.get_value(Self::scale_property())
    }

    pub fn set_scale(&self, value: f64) {
        self.set_value(Self::scale_property(), value)
    }

    /// Rotation, measured in Radians!
    pub fn rotation(&self) -> f64 {
        self.get_value(Self::rotation_property())
    }

    pub fn set_rotation(&self, value: f64) {
        self.set_value(Self::rotation_property(), value)
    }

    pub fn viewport_center_y(&self) -> f64 {
        self.get_value(Self::viewport_center_y_property())
    }

    pub fn set_viewport_center_y(&self, value: f64) {
        self.set_value(Self::viewport_center_y_property(), value)
    }

    pub fn viewport_center_x(&self) -> f64 {
        self.get_value(Self::viewport_center_x_property())
    }

    pub fn set_viewport_center_x(&self, value: f64) {
        self.set_value(Self::viewport_center_x_property(), value)
    }

    fn coerce_rotation(_sender: &FerroObject, value: f64) -> f64 {
        value % (std::f64::consts::PI * 2.0)
    }

    fn ui_point_to_world_point(
        &self,
        in_point: Point,
        viewport_center_x: f64,
        viewport_center_y: f64,
        scale: f64,
        rotation: f64,
    ) -> Point {
        let bounds = self.bounds();
        let mut working_point = Point::new(in_point.x, -in_point.y);
        working_point += Vector::new(-bounds.width / 2.0, bounds.height / 2.0);
        working_point = working_point / scale;

        working_point = Matrix::create_rotation(rotation).transform(working_point);

        working_point += Vector::new(viewport_center_x, viewport_center_y);

        working_point
    }

    #[allow(dead_code)]
    fn world_point_to_ui_point(
        &self,
        in_point: Point,
        viewport_center_x: f64,
        viewport_center_y: f64,
        scale: f64,
        rotation: f64,
    ) -> Point {
        let bounds = self.bounds();
        let mut working_point = Point::new(in_point.x, in_point.y);

        working_point = working_point - Vector::new(viewport_center_x, viewport_center_y);
        // undo rotation
        working_point = Matrix::create_rotation(-rotation).transform(working_point);
        working_point = working_point * scale;
        working_point = working_point - Vector::new(-bounds.width / 2.0, bounds.height / 2.0);
        working_point = Point::new(working_point.x, -working_point.y);

        working_point
    }
}
