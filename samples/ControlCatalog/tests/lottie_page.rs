use super::support::start_catalog_compositor_application;
use crate::pages::LottiePage;
use ferroui_controls::primitives::RangeBase;
use ferroui_controls::testing::MockWindowingPlatform;
use ferroui_controls::{ComboBox, Control, Slider, Window};
use ferroui_controls_lottie::Lottie;

#[test]
fn lottie_page_loads_relative_assets_and_binds_the_scrubber() {
    let services = start_catalog_compositor_application();
    let page = LottiePage::new();
    let player = page.get_control::<Lottie>("Player");
    let scrubber = page.get_control::<Slider>("PositionSlider");
    let window_impl = MockWindowingPlatform::create_window_mock();
    services.setup(&window_impl);
    let window = Window::with_impl(window_impl);
    window.set_width(850.0);
    window.set_height(720.0);
    window.set_content(Some(Control::boxed(page.clone())));
    window.show();
    for _ in 0..3 {
        services.run_jobs();
    }

    assert_eq!(
        player.path().as_deref(),
        Some("../Assets/Lottie/Orbit.json")
    );
    assert_eq!(player.duration_seconds(), 4.0, "{:?}", player.status());
    assert_eq!(scrubber.maximum(), 4.0);
    player.pause();
    services.run_jobs();

    scrubber.set_current_value(RangeBase::value_property(), 2.0);
    services.run_jobs();
    assert_eq!(player.position_seconds(), 2.0);
    player.seek_progress(1.0);
    services.run_jobs();
    assert_eq!(scrubber.value(), 4.0);
    player.stop();
    services.run_jobs();
    assert_eq!(scrubber.value(), 0.0);

    page.get_control::<ComboBox>("Animation")
        .set_selected_index(1);
    services.run_jobs();
    assert_eq!(player.path().as_deref(), Some("/Assets/Lottie/Signal.json"));
    assert_eq!(player.duration_seconds(), 4.0);

    player.set_path(Some("/Assets/Lottie/missing.json".into()));
    services.run_jobs();
    assert_eq!(player.duration_seconds(), 0.0);
    assert_eq!(scrubber.maximum(), 0.0);
    assert!(player.status().unwrap().contains("Could not load"));

    player.set_path(Some("/Assets/Lottie/Orbit.json".into()));
    services.run_jobs();
    window.set_content(None);
    services.run_jobs();
    assert_eq!(player.duration_seconds(), 0.0);

    window.set_content(Some(Control::boxed(page)));
    for _ in 0..3 {
        services.run_jobs();
    }
    assert_eq!(player.duration_seconds(), 4.0);
    window.close();
}
