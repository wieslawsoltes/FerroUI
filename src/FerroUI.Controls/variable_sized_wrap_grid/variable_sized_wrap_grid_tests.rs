//! Tests of `VariableSizedWrapGrid`: WinUI's integration tests
//! (`dxaml/test/native/external/controls/variablesizedwrapgrid/
//! VariableSizedWrapGridIntegrationTests.cpp` with `PanelsHelper.h`: eight 100 by 100 items in a
//! 300 by 300 panel) and the layout of the WinUI Gallery example.

use crate::{Border, VariableSizedWrapGrid};
use ferroui_base::input::{INavigableContainer, InputElement, NavigationDirection};
use ferroui_base::layout::{HorizontalAlignment, Orientation, VerticalAlignment};
use ferroui_base::{Point, Rect, Ref, Size};

fn item(width: f64, height: f64) -> Ref<Border> {
    let border = Border::new();
    border.set_width(width);
    border.set_height(height);
    border
}

/// `PanelsHelper::AddPanelWithContent` with `CreateDefaultPanelContent`.
fn panel(orientation: Orientation, items: &[Ref<Border>]) -> Ref<VariableSizedWrapGrid> {
    let panel = VariableSizedWrapGrid::new();
    panel.set_width(300.0);
    panel.set_height(300.0);
    panel.set_orientation(orientation);
    for item in items {
        panel.children().add(item.clone());
    }
    layout(&panel);
    panel
}

fn layout(panel: &VariableSizedWrapGrid) {
    panel.measure(Size::INFINITY);
    panel.arrange(Rect::from_size(panel.desired_size()));
}

fn default_items() -> Vec<Ref<Border>> {
    (0..8).map(|_| item(100.0, 100.0)).collect()
}

fn positions(panel: &VariableSizedWrapGrid, count: usize) -> Vec<Point> {
    (0..count)
        .map(|index| panel.children().get(index).bounds().position())
        .collect()
}

#[test]
fn has_the_winui_defaults() {
    let panel = VariableSizedWrapGrid::new();
    assert!(panel.item_width().is_nan());
    assert!(panel.item_height().is_nan());
    assert_eq!(Orientation::Vertical, panel.orientation());
    assert_eq!(HorizontalAlignment::Left, panel.horizontal_children_alignment());
    assert_eq!(VerticalAlignment::Top, panel.vertical_children_alignment());
    assert_eq!(-1, panel.maximum_rows_or_columns());
    let child = Border::new();
    assert_eq!(1, VariableSizedWrapGrid::get_row_span(&child));
    assert_eq!(1, VariableSizedWrapGrid::get_column_span(&child));
}

/// `CanWrapItemsHorizontally`.
#[test]
fn wraps_items_horizontally() {
    let panel = panel(Orientation::Horizontal, &default_items());
    let expected: Vec<Point> = (0..8)
        .map(|i| Point::new(100.0 * (i % 3) as f64, 100.0 * (i / 3) as f64))
        .collect();
    assert_eq!(expected, positions(&panel, 8));
}

/// `CanWrapItemsVertically`.
#[test]
fn wraps_items_vertically() {
    let panel = panel(Orientation::Vertical, &default_items());
    let expected: Vec<Point> = (0..8)
        .map(|i| Point::new(100.0 * (i / 3) as f64, 100.0 * (i % 3) as f64))
        .collect();
    assert_eq!(expected, positions(&panel, 8));
}

/// `CanChangeRowAndColumnSpans`: the first item spans two columns and the third two rows; the
/// 100 by 100 items are centred in their larger tiles, and the eighth does not fit (it is not
/// arranged).
#[test]
fn places_row_and_column_spans() {
    let items = default_items();
    VariableSizedWrapGrid::set_column_span(&items[0], 2);
    VariableSizedWrapGrid::set_row_span(&items[2], 2);
    let panel = panel(Orientation::Horizontal, &items);
    let expected = vec![
        Point::new(50.0, 0.0),
        Point::new(200.0, 0.0),
        Point::new(0.0, 150.0),
        Point::new(100.0, 100.0),
        Point::new(200.0, 100.0),
        Point::new(100.0, 200.0),
        Point::new(200.0, 200.0),
    ];
    assert_eq!(expected, positions(&panel, 7));
    assert_eq!(Rect::default(), panel.children().get(7).bounds());
}

/// The gallery's example: 44 pixel cells, three per column, a 2 row, a 2 column and a 2 by 2
/// tile; and the same items in rows of three cells when the orientation is horizontal.
#[test]
fn lays_out_the_gallery_example() {
    let panel = VariableSizedWrapGrid::new();
    panel.set_width(400.0);
    panel.set_item_width(44.0);
    panel.set_item_height(44.0);
    panel.set_maximum_rows_or_columns(3);
    let red = Border::new();
    let blue = Border::new();
    blue.set_height(80.0);
    VariableSizedWrapGrid::set_row_span(&blue, 2);
    let green = Border::new();
    green.set_width(80.0);
    VariableSizedWrapGrid::set_column_span(&green, 2);
    let yellow = item(80.0, 80.0);
    VariableSizedWrapGrid::set_row_span(&yellow, 2);
    VariableSizedWrapGrid::set_column_span(&yellow, 2);
    for child in [&red, &blue, &green, &yellow] {
        panel.children().add(child.clone());
    }
    panel.measure(Size::new(400.0, f64::INFINITY));
    // Three columns of three cells: red and blue fill the first, green the second and third
    // rows' first cells, yellow the 2 by 2 tile below green's start.
    assert_eq!(Size::new(400.0, 132.0), panel.desired_size());
    panel.arrange(Rect::new(0.0, 0.0, 400.0, 132.0));
    assert_eq!(Rect::new(0.0, 0.0, 44.0, 44.0), red.bounds());
    assert_eq!(Rect::new(0.0, 48.0, 44.0, 80.0), blue.bounds());
    assert_eq!(Rect::new(48.0, 0.0, 80.0, 44.0), green.bounds());
    assert_eq!(Rect::new(48.0, 48.0, 80.0, 80.0), yellow.bounds());

    panel.set_orientation(Orientation::Horizontal);
    panel.measure(Size::new(400.0, f64::INFINITY));
    panel.arrange(Rect::from_size(panel.desired_size()));
    assert_eq!(Rect::new(0.0, 0.0, 44.0, 44.0), red.bounds());
    assert_eq!(Rect::new(44.0, 4.0, 44.0, 80.0), blue.bounds());
    // Blue holds the second cell of the second row, so green's two cells start the third row.
    assert_eq!(Rect::new(4.0, 88.0, 80.0, 44.0), green.bounds());
}

/// A child's tile that is larger than a line keeps its length at the start of a line.
#[test]
fn keeps_a_tile_longer_than_a_line_at_its_start() {
    let panel = VariableSizedWrapGrid::new();
    panel.set_item_width(10.0);
    panel.set_item_height(10.0);
    panel.set_maximum_rows_or_columns(2);
    let tall = Border::new();
    VariableSizedWrapGrid::set_row_span(&tall, 4);
    panel.children().add(tall.clone());
    panel.measure(Size::new(100.0, 100.0));
    panel.arrange(Rect::new(0.0, 0.0, 100.0, 100.0));
    assert_eq!(Rect::new(0.0, 0.0, 10.0, 40.0), tall.bounds());
}

#[test]
fn aligns_the_cells_with_the_children_alignments() {
    let items = default_items();
    let panel = VariableSizedWrapGrid::new();
    panel.set_maximum_rows_or_columns(2);
    for item in items.iter().take(4) {
        panel.children().add(item.clone());
    }
    panel.set_horizontal_children_alignment(HorizontalAlignment::Center);
    panel.set_vertical_children_alignment(VerticalAlignment::Bottom);
    panel.measure(Size::new(400.0, 400.0));
    panel.arrange(Rect::new(0.0, 0.0, 400.0, 400.0));
    // Two columns of two: 100 pixels on either side, 200 above.
    assert_eq!(Point::new(100.0, 200.0), items[0].bounds().position());
    assert_eq!(Point::new(200.0, 300.0), items[3].bounds().position());

    panel.set_horizontal_children_alignment(HorizontalAlignment::Stretch);
    panel.set_vertical_children_alignment(VerticalAlignment::Top);
    panel.measure(Size::new(500.0, 400.0));
    panel.arrange(Rect::new(0.0, 0.0, 500.0, 400.0));
    // (500 - 200) / 3 between the columns.
    assert_eq!(Point::new(100.0, 0.0), items[0].bounds().position());
    assert_eq!(Point::new(300.0, 100.0), items[3].bounds().position());
}

#[test]
fn an_empty_panel_measures_empty() {
    let panel = VariableSizedWrapGrid::new();
    panel.measure(Size::INFINITY);
    assert_eq!(Size::new(0.0, 0.0), panel.desired_size());
}

#[test]
fn navigates_over_the_tiles() {
    let items = default_items();
    VariableSizedWrapGrid::set_column_span(&items[0], 2);
    VariableSizedWrapGrid::set_row_span(&items[2], 2);
    let panel = panel(Orientation::Horizontal, &items);
    let input = |index: usize| -> Ref<InputElement> { items[index].clone().upcast() };
    let to = |direction, index: usize| -> Option<usize> {
        panel
            .get_control(direction, Some(&input(index)), false)
            .map(|target| (0..8).find(|candidate| input(*candidate) == target).unwrap())
    };
    // The first row: the wide first item, then the second.
    assert_eq!(Some(1), to(NavigationDirection::Right, 0));
    assert_eq!(Some(0), to(NavigationDirection::Left, 1));
    assert_eq!(None, to(NavigationDirection::Up, 0));
    // The wide item was entered from the right at its second cell: down goes below that cell.
    assert_eq!(Some(3), to(NavigationDirection::Down, 0));
    assert_eq!(Some(3), to(NavigationDirection::Right, 2));
    assert_eq!(Some(4), to(NavigationDirection::Down, 1));
    assert_eq!(Some(0), to(NavigationDirection::First, 4));
    // From its first cell, down reaches the wide third item below it.
    assert_eq!(Some(2), to(NavigationDirection::Down, 0));
    assert_eq!(Some(6), to(NavigationDirection::Last, 0));
    assert_eq!(Some(5), to(NavigationDirection::Next, 4));
    assert_eq!(Some(3), to(NavigationDirection::Previous, 4));
}
