//! The code behind of the GridView page of the gallery (`GridViewPage.xaml.cs`), in Rust: the
//! items of its three examples, the clicked and selected items, and the options of the last two.

use crate::list_samples::{self, CustomDataObject};
use ferroui_base::collections::FerroList;
use ferroui_base::input::DragDrop;
use ferroui_base::media::FlowDirection;
use ferroui_base::{BoxedValue, Ref, Thickness};
use ferroui_controls::primitives::{SelectingItemsControl, ToggleButton};
use ferroui_controls::{CheckBox, ComboBox, Control, GridView, ItemsWrapGrid, ListViewSelectionMode, NumberBox, TextBlock};
use std::rc::Rc;

pub(crate) fn wire(page_id: &str, root: &Ref<Control>) {
    if page_id != "GridView" {
        return;
    }
    list_samples::register();
    wire_basic(root);
    wire_layout(root);
    wire_content(root);
}

/// The items of `GetDataObjects`, in a list the grid can reorder.
fn data_objects() -> Rc<FerroList<BoxedValue>> {
    Rc::new(FerroList::from_items(
        CustomDataObject::get_data_objects().into_iter().map(|item| Rc::new(item) as BoxedValue),
    ))
}

/// The item of a list of the page, if it is one.
pub(crate) fn data_object_of(item: &BoxedValue) -> Option<Rc<CustomDataObject>> {
    item.downcast_ref::<Rc<CustomDataObject>>().cloned()
}

/// The string of the selected item of a combo box of the page.
fn selected_text(combo_box: &ComboBox) -> String {
    combo_box
        .selected_item()
        .as_ref()
        .and_then(|item| item.downcast_ref::<String>().cloned())
        .unwrap_or_default()
}

/// `on_change` runs with the page when the selection of the combo box changes.
fn on_combo_box(root: &Ref<Control>, name: &str, on_change: impl Fn(&Ref<Control>, &str) + 'static) {
    let combo_box = root.get_control::<ComboBox>(name);
    let weak_root = root.downgrade();
    let weak_combo_box = combo_box.downgrade();
    combo_box.property_changed(move |change| {
            if change.property() != SelectingItemsControl::selected_index_property().as_property() {
                return;
            }
            if let (Some(root), Some(combo_box)) = (weak_root.upgrade(), weak_combo_box.upgrade()) {
                on_change(&root, &selected_text(&combo_box));
            }
        });
}

/// `on_change` runs with the page and the state of the check box when it changes.
fn on_check_box(root: &Ref<Control>, name: &str, on_change: impl Fn(&Ref<Control>, bool) + 'static) {
    let check_box = root.get_control::<CheckBox>(name);
    let weak_root = root.downgrade();
    let weak_check_box = check_box.downgrade();
    check_box.property_changed(move |change| {
            if change.property() != ToggleButton::is_checked_property().as_property() {
                return;
            }
            if let (Some(root), Some(check_box)) = (weak_root.upgrade(), weak_check_box.upgrade()) {
                on_change(&root, check_box.is_checked() == Some(true));
            }
        });
}

/// The first example and `BasicGridView_ItemClick`: the title of the clicked item below the grid.
fn wire_basic(root: &Ref<Control>) {
    let grid = root.get_control::<GridView>("BasicGridView");
    grid.set_items_source(Some(data_objects().into()));
    let output = root.get_control::<TextBlock>("BasicClickOutput").downgrade();
    grid.item_click(move |_, e| {
        let (Some(output), Some(item)) = (output.upgrade(), e.clicked_item().as_ref().and_then(data_object_of)) else {
            return;
        };
        output.set_text(Some(&format!("You clicked {}.", item.title)));
    })
    .detach();
}

/// The second example: the margins of the items and the items of a row of the ItemsWrapGrid.
fn wire_layout(root: &Ref<Control>) {
    let grid = root.get_control::<GridView>("StyledGrid");
    grid.set_items_source(Some(data_objects().into()));
    // The margins of the ItemContainerStyle of the source, for each container that is prepared.
    let weak_root = root.downgrade();
    grid.container_prepared(move |e| {
        if let Some(root) = weak_root.upgrade() {
            e.container().set_margin(item_margin(&root));
        }
    })
    .detach();
    for name in ["ColumnMarginBox", "RowMarginBox", "MaxItemsBox"] {
        let weak_root = root.downgrade();
        root.get_control::<NumberBox>(name)
            .value_changed(move |_| {
                if let Some(root) = weak_root.upgrade() {
                    apply_layout(&root);
                }
            })
            .detach();
    }
    let weak_root = root.downgrade();
    grid.loaded(move |_, _| {
        if let Some(root) = weak_root.upgrade() {
            apply_layout(&root);
        }
    });
}

/// `$(ColMargin), $(RowMargin), $(ColMargin), $(RowMargin)`.
fn item_margin(root: &Ref<Control>) -> Thickness {
    let column = root.get_control::<NumberBox>("ColumnMarginBox").value();
    let row = root.get_control::<NumberBox>("RowMarginBox").value();
    let column = if column.is_finite() { column } else { 0.0 };
    let row = if row.is_finite() { row } else { 0.0 };
    Thickness::new(column, row, column, row)
}

/// The margins of the realized items and the `MaximumRowsOrColumns` of `MaxItemsWrapGrid`.
pub(crate) fn apply_layout(root: &Ref<Control>) {
    let grid = root.get_control::<GridView>("StyledGrid");
    let margin = item_margin(root);
    for container in grid.get_realized_containers() {
        container.set_margin(margin);
    }
    let most = root.get_control::<NumberBox>("MaxItemsBox").value();
    if let Some(panel) = grid.items_panel_root().and_then(|panel| panel.cast::<ItemsWrapGrid>()) {
        if most.is_finite() && most >= 1.0 {
            panel.set_maximum_rows_or_columns(most as i32);
        }
    }
}

/// The selection modes of the options, by name.
pub(crate) fn selection_mode(name: &str) -> Option<ListViewSelectionMode> {
    match name {
        "None" => Some(ListViewSelectionMode::None),
        "Single" => Some(ListViewSelectionMode::Single),
        "Multiple" => Some(ListViewSelectionMode::Multiple),
        "Extended" => Some(ListViewSelectionMode::Extended),
        _ => None,
    }
}

/// The third example: `ContentGridView_SelectionChanged`, `ContentGridView_ItemClick` and the
/// options that stand for the `$(...)` values of the sample.
fn wire_content(root: &Ref<Control>) {
    let grid = root.get_control::<GridView>("ContentGridView");
    grid.set_items_source(Some(data_objects().into()));
    grid.set_selection_mode(ListViewSelectionMode::Single);

    let output = root.get_control::<TextBlock>("ContentSelectionOutput").downgrade();
    let weak_grid = grid.downgrade();
    grid.selection_changed(move |_, _| {
        let (Some(output), Some(grid)) = (output.upgrade(), weak_grid.upgrade()) else { return };
        let count = grid.selected_items().map_or(0, |items| items.0.count());
        output.set_text(Some(&format!("{count} items selected.")));
    });
    let output = root.get_control::<TextBlock>("ContentClickOutput").downgrade();
    grid.item_click(move |_, e| {
        let (Some(output), Some(item)) = (output.upgrade(), e.clicked_item().as_ref().and_then(data_object_of)) else {
            return;
        };
        output.set_text(Some(&format!("You clicked {}.", item.title)));
    })
    .detach();

    on_combo_box(root, "ItemTemplateComboBox", |root, name| {
        let grid = root.get_control::<GridView>("ContentGridView");
        if let Some(template) = crate::repeater_samples::page_template(root, name) {
            grid.set_item_template(Some(template));
        }
        root.get_control::<TextBlock>("TemplateName").set_text(Some(&format!("The item template is {name}.")));
    });
    root.get_control::<TextBlock>("TemplateName").set_text(Some("The item template is ImageTemplate."));
    on_check_box(root, "ItemClickCheckBox", |root, on| {
        root.get_control::<GridView>("ContentGridView").set_is_item_click_enabled(on);
    });
    on_check_box(root, "DragCheckBox", |root, on| {
        root.get_control::<GridView>("ContentGridView").set_can_drag_items(on);
    });
    on_check_box(root, "DropCheckBox", |root, on| {
        DragDrop::set_allow_drop(&root.get_control::<GridView>("ContentGridView"), on);
    });
    on_check_box(root, "ReorderCheckBox", |root, on| {
        root.get_control::<GridView>("ContentGridView").set_can_reorder_items(on);
    });
    on_combo_box(root, "SelectionModeComboBox", |root, name| {
        if let Some(mode) = selection_mode(name) {
            root.get_control::<GridView>("ContentGridView").set_selection_mode(mode);
        }
    });
    on_combo_box(root, "FlowDirectionComboBox", |root, name| {
        let direction = if name == "RightToLeft" { FlowDirection::RightToLeft } else { FlowDirection::LeftToRight };
        root.get_control::<GridView>("ContentGridView").set_flow_direction(direction);
    });
}
