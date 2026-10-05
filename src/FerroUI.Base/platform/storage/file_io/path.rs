//! The path string operations the storage helpers need, with the semantics
//! of the path class of the .NET base library (no file-system access).

/// Whether the character separates directories on the current platform.
pub fn is_directory_separator(c: char) -> bool {
    c == '/' || (cfg!(windows) && c == '\\')
}

/// Whether the character ends the directory part of a path: a directory
/// separator, or the volume separator where there is one.
fn is_directory_or_volume_separator(c: char) -> bool {
    is_directory_separator(c) || (cfg!(windows) && c == ':')
}

/// The file name and extension of a path (`Path.GetFileName`).
pub fn get_file_name(path: &str) -> &str {
    match path.rfind(is_directory_or_volume_separator) {
        Some(index) => &path[index + 1..],
        None => path,
    }
}

/// The index of the period that starts the extension of the path, if the
/// last segment of the path has a period.
fn extension_start(path: &str) -> Option<usize> {
    for (index, c) in path.char_indices().rev() {
        if c == '.' {
            return Some(index);
        }
        if is_directory_or_volume_separator(c) {
            break;
        }
    }
    None
}

/// The extension of a path including the period, or an empty string
/// (`Path.GetExtension`).
pub fn get_extension(path: &str) -> &str {
    match extension_start(path) {
        Some(index) if index != path.len() - 1 => &path[index..],
        _ => "",
    }
}

/// Whether the path has an extension (`Path.HasExtension`).
pub fn has_extension(path: &str) -> bool {
    extension_start(path).is_some_and(|index| index != path.len() - 1)
}

/// Changes the extension of a path; `None` removes it
/// (`Path.ChangeExtension`).
pub fn change_extension(path: &str, extension: Option<&str>) -> String {
    if path.is_empty() {
        return String::new();
    }

    let sub_length = extension_start(path).unwrap_or(path.len());
    let Some(extension) = extension else {
        return path[..sub_length].to_owned();
    };

    let mut result = String::with_capacity(sub_length + extension.len() + 1);
    result.push_str(&path[..sub_length]);
    if !extension.starts_with('.') {
        result.push('.');
    }
    result.push_str(extension);
    result
}

/// The length of the root of a path: a leading separator, or a drive
/// specification where there are drives.
fn get_root_length(path: &str) -> usize {
    let bytes = path.as_bytes();
    if cfg!(windows) {
        if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
            return if bytes.len() >= 3 && is_directory_separator(bytes[2] as char) { 3 } else { 2 };
        }
        // A rooted path of the current drive, or the start of a UNC path.
        return bytes.iter().take(2).take_while(|b| is_directory_separator(**b as char)).count();
    }
    usize::from(bytes.first() == Some(&b'/'))
}

/// The directory part of a path, or `None` when the path is empty or is a
/// root (`Path.GetDirectoryName`). Separators are not normalized.
pub fn get_directory_name(path: &str) -> Option<&str> {
    if path.is_empty() {
        return None;
    }

    let bytes = path.as_bytes();
    let root_length = get_root_length(path);
    let mut end = bytes.len();
    if end <= root_length {
        return None;
    }

    loop {
        end -= 1;
        if end <= root_length || is_directory_separator(bytes[end] as char) {
            break;
        }
    }

    // Trim off any remaining separators (to deal with C:\foo\\bar)
    while end > root_length && is_directory_separator(bytes[end - 1] as char) {
        end -= 1;
    }

    Some(&path[..end])
}

/// Whether the character cannot be part of a path
/// (`Path.GetInvalidPathChars`).
pub(crate) fn is_invalid_path_char(c: char) -> bool {
    if cfg!(windows) {
        c == '|' || (c as u32) < 32
    } else {
        c == '\0'
    }
}
