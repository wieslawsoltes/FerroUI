//! The GridView page: the source's three examples with their code behind (`grid_view_samples.rs`),
//! laid out in a window with the WinUI theme.

use crate::grid_view_samples::{data_object_of, selection_mode};
use crate::{examples, tests::start_winui};
use ferroui_base::input::DragDrop;
use ferroui_base::media::FlowDirection;
use ferroui_base::{Ref, Thickness};
use ferroui_controls::{
    CheckBox, ComboBox, Control, GridView, ItemsWrapGrid, ListViewSelectionMode, NumberBox, TextBlock, Window,
};

fn shown() -> (Ref<Control>, Ref<Window>) {
    let root = examples::create(examples::find("GridView").unwrap()).unwrap_or_else(|error| panic!("GridView: {error}"));
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    window.update_layout();
    (root, window)
}

fn titles(grid: &GridView) -> Vec<String> {
    let items = grid.items_source().expect("the grid has items").to_vec();
    items.iter().flatten().filter_map(data_object_of).map(|item| item.title.clone()).collect()
}

#[test]
fn each_grid_gets_the_data_objects_of_the_source() {
    let _application = start_winui();
    let (root, window) = shown();
    for name in ["BasicGridView", "StyledGrid", "ContentGridView"] {
        let grid = root.get_control::<GridView>(name);
        assert_eq!(titles(&grid), (1..=8).map(|index| format!("Item {index}")).collect::<Vec<_>>(), "{name}");
        assert!(!grid.get_realized_containers().is_empty(), "{name} realizes its items");
    }
    window.close();
}

#[test]
fn a_click_in_the_basic_grid_shows_the_title_of_the_item() {
    let _application = start_winui();
    let (root, window) = shown();
    let grid = root.get_control::<GridView>("BasicGridView");
    assert!(grid.is_item_click_enabled());
    grid.invoke_item(2);
    assert_eq!(root.get_control::<TextBlock>("BasicClickOutput").text().as_deref(), Some("You clicked Item 3."));
    window.close();
}

#[test]
fn the_layout_options_set_the_margins_and_the_items_of_a_row() {
    let _application = start_winui();
    let (root, window) = shown();
    root.get_control::<NumberBox>("ColumnMarginBox").set_value(NumberBox::value_property(), 12.0);
    root.get_control::<NumberBox>("RowMarginBox").set_value(NumberBox::value_property(), 3.0);
    root.get_control::<NumberBox>("MaxItemsBox").set_value(NumberBox::value_property(), 2.0);
    window.update_layout();
    let grid = root.get_control::<GridView>("StyledGrid");
    for container in grid.get_realized_containers() {
        assert_eq!(container.margin(), Thickness::new(12.0, 3.0, 12.0, 3.0));
    }
    let panel = grid.items_panel_root().and_then(|panel| panel.cast::<ItemsWrapGrid>()).expect("an ItemsWrapGrid");
    assert_eq!(panel.maximum_rows_or_columns(), 2);
    window.close();
}

#[test]
fn the_content_options_reach_the_grid() {
    let _application = start_winui();
    let (root, window) = shown();
    let grid = root.get_control::<GridView>("ContentGridView");
    let check = |name: &str| root.get_control::<CheckBox>(name).set_is_checked(Some(true));
    check("ItemClickCheckBox");
    check("DragCheckBox");
    check("DropCheckBox");
    check("ReorderCheckBox");
    assert!(grid.is_item_click_enabled() && grid.can_drag_items() && grid.can_reorder_items());
    assert!(DragDrop::get_allow_drop(&grid));

    root.get_control::<ComboBox>("SelectionModeComboBox").set_selected_index(2);
    assert_eq!(grid.selection_mode(), ListViewSelectionMode::Multiple);
    root.get_control::<ComboBox>("FlowDirectionComboBox").set_selected_index(1);
    assert_eq!(grid.flow_direction(), FlowDirection::RightToLeft);
    root.get_control::<ComboBox>("ItemTemplateComboBox").set_selected_index(2);
    assert_eq!(root.get_control::<TextBlock>("TemplateName").text().as_deref(), Some("The item template is TextTemplate."));
    assert!(grid.item_template().is_some());

    window.update_layout();
    grid.invoke_item(0);
    assert_eq!(root.get_control::<TextBlock>("ContentClickOutput").text().as_deref(), Some("You clicked Item 1."));
    grid.select_all();
    assert_eq!(root.get_control::<TextBlock>("ContentSelectionOutput").text().as_deref(), Some("8 items selected."));
    window.close();
}

#[test]
fn the_selection_modes_of_the_options_are_those_of_winui() {
    assert_eq!(selection_mode("None"), Some(ListViewSelectionMode::None));
    assert_eq!(selection_mode("Extended"), Some(ListViewSelectionMode::Extended));
    assert_eq!(selection_mode("Other"), None);
}
