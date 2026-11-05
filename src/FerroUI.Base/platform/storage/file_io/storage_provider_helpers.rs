use super::path;
use crate::platform::storage::FilePickerFileType;
use crate::utilities::UriExtensions;
use std::fmt::Write;
use url::Url;

/// Path and URI helpers of the storage providers.
///
/// This is an implementation detail of the platform backends.
pub struct StorageProviderHelpers;

/// The characters that stay as they are in the path of a URI: the
/// unreserved and the reserved characters, except `?` and `#`.
fn is_unescaped_in_path(c: char) -> bool {
    c.is_ascii_alphanumeric() || "-._~:/[]@!$&'()*+,;=".contains(c)
}

/// Builds the file URL of an already prepared path, as the URI builder of
/// the .NET base library does for the scheme `file`, an empty host and the
/// given path: back slashes become slashes and every character that is not
/// allowed in a path is percent-encoded, except existing escape sequences.
///
/// The text is made here and parsed with [`Url::parse`], never with
/// `Url::from_file_path`, which does not exist on `wasm32-unknown-unknown`
/// (F12 of docs/idiomatic-rust-signatures.md) and reads a path by the rules
/// of the host: a drive path is a drive path on every target, as it is for
/// .NET.
fn file_uri_from_path(path: &str) -> Option<Url> {
    let mut text = String::with_capacity(path.len() + 16);
    text.push_str("file://");

    let bytes = path.as_bytes();
    // A drive path has no host: "file:///C:/..."
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        text.push('/');
    }

    if path.is_empty() {
        text.push('/');
    }

    let mut buffer = [0u8; 4];
    for (index, c) in path.char_indices() {
        match c {
            '\\' => text.push('/'),
            '%' if bytes.get(index + 1).is_some_and(u8::is_ascii_hexdigit)
                && bytes.get(index + 2).is_some_and(u8::is_ascii_hexdigit) =>
            {
                text.push('%')
            }
            c if is_unescaped_in_path(c) => text.push(c),
            c => {
                for byte in c.encode_utf8(&mut buffer).bytes() {
                    // Writing to a string cannot fail.
                    let _ = write!(text, "%{byte:02X}");
                }
            }
        }
    }

    Url::parse(&text).ok()
}

impl StorageProviderHelpers {
    /// The storage item of an existing directory or file of the local file
    /// system; `None` when the path names neither.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_create_bcl_storage_item(path: Option<&str>) -> Option<super::BclStorageItemHandle> {
        use super::{BclStorageFile, BclStorageFolder, BclStorageItemHandle, FileSystemInfo};

        let path = path.filter(|path| !path.trim().is_empty())?;
        let directory = FileSystemInfo::directory(path);
        if directory.exists() {
            return Some(BclStorageItemHandle::Folder(BclStorageFolder::new(directory)));
        }

        let file = FileSystemInfo::file(path);
        if file.exists() {
            return Some(BclStorageItemHandle::File(BclStorageFile::new(file)));
        }

        None
    }

    /// The local path of a file URL; `None` for anything else.
    ///
    /// A storage item whose path is a relative reference (the browser, the
    /// root of a file system) passes the URL of its path,
    /// [`UriRef::as_url`](crate::utilities::UriRef::as_url), which is `None`.
    pub fn try_get_path_from_file_uri(uri: Option<&Url>) -> Option<String> {
        // android "content:", browser and ios relative links are ignored.
        match uri {
            Some(uri) if uri.scheme() == "file" => Some(Self::local_path(uri)),
            _ => None,
        }
    }

    /// The local operating-system representation of the path of a URL (C#
    /// `Uri.LocalPath`): the unescaped path, which for a file URL of a drive
    /// path (`file:///C:/dir`) is the drive path with back slashes (`C:\dir`)
    /// and for a file URL with a host is the UNC path (`\\host\share`).
    ///
    /// The mapping is the same on every target and does not depend on the
    /// host: `Url::to_file_path` does not exist on `wasm32-unknown-unknown`
    /// (F12 of docs/idiomatic-rust-signatures.md) and reads a drive path only
    /// on Windows, where .NET reads it everywhere.
    pub fn local_path(url: &Url) -> String {
        let path = url.path();
        if url.scheme() != "file" {
            return UriExtensions::unescape_data_string(path);
        }

        if let Some(host) = url.host_str().filter(|host| !host.is_empty()) {
            return UriExtensions::unescape_data_string(&format!("\\\\{host}{}", path.replace('/', "\\")));
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

    /// The file URL of a path of the local file system.
    ///
    /// # Panics
    /// Panics when the path cannot be expressed as a URL; see
    /// [`try_get_uri_from_file_path`](Self::try_get_uri_from_file_path).
    pub fn uri_from_file_path(path: &str, is_directory: bool) -> Url {
        Self::try_get_uri_from_file_path(path, is_directory).expect("Invalid URI: The format of the URI could not be determined.")
    }

    /// The file URL of a path of the local file system, or `None` when the
    /// path cannot be expressed as a URL.
    ///
    /// The URL is the same on every target, the browser included (F12 of
    /// docs/idiomatic-rust-signatures.md): a drive path (`C:\Work`) is
    /// `file:///C:/Work`, a path with `/` is that path.
    pub fn try_get_uri_from_file_path(path: &str, is_directory: bool) -> Option<Url> {
        // Windows long path prefix
        let is_long_path = path.starts_with(r"\\?\");
        let mut uri_path = if is_long_path { &path[4..] } else { path }
            .replace('%', "%25")
            .replace('[', "%5B")
            .replace(']', "%5D");
        if !path.ends_with('/') && is_directory {
            uri_path.push('/');
        }

        file_uri_from_path(&uri_path)
    }

    /// The path with the extension the save dialog implies, when its file
    /// name has none: the default extension if the selected filter lists
    /// it, else the extension of the first specific pattern of the filter,
    /// else the default extension.
    pub fn name_with_extension(
        path: Option<&str>,
        default_extension: Option<&str>,
        filter: Option<&FilePickerFileType>,
    ) -> Option<String> {
        let path = path?;
        let name = path::get_file_name(path);
        if !path::has_extension(name) {
            if let Some(patterns) = filter.and_then(|filter| filter.patterns()).filter(|patterns| !patterns.is_empty()) {
                if let Some(default_extension) = default_extension {
                    if patterns.iter().any(|pattern| pattern == default_extension) {
                        return Some(path::change_extension(path, Some(default_extension.trim_start_matches('.'))));
                    }
                }

                let ext = patterns.iter().find(|x| *x != "*.*");
                let ext = ext.and_then(|ext| ext.rsplit("*.").find(|part| !part.is_empty()));
                if let Some(ext) = ext {
                    return Some(path::change_extension(path, Some(ext)));
                }
            }

            if let Some(default_extension) = default_extension {
                return Some(path::change_extension(path, Some(default_extension)));
            }
        }

        Some(path.to_owned())
    }
}
