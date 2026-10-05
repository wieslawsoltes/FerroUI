use super::{IWindowBaseImpl, Screen};
use ferroui_base::utilities::MathUtilities;
use ferroui_base::{PixelPoint, PixelRect, PixelSize};
use std::rc::Rc;

/// Finds the screen of a point, a rectangle or a window in a list of
/// screens.
pub struct ScreenHelper;

impl ScreenHelper {
    /// The first screen whose bounds contain `point`.
    pub fn screen_from_point(point: PixelPoint, screens: &[Rc<Screen>]) -> Option<Rc<Screen>> {
        screens.iter().find(|screen| screen.bounds().contains_exclusive(point)).cloned()
    }

    /// The screen that the largest part of `bounds` is on.
    pub fn screen_from_rect(bounds: PixelRect, screens: &[Rc<Screen>]) -> Option<Rc<Screen>> {
        let mut curr_max_screen = None;
        let mut max_area_size = 0.0;

        for screen in screens {
            let screen_bounds = screen.bounds();
            let (screen_left, screen_top) = (f64::from(screen_bounds.x), f64::from(screen_bounds.y));
            let screen_right = f64::from(screen_bounds.x) + f64::from(screen_bounds.width);
            let screen_bottom = f64::from(screen_bounds.y) + f64::from(screen_bounds.height);

            let left = MathUtilities::clamp(f64::from(bounds.x), screen_left, screen_right);
            let top = MathUtilities::clamp(f64::from(bounds.y), screen_top, screen_bottom);
            let right = MathUtilities::clamp(f64::from(bounds.x) + f64::from(bounds.width), screen_left, screen_right);
            let bottom =
                MathUtilities::clamp(f64::from(bounds.y) + f64::from(bounds.height), screen_top, screen_bottom);
            let area = (right - left) * (bottom - top);
            if area > max_area_size {
                max_area_size = area;
                curr_max_screen = Some(screen);
            }
        }

        curr_max_screen.cloned()
    }

    /// The screen that the largest part of `window` is on.
    pub fn screen_from_window(window: &dyn IWindowBaseImpl, screens: &[Rc<Screen>]) -> Option<Rc<Screen>> {
        let rect = PixelRect::from_position_size(
            window.position(),
            PixelSize::from_size(window.frame_size().unwrap_or_else(|| window.client_size()), window.desktop_scaling()),
        );

        Self::screen_from_rect(rect, screens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn screens() -> Vec<Rc<Screen>> {
        let first = Rc::new(Screen::new(None));
        first.set_bounds(PixelRect::new(0, 0, 100, 100));
        let second = Rc::new(Screen::new(None));
        second.set_bounds(PixelRect::new(100, 0, 200, 100));
        vec![first, second]
    }

    #[test]
    fn screen_from_point_uses_exclusive_bounds() {
        let screens = screens();
        assert!(Rc::ptr_eq(&ScreenHelper::screen_from_point(PixelPoint::new(0, 0), &screens).unwrap(), &screens[0]));
        assert!(Rc::ptr_eq(&ScreenHelper::screen_from_point(PixelPoint::new(99, 99), &screens).unwrap(), &screens[0]));
        assert!(Rc::ptr_eq(&ScreenHelper::screen_from_point(PixelPoint::new(100, 0), &screens).unwrap(), &screens[1]));
        assert!(ScreenHelper::screen_from_point(PixelPoint::new(300, 0), &screens).is_none());
        assert!(ScreenHelper::screen_from_point(PixelPoint::new(0, 100), &screens).is_none());
        assert!(ScreenHelper::screen_from_point(PixelPoint::new(0, 0), &[]).is_none());
    }

    #[test]
    fn screen_from_rect_picks_the_largest_intersection() {
        let screens = screens();
        let mostly_first = PixelRect::new(40, 10, 80, 50);
        assert!(Rc::ptr_eq(&ScreenHelper::screen_from_rect(mostly_first, &screens).unwrap(), &screens[0]));

        let mostly_second = PixelRect::new(80, 10, 80, 50);
        assert!(Rc::ptr_eq(&ScreenHelper::screen_from_rect(mostly_second, &screens).unwrap(), &screens[1]));

        // On a tie the first screen wins.
        let tie = PixelRect::new(60, 10, 80, 50);
        assert!(Rc::ptr_eq(&ScreenHelper::screen_from_rect(tie, &screens).unwrap(), &screens[0]));

        assert!(ScreenHelper::screen_from_rect(PixelRect::new(1000, 1000, 10, 10), &screens).is_none());
        assert!(ScreenHelper::screen_from_rect(PixelRect::new(10, 10, 0, 0), &screens).is_none());
    }
}
