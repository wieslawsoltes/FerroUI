use super::RealizedStackElements;
use crate::primitives::SnapPointsAlignment;
use ferroui_base::layout::Orientation;
use std::rc::Rc;

const EXTRA_COUNT: i32 = 2;

/// The snap points of a virtualizing stack panel: the positions of the
/// realized elements, extended on both sides by estimated positions.
///
/// The list is read-only and computes each snap point when it is read.
pub(crate) struct VirtualizingSnapPointsList {
    realized_elements: Rc<RealizedStackElements>,
    orientation: Orientation,
    parent_orientation: Orientation,
    snap_points_alignment: SnapPointsAlignment,
    size: f64,
    start: i32,
    end: i32,
}

impl VirtualizingSnapPointsList {
    pub fn new(
        realized_elements: Rc<RealizedStackElements>,
        count: i32,
        orientation: Orientation,
        parent_orientation: Orientation,
        snap_points_alignment: SnapPointsAlignment,
        size: f64,
    ) -> Self {
        let mut start = -1;
        let mut end = 0;

        if parent_orientation == orientation {
            start = (realized_elements.first_index() - EXTRA_COUNT).max(0);
            end = (count - 1).min(realized_elements.last_index() + EXTRA_COUNT);
        }

        Self { realized_elements, orientation, parent_orientation, snap_points_alignment, size, start, end }
    }

    /// The snap point at `index`. Panics if the index is out of range.
    pub fn get(&self, index: i32) -> f64 {
        if index < 0 || index >= self.count() {
            panic!("Specified argument was out of the range of valid values. (Parameter 'index')");
        }

        let index = index + self.start;

        let snap_point;
        let average_element_size = self.size;
        let realized_elements = &self.realized_elements;

        match self.orientation {
            Orientation::Horizontal => {
                if let Some(container) = realized_elements.get_element(index) {
                    let bounds = container.bounds();
                    snap_point = match self.snap_points_alignment {
                        SnapPointsAlignment::Near => bounds.left(),
                        SnapPointsAlignment::Center => bounds.center().x,
                        SnapPointsAlignment::Far => bounds.right(),
                    };
                } else {
                    let mut ind = index;
                    if index > realized_elements.last_index() {
                        ind -= realized_elements.last_index() + 1;
                    }
                    let mut estimated = ind as f64 * average_element_size;
                    match self.snap_points_alignment {
                        SnapPointsAlignment::Center => estimated += average_element_size / 2.0,
                        SnapPointsAlignment::Far => estimated += average_element_size,
                        SnapPointsAlignment::Near => {}
                    }
                    if index > realized_elements.last_index() {
                        let last_element = realized_elements.get_element(realized_elements.last_index());
                        if let Some(last_element) = last_element {
                            estimated += last_element.bounds().right();
                        }
                    }
                    snap_point = estimated;
                }
            }
            Orientation::Vertical => {
                if let Some(container) = realized_elements.get_element(index) {
                    let bounds = container.bounds();
                    snap_point = match self.snap_points_alignment {
                        SnapPointsAlignment::Near => bounds.top(),
                        SnapPointsAlignment::Center => bounds.center().y,
                        SnapPointsAlignment::Far => bounds.bottom(),
                    };
                } else {
                    let mut ind = index;
                    if index > realized_elements.last_index() {
                        ind -= realized_elements.last_index() + 1;
                    }
                    let mut estimated = ind as f64 * average_element_size;
                    match self.snap_points_alignment {
                        SnapPointsAlignment::Center => estimated += average_element_size / 2.0,
                        SnapPointsAlignment::Far => estimated += average_element_size,
                        SnapPointsAlignment::Near => {}
                    }
                    if index > realized_elements.last_index() {
                        let last_element = realized_elements.get_element(realized_elements.last_index());
                        if let Some(last_element) = last_element {
                            estimated += last_element.bounds().bottom();
                        }
                    }
                    snap_point = estimated;
                }
            }
        }

        snap_point
    }

    /// The number of snap points.
    pub fn count(&self) -> i32 {
        if self.parent_orientation != self.orientation {
            0
        } else {
            self.end - self.start + 1
        }
    }

    /// Enumerates the snap points.
    pub fn iter(&self) -> impl Iterator<Item = f64> + '_ {
        (0..self.count()).map(move |i| self.get(i))
    }
}
