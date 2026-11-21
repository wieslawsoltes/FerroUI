use crate::{
    compat::{Symbol, SymbolIcon, SymbolIconSource},
    controls::RibbonItemHelper,
    primitives::RibbonIconPresenter,
};
use ferroui_base::{
    Rect, Ref, Size,
    layout::ILayoutManager,
    media::{FontStyle, FontWeight, SolidColorBrush},
    metadata::MarkupTyped,
};
use ferroui_controls::{
    ContentControl, Control, Grid, TextBlock, Viewbox, Window,
    automation::{AccessibilityView, AutomationProperties},
};
use serde_json::Value;
use std::{collections::HashMap, rc::Rc};
fn text(icon: &SymbolIcon) -> Ref<TextBlock> {
    icon.visual_children_snapshot().unwrap()[0]
        .cast::<Grid>()
        .unwrap()
        .children()
        .get(0)
        .cast::<TextBlock>()
        .unwrap()
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("../../tests/fixtures/symbol-icons.json")).unwrap()
}

#[test]
fn unchanged_symbol_mapping_covers_every_bmp_unit_i32_wrap_and_named_enum_member() {
    let f = fixture();
    let c = &f["corpus"];
    let overrides = c["mappings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| {
            (
                m["value"].as_i64().unwrap() as i32,
                m["glyph"].as_u64().unwrap() as u16,
            )
        })
        .collect::<HashMap<_, _>>();
    for value in 0..=65535 {
        assert_eq!(
            SymbolIcon::convert_symbol_value_to_glyph(value),
            overrides.get(&value).copied().unwrap_or(value as u16),
            "{value}"
        );
    }
    for b in c["boundaries"].as_array().unwrap() {
        assert_eq!(
            SymbolIcon::convert_symbol_value_to_glyph(b["value"].as_i64().unwrap() as i32),
            b["glyph"].as_u64().unwrap() as u16
        );
    }
    let metadata = crate::metadata::enums::SymbolMetadata::MARKUP;
    assert_eq!(
        metadata.enum_members.len(),
        c["members"].as_array().unwrap().len()
    );
    for member in c["members"].as_array().unwrap() {
        let m = metadata
            .enum_members
            .iter()
            .find(|m| m.name == member["name"].as_str().unwrap())
            .unwrap();
        let v = m.get(metadata).unwrap();
        let value = v.downcast_ref::<Symbol>().unwrap();
        assert_eq!(i64::from(value.raw()), member["value"].as_i64().unwrap());
        assert_eq!(
            SymbolIcon::convert_symbol_value_to_glyph(value.raw()),
            member["glyph"].as_u64().unwrap() as u16
        );
    }
    assert_eq!(
        SymbolIcon::convert_symbol_value_to_glyph(Symbol::Target.raw()),
        0xe1d2
    );
    assert_eq!(SymbolIcon::convert_symbol_value_to_glyph(0x1e10b), 0xe10b);
}

#[test]
fn original_symbol_constructor_units_map_to_native_text_with_explicit_surrogate_boundary() {
    crate::tests::run(|| {
        for expected in fixture()["corpus"]["controls"].as_array().unwrap() {
            let value = Symbol::from_raw(expected["value"].as_i64().unwrap() as i32);
            let units = expected["units"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u16)
                .collect::<Vec<_>>();
            if (0xd800..=0xdfff).contains(&units[0]) {
                let e = SymbolIcon::with_symbol(value).expect_err("native text limitation");
                assert_eq!(e.glyph, units[0]);
                let icon = SymbolIcon::new();
                assert!(icon.set_symbol(value).is_err());
                assert_eq!(icon.symbol(), Symbol::Emoji);
                continue;
            }
            let icon = SymbolIcon::with_symbol(value).unwrap();
            let child = text(&icon);
            assert_eq!(icon.symbol(), value);
            assert_eq!(
                child.text().unwrap().encode_utf16().collect::<Vec<_>>(),
                units
            );
            assert_eq!(
                child.font_size().to_bits(),
                expected["size"].as_f64().unwrap().to_bits()
            );
            assert_eq!(child.font_style(), FontStyle::Normal);
            assert_eq!(
                child.font_family(),
                ferroui_base::media::FontFamily::new(SymbolIcon::FONT_FAMILY)
            );
            assert_eq!(
                child.get_value(AutomationProperties::accessibility_view_property()),
                AccessibilityView::Raw
            );
        }
    });
}

#[test]
fn symbol_native_grid_text_foreground_invalidation_snapshots_and_release() {
    crate::tests::run(|| {
        let window = Window::new();
        let parent = ContentControl::new();
        parent.set_font_size(80.);
        parent.set_font_style(FontStyle::Italic);
        parent.set_font_weight(FontWeight::Bold);
        let icon = SymbolIcon::new();
        parent.set_content(Some(Control::boxed(icon.clone())));
        window.set_content(Some(Control::boxed(parent.clone())));
        window.show();
        window.layout_manager().execute_layout_pass();
        let child = text(&icon);
        assert_eq!(icon.symbol(), Symbol::Emoji);
        assert_eq!(child.font_size(), 20.);
        assert_eq!(child.font_style(), FontStyle::Normal);
        assert_eq!(child.font_weight(), FontWeight::Bold);
        parent.set_foreground(Some(SolidColorBrush::from_uint32(0xffff0000).into()));
        assert_eq!(child.foreground(), icon.foreground());
        icon.measure(Size::INFINITY);
        icon.arrange(Rect::from_size(icon.desired_size()));
        assert!(icon.is_measure_valid());
        icon.set_symbol(Symbol::Add).unwrap();
        // Native layout queues the invalidated text; ancestors are measured
        // when the child's desired size changes, rather than synchronously.
        assert!(!child.is_measure_valid());
        window.layout_manager().execute_layout_pass();
        assert!(icon.is_measure_valid() && child.is_measure_valid());
        assert_eq!(child.text().as_deref(), Some("\u{e710}"));
        let p = RibbonIconPresenter::new();
        p.set_icon(Some(Rc::new(icon.clone())));
        let cloned = p
            .children()
            .get(0)
            .cast::<Viewbox>()
            .unwrap()
            .child()
            .unwrap()
            .cast::<SymbolIcon>()
            .unwrap();
        assert!(!cloned.ptr_eq(&icon));
        assert_eq!(cloned.symbol(), Symbol::Add);
        let source = SymbolIconSource::new();
        assert_eq!(source.symbol(), Symbol::Emoji);
        source.set_symbol(Symbol::Save);
        let menu = RibbonItemHelper::create_menu_icon(Some(Rc::new(source)))
            .unwrap()
            .unwrap()
            .cast::<SymbolIcon>()
            .unwrap();
        assert_eq!(menu.symbol(), Symbol::Save);
        let weak = icon.downgrade();
        let weak_child = child.downgrade();
        p.set_icon(None);
        parent.set_content(None);
        window.set_content(None);
        window.close();
        drop(child);
        drop(icon);
        drop(parent);
        drop(window);
        assert!(weak.upgrade().is_none());
        assert!(weak_child.upgrade().is_none());
    });
}
