//! Windows path syntax stays testable on every host. File URIs omit Win32's
//! extended-length prefix; decoded paths retain it for lossless long-path IO.
use super::{decode, encode};
use crate::{require, ProjectError, Result};

pub(super) fn path_to_uri(path: &str) -> Result<String> {
    require(!path.contains('\0'), "NUL is not a path character")?;
    let verbatim = path.strip_prefix(r"\\?\");
    if verbatim.is_some() {
        require(
            !path.contains('/'),
            "Verbatim Windows paths must use backslash separators",
        )?;
    }
    let path = verbatim.unwrap_or(path).replace('\\', "/");
    let unc = if verbatim.is_some() {
        path.get(..4)
            .filter(|prefix| prefix.eq_ignore_ascii_case("UNC/"))
            .map(|_| &path[4..])
    } else {
        path.strip_prefix("//")
    };
    if let Some(unc) = unc {
        let (server, path) = unc
            .split_once('/')
            .ok_or_else(|| ProjectError("UNC paths need a server and share".into()))?;
        let (share, tail) = path.split_once('/').unwrap_or((path, ""));
        validate_server(server)?;
        validate_share(share)?;
        let tail = normalize_segments(tail.split('/').map(str::to_owned))?;
        return Ok(format!(
            "file://{}/{}/{}",
            encode(&server.to_ascii_lowercase()),
            encode(share),
            encode(&tail.join("/"))
        ));
    }
    let drive = drive_letter(&path).ok_or_else(|| ProjectError("Expected an absolute Windows drive or UNC path; device namespaces are not document paths".into()))?;
    let tail = normalize_segments(path[3..].split('/').map(str::to_owned))?;
    Ok(format!("file:///{drive}:/{}", encode(&tail.join("/"))))
}

pub(super) fn uri_to_path(authority: &str, path: &str) -> Result<String> {
    require(
        path.starts_with('/') && !path.starts_with("//") && !path.contains('\\'),
        "File URI path must use unambiguous forward-slash separators",
    )?;
    let mut segments = path[1..].split('/').map(|segment| {
        let decoded = decode(segment)?;
        require(
            !decoded.contains(['/', '\\']),
            "Encoded file separators are not allowed",
        )?;
        Ok(decoded)
    });
    let root = segments.next().transpose()?.unwrap_or_default();
    let tail = normalize_segments(segments.collect::<Result<Vec<_>>>()?)?;
    let drive = drive_letter(&format!("{root}/"));
    if authority.is_empty() || authority.eq_ignore_ascii_case("localhost") && drive.is_some() {
        let drive = drive.ok_or_else(|| {
            ProjectError("Windows file URIs need an absolute drive: file:///C:/path".into())
        })?;
        require(
            path[1..].contains('/'),
            "Drive-relative file URIs are ambiguous",
        )?;
        Ok(format!(r"\\?\{drive}:\{}", tail.join(r"\")))
    } else {
        require(
            !authority.eq_ignore_ascii_case("localhost") || path[1..].contains('/'),
            "A localhost file URI needs a drive or an explicit UNC share path",
        )?;
        let server = decode(authority)?;
        validate_server(&server)?;
        validate_share(&root)?;
        Ok(format!(
            r"\\?\UNC\{}\{}\{}",
            server.to_ascii_lowercase(),
            root,
            tail.join(r"\")
        ))
    }
}

fn drive_letter(path: &str) -> Option<char> {
    let bytes = path.as_bytes();
    (bytes.len() >= 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' && bytes[2] == b'/')
        .then(|| bytes[0].to_ascii_uppercase() as char)
}
fn validate_server(server: &str) -> Result<()> {
    require(
        !server.is_empty()
            && !matches!(server, "." | "..")
            && server
                .chars()
                .all(|ch| ch.is_alphanumeric() || matches!(ch, '-' | '_' | '.')),
        "UNC server must be a host name without credentials, ports or device prefixes",
    )
}
fn validate_share(share: &str) -> Result<()> {
    require(
        !share.is_empty()
            && !matches!(share, "." | "..")
            && !share.contains(['/', '\\', ':', '\0']),
        "UNC paths need a nonempty share name",
    )
}
fn normalize_segments(segments: impl IntoIterator<Item = String>) -> Result<Vec<String>> {
    let mut normalized = Vec::new();
    for segment in segments {
        match segment.as_str() {
            "" | "." => {}
            ".." => {
                require(
                    normalized.pop().is_some(),
                    "Path escapes its drive or share root",
                )?;
            }
            _ => {
                require(!segment.contains('\0'), "NUL is not a path character")?;
                normalized.push(segment);
            }
        }
    }
    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drives_and_verbatim_drives_share_conventional_uris() {
        let expected = "file:///C:/Views/%C3%81gua%20%23%25%20%F0%9F%A6%80.xaml";
        for path in [
            r"C:\Views\Água #% 🦀.xaml",
            r"c:/Views/Água #% 🦀.xaml",
            r"\\?\C:\Views\Água #% 🦀.xaml",
        ] {
            assert_eq!(path_to_uri(path).unwrap(), expected);
        }
        let native = uri_to_path("", "/c:/Views/%C3%81gua%20%23%25%20%F0%9F%A6%80.xaml").unwrap();
        assert_eq!(native, r"\\?\C:\Views\Água #% 🦀.xaml");
        assert_eq!(path_to_uri(&native).unwrap(), expected);
        assert_eq!(uri_to_path("LOCALHOST", "/C:/").unwrap(), r"\\?\C:\");
    }

    #[test]
    fn unc_and_verbatim_unc_preserve_server_share_and_unicode() {
        let expected = "file://server/Design%20Share/Views/%C3%81.xaml";
        for path in [
            r"\\SERVER\Design Share\Views\Á.xaml",
            r"\\?\UNC\server\Design Share\Views\Á.xaml",
        ] {
            assert_eq!(path_to_uri(path).unwrap(), expected);
        }
        let native = uri_to_path("SERVER", "/Design%20Share/Views/%C3%81.xaml").unwrap();
        assert_eq!(native, r"\\?\UNC\server\Design Share\Views\Á.xaml");
        assert_eq!(path_to_uri(&native).unwrap(), expected);
        assert_eq!(
            path_to_uri(r"\\server\share").unwrap(),
            "file://server/share/"
        );
        assert_eq!(
            uri_to_path("localhost", "/share/View.xaml").unwrap(),
            r"\\?\UNC\localhost\share\View.xaml"
        );
        assert_eq!(
            path_to_uri(r"\\localhost\share\View.xaml").unwrap(),
            "file://localhost/share/View.xaml"
        );
    }

    #[test]
    fn ambiguous_roots_device_paths_and_encoded_separators_are_rejected() {
        for path in [
            r"C:relative.xaml",
            r"C:",
            r"\root.xaml",
            "/root.xaml",
            r"\\.\pipe\name",
            r"\\?\Volume{123}\View.xaml",
            r"\\server",
            r"\\server\",
            r"C:\..\escape.xaml",
            r"\\server\share\..\escape.xaml",
        ] {
            assert!(path_to_uri(path).is_err(), "accepted {path:?}");
        }
        for (authority, path) in [
            ("", "/View.xaml"),
            ("", "/C:"),
            ("", "/C:relative.xaml"),
            ("", "//server/share/View.xaml"),
            ("", "/C:/bad%2Fname"),
            ("", "/C:/bad%5Cname"),
            ("user@server", "/share/View.xaml"),
            ("server:80", "/share/View.xaml"),
            (".", "/share/View.xaml"),
            ("server", "/"),
            ("server", "/share/../escape.xaml"),
        ] {
            assert!(
                uri_to_path(authority, path).is_err(),
                "accepted {authority:?} {path:?}"
            );
        }
    }
}
