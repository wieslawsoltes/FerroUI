//! Fresh-process probe of first WinUI theme construction and deferred Button resource lookup.
use ferroui_base::controls::ResourceKey;
use ferroui_controls::Button;
use ferroui_themes_winui::WinUITheme;
use std::time::Instant;

fn main() {
    let start = Instant::now();
    let theme = WinUITheme::new();
    let created = start.elapsed();
    let button = theme.try_get_resource(&ResourceKey::Type(Button::TYPE), None).flatten();
    assert!(button.is_some(), "WinUI must resolve the Button control theme");
    println!("{} {}", created.as_nanos(), start.elapsed().as_nanos());
}
