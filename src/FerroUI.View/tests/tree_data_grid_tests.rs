use crate::prelude::*;
use crate::reactive::Owner;
use ferroui_base::collections::NotifyCollectionChangedAction;
use ferroui_controls::{GridLength, ItemsSource};
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn rust_tree_grid_reconciles_all_five_column_types_without_replacing_the_control() {
    let owner = Owner::new_root();
    let (step, control) = owner.with(|| {
        let items = ItemsSource::from_strs(["Alpine", "Cedar"]);
        let step = signal(0);
        let grid = rerender(move || {
            let items = items.clone();
            match step.get() {
                0 => rsx! {
                    TreeDataGrid {
                        items_source: items,
                        column_definitions: ::ferroui_view::tree_data_grid_columns([
                            ::ferroui_view::tree_data_grid_row_header_column().build(),
                            ::ferroui_view::tree_data_grid_expander_column(
                                ::ferroui_view::tree_data_grid_text_column().header("Account").build()
                            ).build(),
                            ::ferroui_view::tree_data_grid_text_column().header("Owner").build(),
                            ::ferroui_view::tree_data_grid_check_box_column().header("Active").build(),
                            ::ferroui_view::tree_data_grid_template_column().header("Status").build(),
                        ])
                    }
                },
                1 => rsx! {
                    TreeDataGrid {
                        items_source: items,
                        column_definitions: ::ferroui_view::tree_data_grid_columns([
                            ::ferroui_view::tree_data_grid_text_column().header("Customer")
                                .width(GridLength::from_pixels(180.0)).build(),
                        ])
                    }
                },
                _ => rsx! { TreeDataGrid { items_source: items } },
            }
        }).into_element().mount();
        (step, grid)
    });
    let grid = control.cast::<TreeDataGrid>().unwrap();
    let columns = grid.column_definitions().clone();
    let original = columns.snapshot();
    assert_eq!(original.len(), 5);
    assert!(grid.source().is_some());

    let events = Rc::new(RefCell::new(Vec::new()));
    let captured = events.clone();
    let captured_columns = columns.clone();
    let subscription = columns.add_collection_changed(Rc::new(move |event| {
        captured
            .borrow_mut()
            .push((event.action, captured_columns.count()));
    }));
    step.set(1);
    assert!(columns == *grid.column_definitions());
    assert_eq!(original.len(), 5);
    assert_eq!(
        columns.get(0).base().width(),
        GridLength::from_pixels(180.0)
    );
    assert_eq!(
        events.borrow().as_slice(),
        &[(NotifyCollectionChangedAction::Reset, 1)]
    );
    assert!(grid.source().is_some());

    step.set(2);
    assert_eq!(columns.count(), 0);
    assert_eq!(
        events.borrow().as_slice(),
        &[
            (NotifyCollectionChangedAction::Reset, 1),
            (NotifyCollectionChangedAction::Reset, 0),
        ]
    );
    columns.remove_collection_changed(subscription);
    owner.dispose();
}
