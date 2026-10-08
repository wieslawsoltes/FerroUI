use crate::platform::IAssetLoader;
use crate::utilities::{Uri, UriExtensions, UriKind};
use crate::{FerroLocator, LocatorExtensions};

/// Finds the font assets of a font family source.
pub struct FontFamilyLoader;

impl FontFamilyLoader {
    /// Loads all font assets that belong to the specified source: a single
    /// font file, a file name pattern (`MyFont*.ttf`) or a location holding
    /// font files.
    ///
    /// Panics when no asset loader is registered with the locator.
    pub fn load_font_assets(source: &Uri) -> Vec<Uri> {
        if UriExtensions::is_asset(source) || UriExtensions::is_absolute_resm(source) {
            return if Self::is_font_source(source) {
                Self::get_font_assets_by_expression(source)
            } else {
                Self::get_font_assets_by_source(source)
            };
        }

        Vec::new()
    }

    /// Whether the URI (without its query) names a font file.
    pub fn is_font_source(uri: &Uri) -> bool {
        let source_without_arguments = Self::get_sub_string(uri.original_string(), '?');
        Self::is_font_file(source_without_arguments)
    }

    /// Whether the path has a font file extension (`.ttf`, `.otf`, `.ttc`).
    pub fn is_font_file(file_path: &str) -> bool {
        ends_with_ignore_ascii_case(file_path, ".ttf")
            || ends_with_ignore_ascii_case(file_path, ".otf")
            || ends_with_ignore_ascii_case(file_path, ".ttc")
    }

    /// Searches for font assets at a given location and returns a list of them.
    fn get_font_assets_by_source(source: &Uri) -> Vec<Uri> {
        let asset_loader = FerroLocator::current().get_required_service::<dyn IAssetLoader>();
        let mut available_assets = asset_loader.get_assets(source, None);
        available_assets.retain(Self::is_font_source);
        available_assets
    }

    /// Searches for font assets at a given location and only accepts assets
    /// that fit to a given filename expression. File names can target
    /// multiple files with `*` wildcard, for example `FontFile*.ttf`.
    fn get_font_assets_by_expression(source: &Uri) -> Vec<Uri> {
        let Some(((file_name_without_extension, extension), location)) = Self::get_file_name(source) else {
            return Vec::new();
        };

        let file_pattern = Self::create_file_pattern(source, &location, &file_name_without_extension);

        let asset_loader = FerroLocator::current().get_required_service::<dyn IAssetLoader>();
        let mut available_resources = asset_loader.get_assets(&location, None);

        available_resources.retain(|asset| Self::is_contains_file(asset, &file_pattern, &extension));
        available_resources
    }

    /// The file name (without extension) and the extension of the source, and
    /// the location the file is in. `None` when the location is not a valid URI.
    fn get_file_name(source: &Uri) -> Option<((String, String), Uri)> {
        if UriExtensions::is_absolute_resm(source) {
            let file_name = Self::get_file_name_and_extension(&UriExtensions::get_unescape_absolute_path(source), '.');

            let uri_location =
                UriExtensions::get_unescape_absolute_uri(source).replace(&format!(".{}{}", file_name.0, file_name.1), "");
            let location = Uri::try_create(&uri_location, UriKind::RelativeOrAbsolute)?;

            return Some((file_name, location));
        }

        let file_name = Self::get_file_name_and_extension(source.original_string(), '/');
        let full_filename = format!("{}{}", file_name.0, file_name.1);

        let uri_string = UriExtensions::get_unescape_absolute_uri(source).replace(&full_filename, "");
        let location = Uri::try_create(&uri_string, UriKind::Absolute)?;

        Some((file_name, location))
    }

    fn create_file_pattern(source: &Uri, location: &Uri, file_name_without_extension: &str) -> String {
        let path = UriExtensions::get_unescape_absolute_path(location);
        let file = Self::get_sub_string(file_name_without_extension, '*');

        if UriExtensions::is_absolute_resm(source) {
            format!("{path}.{file}")
        } else {
            format!("{path}{file}")
        }
    }

    fn is_contains_file(asset: &Uri, file_pattern: &str, file_extension: &str) -> bool {
        let path = UriExtensions::get_unescape_absolute_path(asset);
        path.contains(file_pattern) && ends_with_ignore_ascii_case(&path, file_extension)
    }

    fn get_file_name_and_extension(path: &str, directory_separator: char) -> (String, String) {
        let path = if Self::is_path_rooted(path, directory_separator) {
            &path[directory_separator.len_utf8()..]
        } else {
            path
        };

        let extension = Self::get_file_extension(path);

        if extension.len() == path.len() {
            return (extension.to_owned(), String::new());
        }

        let file_name = Self::get_file_name_part(path, directory_separator, extension.len());

        (file_name.to_owned(), extension.to_owned())
    }

    fn is_path_rooted(path: &str, directory_separator: char) -> bool {
        path.starts_with(directory_separator)
    }

    /// The extension including its dot; the whole path when it has no dot
    /// after its first character.
    fn get_file_extension(path: &str) -> &str {
        match path.rfind('.') {
            Some(index) if index > 0 => &path[index..],
            _ => path,
        }
    }

    fn get_file_name_part(path: &str, directory_separator: char, extension_length: usize) -> &str {
        let without_extension = &path[..path.len() - extension_length];

        match without_extension.rfind(directory_separator) {
            Some(index) => &without_extension[index + directory_separator.len_utf8()..],
            None => without_extension,
        }
    }

    fn get_sub_string(path: &str, separator: char) -> &str {
        match path.find(separator) {
            Some(index) => &path[..index],
            None => path,
        }
    }
}

/// C# `EndsWith(value, StringComparison.OrdinalIgnoreCase)` for ASCII suffixes.
fn ends_with_ignore_ascii_case(value: &str, suffix: &str) -> bool {
    let (value, suffix) = (value.as_bytes(), suffix.as_bytes());

    value.len() >= suffix.len() && value[value.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::media::fonts::testing::{test_fonts, TestAssetLoader, TestFontManagerImpl, TestFontScope};

    const FONT_NAME: &str = "#MyFont";
    const ASSEMBLY: &str = "?assembly=FerroUI.Visuals.UnitTests";
    const ASSET_LOCATION: &str = "resm:FerroUI.Visuals.UnitTests.Assets";
    const ASSET_LOCATION_FERRES: &str = "ferres://FerroUI.Visuals.UnitTests";
    const ASSET_YOUR_FILE_NAME: &str = "/Assets/YourFont.ttf";

    fn asset(file_name: &str) -> String {
        format!("{ASSET_LOCATION}{file_name}{ASSEMBLY}{FONT_NAME}")
    }

    fn asset_my_font_regular() -> String {
        asset(".MyFont Regular.ttf")
    }

    fn asset_your_font_ferres() -> String {
        format!("{ASSET_LOCATION_FERRES}{ASSET_YOUR_FILE_NAME}")
    }

    fn start_with_resources() -> TestFontScope {
        let asset_loader = TestAssetLoader::new();

        for uri in [asset_my_font_regular(), asset(".MyFont Bold.ttf"), asset(".YourFont.ttf"), asset_your_font_ferres()]
        {
            asset_loader.add_asset(&uri, b"AssetData".to_vec());
        }

        TestFontScope::with_services(TestFontManagerImpl::headless(), Rc::new(asset_loader))
    }

    #[test]
    fn should_load_single_font_asset() {
        let _scope = start_with_resources();

        let source = Uri::new(&asset_my_font_regular(), UriKind::RelativeOrAbsolute).unwrap();

        let font_assets = FontFamilyLoader::load_font_assets(&source);

        assert_eq!(font_assets.len(), 1);
    }

    #[test]
    fn should_load_single_font_asset_ferres_without_base_uri() {
        let _scope = start_with_resources();

        let source = Uri::absolute(&asset_your_font_ferres()).unwrap();

        let font_assets = FontFamilyLoader::load_font_assets(&source);

        assert_eq!(font_assets.len(), 1);
    }

    #[test]
    fn should_load_single_font_asset_ferres_with_base_uri() {
        let _scope = start_with_resources();

        let source = Uri::new(ASSET_YOUR_FILE_NAME, UriKind::RelativeOrAbsolute).unwrap();
        let base_uri = Uri::absolute(ASSET_LOCATION_FERRES).unwrap();

        let font_assets = FontFamilyLoader::load_font_assets(&Uri::combine(&base_uri, &source));

        assert_eq!(font_assets.len(), 1);
    }

    #[test]
    fn should_load_matching_assets() {
        let _scope = start_with_resources();

        let source = Uri::new(&asset(".MyFont*.ttf"), UriKind::RelativeOrAbsolute).unwrap();

        let font_assets = FontFamilyLoader::load_font_assets(&source);

        assert_eq!(font_assets.len(), 2);
    }

    #[test]
    fn should_load_embedded_font() {
        let scope = TestFontScope::with_assets();

        let source = Uri::new(&format!("{}#Noto Mono", test_fonts::ASSETS), UriKind::RelativeOrAbsolute).unwrap();

        let font_assets = FontFamilyLoader::load_font_assets(&source);

        assert!(!font_assets.is_empty());

        for font_asset in &font_assets {
            assert!(scope.asset_loader().open(font_asset, None).is_ok());
        }
    }

    #[test]
    fn other_schemes_have_no_font_assets() {
        let _scope = start_with_resources();

        assert!(FontFamilyLoader::load_font_assets(&Uri::absolute("fonts:Inter").unwrap()).is_empty());
        assert!(FontFamilyLoader::load_font_assets(&Uri::absolute("file:///fonts/a.ttf").unwrap()).is_empty());
    }

    #[test]
    fn font_files_are_recognized_by_extension() {
        for path in ["a.ttf", "A.TTF", "dir/b.otf", "c.Ttc"] {
            assert!(FontFamilyLoader::is_font_file(path), "{path}");
        }

        for path in ["a.txt", "ttf", "a.ttf.bak", ""] {
            assert!(!FontFamilyLoader::is_font_file(path), "{path}");
        }

        assert!(FontFamilyLoader::is_font_source(&Uri::absolute("resm:A.B.c.ttf?assembly=A#Name").unwrap()));
        assert!(!FontFamilyLoader::is_font_source(&Uri::absolute("resm:A.B?assembly=A.ttf").unwrap()));
    }

    #[test]
    fn file_names_are_split_from_their_extension() {
        assert_eq!(
            FontFamilyLoader::get_file_name_and_extension("/Assets/Fonts/My Font.ttf", '/'),
            ("My Font".to_owned(), ".ttf".to_owned())
        );
        assert_eq!(
            FontFamilyLoader::get_file_name_and_extension("App.Assets.MyFont*.ttf", '.'),
            ("MyFont*".to_owned(), ".ttf".to_owned())
        );
        // Without an extension the whole path is the name.
        assert_eq!(FontFamilyLoader::get_file_name_and_extension("Fonts", '/'), ("Fonts".to_owned(), String::new()));
    }
}
