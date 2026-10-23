//! Public native icon contracts, including actual text shaping/drawing.

use ferroui_base::layout::Layoutable;
use ferroui_base::media::text_formatting::testing::{
    DrawCall, RecordingDrawingSink, TestFont, TextTestScope,
};
use ferroui_base::media::{Brushes, FlowDirection, FontFamily, FontStyle, FontWeight};
use ferroui_base::{Point, Rect, Ref, Size, Visual};
use ferroui_controls::automation::{AccessibilityView, AutomationProperties};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::{FontIcon, Symbol, SymbolIcon};
use ferroui_controls::{IconElement, TextBlock};

fn fonts() -> TextTestScope {
    TextTestScope::with_fonts(vec![
        TestFont::new("Segoe Fluent Icons").with_ranges(&[(0x20, 0xF8FF)]),
        TestFont::new("Custom Icons").with_ranges(&[(0x20, 0xF8FF)]),
    ])
}

fn text_child(icon: &IconElement) -> Ref<TextBlock> {
    icon.visual_children_snapshot().unwrap()[0].cast::<TextBlock>().expect("native TextBlock template")
}

fn layout(control: &Layoutable) {
    control.measure(Size::new(1000.0, 1000.0));
    let size = control.desired_size();
    control.arrange(Rect::new(0.0, 0.0, size.width, size.height));
}

#[test]
fn symbol_contract_and_glyph_map_match_every_pinned_upstream_entry() {
    let entries: Vec<_> = include_str!("../../../docs/porting/winui-gallery/data/symbol-glyphs.tsv")
        .lines().filter(|line| !line.starts_with('#')).collect();
    assert_eq!(entries.len(), 197);
    assert_eq!(Symbol::ALL.len(), entries.len());
    for (symbol, entry) in Symbol::ALL.iter().zip(entries) {
        let fields: Vec<_> = entry.split('\t').collect();
        assert_eq!(format!("{symbol:?}"), fields[0]);
        assert_eq!(*symbol as u32, u32::from_str_radix(&fields[1][2..], 16).unwrap(), "{} API value", fields[0]);
        assert_eq!(symbol.glyph() as u32, u32::from_str_radix(&fields[2][2..], 16).unwrap(), "{} rendered glyph", fields[0]);
    }
    assert_eq!(Symbol::Play as u32, 0xE102);
    assert_eq!(Symbol::Play.glyph(), '\u{E768}');
    assert_eq!(Symbol::Target.glyph(), '\u{F5F0}');
}

#[test]
fn font_icon_shapes_and_draws_glyph_text_with_native_font_metrics() {
    let _app = UnitTestApplication::start(TestServices::mock_threading_interface());
    let _fonts = fonts();
    let icon = FontIcon::new();
    icon.set_glyph("AB");
    layout(&icon);
    assert_eq!(icon.font_size(), 20.0);
    assert_eq!(icon.font_family().name(), "Segoe Fluent Icons");
    assert!((icon.desired_size().width - 20.0).abs() < 0.001);
    let text = text_child(&icon);
    let mut sink = RecordingDrawingSink::new();
    text.text_layout().draw(&mut sink, Point::new(0.0, 0.0));
    assert!(sink.calls().iter().any(|call| matches!(call, DrawCall::GlyphRun { text, .. } if text == "AB")));
    assert_eq!(sink.transform_depth(), 0);
}

#[test]
fn glyph_typography_and_foreground_changes_update_existing_native_presenter() {
    let _app = UnitTestApplication::start(TestServices::mock_threading_interface());
    let _fonts = fonts();
    let icon = FontIcon::new();
    icon.set_glyph("A");
    layout(&icon);
    let text = text_child(&icon);
    icon.set_font_size(40.0);
    icon.set_font_family(FontFamily::new("Custom Icons"));
    icon.set_font_style(FontStyle::Italic);
    icon.set_font_weight(FontWeight::Bold);
    icon.set_foreground(Some(Brushes::red()));
    icon.set_glyph("BC");
    layout(&icon);
    assert_eq!(text_child(&icon), text);
    assert_eq!(text.text().as_deref(), Some("BC"));
    assert_eq!(text.font_size(), 40.0);
    assert_eq!(text.font_family().name(), "Custom Icons");
    assert_eq!(text.font_style(), FontStyle::Italic);
    assert_eq!(text.font_weight(), FontWeight::Bold);
    assert_eq!(text.foreground(), icon.foreground());
    assert!((icon.desired_size().width - 40.0).abs() < 0.001, "icon {:?}, text {:?}, layout {}", icon.desired_size(), text.desired_size(), text.text_layout().width());
}

#[test]
fn rtl_mirroring_is_opt_in_and_updates_without_replacing_user_transform() {
    let _app = UnitTestApplication::start(TestServices::mock_threading_interface());
    let _fonts = fonts();
    let icon = FontIcon::new();
    icon.set_glyph("A");
    layout(&icon);
    let text = text_child(&icon);
    icon.set_flow_direction(FlowDirection::RightToLeft);
    assert!(text.render_transform().is_none());
    icon.set_mirrored_when_right_to_left(true);
    assert_eq!(text.render_transform().unwrap().value().m11, -1.0);
    assert!(icon.render_transform().is_none());
    icon.set_flow_direction(FlowDirection::LeftToRight);
    assert!(text.render_transform().is_none());
    icon.set_flow_direction(FlowDirection::RightToLeft);
    icon.set_mirrored_when_right_to_left(false);
    assert!(text.render_transform().is_none());
}

#[test]
fn every_symbol_reaches_the_native_glyph_drawing_pipeline() {
    let _app = UnitTestApplication::start(TestServices::mock_threading_interface());
    let _fonts = fonts();
    let icon = SymbolIcon::new();
    assert_eq!(icon.symbol(), Symbol::Emoji);
    for symbol in Symbol::ALL {
        icon.set_symbol(*symbol);
        layout(&icon);
        let text = text_child(&icon);
        assert_eq!(text.text(), Some(symbol.glyph().to_string()));
        let mut sink = RecordingDrawingSink::new();
        text.text_layout().draw(&mut sink, Point::new(0.0, 0.0));
        assert!(sink.calls().iter().any(|call| matches!(call, DrawCall::GlyphRun { text, .. } if text == &symbol.glyph().to_string())), "{symbol:?}");
    }
}

#[test]
fn symbol_constructor_foreground_and_automation_use_native_contracts() {
    let _app = UnitTestApplication::start(TestServices::mock_threading_interface());
    let _fonts = fonts();
    let icon = SymbolIcon::with_symbol(Symbol::Accept);
    icon.set_foreground(Some(Brushes::blue()));
    layout(&icon);
    let text = text_child(&icon);
    assert_eq!(text.text().as_deref(), Some("\u{E8FB}"));
    assert_eq!(text.foreground(), icon.foreground());
    assert_eq!(text.font_size(), 20.0);
    assert_eq!(AutomationProperties::get_accessibility_view(&text), AccessibilityView::Raw);
    icon.set_foreground(Some(Brushes::red()));
    assert_eq!(text.foreground(), icon.foreground());
    let visual: Ref<Visual> = icon.upcast();
    assert!(visual.bypass_flow_direction_policies());
}

#[test]
fn opt_in_icons_do_not_change_existing_text_defaults() {
    let _app = UnitTestApplication::start(TestServices::mock_threading_interface());
    let _fonts = fonts();
    let before = TextBlock::new();
    let size = before.font_size();
    let family = before.font_family();
    let _icon = FontIcon::new();
    let _symbol = SymbolIcon::new();
    let after = TextBlock::new();
    assert_eq!(after.font_size(), size);
    assert_eq!(after.font_family(), family);
}
