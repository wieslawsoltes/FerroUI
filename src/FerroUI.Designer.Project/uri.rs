use crate::{require, ProjectError, Result};
use std::path::{Component, Path, PathBuf};

#[cfg(any(windows, test))]
mod windows;

pub fn file_uri(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    #[cfg(windows)]
    require(
        path.is_absolute()
            || !path.has_root() && !matches!(path.components().next(), Some(Component::Prefix(_))),
        "Windows paths need an explicit drive or UNC share; rooted and drive-relative paths are ambiguous",
    )?;
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    #[cfg(windows)]
    {
        windows::path_to_uri(
            absolute
                .to_str()
                .ok_or_else(|| ProjectError("Document paths must be UTF-8".into()))?,
        )
    }
    #[cfg(not(windows))]
    {
        let normalized = normalize_path(&absolute)?;
        let text = normalized
            .to_str()
            .ok_or_else(|| ProjectError("Document paths must be UTF-8".into()))?
            .to_owned();
        let text = if text.starts_with('/') {
            text
        } else {
            format!("/{text}")
        };
        Ok(format!("file://{}", encode(&text)))
    }
}

pub fn file_path(uri: &str) -> Result<PathBuf> {
    let rest = uri
        .strip_prefix("file://")
        .ok_or_else(|| ProjectError("Expected a file URI".into()))?;
    let slash = rest
        .find('/')
        .ok_or_else(|| ProjectError("File URI needs an absolute path".into()))?;
    let (authority, path) = rest.split_at(slash);
    require(
        !rest.contains(['?', '#']),
        "File URIs must encode literal query and fragment characters",
    )?;
    #[cfg(windows)]
    {
        Ok(PathBuf::from(windows::uri_to_path(authority, path)?))
    }
    #[cfg(not(windows))]
    {
        require(
            authority.is_empty() || authority.eq_ignore_ascii_case("localhost"),
            "Remote file authorities are not local project paths",
        )?;
        require(
            !path.starts_with("//"),
            "File URIs must not disguise an authority inside the path",
        )?;
        normalize_path(Path::new(&decode(path)?))
    }
}

/// Resolve filesystem aliases for document identity without requiring the file
/// itself to exist. The closest existing parent is canonicalized for new files.
/// `file_uri` remains a lexical encoder when callers need the original spelling.
pub fn canonical_file_uri(uri: &str) -> Result<String> {
    let path = file_path(uri)?;
    file_uri(canonical_path(&path)?)
}

pub(crate) fn canonical_path(path: &Path) -> Result<PathBuf> {
    let mut ancestor = path;
    let mut suffix = Vec::new();
    loop {
        match ancestor.canonicalize() {
            Ok(mut canonical) => {
                for component in suffix.into_iter().rev() {
                    canonical.push(component);
                }
                return Ok(canonical);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                // A dangling symlink is an existing directory entry, not a new
                // pathname. Never invent a local identity for its missing target.
                match std::fs::symlink_metadata(ancestor) {
                    Ok(_) => return Err(error.into()),
                    Err(metadata_error)
                        if metadata_error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(metadata_error) => return Err(metadata_error.into()),
                }
                suffix.push(
                    ancestor
                        .file_name()
                        .ok_or_else(|| ProjectError(error.to_string()))?,
                );
                ancestor = ancestor
                    .parent()
                    .ok_or_else(|| ProjectError(error.to_string()))?;
            }
            Err(error) => return Err(error.into()),
        }
    }
}

/// Resolve file, ferres, untitled and relative resource addresses without performing IO.
pub fn resolve_uri(base: &str, reference: &str) -> Result<String> {
    if has_scheme(reference) {
        return normalize_uri(reference);
    }
    let (prefix, path) = split_uri(base)?;
    if reference.is_empty() {
        return normalize_uri(base.split('#').next().unwrap());
    }
    if reference.starts_with('#') {
        return Ok(format!("{}{}", base.split('#').next().unwrap(), reference));
    }
    if reference.starts_with('?') {
        return normalize_uri(&format!(
            "{}{reference}",
            base.split(['?', '#']).next().unwrap()
        ));
    }
    if reference.starts_with("//") {
        let scheme = base.find(':').unwrap();
        return normalize_uri(&format!("{}{reference}", &base[..=scheme]));
    }
    let path = path.split(['?', '#']).next().unwrap();
    let joined = if reference.starts_with('/') {
        reference.to_owned()
    } else {
        let parent = &path[..path.rfind('/').map_or(0, |i| i + 1)];
        format!(
            "{}{reference}",
            if parent.is_empty() && prefix.contains("://") {
                "/"
            } else {
                parent
            }
        )
    };
    normalize_uri(&format!("{prefix}{joined}"))
}

#[cfg(not(windows))]
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
            .find(['/', '?', '#'])
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
    let directory = path.ends_with('/') || path.ends_with("/.") || path.ends_with("/..");
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
    let trailing = if directory && !parts.is_empty() {
        "/"
    } else {
        ""
    };
    Ok(format!(
        "{prefix}{leading}{}{trailing}{suffix}",
        parts.join("/")
    ))
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
            require(bytes[i] != 0, "NUL is not a path character")?;
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| ProjectError("URI path is not UTF-8".into()))
}
