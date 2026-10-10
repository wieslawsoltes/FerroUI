//! Port of `Controls/ResizePattern.cs`.

use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{
    BoxShadows, Brushes, Color, DrawingContext, FlowDirection, FormattedText, IBrush, IPen, Pen, SolidColorBrush,
    Typeface,
};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Rect, Ref,
    StyledElementImpl, Visual, VisualImpl,
};
use ferroui_controls::{Control, ControlImpl};
use std::rc::Rc;

thread_local! {
    static GRID_PEN: Rc<dyn IPen> =
        Pen::with_brush(Some(SolidColorBrush::with_color(Color::from_rgb(60, 90, 130)).into()), 1.0).into();
    static DIAGONAL_PEN: Rc<dyn IPen> =
        Pen::with_brush(Some(SolidColorBrush::with_color(Color::from_rgb(90, 90, 90)).into()), 1.0).into();
    static CIRCLE_PEN: Rc<dyn IPen> = Pen::with_brush(Some(Brushes::orange_red() as Rc<dyn IBrush>), 4.0).into();
    static EDGE_PEN: Rc<dyn IPen> = Pen::with_brush(Some(Brushes::lime() as Rc<dyn IBrush>), 6.0).into();
    static BACKGROUND: Rc<dyn IBrush> = SolidColorBrush::with_color(Color::from_rgb(16, 16, 24)).into();
    static TEXT_BRUSH: Rc<dyn IBrush> = Brushes::white();
}

const GRID_STEP: f64 = 40.0;

#[repr(C)]
pub struct ResizePattern {
    base: Control,
}

ferro_class!(ResizePattern: Control);
ferro_impl_classes!(
    ResizePattern: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);
ferro_class_info!(ResizePattern { new: ResizePattern::new });

impl VisualImpl for ResizePattern {
    fn render(this: &Self, context: &mut DrawingContext) {
        let w = this.bounds().width;
        let h = this.bounds().height;
        if w <= 0.0 || h <= 0.0 {
            return;
        }

        let grid_pen = GRID_PEN.with(Rc::clone);
        let diagonal_pen = DIAGONAL_PEN.with(Rc::clone);
        let circle_pen = CIRCLE_PEN.with(Rc::clone);
        let edge_pen = EDGE_PEN.with(Rc::clone);
        let background = BACKGROUND.with(Rc::clone);
        let text_brush = TEXT_BRUSH.with(Rc::clone);

        let rect = Rect::new(0.0, 0.0, w, h);
        context.fill_rectangle(&background, rect, 0.0);

        // Grid -- uneven spacing on the far edges is the tell-tale of stretching.
        let mut x = 0.0;
        while x <= w {
            context.draw_line(&grid_pen, Point::new(x, 0.0), Point::new(x, h));
            x += GRID_STEP;
        }
        let mut y = 0.0;
        while y <= h {
            context.draw_line(&grid_pen, Point::new(0.0, y), Point::new(w, y));
            y += GRID_STEP;
        }

        // Corner-to-corner diagonals -- they only meet exactly at the centre
        // when the aspect ratio is correct.
        context.draw_line(&diagonal_pen, Point::new(0.0, 0.0), Point::new(w, h));
        context.draw_line(&diagonal_pen, Point::new(w, 0.0), Point::new(0.0, h));

        // Edge frame -- a stretched drawable makes this peel away from the window edge.
        context.draw_rectangle(None, Some(&edge_pen), rect.deflate(3.0), 0.0, 0.0, &BoxShadows::default());

        // Concentric perfect circles centred in the window.
        let center = Point::new(w / 2.0, h / 2.0);
        let max_radius = f64::max(10.0, f64::min(w, h) / 2.0 - 16.0);
        for i in 1..=3 {
            let r = max_radius * f64::from(i) / 3.0;
            context.draw_ellipse_at(None, Some(&circle_pen), center, r, r);
        }

        // Centre crosshair.
        context.draw_line(&circle_pen, Point::new(center.x - 20.0, center.y), Point::new(center.x + 20.0, center.y));
        context.draw_line(&circle_pen, Point::new(center.x, center.y - 20.0), Point::new(center.x, center.y + 20.0));

        // Live size read-out.
        let text = FormattedText::new(
            // The format `0` rounds midpoints away from zero, as `round` does.
            &format!("{} x {} DIP", w.round(), h.round()),
            CultureInfo::invariant_culture(),
            FlowDirection::LeftToRight,
            Typeface::default(),
            22.0,
            Some(text_brush),
        );
        context.draw_text(&text, Point::new(12.0, 8.0));
    }
}

impl ResizePattern {
    fn static_constructor() {
        Visual::affects_render::<ResizePattern>(&[Visual::bounds_property().as_property()]);
    }

    pub fn construct() -> Self {
        Self { base: Control::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
