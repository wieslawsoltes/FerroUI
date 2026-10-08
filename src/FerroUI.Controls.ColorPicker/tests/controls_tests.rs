//! Not from upstream: the behaviour of the controls without a template.

use crate::primitives::{ColorPreviewer, ColorSlider, ColorSpectrum};
use crate::*;
use ferroui_base::media::{Color, Colors, HsvColor};
use std::cell::RefCell;
use std::rc::Rc;

/// Records the colour changes an event reports.
fn recorder() -> (Rc<RefCell<Vec<(Color, Color)>>>, impl Fn(&ColorChangedEventArgs) + 'static) {
    let changes = Rc::new(RefCell::new(Vec::new()));
    let sink = changes.clone();
    (changes, move |e: &ColorChangedEventArgs| sink.borrow_mut().push((e.old_color(), e.new_color())))
}

#[test]
fn color_view_keeps_its_two_colors_in_sync() {
    let view = ColorView::new();
    let (changes, handler) = recorder();
    let _subscription = view.color_changed(handler);

    view.set_color(Colors::RED);
    assert_eq!(Colors::RED.to_hsv(), view.hsv_color());

    view.set_hsv_color(HsvColor::new(1.0, 240.0, 1.0, 1.0));
    assert_eq!(Colors::BLUE, view.color());

    assert_eq!(vec![(Colors::WHITE, Colors::RED), (Colors::RED, Colors::BLUE)], *changes.borrow());
}

#[test]
fn color_view_coerces_alpha_when_alpha_is_disabled() {
    let view = ColorView::new();
    view.set_color(Color::from_argb(0x40, 1, 2, 3));
    assert_eq!(0x40, view.color().a);

    view.set_is_alpha_enabled(false);
    assert_eq!(1.0, view.hsv_color().a);
    assert_eq!(0xFF, view.color().a);

    view.set_color(Color::from_argb(0x10, 4, 5, 6));
    assert_eq!(Color::from_argb(0xFF, 4, 5, 6), view.color());
}

#[test]
fn color_view_fills_the_palette_colors_from_the_palette() {
    let view = ColorView::new();
    assert_eq!(4, view.palette_column_count());
    assert!(view.palette_colors().is_none());

    let palette: Rc<dyn IColorPalette> = Rc::new(SixteenColorPalette::new());
    view.set_palette(Some(palette.clone()));

    assert_eq!(8, view.palette_column_count());
    let colors = view.palette_colors().expect("the palette colors");
    assert_eq!(16, colors.count());
    // Shade by shade, color by color.
    assert_eq!(palette.get_color(0, 0), colors.get(0));
    assert_eq!(palette.get_color(1, 0), colors.get(1));
    assert_eq!(palette.get_color(0, 1), colors.get(8));
}

#[test]
fn color_picker_is_a_color_view_with_content() {
    let picker = ColorPicker::new();
    assert_eq!(ColorViewTab::Spectrum as i32, picker.selected_index());

    let content: ferroui_base::BoxedValue = Rc::new("content".to_string());
    picker.set_content(Some(content.clone()));
    assert!(Rc::ptr_eq(&content, &picker.content().unwrap()));

    picker.set_color(Colors::LIME);
    assert_eq!(Colors::LIME.to_hsv(), picker.hsv_color());
}

#[test]
fn color_spectrum_keeps_its_two_colors_in_sync() {
    let spectrum = ColorSpectrum::new();
    let (changes, handler) = recorder();
    let _subscription = spectrum.color_changed(handler);

    spectrum.set_color(Colors::RED);
    assert_eq!(Colors::RED.to_hsv(), spectrum.hsv_color());

    spectrum.set_hsv_color(HsvColor::new(1.0, 120.0, 1.0, 1.0));
    assert_eq!(Colors::LIME, spectrum.color());

    assert_eq!(vec![(Colors::WHITE, Colors::RED), (Colors::RED, Colors::LIME)], *changes.borrow());
}

#[test]
fn color_spectrum_computes_the_third_component() {
    let spectrum = ColorSpectrum::new();
    assert_eq!(ColorComponent::Component3, spectrum.third_component());

    spectrum.set_components(ColorSpectrumComponents::HueValue);
    assert_eq!(ColorComponent::Component2, spectrum.third_component());

    spectrum.set_components(ColorSpectrumComponents::ValueSaturation);
    assert_eq!(ColorComponent::Component1, spectrum.third_component());
}

#[test]
#[should_panic(expected = "MinHue must be between 0 and 359.")]
fn color_spectrum_rejects_a_hue_out_of_range() {
    ColorSpectrum::new().set_min_hue(400);
}

#[test]
fn color_slider_maps_the_component_to_its_range() {
    let slider = ColorSlider::new();
    slider.set_color_model(ColorModel::Hsva);
    slider.set_color_component(ColorComponent::Component1);
    slider.set_hsv_color(HsvColor::new(1.0, 200.0, 0.5, 0.5));

    assert_eq!(0.0, slider.minimum());
    assert_eq!(359.0, slider.maximum());
    assert_eq!(200.0, slider.value());

    slider.set_color_component(ColorComponent::Component2);
    assert_eq!(100.0, slider.maximum());
    assert_eq!(50.0, slider.value());

    // Moving the slider changes the color.
    slider.set_range_value(25.0);
    assert!((slider.hsv_color().s - 0.25).abs() < 1e-12);
    assert_eq!(slider.hsv_color().to_rgb(), slider.color());
}

#[test]
fn color_slider_in_rgb_model_uses_bytes() {
    let slider = ColorSlider::new();
    slider.set_color_model(ColorModel::Rgba);
    slider.set_color_component(ColorComponent::Component3);
    slider.set_color(Color::from_argb(255, 10, 20, 30));

    assert_eq!(255.0, slider.maximum());
    assert_eq!(30.0, slider.value());

    slider.set_range_value(255.0);
    assert_eq!(Color::from_argb(255, 10, 20, 255), slider.color());
}

#[test]
fn color_previewer_reports_color_changes() {
    let previewer = ColorPreviewer::new();
    let (changes, handler) = recorder();
    let _subscription = previewer.color_changed(handler);

    previewer.set_hsv_color(Colors::RED.to_hsv());

    assert_eq!(vec![(Colors::TRANSPARENT.to_hsv().to_rgb(), Colors::RED)], *changes.borrow());
}
