//! `Primitives/INaturalWidthMeasureCache.cs`: the size an element desired when it was measured with
//! an unconstrained width, kept until its measure is invalidated.

use super::{TreeDataGridCell, TreeDataGridColumnHeader};
use ferroui_base::Size;
use ferroui_controls::Control;

/// An element that remembers its natural (unconstrained width) desired size
/// (`INaturalWidthMeasureCache`).
pub trait INaturalWidthMeasureCache {
    fn natural_desired_size(&self) -> Option<Size>;
    fn set_natural_desired_size(&self, value: Option<Size>);
}

/// The element as a natural width measure cache (`element is INaturalWidthMeasureCache`): the cells
/// and the column headers of the control.
pub fn as_natural_width_measure_cache(element: &Control) -> Option<&dyn INaturalWidthMeasureCache> {
    if let Some(cell) = element.downcast_ref::<TreeDataGridCell>() {
        return Some(cell as &dyn INaturalWidthMeasureCache);
    }

    if let Some(header) = element.downcast_ref::<TreeDataGridColumnHeader>() {
        return Some(header as &dyn INaturalWidthMeasureCache);
    }

    None
}
