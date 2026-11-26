//! Tests of the groups of a grid view (#1424): the headers of the `ItemsWrapGrid` of a grouped
//! source, each group from a new line, the inline and the parallel headers, the padding, the
//! hidden empty groups, and the check boxes of the multiple selection mode.

use super::list_view_tests::list_view_template;
use super::*;
use crate::test_support::{boxed_str, string_of, test_scope, TestRoot};
use crate::ItemsSource;
use ferroui_base::layout::{Layoutable, Orientation};
use ferroui_base::styling::{ControlTheme, Setter};
use ferroui_base::{BoxedValue, Rect, Ref, Thickness};
use std::rc::Rc;

const CELL: f64 = 50.0;
const HEADER_WIDTH: f64 = 40.0;
const HEADER_HEIGHT: f64 = 30.0;

fn groups(counts: &[usize]) -> Rc<GroupedItemsView> {
    let groups = counts
        .iter()
        .enumerate()
        .map(|(group, count)| {
            let items = (0..*count).map(|index| boxed_str(&format!("G{group}-{index}")));
            CollectionViewGroup::new(boxed_str(&format!("G{group}")), ItemsSource::from_items(items))
        })
        .collect();
    GroupedItemsView::new(groups)
}

struct Grid {
    grid: Ref<GridView>,
    panel: Ref<ItemsWrapGrid>,
    root: Ref<TestRoot>,
}

impl Grid {
    /// A grid 230 wide: four cells of 50 in a line.
    fn new(counts: &[usize], configure: impl Fn(&ItemsWrapGrid, &GroupStyle)) -> Self {
        let grid = GridView::new();
        grid.set_template(list_view_template());
        grid.set_width(230.0);
        grid.set_height(400.0);
        let theme = ControlTheme::with_target_type(GridViewHeaderItem::TYPE);
        theme.setters().add(Setter::new(Layoutable::width_property(), HEADER_WIDTH));
        theme.setters().add(Setter::new(Layoutable::height_property(), HEADER_HEIGHT));
        let style = GroupStyle::new();
        style.set_header_container_style(Some(theme));
        grid.group_style().add(style.clone());
        grid.set_items_source(Some(ItemsSource::new(groups(counts))));
        let root = TestRoot::with_child(grid.clone());
        root.layout_manager().execute_initial_layout_pass();
        let panel = grid.items_panel_root().and_then(|panel| panel.cast::<ItemsWrapGrid>()).expect("a wrap grid");
        panel.set_item_width(CELL);
        panel.set_item_height(CELL);
        configure(&panel, &style);
        root.layout_manager().execute_layout_pass();
        Self { grid, panel, root }
    }

    /// The visible headers, by their text, with their bounds in the panel.
    fn headers(&self) -> Vec<(String, Rect)> {
        self.panel
            .group_headers()
            .into_iter()
            .filter(|header| header.is_visible())
            .map(|header| (text(&header.content()), header.bounds()))
            .collect()
    }

    fn item(&self, index: i32) -> Rect {
        self.grid.container_from_index(index).expect("a container").bounds()
    }
}

fn text(value: &Option<BoxedValue>) -> String {
    value.as_ref().and_then(string_of).unwrap_or_default()
}

/// Top in rows (WinUI's inline headers): each group starts a new row under its header.
#[test]
fn each_group_starts_a_new_row_under_its_header() {
    let _scope = test_scope();
    let grid = Grid::new(&[5, 2], |_, _| {});
    let headers = grid.headers();
    assert_eq!(headers.len(), 2);
    assert_eq!(headers[0], ("G0".to_string(), Rect::new(0.0, 0.0, HEADER_WIDTH, HEADER_HEIGHT)));
    // Four items in the first row, the fifth in the second.
    assert_eq!(grid.item(0), Rect::new(0.0, 30.0, CELL, CELL));
    assert_eq!(grid.item(3), Rect::new(150.0, 30.0, CELL, CELL));
    assert_eq!(grid.item(4), Rect::new(0.0, 80.0, CELL, CELL));
    // The second group under its header, after the two rows of the first.
    assert_eq!(headers[1].1, Rect::new(0.0, 130.0, HEADER_WIDTH, HEADER_HEIGHT));
    assert_eq!(grid.item(5), Rect::new(0.0, 160.0, CELL, CELL));
    assert_eq!(grid.panel.desired_size(), ferroui_base::Size::new(200.0, 210.0));
    // The headers are logical children of the grid, not children of the panel.
    let header = grid.panel.group_headers()[0].clone();
    assert!(header.parent().is_some_and(|parent| parent == grid.grid.clone().upcast::<ferroui_base::StyledElement>()));
    assert_eq!(grid.panel.children().count(), 7);
}

/// Left in rows (WinUI's parallel headers): the header beside its group, the items moved by it.
#[test]
fn a_header_on_the_left_is_beside_its_group() {
    let _scope = test_scope();
    let grid = Grid::new(&[5, 1], |panel, _| panel.set_group_header_placement(GroupHeaderPlacement::Left));
    let headers = grid.headers();
    assert_eq!(headers[0].1, Rect::new(0.0, 0.0, HEADER_WIDTH, HEADER_HEIGHT));
    // 230 - 40 leaves three cells in a row.
    assert_eq!(grid.item(0), Rect::new(40.0, 0.0, CELL, CELL));
    assert_eq!(grid.item(2), Rect::new(140.0, 0.0, CELL, CELL));
    assert_eq!(grid.item(3), Rect::new(40.0, 50.0, CELL, CELL));
    assert_eq!(headers[1].1.y, 100.0);
    assert_eq!(grid.item(5), Rect::new(40.0, 100.0, CELL, CELL));
}

/// Columns: Left is inline (before the items, along the lines), Top parallel (above them).
#[test]
fn in_columns_the_groups_follow_each_other_from_left_to_right() {
    let _scope = test_scope();
    let grid = Grid::new(&[3, 1], |panel, _| {
        panel.set_orientation(Orientation::Vertical);
        panel.set_group_header_placement(GroupHeaderPlacement::Left);
    });
    let headers = grid.headers();
    assert_eq!(headers[0].1, Rect::new(0.0, 0.0, HEADER_WIDTH, HEADER_HEIGHT));
    assert_eq!(grid.item(0), Rect::new(40.0, 0.0, CELL, CELL));
    assert_eq!(grid.item(2), Rect::new(40.0, 100.0, CELL, CELL));
    assert_eq!(headers[1].1.x, 90.0);
    assert_eq!(grid.item(3), Rect::new(130.0, 0.0, CELL, CELL));
    grid.panel.set_group_header_placement(GroupHeaderPlacement::Top);
    grid.root.layout_manager().execute_layout_pass();
    assert_eq!(grid.item(0), Rect::new(0.0, 30.0, CELL, CELL));
    assert_eq!(grid.headers()[1].1, Rect::new(50.0, 0.0, HEADER_WIDTH, HEADER_HEIGHT));
}

#[test]
fn the_padding_of_a_group_surrounds_it() {
    let _scope = test_scope();
    let grid = Grid::new(&[1, 1], |panel, _| panel.set_group_padding(Thickness::new(5.0, 10.0, 5.0, 20.0)));
    let headers = grid.headers();
    assert_eq!(headers[0].1, Rect::new(5.0, 10.0, HEADER_WIDTH, HEADER_HEIGHT));
    assert_eq!(grid.item(0), Rect::new(5.0, 40.0, CELL, CELL));
    // 10 + 30 + 50 + 20 to the end of the first group, 10 before the second.
    assert_eq!(headers[1].1.y, 120.0);
}

#[test]
fn an_empty_group_shows_its_header_unless_it_hides() {
    let _scope = test_scope();
    let grid = Grid::new(&[1, 0, 1], |_, _| {});
    let names: Vec<String> = grid.headers().into_iter().map(|(name, _)| name).collect();
    assert_eq!(names, ["G0", "G1", "G2"]);
    assert_eq!(grid.headers()[2].1.y, 110.0, "the empty group is its header");
    let style = grid.grid.group_style().get(0);
    style.set_hides_if_empty(true);
    grid.panel.invalidate_measure();
    grid.root.layout_manager().execute_layout_pass();
    let headers = grid.headers();
    assert_eq!(headers.iter().map(|(name, _)| name.as_str()).collect::<Vec<_>>(), ["G0", "G2"]);
    assert_eq!(headers[1].1.y, 80.0);
}

#[test]
fn a_source_without_groups_removes_the_headers() {
    let _scope = test_scope();
    let grid = Grid::new(&[2, 2], |_, _| {});
    assert_eq!(grid.panel.group_headers().len(), 2);
    let header = grid.panel.group_headers()[0].clone();
    grid.grid.set_items_source(Some(ItemsSource::from_items((0..5).map(|index| boxed_str(&format!("{index}"))))));
    grid.root.layout_manager().execute_layout_pass();
    assert!(grid.panel.group_headers().is_empty());
    assert!(header.parent().is_none());
    assert!(header.visual_parent().is_none());
    assert_eq!(grid.item(4), Rect::new(0.0, 50.0, CELL, CELL), "the cells of the plain grid");
    // A change of the groups: a group more, a header more.
    grid.grid.set_items_source(Some(ItemsSource::new(groups(&[1, 1, 1]))));
    grid.root.layout_manager().execute_layout_pass();
    assert_eq!(grid.headers().len(), 3);
}

/// The items of a grid in `Multiple` get the `:multiselect` pseudo-class of the check boxes, as
/// the items of a list view.
#[test]
fn the_items_of_a_grid_in_multiple_get_the_check_box_state() {
    let _scope = test_scope();
    let grid = Grid::new(&[2, 1], |_, _| {});
    let states = || -> Vec<bool> {
        grid.grid.get_realized_containers().iter().map(|item| item.classes().contains(":multiselect")).collect()
    };
    assert_eq!(states(), [false; 3]);
    grid.grid.set_selection_mode(ListViewSelectionMode::Multiple);
    assert_eq!(states(), [true; 3]);
    grid.grid.set_is_multi_select_check_box_enabled(false);
    assert_eq!(states(), [false; 3]);
}
