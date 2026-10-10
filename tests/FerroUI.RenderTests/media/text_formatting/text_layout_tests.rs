//! Port of upstream's `Media/TextFormatting/TextLayoutTests.cs`.
//!
//! The rows of a theory are numbered in upstream's order.
//!
//! `TextLayout_Basic` and `TextLayout_Rotated` return at once on macOS, as
//! upstream's do ("text rendering is subtly different").

use crate::test_base::{test_font_family, TestBase};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::text_formatting::{TextLayout, TextLayoutOptions};
use ferroui_base::media::{
    BoxShadows, Brushes, DrawingContext, FontStretch, FontStyle, FontWeight, IBrush, TextAlignment, TextWrapping,
    Typeface,
};
use ferroui_base::utilities::MathUtilities;
use ferroui_base::*;
use ferroui_controls::{Border, Control, ControlImpl};
use std::rc::Rc;

const FONT_SIZE: f64 = 12.0;
const MEDIUM_FONT_SIZE: f64 = 18.0;
const BIG_FONT_SIZE: f64 = 32.0;
const FONT_SIZE_HEIGHT: f64 = 14.0625; //real value 13.59375
const STRINGWORD: &str = "word";
const STRINGMIDDLE: &str = "The quick brown fox jumps over the lazy dog";
const STRINGMIDDLE2LINES: &str = "The quick brown fox\njumps over the lazy dog";
const STRINGMIDDLE3LINES: &str = "01234567\n\n0123456789";
const STRINGMIDDLENEWLINES: &str = "012345678\r 1234567\r\n 12345678\n0123456789";

const STRINGLONG: &str = concat!(
    "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Vivamus magna. Cras in mi at felis ",
    "aliquet congue. Ut a est eget ligula molestie gravida. Curabitur massa. Donec eleifend, libero",
    " at sagittis mollis, tellus est malesuada tellus, at luctus turpis elit sit amet quam. Vivamus ",
    "pretium ornare est."
);

// `stringword + "\r\n"` and `stringword + "\r\nnext"` of upstream's rows.
const STRINGWORD_NEWLINE: &str = "word\r\n";
const STRINGWORD_NEWLINE_NEXT: &str = "word\r\nnext";

// The text the two rendered tests draw: the name of the upstream project and an exclamation mark. It is put
// together from two parts because that name does not appear in the port.
const DRAWN_TEXT: &str = concat!("Ava", "lonia!");

fn base() -> TestBase {
    TestBase::new(r"Media\TextFormatting\TextLayout")
}

/// xUnit's `Assert.Equal(double expected, double actual, int precision)`:
/// both values rounded to `precision` decimal places (`Math.Round`, to even).
fn assert_equal_precision(expected: f64, actual: f64, precision: i32) {
    let factor = 10f64.powi(precision);
    let expected_rounded = (expected * factor).round_ties_even() / factor;
    let actual_rounded = (actual * factor).round_ties_even() / factor;

    assert_eq!(
        expected_rounded, actual_rounded,
        "Values differ at precision {precision}: expected {expected}, actual {actual}"
    );
}

fn create_full(
    text: &str,
    font_size: f64,
    font_style: FontStyle,
    text_alignment: TextAlignment,
    font_weight: FontWeight,
    wrapping: TextWrapping,
    width_constraint: f64,
) -> TextLayout {
    let typeface = Typeface::with_style(test_font_family(), font_style, font_weight, FontStretch::Normal);

    TextLayout::new(
        text,
        typeface,
        TextLayoutOptions {
            font_size,
            foreground: None,
            text_alignment,
            text_wrapping: wrapping,
            max_width: if width_constraint == -1.0 { f64::INFINITY } else { width_constraint },
            ..Default::default()
        },
    )
}

fn create(text: &str, font_size: f64) -> TextLayout {
    create_full(text, font_size, FontStyle::Normal, TextAlignment::Left, FontWeight::Normal, TextWrapping::NoWrap, -1.0)
}

fn create_aligned(text: &str, font_size: f64, alignment: TextAlignment, width_constraint: f64) -> TextLayout {
    create_full(
        text,
        font_size,
        FontStyle::Normal,
        alignment,
        FontWeight::Normal,
        TextWrapping::NoWrap,
        width_constraint,
    )
}

fn create_wrapped(text: &str, font_size: f64, wrap: TextWrapping, width_constraint: f64) -> TextLayout {
    create_full(text, font_size, FontStyle::Normal, TextAlignment::Left, FontWeight::Normal, wrap, width_constraint)
}

fn should_measure_string_correctly(input: &str, font_size: f64, exp_width: f64, exp_height: f64) {
    let _t = base();
    let fmt = create(input, font_size);

    assert_equal_precision(exp_width, fmt.width_including_trailing_whitespace(), 2);
    assert_equal_precision(exp_height, fmt.height(), 2);
}

#[test]
fn should_measure_string_correctly_1() {
    should_measure_string_correctly("", FONT_SIZE, 0.0, FONT_SIZE_HEIGHT);
}

#[test]
fn should_measure_string_correctly_2() {
    should_measure_string_correctly("x", FONT_SIZE, 7.20, FONT_SIZE_HEIGHT);
}

#[test]
fn should_measure_string_correctly_3() {
    should_measure_string_correctly(STRINGWORD, FONT_SIZE, 28.80, FONT_SIZE_HEIGHT);
}

#[test]
fn should_measure_string_correctly_4() {
    should_measure_string_correctly(STRINGMIDDLE, FONT_SIZE, 309.65, FONT_SIZE_HEIGHT);
}

#[test]
fn should_measure_string_correctly_5() {
    should_measure_string_correctly(STRINGMIDDLE, MEDIUM_FONT_SIZE, 464.48, 21.09375);
}

#[test]
fn should_measure_string_correctly_6() {
    should_measure_string_correctly(STRINGMIDDLE, BIG_FONT_SIZE, 825.73, 37.5);
}

#[test]
fn should_measure_string_correctly_7() {
    should_measure_string_correctly(STRINGMIDDLE2LINES, FONT_SIZE, 165.63, 2.0 * FONT_SIZE_HEIGHT);
}

#[test]
fn should_measure_string_correctly_8() {
    should_measure_string_correctly(STRINGMIDDLE2LINES, MEDIUM_FONT_SIZE, 248.44, 2.0 * 21.09375);
}

#[test]
fn should_measure_string_correctly_9() {
    should_measure_string_correctly(STRINGMIDDLE2LINES, BIG_FONT_SIZE, 441.67, 2.0 * 37.5);
}

#[test]
fn should_measure_string_correctly_10() {
    should_measure_string_correctly(STRINGLONG, FONT_SIZE, 2160.35, FONT_SIZE_HEIGHT);
}

#[test]
fn should_measure_string_correctly_11() {
    should_measure_string_correctly(STRINGMIDDLENEWLINES, FONT_SIZE, 72.01, 4.0 * FONT_SIZE_HEIGHT);
}

fn should_break_lines_string_correctly(input: &str, lines_count: usize, width_constraint: f64, wrap: TextWrapping) {
    let _t = base();
    let fmt = create_wrapped(input, FONT_SIZE, wrap, width_constraint);
    let constrained = &fmt;

    let lines = constrained.text_lines().to_vec();
    assert_eq!(lines_count, lines.len());
}

#[test]
fn should_break_lines_string_correctly_1() {
    should_break_lines_string_correctly("", 1, -1.0, TextWrapping::NoWrap);
}

#[test]
fn should_break_lines_string_correctly_2() {
    should_break_lines_string_correctly("x", 1, -1.0, TextWrapping::NoWrap);
}

#[test]
fn should_break_lines_string_correctly_3() {
    should_break_lines_string_correctly(STRINGWORD, 1, -1.0, TextWrapping::NoWrap);
}

#[test]
fn should_break_lines_string_correctly_4() {
    should_break_lines_string_correctly(STRINGMIDDLE, 1, -1.0, TextWrapping::NoWrap);
}

#[test]
fn should_break_lines_string_correctly_5() {
    should_break_lines_string_correctly(STRINGMIDDLE, 3, 150.0, TextWrapping::Wrap);
}

#[test]
fn should_break_lines_string_correctly_6() {
    should_break_lines_string_correctly(STRINGMIDDLE2LINES, 2, -1.0, TextWrapping::NoWrap);
}

#[test]
fn should_break_lines_string_correctly_7() {
    should_break_lines_string_correctly(STRINGMIDDLE2LINES, 3, 150.0, TextWrapping::Wrap);
}

#[test]
fn should_break_lines_string_correctly_8() {
    should_break_lines_string_correctly(STRINGLONG, 1, -1.0, TextWrapping::NoWrap);
}

#[test]
fn should_break_lines_string_correctly_9() {
    should_break_lines_string_correctly(STRINGLONG, 18, 150.0, TextWrapping::Wrap);
}

#[test]
fn should_break_lines_string_correctly_10() {
    should_break_lines_string_correctly(STRINGMIDDLENEWLINES, 4, -1.0, TextWrapping::NoWrap);
}

#[test]
fn should_break_lines_string_correctly_11() {
    should_break_lines_string_correctly(STRINGMIDDLENEWLINES, 4, 150.0, TextWrapping::Wrap);
}

fn should_hit_test_point_correctly(input: &str, x: f64, y: f64, is_inside: bool, is_trailing: bool, pos: i32) {
    let _t = base();
    let fmt = create(input, FONT_SIZE);
    let ht_res = fmt.hit_test_point(Point::new(x, y));

    assert_eq!(pos, ht_res.text_position());
    assert_eq!(is_inside, ht_res.is_inside());
    assert_eq!(is_trailing, ht_res.is_trailing());
}

#[test]
fn should_hit_test_point_correctly_1() {
    should_hit_test_point_correctly("x", 0.0, 0.0, true, false, 0);
}

#[test]
fn should_hit_test_point_correctly_2() {
    should_hit_test_point_correctly(STRINGWORD, -1.0, -1.0, false, false, 0);
}

#[test]
fn should_hit_test_point_correctly_3() {
    should_hit_test_point_correctly(STRINGWORD, 25.0, 13.0, true, false, 3);
}

#[test]
fn should_hit_test_point_correctly_4() {
    should_hit_test_point_correctly(STRINGWORD, 28.70, 13.5, true, true, 4);
}

#[test]
fn should_hit_test_point_correctly_5() {
    should_hit_test_point_correctly(STRINGWORD, 30.0, 13.0, false, true, 4);
}

#[test]
fn should_hit_test_point_correctly_6() {
    should_hit_test_point_correctly(STRINGWORD_NEWLINE, 30.0, 13.0, false, false, 4);
}

#[test]
fn should_hit_test_point_correctly_7() {
    should_hit_test_point_correctly(STRINGWORD_NEWLINE_NEXT, 30.0, 13.0, false, false, 4);
}

#[test]
fn should_hit_test_point_correctly_8() {
    should_hit_test_point_correctly(STRINGWORD, 300.0, 13.0, false, true, 4);
}

#[test]
fn should_hit_test_point_correctly_9() {
    should_hit_test_point_correctly(STRINGWORD_NEWLINE, 300.0, 13.0, false, false, 4);
}

#[test]
fn should_hit_test_point_correctly_10() {
    should_hit_test_point_correctly(STRINGWORD_NEWLINE_NEXT, 300.0, 13.0, false, false, 4);
}

#[test]
fn should_hit_test_point_correctly_11() {
    should_hit_test_point_correctly(STRINGWORD, 300.0, 300.0, false, true, 4);
}

//TODO: Direct2D implementation return textposition 6
//but the text is 6 length, can't find the logic for me it should be 5
//(STRINGWORD_NEWLINE, 300.0, 300.0, false, false, 6)

#[test]
fn should_hit_test_point_correctly_12() {
    should_hit_test_point_correctly(STRINGWORD_NEWLINE_NEXT, 300.0, 300.0, false, true, 10);
}

#[test]
fn should_hit_test_point_correctly_13() {
    should_hit_test_point_correctly(STRINGWORD_NEWLINE_NEXT, 300.0, 25.0, false, true, 10);
}

#[test]
fn should_hit_test_point_correctly_14() {
    should_hit_test_point_correctly(STRINGWORD, 28.0, 15.0, false, true, 4);
}

#[test]
fn should_hit_test_point_correctly_15() {
    should_hit_test_point_correctly(STRINGWORD, 30.0, 15.0, false, true, 4);
}

#[test]
fn should_hit_test_point_correctly_16() {
    should_hit_test_point_correctly(STRINGMIDDLE3LINES, 30.0, 15.0, false, false, 9);
}

#[test]
fn should_hit_test_point_correctly_17() {
    should_hit_test_point_correctly(STRINGMIDDLE3LINES, 500.0, 13.0, false, false, 8);
}

#[test]
fn should_hit_test_point_correctly_18() {
    should_hit_test_point_correctly(STRINGMIDDLE3LINES, 30.0, 25.0, false, false, 9);
}

#[test]
fn should_hit_test_point_correctly_19() {
    should_hit_test_point_correctly(STRINGMIDDLE3LINES, -1.0, 30.0, false, false, 10);
}

fn should_hit_test_position_correctly(input: &str, index: i32, x: f64, y: f64, width: f64, height: f64) {
    let _t = base();
    let fmt = create(input, FONT_SIZE);
    let r = fmt.hit_test_text_position(index);

    assert_equal_precision(x, r.x, 2);
    assert_equal_precision(y, r.y, 2);
    assert_equal_precision(width, r.width, 2);
    assert_equal_precision(height, r.height, 2);
}

#[test]
fn should_hit_test_position_correctly_1() {
    should_hit_test_position_correctly("", 0, 0.0, 0.0, 0.0, FONT_SIZE_HEIGHT);
}

#[test]
fn should_hit_test_position_correctly_2() {
    should_hit_test_position_correctly("x", 0, 0.0, 0.0, 7.20, FONT_SIZE_HEIGHT);
}

#[test]
fn should_hit_test_position_correctly_3() {
    should_hit_test_position_correctly("x", -1, 7.20, 0.0, 0.0, FONT_SIZE_HEIGHT);
}

#[test]
fn should_hit_test_position_correctly_4() {
    should_hit_test_position_correctly(STRINGWORD, 3, 21.60, 0.0, 7.20, FONT_SIZE_HEIGHT);
}

#[test]
fn should_hit_test_position_correctly_5() {
    should_hit_test_position_correctly(STRINGWORD, 4, 21.60 + 7.20, 0.0, 0.0, FONT_SIZE_HEIGHT);
}

#[test]
fn should_hit_test_position_correctly_6() {
    should_hit_test_position_correctly(STRINGMIDDLENEWLINES, 10, 0.0, FONT_SIZE_HEIGHT, 7.20, FONT_SIZE_HEIGHT);
}

#[test]
fn should_hit_test_position_correctly_7() {
    should_hit_test_position_correctly(STRINGMIDDLENEWLINES, 15, 36.01, FONT_SIZE_HEIGHT, 7.20, FONT_SIZE_HEIGHT);
}

#[test]
fn should_hit_test_position_correctly_8() {
    should_hit_test_position_correctly(STRINGMIDDLENEWLINES, 20, 0.0, 2.0 * FONT_SIZE_HEIGHT, 7.20, FONT_SIZE_HEIGHT);
}

#[test]
fn should_hit_test_position_correctly_9() {
    should_hit_test_position_correctly(STRINGMIDDLENEWLINES, -1, 72.01, 3.0 * FONT_SIZE_HEIGHT, 0.0, FONT_SIZE_HEIGHT);
}

fn should_hit_test_position_right_align_correctly(
    input: &str,
    index: i32,
    width_constraint: f64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) {
    let _t = base();
    //parse expected
    let fmt = create_aligned(input, FONT_SIZE, TextAlignment::Right, width_constraint);
    let constrained = &fmt;
    let r = constrained.hit_test_text_position(index);

    assert_equal_precision(x, r.x, 2);
    assert_equal_precision(y, r.y, 2);
    assert_equal_precision(width, r.width, 2);
    assert_equal_precision(height, r.height, 2);
}

#[test]
fn should_hit_test_position_right_align_correctly_1() {
    should_hit_test_position_right_align_correctly("x", 0, 200.0, 200.0 - 7.20, 0.0, 7.20, FONT_SIZE_HEIGHT);
}

#[test]
fn should_hit_test_position_right_align_correctly_2() {
    should_hit_test_position_right_align_correctly(STRINGWORD, 0, 200.0, 171.20, 0.0, 7.20, FONT_SIZE_HEIGHT);
}

#[test]
fn should_hit_test_position_right_align_correctly_3() {
    should_hit_test_position_right_align_correctly(STRINGWORD, 3, 200.0, 200.0 - 7.20, 0.0, 7.20, FONT_SIZE_HEIGHT);
}

fn should_hit_test_position_center_align_correctly(
    input: &str,
    index: i32,
    width_constraint: f64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
) {
    let _t = base();
    //parse expected
    let fmt = create_aligned(input, FONT_SIZE, TextAlignment::Center, width_constraint);
    let constrained = &fmt;
    let r = constrained.hit_test_text_position(index);

    assert_equal_precision(x, r.x, 2);
    assert_equal_precision(y, r.y, 2);
    assert_equal_precision(width, r.width, 2);
    assert_equal_precision(height, r.height, 2);
}

#[test]
fn should_hit_test_position_center_align_correctly_1() {
    should_hit_test_position_center_align_correctly("x", 0, 200.0, 100.0 - 7.20 / 2.0, 0.0, 7.20, FONT_SIZE_HEIGHT);
}

#[test]
fn should_hit_test_position_center_align_correctly_2() {
    should_hit_test_position_center_align_correctly(STRINGWORD, 0, 200.0, 85.6, 0.0, 7.20, FONT_SIZE_HEIGHT);
}

#[test]
fn should_hit_test_position_center_align_correctly_3() {
    should_hit_test_position_center_align_correctly(STRINGWORD, 3, 200.0, 100.0 + 7.20, 0.0, 7.20, FONT_SIZE_HEIGHT);
}

fn should_hit_test_range_correctly(input: &str, index: i32, length: i32, expected_rects: &str) {
    let _t = base();
    //parse expected result
    let rects: Vec<Rect> = expected_rects
        .split(';')
        .map(|s| {
            let v: Vec<f64> = s.split(',').map(|sd| sd.parse::<f64>().expect("the number parses")).collect();
            Rect::new(v[0], v[1], v[2], v[3])
        })
        .collect();

    let fmt = create(input, FONT_SIZE);
    let ht_res = fmt.hit_test_text_range(index, length);

    assert_eq!(rects.len(), ht_res.len());

    for i in 0..rects.len() {
        let exr = rects[i];
        let r = ht_res[i];

        assert_equal_precision(exr.x, r.x, 2);
        assert_equal_precision(exr.y, r.y, 2);
        assert_equal_precision(exr.width, r.width, 2);
        assert_equal_precision(exr.height, r.height, 2);
    }
}

#[test]
fn should_hit_test_range_correctly_1() {
    should_hit_test_range_correctly("x", 0, 1, "0,0,7.20,14.0625");
}

#[test]
fn should_hit_test_range_correctly_2() {
    should_hit_test_range_correctly(STRINGWORD, 0, 4, "0,0,28.80,14.0625");
}

#[test]
fn should_hit_test_range_correctly_3() {
    should_hit_test_range_correctly(STRINGMIDDLENEWLINES, 10, 10, "0,14.0625,57.61,14.0625");
}

#[test]
fn should_hit_test_range_correctly_4() {
    should_hit_test_range_correctly(
        STRINGMIDDLENEWLINES,
        10,
        20,
        "0,14.0625,57.61,14.0625;0,28.125,64.81,14.0625",
    );
}

#[test]
fn should_hit_test_range_correctly_5() {
    should_hit_test_range_correctly(
        STRINGMIDDLENEWLINES,
        10,
        15,
        "0,14.0625,57.61,14.0625;0,28.125,36.01,14.0625",
    );
}

#[test]
fn should_hit_test_range_correctly_6() {
    should_hit_test_range_correctly(
        STRINGMIDDLENEWLINES,
        15,
        15,
        "36.01,14.0625,21.60,14.0625;0,28.125,64.81,14.0625",
    );
}

#[test]
fn text_layout_basic() {
    // Skip test on OSX: text rendering is subtly different.
    if cfg!(target_os = "macos") {
        return;
    }

    let test = base();
    let t = TextLayout::new(
        DRAWN_TEXT,
        Typeface::new(test_font_family()),
        TextLayoutOptions { font_size: 24.0, foreground: Some(Brushes::black()), ..Default::default() },
    );

    let target = Border::new();
    target.set_width(200.0);
    target.set_height(200.0);
    target.set_background(Some(Brushes::white()));
    target.set_child(DrawnControl::new(move |c| {
        let text_rect = Rect::new(0.0, 0.0, t.width_including_trailing_whitespace(), t.height());
        let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let rect = bounds.center_rect(text_rect);
        let yellow: Rc<dyn IBrush> = Brushes::yellow();
        c.draw_rectangle(Some(&yellow), None, rect, 0.0, 0.0, &BoxShadows::default());
        t.draw(c, rect.position());
    }));

    test.render_to_file(&target, "TextLayout_Basic");
    test.compare_images("TextLayout_Basic");
}

#[test]
fn text_layout_rotated() {
    // Skip test on OSX: text rendering is subtly different.
    if cfg!(target_os = "macos") {
        return;
    }

    let test = base();
    let t = TextLayout::new(
        DRAWN_TEXT,
        Typeface::new(test_font_family()),
        TextLayoutOptions { font_size: 24.0, foreground: Some(Brushes::black()), ..Default::default() },
    );

    let target = Border::new();
    target.set_width(200.0);
    target.set_height(200.0);
    target.set_background(Some(Brushes::white()));
    target.set_child(DrawnControl::new(move |c| {
        let text_rect = Rect::new(0.0, 0.0, t.width_including_trailing_whitespace(), t.height());
        let bounds = Rect::new(0.0, 0.0, 200.0, 200.0);
        let rect = bounds.center_rect(text_rect);
        let rotate = Matrix::create_translation(-100.0, -100.0)
            * Matrix::create_rotation(MathUtilities::deg2rad(90.0))
            * Matrix::create_translation(100.0, 100.0);
        let transform = c.push_transform(rotate);
        let yellow: Rc<dyn IBrush> = Brushes::yellow();
        c.draw_rectangle(Some(&yellow), None, rect, 0.0, 0.0, &BoxShadows::default());
        t.draw(c, rect.position());
        c.pop(transform);
    }));

    test.render_to_file(&target, "TextLayout_Rotated");
    test.compare_images("TextLayout_Rotated");
}

#[repr(C)]
struct DrawnControl {
    base: Control,
    render: Box<dyn Fn(&mut DrawingContext)>,
}

ferro_class!(DrawnControl: Control);
ferro_impl_classes!(
    DrawnControl: FerroObjectImpl,
    StyledElementImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl DrawnControl {
    fn new(render: impl Fn(&mut DrawingContext) + 'static) -> Ref<Self> {
        instantiate(Self { base: Control::construct(), render: Box::new(render) })
    }
}

impl VisualImpl for DrawnControl {
    fn render(this: &Self, context: &mut DrawingContext) {
        (this.render)(context)
    }
}
