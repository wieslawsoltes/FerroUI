//! Original pinned control-detail and sample media exposed through the native asset loader.

use ferroui_base::media::imaging::Bitmap;
use ferroui_base::platform::AssetLoader;
use ferroui_base::utilities::{Uri, UriKind};
use std::sync::Once;

pub(crate) fn register() {
    static REGISTER: Once = Once::new();
    REGISTER.call_once(|| {
        ferroui_base::platform::register_assets("WinUIGallery", crate::assets_data::ASSETS);
        // The sample uses the framework's actual audited dictionary even when
        // its optional application-wide theme is absent in a headless host.
        ferroui_base::platform::register_assets(
            "WinUIGallery",
            &[(
                "/Styles/InfoBadge.xaml",
                include_bytes!("../../src/FerroUI.Themes.WinUI/Controls/InfoBadge.xaml"),
            )],
        );
    });
}

pub(crate) fn load_bitmap(uri: &str) -> std::io::Result<Bitmap> {
    register();
    let uri = Uri::new(uri, UriKind::Absolute).map_err(|error| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, error.to_string())
    })?;
    let mut stream = AssetLoader::open(&uri, None)?;
    Bitmap::from_stream(&mut *stream)
}
