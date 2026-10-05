use crate::platform::ASSET_SCHEME;
use crate::utilities::{FormatError, Uri};

/// Helpers for the URIs that identify assets.
pub struct UriExtensions;

impl UriExtensions {
    /// Whether the URI is an absolute embedded-resource URI.
    pub fn is_absolute_resm(uri: &Uri) -> bool {
        uri.is_absolute_uri() && Self::is_resm(uri)
    }

    /// Whether the URI has the embedded-resource scheme. Panics for a
    /// relative URI.
    pub fn is_resm(uri: &Uri) -> bool {
        uri.scheme() == "resm"
    }

    /// Whether the URI has the asset scheme ([`ASSET_SCHEME`]). Panics for a
    /// relative URI.
    pub fn is_asset(uri: &Uri) -> bool {
        uri.scheme() == ASSET_SCHEME
    }

    /// Whether the URI names a font collection (`fonts:` scheme). Panics for
    /// a relative URI.
    pub fn is_font_collection(uri: &Uri) -> bool {
        uri.scheme() == crate::media::FontManager::FONT_COLLECTION_SCHEME
    }

    /// Makes a relative URI absolute using `base_uri`; absolute URIs are
    /// returned unchanged. Fails when the URI is relative and `base_uri` is
    /// missing or relative.
    pub fn ensure_absolute(uri: &Uri, base_uri: Option<&Uri>) -> Result<Uri, FormatError> {
        if uri.is_absolute_uri() {
            return Ok(uri.clone());
        }
        let Some(base_uri) = base_uri else {
            return Err(FormatError::from_string(format!("Relative uri {uri} without base url")));
        };
        if !base_uri.is_absolute_uri() {
            return Err(FormatError::from_string(format!("Base uri {base_uri} is relative")));
        }
        Ok(Uri::combine(base_uri, uri))
    }

    /// The authority (host) of an absolute URI, or an empty string when it
    /// has none. Panics for a relative URI.
    pub fn authority(uri: &Uri) -> &str {
        let text = uri.absolute_uri();
        let rest = &text[uri.scheme().len() + 1..];
        match rest.strip_prefix("//") {
            Some(stripped) => &stripped[..stripped.find(['/', '?', '#']).unwrap_or(stripped.len())],
            None => "",
        }
    }

    /// The unescaped path of an absolute URI.
    pub fn get_unescape_absolute_path(uri: &Uri) -> String {
        Self::unescape_data_string(uri.absolute_path())
    }

    /// The unescaped text of an absolute URI.
    pub fn get_unescape_absolute_uri(uri: &Uri) -> String {
        Self::unescape_data_string(uri.absolute_uri())
    }

    /// The value of the `assembly` parameter of the query of an absolute
    /// URI, or an empty string.
    pub fn get_assembly_name_from_query(uri: &Uri) -> String {
        const ASSEMBLY: &[u8] = b"assembly";

        let query = Self::unescape_data_string(uri.query());
        let query = query.as_bytes();

        // Skip the '?'
        let mut current_index = 1;
        while current_index < query.len() {
            let mut is_find = false;
            let mut i = 0;
            while i < ASSEMBLY.len() {
                if current_index < query.len() && query[current_index] == ASSEMBLY[i] {
                    is_find = i == ASSEMBLY.len() - 1;
                } else {
                    break;
                }
                current_index += 1;
                i += 1;
            }

            // Skip the '='
            current_index += 1;

            let begin_index = current_index;
            while current_index < query.len() && query[current_index] != b'&' {
                current_index += 1;
            }

            if is_find {
                // A parameter without a value ends the query before its
                // value begins.
                let end_index = current_index.min(query.len());
                let begin_index = begin_index.min(end_index);
                return String::from_utf8_lossy(&query[begin_index..end_index]).into_owned();
            }

            current_index += 1;
        }

        String::new()
    }

    /// Converts the percent-encoded sequences of a string to the characters
    /// they represent. Sequences that do not form valid UTF-8 are left as
    /// they are.
    pub fn unescape_data_string(s: &str) -> String {
        if !s.contains('%') {
            return s.to_owned();
        }

        fn hex(b: u8) -> Option<u8> {
            (b as char).to_digit(16).map(|d| d as u8)
        }

        let bytes = s.as_bytes();
        let mut result = String::with_capacity(s.len());
        let mut literal_start = 0;
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] != b'%' {
                i += 1;
                continue;
            }

            // Collect a run of consecutive escape sequences.
            let run_start = i;
            let mut decoded = Vec::new();
            while i + 2 < bytes.len() && bytes[i] == b'%' {
                match (hex(bytes[i + 1]), hex(bytes[i + 2])) {
                    (Some(high), Some(low)) => {
                        decoded.push(high * 16 + low);
                        i += 3;
                    }
                    _ => break,
                }
            }
            if decoded.is_empty() {
                // A '%' that does not start an escape sequence.
                i += 1;
                continue;
            }

            result.push_str(&s[literal_start..run_start]);
            let mut offset = 0;
            while offset < decoded.len() {
                match std::str::from_utf8(&decoded[offset..]) {
                    Ok(valid) => {
                        result.push_str(valid);
                        offset = decoded.len();
                    }
                    Err(error) => {
                        let valid = error.valid_up_to();
                        result.push_str(std::str::from_utf8(&decoded[offset..offset + valid]).unwrap_or_default());
                        let invalid = error.error_len().unwrap_or(decoded.len() - offset - valid);
                        let source_start = run_start + (offset + valid) * 3;
                        result.push_str(&s[source_start..source_start + invalid * 3]);
                        offset += valid + invalid;
                    }
                }
            }
            literal_start = i;
        }
        result.push_str(&s[literal_start..]);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utilities::UriKind;

    fn absolute(s: &str) -> Uri {
        Uri::absolute(s).unwrap()
    }

    #[test]
    fn assembly_name_from_query_parsed() {
        const KEY: &str = "assembly";
        const VALUE: &str = "Ferro.Themes.Simple";

        let uri = absolute(&format!("resm:Ferro.Themes.Simple.Accents.BaseLight.xaml?{KEY}={VALUE}"));
        let name = UriExtensions::get_assembly_name_from_query(&uri);

        assert_eq!(VALUE, name);
    }

    #[test]
    fn assembly_name_from_empty_query_not_parsed() {
        let uri = absolute("resm:Ferro.Themes.Simple.Accents.BaseLight.xaml");
        let name = UriExtensions::get_assembly_name_from_query(&uri);

        assert_eq!("", name);
    }

    // --- not from upstream ---

    #[test]
    fn schemes() {
        let asset = absolute(&format!("{ASSET_SCHEME}://Lib/Assets/a.png"));
        assert!(UriExtensions::is_asset(&asset));
        assert!(!UriExtensions::is_resm(&asset));
        assert!(!UriExtensions::is_absolute_resm(&asset));
        let resm = absolute("resm:Lib.Assets.a.png?assembly=Lib");
        assert!(UriExtensions::is_absolute_resm(&resm));
        assert!(!UriExtensions::is_asset(&resm));
        let relative = Uri::new("Assets/a.png", UriKind::Relative).unwrap();
        assert!(!UriExtensions::is_absolute_resm(&relative));
    }

    #[test]
    fn authority_and_paths() {
        let asset = absolute(&format!("{ASSET_SCHEME}://Lib.Name/Assets/a%20b.png"));
        assert_eq!("lib.name", UriExtensions::authority(&asset));
        assert_eq!("/Assets/a b.png", UriExtensions::get_unescape_absolute_path(&asset));
        assert_eq!(
            format!("{ASSET_SCHEME}://lib.name/Assets/a b.png"),
            UriExtensions::get_unescape_absolute_uri(&asset)
        );
        assert_eq!("", UriExtensions::authority(&absolute("resm:Lib.a.png")));
    }

    #[test]
    fn ensure_absolute() {
        let base = absolute(&format!("{ASSET_SCHEME}://lib/Views/Main.xaml"));
        let relative = Uri::new("../Assets/a.png", UriKind::Relative).unwrap();
        let result = UriExtensions::ensure_absolute(&relative, Some(&base)).unwrap();
        assert_eq!(format!("{ASSET_SCHEME}://lib/Assets/a.png"), result.absolute_uri());
        assert!(UriExtensions::ensure_absolute(&relative, None).is_err());
        assert!(UriExtensions::ensure_absolute(&relative, Some(&relative)).is_err());
        assert_eq!(base, UriExtensions::ensure_absolute(&base, None).unwrap());
    }

    #[test]
    fn assembly_name_from_query() {
        for (uri, expected) in [
            ("resm:a.png?assembly=Lib", "Lib"),
            ("resm:a.png?x=1&assembly=Lib.Name", "Lib.Name"),
            ("resm:a.png?assembly=Lib&x=1", "Lib"),
            ("resm:a.png?assembly=My%20Lib", "My Lib"),
            ("resm:a.png?x=1", ""),
            ("resm:a.png", ""),
            ("resm:a.png?as", ""),
            ("resm:a.png?assembly", ""),
        ] {
            assert_eq!(expected, UriExtensions::get_assembly_name_from_query(&absolute(uri)), "{uri}");
        }
    }

    #[test]
    fn unescape() {
        assert_eq!("a b", UriExtensions::unescape_data_string("a%20b"));
        assert_eq!("plain", UriExtensions::unescape_data_string("plain"));
        assert_eq!("ż/ó", UriExtensions::unescape_data_string("%C5%BC/%C3%B3"));
        assert_eq!("100%", UriExtensions::unescape_data_string("100%"));
        assert_eq!("%zz%2", UriExtensions::unescape_data_string("%zz%2"));
        assert_eq!("a%FFb", UriExtensions::unescape_data_string("a%FFb"));
        assert_eq!("%C5 ", UriExtensions::unescape_data_string("%C5%20"));
    }
}
