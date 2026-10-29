//! The source TrayContainer's dark inset tray and gold edge, shared by empty piles.
use ferroui::{
    base::media::{Color, IBrush, SolidColorBrush},
    controls::{Border, TextBlock},
    prelude::*,
};
use std::rc::Rc;
pub fn brush(color: &str) -> Rc<dyn IBrush> {
    SolidColorBrush::with_color(Color::parse(color).expect("constant colour")).into()
}
pub fn tray(label: &str, width: f64, height: f64) -> Ref<Border> {
    let border = Border::new();
    border.set_width(width);
    border.set_height(height);
    border.set_corner_radius(CornerRadius::uniform(8.0));
    border.set_background(Some(brush("#18130E07")));
    border.set_border_brush(Some(brush("#609CCDAB")));
    border.set_border_thickness(Thickness::uniform(2.0));
    let text = TextBlock::new();
    text.set_text(Some(label));
    text.set_font_size(36.0);
    text.set_foreground(Some(brush("#709CCDAB")));
    text.set_horizontal_alignment(HorizontalAlignment::Center);
    text.set_vertical_alignment(VerticalAlignment::Center);
    border.set_child(text);
    border
}
