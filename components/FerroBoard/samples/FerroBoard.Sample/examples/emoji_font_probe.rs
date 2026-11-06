//! Native backend smoke check; initializes graphics/font services without opening a window.
use ferroboard_controls::EmojiFont;
use ferroui_base::media::{FontManagerOptions, FontStretch, FontStyle, FontWeight};
use ferroui_controls::{AppBuilder, Application};
use ferroui_desktop::AppBuilderDesktopExtensions;
use std::{cell::Cell, rc::Rc};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args_os()
        .nth(1)
        .ok_or("usage: emoji_font_probe FONT_FILE [HEX_CODEPOINT]")?;
    let codepoint = i32::from_str_radix(
        &std::env::args().nth(2).unwrap_or_else(|| "2122".into()),
        16,
    )?;
    let font = EmojiFont::from_reader(std::fs::File::open(path)?)?;
    let family = font.font_family();
    let mut options = FontManagerOptions::default();
    font.configure(&mut options);
    let verified = Rc::new(Cell::new(false));
    let done = verified.clone();
    AppBuilder::configure::<Application>()
        .use_platform_detect()
        .with(Rc::new(options))
        .configure_fonts(move |manager| {
            font.install(manager).expect("native font installation");
            let matched = manager
                .try_match_character(
                    codepoint,
                    FontStyle::Normal,
                    FontWeight::Normal,
                    FontStretch::Normal,
                    None,
                    None,
                )
                .expect("requested character must be present in supplied font");
            assert_eq!(matched.font_family(), &family);
            done.set(true);
        })
        .setup_without_starting();
    assert!(verified.get());
    println!("native font loading and emoji fallback verified for U+{codepoint:04X}");
    Ok(())
}
