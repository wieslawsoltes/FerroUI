//! What the text suites share: the test fonts as manifest resources and the
//! services of upstream's `TestServices.MockPlatformRenderInterface`.
//!
//! The fonts are the ones of the Skia backend's unit tests
//! (`src/Skia/FerroUI.Skia/test_assets`, 31 MB): they are included from
//! there and not copied, and registered under the resource names of this
//! crate (`FerroUI.Vello.UnitTests.Assets.*` for the render test assets,
//! `FerroUI.Vello.UnitTests.Fonts.*` for the fonts of the test project) in
//! the assembly `ferroui-vello`.

use super::TestFontManager;
use ferroui_base::platform::{register_manifest_resources, IPlatformRenderInterface, ITextShaperImpl, StandardAssetLoader};
use ferroui_base::rendering::testing::{DrawingLog, MockPlatformRenderInterface};
use ferroui_controls::testing::TestServices;
use ferroui_harfbuzz::HarfBuzzTextShaper;
use std::rc::Rc;
use std::sync::Once;

/// The assembly the test fonts are manifest resources of.
pub(crate) const ASSEMBLY: &str = "ferroui-vello";

/// Registers the embedded test fonts and loads the Inter font assembly,
/// which upstream's test project references. Idempotent.
pub(crate) fn register_test_assets() {
    static REGISTERED: Once = Once::new();

    REGISTERED.call_once(|| {
        register_manifest_resources(ASSEMBLY, RESOURCES);

        // The assets of a referenced assembly are there from the start.
        ferroui_fonts_inter::register_assets();
    });
}

/// Upstream's `TestServices.MockPlatformRenderInterface`: the standard asset
/// loader, a render interface that records nothing, the test font manager
/// and the HarfBuzz text shaper. Tests replace services with the `with_*`
/// methods, as upstream's `With(...)`.
///
/// Upstream's render interface is the headless one; here it is the mock
/// render interface of the base crate's render test doubles, as in the
/// suites of the Skia backend.
pub(crate) fn mock_platform_render_interface() -> TestServices {
    register_test_assets();

    TestServices {
        asset_loader: Some(Rc::new(StandardAssetLoader::new(None))),
        render_interface: Some(
            MockPlatformRenderInterface::new(DrawingLog::new()) as Rc<dyn IPlatformRenderInterface>
        ),
        font_manager_impl: Some(Rc::new(TestFontManager::new())),
        text_shaper_impl: Some(Rc::new(HarfBuzzTextShaper::new()) as Rc<dyn ITextShaperImpl>),
        ..TestServices::default()
    }
}

macro_rules! test_fonts {
    ($(($folder:literal, $namespace:literal, $file:literal)),* $(,)?) => {
        &[$((
            concat!("FerroUI.Vello.UnitTests.", $namespace, ".", $file),
            include_bytes!(concat!("../../../Skia/FerroUI.Skia/test_assets/", $folder, "/", $file)) as &[u8],
        )),*]
    };
}

static RESOURCES: &[(&str, &[u8])] = test_fonts![
    ("assets", "Assets", "AdobeBlank2VF.ttf"),
    ("assets", "Assets", "Inter-Bold.ttf"),
    ("assets", "Assets", "Inter-Regular.ttf"),
    ("assets", "Assets", "InterVariable.ttf"),
    ("assets", "Assets", "Manrope-Light.ttf"),
    ("assets", "Assets", "MiSans-Normal.ttf"),
    ("assets", "Assets", "NISC18030.ttf"),
    ("assets", "Assets", "NotoMono-Regular.ttf"),
    ("assets", "Assets", "NotoSans-Italic.ttf"),
    ("assets", "Assets", "NotoSansArabic-Regular.ttf"),
    ("assets", "Assets", "NotoSansDeseret-Regular.ttf"),
    ("assets", "Assets", "NotoSansHebrew-Regular.ttf"),
    ("assets", "Assets", "NotoSansMiao-Regular.ttf"),
    ("assets", "Assets", "NotoSansTamil-Regular.ttf"),
    ("assets", "Assets", "PointMatch.ttf"),
    ("assets", "Assets", "SourceSerif4_36pt-Italic.ttf"),
    ("assets", "Assets", "TwitterColorEmoji-SVGinOT.ttf"),
    ("fonts", "Fonts", "BareMinimum.ttf"),
    ("fonts", "Fonts", "CascadiaCode.ttf"),
    ("fonts", "Fonts", "DF7segHMI.ttf"),
    ("fonts", "Fonts", "DejaVuSans.ttf"),
    ("fonts", "Fonts", "Inter-Regular.LineGap800.ttf"),
    ("fonts", "Fonts", "Manrope-Light.ttf"),
    ("fonts", "Fonts", "NotoSansArabic-NoLayout.ttf"),
    ("fonts", "Fonts", "NotoSansJP-Subset.ttf"),
    ("fonts", "Fonts", "NotoSansSC-Subset.ttf"),
    ("fonts", "Fonts", "TestFontNoCmap412.ttf"),
    ("fonts", "Fonts", "WinSymbols3.ttf"),
];
