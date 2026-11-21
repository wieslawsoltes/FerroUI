//! Actual bundled glyph rendering.
use super::*;
use ferroribbon::compat::{Symbol, SymbolIcon};
use ferroui_base::{
    PixelRect,
    layout::{HorizontalAlignment, VerticalAlignment},
    media::{FontManager, SolidColorBrush, TextAlignment, Typeface},
};
use ferroui_controls::{Control, Grid, TextBlock, Window};

#[test]
#[ignore = "Explicit Skia/HarfBuzz symbol glyphs"]
fn source_symbol_mapping_renders_bundled_glyphs_without_reconstruction() {
    let directory = PathBuf::from(
        std::env::var_os("FERRORIBBON_CAPTURE_DIRECTORY").expect("explicit capture directory"),
    );
    assert!(directory.is_absolute());
    std::fs::create_dir_all(&directory).unwrap();
    let session = HeadlessUnitTestSession::start_new(build);
    let result = session
        .dispatch(
            move || {
                register_types();
                let family = FontFamily::new(SymbolIcon::FONT_FAMILY);
                let glyphs = FontManager::current()
                    .try_get_glyph_typeface(&Typeface::new(family.clone()))
                    .expect("bundled source font");
                let window = Window::new();
                window.set_width(40.);
                window.set_height(40.);
                let icon = SymbolIcon::new();
                icon.set_foreground(Some(SolidColorBrush::from_uint32(0xffff0000).into()));
                window.set_content(Some(Control::boxed(icon.clone())));
                window.show();
                let frame = |name: &str| {
                    window.update_layout();
                    capture(&window, &directory, name);
                    let image = window.get_last_rendered_frame().unwrap();
                    let size = image.pixel_size();
                    let mut bytes = vec![0; (size.width * size.height * 4) as usize];
                    image.copy_pixels(
                        PixelRect::new(0, 0, size.width, size.height),
                        &mut bytes,
                        size.width * 4,
                    );
                    bytes
                };
                let mut previous = None;
                for (name, symbol) in [
                    ("Save", Symbol::Save),
                    ("Accept", Symbol::Accept),
                    ("Emoji", Symbol::Emoji),
                ] {
                    let unit = SymbolIcon::convert_symbol_value_to_glyph(symbol.raw());
                    assert_ne!(
                        glyphs.character_to_glyph_map().get_glyph(i32::from(unit)),
                        0
                    );
                    icon.set_symbol(symbol).unwrap();
                    window.set_content(Some(Control::boxed(icon.clone())));
                    let actual = frame(&format!("symbol-{name}.png"));
                    assert!(
                        actual
                            .as_chunks::<4>()
                            .0
                            .iter()
                            .any(|p| p[3] != 0 && (p[0] != p[1] || p[2] != p[1])),
                        "visible colored glyph"
                    );
                    if let Some(old) = previous.replace(actual.clone()) {
                        assert_ne!(old, actual);
                    }
                    // Independent native TextBlock uses the oracle's resulting unit.
                    let root = Grid::new();
                    let text = TextBlock::new();
                    text.set_text(Some(&char::from_u32(u32::from(unit)).unwrap().to_string()));
                    text.set_font_family(family.clone());
                    text.set_font_size(20.);
                    text.set_foreground(icon.foreground());
                    text.set_horizontal_alignment(HorizontalAlignment::Stretch);
                    text.set_vertical_alignment(VerticalAlignment::Center);
                    text.set_text_alignment(TextAlignment::Center);
                    root.children().add(text);
                    window.set_content(Some(Control::boxed(root)));
                    assert_eq!(actual, frame(&format!("symbol-{name}-native-text.png")));
                    window.set_content(None);
                }
                window.close();
            },
            CancellationToken::none(),
        )
        .wait();
    session.dispose();
    result.unwrap();
}
