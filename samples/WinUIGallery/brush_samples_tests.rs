//! The RadialGradientBrush, IconElement, Spacing and Typography pages: the source's examples with
//! their code behind (`brush_samples.rs`).

use crate::brush_samples::{design_image_uri, is_monochrome, radial_brush, TYPE_RAMP};
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

#[test]
fn typography_page_shows_the_type_ramp_the_theme_image_and_the_tips() {
    use ferroui_base::interactivity::RoutedEventArgs;
    use ferroui_controls::{Button, Flyout, SelectableTextBlock};
    let _application = start();
    let root = page("Typography");
    assert!(design_image_uri("Typography", Some(&ThemeVariant::light())).ends_with("Typography.light.png"));
    assert!(design_image_uri("Typography", None).ends_with("Typography.dark.png"));
    // The theme's image where the platform decodes it (the test platform may not).
    let decoded = crate::assets::load_bitmap(&design_image_uri("Typography", None)).is_ok();
    assert_eq!(root.get_control::<Image>("HeaderImage").source().is_some(), decoded);
    // The five tips of ShowTypographyButtonClick1 to 5, the Flyouts of their buttons.
    for (index, title) in ["Caption", "Body", "Body Strong", "Title", "Display"].into_iter().enumerate() {
        let button = format!("ShowTypographyButton{}", index + 1);
        let flyout = root
            .get_control::<Button>(&button)
            .flyout()
            .and_then(|flyout| flyout.cast::<Flyout>())
            .unwrap_or_else(|| panic!("{button} has the tip"));
        let content = flyout.content().expect("the tip's content");
        let text = Control::from_boxed(&content)
            .and_then(|control| control.cast::<TextBlock>())
            .expect("the tip's title");
        assert_eq!(text.text().as_deref(), Some(title));
    }
    // The nine rows of the type ramp: the example and its resource name.
    for (example, resource, _, _) in TYPE_RAMP {
        assert_eq!(
            root.get_control::<TextBlock>(&format!("{resource}Example")).text().as_deref(),
            Some(example)
        );
        let names = root
            .get_logical_descendants()
            .filter_map(|control| control.cast::<SelectableTextBlock>())
            .filter(|name| name.text().as_deref() == Some(resource))
            .count();
        assert_eq!(names, 1, "{resource}");
    }
    // Outside a window there is no clipboard: the copy button says so.
    root.get_control::<Button>("CopyBodyTextBlockStyle")
        .raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
    assert_eq!(
        root.get_control::<TextBlock>("CopyOutput").text().as_deref(),
        Some("This window has no clipboard service.")
    );
}

#[cfg(feature = "theme")]
#[test]
fn typography_examples_take_the_text_styles_of_the_winui_theme() {
    use ferroui_controls::Window;
    let _application = crate::tests::start_winui();
    let root = page("Typography");
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    window.update_layout();
    // Each example's style is the ControlTheme of its resource name, found as a dynamic resource
    // once the page is in the window: the sizes of the type ramp.
    for ((_, resource, size, _), expected) in TYPE_RAMP
        .into_iter()
        .zip([12.0, 14.0, 14.0, 18.0, 18.0, 20.0, 28.0, 40.0, 68.0])
    {
        let example = root.get_control::<TextBlock>(&format!("{resource}Example"));
        assert!(example.theme().is_some(), "{resource}");
        assert_eq!(example.font_size(), expected, "{resource} is {size}");
    }
    window.close();
}
