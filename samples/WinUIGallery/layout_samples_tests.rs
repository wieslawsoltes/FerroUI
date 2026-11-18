//! The ported Border, Canvas, Grid, StackPanel and RelativePanel pages: the source's options with
//! their handlers (`layout_samples.rs`) and x:Bind.

use crate::{examples, tests::start};
use ferroui_base::media::{Color, Colors, IBrush};
use ferroui_base::{Ref, Thickness};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{Border, Canvas, Control, Grid, RadioButton, Slider, StackPanel, Window};
use std::rc::Rc;

fn page(id: &str) -> Ref<Control> {
    examples::create(examples::find(id).unwrap()).unwrap_or_else(|error| panic!("{id}: {error}"))
}

fn color(brush: Option<Rc<dyn IBrush>>) -> Option<Color> {
    brush.and_then(|brush| brush.as_solid_color_brush().map(|brush| brush.color()))
}

#[test]
fn border_options_set_the_thickness_background_and_border_brush() {
    let _application = start();
    let root = page("Border");
    let border = root.get_control::<Border>("Control1");
    assert_eq!(
        border.border_thickness(),
        Thickness::new(2.0, 2.0, 2.0, 2.0)
    );
    root.get_control::<Slider>("ThicknessSlider")
        .set_range_value(5.0);
    assert_eq!(
        border.border_thickness(),
        Thickness::new(5.0, 5.0, 5.0, 5.0)
    );
    let backgrounds = [Colors::GREEN, Colors::YELLOW, Colors::BLUE, Colors::WHITE];
    let borders = [
        Colors::DARK_GREEN,
        Colors::GOLD,
        Colors::DARK_BLUE,
        Colors::WHITE,
    ];
    for (index, name) in crate::layout_samples::BORDER_COLORS.iter().enumerate() {
        root.get_control::<RadioButton>(&format!("Background{name}"))
            .set_is_checked(Some(true));
        assert_eq!(
            color(border.background()),
            Some(backgrounds[index]),
            "{name}"
        );
        root.get_control::<RadioButton>(&format!("BorderBrush{name}"))
            .set_is_checked(Some(true));
        assert_eq!(color(border.border_brush()), Some(borders[index]), "{name}");
    }
}

#[test]
fn canvas_options_move_the_red_rectangle_and_name_the_z_index_slider() {
    let _application = start();
    let root = page("Canvas");
    let rectangle = root.get_control::<Control>("RedRectangle");
    let z = root.get_control::<Slider>("ZSlider");
    assert_eq!(
        AutomationProperties::get_name(&z).as_deref(),
        Some("Canvas.ZIndex value 0 of range 0 to 4")
    );
    z.set_range_value(3.0);
    assert_eq!(rectangle.z_index(), 3);
    assert_eq!(
        AutomationProperties::get_name(&z).as_deref(),
        Some("Canvas.ZIndex value 3 of range 0 to 4")
    );
    root.get_control::<Slider>("LeftSlider")
        .set_range_value(30.0);
    root.get_control::<Slider>("TopSlider")
        .set_range_value(70.0);
    assert_eq!(
        (Canvas::get_left(&rectangle), Canvas::get_top(&rectangle)),
        (30.0, 70.0)
    );
}

#[test]
fn grid_options_bind_the_spacings_and_the_red_rectangle_cell() {
    let _application = start();
    let root = page("Grid");
    let grid = root.get_control::<Grid>("Control1");
    let rectangle = root.get_control::<Control>("Rectangle1");
    assert_eq!((grid.column_spacing(), grid.row_spacing()), (8.0, 8.0));
    assert_eq!(
        (Grid::get_column(&rectangle), Grid::get_row(&rectangle)),
        (0, 0)
    );
    root.get_control::<Slider>("ColumnSpacingSlider")
        .set_range_value(3.0);
    root.get_control::<Slider>("RowSpacingSlider")
        .set_range_value(12.0);
    assert_eq!((grid.column_spacing(), grid.row_spacing()), (3.0, 12.0));
    root.get_control::<Slider>("ColumnSlider")
        .set_range_value(2.0);
    root.get_control::<Slider>("RowSlider").set_range_value(1.0);
    assert_eq!(
        (Grid::get_column(&rectangle), Grid::get_row(&rectangle)),
        (2, 1)
    );
}

#[test]
fn stack_panel_options_set_the_orientation_and_the_spacing() {
    use ferroui_base::layout::Orientation;
    let _application = start();
    let root = page("StackPanel");
    let panel = root.get_control::<StackPanel>("Control1");
    assert_eq!(
        (panel.orientation(), panel.spacing()),
        (Orientation::Vertical, 8.0)
    );
    root.get_control::<RadioButton>("HorizontalOrientation")
        .set_is_checked(Some(true));
    assert_eq!(panel.orientation(), Orientation::Horizontal);
    root.get_control::<RadioButton>("VerticalOrientation")
        .set_is_checked(Some(true));
    assert_eq!(panel.orientation(), Orientation::Vertical);
    root.get_control::<Slider>("SpacingSlider")
        .set_range_value(12.0);
    assert_eq!(panel.spacing(), 12.0);
}

#[test]
fn relative_panel_places_the_rectangles_by_their_relations() {
    let _application = start();
    let root = page("RelativePanel");
    let window = Window::new();
    window.set_content(Some(Control::boxed(root.clone())));
    window.show();
    window.update_layout();
    let position = |name: &str| {
        let bounds = root.get_control::<Control>(name).bounds();
        (bounds.x, bounds.y)
    };
    // Red at the origin, blue right of it after its margin, green at the right edge of the
    // 300-wide panel, yellow below green (after its margin) and centred with it.
    assert_eq!(position("Rectangle1"), (0.0, 0.0));
    assert_eq!(position("Rectangle2"), (58.0, 0.0));
    assert_eq!(position("Rectangle3"), (250.0, 0.0));
    assert_eq!(position("Rectangle4"), (250.0, 58.0));
    window.close();
}
