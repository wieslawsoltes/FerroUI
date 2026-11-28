//! Crisp 24×24 Material/Fluent vector SVG path icons matching `ControlCatalog` style.

use ferroui_base::layout::{Orientation, VerticalAlignment};
use ferroui_base::media::StreamGeometry;
use ferroui_base::Ref;
use ferroui_controls::{IconElement, PathIcon};
use ferroui_view::prelude::*;

pub struct Icons;

impl Icons {
    // Navigation & sections
    pub const HOME: &'static str = "M10,20V14H14V20H19V12H22L12,3L2,12H5V20H10Z";
    pub const SPARKLE: &'static str = "M12,1L9,9L1,12L9,15L12,23L15,15L23,12L15,9Z";
    pub const LAYERS: &'static str = "M12,16L19.36,10.27L21,9L12,2L3,9L4.63,10.27M12,18.54L4.62,12.81L3,14.07L12,21.07L21,14.07L19.37,12.8Z";
    pub const NODE_GRAPH: &'static str =
        "M4,4H10V10H4V4M14,4H20V10H14V4M9,14H15V20H9V14M10,7H14M7,10V17H9M17,10V17H15";
    pub const CUBE: &'static str = "M21,16.5C21,16.88 20.79,17.21 20.47,17.38L12.57,21.82C12.41,21.94 12.21,22 12,22C11.79,22 11.59,21.94 11.43,21.82L3.53,17.38C3.21,17.21 3,16.88 3,16.5V7.5C3,7.12 3.21,6.79 3.53,6.62L11.43,2.18C11.59,2.06 11.79,2 12,2C12.21,2 12.41,2.06 12.57,2.18L20.47,6.62C20.79,6.79 21,7.12 21,7.5V16.5M12,4.15L5,8.09V15.91L12,19.85L19,15.91V8.09L12,4.15Z";
    pub const WATER: &'static str = "M12,2C12,2 5,10.25 5,15A7,7 0 0,0 12,22A7,7 0 0,0 19,15C19,10.25 12,2 12,2M12,20A5,5 0 0,1 7,15C7,12.1 10.1,7.4 12,5C13.9,7.4 17,12.1 17,15A5,5 0 0,1 12,20Z";
    pub const CLOTH: &'static str =
        "M3,3H21V5H3V3M3,7H21V9H3V7M3,11H21V13H3V11M3,15H17V17H3V15M3,19H13V21H3V19Z";
    pub const UI_MOTION: &'static str = "M4,4H20A2,2 0 0,1 22,6V18A2,2 0 0,1 20,20H4A2,2 0 0,1 2,18V6A2,2 0 0,1 4,4M4,8V18H20V8H4M6,10H12V16H6V10M14,10H18V12H14V10M14,14H18V16H14V14Z";
    pub const SHATTER: &'static str =
        "M12,2L14.8,8.2L21.5,9L16.5,13.6L17.9,20.3L12,16.9L6.1,20.3L7.5,13.6L2.5,9L9.2,8.2L12,2Z";
    pub const VEHICLE: &'static str = "M5,11L6.5,6.5H17.5L19,11M17.5,16A1.5,1.5 0 0,1 16,14.5A1.5,1.5 0 0,1 17.5,13A1.5,1.5 0 0,1 19,14.5A1.5,1.5 0 0,1 17.5,16M6.5,16A1.5,1.5 0 0,1 5,14.5A1.5,1.5 0 0,1 6.5,13A1.5,1.5 0 0,1 8,14.5A1.5,1.5 0 0,1 6.5,16M18.92,6C18.72,5.42 18.16,5 17.5,5H6.5C5.84,5 5.28,5.42 5.08,6L3,12V20A1,1 0 0,0 4,21H5A1,1 0 0,0 6,20V19H18V20A1,1 0 0,0 19,21H20A1,1 0 0,0 21,20V12L18.92,6Z";
    pub const PERSON: &'static str =
        "M12,3A4,4 0 1,1 12,11A4,4 0 1,1 12,3M4,20C4,16 7.6,13 12,13C16.4,13 20,16 20,20Z";
    pub const ORBIT: &'static str = "M12,2A10,10 0 0,0 2,12A10,10 0 0,0 12,22A10,10 0 0,0 22,12A10,10 0 0,0 12,2M12,4A8,8 0 0,1 20,12A8,8 0 0,1 12,20A8,8 0 0,1 4,12A8,8 0 0,1 12,4M12,8A4,4 0 0,0 8,12A4,4 0 0,0 12,16A4,4 0 0,0 16,12A4,4 0 0,0 12,8Z";

    // Transport, tools & shader controls
    pub const PLAY: &'static str = "M8,5.14V19.14L19,12.14L8,5.14Z";
    pub const PAUSE: &'static str = "M14,19H18V5H14M6,19H10V5H6V19Z";
    pub const STEP_BACK: &'static str = "M6,6H8V18H6M9.5,12L18,18V6L9.5,12Z";
    pub const STEP_FWD: &'static str = "M16,18H18V6H16M6,18L14.5,12L6,6V18Z";
    pub const RESET: &'static str = "M17.65,6.35C16.2,4.9 14.21,4 12,4A8,8 0 0,0 4,12A8,8 0 0,0 12,20C15.73,20 18.84,17.45 19.73,14H17.65C16.83,16.33 14.61,18 12,18A6,6 0 0,1 6,12A6,6 0 0,1 12,6C13.66,6 15.14,6.69 16.22,7.78L13,11H20V4L17.65,6.35Z";
    pub const LIGHTNING: &'static str = "M11,21H7L13,3H17L14,10H20L11,21Z";
    pub const TARGET: &'static str = "M12,8A4,4 0 0,0 8,12A4,4 0 0,0 12,16A4,4 0 0,0 16,12A4,4 0 0,0 12,8M3.05,13H1V11H3.05C3.5,6.83 6.83,3.5 11,3.05V1H13V3.05C17.17,3.5 20.5,6.83 20.95,11H23V13H20.95C20.5,17.17 17.17,20.5 13,20.95V23H11V20.95C6.83,20.5 3.5,17.17 3.05,13M12,5A7,7 0 0,0 5,12A7,7 0 0,0 12,19A7,7 0 0,0 19,12A7,7 0 0,0 12,5Z";
    pub const WAVE: &'static str = "M2,12C4,9 6,9 8,12C10,15 12,15 14,12C16,9 18,9 20,12L22,12C20,15 18,17 14,17C10,17 8,14 6,14C4,14 3,15 2,16V12Z";
    pub const MAGNET: &'static str = "M3,7V13A9,9 0 0,0 12,22A9,9 0 0,0 21,13V7H17V13A5,5 0 0,1 12,18A5,5 0 0,1 7,13V7H3M3,3V6H7V3H3M17,3V6H21V3H17Z";
    pub const CURSOR: &'static str = "M13.64,21.97C13.14,22.21 12.54,22 12.31,21.5L10.13,16.76L7.62,18.78C7.45,18.92 7.24,19 7,19A1,1 0 0,1 6,18V3A1,1 0 0,1 7,2C7.24,2 7.47,2.09 7.64,2.23L19.14,11.86C19.57,12.22 19.39,12.92 18.83,13.03L14.83,13.78L17,18.5C17.24,19 17,19.59 16.5,19.83L13.64,21.97Z";
    pub const BOX_SHAPE: &'static str = "M4,4H20V20H4V4M6,6V18H18V6H6Z";
    pub const CIRCLE_SHAPE: &'static str = "M12,2A10,10 0 0,0 2,12A10,10 0 0,0 12,22A10,10 0 0,0 22,12A10,10 0 0,0 12,2M12,4A8,8 0 0,1 20,12A8,8 0 0,1 12,20A8,8 0 0,1 4,12A8,8 0 0,1 12,4Z";
    pub const HEX_SHAPE: &'static str = "M17.5,3.5L22,12L17.5,20.5H6.5L2,12L6.5,3.5H17.5M16.3,5.5H7.7L4.3,12L7.7,18.5H16.3L19.7,12L16.3,5.5Z";
    pub const THEME: &'static str = "M12,2A10,10 0 0,0 2,12A10,10 0 0,0 12,22A10,10 0 0,0 22,12A10,10 0 0,0 12,2M12,4A8,8 0 0,1 20,12A8,8 0 0,1 12,20V4Z";
    pub const BLUR: &'static str = "M14.5,10C16.43,10 18,8.43 18,6.5C18,4.57 16.43,3 14.5,3C12.57,3 11,4.57 11,6.5C11,8.43 12.57,10 14.5,10M8.5,14C10.43,14 12,12.43 12,10.5C12,8.57 10.43,7 8.5,7C6.57,7 5,8.57 5,10.5C5,12.43 6.57,14 8.5,14M14.5,16A4.5,4.5 0 0,0 10,20.5A4.5,4.5 0 0,0 14.5,25A4.5,4.5 0 0,0 19,20.5A4.5,4.5 0 0,0 14.5,16Z";
    pub const SLIDER: &'static str = "M7,5H21V7H7V5M7,13V11H21V13H7M4,4.5A1.5,1.5 0 0,1 5.5,6A1.5,1.5 0 0,1 4,7.5A1.5,1.5 0 0,1 2.5,6A1.5,1.5 0 0,1 4,4.5M4,10.5A1.5,1.5 0 0,1 5.5,12A1.5,1.5 0 0,1 4,13.5A1.5,1.5 0 0,1 2.5,12A1.5,1.5 0 0,1 4,10.5M7,19V17H21V19H7M4,16.5A1.5,1.5 0 0,1 5.5,18A1.5,1.5 0 0,1 4,19.5A1.5,1.5 0 0,1 2.5,18A1.5,1.5 0 0,1 4,16.5Z";
}

/// Constructs a `Ref<IconElement>` backed by a `PathIcon` for `NavigationViewItem::set_icon`.
pub fn nav_icon(path_data: &str) -> Option<Ref<IconElement>> {
    let geom = StreamGeometry::parse(path_data).ok()?;
    let icon = PathIcon::new();
    icon.set_data(geom);
    Some(icon.upcast())
}

/// Builds a reactive `ferroui-view` `PathIcon` element of size `size × size`.
pub fn icon_view(path_data: &'static str, size: f64) -> Element {
    let geom = StreamGeometry::parse(path_data).ok();
    path_icon()
        .width(size)
        .height(size)
        .vertical_alignment(VerticalAlignment::Center)
        .with(move |p: &Ref<PathIcon>| {
            if let Some(ref g) = geom {
                p.set_data(g.clone());
            }
        })
        .into()
}

/// Builds a clean horizontal icon + text label content element for stock WinUI buttons.
pub fn icon_label(path_data: &'static str, text: &'static str, icon_size: f64) -> Element {
    stack_panel()
        .orientation(Orientation::Horizontal)
        .spacing(6.0)
        .vertical_alignment(VerticalAlignment::Center)
        .child(icon_view(path_data, icon_size))
        .child(
            text_block()
                .text(text)
                .vertical_alignment(VerticalAlignment::Center),
        )
        .into()
}
