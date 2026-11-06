use ferroboard_sample::App;
use ferroui_controls::AppBuilder;
use ferroui_desktop::AppBuilderDesktopExtensions;
fn main() -> std::process::ExitCode {
    // Initialize the OS locale at the composition root. Hosted consumers retain their
    // explicitly selected culture when they construct App themselves.
    let culture = match ferroboard_core::text::locale_text::system_locale_name() {
        Ok(culture) => culture,
        Err(error) => {
            eprintln!("FerroBoard culture initialization failed: {error}");
            return std::process::ExitCode::FAILURE;
        }
    };
    ferroui_base::utilities::CultureInfo::set_current_formatting(
        ferroui_base::utilities::CultureInfo::get_culture_info(&culture),
    );
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let builder = AppBuilder::configure::<App>().use_platform_detect();
    let builder = if let Some(path) = std::env::var_os("FERROBOARD_EMOJI_FONT") {
        let font = match std::fs::File::open(&path)
            .map_err(ferroboard_controls::EmojiFontError::Read)
            .and_then(ferroboard_controls::EmojiFont::from_reader)
        {
            Ok(font) => font,
            Err(error) => {
                eprintln!("FerroBoard emoji font: {error}");
                return std::process::ExitCode::FAILURE;
            }
        };
        let mut options = ferroui_base::media::FontManagerOptions::default();
        font.configure(&mut options);
        builder
            .with(std::rc::Rc::new(options))
            .configure_fonts(move |manager| {
                if let Err(error) = font.install(manager) {
                    eprintln!("FerroBoard emoji font: {error}; retaining system font fallback");
                }
            })
    } else {
        builder
    };
    let code = builder.start_with_classic_desktop_lifetime(&args);
    std::process::ExitCode::from(code as u8)
}
