//! A URI reference on [`url::Url`]: an absolute URL, or a reference relative to
//! a base (C5 of #423, section 3.5 of docs/idiomatic-rust-signatures.md).
//!
//! `url::Url` holds absolute URLs only, and markup writes relative references
//! where it names a resource, a style or a font (`Source="Values.xaml"`,
//! `Source="/Styles/Base.xaml"`): [`UriRef`] is the type of those positions,
//! and the port [`Uri`] converts to and from it.

use std::fmt;
use std::str::FromStr;

use super::uri::{canonical_text, remove_dot_segments, scheme_length};
use super::{Uri, UriToUrlError};

/// A URI reference (RFC 3986, section 4.1): an absolute URL, or a reference
/// relative to a base URL.
///
/// Equality and hashing are the ones of [`url::Url`] for an absolute
/// reference, with the fragment (decision O5 of
/// docs/idiomatic-rust-signatures.md: `resm:A.B#Name` is not `resm:A.B`), and
/// the text for a relative one; an absolute reference never equals a relative
/// one. [`Uri`] compares absolute URIs without their fragment, as .NET does: a
/// map that needs that rule keys by [`without_fragment`](Self::without_fragment)
/// explicitly.
///
/// The text of a URL is the one of the URL standard: the special schemes
/// (`http`, `https`, `file`, ...) are normalised (`http://host` is
/// `http://host/`, default ports and dot segments go, characters outside the
/// allowed set are percent-encoded); the schemes of the framework (`ferres://`,
/// `fonts:`, `resm:`) and `file://` URLs keep their text.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum UriRef {
    /// An absolute URL.
    Absolute(url::Url),
    /// A reference relative to a base URL, as written.
    Relative(Box<str>),
}

impl UriRef {
    /// Parses a URI reference with the rule of markup (`FerroUriTypeConverter`
    /// and the URI literals of the compiler): a text that starts with `/` is
    /// relative, never a file path; a text with a scheme is absolute, where a
    /// scheme has two characters or more (`C:` is a drive letter, as for
    /// [`Uri`]); any other text is relative.
    ///
    /// The URL of an absolute reference is the canonical text of the [`Uri`]
    /// of the text (the scheme and the authority in lower case) parsed with
    /// [`url::Url::parse`], so that for every text `Uri` accepts,
    /// `UriRef::parse(text)` is the reference of
    /// `Uri::new(text, UriKind::RelativeOrAbsolute)` (`TryFrom<&Uri> for UriRef`).
    /// A text the URL parser rejects (`http://host:99999/`) is its error.
    /// Unlike `Uri`, a scheme with nothing after it (`foo:`) is a URL.
    pub fn parse(text: &str) -> Result<Self, url::ParseError> {
        match scheme_length(text) {
            Some(length) if !text.starts_with('/') => url::Url::parse(&canonical_text(text, length)).map(UriRef::Absolute),
            _ => Ok(UriRef::Relative(text.into())),
        }
    }

    /// Parses an absolute URL with the rule of [`parse`](Self::parse) (the
    /// scheme and the authority in lower case, as the canonical text of
    /// [`Uri`] writes them): the base URI of a document (C#
    /// `new Uri(text, UriKind.Absolute)`). Two spellings of one document
    /// (`FERRES://App/Main.xaml`, `ferres://app/Main.xaml`) are one URL, as
    /// they are one `Uri`; [`url::Url::parse`] keeps the case of the host of a
    /// scheme that is not special.
    ///
    /// A relative reference is [`UriToUrlError::Relative`], and a text the
    /// URL parser rejects [`UriToUrlError::Invalid`].
    pub fn parse_absolute(text: &str) -> Result<url::Url, UriToUrlError> {
        match UriRef::parse(text).map_err(UriToUrlError::Invalid)? {
            UriRef::Absolute(url) => Ok(url),
            UriRef::Relative(_) => Err(UriToUrlError::Relative),
        }
    }

    /// Resolves the reference against the absolute `base` (C# `new Uri(Uri, Uri)`,
    /// [`Uri::combine`] of the port): an absolute reference is itself, and a
    /// relative one is resolved with [`url::Url::join`].
    ///
    /// For the references markup writes (a relative path, a path that starts
    /// with `/`, dot segments, a query, a fragment) against a base of the
    /// framework (`ferres://`, `fonts:`, `resm:`, `file://`), the URL is the one
    /// of `Uri::combine`, converted to a URL. `Url::join` cannot resolve
    /// against a URL whose path is opaque (`fonts:SystemFonts`,
    /// `resm:Fonts.Assets?assembly=App`); there the reference is resolved as
    /// `Uri::combine` resolves it (`fonts:Other`). A first segment with a colon
    /// and no scheme (`C:x`) stays a path, as for `Uri`.
    ///
    /// Where the URL standard differs from `Uri::combine`, the URL follows the
    /// standard (and RFC 3986, as .NET does):
    ///
    /// - a reference that is only a fragment (`#Name`), or empty, keeps the
    ///   query of the base, which `Uri::combine` drops;
    /// - a reference that starts with `//` names an authority (`//host/path`),
    ///   where `Uri::combine` reads a path of the authority of the base;
    /// - empty segments (`a//b`) stay, where `Uri::combine` removes them;
    /// - a `file:` URL of a drive (`file:///C:/dir/`) keeps its drive for a
    ///   path that starts with `/` and for `..` above the drive, where
    ///   `Uri::combine` goes to the root (`file:///Styles/Base.xaml`);
    /// - the special schemes are normalised as the text of every URL is (the
    ///   type documentation), and the host of another scheme has the case of
    ///   `base` (`UriRef::parse` and the conversions from `Uri` write it in
    ///   lower case, as `Uri` does).
    pub fn resolve(&self, base: &url::Url) -> Result<url::Url, url::ParseError> {
        let reference = match self {
            UriRef::Absolute(url) => return Ok(url.clone()),
            UriRef::Relative(reference) => &**reference,
        };
        // A relative reference with a scheme (a `Relative` made by hand): the absolute URI
        // it is, as `Uri::combine` returns an absolute reference.
        if let Some(length) = scheme_length(reference) {
            return url::Url::parse(&canonical_text(reference, length));
        }
        if base.cannot_be_a_base() {
            return url::Url::parse(&resolve_against_opaque_path(base, reference));
        }
        // A colon in the first segment of a reference without a scheme (`C:x`, `a:b`): a dot
        // segment keeps the URL parser from reading a scheme in it (RFC 3986, section 4.2).
        let first_segment = &reference[..reference.find(['/', '?', '#']).unwrap_or(reference.len())];
        if first_segment.contains(':') {
            return base.join(&format!("./{reference}"));
        }
        base.join(reference)
    }

    /// The URL of an absolute reference; `None` for a relative one.
    pub fn as_url(&self) -> Option<&url::Url> {
        match self {
            UriRef::Absolute(url) => Some(url),
            UriRef::Relative(_) => None,
        }
    }

    /// Whether the reference is relative (C# `!IsAbsoluteUri`).
    pub fn is_relative(&self) -> bool {
        matches!(self, UriRef::Relative(_))
    }

    /// The reference without its fragment (`resm:A.B#Name` is `resm:A.B`): the
    /// key of a map that compares URIs as .NET's `Uri` does (decision O5 of
    /// docs/idiomatic-rust-signatures.md). `Uri` ignores the fragment of an
    /// absolute URI only and compares a relative one by its whole text, so a
    /// key that follows `Uri` exactly takes this of the absolute references.
    pub fn without_fragment(&self) -> UriRef {
        match self {
            UriRef::Absolute(url) => {
                let mut url = url.clone();
                url.set_fragment(None);
                UriRef::Absolute(url)
            }
            UriRef::Relative(text) => UriRef::Relative(text[..text.find('#').unwrap_or(text.len())].into()),
        }
    }
}

/// The text `Uri::combine` makes of the relative `reference` against `base`, a
/// URL with an opaque path (`fonts:SystemFonts`): the scheme, the path of the
/// reference merged with the path of the base and without its dot segments,
/// and the query and the fragment of the reference. The query of the base is
/// dropped.
fn resolve_against_opaque_path(base: &url::Url, reference: &str) -> String {
    let base_path = base.path();
    let (path, suffix) = reference.split_at(reference.find(['?', '#']).unwrap_or(reference.len()));
    let merged = if path.is_empty() {
        base_path.to_owned()
    } else if path.starts_with('/') {
        path.to_owned()
    } else {
        let directory = base_path.rfind('/').map_or("", |index| &base_path[..=index]);
        format!("{directory}{path}")
    };
    format!("{}:{}{suffix}", base.scheme(), remove_dot_segments(&merged))
}

impl fmt::Display for UriRef {
    /// The text of the URL, or the relative reference as written.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UriRef::Absolute(url) => f.write_str(url.as_str()),
            UriRef::Relative(text) => f.write_str(text),
        }
    }
}

impl FromStr for UriRef {
    type Err = url::ParseError;

    /// [`UriRef::parse`].
    fn from_str(text: &str) -> Result<Self, Self::Err> {
        UriRef::parse(text)
    }
}

impl From<url::Url> for UriRef {
    fn from(url: url::Url) -> Self {
        UriRef::Absolute(url)
    }
}

impl TryFrom<UriRef> for url::Url {
    type Error = UriToUrlError;

    /// The URL of an absolute reference; a relative one is an error
    /// ([`UriToUrlError::Relative`]): resolve it with [`UriRef::resolve`] first.
    fn try_from(value: UriRef) -> Result<Self, Self::Error> {
        match value {
            UriRef::Absolute(url) => Ok(url),
            UriRef::Relative(_) => Err(UriToUrlError::Relative),
        }
    }
}

impl TryFrom<&UriRef> for url::Url {
    type Error = UriToUrlError;

    /// The URL of an absolute reference; a relative one is an error
    /// ([`UriToUrlError::Relative`]).
    fn try_from(value: &UriRef) -> Result<Self, Self::Error> {
        value.as_url().cloned().ok_or(UriToUrlError::Relative)
    }
}

impl TryFrom<&Uri> for UriRef {
    type Error = url::ParseError;

    /// The reference of the same text: a relative URI is the relative
    /// reference of its original string, and an absolute one the URL of its
    /// canonical text ([`Uri::absolute_uri`], as `TryFrom<&Uri> for Url`
    /// parses it), an error when the URL parser rejects it.
    fn try_from(value: &Uri) -> Result<Self, Self::Error> {
        if !value.is_absolute_uri() {
            return Ok(UriRef::Relative(value.original_string().into()));
        }
        url::Url::parse(value.absolute_uri()).map(UriRef::Absolute)
    }
}

impl From<&UriRef> for Uri {
    /// The URI of the same text: the absolute URI of the URL (`From<Url> for
    /// Uri`), or the relative URI of the reference as written.
    fn from(value: &UriRef) -> Self {
        match value {
            UriRef::Absolute(url) => Uri::from(url.clone()),
            UriRef::Relative(text) => Uri::relative_reference(text),
        }
    }
}

impl From<UriRef> for Uri {
    /// The URI of the same text, as `From<&UriRef> for Uri`.
    fn from(value: UriRef) -> Self {
        Uri::from(&value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utilities::UriKind;
    use std::collections::HashSet;

    fn parsed(text: &str) -> UriRef {
        UriRef::parse(text).unwrap_or_else(|error| panic!("{text}: {error}"))
    }

    fn url(text: &str) -> url::Url {
        url::Url::parse(text).unwrap_or_else(|error| panic!("{text}: {error}"))
    }

    /// `Uri::combine` of `reference` against `base`, as a URL.
    fn combined(base: &str, reference: &str) -> url::Url {
        let base = Uri::absolute(base).unwrap();
        let reference = Uri::new(reference, UriKind::RelativeOrAbsolute).unwrap();
        url::Url::try_from(&Uri::combine(&base, &reference)).unwrap()
    }

    /// `UriRef::resolve` of `reference` against the URL of `base`.
    fn resolved(base: &str, reference: &str) -> url::Url {
        let base = url::Url::try_from(&Uri::absolute(base).unwrap()).unwrap();
        parsed(reference).resolve(&base).unwrap_or_else(|error| panic!("{reference}: {error}"))
    }

    #[test]
    fn uri_ref_parses_with_the_rule_of_markup() {
        // A leading `/` is relative, never a file path.
        assert_eq!(parsed("/Styles/Base.xaml"), UriRef::Relative("/Styles/Base.xaml".into()));
        assert_eq!(parsed("Values.xaml"), UriRef::Relative("Values.xaml".into()));
        assert_eq!(parsed("../Fonts/#Name"), UriRef::Relative("../Fonts/#Name".into()));
        // A single letter is a drive letter, not a scheme.
        assert_eq!(parsed("C:x"), UriRef::Relative("C:x".into()));
        assert_eq!(parsed(""), UriRef::Relative("".into()));
        assert!(parsed("Values.xaml").is_relative() && parsed("Values.xaml").as_url().is_none());

        let resource = format!("{}://app/Assets/icon.png", crate::platform::ASSET_SCHEME);
        assert_eq!(parsed(&resource), UriRef::Absolute(url(&resource)));
        assert_eq!(parsed("fonts:SystemFonts").as_url().map(url::Url::scheme), Some("fonts"));
        assert_eq!(parsed("resm:Fonts.Assets?assembly=App#Name").as_url().and_then(url::Url::fragment), Some("Name"));
        // The scheme and the authority in lower case, as the canonical text of `Uri`.
        assert_eq!(parsed("FERRES://App/Assets").to_string(), "ferres://app/Assets");
        assert_eq!(parsed("HTTPS://Example.COM/Path").to_string(), "https://example.com/Path");
        // A text the URL parser rejects is its error.
        assert_eq!(UriRef::parse("http://host:99999/"), Err(url::ParseError::InvalidPort));
    }

    #[test]
    fn uri_ref_parses_an_absolute_url_as_a_base() {
        // Two spellings of one document are one URL, as they are one `Uri`.
        let base = UriRef::parse_absolute("FERRES://App/Views/Main.xaml").unwrap();
        assert_eq!(base.as_str(), "ferres://app/Views/Main.xaml");
        assert_eq!(base, UriRef::parse_absolute("ferres://app/Views/Main.xaml").unwrap());
        assert_eq!(base.as_str(), Uri::absolute("FERRES://App/Views/Main.xaml").unwrap().absolute_uri());
        assert_eq!(UriRef::parse_absolute("file:///C:/dir/page.xaml").unwrap().as_str(), "file:///C:/dir/page.xaml");
        // A relative reference is not a base, and a text the URL parser rejects is its error.
        assert_eq!(UriRef::parse_absolute("/Views/Main.xaml"), Err(UriToUrlError::Relative));
        assert_eq!(UriRef::parse_absolute("Main.xaml"), Err(UriToUrlError::Relative));
        assert_eq!(UriRef::parse_absolute("http://host:99999/"), Err(UriToUrlError::Invalid(url::ParseError::InvalidPort)));
    }

    #[test]
    fn uri_ref_resolves_like_uri_combine() {
        let resource = format!("{}://app/dir/page.xaml", crate::platform::ASSET_SCHEME);
        let bases = [
            resource.as_str(),
            "ferres://app",
            "app://Host/dir/page.xaml",
            "file:///C:/dir/page.xaml",
            "file:///home/user/dir/page.xaml",
            "https://example.com/a/b",
        ];
        let references = [
            "Values.xaml",
            "../Fonts/#Name",
            "./a/./b/../c.xaml",
            "a.xaml?x=1#f",
            "?q",
            "#f",
            "",
            "Fonts/",
            "Fonts/..",
            "a b.xaml",
            "a:b",
        ];
        for base in bases {
            for reference in references {
                assert_eq!(resolved(base, reference), combined(base, reference), "{reference} against {base}");
            }
            // Above the root, and a leading `/` (the drive of a `file:` URL differs: below).
            if !base.starts_with("file:///C:") {
                for reference in ["../../../../x.xaml", "/Styles/Base.xaml"] {
                    assert_eq!(resolved(base, reference), combined(base, reference), "{reference} against {base}");
                }
            }
        }
        assert_eq!(resolved(&resource, "/Styles/Base.xaml").as_str(), "ferres://app/Styles/Base.xaml");
        assert_eq!(resolved(&resource, "../Fonts/#Name").as_str(), "ferres://app/Fonts/#Name");
        assert_eq!(resolved("file:///C:/dir/page.xaml", "../Fonts/a.ttf").as_str(), "file:///C:/Fonts/a.ttf");

        // The opaque paths of the framework's schemes, which `Url::join` cannot resolve against.
        for (base, reference, expected) in [
            ("fonts:SystemFonts", "Other", "fonts:Other"),
            ("resm:Fonts.Assets?assembly=App", "Other", "resm:Other"),
            ("resm:Fonts.Assets?assembly=App", "#Name", "resm:Fonts.Assets#Name"),
            ("resm:Fonts.Assets?assembly=App", "?assembly=Other", "resm:Fonts.Assets?assembly=Other"),
            ("fonts:a/b", "c", "fonts:a/c"),
            ("fonts:a/b", "../c", "fonts:c"),
            ("fonts:a/b", "/c", "fonts:/c"),
            ("fonts:SystemFonts", "", "fonts:SystemFonts"),
        ] {
            assert_eq!(resolved(base, reference), combined(base, reference), "{reference} against {base}");
            assert_eq!(resolved(base, reference).as_str(), expected, "{reference} against {base}");
        }

        // An absolute reference is itself, and so is a relative one with a scheme made by hand.
        let base = url(&resource);
        assert_eq!(parsed("fonts:Inter").resolve(&base), Ok(url("fonts:Inter")));
        assert_eq!(UriRef::Relative("FONTS:Inter".into()).resolve(&base), Ok(url("fonts:Inter")));
    }

    /// The cases where the URL standard differs from `Uri::combine` (the documentation of
    /// `UriRef::resolve`): `resolve` follows the standard.
    #[test]
    fn uri_ref_resolution_follows_the_url_standard_where_uri_combine_differs() {
        let cases = [
            ("file:///C:/dir/page.xaml", "/Styles/Base.xaml", "file:///C:/Styles/Base.xaml", "file:///Styles/Base.xaml"),
            ("file:///C:/dir/page.xaml", "../../x.xaml", "file:///C:/x.xaml", "file:///x.xaml"),
            ("ferres://app/dir/page.xaml?a=1", "#f", "ferres://app/dir/page.xaml?a=1#f", "ferres://app/dir/page.xaml#f"),
            ("ferres://app/dir/page.xaml", "//other/x", "ferres://other/x", "ferres://app/other/x"),
            ("ferres://app/dir/page.xaml", "a//b", "ferres://app/dir/a//b", "ferres://app/dir/a/b"),
        ];
        for (base, reference, standard, port) in cases {
            assert_eq!(resolved(base, reference).as_str(), standard, "{reference} against {base}");
            assert_eq!(combined(base, reference).as_str(), port, "{reference} against {base}");
        }
    }

    #[test]
    fn uri_ref_equality() {
        // The fragment takes part (decision O5), unlike in `Uri`.
        let with = parsed("resm:Fonts.Assets?assembly=App#Name");
        let without = parsed("resm:Fonts.Assets?assembly=App");
        assert_ne!(with, without);
        assert_eq!(Uri::absolute("resm:Fonts.Assets?assembly=App#Name").unwrap(), Uri::absolute("resm:Fonts.Assets?assembly=App").unwrap());
        assert_eq!(with.without_fragment(), without);
        assert_eq!(without.without_fragment(), without);
        assert_eq!(parsed("ferres://app/a.xaml#x").without_fragment().to_string(), "ferres://app/a.xaml");
        assert_eq!(parsed("Fonts/a.ttf#Inter").without_fragment(), parsed("Fonts/a.ttf"));

        // Hashing agrees: two keys with the fragment, one without.
        let keys: HashSet<UriRef> = [with.clone(), without.clone()].into_iter().collect();
        assert_eq!(keys.len(), 2);
        let keys: HashSet<UriRef> = [with.without_fragment(), without.without_fragment()].into_iter().collect();
        assert_eq!(keys.len(), 1);

        // The URL of the same text, and the case of the scheme and the authority.
        assert_eq!(parsed("FONTS:SystemFonts"), parsed("fonts:SystemFonts"));
        assert_eq!(parsed("ferres://App/a.xaml"), parsed("ferres://app/a.xaml"));
        assert_ne!(parsed("fonts:a"), parsed("fonts:b"));
        // A relative reference compares by its text, and never equals an absolute one.
        assert_eq!(parsed("Values.xaml"), UriRef::Relative("Values.xaml".into()));
        assert_ne!(parsed("Values.xaml"), parsed("values.xaml"));
        assert_ne!(parsed("/a"), UriRef::Absolute(url("file:///a")));

        // With `without_fragment`, absolute references are equal exactly when their `Uri`s are.
        let texts = ["resm:A.B#One", "resm:A.B#Two", "resm:A.B", "fonts:SystemFonts#Inter", "fonts:SystemFonts", "ferres://app/a#b"];
        for a in texts {
            for b in texts {
                let uris = Uri::absolute(a).unwrap() == Uri::absolute(b).unwrap();
                assert_eq!(parsed(a).without_fragment() == parsed(b).without_fragment(), uris, "{a} and {b}");
            }
        }
    }

    #[test]
    fn uri_ref_display_and_from_str_round_trip() {
        let resource = format!("{}://app/Assets/icon.png", crate::platform::ASSET_SCHEME);
        for text in [
            "https://example.com/a/b?q=1#top",
            resource.as_str(),
            "fonts:SystemFonts",
            "resm:Fonts.Assets?assembly=App#Name",
            "file:///C:/dir/file.txt",
            "file:///home/user/a.png",
            "Values.xaml",
            "/Styles/Base.xaml",
            "../Fonts/#Name",
            "C:x",
            "",
        ] {
            let reference: UriRef = text.parse().unwrap();
            // The text as written: these are the texts of their URLs.
            assert_eq!(reference.to_string(), text);
            assert_eq!(reference.to_string().parse::<UriRef>(), Ok(reference.clone()), "{text}");
        }
        // A special scheme is normalised, and the normalised text round-trips.
        let normalised = parsed("http://host");
        assert_eq!(normalised.to_string(), "http://host/");
        assert_eq!(normalised.to_string().parse::<UriRef>(), Ok(normalised));
    }

    #[test]
    fn uri_ref_converts_to_and_from_urls_and_uris() {
        // A URL is an absolute reference, and an absolute reference is its URL.
        let address = url("https://example.com/a?q=1#top");
        let reference = UriRef::from(address.clone());
        assert_eq!(reference, UriRef::Absolute(address.clone()));
        assert_eq!(url::Url::try_from(&reference), Ok(address.clone()));
        assert_eq!(url::Url::try_from(reference), Ok(address.clone()));
        // A relative reference is not a URL.
        assert_eq!(url::Url::try_from(&parsed("/Assets/Fonts")), Err(UriToUrlError::Relative));
        assert_eq!(url::Url::try_from(parsed("Fonts")), Err(UriToUrlError::Relative));

        // A `Uri` is the reference of the same text, and back.
        let resource = format!("{}://app/Assets/icon.png", crate::platform::ASSET_SCHEME);
        for text in [
            resource.as_str(),
            "https://example.com/a/b?q=1#top",
            "fonts:SystemFonts",
            "resm:Fonts.Assets?assembly=App#Name",
            "file:///C:/dir/file.txt",
            "/Assets/Fonts",
            "Values.xaml",
            "../Fonts/#Name",
        ] {
            let uri = Uri::new(text, UriKind::RelativeOrAbsolute).unwrap();
            let reference = UriRef::try_from(&uri).unwrap();
            assert_eq!(reference, parsed(text), "{text}");
            assert_eq!(reference.is_relative(), !uri.is_absolute_uri(), "{text}");
            let back = Uri::from(&reference);
            assert_eq!(back, uri, "{text}");
            assert_eq!(back.to_string(), uri.to_string(), "{text}");
            assert_eq!(Uri::from(reference).is_absolute_uri(), uri.is_absolute_uri(), "{text}");
        }
        // The canonical text of an absolute `Uri` is the text of the URL.
        let uri = Uri::absolute("FERRES://App/Assets").unwrap();
        assert_eq!(UriRef::try_from(&uri).unwrap().to_string(), "ferres://app/Assets");
        // A relative reference made by hand converts to the relative `Uri` of its text.
        let relative = Uri::from(&UriRef::Relative("http://host/".into()));
        assert!(!relative.is_absolute_uri());
        assert_eq!(relative.original_string(), "http://host/");
        // An absolute `Uri` whose text the URL parser rejects is its error.
        assert_eq!(UriRef::try_from(&Uri::absolute("http://host:99999/").unwrap()), Err(url::ParseError::InvalidPort));
    }
}
