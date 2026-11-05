//! Not from upstream: the colour helper and the increment helpers.

use crate::helpers::color_picker_helpers::ColorPickerHelpers;
use crate::helpers::hsv::Hsv;
use crate::helpers::increment_amount::IncrementAmount;
use crate::helpers::increment_direction::IncrementDirection;
use crate::primitives::ColorHelper;
use crate::{ColorComponent, ColorModel, HsvComponent};
use ferroui_base::layout::Orientation;
use ferroui_base::media::{Color, Colors, HsvColor};
use ferroui_base::utilities::{current_ui_locale, set_current_ui_locale, Locale};

/// Sets the UI culture of the thread and restores it when dropped.
struct UiCultureScope(Locale);

impl UiCultureScope {
    fn new(name: &str) -> Self {
        let previous = current_ui_locale();
        set_current_ui_locale(Locale::try_from_str(name).unwrap());
        Self(previous)
    }
}

impl Drop for UiCultureScope {
    fn drop(&mut self) {
        set_current_ui_locale(self.0.clone());
    }
}

#[test]
fn relative_luminance_orders_black_and_white() {
    assert!(ColorHelper::get_relative_luminance(Colors::BLACK) < 0.01);
    assert!(ColorHelper::get_relative_luminance(Colors::WHITE) > 0.99);
    assert!(ColorHelper::get_relative_luminance(Colors::LIME) > ColorHelper::get_relative_luminance(Colors::BLUE));
}

#[test]
fn display_names_need_an_english_ui_culture() {
    {
        let _scope = UiCultureScope::new("en-US");
        assert!(ColorHelper::to_display_name_exists());
    }
    {
        let _scope = UiCultureScope::new("de-DE");
        assert!(!ColorHelper::to_display_name_exists());
    }
}

#[test]
fn display_names_are_the_closest_known_colors() {
    assert_eq!("Red", ColorHelper::to_display_name(Colors::RED));
    assert_eq!("White", ColorHelper::to_display_name(Colors::WHITE));
    assert_eq!("Transparent", ColorHelper::to_display_name(Color::from_argb(0, 10, 20, 30)));
    // PascalCase names are split into words.
    assert_eq!("Deep Sky Blue", ColorHelper::to_display_name(Colors::DEEP_SKY_BLUE));
    // A second call is answered from the cache with the same name.
    assert_eq!("Deep Sky Blue", ColorHelper::to_display_name(Colors::DEEP_SKY_BLUE));
}

#[test]
fn small_increments_change_a_component_by_one_unit_and_wrap_at_the_bounds() {
    let hsv = Hsv::new(10.0, 0.5, 0.5);

    let hue = ColorPickerHelpers::increment_color_component(
        hsv,
        HsvComponent::Hue,
        IncrementDirection::Higher,
        IncrementAmount::Small,
        true,
        0.0,
        359.0,
    );
    assert_eq!(11.0, hue.h);

    let saturation = ColorPickerHelpers::increment_color_component(
        hsv,
        HsvComponent::Saturation,
        IncrementDirection::Lower,
        IncrementAmount::Small,
        true,
        0.0,
        100.0,
    );
    assert!((saturation.s - 0.49).abs() < 1e-12);

    let wrapped = ColorPickerHelpers::increment_color_component(
        Hsv::new(359.0, 0.5, 0.5),
        HsvComponent::Hue,
        IncrementDirection::Higher,
        IncrementAmount::Small,
        true,
        0.0,
        359.0,
    );
    assert_eq!(0.0, wrapped.h);

    let clamped = ColorPickerHelpers::increment_color_component(
        Hsv::new(358.5, 0.5, 0.5),
        HsvComponent::Hue,
        IncrementDirection::Higher,
        IncrementAmount::Small,
        true,
        0.0,
        359.0,
    );
    assert_eq!(359.0, clamped.h);
}

#[test]
fn large_increments_find_the_next_named_color() {
    let _scope = UiCultureScope::new("en-US");
    let red = Hsv::new(0.0, 1.0, 1.0);

    let next = ColorPickerHelpers::increment_color_component(
        red,
        HsvComponent::Hue,
        IncrementDirection::Higher,
        IncrementAmount::Large,
        true,
        0.0,
        359.0,
    );

    assert!(next.h > 0.0 && next.h < 60.0, "{next:?}");
    assert_ne!("Red", ColorHelper::to_display_name(next.to_rgb().to_color(1.0)));
}

#[test]
fn rgb_and_hsv_round_trip() {
    let hsv = crate::helpers::rgb::Rgb::from_color(Colors::ORANGE).to_hsv();
    assert_eq!(Colors::ORANGE, hsv.to_rgb().to_color(1.0));
    assert_eq!(HsvColor::from_ahsv(0.5, hsv.h, hsv.s, hsv.v), hsv.to_hsv_color(0.5));
}

#[test]
fn component_bitmaps_sweep_the_component() {
    let _dispatcher = ferroui_base::threading::Dispatcher::unit_test_scope();
    let width = 4;
    let height = 2;
    let mut data = vec![0u8; (width * height * 4) as usize];

    let mut task = ferroui_base::threading::Dispatcher::ui_thread().start_local(
        ColorPickerHelpers::create_component_bitmap_async(
            data.clone(),
            width,
            height,
            Orientation::Horizontal,
            ColorModel::Rgba,
            ColorComponent::Component1,
            Colors::WHITE.to_hsv(),
            false,
            true,
        ),
    );
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    data = match std::future::Future::poll(std::pin::Pin::new(&mut task), &mut context) {
        std::task::Poll::Ready(outcome) => outcome.expect("the task result").expect("the bitmap data"),
        std::task::Poll::Pending => panic!("the task has not ended"),
    };

    // Perceptive red sweep: blue and green are zero, red grows from the left.
    let reds: Vec<u8> = (0..width as usize).map(|x| data[x * 4 + 2]).collect();
    assert_eq!(vec![0, 64, 128, 191], reds);
    assert!(data.chunks(4).all(|pixel| pixel[0] == 0 && pixel[1] == 0 && pixel[3] == 255));
    // The second row repeats the first.
    assert_eq!(data[..16], data[16..]);
}
