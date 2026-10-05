//! Port of the upstream `StorageProviderHelperTests`, and of the two cases
//! of the upstream `UriExtensionsTests` that exercise
//! [`StorageProviderHelpers`].
//!
//! The encoded bookmarks are the upstream values re-encoded with the header
//! prefix of this framework (`frn.v1.`); the layout is otherwise the same.

use super::{DecodeResult, StorageBookmarkHelper, StorageProviderHelpers};

#[test]
fn can_encode_and_decode_bookmark() {
    let platform = b"test";
    let native_bookmark = b"bookmark";

    let bookmark = StorageBookmarkHelper::encode_bookmark_bytes(platform, native_bookmark);

    assert!(bookmark.is_some());

    let (result, native_bookmark_ret) = StorageBookmarkHelper::try_decode_bookmark(platform, bookmark.as_deref());
    assert_eq!(DecodeResult::Success, result);

    assert!(native_bookmark_ret.is_some());

    assert_eq!(native_bookmark.as_slice(), native_bookmark_ret.unwrap().as_slice());
}

#[test]
fn can_encode_bookmark() {
    for (native_bookmark, expected_encoded_bookmark) in [("C://file.txt", "ZnJuLnYxLnRlc3QAAAAAAEM6Ly9maWxlLnR4dA==")] {
        let platform = b"test";

        let bookmark = StorageBookmarkHelper::encode_bookmark(platform, Some(native_bookmark));

        assert_eq!(Some(expected_encoded_bookmark), bookmark.as_deref());
        assert!(bookmark.is_some());
    }
}

#[test]
fn can_decode_bookmark() {
    for (encoded_bookmark, expected_native_bookmark) in [("ZnJuLnYxLnRlc3QAAAAAAEM6Ly9maWxlLnR4dA==", "C://file.txt")] {
        let platform = b"test";
        let expected_native_bookmark_bytes = expected_native_bookmark.as_bytes();

        let (result, native_bookmark) = StorageBookmarkHelper::try_decode_bookmark(platform, Some(encoded_bookmark));
        assert_eq!(DecodeResult::Success, result);

        assert_eq!(Some(expected_native_bookmark_bytes), native_bookmark.as_deref());
    }
}

#[test]
fn can_decode_bcl_bookmarks() {
    for (bookmark, expected) in
        [("ZnJuLnYxLmJjbAAAAAAAAEM6Ly9maWxlLnR4dA==", "C://file.txt"), ("C://file.txt", "C://file.txt")]
    {
        let _a = StorageBookmarkHelper::encode_bcl_bookmark(expected);
        let local_path = StorageBookmarkHelper::try_decode_bcl_bookmark(bookmark);
        assert!(local_path.is_some());
        assert_eq!(Some(expected), local_path.as_deref());
    }
}

#[test]
fn fails_to_decode_invalid_bcl_bookmarks() {
    for bookmark in [
        // "test" platform passed instead of "bcl"
        "ZnJuLnYxLnRlc3QAAAAAAEM6Ly9maWxlLnR4dA==",
        "ZYXasHKJASd87124",
    ] {
        let local_path = StorageBookmarkHelper::try_decode_bcl_bookmark(bookmark);
        assert!(local_path.is_none());
    }
}

// --- upstream `UriExtensionsTests` ---

#[test]
fn should_convert_file_path_to_uri_and_back() {
    for path in [
        "/home/Projects.txt",
        "/home/Stahování/Požární kniha 2.txt",
        "C:\\%51.txt",
        "/home/asd#xcv.txt",
        "C:\\\\Work\\Projects.txt",
    ] {
        let uri = StorageProviderHelpers::uri_from_file_path(path, false);

        assert_eq!(path, uri.local_path());
    }
}

#[test]
fn should_convert_long_file_path_to_uri_and_back() {
    for (prepath, path) in [(
        r"\\?\D:\abcdefgh\abcdefgh\abcdefabcdefgh\abcdefghabcdefghabcdefgha\bcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefgh\abcdefghabcdefghabcdefgha\bcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefgh",
        r"D:\abcdefgh\abcdefgh\abcdefabcdefgh\abcdefghabcdefghabcdefgha\bcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefgh\abcdefghabcdefghabcdefgha\bcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefgh",
    )] {
        let uri = StorageProviderHelpers::uri_from_file_path(prepath, false);

        assert_eq!(path, uri.local_path());
    }
}

// --- not from upstream ---

#[test]
fn file_uris_are_escaped_and_local_paths_only_come_from_file_uris() {
    use crate::utilities::Uri;

    let uri = StorageProviderHelpers::uri_from_file_path("/home/a b/[x]?.txt", false);
    assert_eq!("file:///home/a%20b/%5Bx%5D%3F.txt", uri.absolute_uri());
    assert_eq!(Some("/home/a b/[x]?.txt"), StorageProviderHelpers::try_get_path_from_file_uri(Some(&uri)).as_deref());

    let directory = StorageProviderHelpers::uri_from_file_path("/home/user", true);
    assert_eq!("file:///home/user/", directory.absolute_uri());
    assert_eq!("file:///C:/Work/", StorageProviderHelpers::uri_from_file_path("C:\\Work", true).absolute_uri());

    let content = Uri::absolute("content://media/1").unwrap();
    assert_eq!(None, StorageProviderHelpers::try_get_path_from_file_uri(Some(&content)));
    let relative = Uri::new("a/b.txt", crate::utilities::UriKind::Relative).unwrap();
    assert_eq!(None, StorageProviderHelpers::try_get_path_from_file_uri(Some(&relative)));
    assert_eq!(None, StorageProviderHelpers::try_get_path_from_file_uri(None));
}

#[test]
fn name_with_extension_follows_the_filter_and_the_default_extension() {
    use crate::platform::storage::FilePickerFileType;

    let filter = FilePickerFileType::new(Some("Images")).with_patterns(&["*.*", "*.png", "*.jpg"]);
    let name = |path, default_extension, filter| StorageProviderHelpers::name_with_extension(path, default_extension, filter);

    assert_eq!(None, name(None, Some("txt"), Some(&*filter)));
    // A name with an extension is kept.
    assert_eq!(Some("/a/b.gif"), name(Some("/a/b.gif"), Some("txt"), Some(&*filter)).as_deref());
    // The first specific pattern of the filter wins over the default extension.
    assert_eq!(Some("/a/b.png"), name(Some("/a/b"), Some("txt"), Some(&*filter)).as_deref());
    // Unless the default extension is one of the patterns.
    let plain = FilePickerFileType::new(Some("Images")).with_patterns(&["*.png", ".jpg"]);
    assert_eq!(Some("/a/b.jpg"), name(Some("/a/b"), Some(".jpg"), Some(&*plain)).as_deref());
    // Without a usable filter the default extension is used.
    let all = FilePickerFileType::new(Some("All")).with_patterns(&["*.*"]);
    assert_eq!(Some("/a/b.txt"), name(Some("/a/b"), Some("txt"), Some(&*all)).as_deref());
    assert_eq!(Some("/a.d/b.txt"), name(Some("/a.d/b"), Some(".txt"), None).as_deref());
    assert_eq!(Some("/a/b"), name(Some("/a/b"), None, None).as_deref());
}
