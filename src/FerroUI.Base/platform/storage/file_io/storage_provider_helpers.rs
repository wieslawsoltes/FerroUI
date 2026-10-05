use super::path;
use crate::platform::storage::FilePickerFileType;
use crate::utilities::{Uri, UriKind};
use std::fmt::Write;

/// Path and URI helpers of the storage providers.
///
/// This is an implementation detail of the platform backends.
pub struct StorageProviderHelpers;

/// The characters that stay as they are in the path of a URI: the
/// unreserved and the reserved characters, except `?` and `#`.
fn is_unescaped_in_path(c: char) -> bool {
    c.is_ascii_alphanumeric() || "-._~:/[]@!$&'()*+,;=".contains(c)
}

/// Builds the file URI of an already prepared path, as the URI builder of
/// the .NET base library does for the scheme `file`, an empty host and the
/// given path: back slashes become slashes and every character that is not
/// allowed in a path is percent-encoded, except existing escape sequences.
fn file_uri_from_path(path: &str) -> Option<Uri> {
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

    Uri::try_create(&text, UriKind::Absolute)
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

    /// The local path of an absolute file URI; `None` for anything else.
    pub fn try_get_path_from_file_uri(uri: Option<&Uri>) -> Option<String> {
        // android "content:", browser and ios relative links are ignored.
        match uri {
            Some(uri) if uri.is_absolute_uri() && uri.scheme() == "file" => Some(uri.local_path()),
            _ => None,
        }
    }

    /// The file URI of a path of the local file system.
    ///
    /// # Panics
    /// Panics when the path cannot be expressed as a URI; see
    /// [`try_get_uri_from_file_path`](Self::try_get_uri_from_file_path).
    pub fn uri_from_file_path(path: &str, is_directory: bool) -> Uri {
        Self::try_get_uri_from_file_path(path, is_directory).expect("Invalid URI: The format of the URI could not be determined.")
    }

    /// The file URI of a path of the local file system, or `None` when the
    /// path cannot be expressed as a URI.
    pub fn try_get_uri_from_file_path(path: &str, is_directory: bool) -> Option<Uri> {
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
