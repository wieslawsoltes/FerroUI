//! Port of upstream's Skia unit test project (`tests/*.Skia.UnitTests`):
//! the suites under their upstream names, one module per upstream file, and
//! the test doubles they share.
//!
//! The fonts upstream embeds as manifest resources are registered here
//! under the same resource names (`FerroUI.Skia.UnitTests.Assets.*` for the
//! render test assets, `FerroUI.Skia.UnitTests.Fonts.*` for the fonts of
//! the project) in the assembly `ferroui-skia`. Upstream's `Win32Fact` and
//! `Win32Theory` (skipped unless the tests run on Windows) are
//! `#[cfg_attr(not(windows), ignore = "...")]` with upstream's message.

mod combined_geometry_impl_tests;
mod drawing_context_impl_tests;
mod hit_testing;
mod media;
mod render_bounds_tests;
mod skia_options_tests;
mod test_font_manager;

pub(crate) use test_font_manager::TestFontManager;

use ferroui_base::platform::{register_manifest_resources, IPlatformRenderInterface, ITextShaperImpl, StandardAssetLoader};
use ferroui_base::rendering::testing::{DrawingLog, MockPlatformRenderInterface};
use ferroui_controls::testing::TestServices;
use ferroui_harfbuzz::HarfBuzzTextShaper;
use std::rc::Rc;
use std::sync::Once;

/// The assembly the test fonts are manifest resources of.
pub(crate) const ASSEMBLY: &str = "ferroui-skia";

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
/// Upstream's render interface is the headless one; the port has no
/// headless platform, so it is the mock render interface of the base
/// crate's render test doubles.
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

static RESOURCES: &[(&str, &[u8])] = &[
        ("FerroUI.Skia.UnitTests.Assets.AdobeBlank2VF.ttf", include_bytes!("../test_assets/assets/AdobeBlank2VF.ttf")),
        ("FerroUI.Skia.UnitTests.Assets.Inter-Bold.ttf", include_bytes!("../test_assets/assets/Inter-Bold.ttf")),
        ("FerroUI.Skia.UnitTests.Assets.Inter-Regular.ttf", include_bytes!("../test_assets/assets/Inter-Regular.ttf")),
        ("FerroUI.Skia.UnitTests.Assets.InterVariable.ttf", include_bytes!("../test_assets/assets/InterVariable.ttf")),
        ("FerroUI.Skia.UnitTests.Assets.Manrope-Light.ttf", include_bytes!("../test_assets/assets/Manrope-Light.ttf")),
        ("FerroUI.Skia.UnitTests.Assets.MiSans-Normal.ttf", include_bytes!("../test_assets/assets/MiSans-Normal.ttf")),
        ("FerroUI.Skia.UnitTests.Assets.NISC18030.ttf", include_bytes!("../test_assets/assets/NISC18030.ttf")),
        ("FerroUI.Skia.UnitTests.Assets.NotoMono-Regular.ttf", include_bytes!("../test_assets/assets/NotoMono-Regular.ttf")),
        ("FerroUI.Skia.UnitTests.Assets.NotoSans-Italic.ttf", include_bytes!("../test_assets/assets/NotoSans-Italic.ttf")),
        ("FerroUI.Skia.UnitTests.Assets.NotoSansArabic-Regular.ttf", include_bytes!("../test_assets/assets/NotoSansArabic-Regular.ttf")),
        ("FerroUI.Skia.UnitTests.Assets.NotoSansDeseret-Regular.ttf", include_bytes!("../test_assets/assets/NotoSansDeseret-Regular.ttf")),
        ("FerroUI.Skia.UnitTests.Assets.NotoSansHebrew-Regular.ttf", include_bytes!("../test_assets/assets/NotoSansHebrew-Regular.ttf")),
        ("FerroUI.Skia.UnitTests.Assets.NotoSansMiao-Regular.ttf", include_bytes!("../test_assets/assets/NotoSansMiao-Regular.ttf")),
        ("FerroUI.Skia.UnitTests.Assets.NotoSansTamil-Regular.ttf", include_bytes!("../test_assets/assets/NotoSansTamil-Regular.ttf")),
        ("FerroUI.Skia.UnitTests.Assets.PointMatch.ttf", include_bytes!("../test_assets/assets/PointMatch.ttf")),
        ("FerroUI.Skia.UnitTests.Assets.SourceSerif4_36pt-Italic.ttf", include_bytes!("../test_assets/assets/SourceSerif4_36pt-Italic.ttf")),
        ("FerroUI.Skia.UnitTests.Assets.TwitterColorEmoji-SVGinOT.ttf", include_bytes!("../test_assets/assets/TwitterColorEmoji-SVGinOT.ttf")),
        ("FerroUI.Skia.UnitTests.Fonts.BareMinimum.ttf", include_bytes!("../test_assets/fonts/BareMinimum.ttf")),
        ("FerroUI.Skia.UnitTests.Fonts.CascadiaCode.ttf", include_bytes!("../test_assets/fonts/CascadiaCode.ttf")),
        ("FerroUI.Skia.UnitTests.Fonts.DF7segHMI.ttf", include_bytes!("../test_assets/fonts/DF7segHMI.ttf")),
        ("FerroUI.Skia.UnitTests.Fonts.DejaVuSans.ttf", include_bytes!("../test_assets/fonts/DejaVuSans.ttf")),
        ("FerroUI.Skia.UnitTests.Fonts.Inter-Regular.LineGap800.ttf", include_bytes!("../test_assets/fonts/Inter-Regular.LineGap800.ttf")),
        ("FerroUI.Skia.UnitTests.Fonts.Manrope-Light.ttf", include_bytes!("../test_assets/fonts/Manrope-Light.ttf")),
        ("FerroUI.Skia.UnitTests.Fonts.NotoSansArabic-NoLayout.ttf", include_bytes!("../test_assets/fonts/NotoSansArabic-NoLayout.ttf")),
        ("FerroUI.Skia.UnitTests.Fonts.NotoSansJP-Subset.ttf", include_bytes!("../test_assets/fonts/NotoSansJP-Subset.ttf")),
        ("FerroUI.Skia.UnitTests.Fonts.NotoSansSC-Subset.ttf", include_bytes!("../test_assets/fonts/NotoSansSC-Subset.ttf")),
        ("FerroUI.Skia.UnitTests.Fonts.TestFontNoCmap412.ttf", include_bytes!("../test_assets/fonts/TestFontNoCmap412.ttf")),
        ("FerroUI.Skia.UnitTests.Fonts.WinSymbols3.ttf", include_bytes!("../test_assets/fonts/WinSymbols3.ttf")),
];
