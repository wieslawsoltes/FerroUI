//! A minimal counterpart of .NET's `System.Uri`, covering what resource and
//! font keys need: absolute/relative detection, scheme, path, resolution of a
//! relative reference against a base and value equality.
//!
//! At the edges the type converts from and to [`url::Url`]: every `Url` is an
//! absolute [`Uri`] (`From`), and an absolute `Uri` is a `Url` when the URL
//! parser accepts its text (`TryFrom`); a relative reference is not a `Url`.

use std::fmt;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

/// Defines the kinds of [`Uri`] accepted when parsing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UriKind {
    /// The kind is determined from the string.
    RelativeOrAbsolute,
    /// The string must be an absolute URI (it has a scheme).
    Absolute,
    /// The string must be a relative reference.
    Relative,
}

/// Error returned when a string is not a valid URI of the requested kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UriFormatError {
    message: &'static str,
}

impl fmt::Display for UriFormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message)
    }
}

impl std::error::Error for UriFormatError {}

/// A uniform resource identifier.
///
/// Cloning is cheap (one reference count).
#[derive(Clone)]
pub struct Uri {
    original: Rc<str>,
    /// For absolute URIs: the canonical text (scheme lower-cased).
    absolute: Option<Rc<str>>,
    /// Length of the scheme in `absolute`.
    scheme_length: usize,
}

fn scheme_length(s: &str) -> Option<usize> {
    let colon = s.find(':')?;
    let scheme = &s[..colon];
    let mut chars = scheme.chars();
    let first = chars.next()?;
    // A single letter followed by ':' is a drive letter, not a scheme.
    if scheme.len() < 2 || !first.is_ascii_alphabetic() {
        return None;
    }
    if chars.all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.') {
        Some(colon)
    } else {
        None
    }
}

impl Uri {
    /// Parses a URI of the given kind (C# `new Uri(string, UriKind)`).
    pub fn new(uri_string: &str, kind: UriKind) -> Result<Uri, UriFormatError> {
        Self::try_create(uri_string, kind).ok_or(UriFormatError {
            message: "Invalid URI: The format of the URI could not be determined.",
        })
    }

    /// Parses an absolute URI (C# `new Uri(string)`).
    pub fn absolute(uri_string: &str) -> Result<Uri, UriFormatError> {
        Self::new(uri_string, UriKind::Absolute)
    }

    /// Parses a URI of the given kind (C# `Uri.TryCreate`).
    pub fn try_create(uri_string: &str, kind: UriKind) -> Option<Uri> {
        let scheme = scheme_length(uri_string);

        match (kind, scheme) {
            (UriKind::Absolute, None) | (UriKind::Relative, Some(_)) => None,
            (_, Some(length)) => {
                if uri_string.len() == length + 1 {
                    // A scheme without anything after it.
                    return None;
                }
                Some(Self::absolute_with_scheme_length(uri_string, length))
            }
            (_, None) => Some(Uri { original: Rc::from(uri_string), absolute: None, scheme_length: 0 }),
        }
    }

    /// The absolute URI of `uri_string`, whose scheme is its first
    /// `length` bytes (followed by `:`).
    fn absolute_with_scheme_length(uri_string: &str, length: usize) -> Uri {
        let mut canonical = String::with_capacity(uri_string.len());
        canonical.push_str(&uri_string[..length].to_ascii_lowercase());
        let rest = &uri_string[length..];
        // The authority (host) is case-insensitive and canonically lower case.
        match rest.strip_prefix("://") {
            Some(after) => {
                let authority_length = after.find(['/', '?', '#']).unwrap_or(after.len());
                canonical.push_str("://");
                canonical.push_str(&after[..authority_length].to_ascii_lowercase());
                canonical.push_str(&after[authority_length..]);
            }
            None => canonical.push_str(rest),
        }
        Uri { original: Rc::from(uri_string), absolute: Some(Rc::from(canonical)), scheme_length: length }
    }

    /// Resolves `relative` against the absolute `base` (C# `new Uri(Uri, Uri)`).
    ///
    /// Panics when `base` is not absolute. An absolute `relative` is returned unchanged.
    pub fn combine(base: &Uri, relative: &Uri) -> Uri {
        if relative.is_absolute_uri() {
            return relative.clone();
        }

        let base_text = base.absolute.as_deref().expect("The base URI must be absolute.");
        let reference: &str = &relative.original;

        // Split the base into "scheme:[//authority]" and path (query/fragment dropped).
        let after_scheme = base.scheme_length + 1;
        let (prefix_end, path_end) = {
            let rest = &base_text[after_scheme..];
            let authority_length = if let Some(stripped) = rest.strip_prefix("//") {
                2 + stripped.find(['/', '?', '#']).unwrap_or(stripped.len())
            } else {
                0
            };
            let path_start = after_scheme + authority_length;
            let path_length = base_text[path_start..].find(['?', '#']).unwrap_or(base_text.len() - path_start);
            (path_start, path_start + path_length)
        };
        let prefix = &base_text[..prefix_end];
        let base_path = &base_text[prefix_end..path_end];

        let (reference_path, reference_suffix) = match reference.find(['?', '#']) {
            Some(index) => (&reference[..index], &reference[index..]),
            None => (reference, ""),
        };

        let merged = if reference_path.is_empty() {
            base_path.to_owned()
        } else if reference_path.starts_with('/') {
            reference_path.to_owned()
        } else {
            let directory = match base_path.rfind('/') {
                Some(index) => &base_path[..=index],
                None => {
                    if prefix.ends_with(':') {
                        ""
                    } else {
                        "/"
                    }
                }
            };
            format!("{directory}{reference_path}")
        };

        let mut result = String::with_capacity(prefix.len() + merged.len() + reference_suffix.len());
        result.push_str(prefix);
        result.push_str(&remove_dot_segments(&merged));
        result.push_str(reference_suffix);

        Uri::try_create(&result, UriKind::Absolute).expect("combining with an absolute base gives an absolute URI")
    }

    /// Whether the URI is absolute (C# `IsAbsoluteUri`).
    #[inline]
    pub fn is_absolute_uri(&self) -> bool {
        self.absolute.is_some()
    }

    /// The string the URI was created from (C# `OriginalString`).
    #[inline]
    pub fn original_string(&self) -> &str {
        &self.original
    }

    /// The lower-cased scheme (C# `Scheme`).
    ///
    /// Panics for a relative URI (C# throws `InvalidOperationException`).
    pub fn scheme(&self) -> &str {
        &self.expect_absolute()[..self.scheme_length]
    }

    /// The canonical absolute text (C# `AbsoluteUri`).
    ///
    /// Panics for a relative URI (C# throws `InvalidOperationException`).
    pub fn absolute_uri(&self) -> &str {
        self.expect_absolute()
    }

    /// The path component (C# `AbsolutePath`).
    ///
    /// Panics for a relative URI (C# throws `InvalidOperationException`).
    pub fn absolute_path(&self) -> &str {
        let text = self.expect_absolute();
        let rest = &text[self.scheme_length + 1..];
        let path = match rest.strip_prefix("//") {
            Some(stripped) => &stripped[stripped.find(['/', '?', '#']).unwrap_or(stripped.len())..],
            None => rest,
        };
        &path[..path.find(['?', '#']).unwrap_or(path.len())]
    }

    /// The query component including the leading `?`, or an empty string (C# `Query`).
    pub fn query(&self) -> &str {
        let text = self.expect_absolute();
        match text.find('?') {
            Some(start) => {
                let query = &text[start..];
                &query[..query.find('#').unwrap_or(query.len())]
            }
            None => "",
        }
    }

    /// The local operating-system representation of a file name (C# `LocalPath`):
    /// the unescaped path, which for a file URI of a drive path (`file:///C:/dir`)
    /// is the drive path with back slashes (`C:\dir`) and for a file URI with a
    /// host is the UNC path (`\\host\share`).
    ///
    /// Panics for a relative URI (C# throws `InvalidOperationException`).
    pub fn local_path(&self) -> String {
        use super::UriExtensions;

        let text = self.expect_absolute();
        if self.scheme() != "file" {
            return UriExtensions::unescape_data_string(self.absolute_path());
        }

        let rest = &text[self.scheme_length + 1..];
        let (authority, path) = match rest.strip_prefix("//") {
            Some(stripped) => stripped.split_at(stripped.find(['/', '?', '#']).unwrap_or(stripped.len())),
            None => ("", rest),
        };
        let path = &path[..path.find(['?', '#']).unwrap_or(path.len())];

        if !authority.is_empty() {
            return UriExtensions::unescape_data_string(&format!("\\\\{authority}{}", path.replace('/', "\\")));
        }

        let bytes = path.as_bytes();
        let is_drive_path = bytes.len() >= 3
            && bytes[0] == b'/'
            && bytes[1].is_ascii_alphabetic()
            && bytes[2] == b':'
            && bytes.get(3).is_none_or(|b| *b == b'/');
        if is_drive_path {
            return UriExtensions::unescape_data_string(&path[1..].replace('/', "\\"));
        }

        UriExtensions::unescape_data_string(path)
    }

    /// Makes a relative URI absolute using `base_uri`; absolute URIs are returned unchanged.
    ///
    /// Panics when the URI is relative and `base_uri` is missing or relative
    /// (C# throws `ArgumentException`).
    pub fn ensure_absolute(&self, base_uri: Option<&Uri>) -> Uri {
        if self.is_absolute_uri() {
            return self.clone();
        }
        let Some(base_uri) = base_uri else {
            panic!("Relative uri {self} without base url");
        };
        if !base_uri.is_absolute_uri() {
            panic!("Base uri {base_uri} is relative");
        }
        Uri::combine(base_uri, self)
    }

    fn expect_absolute(&self) -> &str {
        self.absolute.as_deref().expect("This operation is not supported for a relative URI.")
    }
}

fn remove_dot_segments(path: &str) -> String {
    let mut output: Vec<&str> = Vec::new();
    let absolute = path.starts_with('/');
    let trailing_slash = path.ends_with('/') || path.ends_with("/.") || path.ends_with("/..");

    for segment in path.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                output.pop();
            }
            other => output.push(other),
        }
    }

    let mut result = String::with_capacity(path.len());
    if absolute {
        result.push('/');
    }
    result.push_str(&output.join("/"));
    if trailing_slash && !result.ends_with('/') {
        result.push('/');
    }
    result
}

/// The text without its fragment: as in .NET, the fragment does not take part
/// in the comparison of absolute URIs.
fn without_fragment(text: &str) -> &str {
    &text[..text.find('#').unwrap_or(text.len())]
}

impl PartialEq for Uri {
    fn eq(&self, other: &Self) -> bool {
        match (&self.absolute, &other.absolute) {
            (Some(a), Some(b)) => without_fragment(a) == without_fragment(b),
            (None, None) => self.original == other.original,
            _ => false,
        }
    }
}

impl Eq for Uri {}

impl Hash for Uri {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match &self.absolute {
            Some(absolute) => without_fragment(absolute).hash(state),
            None => self.original.hash(state),
        }
    }
}

impl fmt::Display for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.absolute {
            Some(absolute) => f.write_str(absolute),
            None => f.write_str(&self.original),
        }
    }
}

impl fmt::Debug for Uri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl From<url::Url> for Uri {
    /// The absolute URI of the text of the URL ([`url::Url::as_str`]), which
    /// is its [`original_string`](Uri::original_string). The text is the one
    /// [`Uri::absolute`] would parse, with two exceptions that `Uri` rejects
    /// and a `Url` can hold: a scheme of a single letter (which `Uri` takes
    /// for a drive letter) and a scheme with nothing after it (`foo:`).
    fn from(value: url::Url) -> Self {
        Uri::absolute_with_scheme_length(value.as_str(), value.scheme().len())
    }
}

impl TryFrom<&Uri> for url::Url {
    type Error = UriToUrlError;

    /// Parses the canonical text of an absolute URI ([`Uri::absolute_uri`],
    /// the text of `ToString`) with [`url::Url::parse`]. A relative reference
    /// is an error: a `Url` is always absolute (resolve it with
    /// [`Uri::ensure_absolute`] first).
    ///
    /// The text of the `Url` is the text of the `Uri` except where the URL
    /// standard normalizes it: an empty path of the special schemes becomes
    /// `/` (`http://host` is `http://host/`), dot segments are removed, the
    /// default port of a special scheme is dropped, characters outside the
    /// allowed set are percent-encoded and international host names of the
    /// special schemes are written in Punycode. The schemes of the framework
    /// (`ferres://`, `fonts:`, `resm:`) and `file://` URIs keep their text.
    fn try_from(value: &Uri) -> Result<Self, Self::Error> {
        let Some(text) = value.absolute.as_deref() else {
            return Err(UriToUrlError::Relative);
        };
        url::Url::parse(text).map_err(UriToUrlError::Invalid)
    }
}

/// The error of the conversion of a [`Uri`] to a [`url::Url`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UriToUrlError {
    /// The URI is a relative reference; a `Url` is always absolute.
    Relative,
    /// The URL parser rejects the text of the absolute URI.
    Invalid(url::ParseError),
}

impl fmt::Display for UriToUrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UriToUrlError::Relative => f.write_str("A relative URI cannot be converted to a URL."),
            UriToUrlError::Invalid(error) => write!(f, "The URI is not a valid URL: {error}."),
        }
    }
}

impl std::error::Error for UriToUrlError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            UriToUrlError::Relative => None,
            UriToUrlError::Invalid(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_and_relative() {
        let uri = Uri::absolute("Resm:Fonts.Assets?assembly=App#Name").unwrap();
        assert!(uri.is_absolute_uri());
        assert_eq!(uri.scheme(), "resm");
        assert_eq!(uri.absolute_path(), "Fonts.Assets");
        assert_eq!(uri.query(), "?assembly=App");

        assert!(Uri::try_create("/Assets/Fonts", UriKind::Absolute).is_none());
        assert!(Uri::try_create("/Assets/Fonts", UriKind::Relative).is_some());
        assert!(Uri::try_create("fonts:Inter", UriKind::Relative).is_none());
        assert!(Uri::try_create("C:x", UriKind::Absolute).is_none());
    }

    #[test]
    fn combine_resolves_against_base() {
        let base = Uri::absolute("app://Host/dir/page.xaml").unwrap();
        let relative = Uri::new("/Assets/Fonts", UriKind::Relative).unwrap();
        assert_eq!(Uri::combine(&base, &relative).to_string(), "app://host/Assets/Fonts");

        let relative = Uri::new("../Fonts/#Name", UriKind::Relative).unwrap();
        assert_eq!(Uri::combine(&base, &relative).to_string(), "app://host/Fonts/#Name");

        let relative = Uri::new("Fonts", UriKind::Relative).unwrap();
        assert_eq!(relative.ensure_absolute(Some(&base)).absolute_path(), "/dir/Fonts");
    }

    #[test]
    fn equality_ignores_scheme_case() {
        assert_eq!(Uri::absolute("FONTS:SystemFonts").unwrap(), Uri::absolute("fonts:SystemFonts").unwrap());
        assert_ne!(Uri::absolute("fonts:a").unwrap(), Uri::absolute("fonts:b").unwrap());
        assert_eq!(Uri::absolute("resm:A.B#MyFont").unwrap(), Uri::absolute("resm:A.B").unwrap());
        assert_eq!(Uri::absolute("app://Host/Path").unwrap().absolute_uri(), "app://host/Path");
    }

    /// The URL of the text, through `Uri`, and the `Uri` back.
    fn round_trip(text: &str) -> (url::Url, Uri) {
        let uri = Uri::absolute(text).unwrap();
        let converted = url::Url::try_from(&uri).unwrap();
        let back = Uri::from(converted.clone());
        (converted, back)
    }

    #[test]
    fn converts_absolute_uris_to_and_from_urls() {
        let resource = format!("{}://app/Assets/icon.png", crate::platform::ASSET_SCHEME);
        for text in [
            "https://example.com/a/b?q=1#top",
            "http://example.com:8080/path",
            "file:///C:/dir/file.txt",
            "file:///home/user/a.png",
            resource.as_str(),
            "fonts:SystemFonts",
            "resm:Fonts.Assets?assembly=App#Name",
        ] {
            let uri = Uri::absolute(text).unwrap();
            let (converted, back) = round_trip(text);
            // The text of the URL is the one of `ToString`, and comes back unchanged.
            assert_eq!(converted.as_str(), uri.to_string(), "{text}");
            assert_eq!(back.to_string(), uri.to_string(), "{text}");
            assert_eq!(back.original_string(), text, "{text}");
            assert_eq!(back, uri, "{text}");
            assert_eq!(back.scheme(), converted.scheme(), "{text}");
        }

        let (converted, back) = round_trip(&resource);
        assert_eq!(converted.host_str(), Some("app"));
        assert_eq!(converted.path(), "/Assets/icon.png");
        assert_eq!(back.absolute_path(), "/Assets/icon.png");
        assert_eq!(round_trip("file:///C:/dir/file.txt").1.local_path(), "C:\\dir\\file.txt");
        assert_eq!(round_trip("resm:Fonts.Assets?assembly=App#Name").1.query(), "?assembly=App");
    }

    #[test]
    fn url_normalization_shows_in_the_converted_text() {
        // The canonical text of the `Uri` (scheme and host lower case) is what is parsed.
        assert_eq!(round_trip("HTTPS://Example.COM/Path").0.as_str(), "https://example.com/Path");
        assert_eq!(round_trip("ferres://App/Assets").0.as_str(), "ferres://app/Assets");
        // The URL standard gives a special scheme a path, drops its default port and dot segments.
        assert_eq!(round_trip("http://host").1.to_string(), "http://host/");
        assert_eq!(round_trip("http://host:80/a/../b").1.to_string(), "http://host/b");
    }

    #[test]
    fn a_url_with_a_one_letter_scheme_is_an_absolute_uri() {
        let uri = Uri::from(url::Url::parse("a:b").unwrap());
        assert!(uri.is_absolute_uri());
        assert_eq!(uri.scheme(), "a");
        assert_eq!(uri.to_string(), "a:b");
    }

    #[test]
    fn relative_and_invalid_uris_are_not_urls() {
        let relative = Uri::new("/Assets/Fonts", UriKind::Relative).unwrap();
        assert_eq!(url::Url::try_from(&relative), Err(UriToUrlError::Relative));
        let relative = Uri::new("../Fonts/#Name", UriKind::RelativeOrAbsolute).unwrap();
        assert_eq!(url::Url::try_from(&relative), Err(UriToUrlError::Relative));

        let invalid = Uri::absolute("http://host:99999/").unwrap();
        let error = url::Url::try_from(&invalid).unwrap_err();
        assert_eq!(error, UriToUrlError::Invalid(url::ParseError::InvalidPort));
        assert!(std::error::Error::source(&error).is_some());
        assert_eq!(UriToUrlError::Relative.to_string(), "A relative URI cannot be converted to a URL.");
    }
}
