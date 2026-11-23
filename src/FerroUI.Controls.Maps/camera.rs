//! Where the map looks, and how it keeps moving after a gesture.

use crate::projection::{pan, zoom_about, BasicGeoposition};

/// The share of its speed a flick keeps after one second.
const FLICK_RETAINED_PER_SECOND: f64 = 0.02;
/// Below this speed, in pixels per second, a flick has ended.
const FLICK_STOP_SPEED: f64 = 12.0;
/// A flick slower than this does not start.
const FLICK_START_SPEED: f64 = 60.0;
/// The fastest flick, in pixels per second.
const FLICK_MAX_SPEED: f64 = 6000.0;
/// The share of the remaining zoom a zoom animation has left after one second.
const ZOOM_REMAINING_PER_SECOND: f64 = 0.000_1;
/// A zoom animation this close to its target ends on it.
const ZOOM_EPSILON: f64 = 0.001;

/// The lowest and highest zoom levels of the control.
pub const MIN_ZOOM: f64 = 0.0;
pub const MAX_ZOOM: f64 = 22.0;

/// The center and zoom of a map, with the motion left from the last gesture.
#[derive(Clone, Debug, PartialEq)]
pub struct MapCamera {
    center: BasicGeoposition,
    zoom: f64,
    min_zoom: f64,
    max_zoom: f64,
    velocity: (f64, f64),
    zoom_target: Option<ZoomTarget>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ZoomTarget {
    zoom: f64,
    anchor: (f64, f64),
}

impl Default for MapCamera {
    fn default() -> Self {
        Self::new(BasicGeoposition::default(), 2.0)
    }
}

impl MapCamera {
    pub fn new(center: BasicGeoposition, zoom: f64) -> Self {
        let mut camera = Self {
            center: center.normalized(),
            zoom: MIN_ZOOM,
            min_zoom: MIN_ZOOM,
            max_zoom: MAX_ZOOM,
            velocity: (0.0, 0.0),
            zoom_target: None,
        };
        camera.zoom = camera.limit(zoom);
        camera
    }

    fn limit(&self, zoom: f64) -> f64 {
        if zoom.is_nan() {
            self.min_zoom
        } else {
            zoom.clamp(self.min_zoom, self.max_zoom)
        }
    }

    pub fn center(&self) -> BasicGeoposition {
        self.center
    }

    pub fn zoom(&self) -> f64 {
        self.zoom
    }

    /// The zoom level a running animation ends on, or the current one.
    pub fn target_zoom(&self) -> f64 {
        self.zoom_target.map_or(self.zoom, |target| target.zoom)
    }

    /// Whether [`step`](Self::step) still has something to do.
    pub fn is_moving(&self) -> bool {
        self.velocity != (0.0, 0.0) || self.zoom_target.is_some()
    }

    /// Limits the zoom. The range is put in order and kept within what the control supports.
    pub fn set_zoom_range(&mut self, min_zoom: f64, max_zoom: f64) {
        let first = if min_zoom.is_nan() {
            MIN_ZOOM
        } else {
            min_zoom.clamp(MIN_ZOOM, MAX_ZOOM)
        };
        let second = if max_zoom.is_nan() {
            MAX_ZOOM
        } else {
            max_zoom.clamp(MIN_ZOOM, MAX_ZOOM)
        };
        self.min_zoom = first.min(second);
        self.max_zoom = first.max(second);
        self.zoom = self.limit(self.zoom);
        if let Some(target) = &mut self.zoom_target {
            target.zoom = target.zoom.clamp(self.min_zoom, self.max_zoom);
        }
    }

    /// Goes to a place at once, ending any motion.
    pub fn set_view(&mut self, center: BasicGeoposition, zoom: f64) {
        self.stop();
        self.center = center.normalized();
        self.zoom = self.limit(zoom);
    }

    /// Ends any motion where it is.
    pub fn stop(&mut self) {
        self.velocity = (0.0, 0.0);
        self.zoom_target = None;
    }

    /// Moves the content by a number of pixels, as a drag does.
    pub fn drag(&mut self, dx: f64, dy: f64) {
        self.velocity = (0.0, 0.0);
        self.center = pan(self.center, self.zoom, dx, dy);
    }

    /// Lets go of a drag at a speed in pixels per second.
    pub fn flick(&mut self, vx: f64, vy: f64) {
        let speed = vx.hypot(vy);
        if !speed.is_finite() || speed < FLICK_START_SPEED {
            self.velocity = (0.0, 0.0);
            return;
        }
        let scale = (FLICK_MAX_SPEED / speed).min(1.0);
        self.velocity = (vx * scale, vy * scale);
    }

    /// Zooms at once, keeping the place under `anchor`, in pixels from the center of the view.
    pub fn zoom_at(&mut self, zoom: f64, anchor: (f64, f64)) {
        self.zoom_target = None;
        let zoom = self.limit(zoom);
        self.center = zoom_about(self.center, self.zoom, zoom, anchor);
        self.zoom = zoom;
    }

    /// Starts or retargets a smooth zoom that keeps the place under `anchor`.
    pub fn animate_zoom(&mut self, zoom: f64, anchor: (f64, f64)) {
        self.velocity = (0.0, 0.0);
        let zoom = self.limit(zoom);
        if (zoom - self.zoom).abs() <= ZOOM_EPSILON {
            self.zoom_at(zoom, anchor);
        } else {
            self.zoom_target = Some(ZoomTarget { zoom, anchor });
        }
    }

    /// Advances the motion by `seconds`. Returns whether the view changed.
    pub fn step(&mut self, seconds: f64) -> bool {
        if !(seconds > 0.0) || !seconds.is_finite() || !self.is_moving() {
            return false;
        }
        if let Some(target) = self.zoom_target {
            let remaining = (target.zoom - self.zoom) * ZOOM_REMAINING_PER_SECOND.powf(seconds);
            let zoom = if remaining.abs() <= ZOOM_EPSILON {
                target.zoom
            } else {
                target.zoom - remaining
            };
            self.center = zoom_about(self.center, self.zoom, zoom, target.anchor);
            self.zoom = zoom;
            if zoom == target.zoom {
                self.zoom_target = None;
            }
        }
        if self.velocity != (0.0, 0.0) {
            // The distance is the integral of the decay over the step, so that the path of a
            // flick does not depend on the frame rate.
            let decay = FLICK_RETAINED_PER_SECOND.powf(seconds);
            let travelled = (decay - 1.0) / FLICK_RETAINED_PER_SECOND.ln();
            self.center = pan(
                self.center,
                self.zoom,
                self.velocity.0 * travelled,
                self.velocity.1 * travelled,
            );
            self.velocity = (self.velocity.0 * decay, self.velocity.1 * decay);
            if self.velocity.0.hypot(self.velocity.1) < FLICK_STOP_SPEED {
                self.velocity = (0.0, 0.0);
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::project;

    fn near(a: f64, b: f64, tolerance: f64) {
        assert!((a - b).abs() <= tolerance, "{a} is not {b}");
    }

    fn place() -> BasicGeoposition {
        BasicGeoposition {
            latitude: 48.0,
            longitude: 16.0,
        }
    }

    #[test]
    fn the_zoom_stays_in_its_range() {
        let mut camera = MapCamera::new(place(), 40.0);
        assert_eq!(camera.zoom(), MAX_ZOOM);
        camera.set_zoom_range(12.0, 3.0);
        assert_eq!(camera.zoom(), 12.0);
        camera.zoom_at(1.0, (0.0, 0.0));
        assert_eq!(camera.zoom(), 3.0);
        camera.set_view(place(), f64::NAN);
        assert_eq!(camera.zoom(), 3.0);
    }

    #[test]
    fn a_drag_moves_the_center_the_other_way() {
        let mut camera = MapCamera::new(place(), 6.0);
        let (x, y) = project(camera.center(), 6.0);
        camera.drag(40.0, -25.0);
        let (moved_x, moved_y) = project(camera.center(), 6.0);
        near(moved_x, x - 40.0, 1e-6);
        near(moved_y, y + 25.0, 1e-6);
    }

    #[test]
    fn a_slow_release_does_not_flick() {
        let mut camera = MapCamera::new(place(), 6.0);
        camera.flick(10.0, 10.0);
        assert!(!camera.is_moving());
        camera.flick(f64::INFINITY, 0.0);
        assert!(!camera.is_moving());
    }

    #[test]
    fn a_flick_slows_down_and_ends() {
        let mut camera = MapCamera::new(place(), 6.0);
        camera.flick(1000.0, 0.0);
        let mut steps = 0;
        while camera.is_moving() {
            assert!(camera.step(1.0 / 60.0));
            steps += 1;
            assert!(steps < 600, "the flick does not end");
        }
        assert!(!camera.step(1.0 / 60.0));
        assert!(camera.center().longitude < place().longitude);
    }

    #[test]
    fn a_flick_goes_as_far_at_any_frame_rate() {
        let run = |rate: u32| {
            let mut camera = MapCamera::new(place(), 6.0);
            camera.flick(1500.0, -600.0);
            for _ in 0..rate / 2 {
                camera.step(1.0 / f64::from(rate));
            }
            project(camera.center(), 6.0)
        };
        let (slow, fast) = (run(30), run(240));
        near(slow.0, fast.0, 1e-3);
        near(slow.1, fast.1, 1e-3);
    }

    #[test]
    fn a_smooth_zoom_ends_on_its_target_with_the_place_held() {
        let mut camera = MapCamera::new(place(), 5.0);
        let anchor = (100.0, 60.0);
        let (x, y) = project(camera.center(), 5.0);
        let held = crate::projection::unproject(x + anchor.0, y + anchor.1, 5.0);
        camera.animate_zoom(8.0, anchor);
        assert_eq!(camera.target_zoom(), 8.0);
        let mut steps = 0;
        while camera.is_moving() {
            camera.step(1.0 / 60.0);
            steps += 1;
            assert!(steps < 600, "the zoom does not end");
        }
        assert_eq!(camera.zoom(), 8.0);
        let (x, y) = project(camera.center(), 8.0);
        let after = crate::projection::unproject(x + anchor.0, y + anchor.1, 8.0);
        near(after.latitude, held.latitude, 1e-6);
        near(after.longitude, held.longitude, 1e-6);
    }

    #[test]
    fn a_new_gesture_ends_the_motion() {
        let mut camera = MapCamera::new(place(), 6.0);
        camera.flick(1000.0, 0.0);
        camera.drag(1.0, 0.0);
        assert!(!camera.is_moving());
        camera.animate_zoom(9.0, (0.0, 0.0));
        camera.stop();
        assert!(!camera.is_moving());
        assert_eq!(camera.target_zoom(), camera.zoom());
    }

    #[test]
    fn a_step_of_no_time_changes_nothing() {
        let mut camera = MapCamera::new(place(), 6.0);
        camera.flick(1000.0, 0.0);
        let before = camera.clone();
        assert!(!camera.step(0.0));
        assert!(!camera.step(f64::NAN));
        assert_eq!(camera, before);
    }
}
