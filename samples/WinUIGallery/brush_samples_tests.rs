//! The RadialGradientBrush, IconElement and Spacing pages: the source's examples with their code
//! behind (`brush_samples.rs`).

use crate::brush_samples::{design_image_uri, is_monochrome, radial_brush};
use crate::{examples, tests::start};
use ferroui_base::media::GradientSpreadMethod;
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{Point, Ref, RelativeUnit};
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::shapes::Rectangle;
use ferroui_controls::{Border, ComboBox, Control, Image, Slider, TextBlock};

fn near(actual: Point, x: f64, y: f64) -> bool {
    (actual.x - x).abs() < 1e-9 && (actual.y - y).abs() < 1e-9
}

fn page(id: &str) -> Ref<Control> {
    examples::create(examples::find(id).unwrap()).unwrap_or_else(|error| panic!("{id}: {error}"))
}

#[test]
fn radial_gradient_brush_sliders_and_boxes_set_the_brush() {
    let _application = start();
    let root = page("RadialGradientBrush");
    let brush = radial_brush(&root).expect("the rectangle's radial gradient brush");
    // InitializeSliders of RelativeToBoundingBox: every value 0.5 of the rectangle.
    assert!(near(brush.center().point, 0.5, 0.5));
    assert_eq!(brush.center().unit, RelativeUnit::Relative);
    assert_eq!(root.get_control::<Slider>("CenterXSlider").maximum(), 1.0);
    // OnSliderValueChanged.
    root.get_control::<Slider>("CenterXSlider").set_range_value(0.24);
    root.get_control::<Slider>("RadiusYSlider").set_range_value(0.3);
    root.get_control::<Slider>("OriginYSlider").set_range_value(0.1);
    assert!(near(brush.center().point, 0.24, 0.5));
    assert!((brush.radius_y().scalar - 0.3).abs() < 1e-9);
    assert!(near(brush.gradient_origin().point, 0.5, 0.1));
    // OnMappingModeChanged: Absolute, in pixels of the 200 by 200 rectangle.
    root.get_control::<ComboBox>("MappingModeComboBox")
        .set_selected_index(1);
    assert_eq!(root.get_control::<Slider>("RadiusXSlider").maximum(), 200.0);
    assert!(near(brush.center().point, 100.0, 100.0));
    assert_eq!(brush.center().unit, RelativeUnit::Absolute);
    assert_eq!(brush.radius_x().unit, RelativeUnit::Absolute);
    // OnSpreadMethodChanged.
    root.get_control::<ComboBox>("SpreadMethodComboBox")
        .set_selected_index(2);
    assert_eq!(brush.spread_method(), GradientSpreadMethod::Repeat);
}

#[test]
fn icon_element_monochrome_option_draws_the_bitmap_icon_in_one_color() {
    let _application = start();
    let root = page("IconElement");
    assert!(!is_monochrome(&root), "the markup's ShowAsMonochrome is False");
    assert_eq!(root.get_control::<Image>("SlicesImage").opacity(), 1.0);
    let check = root.get_control::<ToggleButton>("MonochromeButton");
    check.set_is_checked(Some(true));
    assert!(is_monochrome(&root));
    assert_eq!(root.get_control::<Image>("SlicesImage").opacity(), 0.0);
    assert!(root
        .get_control::<Rectangle>("SlicesMonochrome")
        .opacity_mask()
        .is_some());
    check.set_is_checked(Some(false));
    assert!(!is_monochrome(&root));
    assert!(root.get_control::<TextBlock>("SvgUnavailable").text().is_some());
}

#[test]
fn spacing_page_shows_the_spacing_table_and_the_theme_images() {
    let _application = start();
    let root = page("Spacing");
    for value in [4, 8, 12, 16, 24, 36, 48] {
        assert_eq!(
            root.get_control::<Border>(&format!("Spacing{value}")).width(),
            f64::from(value)
        );
    }
    assert!(design_image_uri("Cards", Some(&ThemeVariant::light())).ends_with("Cards.light.png"));
    assert!(design_image_uri("Dialog", Some(&ThemeVariant::dark())).ends_with("Dialog.dark.png"));
    assert!(
        design_image_uri("Cards", None).ends_with("Cards.dark.png"),
        "the Default dictionary"
    );
    let _ = root.get_control::<Image>("CardsImage");
    let _ = root.get_control::<Image>("DialogImage");
}
