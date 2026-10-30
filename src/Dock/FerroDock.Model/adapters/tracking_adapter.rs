// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use std::cell::Cell;
use std::rc::Rc;

/// Tracks a dockable's layout bounds and pointer positions independently.
#[derive(Debug)]
pub struct TrackingAdapter {
    visible: Cell<(f64, f64, f64, f64)>,
    pinned: Cell<(f64, f64, f64, f64)>,
    tab: Cell<(f64, f64, f64, f64)>,
    pointer: Cell<(f64, f64)>,
    pointer_screen: Cell<(f64, f64)>,
}

impl Default for TrackingAdapter {
    fn default() -> Self {
        Self {
            visible: Cell::new((f64::NAN, f64::NAN, f64::NAN, f64::NAN)),
            pinned: Cell::new((f64::NAN, f64::NAN, f64::NAN, f64::NAN)),
            tab: Cell::new((f64::NAN, f64::NAN, f64::NAN, f64::NAN)),
            pointer: Cell::new((f64::NAN, f64::NAN)),
            pointer_screen: Cell::new((f64::NAN, f64::NAN)),
        }
    }
}

impl TrackingAdapter {
    /// Creates a shared adapter with every tracked coordinate set to `NaN`.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Gets the tracked visible bounds as `(x, y, width, height)`.
    pub fn get_visible_bounds(&self) -> (f64, f64, f64, f64) {
        self.visible.get()
    }

    /// Sets the tracked visible bounds without changing other coordinates.
    pub fn set_visible_bounds(&self, x: f64, y: f64, width: f64, height: f64) {
        self.visible.set((x, y, width, height));
    }

    /// Gets the tracked pinned preview bounds as `(x, y, width, height)`.
    pub fn get_pinned_bounds(&self) -> (f64, f64, f64, f64) {
        self.pinned.get()
    }

    /// Sets the tracked pinned bounds without validating or normalizing them.
    pub fn set_pinned_bounds(&self, x: f64, y: f64, width: f64, height: f64) {
        self.pinned.set((x, y, width, height));
    }

    /// Gets the tracked tab bounds as `(x, y, width, height)`.
    pub fn get_tab_bounds(&self) -> (f64, f64, f64, f64) {
        self.tab.get()
    }

    /// Sets the tracked tab bounds without changing other coordinates.
    pub fn set_tab_bounds(&self, x: f64, y: f64, width: f64, height: f64) {
        self.tab.set((x, y, width, height));
    }

    /// Gets the pointer position in dockable coordinates.
    pub fn get_pointer_position(&self) -> (f64, f64) {
        self.pointer.get()
    }

    /// Sets the pointer position in dockable coordinates.
    pub fn set_pointer_position(&self, x: f64, y: f64) {
        self.pointer.set((x, y));
    }

    /// Gets the pointer position in screen coordinates.
    pub fn get_pointer_screen_position(&self) -> (f64, f64) {
        self.pointer_screen.get()
    }

    /// Sets the pointer position in screen coordinates.
    pub fn set_pointer_screen_position(&self, x: f64, y: f64) {
        self.pointer_screen.set((x, y));
    }
}

impl PartialEq for TrackingAdapter {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_coordinate_starts_unset() {
        let adapter = TrackingAdapter::new();
        for (x, y, width, height) in [
            adapter.get_visible_bounds(),
            adapter.get_pinned_bounds(),
            adapter.get_tab_bounds(),
        ] {
            assert!(x.is_nan() && y.is_nan() && width.is_nan() && height.is_nan());
        }
        for (x, y) in [
            adapter.get_pointer_position(),
            adapter.get_pointer_screen_position(),
        ] {
            assert!(x.is_nan() && y.is_nan());
        }
    }

    #[test]
    fn visible_bounds_are_tracked() {
        let adapter = TrackingAdapter::new();
        adapter.set_visible_bounds(1.0, 2.0, 3.0, 4.0);
        assert_eq!(adapter.get_visible_bounds(), (1.0, 2.0, 3.0, 4.0));
    }

    #[test]
    fn pinned_bounds_are_tracked() {
        let adapter = TrackingAdapter::new();
        adapter.set_pinned_bounds(5.0, 6.0, 7.0, 8.0);
        assert_eq!(adapter.get_pinned_bounds(), (5.0, 6.0, 7.0, 8.0));
    }

    #[test]
    fn tab_bounds_are_tracked() {
        let adapter = TrackingAdapter::new();
        adapter.set_tab_bounds(9.0, 10.0, 11.0, 12.0);
        assert_eq!(adapter.get_tab_bounds(), (9.0, 10.0, 11.0, 12.0));
    }

    #[test]
    fn pointer_positions_are_tracked() {
        let adapter = TrackingAdapter::new();
        adapter.set_pointer_position(13.0, 14.0);
        adapter.set_pointer_screen_position(15.0, 16.0);
        assert_eq!(adapter.get_pointer_position(), (13.0, 14.0));
        assert_eq!(adapter.get_pointer_screen_position(), (15.0, 16.0));
    }

    #[test]
    fn coordinate_spaces_remain_independent_and_handles_share_updates() {
        let adapter = TrackingAdapter::new();
        let shared = adapter.clone();
        shared.set_visible_bounds(1.0, 2.0, 3.0, 4.0);
        shared.set_pinned_bounds(5.0, 6.0, 7.0, 8.0);
        shared.set_tab_bounds(9.0, 10.0, 11.0, 12.0);
        shared.set_pointer_position(13.0, 14.0);
        shared.set_pointer_screen_position(15.0, 16.0);
        adapter.set_visible_bounds(-1.0, -2.0, -3.0, -4.0);
        assert_eq!(shared.get_visible_bounds(), (-1.0, -2.0, -3.0, -4.0));
        assert_eq!(shared.get_pinned_bounds(), (5.0, 6.0, 7.0, 8.0));
        assert_eq!(shared.get_tab_bounds(), (9.0, 10.0, 11.0, 12.0));
        assert_eq!(shared.get_pointer_position(), (13.0, 14.0));
        assert_eq!(shared.get_pointer_screen_position(), (15.0, 16.0));
        assert_eq!(adapter, shared);
        assert_ne!(adapter, TrackingAdapter::new());
    }

    #[test]
    fn tracking_retains_special_values_and_negative_sizes_without_filtering() {
        let adapter = TrackingAdapter::new();
        let nan = f64::from_bits(0x7ff8_0000_0000_1234);
        let expected = [nan, -0.0, f64::INFINITY, f64::NEG_INFINITY];
        adapter.set_visible_bounds(expected[0], expected[1], expected[2], expected[3]);
        adapter.set_pinned_bounds(expected[0], expected[1], expected[2], expected[3]);
        adapter.set_tab_bounds(expected[0], expected[1], expected[2], expected[3]);
        for bounds in [
            adapter.get_visible_bounds(),
            adapter.get_pinned_bounds(),
            adapter.get_tab_bounds(),
        ] {
            let actual = [bounds.0, bounds.1, bounds.2, bounds.3];
            for (actual, expected) in actual.into_iter().zip(expected) {
                assert_eq!(actual.to_bits(), expected.to_bits());
            }
        }
        adapter.set_pointer_position(-0.0, nan);
        adapter.set_pointer_screen_position(f64::NEG_INFINITY, f64::INFINITY);
        assert_eq!(
            adapter.get_pointer_position().0.to_bits(),
            (-0.0f64).to_bits()
        );
        assert_eq!(adapter.get_pointer_position().1.to_bits(), nan.to_bits());
        assert_eq!(
            adapter.get_pointer_screen_position(),
            (f64::NEG_INFINITY, f64::INFINITY)
        );
    }
}
