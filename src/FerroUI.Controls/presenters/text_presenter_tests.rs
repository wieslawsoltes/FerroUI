//! The reference tests run on a mocked render interface; these run on the
//! test fonts (see `ferroui_base::media::text_formatting::testing`): every
//! glyph advances half an em and a line is 1.1 em high.

use crate::presenters::TextPresenter;
use crate::test_support::{test_scope, TestRoot};
use ferroui_base::media::text_formatting::LogicalDirection;
use ferroui_base::media::{
    Brushes, CharacterHit, DrawingContext, FontStretch, FontStyle, IBrush, PlatformDrawingContext, TextWrapping,
};
use ferroui_base::rendering::testing::{DrawingLog, MockDrawingContextImpl};
use ferroui_base::{Point, Rect, Ref, Size};
use std::cell::Cell;
use std::rc::Rc;

fn infinity() -> Size {
    Size::new(f64::INFINITY, f64::INFINITY)
}

fn presenter(text: &str) -> Ref<TextPresenter> {
    let presenter = TextPresenter::new();
    presenter.set_text(Some(text));
    presenter
}

fn layout_text(presenter: &TextPresenter) -> String {
    presenter
        .text_layout()
        .text_lines()
        .iter()
        .flat_map(|line| line.text_runs().iter().map(|run| run.text().to_string_lossy()).collect::<Vec<_>>())
        .collect()
}

#[test]
fn text_presenter_can_contain_null_with_password_char_set() {
    let _scope = test_scope();
    let target = TextPresenter::new();
    target.set_password_char('*');
    target.set_text(None);

    assert_eq!(target.text_layout().text_lines().len(), 1);
}

#[test]
fn text_presenter_can_contain_null_without_password_char_set() {
    let _scope = test_scope();
    let target = TextPresenter::new();
    target.set_text(None);

    assert_eq!(target.text_layout().text_lines().len(), 1);
}

#[test]
fn default_text_is_empty() {
    let _scope = test_scope();

    assert_eq!(TextPresenter::new().text().as_deref(), Some(""));
}

#[test]
fn text_presenter_replaces_formatted_text_with_password_char() {
    let _scope = test_scope();
    let target = presenter("Test");
    target.set_password_char('*');

    target.measure(infinity());

    assert_eq!(layout_text(&target), "****");

    target.set_reveal_password(true);

    assert_eq!(layout_text(&target), "Test");
}

#[test]
fn text_presenter_should_use_font_stretch_property() {
    for font_stretch in [
        FontStretch::Condensed,
        FontStretch::Expanded,
        FontStretch::Normal,
        FontStretch::ExtraCondensed,
        FontStretch::SemiCondensed,
        FontStretch::ExtraExpanded,
        FontStretch::SemiExpanded,
        FontStretch::UltraCondensed,
        FontStretch::UltraExpanded,
    ] {
        let _scope = test_scope();
        let presenter = presenter("test");
        presenter.set_font_stretch(font_stretch);

        let text_layout = presenter.text_layout();
        assert_eq!(text_layout.text_lines().len(), 1);
        let text_runs = text_layout.text_lines()[0].text_runs();
        assert_eq!(text_runs.len(), 1);
        let properties = text_runs[0].properties().expect("the run has properties");
        assert_eq!(properties.typeface().stretch(), font_stretch);
    }
}

#[test]
fn measure_and_arrange_should_use_width_including_trailing_whitespace_for_bounds() {
    let _scope = test_scope();
    let presenter = presenter("fy");
    presenter.set_font_style(FontStyle::Italic);
    presenter.set_font_size(48.0);
    presenter.set_use_layout_rounding(false);

    presenter.measure(infinity());

    let text_layout = presenter.text_layout();
    let expected_size = Size::new(text_layout.width_including_trailing_whitespace(), text_layout.height());

    assert_eq!(presenter.desired_size(), expected_size);
    // Two glyphs of half an em, one line of 1.1 em.
    assert_eq!(expected_size, Size::new(48.0, 52.8));

    presenter.arrange(Rect::from_position_size(Point::default(), presenter.desired_size()));

    assert_eq!(presenter.bounds(), Rect::from_position_size(Point::default(), expected_size));
}

#[test]
fn hide_caret_should_keep_the_text_layout() {
    let _scope = test_scope();
    let presenter = presenter("hello");

    presenter.measure(infinity());

    let text_layout = presenter.text_layout();

    presenter.hide_caret();

    // The caret blinks over the text rather than taking part in it, so
    // hiding it repaints; rebuilding the layout would reshape the text for
    // nothing, and would dispose a layout its callers may still be holding.
    assert!(Rc::ptr_eq(&text_layout, &presenter.text_layout()));
}

// The tests below are not part of the reference suite: they pin the caret,
// selection and pre-edit behaviour the text box relies on.

#[test]
fn text_positions_are_coerced_to_the_text() {
    let _scope = test_scope();
    let presenter = presenter("ab\r\ncd");

    presenter.set_caret_index(100);
    assert_eq!(presenter.caret_index(), 6);

    presenter.set_selection_start(-3);
    assert_eq!(presenter.selection_start(), 0);

    // Never between the two halves of a line break.
    presenter.set_selection_end(3);
    assert_eq!(presenter.selection_end(), 4);

    presenter.set_text(None);
    presenter.set_selection_end(2);
    assert_eq!(presenter.selection_end(), 0);
}

#[test]
fn caret_index_moves_the_caret_and_raises_caret_bounds_changed() {
    let _scope = test_scope();
    let presenter = presenter("hello");
    presenter.measure(infinity());

    let raised = Rc::new(Cell::new(0));
    let subscription = presenter.caret_bounds_changed({
        let raised = raised.clone();
        move || raised.set(raised.get() + 1)
    });

    presenter.set_caret_index(2);

    // Two glyphs of 6 at the default size of 12; a line is 13.2 high.
    assert_eq!(presenter.get_cursor_rectangle(), Rect::new(12.0, 0.0, 0.0, 13.2));
    assert_eq!(raised.get(), 1);

    // The same position again: the bounds do not change.
    presenter.move_caret_to_text_position(2, false);
    assert_eq!(raised.get(), 1);

    subscription.dispose();
    presenter.set_caret_index(3);
    assert_eq!(raised.get(), 1);

    let (p1, p2) = presenter.get_caret_points();
    assert_eq!(p1, Point::new(18.5, 0.5));
    assert_eq!(p2, Point::new(18.5, 13.5));
}

#[test]
fn move_caret_horizontal_walks_the_text_and_stops_at_its_ends() {
    let _scope = test_scope();
    let presenter = presenter("ab");
    presenter.measure(infinity());

    let next = presenter.get_next_character_hit(LogicalDirection::Forward);
    assert_eq!(next.first_character_index() + next.trailing_length(), 1);

    presenter.move_caret_horizontal(LogicalDirection::Forward);
    assert_eq!(presenter.caret_index(), 1);
    presenter.move_caret_horizontal(LogicalDirection::Forward);
    assert_eq!(presenter.caret_index(), 2);
    presenter.move_caret_horizontal(LogicalDirection::Forward);
    assert_eq!(presenter.caret_index(), 2);

    presenter.move_caret_horizontal(LogicalDirection::Backward);
    assert_eq!(presenter.caret_index(), 1);
    presenter.move_caret_horizontal(LogicalDirection::Backward);
    presenter.move_caret_horizontal(LogicalDirection::Backward);
    assert_eq!(presenter.caret_index(), 0);
}

#[test]
fn get_next_character_hit_is_default_without_text() {
    let _scope = test_scope();
    let presenter = TextPresenter::new();
    presenter.set_text(None);

    assert_eq!(presenter.get_next_character_hit(LogicalDirection::Forward), CharacterHit::default());
}

#[test]
fn move_caret_vertical_keeps_the_horizontal_position() {
    let _scope = test_scope();
    let presenter = presenter("abcd\nef\nghij");
    presenter.measure(infinity());

    presenter.move_caret_to_text_position(3, false);
    assert_eq!(presenter.get_cursor_rectangle().x, 18.0);

    presenter.move_caret_vertical(LogicalDirection::Forward);
    // The second line is shorter: the caret goes to its end.
    assert_eq!(presenter.caret_index(), 7);
    assert_eq!(presenter.get_cursor_rectangle().y, 13.2);

    presenter.move_caret_vertical(LogicalDirection::Forward);
    // The navigation position is remembered across the short line.
    assert_eq!(presenter.caret_index(), 11);

    // No line below the last one.
    presenter.move_caret_vertical(LogicalDirection::Forward);
    assert_eq!(presenter.caret_index(), 11);

    presenter.move_caret_vertical(LogicalDirection::Backward);
    presenter.move_caret_vertical(LogicalDirection::Backward);
    assert_eq!(presenter.caret_index(), 3);

    // No line above the first one.
    presenter.move_caret_vertical(LogicalDirection::Backward);
    assert_eq!(presenter.caret_index(), 3);
}

#[test]
fn move_caret_to_point_hits_the_nearest_character_edge() {
    let _scope = test_scope();
    let presenter = presenter("hello");
    presenter.measure(infinity());

    presenter.move_caret_to_point(Point::new(13.0, 5.0));
    assert_eq!(presenter.caret_index(), 2);

    presenter.move_caret_to_point(Point::new(17.0, 5.0));
    assert_eq!(presenter.caret_index(), 3);

    presenter.move_caret_to_point(Point::new(500.0, 5.0));
    assert_eq!(presenter.caret_index(), 5);
}

#[test]
fn preedit_text_is_inserted_at_the_caret() {
    let _scope = test_scope();
    let presenter = presenter("abcd");
    presenter.measure(infinity());
    presenter.set_caret_index(2);

    presenter.set_preedit_text(Some("XY"));

    assert_eq!(layout_text(&presenter), "abXYcd");

    presenter.measure(infinity());
    // The caret is at the end of the pre-edit text.
    assert_eq!(presenter.get_cursor_rectangle().x, 24.0);

    presenter.set_preedit_text_cursor_position(Some(1));
    presenter.measure(infinity());
    assert_eq!(presenter.get_cursor_rectangle().x, 18.0);

    // Changing the text ends the composition.
    presenter.set_text(Some("abcde"));
    assert_eq!(presenter.preedit_text(), None);
    assert_eq!(layout_text(&presenter), "abcde");
}

#[test]
fn preedit_text_alone_is_displayed_when_there_is_no_text() {
    let _scope = test_scope();
    let presenter = TextPresenter::new();

    presenter.set_preedit_text(Some("XY"));

    assert_eq!(layout_text(&presenter), "XY");
}

#[test]
fn selection_changes_recreate_the_text_layout() {
    let _scope = test_scope();
    let presenter = presenter("hello");
    presenter.measure(infinity());
    let text_layout = presenter.text_layout();

    presenter.set_selection_start(1);
    presenter.set_selection_end(3);
    presenter.measure(infinity());

    assert!(!Rc::ptr_eq(&text_layout, &presenter.text_layout()));
    // Without a selection foreground the selection does not split the run.
    assert_eq!(presenter.text_layout().text_lines()[0].text_runs().len(), 1);

    // A brush other than the foreground: runs of equal properties are merged.
    let brush: Rc<dyn IBrush> = Brushes::red();
    presenter.set_selection_foreground_brush(Some(brush));
    presenter.measure(infinity());

    assert_eq!(presenter.text_layout().text_lines()[0].text_runs().len(), 3);

    presenter.set_show_selection_highlight(false);
    presenter.measure(infinity());

    assert_eq!(presenter.text_layout().text_lines()[0].text_runs().len(), 1);
}

#[test]
fn arrange_with_a_different_width_recreates_the_text_layout() {
    let _scope = test_scope();
    let presenter = presenter("hello world");
    presenter.set_text_wrapping(TextWrapping::Wrap);

    presenter.measure(Size::new(200.0, 200.0));
    let text_layout = presenter.text_layout();
    assert_eq!(text_layout.text_lines().len(), 1);

    presenter.arrange(Rect::new(0.0, 0.0, 200.0, 200.0));
    assert!(Rc::ptr_eq(&text_layout, &presenter.text_layout()));

    presenter.measure(Size::new(40.0, 200.0));
    assert_eq!(presenter.text_layout().text_lines().len(), 2);
}

#[test]
fn caret_timer_blinks_while_the_caret_is_shown() {
    let _scope = test_scope();
    let presenter = presenter("hello");
    let root = TestRoot::with_child(presenter.clone());
    root.execute_initial_layout_pass();

    presenter.show_caret();
    presenter.hide_caret();
    presenter.show_caret();

    // Moving the caret restarts the blink without hiding the caret.
    presenter.set_caret_index(1);
    assert_eq!(presenter.caret_index(), 1);

    // No blinking at all with an interval of zero.
    presenter.set_caret_blink_interval(std::time::Duration::ZERO);
    presenter.show_caret();
    presenter.hide_caret();

    root.set_child(None);
}

fn render(presenter: &TextPresenter) -> Vec<String> {
    let log = DrawingLog::new();
    let mut platform_impl = MockDrawingContextImpl::new(log.clone());
    platform_impl.log_transforms = false;
    let mut core = PlatformDrawingContext::borrowed(&mut platform_impl);
    let mut context = DrawingContext::new(&mut core);
    presenter.render(&mut context);
    context.dispose();
    log.entries()
}

#[test]
fn render_draws_the_selection_then_the_text_then_the_caret() {
    let _scope = test_scope();
    let presenter = presenter("hello");
    let root = TestRoot::with_child(presenter.clone());
    root.execute_initial_layout_pass();

    // Nothing but the text: the caret is hidden.
    let text_only = render(&presenter);
    assert_eq!(text_only.len(), 1);
    assert!(text_only[0].starts_with("DrawGlyphRun"));

    presenter.set_caret_index(2);
    presenter.show_caret();
    root.execute_initial_layout_pass();

    let with_caret = render(&presenter);
    // A one pixel line through the pixel centres at the caret.
    assert_eq!(with_caret[1..], ["DrawLine Black@1 12.5, 0.5 12.5, 13.5"]);

    // A selection hides the caret and is filled behind the text.
    let brush: Rc<dyn IBrush> = Brushes::red();
    presenter.set_selection_brush(Some(brush));
    presenter.set_selection_start(1);
    presenter.set_selection_end(3);
    root.layout_manager().execute_layout_pass();

    let with_selection = render(&presenter);
    // "el": 6..18, snapped to whole pixels.
    assert_eq!(with_selection.len(), 2);
    assert_eq!(with_selection[0], "DrawRectangle Red none 6, 0, 12, 14 shadows=0");

    presenter.set_show_selection_highlight(false);
    root.layout_manager().execute_layout_pass();

    assert_eq!(render(&presenter).len(), text_only.len());

    root.set_child(None);
}

#[test]
fn caret_brush_is_the_caret_brush_or_the_inverted_background_or_black() {
    let _scope = test_scope();
    let presenter = presenter("hello");
    let root = TestRoot::with_child(presenter.clone());
    root.execute_initial_layout_pass();
    presenter.show_caret();

    // No caret brush and no background: black.
    let entries = render(&presenter);
    assert_eq!(entries.last().unwrap(), "DrawLine Black@1 0.5, 0.5 0.5, 13.5");

    // No caret brush and a solid background: the inverse of the background
    // (alpha is not carried over), drawn over the background and the text.
    let background: Rc<dyn IBrush> = Brushes::red();
    presenter.set_background(Some(background));
    let entries = render(&presenter);
    assert_eq!(entries.len(), 3);
    assert!(entries[0].starts_with("DrawRectangle Red none 0, 0, "), "{}", entries[0]);
    assert!(entries[1].starts_with("DrawGlyphRun"));
    assert_eq!(entries[2], "DrawLine Aqua@1 0.5, 0.5 0.5, 13.5");

    // The caret brush wins over the background.
    let caret_brush: Rc<dyn IBrush> = Brushes::blue();
    presenter.set_caret_brush(Some(caret_brush));
    let entries = render(&presenter);
    assert_eq!(entries[2], "DrawLine Blue@1 0.5, 0.5 0.5, 13.5");

    // A hidden caret is not drawn.
    presenter.hide_caret();
    assert_eq!(render(&presenter).len(), 2);

    root.set_child(None);
}

#[test]
#[should_panic(expected = "out of range")]
fn preedit_text_at_a_caret_outside_of_the_text_is_an_error() {
    let _scope = test_scope();
    let presenter = presenter("abc");
    presenter.set_caret_index(3);

    // The caret index is coerced when it is set, not when the text changes.
    presenter.set_text(Some("a"));
    assert_eq!(presenter.caret_index(), 3);

    presenter.set_preedit_text(Some("x"));

    // The reference throws from the substring before the caret.
    presenter.measure(infinity());
}
