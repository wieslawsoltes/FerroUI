//! The real application of the sample, headless (`sample_testing::Shell`): the application
//! populates itself from `App.xaml`, the main window is created and shown as the desktop
//! lifetime shows it, and its frames are rendered through the compositor with Skia.
//!
//! The tests look at what the user sees and does: the line the line control formats and
//! draws, the text, the font size and the font features of the boxes of the window reaching
//! the line, the mouse over the line selecting the character hit under it (which the
//! selection adorner draws), and the rows of the two lists.

use crate::{App, GridRow, InteractiveLineControl, MainWindow, SelectionAdorner, TextRunTag, SAMPLE};
use ferroui_base::media::text_formatting::TextRunProperties;
use ferroui_base::media::{CharacterHit, FontFeature};
use ferroui_base::{AnyValue, BoxedValue, Point, Rect, Ref};
use ferroui_controls::primitives::AdornerLayer;
use ferroui_controls::{Border, Control, Grid, ListBox, SelectionMode, TextBlock, TextBox, ToggleSwitch};
use sample_testing::{assert_accepted, Shell};
use std::cell::Cell;
use std::rc::Rc;

/// The size of the window of the test: the size `MainWindow.xaml` gives the window.
const SIZE: (f64, f64) = (700.0, 700.0);

/// The background of the line (`Background="BlanchedAlmond"` of the line control).
const BLANCHED_ALMOND: [u8; 4] = [255, 235, 205, 255];

/// The binding errors the sample is accepted to report (reports the upstream sample makes
/// too): none.
const ACCEPTED: &[(&str, &str, usize)] = &[];

/// The report of the open gap T003 (`GAPS.md`), which the upstream sample does not make: the
/// font family of the line is bound to the selected value of the font box, which is null
/// until a font is chosen. A null is a value of the property upstream; here the property is
/// never null and the binding reports the null it cannot convert. The line is formatted with
/// the default font family in both.
pub(super) const GAP_T003: (&str, &str, usize) = (
    "MainWindow",
    "InteractiveLineControl.FontFamily <- #_font.SelectedValue: Could not convert '(null)' (null) to 'ferroui_base::media::font_family::FontFamily'.",
    1,
);

/// The accepted reports and the report of the open gap.
fn expected_reports() -> Vec<(&'static str, &'static str, usize)> {
    ACCEPTED.iter().copied().chain([GAP_T003]).collect()
}

pub(super) fn start() -> (Shell, Ref<MainWindow>) {
    let shell = Shell::start(&SAMPLE, SIZE.0, SIZE.1, || App::new().upcast());
    let window = MainWindow::new();
    window.show();
    shell.settle();
    (shell, window)
}

pub(super) fn reports(shell: &Shell, place: &str) -> Vec<(String, String, usize)> {
    shell.take_binding_reports().into_iter().map(|(report, count)| (place.to_string(), report.line(), count)).collect()
}

pub(super) fn rendering(window: &MainWindow) -> Ref<InteractiveLineControl> {
    window.get_control::<InteractiveLineControl>("_rendering")
}

/// The selection adorner of the line control.
fn selection_adorner(window: &MainWindow) -> Ref<SelectionAdorner> {
    AdornerLayer::get_adorner(&rendering(window))
        .and_then(|adorner| adorner.cast::<SelectionAdorner>())
        .expect("the window gives the line control its selection adorner")
}

/// The items of a list box as the controls they are.
fn rows(list: &ListBox) -> Vec<Ref<Control>> {
    (0..list.item_count())
        .map(|index| {
            let item = list.items().view().get_at(index as usize).expect("an item of the list");
            Control::from_boxed(&item).expect("the items of the lists of the window are controls")
        })
        .collect()
}

/// The texts of the cells of a row; an empty text for a cell that is not a text block.
fn cells(row: &GridRow) -> Vec<String> {
    row.children()
        .to_vec()
        .iter()
        .map(|child| child.cast::<TextBlock>().and_then(|text_block| text_block.text()).unwrap_or_default())
        .collect()
}

fn tag<T: Clone + 'static>(control: &Control) -> Option<T> {
    let tag: BoxedValue = control.tag()?;
    let tag: &dyn AnyValue = &*tag;
    tag.downcast_ref::<T>().cloned()
}

/// The rectangle of the line (the line render bounds of the line control) in the window.
fn line_rect(window: &MainWindow) -> Rect {
    let rendering = rendering(window);
    let control = Shell::bounds_of(window, &rendering);
    let line = rendering.line_render_bounds();
    Rect::new(control.x + line.x, control.y + line.y, line.width, line.height)
}

fn frame_rect(rect: Rect) -> (f64, f64, f64, f64) {
    (rect.x, rect.y, rect.x + rect.width, rect.y + rect.height)
}

#[test]
fn the_line_control_formats_and_draws_the_line() {
    let (shell, window) = start();
    let rendering = rendering(&window);

    // The bindings of the document: the text of the text box, the size of the size box and
    // the features of the features box.
    assert_eq!(rendering.text().as_deref(), Some("Hello!"));
    assert_eq!(rendering.font_size(), 64.0, "the text of the size box is the font size of the line");
    assert_eq!(
        rendering.font_features().map(|features| features.len()),
        Some(4),
        "the text of the features box is the four font features of the line"
    );
    assert_eq!(
        TextRunProperties::font_rendering_em_size(&*rendering.text_run_properties()),
        64.0,
        "the runs of the line are formatted with the font size"
    );

    let text_line = rendering.text_line().expect("the control formats its text as a line");
    assert!(text_line.length() >= 6, "the line holds the six characters of the text");
    assert!(text_line.width_including_trailing_whitespace() > 0.0, "the line has a width");
    assert!(text_line.height() >= 64.0, "the line is as high as its font size at least");
    assert_eq!(text_line.get_distance_from_character_hit(CharacterHit::new(0)), 0.0);
    assert!(
        text_line.get_distance_from_character_hit(CharacterHit::new(5))
            > text_line.get_distance_from_character_hit(CharacterHit::new(1)),
        "the distances of the characters grow along the line"
    );

    // The line was drawn: the control recorded where, and the frame has the background of
    // the line with the glyphs and the strokes over it.
    let line = rendering.line_render_bounds();
    assert!(line.width > 0.0 && line.height > 0.0, "the control drew the line: {line:?}");
    assert_eq!(line.width, text_line.width_including_trailing_whitespace());
    assert!(shell.frames(0) > 0, "the compositor drew the window");
    let frame = shell.last_frame(0);
    let rect = frame_rect(line_rect(&window));
    assert!(frame.count(rect, BLANCHED_ALMOND, 2) > 0, "the background of the line is drawn");
    assert!(frame.count(rect, [0, 0, 0, 255], 8) > 0, "the glyphs of the line are drawn in black");
    assert!(frame.colors(rect) > 3, "the line is drawn with its strokes");

    // The control measures as the line and its ink, and draws the hit stops and the
    // distances below the line.
    let control = Shell::frame_rect_of(&window, &rendering);
    assert!(rendering.bounds().width >= line.width, "the control is as wide as its line at least");
    let below = (control.0, line_rect(&window).bottom() + 1.0, control.2, SIZE.1);
    assert!(frame.colors(below) > 1, "the caret stops and the distances are drawn below the line");

    assert_accepted(&reports(&shell, "MainWindow"), &expected_reports());

    window.close();
    drop(shell);
}

#[test]
fn the_boxes_of_the_window_change_what_the_control_formats_and_draws() {
    let (shell, window) = start();
    let rendering = rendering(&window);
    let changes = Rc::new(Cell::new(0));
    let subscription = {
        let changes = changes.clone();
        rendering.text_line_changed(move || changes.set(changes.get() + 1))
    };

    let before = shell.last_frame(0);
    let width = rendering.text_line().expect("the line").width_including_trailing_whitespace();
    let height = rendering.text_line().expect("the line").height();

    // The text.
    window.get_control::<TextBox>("_text").set_text(Some("Hello, text!"));
    shell.settle();
    assert_eq!(rendering.text().as_deref(), Some("Hello, text!"));
    assert!(changes.get() > 0, "the control says that its line changed");
    let longer = rendering.text_line().expect("the line").width_including_trailing_whitespace();
    assert!(longer > width, "the line of a longer text is longer: {longer} and {width}");
    assert_eq!(rendering.line_render_bounds().width, longer, "the control drew the new line");
    let after = shell.last_frame(0);
    assert!(before.difference(&after, (0.0, 0.0, SIZE.0, SIZE.1)) > 0, "the window shows the new line");

    // The font size.
    changes.set(0);
    window.get_control::<TextBox>("_size").set_text(Some("32"));
    shell.settle();
    assert_eq!(rendering.font_size(), 32.0);
    assert!(changes.get() > 0, "the control says that its line changed");
    let smaller = rendering.text_line().expect("the line").height();
    assert!(smaller < height, "the line of a smaller font is lower: {smaller} and {height}");
    assert_eq!(rendering.line_render_bounds().height, smaller, "the control drew the new line");

    // A font feature.
    changes.set(0);
    window.get_control::<TextBox>("_features").set_text(Some("-kern"));
    shell.settle();
    let features = rendering.font_features().expect("the features of the line");
    assert_eq!(features.len(), 1, "the text of the features box is one font feature");
    assert_eq!(features.get(0), FontFeature::parse("-kern"));
    assert!(changes.get() > 0, "the control says that its line changed");
    let run_properties = rendering.text_run_properties();
    assert_eq!(
        TextRunProperties::font_features(&*run_properties).map(|features| features.len()),
        Some(1),
        "the runs of the line are formatted with the font feature"
    );

    subscription.dispose();
    assert_accepted(&reports(&shell, "MainWindow"), &expected_reports());

    window.close();
    drop(shell);
}

#[test]
fn the_mouse_over_the_line_selects_the_character_hit_under_it() {
    let (shell, window) = start();
    let rendering = rendering(&window);
    let adorner = selection_adorner(&window);
    let hits = window.get_control::<ListBox>("_hits");
    let coordinates = window.get_control::<TextBlock>("_coordinates");
    let hit = window.get_control::<TextBlock>("_hit");

    assert_eq!(hits.selected_index(), -1, "no character hit is selected before the mouse is over the line");
    assert!(adorner.rectangles().is_none_or(|rectangles| rectangles.is_empty()), "the adorner has nothing to draw");
    let before = shell.last_frame(0);

    // The mouse over the leading half of the third character of the line.
    let text_line = rendering.text_line().expect("the line");
    let start = text_line.get_distance_from_character_hit(CharacterHit::new(2));
    let end = text_line.get_distance_from_character_hit(CharacterHit::new(3));
    let x = start + (end - start) / 4.0;
    let line = line_rect(&window);
    shell.pointer_move(0, Point::new(line.x + x, line.y + line.height / 2.0));
    shell.settle();

    let shown = coordinates.text().unwrap_or_default();
    assert!(shown.contains(", "), "the window shows the position of the mouse in the line control: {shown:?}");
    let shown = hit.text().unwrap_or_default();
    assert_eq!(shown, "2 (2+0)", "the window shows the text position under the mouse");
    assert_eq!(hits.selected_index(), 3, "the row of the character hit is selected (after the header)");

    // The selection is one rectangle: the character under the mouse.
    let rectangles = adorner.rectangles().expect("the rectangles of the selection");
    assert_eq!(rectangles.len(), 1, "one character hit is selected: {rectangles:?}");
    let expected = rendering.text_layout().hit_test_text_position(2);
    assert_eq!(rectangles[0], expected);
    assert!(rectangles[0].width > 0.0, "the rectangle of a character has a width");
    assert_eq!(
        (adorner.transform().m31, adorner.transform().m32),
        (rendering.line_render_bounds().x, rendering.line_render_bounds().y),
        "the adorner draws in the coordinates of the line"
    );

    // The adorner drew it over the line.
    let after = shell.last_frame(0);
    assert!(before.difference(&after, frame_rect(line)) > 0, "the selection is drawn over the line");

    assert_accepted(&reports(&shell, "MainWindow"), &expected_reports());

    window.close();
    drop(shell);
}

#[test]
fn a_range_of_character_hits_is_one_rectangle() {
    let (shell, window) = start();
    let rendering = rendering(&window);
    let adorner = selection_adorner(&window);
    let hits = window.get_control::<ListBox>("_hits");

    assert_eq!(hits.selection_mode(), SelectionMode::SINGLE, "a list box selects one item");
    window.get_control::<ToggleSwitch>("_hitRangeToggle").set_is_checked(Some(true));
    shell.settle();
    assert_eq!(hits.selection_mode(), SelectionMode::MULTIPLE, "the switch makes the list select a range");

    // The rows of the characters 1 to 3 (the first row is the header).
    hits.selection().select_range(2, 4);
    shell.settle();

    let rectangles = adorner.rectangles().expect("the rectangles of the selection");
    assert_eq!(rectangles, rendering.text_layout().hit_test_text_range(1, 3));
    assert_eq!(rectangles.len(), 1, "a continuous range of one run is one rectangle: {rectangles:?}");

    window.close();
    drop(shell);
}

#[test]
fn the_rows_of_the_lists_show_the_metrics_of_the_line() {
    let (shell, window) = start();
    let rendering = rendering(&window);
    let text_line = rendering.text_line().expect("the line");
    let length = text_line.length();

    // The character hits: the header and one row per character.
    let hits = window.get_control::<ListBox>("_hits");
    let hit_rows = rows(&hits);
    assert_eq!(hit_rows.len() as i32, 1 + length, "the header and a row for every character of the line");

    let header = hit_rows[0].cast::<Border>().and_then(|border| border.child()).and_then(|child| child.cast::<GridRow>());
    let header = header.expect("the header of the list is a row in a border");
    assert_eq!(
        cells(&header),
        ["", "Backspace Hit", "Previous Hit", "Index", "Next Hit", "Codepoint", "Character", "Distance"]
    );
    assert_eq!(header.column_definitions().count(), 8, "a row has a column for every child");
    assert_eq!(header.column_definitions().get(7).shared_size_group().as_deref(), Some("c7"));

    let text: Vec<u16> = "Hello!".encode_utf16().collect();
    for (index, row) in hit_rows.iter().skip(1).enumerate().take(text.len()) {
        let index = index as i32;
        let row = row.cast::<GridRow>().expect("the row of a character hit");
        let hit = CharacterHit::new(index);
        let previous = text_line.get_previous_caret_character_hit(hit);
        let next = text_line.get_next_caret_character_hit(hit);
        let backspace = text_line.get_backspace_caret_character_hit(hit);
        assert_eq!(
            cells(&row),
            [
                String::new(),
                format!("{}+{}", backspace.first_character_index(), backspace.trailing_length()),
                format!("{}+{}", previous.first_character_index(), previous.trailing_length()),
                index.to_string(),
                format!("{}+{}", next.first_character_index(), next.trailing_length()),
                format!("{:04X}", text[index as usize]),
                String::from_utf16_lossy(&text[index as usize..index as usize + 1]),
                text_line.get_distance_from_character_hit(hit).to_string(),
            ],
            "the row of the character {index}"
        );
        assert_eq!(tag::<i32>(&row), Some(index), "the row is tagged with the index of its character");
        assert_eq!(row.column_spacing(), 10.0);
        assert_eq!(row.column_definitions().count(), 8);
        for (column, child) in row.children().to_vec().iter().enumerate() {
            assert_eq!(Grid::get_column(child), column as i32, "every child of a row is in a column of its own");
        }
    }

    // The shaped buffers: the header, a row that names each run and a row per glyph.
    let buffer = window.get_control::<ListBox>("_buffer");
    let buffer_rows = rows(&buffer);
    let header = buffer_rows[0].cast::<Border>().and_then(|border| border.child()).and_then(|child| child.cast::<GridRow>());
    assert_eq!(
        cells(&header.expect("the header of the list is a row in a border")),
        ["", "Index", "Characters", "Codepoints", "Glyph", "Glyph ID", "Advance", "Offset", "Ink Bounds"]
    );

    let run = buffer_rows[1].cast::<TextBlock>().expect("the row that names the first run of the line");
    let name = run.text().unwrap_or_default();
    assert!(name.starts_with("ShapedTextRun: Bidi = 0, Font = "), "the first run of the line is a shaped run: {name:?}");
    assert!(tag::<TextRunTag>(&run).is_some(), "the row is tagged with its run");

    let glyphs: Vec<Ref<Border>> = buffer_rows.iter().filter_map(|row| row.cast::<Border>()).skip(1).collect();
    assert_eq!(glyphs.len(), text.len(), "a row for every glyph of the text");
    let mut advances = 0.0;
    for (index, glyph) in glyphs.iter().enumerate() {
        let row = glyph.child().and_then(|child| child.cast::<GridRow>()).expect("the row of a glyph");
        let cells = cells(&row);
        assert_eq!(cells.len(), 9, "the cells of the row of a glyph: {cells:?}");
        assert_eq!(cells[1], index.to_string(), "the cluster of the glyph");
        assert_eq!(cells[2], String::from_utf16_lossy(&text[index..index + 1]), "the characters of the cluster");
        assert_eq!(cells[3], format!("{:04X}", text[index]), "the code points of the cluster");
        assert!(cells[5].parse::<u16>().is_ok_and(|glyph_index| glyph_index > 0), "the glyph: {cells:?}");
        let advance = cells[6].parse::<f64>().expect("the advance of the glyph");
        assert!(advance > 0.0, "the advance of the glyph: {cells:?}");
        advances += advance;
        assert!(glyph.background().is_some(), "the rows of a cluster have a background");
        assert!(tag::<Rect>(glyph).is_some(), "the row is tagged with the ink bounds of its glyph");
    }
    let width = text_line.width_including_trailing_whitespace();
    assert!((advances - width).abs() < 0.01, "the advances of the glyphs are the width of the line: {advances} and {width}");

    // The list of the shaped buffers is on the tab that is shown.
    let frame = shell.last_frame(0);
    assert!(frame.colors(Shell::frame_rect_of(&window, &buffer)) > 2, "the rows of the shaped buffer are drawn");

    assert_accepted(&reports(&shell, "MainWindow"), &expected_reports());

    window.close();
    drop(shell);
}

#[test]
fn selecting_glyph_rows_draws_their_ink_bounds() {
    let (shell, window) = start();
    let adorner = selection_adorner(&window);
    let buffer = window.get_control::<ListBox>("_buffer");

    // The first glyph row: after the header and the row that names the run.
    let bounds = tag::<Rect>(&rows(&buffer)[2]).expect("the ink bounds of the first glyph");
    buffer.set_selected_index(2);
    shell.settle();
    assert_eq!(adorner.rectangles(), Some(vec![bounds]), "the adorner draws the ink bounds of the selected glyph");

    // The row that names the run has no rectangle.
    buffer.set_selected_index(1);
    shell.settle();
    assert_eq!(adorner.rectangles(), Some(Vec::new()));

    window.close();
    drop(shell);
}
