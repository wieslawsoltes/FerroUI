//! A minimal counterpart of .NET's `System.Uri`, covering what resource and
//! font keys need: absolute/relative detection, scheme, path, resolution of a
//! relative reference against a base and value equality.

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
                Some(Uri { original: Rc::from(uri_string), absolute: Some(Rc::from(canonical)), scheme_length: length })
            }
            (_, None) => Some(Uri { original: Rc::from(uri_string), absolute: None, scheme_length: 0 }),
        }
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
}
