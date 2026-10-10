//! Port of `Controls/LineBoundsDemoControl.cs`.

use crate::controls::line_bounds_helper::LineBoundsHelper;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{Brushes, DrawingContext, IBrush, IPen, Pen, PenLineCap, PenLineJoin};
use ferroui_base::threading::DispatcherTimer;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties, instantiate, FerroObjectImpl, FerroProperty,
    Point, Ref, StyledElementImpl, StyledProperty, Visual, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use std::rc::Rc;
use std::time::Duration;

#[repr(C)]
pub struct LineBoundsDemoControl {
    base: Control,
}

ferro_class!(LineBoundsDemoControl: Control);
ferro_impl_classes!(
    LineBoundsDemoControl: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);
ferro_class_info!(LineBoundsDemoControl { new: LineBoundsDemoControl::new });

ferro_properties! {
    impl LineBoundsDemoControl {
        pub fn angle_property() -> StyledProperty<f64> {
            FerroProperty::register::<LineBoundsDemoControl, _>("Angle", 0.0)
        }
    }
}

impl VisualImpl for LineBoundsDemoControl {
    fn render(this: &Self, drawing_context: &mut DrawingContext) {
        let line_length = f64::sqrt((100.0 * 100.0) + (100.0 * 100.0));

        let diff_x = LineBoundsHelper::calculate_adj_side(this.angle(), line_length);
        let diff_y = LineBoundsHelper::calculate_opp_side(this.angle(), line_length);

        let p1 = Point::new(200.0, 200.0);
        let p2 = Point::new(p1.x + diff_x, p1.y + diff_y);

        let pen: Rc<dyn IPen> = Pen::with_all(
            Some(Brushes::green() as Rc<dyn IBrush>),
            20.0,
            None,
            PenLineCap::Square,
            PenLineJoin::Miter,
            10.0,
        )
        .into();
        let bound_pen: Rc<dyn IPen> = Pen::with_brush(Some(Brushes::black() as Rc<dyn IBrush>), 1.0).into();

        drawing_context.draw_line(&pen, p1, p2);

        drawing_context.draw_rectangle_outline(&bound_pen, LineBoundsHelper::calculate_bounds(p1, p2, &*pen), 0.0);
    }
}

impl LineBoundsDemoControl {
    fn static_constructor() {
        Visual::affects_render::<LineBoundsDemoControl>(&[Self::angle_property().as_property()]);
    }

    pub fn construct() -> Self {
        Self { base: Control::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());

        let timer = DispatcherTimer::new();
        timer.set_interval(Duration::from_secs_f64(1.0 / 60.0));
        // The timer is kept by the dispatcher while it runs: its handler holds the control weakly.
        let weak = this.downgrade();
        timer.tick(move |_| {
            if let Some(this) = weak.upgrade() {
                this.set_angle(this.angle() + std::f64::consts::PI / 360.0);
            }
        });
        timer.start();

        this
    }

    pub fn angle(&self) -> f64 {
        self.get_value(Self::angle_property())
    }

    pub fn set_angle(&self, value: f64) {
        self.set_value(Self::angle_property(), value)
    }
}
