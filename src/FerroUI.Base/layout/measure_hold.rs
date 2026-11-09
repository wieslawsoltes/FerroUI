//! Measure holds: the range of available sizes over which the measure of an element returns the
//! same desired size and leaves its descendants as they are. An addition (FerroUI deviation,
//! desktop-performance.md section 9.15): an element whose measure is valid and whose new available
//! size is inside its hold is not measured again.
//!
//! An element reports a hold from `LayoutableImpl::measure_hold_override`, in the space of the
//! constraint its `measure_override` received; `Layoutable::measure_core` maps it to the space of
//! the available size through the margin, the minimum, maximum and explicit sizes and the clamp of
//! the desired size to the available size. The default is no hold: an element is measured again
//! at every new available size, as upstream measures it.

use crate::Size;

/// A range `[low, high)` of widths and one of heights.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeasureHold {
    /// The widths, from `width.0` (included) to `width.1` (excluded).
    pub width: (f64, f64),
    /// The heights, from `height.0` (included) to `height.1` (excluded).
    pub height: (f64, f64),
}

impl MeasureHold {
    /// Every size.
    pub const ANY: MeasureHold =
        MeasureHold { width: (f64::NEG_INFINITY, f64::INFINITY), height: (f64::NEG_INFINITY, f64::INFINITY) };

    /// Every size whose width and height are inside the ranges.
    pub fn new(width: (f64, f64), height: (f64, f64)) -> Self {
        Self { width, height }
    }

    /// Whether `size` is inside the hold. An infinite upper bound includes infinity: an element
    /// measured at an infinite size (a child of a stack along the stack) is inside its hold.
    pub fn contains(&self, size: Size) -> bool {
        fn inside(value: f64, (low, high): (f64, f64)) -> bool {
            low <= value && (value < high || (high == f64::INFINITY && value == f64::INFINITY))
        }
        inside(size.width, self.width) && inside(size.height, self.height)
    }

    /// The sizes inside both holds, or `None` when there are none.
    pub fn intersect(self, other: MeasureHold) -> Option<MeasureHold> {
        let width = (self.width.0.max(other.width.0), self.width.1.min(other.width.1));
        let height = (self.height.0.max(other.height.0), self.height.1.min(other.height.1));
        (width.0 < width.1 && height.0 < height.1).then_some(MeasureHold { width, height })
    }

    /// The hold moved by `width` and `height`: the hold of an element that gives its child its own
    /// available size less a fixed amount (a padding, a border).
    pub fn inflate(self, width: f64, height: f64) -> MeasureHold {
        MeasureHold {
            width: (self.width.0 + width, self.width.1 + width),
            height: (self.height.0 + height, self.height.1 + height),
        }
    }

    /// The hold with every width.
    pub fn any_width(self) -> MeasureHold {
        MeasureHold { width: Self::ANY.width, ..self }
    }

    /// The hold with every height.
    pub fn any_height(self) -> MeasureHold {
        MeasureHold { height: Self::ANY.height, ..self }
    }
}

/// Maps the range `[low, high)` of one axis of the constraint a `measure_override` received to
/// the range of the available size the element was measured with, for a constraint computed as
/// `clamp(available - margin, min, max)` and a desired size (margin included, before the clamp to
/// the available size) of `desired`. `None` when no available size other than the one measured
/// is known to give the same result.
pub(crate) fn map_axis(range: (f64, f64), margin: f64, min: f64, max: f64, desired: f64, available: f64) -> Option<(f64, f64)> {
    let (low, high) = range;
    // The constraint is in `[min, max]`: the range must meet it.
    if !(min < high && low <= max) {
        return None;
    }
    // `clamp(x)` is `min` below `min`, `max` above `max`, `x` between: below `min` it is in the
    // range when `min` is, above `max` when `max` is.
    let x_low = if min >= low { f64::NEG_INFINITY } else { low };
    let x_high = if max < high { f64::INFINITY } else { high };
    // A desired size larger than the available size is clamped to it: it depends on it.
    if desired > available {
        return None;
    }
    let mapped = ((x_low + margin).max(desired), x_high + margin);
    let inside = mapped.0 <= available && (available < mapped.1 || (mapped.1 == f64::INFINITY && available == f64::INFINITY));
    inside.then_some(mapped)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_constraint_range_maps_through_the_margin_and_the_desired_size() {
        // No limits: the range moves by the margin, and starts at the desired size at the least.
        assert_eq!(Some((110.0, 210.0)), map_axis((100.0, 200.0), 10.0, 0.0, f64::INFINITY, 50.0, 150.0));
        assert_eq!(Some((120.0, 210.0)), map_axis((100.0, 200.0), 10.0, 0.0, f64::INFINITY, 120.0, 150.0));
        // A desired size clamped to the available size depends on it.
        assert_eq!(None, map_axis((100.0, 200.0), 10.0, 0.0, f64::INFINITY, 160.0, 150.0));
        // The measured size must be inside.
        assert_eq!(None, map_axis((100.0, 200.0), 10.0, 0.0, f64::INFINITY, 50.0, 300.0));
    }

    #[test]
    fn an_explicit_size_holds_for_every_available_size_it_fits() {
        // Width 80: min = max = 80; any available size at least the desired size holds.
        assert_eq!(Some((90.0, f64::INFINITY)), map_axis((0.0, 1000.0), 10.0, 80.0, 80.0, 90.0, 400.0));
        // A maximum inside the range: above it the constraint is the maximum.
        assert_eq!(Some((60.0, f64::INFINITY)), map_axis((50.0, 1000.0), 0.0, 0.0, 300.0, 60.0, 200.0));
        // A minimum inside the range: below it the constraint is the minimum.
        assert_eq!(Some((40.0, 500.0)), map_axis((0.0, 500.0), 0.0, 100.0, f64::INFINITY, 40.0, 200.0));
        // A range the limits never reach.
        assert_eq!(None, map_axis((0.0, 50.0), 0.0, 100.0, f64::INFINITY, 40.0, 200.0));
    }

    #[test]
    fn holds_intersect_and_move() {
        let a = MeasureHold::new((0.0, 100.0), (10.0, 20.0));
        let b = MeasureHold::new((50.0, 200.0), (0.0, f64::INFINITY));
        assert_eq!(Some(MeasureHold::new((50.0, 100.0), (10.0, 20.0))), a.intersect(b));
        assert_eq!(None, a.intersect(MeasureHold::new((100.0, 200.0), MeasureHold::ANY.height)));
        assert_eq!(MeasureHold::new((5.0, 105.0), (12.0, 22.0)), a.inflate(5.0, 2.0));
        assert!(a.contains(Size::new(0.0, 10.0)) && !a.contains(Size::new(100.0, 10.0)));
    }
}
