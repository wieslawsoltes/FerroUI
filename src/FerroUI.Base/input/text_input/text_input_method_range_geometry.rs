//! A short-lived geometry snapshot for a platform query of the visible text.

use super::{TextInputMethodError, TextSelection};
use crate::{Matrix, Point, Rect};
use std::fmt;
use std::rc::Rc;

/// One shaped text fragment and its transform to the semantic input owner.
///
/// Keeping the local rectangle separate from its transform lets a platform
/// transform its four corners before taking screen bounds. Taking a bounding
/// box at each intermediate visual would inflate rotated text unnecessarily.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextInputMethodTextBounds {
    pub rectangle: Rect,
    pub transform: Matrix,
}

impl TextInputMethodTextBounds {
    /// A rectangle already expressed in the semantic owner's coordinates.
    pub const fn new(rectangle: Rect) -> Self {
        Self::with_transform(rectangle, Matrix::IDENTITY)
    }

    pub const fn with_transform(rectangle: Rect, transform: Matrix) -> Self {
        Self {
            rectangle,
            transform,
        }
    }

    /// The actual fragment corners, in the semantic owner's coordinates.
    pub fn corners(&self) -> [Point; 4] {
        let rect = self.rectangle;
        [
            Point::new(rect.x, rect.y),
            Point::new(rect.right(), rect.y),
            Point::new(rect.right(), rect.bottom()),
            Point::new(rect.x, rect.bottom()),
        ]
        .map(|point| point.transform(self.transform))
    }

    fn is_valid(&self) -> bool {
        let rect = self.rectangle;
        [rect.x, rect.y, rect.width, rect.height]
            .into_iter()
            .all(f64::is_finite)
            && rect.width >= 0.0
            && rect.height >= 0.0
            && self
                .corners()
                .into_iter()
                .all(|point| point.x.is_finite() && point.y.is_finite())
    }
}

/// Complete geometry for one ordered UTF-16 range of the currently displayed
/// input story, including any visible composition preview.
///
/// The fragments are immutable; an empty range describes the requested
/// insertion position. A nonempty range can span shaped clusters, bidi runs,
/// lines or linked views. No fragment is a candidate-window rectangle.
///
/// Layout and native property getters can invoke application code. Adapters
/// must call [`is_current`](Self::is_current) after callbacks and before each
/// native publication, in addition to checking their input-session ownership.
/// A snapshot never authorizes an edit or keeps an input connection current.
#[derive(Clone)]
pub struct TextInputMethodRangeGeometry {
    range: TextSelection,
    fragments: Rc<[TextInputMethodTextBounds]>,
    visual_version: u64,
    is_current: Rc<dyn Fn() -> bool>,
}

impl TextInputMethodRangeGeometry {
    /// Creates a snapshot with a client-supplied validity check. The check must
    /// be observational and detect document, layout, scrolling and view changes
    /// not already represented by the framework's visual geometry version.
    /// Capture weak references so a retained result cannot retain its control.
    ///
    /// Empty fragments mean geometry is unavailable, not a request to use the
    /// current caret. Zero-width fragments are valid for line/paragraph breaks.
    pub fn new(
        range: TextSelection,
        fragments: impl Into<Rc<[TextInputMethodTextBounds]>>,
        is_current: impl Fn() -> bool + 'static,
    ) -> Result<Self, TextInputMethodError> {
        if range.start < 0 || range.end < range.start {
            return Err(TextInputMethodError::InvalidRange);
        }
        let fragments = fragments.into();
        if fragments.is_empty() || fragments.iter().any(|fragment| !fragment.is_valid()) {
            return Err(TextInputMethodError::Unavailable);
        }
        Ok(Self {
            range,
            fragments,
            visual_version: crate::visual::geometry_version(),
            is_current: Rc::new(is_current),
        })
    }

    pub fn range(&self) -> TextSelection {
        self.range
    }

    pub fn fragments(&self) -> &[TextInputMethodTextBounds] {
        &self.fragments
    }

    /// Checks both client layout state and synchronous visual changes. The
    /// second version check also rejects a visual change made by the callback.
    /// The caller must still recheck its own generation after this client call.
    pub fn is_current(&self) -> bool {
        self.visual_version == crate::visual::geometry_version()
            && (self.is_current)()
            && self.visual_version == crate::visual::geometry_version()
    }
}

impl fmt::Debug for TextInputMethodRangeGeometry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextInputMethodRangeGeometry")
            .field("range", &self.range)
            .field("fragments", &self.fragments)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Visual;
    use std::cell::Cell;

    #[test]
    fn input_range_geometry_keeps_utf16_indices_and_unflattened_fragments() {
        let fragment = TextInputMethodTextBounds::with_transform(
            Rect::new(2.0, 3.0, 4.0, 5.0),
            Matrix::new(0.0, 1.0, -1.0, 0.0, 10.0, 20.0),
        );
        let result =
            TextInputMethodRangeGeometry::new(TextSelection::new(2, 3), vec![fragment], || true)
                .unwrap();
        assert_eq!(result.range(), TextSelection::new(2, 3));
        assert_eq!(result.fragments(), &[fragment]);
        assert_eq!(
            fragment.corners(),
            [
                Point::new(7.0, 22.0),
                Point::new(7.0, 26.0),
                Point::new(2.0, 26.0),
                Point::new(2.0, 22.0),
            ]
        );
        assert!(result.is_current());
    }

    #[test]
    fn input_range_geometry_rejects_missing_invalid_and_nonfinite_bounds() {
        let good = TextInputMethodTextBounds::new(Rect::new(0.0, 0.0, 1.0, 10.0));
        for range in [TextSelection::new(-1, 2), TextSelection::new(3, 2)] {
            assert_eq!(
                TextInputMethodRangeGeometry::new(range, vec![good], || true).unwrap_err(),
                TextInputMethodError::InvalidRange
            );
        }
        for fragments in [
            vec![],
            vec![TextInputMethodTextBounds::new(Rect::new(
                0.0, 0.0, -1.0, 10.0,
            ))],
            vec![TextInputMethodTextBounds::new(Rect::new(
                f64::NAN,
                0.0,
                1.0,
                10.0,
            ))],
            vec![TextInputMethodTextBounds::with_transform(
                good.rectangle,
                Matrix::new(f64::INFINITY, 0.0, 0.0, 1.0, 0.0, 0.0),
            )],
        ] {
            assert_eq!(
                TextInputMethodRangeGeometry::new(TextSelection::new(0, 0), fragments, || true)
                    .unwrap_err(),
                TextInputMethodError::Unavailable
            );
        }
        let empty = TextInputMethodRangeGeometry::new(
            TextSelection::new(0, 0),
            vec![TextInputMethodTextBounds::new(Rect::new(
                3.0, 4.0, 0.0, 8.0,
            ))],
            || true,
        )
        .unwrap();
        assert_eq!(empty.fragments()[0].rectangle.height, 8.0);
        assert_eq!(empty.fragments()[0].rectangle.width, 0.0);
    }

    #[test]
    fn input_range_geometry_rejects_client_changes_and_reentrant_visual_changes() {
        let _scope = crate::threading::Dispatcher::unit_test_scope();
        let current = Rc::new(Cell::new(true));
        let captured = current.clone();
        let fragment = TextInputMethodTextBounds::new(Rect::new(0.0, 0.0, 1.0, 10.0));
        let result = TextInputMethodRangeGeometry::new(
            TextSelection::new(0, 0),
            vec![fragment],
            move || captured.get(),
        )
        .unwrap();
        assert!(result.is_current());
        current.set(false);
        assert!(!result.is_current());

        let visual = Visual::new();
        let result = TextInputMethodRangeGeometry::new(
            TextSelection::new(0, 0),
            vec![fragment],
            move || {
                visual.set_bounds(Rect::new(0.0, 0.0, 100.0, 20.0));
                true
            },
        )
        .unwrap();
        assert!(!result.is_current());
        assert!(!result.is_current());
    }
}
