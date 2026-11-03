#[path = "support/headless.rs"]
mod headless;
use ferroboard_controls::{EmojiFont, EmojiFontError};
use ferroui_base::{
    FerroLocator, LocatorExtensions,
    media::{
        FontFallback, FontFamily, FontManager, FontManagerOptions, FontStretch, FontStyle,
        FontWeight, Typeface, UnicodeRange,
    },
    platform::IFontManagerImpl,
};
use std::{
    io::{self, Read},
    rc::Rc,
};
const FONT: &[u8] = include_bytes!("../../../tests/fixtures/fonts/Inter-Regular.ttf");
#[test]
fn font_readers_report_io_failure_and_empty_data() {
    struct Denied;
    impl Read for Denied {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::PermissionDenied, "fixture"))
        }
    }
    assert!(
        matches!(EmojiFont::from_reader(Denied),Err(EmojiFontError::Read(e))if e.kind()==io::ErrorKind::PermissionDenied)
    );
    assert!(matches!(
        EmojiFont::from_bytes(Vec::<u8>::new()),
        Err(EmojiFontError::Empty)
    ));
}
#[test]
fn installed_font_is_cached_by_ferroui_and_owned_by_the_manager() {
    headless::run(|| {
        let _scope = FerroLocator::enter_scope();
        let font = EmojiFont::from_reader(FONT).unwrap();
        let family = font.font_family();
        let backend = FerroLocator::current().get_required_service::<dyn IFontManagerImpl>();
        let manager = FontManager::new(backend);
        font.install(&manager).unwrap();
        let face = manager
            .try_get_glyph_typeface(&Typeface::new(family.clone()))
            .unwrap();
        assert_eq!(face.family_name(), "Inter");
        assert!(
            face.character_to_glyph_map()
                .try_get_glyph('A' as i32)
                .is_some()
        );
        font.install(&manager).unwrap();
        assert!(Rc::ptr_eq(
            &face,
            &manager
                .try_get_glyph_typeface(&Typeface::new(family.clone()))
                .unwrap()
        ));
        manager.remove_font_collection(family.key().unwrap().source());
        font.install(&manager).unwrap();
        let replacement = manager
            .try_get_glyph_typeface(&Typeface::new(family.clone()))
            .unwrap();
        assert!(!Rc::ptr_eq(&face, &replacement));
        drop(font);
        assert!(
            manager
                .try_get_glyph_typeface(&Typeface::new(family))
                .is_some()
        );
    });
}
#[test]
fn emoji_fallback_preserves_options_and_uses_native_glyph_coverage() {
    headless::run(|| {
        let _scope = FerroLocator::enter_scope();
        let font = EmojiFont::from_bytes(FONT).unwrap();
        let previous = FontFallback {
            font_family: FontFamily::new("Prior Emoji"),
            unicode_range: UnicodeRange::new(0x1f600, 0x1f600),
        };
        let mut options = FontManagerOptions {
            default_family_name: Some("Body".into()),
            font_fallbacks: Some(vec![previous.clone()]),
            font_family_mappings: Some(
                [("Alias".into(), FontFamily::new("Body"))]
                    .into_iter()
                    .collect(),
            ),
        };
        font.configure(&mut options);
        font.configure(&mut options);
        assert_eq!(options.default_family_name.as_deref(), Some("Body"));
        assert_eq!(options.font_family_mappings.as_ref().unwrap().len(), 1);
        let fallbacks = options.font_fallbacks.as_ref().unwrap();
        assert_eq!(fallbacks.len(), 2);
        assert_eq!(fallbacks[1], previous);
        assert!(!fallbacks[0].unicode_range.is_in_range('A' as i32));
        assert!(fallbacks[0].unicode_range.is_in_range(0x1f600));
        FerroLocator::current_mutable()
            .bind::<FontManagerOptions>()
            .to_constant(Rc::new(options));
        let manager = FontManager::new(
            FerroLocator::current().get_required_service::<dyn IFontManagerImpl>(),
        );
        font.install(&manager).unwrap();
        // Inter contains this symbol; it is in the original keyboard's emoji classification range.
        let matched = manager
            .try_match_character(
                0x2122,
                FontStyle::Normal,
                FontWeight::Normal,
                FontStretch::Normal,
                None,
                None,
            )
            .unwrap();
        assert_eq!(matched.font_family(), &font.font_family());
        // A valid font lacking a requested emoji leaves that character to the remaining native chain.
        let own = manager
            .try_get_glyph_typeface(&Typeface::new(font.font_family()))
            .unwrap();
        assert!(
            own.character_to_glyph_map()
                .try_get_glyph(0x1f600)
                .is_none()
        );
    });
}

#[test]
fn source_font_constants_and_unicode_classifier_match_without_runtime_emulation() {
    use ferroboard_controls::EmojiFontFallback;
    let rows: Vec<serde_json::Value> = include_str!("../../../tests/fixtures/emoji-font.jsonl")
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let constants = rows.iter().find(|row| row["tag"] == "constants").unwrap();
    assert_eq!(
        constants["family"].as_str(),
        Some(EmojiFontFallback::FAMILY_NAME)
    );
    assert_eq!(
        constants["url"].as_str(),
        Some(EmojiFontFallback::DEFAULT_NOTO_COLOR_EMOJI_URL)
    );
    for row in rows.iter().filter(|row| row["tag"] == "classify") {
        let cp = i32::try_from(row["cp"].as_i64().unwrap()).unwrap();
        assert_eq!(
            EmojiFontFallback::is_emoji_code_point(cp),
            row["value"].as_bool().unwrap(),
            "code point {cp}"
        );
    }
    let expected = include_bytes!("../../../tests/fixtures/emoji-font-classification.bin");
    let mut actual = vec![0u8; 0x110000 / 8];
    for cp in 0..0x110000 {
        if EmojiFontFallback::is_emoji_code_point(cp) {
            actual[cp as usize / 8] |= 1 << (cp % 8);
        }
    }
    assert_eq!(
        actual.as_slice(),
        expected,
        "full unchanged C# classifier bitmap"
    );
}
