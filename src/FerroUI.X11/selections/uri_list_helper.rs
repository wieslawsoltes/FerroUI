//! The `text/uri-list` format of files (the port of `UriListHelper.cs`).

use ferroui_base::platform::storage::file_io::{BclStorageItemHandle, StorageProviderHelpers};
use ferroui_base::platform::storage::IStorageItem;
use ferroui_base::utilities::{Uri, UriKind};
use std::rc::Rc;

/// The lines of a text as the stream reader of the reference's base
/// library gives them: a line ends with a line feed, a carriage return or
/// both, and the end of the text after a line end is not one more line.
pub fn read_lines(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0;
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'\n' => {
                lines.push(&text[start..index]);
                index += 1;
                start = index;
            }
            b'\r' => {
                lines.push(&text[start..index]);
                index += 1;
                if bytes.get(index) == Some(&b'\n') {
                    index += 1;
                }
                start = index;
            }
            _ => index += 1,
        }
    }
    if start < bytes.len() {
        lines.push(&text[start..]);
    }
    lines
}

/// The local paths of the file URIs of a URI list. Lines that are not
/// absolute file URIs (comments, other schemes, empty lines) are skipped.
pub fn utf8_bytes_to_file_paths(utf8_bytes: &[u8]) -> Vec<String> {
    // The reader of the reference skips a byte order mark and replaces
    // invalid sequences.
    let utf8_bytes = utf8_bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(utf8_bytes);
    let text = String::from_utf8_lossy(utf8_bytes);

    read_lines(&text)
        .into_iter()
        .filter_map(|line| Uri::try_create(line, UriKind::Absolute))
        .filter(|uri| uri.scheme() == "file")
        .map(|uri| uri.local_path())
        .collect()
}

/// The storage items of the files and the folders of a URI list that
/// exist.
pub fn utf8_bytes_to_file_uri_list(utf8_bytes: &[u8]) -> Vec<Rc<dyn IStorageItem>> {
    utf8_bytes_to_file_paths(utf8_bytes)
        .iter()
        .filter_map(|local_path| StorageProviderHelpers::try_create_bcl_storage_item(Some(local_path)))
        .map(|storage_item| match storage_item {
            BclStorageItemHandle::Folder(folder) => folder as Rc<dyn IStorageItem>,
            BclStorageItemHandle::File(file) => file as Rc<dyn IStorageItem>,
        })
        .collect()
}

/// A URI list of the given URIs.
pub fn uris_to_utf8_bytes<'a>(uris: impl IntoIterator<Item = &'a str>) -> Vec<u8> {
    let mut bytes = Vec::new();
    for uri in uris {
        bytes.extend_from_slice(uri.as_bytes());
        // CR+LF is mandatory according to the text/uri-list spec
        bytes.extend_from_slice(b"\r\n");
    }
    bytes
}

/// The URI list of storage items.
///
/// # Panics
/// Panics when the path of an item is a relative URI (the reference
/// throws).
pub fn file_uri_list_to_utf8_bytes(items: &[Rc<dyn IStorageItem>]) -> Vec<u8> {
    let uris: Vec<Uri> = items.iter().map(|item| item.path()).collect();
    uris_to_utf8_bytes(uris.iter().map(|uri| uri.absolute_uri()))
}

#[cfg(test)]
mod tests {
    // Not from the reference, which has no tests for this file.
    use super::*;

    #[test]
    fn lines_end_with_any_of_the_three_line_ends() {
        assert_eq!(read_lines("a\r\nb\nc\rd"), ["a", "b", "c", "d"]);
        assert_eq!(read_lines("a\r\n"), ["a"]);
        assert_eq!(read_lines("a\n\nb\n"), ["a", "", "b"]);
        assert_eq!(read_lines("\r\n"), [""]);
        assert!(read_lines("").is_empty());
    }

    #[test]
    fn only_absolute_file_uris_are_files() {
        let list = b"# a comment\r\nfile:///tmp/a%20file.txt\r\nhttps://example.org/x\r\n\r\nrelative/path\r\nfile:///home/user/b\r\n";
        assert_eq!(utf8_bytes_to_file_paths(list), ["/tmp/a file.txt", "/home/user/b"]);
    }

    #[test]
    fn a_list_may_end_without_a_line_end_and_start_with_a_mark() {
        assert_eq!(utf8_bytes_to_file_paths(b"\xef\xbb\xbffile:///a\nfile:///b"), ["/a", "/b"]);
        assert!(utf8_bytes_to_file_paths(b"").is_empty());
    }

    #[test]
    fn files_that_do_not_exist_are_not_items() {
        assert!(utf8_bytes_to_file_uri_list(b"file:///this/path/does/not/exist/anywhere-4f1c\r\n").is_empty());
    }

    #[test]
    fn an_existing_folder_is_an_item_with_its_uri() {
        let directory = std::env::temp_dir();
        let Some(path) = directory.to_str() else {
            return;
        };
        let uri = StorageProviderHelpers::uri_from_file_path(path, true);
        let list = uris_to_utf8_bytes([uri.absolute_uri()]);
        let items = utf8_bytes_to_file_uri_list(&list);
        assert_eq!(items.len(), 1);

        // The list written from the items names the same folder.
        let written = file_uri_list_to_utf8_bytes(&items);
        assert!(written.ends_with(b"\r\n"));
        assert_eq!(utf8_bytes_to_file_paths(&written).len(), 1);
    }

    #[test]
    fn every_uri_of_a_list_ends_with_a_carriage_return_and_a_line_feed() {
        assert_eq!(uris_to_utf8_bytes(["file:///a", "file:///b%20c"]), b"file:///a\r\nfile:///b%20c\r\n");
        assert!(uris_to_utf8_bytes([]).is_empty());
    }
}
