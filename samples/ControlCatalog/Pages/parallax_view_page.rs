//! The page of `ParallaxView`, built in code.

use ferroui_base::layout::{HorizontalAlignment, VerticalAlignment};
use ferroui_base::media::{Color, GradientStop, LinearGradientBrush};
use ferroui_base::{BoxedValue, Ref, RelativePoint, Thickness};
use ferroui_controls::{Border, ContentPage, Control, Grid, ScrollViewer, StackPanel, TextBlock};
use ferroui_controls_motion::ParallaxView;
use std::rc::Rc;

/// A tall gradient, so that the motion of the background is visible.
fn background() -> Ref<Border> {
    let brush = LinearGradientBrush::new();
    brush.set_start_point(RelativePoint::TOP_LEFT);
    brush.set_end_point(RelativePoint::BOTTOM_RIGHT);
    for (offset, color) in [(0.0, Color::from_rgb(0x1b, 0x3a, 0x6b)), (0.5, Color::from_rgb(0x2f, 0x8f, 0x9d)), (1.0, Color::from_rgb(0xf2, 0xc1, 0x4e))] {
        brush.gradient_stops().add(GradientStop::with_color_and_offset(color, offset));
    }
    let border = Border::new();
    border.set_background(Some(brush.into()));
    border
}

/// A list of rows over a background that moves more slowly than they do.
pub fn page() -> Ref<ContentPage> {
    let rows = StackPanel::new();
    rows.set_spacing(8.0);
    for index in 1..=40 {
        let row = TextBlock::new();
        row.set_text(Some(format!("Row {index}").as_str()));
        row.set_margin(Thickness::uniform(12.0));
        rows.children().add(&row);
    }
    let scroller = ScrollViewer::new();
    scroller.set_content(Some(Control::boxed(&rows)));

    let view = ParallaxView::new();
    view.set_name(Some("Parallax".to_string()));
    view.set_horizontal_alignment(HorizontalAlignment::Stretch);
    view.set_vertical_alignment(VerticalAlignment::Stretch);
    view.set_vertical_shift(200.0);
    view.set_child(Some(background().upcast::<Control>()));
    view.set_source(Some(scroller.clone().upcast::<Control>()));

    let grid = Grid::new();
    grid.set_height(420.0);
    grid.set_clip_to_bounds(true);
    grid.children().add(&view);
    grid.children().add(&scroller);

    let panel = StackPanel::new();
    panel.set_margin(Thickness::uniform(24.0));
    panel.set_spacing(16.0);
    let caption = TextBlock::new();
    caption.set_text(Some("Scroll the rows: the background moves through its composition visual, more slowly."));
    panel.children().add(&caption);
    panel.children().add(&grid);

    let page = ContentPage::new();
    page.set_header(Some(Rc::new("ParallaxView".to_owned()) as BoxedValue));
    page.set_content(Some(Control::boxed(&panel)));
    page
}
