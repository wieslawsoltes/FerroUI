//! Tests of `Symbol` and `SymbolIcon` (WinUI; docs/porting/winui-controls.md, the icons).

use crate::icon_test_support::{apply_text_template, fonts, layout, text_part};
use crate::testing::{TestServices, UnitTestApplication};
use crate::{Symbol, SymbolIcon, TextBlock};
use ferroui_base::media::text_formatting::testing::{DrawCall, RecordingDrawingSink};
use ferroui_base::media::Brushes;
use ferroui_base::{Point, Ref, Visual};
use crate::automation::{AccessibilityView, AutomationProperties};

/// The table `scripts/winui-controls/symbol_map.py` writes from the source of WinUI.
const TABLE: &str = include_str!("../../docs/porting/data/winui-controls/symbol-glyphs.tsv");

#[test]
fn the_members_values_and_glyphs_are_the_ones_of_winui() {
    let entries: Vec<_> = TABLE.lines().filter(|line| !line.starts_with('#') && !line.is_empty()).collect();
    assert_eq!(197, entries.len());
    assert_eq!(entries.len(), Symbol::ALL.len());
    for (symbol, entry) in Symbol::ALL.iter().zip(entries) {
        let fields: Vec<_> = entry.split('\t').collect();
        assert_eq!(fields[0], format!("{symbol:?}"));
        assert_eq!(u32::from_str_radix(&fields[1][2..], 16).unwrap(), *symbol as u32, "the value of {}", fields[0]);
        assert_eq!(u32::from_str_radix(&fields[2][2..], 16).unwrap(), symbol.glyph() as u32, "the glyph of {}", fields[0]);
    }
    assert_eq!(0xE102, Symbol::Play as u32);
    assert_eq!('\u{E768}', Symbol::Play.glyph());
}

#[test]
fn the_default_symbol_is_emoji() {
    assert_eq!(Symbol::Emoji, SymbolIcon::new().symbol());
    assert_eq!(Symbol::Emoji, Symbol::default());
}

#[test]
fn every_symbol_is_drawn_as_its_glyph() {
    let _app = UnitTestApplication::start(TestServices::mock_threading_interface());
    let _fonts = fonts();
    let icon = SymbolIcon::new();
    apply_text_template(&icon);
    for symbol in Symbol::ALL {
        icon.set_symbol(*symbol);
        layout(&icon);
        let text = text_part(&icon);
        let glyph = symbol.glyph().to_string();
        assert_eq!(Some(glyph.clone()), text.text());
        let mut sink = RecordingDrawingSink::new();
        text.text_layout().draw(&mut sink, Point::new(0.0, 0.0));
        assert!(
            sink.calls().iter().any(|call| matches!(call, DrawCall::GlyphRun { text, .. } if *text == glyph)),
            "{symbol:?}"
        );
    }
}

#[test]
fn the_symbol_and_the_foreground_go_to_the_text_part() {
    let _app = UnitTestApplication::start(TestServices::mock_threading_interface());
    let _fonts = fonts();
    let icon = SymbolIcon::with_symbol(Symbol::Accept);
    icon.set_foreground(Some(Brushes::blue()));
    apply_text_template(&icon);
    layout(&icon);
    let text: Ref<TextBlock> = text_part(&icon);
    assert_eq!(Some("\u{E8FB}"), text.text().as_deref());
    assert_eq!(icon.foreground(), text.foreground());
    assert_eq!(AccessibilityView::Raw, AutomationProperties::get_accessibility_view(&text));
    icon.set_foreground(Some(Brushes::red()));
    assert_eq!(icon.foreground(), text.foreground());
    icon.set_symbol(Symbol::Cancel);
    assert_eq!(Some(Symbol::Cancel.glyph().to_string()), text.text());
    let visual: Ref<Visual> = icon.upcast();
    assert!(visual.bypass_flow_direction_policies());
}

#[test]
fn an_icon_without_a_template_has_no_text_part_and_no_font_of_its_own() {
    // The template and the font are the theme's (docs/porting/winui-controls.md, section 5): the
    // control states none, so it takes nothing from the defaults of text.
    let _app = UnitTestApplication::start(TestServices::mock_threading_interface());
    let icon = SymbolIcon::new();
    assert!(icon.template().is_none());
    assert_eq!(TextBlock::new().font_family(), icon.font_family());
}
