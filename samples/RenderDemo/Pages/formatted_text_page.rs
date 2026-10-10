//! Port of `Pages/FormattedTextPage.xaml.cs`: the class of the document
//! `Pages/FormattedTextPage.xaml`.

use crate::markup::xaml_class;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::immutable::ImmutablePen;
use ferroui_base::media::{
    Brushes, Colors, DrawingContext, FlowDirection, FontStyle, FontWeight, FormattedText, GradientStop, GradientStops,
    IBrush, IPen, LinearGradientBrush, Typeface,
};
use ferroui_base::utilities::CultureInfo;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Point, Ref, RelativePoint,
    RelativeUnit, StyledElementImpl, VisualImpl,
};
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{ContentControlImpl, ControlImpl, UserControl};
use std::rc::Rc;

#[repr(C)]
pub struct FormattedTextPage {
    base: UserControl,
}

ferro_class!(FormattedTextPage: UserControl);
ferro_impl_classes!(
    FormattedTextPage: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl
);
ferro_class_info!(FormattedTextPage { new: FormattedTextPage::new });
xaml_class!(FormattedTextPage, "/Pages/FormattedTextPage.xaml");

impl VisualImpl for FormattedTextPage {
    fn render(_this: &Self, context: &mut DrawingContext) {
        const TEST_STRING: &str = "Lorem ipsum dolor sit amet, consectetur adipisicing elit, sed do eiusmod tempor";

        // Create the initial formatted text string.
        let formatted_text = FormattedText::new(
            TEST_STRING,
            CultureInfo::get_culture_info("en-us"),
            FlowDirection::LeftToRight,
            Typeface::from_name("Verdana"),
            32.0,
            Some(Brushes::black() as Rc<dyn IBrush>),
        );
        formatted_text.set_max_text_width(300.0);
        formatted_text.set_max_text_height(240.0);

        // Set a maximum width and height. If the text overflows these values, an ellipsis "..." appears.

        // Use a larger font size beginning at the first (zero-based) character and continuing for 5 characters.
        // The font size is calculated in terms of points -- not as device-independent pixels.
        formatted_text.set_font_size_range(36.0 * (96.0 / 72.0), 0, 5);

        // Use a Bold font weight beginning at the 6th character and continuing for 11 characters.
        formatted_text.set_font_weight_range(FontWeight::Bold, 6, 11);

        let gradient = LinearGradientBrush::new();
        gradient.set_gradient_stops(GradientStops::from_items([
            GradientStop::with_color_and_offset(Colors::ORANGE, 0.0),
            GradientStop::with_color_and_offset(Colors::TEAL, 1.0),
        ]));
        gradient.set_start_point(RelativePoint::new(0.0, 0.0, RelativeUnit::Relative));
        gradient.set_end_point(RelativePoint::new(0.0, 1.0, RelativeUnit::Relative));
        let gradient_brush: Rc<dyn IBrush> = (&gradient).into();

        // Use a linear gradient brush beginning at the 6th character and continuing for 11 characters.
        formatted_text.set_foreground_brush_range(Some(gradient_brush.clone()), 6, 11);

        // Use an Italic font style beginning at the 28th character and continuing for 28 characters.
        formatted_text.set_font_style_range(FontStyle::Italic, 28, 28);

        context.draw_text(&formatted_text, Point::new(10.0, 0.0));

        let geometry = formatted_text
            .build_geometry(Point::new(10.0 + formatted_text.width() + 10.0, 0.0))
            .expect("the geometry of the formatted text");

        context.draw_geometry(Some(&gradient_brush), None, &geometry);

        let highlight_geometry = formatted_text
            .build_highlight_geometry(Point::new(10.0 + formatted_text.width() + 10.0, 0.0))
            .expect("the highlight geometry of the formatted text");

        let highlight_pen: Rc<dyn IPen> = Rc::new(ImmutablePen::with_brush(Some(gradient.to_immutable()), 2.0));
        context.draw_geometry(None, Some(&highlight_pen), &highlight_geometry);
    }
}

impl FormattedTextPage {
    pub fn construct() -> Self {
        Self { base: UserControl::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();
        this
    }
}
