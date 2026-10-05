use ferroui_base::input::PointerType;
use ferroui_base::platform::IPlatformSettings;
use ferroui_base::{FerroLocator, LocatorExtensions, Point, Rect, Size, Thickness};
use std::cell::Cell;

/// Detects double clicks on the window chrome.
#[derive(Default)]
pub(crate) struct DoubleClickHelper {
    click_count: Cell<i32>,
    last_click_rect: Cell<Rect>,
    last_click_time: Cell<u64>,
}

impl DoubleClickHelper {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn is_double_click(&self, timestamp: u64, p: Point) -> bool {
        let settings = FerroLocator::current().get_service::<dyn IPlatformSettings>();
        let double_click_time = settings
            .as_ref()
            .map_or(500.0, |settings| settings.get_double_tap_time(PointerType::Mouse).as_secs_f64() * 1000.0);
        let double_click_size =
            settings.as_ref().map_or(Size::new(4.0, 4.0), |settings| settings.get_double_tap_size(PointerType::Mouse));

        if !self.last_click_rect.get().contains(p)
            || timestamp.wrapping_sub(self.last_click_time.get()) as f64 > double_click_time
        {
            self.click_count.set(0);
        }

        self.click_count.set(self.click_count.get() + 1);
        self.last_click_time.set(timestamp);
        self.last_click_rect.set(
            Rect::from_position_size(p, Size::default())
                .inflate_thickness(Thickness::symmetric(double_click_size.width / 2.0, double_click_size.height / 2.0)),
        );

        self.click_count.get() == 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // No platform settings are registered in these tests: the defaults are
    // 500 ms and a 4x4 area.

    #[test]
    fn two_clicks_close_in_time_and_space_are_a_double_click() {
        let helper = DoubleClickHelper::new();
        assert!(!helper.is_double_click(1000, Point::new(10.0, 10.0)));
        assert!(helper.is_double_click(1200, Point::new(11.0, 9.0)));
        // The third click of a series is not a double click.
        assert!(!helper.is_double_click(1300, Point::new(11.0, 9.0)));
    }

    #[test]
    fn slow_clicks_are_not_a_double_click() {
        let helper = DoubleClickHelper::new();
        assert!(!helper.is_double_click(1000, Point::new(10.0, 10.0)));
        assert!(!helper.is_double_click(1501, Point::new(10.0, 10.0)));
        // The slow click starts a new series.
        assert!(helper.is_double_click(1600, Point::new(10.0, 10.0)));
    }

    #[test]
    fn distant_clicks_are_not_a_double_click() {
        let helper = DoubleClickHelper::new();
        assert!(!helper.is_double_click(1000, Point::new(10.0, 10.0)));
        assert!(!helper.is_double_click(1100, Point::new(13.0, 10.0)));
        assert!(helper.is_double_click(1200, Point::new(14.0, 11.0)));
    }
}
