//! The font files as embedded assets (the counterpart of the resource items
//! of the upstream project file).

use crate::ASSEMBLY_NAME;
use ferroui_base::platform::register_assets;
use std::sync::Once;

static ASSETS: &[(&str, &[u8])] = &[
    ("/Assets/Inter-Bold.ttf", include_bytes!("Assets/Inter-Bold.ttf")),
    ("/Assets/Inter-Light.ttf", include_bytes!("Assets/Inter-Light.ttf")),
    ("/Assets/Inter-Medium.ttf", include_bytes!("Assets/Inter-Medium.ttf")),
    ("/Assets/Inter-Regular.ttf", include_bytes!("Assets/Inter-Regular.ttf")),
    ("/Assets/Inter-SemiBold.ttf", include_bytes!("Assets/Inter-SemiBold.ttf")),
    ("/Assets/Inter-Thin.ttf", include_bytes!("Assets/Inter-Thin.ttf")),
];

/// Registers the fonts with the asset loader under the name of the assembly.
/// Cheap and idempotent.
pub(crate) fn register() {
    static REGISTERED: Once = Once::new();
    REGISTERED.call_once(|| register_assets(ASSEMBLY_NAME, ASSETS));
}
