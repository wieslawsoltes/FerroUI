//! The code behind of the pinned SelectorBar page in Rust: `SelectorBar2_SelectionChanged`, which
//! navigates a Frame with a slide in the direction of the selection, and
//! `SelectorBar3_SelectionChanged` with `PopulateColorCollections`.

use crate::navigation_view_samples::{page_type, register_pages};
use ferroui_base::media::{Brushes, IBrush};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::navigation::NavigationTransitionInfo;
use ferroui_controls::{Border, Control, Frame, ItemsControl, SelectorBar};
use ferroui_markup_xaml::XamlLoadError;
use std::cell::Cell;
use std::rc::Rc;

pub(crate) fn wire(root: &Ref<Control>) -> Result<(), XamlLoadError> {
    let Some(bar) = root.find_control::<SelectorBar>("SelectorBar2") else {
        return Ok(());
    };
    wire_frame(root, &bar)?;
    wire_collections(root);
    Ok(())
}

/// The slide of a move from `previous` to `current`: forward comes from the right.
pub(crate) fn slide(previous: i32, current: i32) -> NavigationTransitionInfo {
    if current - previous > 0 {
        NavigationTransitionInfo::SlideFromRight
    } else {
        NavigationTransitionInfo::SlideFromLeft
    }
}

/// The sample page of an index of the bar; the last page for any later index, as in the source.
fn sample_page(index: i32) -> Option<ferroui_controls::PageType> {
    page_type(match index {
        0 => "SamplePage1",
        1 => "SamplePage2",
        2 => "SamplePage3",
        3 => "SamplePage4",
        _ => "SamplePage5",
    })
}

fn wire_frame(root: &Ref<Control>, bar: &Ref<SelectorBar>) -> Result<(), XamlLoadError> {
    let frame = root.get_control::<Frame>("SelectorContentFrame");
    register_pages(&frame)
        .map_err(|error| XamlLoadError::with_inner("Could not initialize the SelectorBar example.", error))?;
    let previous = Rc::new(Cell::new(0));
    let weak_bar = bar.downgrade();
    let weak_frame = frame.downgrade();
    bar.selection_changed(move |_, _| {
        let (Some(bar), Some(frame)) = (weak_bar.upgrade(), weak_frame.upgrade()) else {
            return;
        };
        let current = bar.selected_index();
        if let Some(page) = sample_page(current) {
            // A failed navigation leaves the current page, as a failed `Navigate` does.
            drop(frame.navigate_with_transition(page, None, slide(previous.get(), current)));
        }
        previous.set(current);
    });
    // `IsSelected="True"` of the first item.
    bar.set_selected_index(0);
    Ok(())
}

/// A tile of the source's `ColorsTemplate`.
fn tile(brush: &Rc<dyn IBrush>) -> Ref<Control> {
    let border = Border::new();
    border.set_width(112.0);
    border.set_height(82.0);
    border.set_margin(Thickness::uniform(4.0));
    border.set_background(Some(brush.clone()));
    border.upcast()
}

/// `PopulateColorCollections`: the brush and the number of tiles of each item of the bar.
pub(crate) fn collection(index: i32) -> (Rc<dyn IBrush>, usize) {
    match index {
        0 => (Brushes::pink(), 5),
        1 => (Brushes::plum(), 7),
        _ => (Brushes::powder_blue(), 4),
    }
}

fn show_collection(items: &ItemsControl, index: i32) {
    let (brush, count) = collection(index);
    items.items().clear();
    for _ in 0..count {
        items.items().add(Some(Control::boxed(tile(&brush))));
    }
}

fn wire_collections(root: &Ref<Control>) {
    let bar = root.get_control::<SelectorBar>("SelectorBar3");
    let items = root.get_control::<ItemsControl>("ItemsView3");
    let weak_bar = bar.downgrade();
    let weak_items = items.downgrade();
    bar.selection_changed(move |_, _| {
        if let (Some(bar), Some(items)) = (weak_bar.upgrade(), weak_items.upgrade()) {
            show_collection(&items, bar.selected_index());
        }
    });
    // `IsSelected="True"` of the first item.
    bar.set_selected_index(0);
}
