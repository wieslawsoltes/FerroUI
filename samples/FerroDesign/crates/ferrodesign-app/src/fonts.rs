//! Application fonts use the same public collection and fallback path on
//! native and browser platforms. Hebrew and emoji coverage does not depend on
//! fonts installed on the host.
use ferroui_base::media::fonts::EmbeddedFontCollection;
use ferroui_base::media::{
    FontFallback, FontFamily, FontManager, FontManagerOptions, UnicodeRange,
};
use ferroui_base::platform::{AssetLoader, register_assets};
use ferroui_base::utilities::Uri;
use std::{rc::Rc, sync::Once};

pub(crate) const HEBREW_FAMILY: &str = "fonts:FerroDesignFallback#Noto Sans Hebrew";
pub(crate) const EMOJI_FAMILY: &str = "fonts:FerroDesignFallback#Noto Color Emoji";

/// Keep Inter as the default and resolve only its missing characters through
/// the embedded fallback faces. FontManager checks actual glyph coverage.
pub(crate) fn options() -> FontManagerOptions {
    FontManagerOptions {
        default_family_name: Some("fonts:Inter#Inter".into()),
        font_fallbacks: Some(vec![
            FontFallback {
                font_family: FontFamily::new(HEBREW_FAMILY),
                unicode_range: UnicodeRange::default_range(),
            },
            FontFallback {
                font_family: FontFamily::new(EMOJI_FAMILY),
                unicode_range: UnicodeRange::default_range(),
            },
        ]),
        ..Default::default()
    }
}

pub(crate) fn register_fallbacks(font_manager: &Rc<FontManager>) {
    static REGISTERED: Once = Once::new();
    REGISTERED.call_once(|| {
        register_assets(
            crate::ASSEMBLY.name,
            &[
                (
                    "/Fonts/NotoSansHebrew-Regular.ttf",
                    include_bytes!("../Assets/Fonts/NotoSansHebrew-Regular.ttf"),
                ),
                (
                    "/Fonts/NotoSansHebrew-Bold.ttf",
                    include_bytes!("../Assets/Fonts/NotoSansHebrew-Bold.ttf"),
                ),
                (
                    "/Fonts/NotoColorEmoji-COLRv1.ttf",
                    include_bytes!("../Assets/Fonts/NotoColorEmoji-COLRv1.ttf"),
                ),
            ],
        );
    });
    // Compiled XAML can resolve this assembly during application setup before
    // the font callback runs. Refresh its asset snapshot after registration.
    AssetLoader::invalidate_assembly_cache(crate::ASSEMBLY.name);
    font_manager.add_font_collection(Rc::new(EmbeddedFontCollection::new(
        Uri::absolute("fonts:FerroDesignFallback").expect("a valid font collection key"),
        Uri::absolute("ferres://FerroDesign/Fonts").expect("a valid embedded font source"),
    )));
}
