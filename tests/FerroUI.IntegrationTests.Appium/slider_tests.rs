//! Port of `SliderTests.cs`.

use crate::infrastructure::{Actions, Collection, DefaultAppFixture, Session, TestBase, TestClass, TestPlatforms, Tests};

pub struct SliderTests {
    base: TestBase,
}

impl TestClass for SliderTests {}

/// `double.Parse(text, CultureInfo.InvariantCulture)`.
fn parse_double(text: &str) -> f64 {
    text.parse().unwrap_or_else(|error| panic!("{text:?}: {error}"))
}

impl SliderTests {
    pub fn new(fixture: &DefaultAppFixture) -> Self {
        let base = TestBase::new(fixture, "Slider");
        let reset = base.session().find_element_by_accessibility_id("ResetSliders");
        reset.click();
        SliderTests { base }
    }

    fn session(&self) -> &Session {
        self.base.session()
    }

    pub fn horizontal_changes_value_dragging_thumb_right(&self) {
        let slider = self.session().find_element_by_accessibility_id("HorizontalSlider");
        let thumb = slider.find_element_by_accessibility_id("thumb");
        let initial_thumb_rect = thumb.rect();

        Actions::new(self.session()).click_and_hold_on(&thumb).move_by_offset(100, 0).release().perform();

        let value = parse_double(&slider.text());
        let bound_value = parse_double(&self.session().find_element_by_accessibility_id("HorizontalSliderValue").text());

        assert!(value > 50.0);
        assert!((value - bound_value).abs() < 2.0, "Expected: {value}, Actual: {bound_value}");

        let current_thumb_rect = thumb.rect();
        assert!(current_thumb_rect.left() > initial_thumb_rect.left());
    }

    pub fn horizontal_changes_value_dragging_thumb_left(&self) {
        let slider = self.session().find_element_by_accessibility_id("HorizontalSlider");
        let thumb = slider.find_element_by_accessibility_id("thumb");
        let initial_thumb_rect = thumb.rect();

        Actions::new(self.session()).click_and_hold_on(&thumb).move_by_offset(-100, 0).release().perform();

        let value = parse_double(&slider.text());
        let bound_value = parse_double(&self.session().find_element_by_accessibility_id("HorizontalSliderValue").text());

        assert!(value < 50.0);
        assert!((value - bound_value).abs() < 2.0, "Expected: {value}, Actual: {bound_value}");

        let current_thumb_rect = thumb.rect();
        assert!(current_thumb_rect.left() < initial_thumb_rect.left());
    }

    pub fn horizontal_changes_value_when_clicking_increase_button(&self) {
        let slider = self.session().find_element_by_accessibility_id("HorizontalSlider");
        let thumb = slider.find_element_by_accessibility_id("thumb");
        let initial_thumb_rect = thumb.rect();

        Actions::new(self.session()).move_to_element_center(&slider, 100, 0).click().perform();

        let value = parse_double(&slider.text());
        let bound_value = parse_double(&self.session().find_element_by_accessibility_id("HorizontalSliderValue").text());

        assert!(value > 50.0);
        assert!((value - bound_value).abs() < 2.0, "Expected: {value}, Actual: {bound_value}");

        let current_thumb_rect = thumb.rect();
        assert!(current_thumb_rect.left() > initial_thumb_rect.left());
    }

    pub fn horizontal_changes_value_when_clicking_decrease_button(&self) {
        let slider = self.session().find_element_by_accessibility_id("HorizontalSlider");
        let thumb = slider.find_element_by_accessibility_id("thumb");
        let initial_thumb_rect = thumb.rect();

        Actions::new(self.session()).move_to_element_center(&slider, -100, 0).click().perform();

        let value = parse_double(&slider.text());
        let bound_value = parse_double(&self.session().find_element_by_accessibility_id("HorizontalSliderValue").text());

        assert!(value < 50.0);
        assert!((value - bound_value).abs() < 2.0, "Expected: {value}, Actual: {bound_value}");

        let current_thumb_rect = thumb.rect();
        assert!(current_thumb_rect.left() < initial_thumb_rect.left());
    }

    /// Not in upstream (DEVIATIONS.md, Automation tests): UI Automation reads the small and the
    /// large change of the slider (1 and 10, the defaults of `RangeBase`), where the reference
    /// answers 1 for both.
    pub fn horizontal_reports_its_small_and_large_change(&self) {
        let slider = self.session().find_element_by_accessibility_id("HorizontalSlider");
        let read = |name: &str| {
            parse_double(&slider.get_attribute(name).unwrap_or_else(|| panic!("the slider has no {name}")))
        };

        assert_eq!(1.0, read("RangeValue.SmallChange"));
        assert_eq!(10.0, read("RangeValue.LargeChange"));
    }
}

pub fn tests(tests: &mut Tests) {
    const FLAKY: &str = "Flaky test, slider value is sometimes off by 1 or 2 steps.";
    tests
        .class("slider_tests", Collection::Default, SliderTests::new)
        .add(
            "horizontal_changes_value_dragging_thumb_right",
            TestPlatforms::ALL,
            Some(FLAKY),
            SliderTests::horizontal_changes_value_dragging_thumb_right,
        )
        .add(
            "horizontal_changes_value_dragging_thumb_left",
            TestPlatforms::ALL,
            Some(FLAKY),
            SliderTests::horizontal_changes_value_dragging_thumb_left,
        )
        .fact(
            "horizontal_changes_value_when_clicking_increase_button",
            SliderTests::horizontal_changes_value_when_clicking_increase_button,
        )
        .fact(
            "horizontal_changes_value_when_clicking_decrease_button",
            SliderTests::horizontal_changes_value_when_clicking_decrease_button,
        )
        .add(
            "horizontal_reports_its_small_and_large_change",
            TestPlatforms::WINDOWS,
            None,
            SliderTests::horizontal_reports_its_small_and_large_change,
        );
}
