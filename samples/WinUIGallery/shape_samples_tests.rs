//! The Shape, Line and Geometry pages: the source's examples, their x:Bind and the code behind of
//! the Geometry page (`shape_samples.rs`).

use crate::shape_samples::{header_image_uri, COPIED_RESOURCES};
use crate::{examples, tests::start};
use ferroui_base::interactivity::RoutedEventArgs;
use ferroui_base::media::{EllipseGeometry, GeometryGroup};
use ferroui_base::styling::ThemeVariant;
use ferroui_base::{Point, Ref};
use ferroui_controls::primitives::ToggleButton;
use ferroui_controls::shapes::{Ellipse, Line, Path, Polygon, Rectangle};
use ferroui_controls::{Button, Control, Flyout, Slider, TextBlock};

fn page(id: &str) -> Ref<Control> {
    examples::create(examples::find(id).unwrap()).unwrap_or_else(|error| panic!("{id}: {error}"))
}

fn slide(root: &Ref<Control>, name: &str, value: f64) {
    root.get_control::<Slider>(name).set_range_value(value);
}

#[test]
fn shape_page_sliders_size_and_stroke_the_ellipse_rectangle_and_polygon() {
    let _application = start();
    let root = page("Shape");
    let ellipse = root.get_control::<Ellipse>("Ellipse1");
    // The sliders start at their minimum, as WinUI's coerced Value.
    assert_eq!(
        (ellipse.width(), ellipse.height(), ellipse.stroke_thickness()),
        (100.0, 100.0, 2.0)
    );
    slide(&root, "slider1", 120.5);
    slide(&root, "slider2", 140.0);
    slide(&root, "slider3", 6.0);
    assert_eq!(
        (ellipse.width(), ellipse.height(), ellipse.stroke_thickness()),
        (140.0, 120.5, 6.0)
    );
    let rectangle = root.get_control::<Rectangle>("Rectangle1");
    slide(&root, "recSlider4", 20.0);
    slide(&root, "recSlider5", 30.0);
    slide(&root, "recSlider3", 4.0);
    assert_eq!(
        (rectangle.radius_x(), rectangle.radius_y(), rectangle.stroke_thickness()),
        (30.0, 20.0, 4.0)
    );
    slide(&root, "polySlider1", 8.0);
    assert_eq!(root.get_control::<Polygon>("Polygon1").stroke_thickness(), 8.0);
    let label = root.get_control::<TextBlock>("PolyPoint1");
    assert!(!label.is_visible());
    root.get_control::<ToggleButton>("ToggleSwitchPoly")
        .set_is_checked(Some(true));
    assert!(label.is_visible());
}

#[test]
fn line_page_sliders_move_the_line_and_stroke_the_shapes() {
    let _application = start();
    let root = page("Line");
    let line = root.get_control::<Line>("Line1");
    assert_eq!(
        (line.start_point(), line.end_point()),
        (Point::new(0.0, 0.0), Point::new(200.0, 0.0))
    );
    slide(&root, "lineSlider1", 10.0);
    slide(&root, "lineSlider2", 20.0);
    slide(&root, "lineSlider3", 250.0);
    slide(&root, "lineSlider4", 80.0);
    assert_eq!(
        (line.start_point(), line.end_point()),
        (Point::new(10.0, 20.0), Point::new(250.0, 80.0))
    );
    slide(&root, "lineSlider5", 7.5);
    assert_eq!(line.stroke_thickness(), 7.5);
    slide(&root, "pathSlider1", 9.0);
    assert_eq!(root.get_control::<Path>("Path1").stroke_thickness(), 9.0);
    for (switch, label) in [("ToggleSwitch2", "PolylinePoint1"), ("ToggleSwitch", "PathPoint1")] {
        assert!(!root.get_control::<TextBlock>(label).is_visible());
        root.get_control::<ToggleButton>(switch).set_is_checked(Some(true));
        assert!(root.get_control::<TextBlock>(label).is_visible(), "{label}");
    }
    // The TwoWay x:Bind of the radii of the group's ellipse.
    let group = root
        .get_control::<Path>("GeometryGroupPath")
        .data()
        .and_then(|data| data.cast::<GeometryGroup>())
        .expect("the geometry group");
    let ellipse = group
        .children()
        .to_vec()
        .into_iter()
        .find_map(|geometry| geometry.cast::<EllipseGeometry>())
        .expect("the ellipse of the group");
    slide(&root, "geogroupslider1", 35.0);
    slide(&root, "geogroupslider2", 45.0);
    assert_eq!((ellipse.radius_x(), ellipse.radius_y()), (35.0, 45.0));
}

#[test]
fn geometry_page_shows_the_theme_image_tips_and_copy_buttons() {
    let _application = start();
    let root = page("Geometry");
    assert!(header_image_uri(Some(&ThemeVariant::light())).ends_with("Geometry.light.png"));
    assert!(header_image_uri(Some(&ThemeVariant::dark())).ends_with("Geometry.dark.png"));
    assert!(
        header_image_uri(None).ends_with("Geometry.dark.png"),
        "the Default theme dictionary"
    );
    for (button, title) in [
        ("ShowGeometryButton1", "8px"),
        ("ShowGeometryButton2", "0px"),
        ("ShowGeometryButton3", "4px"),
    ] {
        let flyout = root
            .get_control::<Button>(button)
            .flyout()
            .and_then(|flyout| flyout.cast::<Flyout>())
            .unwrap_or_else(|| panic!("{button} has the tip"));
        let content = flyout.content().expect("the tip's content");
        let text = Control::from_boxed(&content).expect("a control");
        let first = text
            .cast::<TextBlock>()
            .or_else(|| {
                text.cast::<ferroui_controls::StackPanel>()
                    .and_then(|panel| panel.children().get(0).cast::<TextBlock>())
            })
            .expect("the tip's title");
        assert_eq!(first.text().as_deref(), Some(title));
    }
    // Outside a window there is no clipboard: the button says so.
    let (name, _) = COPIED_RESOURCES[1];
    root.get_control::<Button>(name)
        .raise_event(&RoutedEventArgs::with_event(&Button::click_event()));
    assert_eq!(
        root.get_control::<TextBlock>("CopyOutput").text().as_deref(),
        Some("This window has no clipboard service.")
    );
}
