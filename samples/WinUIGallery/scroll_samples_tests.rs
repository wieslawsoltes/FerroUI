//! The ScrollViewer page: the source's example with its code behind (`scroll_samples.rs`).

use crate::scroll_samples::bar_visibility;
use crate::{examples, tests::start};
use ferroui_base::Ref;
use ferroui_controls::primitives::ScrollBarVisibility;
use ferroui_controls::{ComboBox, Control, ScrollViewer, Slider, ZoomMode};

fn page() -> Ref<Control> {
    examples::create(examples::find("ScrollViewer").unwrap()).unwrap_or_else(|error| panic!("ScrollViewer: {error}"))
}

#[test]
fn scroll_viewer_zoom_mode_option_enables_the_slider_and_binds_its_range() {
    let _application = start();
    let root = page();
    let viewer = root.get_control::<ScrollViewer>("ScrollViewerControl");
    let slider = root.get_control::<Slider>("ZoomSlider");
    // The x:Bind of the slider's range.
    assert_eq!(slider.minimum(), f64::from(viewer.min_zoom_factor()));
    assert_eq!(slider.maximum(), f64::from(viewer.max_zoom_factor()));
    // ZoomModeComboBox_SelectionChanged: Disabled turns the slider off.
    root.get_control::<ComboBox>("zoomCombo").set_selected_index(0);
    assert_eq!(viewer.zoom_mode(), ZoomMode::Disabled);
    assert!(!slider.is_enabled());
    root.get_control::<ComboBox>("zoomCombo").set_selected_index(1);
    assert_eq!(viewer.zoom_mode(), ZoomMode::Enabled);
    assert!(slider.is_enabled());
}

/// In a window with the WinUI theme, where the scroll viewer has its presenter: the constructor's
/// zoom, ZoomSlider_ValueChanged, ViewChanged and the zoom of the Disabled mode.
#[cfg(feature = "theme")]
#[test]
fn scroll_viewer_zoom_slider_and_zoom_factor_follow_each_other() {
    use ferroui_controls::Window;
    let _application = crate::tests::start_winui();
    let root = page();
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    window.update_layout();
    // The Loaded event of the scroll viewer, raised by a job of the dispatcher.
    ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);
    window.update_layout();
    let viewer = root.get_control::<ScrollViewer>("ScrollViewerControl");
    let slider = root.get_control::<Slider>("ZoomSlider");
    assert_eq!(viewer.zoom_factor(), 4.0);
    slider.set_range_value(3.0);
    window.update_layout();
    assert_eq!(viewer.zoom_factor(), 3.0);
    viewer.zoom_to_factor(5.0);
    window.update_layout();
    assert_eq!(slider.value(), 5.0);
    root.get_control::<ComboBox>("zoomCombo").set_selected_index(0);
    window.update_layout();
    assert_eq!(viewer.zoom_factor(), 2.0);
}

#[test]
fn scroll_viewer_scroll_modes_and_bar_visibilities_set_the_bars() {
    let _application = start();
    let root = page();
    let viewer = root.get_control::<ScrollViewer>("ScrollViewerControl");
    assert_eq!(viewer.horizontal_scroll_bar_visibility(), ScrollBarVisibility::Auto);
    root.get_control::<ComboBox>("hsbvCombo").set_selected_index(3);
    assert_eq!(viewer.horizontal_scroll_bar_visibility(), ScrollBarVisibility::Visible);
    root.get_control::<ComboBox>("hsmCombo").set_selected_index(0);
    assert_eq!(viewer.horizontal_scroll_bar_visibility(), ScrollBarVisibility::Disabled);
    root.get_control::<ComboBox>("hsmCombo").set_selected_index(2);
    assert_eq!(viewer.horizontal_scroll_bar_visibility(), ScrollBarVisibility::Visible);
    root.get_control::<ComboBox>("vsbvCombo").set_selected_index(2);
    assert_eq!(viewer.vertical_scroll_bar_visibility(), ScrollBarVisibility::Hidden);
    assert_eq!(bar_visibility(1, 0), ScrollBarVisibility::Disabled);
    assert_eq!(bar_visibility(0, 3), ScrollBarVisibility::Disabled);
}
