use crate::{require, ProjectError, Result};
use std::path::{Component, Path, PathBuf};

pub fn file_uri(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let normalized = normalize_path(&absolute)?;
    let text = normalized
        .to_str()
        .ok_or_else(|| ProjectError("Document paths must be UTF-8".into()))?
        .to_owned();
    #[cfg(windows)]
    let text = text.replace('\\', "/");
    let text = if text.starts_with('/') {
        text
    } else {
        format!("/{text}")
    };
    Ok(format!("file://{}", encode(&text)))
}

pub fn file_path(uri: &str) -> Result<PathBuf> {
    let rest = uri
        .strip_prefix("file://")
        .ok_or_else(|| ProjectError("Expected a file URI".into()))?;
    let path = if rest.starts_with('/') {
        rest
    } else if let Some(path) = rest.strip_prefix("localhost/") {
        return file_path(&format!("file:///{path}"));
    } else {
        return Err(ProjectError(
            "Remote file authorities are not local project paths".into(),
        ));
    };
    require(
        !path.contains(['?', '#']),
        "File URIs must encode literal query and fragment characters",
    )?;
    let decoded = decode(path)?;
    #[cfg(windows)]
    let decoded = if decoded.as_bytes().get(2) == Some(&b':') {
        decoded[1..].to_owned()
    } else {
        decoded
    };
    normalize_path(Path::new(&decoded))
}

/// Resolve file, ferres, untitled and relative resource addresses without performing IO.
pub fn resolve_uri(base: &str, reference: &str) -> Result<String> {
    if has_scheme(reference) {
        return normalize_uri(reference);
    }
    let (prefix, path) = split_uri(base)?;
    if reference.starts_with('#') {
        return Ok(format!("{}{}", base.split('#').next().unwrap(), reference));
    }
    let path = path.split(['?', '#']).next().unwrap();
    let joined = if reference.starts_with('/') {
        reference.to_owned()
    } else {
        format!(
            "{}{reference}",
            &path[..path.rfind('/').map_or(0, |i| i + 1)]
        )
    };
    normalize_uri(&format!("{prefix}{joined}"))
}

pub(crate) fn normalize_path(path: &Path) -> Result<PathBuf> {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                require(out.pop(), "Path escapes its root")?;
            }
            other => out.push(other.as_os_str()),
        }
    }
    Ok(out)
}
fn has_scheme(value: &str) -> bool {
    value.find(':').is_some_and(|i| {
        i > 0
            && value[..i].bytes().enumerate().all(|(n, c)| {
                c.is_ascii_alphabetic() || (n > 0 && (c.is_ascii_digit() || b"+-.".contains(&c)))
            })
    })
}
fn split_uri(uri: &str) -> Result<(&str, &str)> {
    let scheme = uri
        .find(':')
        .ok_or_else(|| ProjectError("Absolute base URI required".into()))?;
    let start = if uri[scheme + 1..].starts_with("//") {
        uri[scheme + 3..]
            .find('/')
            .map_or(uri.len(), |i| scheme + 3 + i)
    } else {
        scheme + 1
    };
    Ok((&uri[..start], &uri[start..]))
}
fn normalize_uri(uri: &str) -> Result<String> {
    let (prefix, path) = split_uri(uri)?;
    let suffix_start = path.find(['?', '#']).unwrap_or(path.len());
    let suffix = &path[suffix_start..];
    let path = &path[..suffix_start];
    let mut parts = Vec::new();
    for part in path.split('/') {
        let decoded = decode(part)?;
        match decoded.as_str() {
            "" | "." => {}
            ".." => {
                require(parts.pop().is_some(), "Resource URI escapes its root")?;
            }
            _ => {
                require(
                    !decoded.contains(['/', '\\', '\0']),
                    "Encoded resource separators are not allowed",
                )?;
                parts.push(encode(&decoded));
            }
        }
    }
    let leading = if path.starts_with('/') || prefix.contains("://") {
        "/"
    } else {
        ""
    };
    Ok(format!("{prefix}{leading}{}{suffix}", parts.join("/")))
}
fn encode(value: &str) -> String {
    let mut out = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || b"/-._~:".contains(&byte) {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}
pub(crate) fn decode(value: &str) -> Result<String> {
    let bytes = value.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            require(i + 2 < bytes.len(), "Incomplete URI escape")?;
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3])
                .map_err(|_| ProjectError("Invalid URI escape".into()))?;
            let byte = u8::from_str_radix(hex, 16)
                .map_err(|_| ProjectError("Invalid URI escape".into()))?;
            require(byte != 0, "NUL is not a path character")?;
            out.push(byte);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| ProjectError("URI path is not UTF-8".into()))
}
