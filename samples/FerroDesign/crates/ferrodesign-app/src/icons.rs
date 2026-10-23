use ferroui_base::media::immutable::{
    ImmutableGradientStop, ImmutableLinearGradientBrush, ImmutableSolidColorBrush,
};
use ferroui_base::media::{Color, IBrush, StreamGeometry, Stretch};
use ferroui_base::Ref;
use ferroui_controls::shapes::Path;
use std::rc::Rc;

pub fn brush(hex: &str) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::new(
        Color::parse(hex).unwrap_or(Color::from_rgb(32, 32, 32)),
    ))
}
pub fn icon(name: &str, color: &str, size: f64) -> Ref<Path> {
    let data = match name {
        "logo" => "M4,2 H18 V6 H8 V10 H16 V14 H8 V22 H4 Z",
        "pointer" => "M5,2 L20,12 L13,14 L10,21 Z",
        "frame" => "M7,2 V22 M17,2 V22 M2,7 H22 M2,17 H22",
        "rect" | "gradient" => "M4,4 H20 V20 H4 Z",
        "chevron-down" => "M5,8 L12,15 L19,8",
        "chevron-right" => "M8,5 L15,12 L8,19",
        "ellipse" => "M12,3 A9,9 0 1 1 11.99,3 Z",
        "line" => "M4,20 L20,4",
        "pen" => "M3,21 L7,8 L17,3 L21,7 L16,17 Z M3,21 L12,12 M10,10 L14,14",
        "text" => "M4,5 V3 H20 V5 M12,3 V21 M8,21 H16",
        "hand" => "M8,13 V6 Q8,3 10,5 V11 V3 Q12,1 13,4 V11 V5 Q16,3 16,6 V12 V8 Q19,6 19,10 V16 Q19,22 13,22 H11 Q8,22 6,18 L3,13 Q3,10 5,12 Z",
        "comment" => "M4,3 H20 V17 H10 L4,22 Z M8,8 H16 M8,12 H14",
        "actions" => "M12,2 L15,9 L22,12 L15,15 L12,22 L9,15 L2,12 L9,9 Z",
        "file" => "M5,2 H14 L20,8 V22 H5 Z M14,2 V8 H20",
        "assets" | "component" => "M12,2 L16,6 L12,10 L8,6 Z M6,8 L10,12 L6,16 L2,12 Z M18,8 L22,12 L18,16 L14,12 Z M12,14 L16,18 L12,22 L8,18 Z",
        "tools" => "M3,3 H9 V9 H3 Z M15,3 H21 V9 H15 Z M3,15 H9 V21 H3 Z M15,15 H21 V21 H15 Z",
        "variables" => "M5,2 V22 M12,2 V22 M19,2 V22 M2,7 H8 M9,17 H15 M16,9 H22",
        "history" => "M3,10 A9,9 0 1 1 4,18 M3,3 V10 H10 M12,7 V13 L16,15",
        "theme" => "M20,16 A9,9 0 1 1 8,3 A9,9 0 0 0 20,16 Z",
        "play" => "M6,3 L21,12 L6,21 Z",
        "eye" => "M2,12 Q12,-1 22,12 Q12,25 2,12 Z M12,8 A4,4 0 1 1 11.99,8 Z",
        "lock" => "M7,11 V7 A5,5 0 0 1 17,7 V11 M5,11 H19 V22 H5 Z M12,15 V18",
        "align-left" => "M3,2 V22 M7,5 H21 V9 H7 Z M7,15 H16 V19 H7 Z",
        "align-center" => "M12,2 V22 M3,5 H21 V9 H3 Z M7,15 H17 V19 H7 Z",
        "align-right" => "M21,2 V22 M3,5 H17 V9 H3 Z M8,15 H17 V19 H8 Z",
        "align-top" => "M2,3 H22 M5,7 H9 V21 H5 Z M15,7 H19 V16 H15 Z",
        "align-middle" => "M2,12 H22 M5,3 H9 V21 H5 Z M15,7 H19 V17 H15 Z",
        "align-bottom" => "M2,21 H22 M5,3 H9 V17 H5 Z M15,8 H19 V17 H15 Z",
        "group" => "M2,8 V2 H8 M16,2 H22 V8 M22,16 V22 H16 M8,22 H2 V16 M7,7 H17 V17 H7 Z",
        "polygon" => "M12,2 L23,21 H1 Z",
        "star" => "M12,1 L15,8 L23,9 L17,14 L19,22 L12,18 L5,22 L7,14 L1,9 L9,8 Z",
        "section" => "M2,4 H9 L12,7 H22 V21 H2 Z",
        _ => "M5,12 H19 M12,5 V19",
    };
    let path = Path::new();
    path.set_data(StreamGeometry::parse(data).ok().map(|g| g.upcast()));
    path.set_stroke(Some(brush(color)));
    path.set_stroke_thickness(if name == "logo" { 0.0 } else { 1.5 });
    if matches!(name, "logo" | "pointer" | "play") {
        path.set_fill(Some(brush(color)));
    } else if name == "gradient" {
        path.set_fill(Some(Rc::new(ImmutableLinearGradientBrush::from_stops(&[
            ImmutableGradientStop::new(0.0, Color::from_rgb(255, 255, 255)),
            ImmutableGradientStop::new(
                1.0,
                Color::parse(color).unwrap_or(Color::from_rgb(32, 32, 32)),
            ),
        ]))));
    }
    path.set_width(size);
    path.set_height(size);
    path.set_stretch(Stretch::Uniform);
    path
}
