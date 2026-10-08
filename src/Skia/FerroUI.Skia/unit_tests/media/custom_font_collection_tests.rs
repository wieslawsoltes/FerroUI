//! Port of upstream's `Media/CustomFontCollectionTests.cs` of the Skia unit
//! tests.
//!
//! Upstream reads the test fonts from the `Assets` folder of the test
//! output directory (`AppContext.BaseDirectory`); here that is the
//! `test_assets/assets` folder of the crate (the folder name is lower
//! case). Upstream builds the file URI
//! from the bare path (`new Uri(path, UriKind.Absolute)`, an implicit file
//! URI); the port's `Uri` has no implicit file URIs, so the tests spell out
//! the `file://` scheme.

use crate::unit_tests::mock_platform_render_interface;
use crate::FontManagerImpl;
use ferroui_base::media::fonts::{FontCollectionBase, FontCollectionBaseImpl, IFontCollection};
use ferroui_base::media::{FontManager, FontStretch, FontStyle, FontWeight, GlyphTypeface, Typeface};
use ferroui_base::platform::IAssetLoader;
use ferroui_base::utilities::{Uri, UriKind};
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_controls::testing::UnitTestApplication;
use std::path::{PathBuf, MAIN_SEPARATOR};
use std::rc::Rc;

const ASSETS_NAMESPACE: &str = "FerroUI.Skia.UnitTests.Assets";
const ASSET_FONTS: &str = "resm:FerroUI.Skia.UnitTests.Assets?assembly=ferroui-skia";

#[test]
fn should_add_glyph_typeface_by_stream() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let font_manager = FontManager::current();

    let font_collection = CustomFontCollection::new(Uri::new("fonts:custom", UriKind::Absolute).unwrap());

    font_manager.add_font_collection(font_collection.clone());

    let asset_loader = FerroLocator::current().get_required_service::<dyn IAssetLoader>();

    let infos = [
        FontAssetInfo::new("AdobeBlank2VF.ttf", "Adobe Blank 2 VF R", FontWeight::Normal),
        FontAssetInfo::new("Inter-Bold.ttf", "Inter", FontWeight::Bold),
        FontAssetInfo::new("Inter-Regular.ttf", "Inter", FontWeight::Normal),
        FontAssetInfo::new("InterVariable.ttf", "Inter Variable", FontWeight::Normal),
        FontAssetInfo::new("Manrope-Light.ttf", "Manrope Light", FontWeight::Light),
        FontAssetInfo::new("MiSans-Normal.ttf", "MiSans Normal", FontWeight(305)),
        FontAssetInfo::new("NISC18030.ttf", "GB18030 Bitmap", FontWeight::Normal),
        FontAssetInfo::new("NotoMono-Regular.ttf", "Noto Mono", FontWeight::Normal),
        FontAssetInfo::new("NotoSans-Italic.ttf", "Noto Sans", FontWeight::Normal),
        FontAssetInfo::new("NotoSansArabic-Regular.ttf", "Noto Sans Arabic", FontWeight::Normal),
        FontAssetInfo::new("NotoSansDeseret-Regular.ttf", "Noto Sans Deseret", FontWeight::Normal),
        FontAssetInfo::new("NotoSansHebrew-Regular.ttf", "Noto Sans Hebrew", FontWeight::Normal),
        FontAssetInfo::new("NotoSansMiao-Regular.ttf", "Noto Sans Miao", FontWeight::Normal),
        FontAssetInfo::new("NotoSansTamil-Regular.ttf", "Noto Sans Tamil", FontWeight::Normal),
        FontAssetInfo::new("PointMatch.ttf", "PointMatch", FontWeight::Normal),
        FontAssetInfo::new("SourceSerif4_36pt-Italic.ttf", "Source Serif 4 36pt", FontWeight::Normal),
        FontAssetInfo::new("TwitterColorEmoji-SVGinOT.ttf", "Twitter Color Emoji", FontWeight::Normal),
    ];

    let mut assets = asset_loader.get_assets(&Uri::new(ASSET_FONTS, UriKind::Absolute).unwrap(), None);

    assets.sort_by_key(|uri| uri.absolute_uri().to_uppercase());

    assert_eq!(infos.len(), assets.len());

    let mut glyph_typefaces: Vec<Rc<GlyphTypeface>> = Vec::with_capacity(infos.len());

    // Load fonts
    for i in 0..infos.len() {
        let info = &infos[i];
        let asset = &assets[i];

        assert_eq!(info.path, asset.absolute_path());

        let mut font_stream = asset_loader.open(asset, None).expect("the font stream");

        let glyph_typeface = FontCollectionBase::try_add_glyph_typeface_from_stream(&*font_collection, &mut *font_stream)
            .unwrap_or_else(|| panic!("{} is added", info.path));

        assert_eq!(info.family_name, glyph_typeface.family_name());
        assert_eq!(info.weight, glyph_typeface.weight());

        glyph_typefaces.push(glyph_typeface);
    }

    // Check against the custom collection
    for i in 0..infos.len() {
        let info = &infos[i];
        let glyph_typeface = &glyph_typefaces[i];

        let second_glyph_typeface = font_manager
            .try_get_glyph_typeface(&Typeface::from_name_with_style(
                &format!("fonts:custom#{}", info.family_name),
                FontStyle::Normal,
                info.weight,
                FontStretch::Normal,
            ))
            .unwrap_or_else(|| panic!("{} is found", info.family_name));

        assert!(Rc::ptr_eq(glyph_typeface, &second_glyph_typeface), "{}", info.path);
    }
}

#[test]
fn should_enumerate_font_families() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let font_manager = FontManager::current();

    let font_collection = CustomFontCollection::new(Uri::new("fonts:custom", UriKind::Absolute).unwrap());

    font_manager.add_font_collection(font_collection.clone());

    let asset_loader = FerroLocator::current().get_required_service::<dyn IAssetLoader>();

    let assets: Vec<Uri> = asset_loader
        .get_assets(&Uri::new(ASSET_FONTS, UriKind::Absolute).unwrap(), None)
        .into_iter()
        .filter(|x| x.absolute_path().ends_with(".ttf"))
        .collect();

    for asset in &assets {
        let mut stream = asset_loader.open(asset, None).expect("the font stream");

        FontCollectionBase::try_add_glyph_typeface_from_stream(&*font_collection, &mut *stream);
    }

    let families = font_collection.font_families();

    assert!(families.len() >= assets.len());

    let other = CustomFontCollection::new(Uri::new("fonts:other", UriKind::Absolute).unwrap());

    for family in &families {
        let family_typefaces = family.family_typefaces();

        for typeface in &family_typefaces {
            FontCollectionBase::try_add_glyph_typeface(&*other, &typeface.glyph_typeface());
        }
    }

    assert_eq!(families.len(), other.count());

    for i in 0..families.len() {
        assert_eq!(families[i].name(), other.get(i).name());
    }
}

#[test]
fn should_add_font_source_from_file() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let font_manager = FontManager::current();
    let font_collection = CustomFontCollection::new(Uri::new("fonts:custom", UriKind::Absolute).unwrap());
    font_manager.add_font_collection(font_collection.clone());

    // Path to the test font
    let font_path = base_directory().join("assets").join("Inter-Regular.ttf");
    assert!(font_path.is_file());

    let font_uri = Uri::new(&file_uri(&font_path.to_string_lossy()), UriKind::Absolute).unwrap();

    // Add the font file
    assert!(FontCollectionBase::try_add_font_source(&*font_collection, &font_uri));

    // Check if the font was loaded
    let glyph_typeface = font_collection
        .try_get_glyph_typeface("Inter", FontStyle::Normal, FontWeight::Regular, FontStretch::Normal)
        .expect("the font is loaded");
    assert_eq!("Inter", glyph_typeface.family_name());

    // Check if the FontManager can find the font
    let glyph_typeface2 = font_manager
        .try_get_glyph_typeface(&Typeface::from_name("fonts:custom#Inter"))
        .expect("the font manager finds the font");
    assert!(Rc::ptr_eq(&glyph_typeface, &glyph_typeface2));
}

#[test]
fn should_add_font_source_from_folder() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let font_manager = FontManager::current();
    let font_collection = CustomFontCollection::new(Uri::new("fonts:custom", UriKind::Absolute).unwrap());
    font_manager.add_font_collection(font_collection.clone());

    // Path to the test fonts
    let fonts_folder = base_directory().join("assets");
    assert!(fonts_folder.is_dir());

    let folder_uri =
        Uri::new(&file_uri(&format!("{}{}", fonts_folder.to_string_lossy(), MAIN_SEPARATOR)), UriKind::Absolute)
            .unwrap();

    // Add the fonts
    assert!(FontCollectionBase::try_add_font_source(&*font_collection, &folder_uri));

    // Check if the font was loaded
    let glyph_typeface = font_collection
        .try_get_glyph_typeface("Inter", FontStyle::Normal, FontWeight::Regular, FontStretch::Normal)
        .expect("the font is loaded");
    assert_eq!("Inter", glyph_typeface.family_name());

    // Check if the FontManager can find the font
    let glyph_typeface2 = font_manager
        .try_get_glyph_typeface(&Typeface::from_name("fonts:custom#Inter"))
        .expect("the font manager finds the font");
    assert!(Rc::ptr_eq(&glyph_typeface, &glyph_typeface2));
}

#[test]
fn should_add_font_source_from_resource() {
    let _app = UnitTestApplication::start(
        mock_platform_render_interface().with_font_manager_impl(Rc::new(FontManagerImpl::new())),
    );

    let font_manager = FontManager::current();
    let font_collection = CustomFontCollection::new(Uri::new("fonts:custom", UriKind::Absolute).unwrap());
    font_manager.add_font_collection(font_collection.clone());

    let all_fonts_uri = Uri::new(ASSET_FONTS, UriKind::Absolute).unwrap();

    // Add the font resource
    assert!(FontCollectionBase::try_add_font_source(&*font_collection, &all_fonts_uri));

    // Get the loaded family names
    let families = font_collection.font_families();

    assert!(!families.is_empty());

    // Try to get a GlyphTypeface
    let glyph_typeface = font_collection
        .try_get_glyph_typeface("Noto Mono", FontStyle::Normal, FontWeight::Regular, FontStretch::Normal)
        .expect("the font is loaded");
    assert_eq!("Noto Mono", glyph_typeface.family_name());

    // Check if the FontManager can find the font
    let glyph_typeface2 = font_manager
        .try_get_glyph_typeface(&Typeface::from_name("fonts:custom#Noto Mono"))
        .expect("the font manager finds the font");
    assert!(Rc::ptr_eq(&glyph_typeface, &glyph_typeface2));
}

/// Upstream's `AppContext.BaseDirectory`, the folder the test fonts are
/// copied to: the crate's `test_assets` folder.
fn base_directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("test_assets")
}

/// The file URI of an absolute path.
fn file_uri(path: &str) -> String {
    let path = path.replace('\\', "/");

    if path.starts_with('/') {
        format!("file://{path}")
    } else {
        format!("file:///{path}")
    }
}

struct CustomFontCollection {
    base: FontCollectionBase,
    key: Uri,
}

impl CustomFontCollection {
    fn new(key: Uri) -> Rc<Self> {
        Rc::new(Self { base: FontCollectionBase::new(), key })
    }
}

impl FontCollectionBaseImpl for CustomFontCollection {
    fn base(this: &Self) -> &FontCollectionBase {
        &this.base
    }

    fn key(this: &Self) -> Uri {
        this.key.clone()
    }
}

struct FontAssetInfo {
    path: String,
    family_name: &'static str,
    weight: FontWeight,
}

impl FontAssetInfo {
    fn new(file_name: &str, family_name: &'static str, weight: FontWeight) -> Self {
        Self { path: format!("{ASSETS_NAMESPACE}.{file_name}"), family_name, weight }
    }
}
