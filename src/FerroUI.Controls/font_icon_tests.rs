//! Tests of `FontIcon` (WinUI; docs/porting/winui-controls.md, the icons).

use crate::icon_test_support::{apply_text_template, fonts, layout, text_part};
use crate::testing::{TestServices, UnitTestApplication};
use crate::{FontIcon, SymbolIcon, TextBlock};
use ferroui_base::media::text_formatting::testing::{DrawCall, RecordingDrawingSink};
use ferroui_base::media::{Brushes, FlowDirection, FontFamily, FontStyle, FontWeight};
use ferroui_base::Point;

#[test]
fn the_glyph_is_drawn_with_the_size_of_a_font_icon() {
    let _app = UnitTestApplication::start(TestServices::mock_threading_interface());
    let _fonts = fonts();
    let icon = FontIcon::new();
    icon.set_font_family(FontFamily::new("Segoe Fluent Icons"));
    icon.set_glyph("AB");
    apply_text_template(&icon);
    layout(&icon);
    // FontIcon.FontSize is 20 in WinUI.
    assert_eq!(20.0, icon.font_size());
    assert!((icon.desired_size().width - 20.0).abs() < 0.001, "{:?}", icon.desired_size());
    let text = text_part(&icon);
    let mut sink = RecordingDrawingSink::new();
    text.text_layout().draw(&mut sink, Point::new(0.0, 0.0));
    assert!(sink.calls().iter().any(|call| matches!(call, DrawCall::GlyphRun { text, .. } if text == "AB")));
}

#[test]
fn a_change_of_the_glyph_or_a_text_property_goes_to_the_same_text_part() {
    let _app = UnitTestApplication::start(TestServices::mock_threading_interface());
    let _fonts = fonts();
    let icon = FontIcon::new();
    icon.set_glyph("A");
    apply_text_template(&icon);
    layout(&icon);
    let text = text_part(&icon);
    icon.set_font_size(40.0);
    icon.set_font_family(FontFamily::new("Custom Icons"));
    icon.set_font_style(FontStyle::Italic);
    icon.set_font_weight(FontWeight::Bold);
    icon.set_foreground(Some(Brushes::red()));
    icon.set_glyph("BC");
    layout(&icon);
    assert_eq!(text, text_part(&icon));
    assert_eq!(Some("BC"), text.text().as_deref());
    assert_eq!(40.0, text.font_size());
    assert_eq!("Custom Icons", text.font_family().name());
    assert_eq!(FontStyle::Italic, text.font_style());
    assert_eq!(FontWeight::Bold, text.font_weight());
    assert_eq!(icon.foreground(), text.foreground());
    assert!((icon.desired_size().width - 40.0).abs() < 0.001, "{:?}", icon.desired_size());
}

#[test]
fn the_glyph_is_mirrored_only_when_asked_and_right_to_left() {
    let _app = UnitTestApplication::start(TestServices::mock_threading_interface());
    let _fonts = fonts();
    let icon = FontIcon::new();
    icon.set_glyph("A");
    apply_text_template(&icon);
    layout(&icon);
    let text = text_part(&icon);
    icon.set_flow_direction(FlowDirection::RightToLeft);
    assert!(text.render_transform().is_none());
    icon.set_mirrored_when_right_to_left(true);
    assert_eq!(-1.0, text.render_transform().unwrap().value().m11);
    assert!(icon.render_transform().is_none());
    icon.set_flow_direction(FlowDirection::LeftToRight);
    assert!(text.render_transform().is_none());
    icon.set_flow_direction(FlowDirection::RightToLeft);
    icon.set_mirrored_when_right_to_left(false);
    assert!(text.render_transform().is_none());
}

#[test]
fn the_icons_change_no_default_of_text() {
    let _app = UnitTestApplication::start(TestServices::mock_threading_interface());
    let before = TextBlock::new();
    let (size, family) = (before.font_size(), before.font_family());
    let _icons = (FontIcon::new(), SymbolIcon::new());
    let after = TextBlock::new();
    assert_eq!(size, after.font_size());
    assert_eq!(family, after.font_family());
}
