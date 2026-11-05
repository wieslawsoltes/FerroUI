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

        assert_eq!(path, StorageProviderHelpers::local_path(&uri));
    }
}

#[test]
fn should_convert_long_file_path_to_uri_and_back() {
    for (prepath, path) in [(
        r"\\?\D:\abcdefgh\abcdefgh\abcdefabcdefgh\abcdefghabcdefghabcdefgha\bcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefgh\abcdefghabcdefghabcdefgha\bcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefgh",
        r"D:\abcdefgh\abcdefgh\abcdefabcdefgh\abcdefghabcdefghabcdefgha\bcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefgh\abcdefghabcdefghabcdefgha\bcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefgh",
    )] {
        let uri = StorageProviderHelpers::uri_from_file_path(prepath, false);

        assert_eq!(path, StorageProviderHelpers::local_path(&uri));
    }
}

// --- not from upstream ---

#[test]
fn file_uris_are_escaped_and_local_paths_only_come_from_file_uris() {
    use crate::utilities::UriRef;
    use url::Url;

    let uri = StorageProviderHelpers::uri_from_file_path("/home/a b/[x]?.txt", false);
    assert_eq!("file:///home/a%20b/%5Bx%5D%3F.txt", uri.as_str());
    assert_eq!(Some("/home/a b/[x]?.txt"), StorageProviderHelpers::try_get_path_from_file_uri(Some(&uri)).as_deref());

    let directory = StorageProviderHelpers::uri_from_file_path("/home/user", true);
    assert_eq!("file:///home/user/", directory.as_str());
    assert_eq!("file:///C:/Work/", StorageProviderHelpers::uri_from_file_path("C:\\Work", true).as_str());

    let content = Url::parse("content://media/1").unwrap();
    assert_eq!(None, StorageProviderHelpers::try_get_path_from_file_uri(Some(&content)));
    // A relative path (the browser's items) has no URL, so no local path.
    let relative = UriRef::parse("a/b.txt").unwrap();
    assert_eq!(None, StorageProviderHelpers::try_get_path_from_file_uri(relative.as_url()));
    assert_eq!(None, StorageProviderHelpers::try_get_path_from_file_uri(None));
}

/// The file URLs of the storage helpers and their local paths are the same on
/// every target, `wasm32-unknown-unknown` included, where `Url::from_file_path`
/// and `Url::to_file_path` do not exist (F12 of
/// docs/idiomatic-rust-signatures.md): a drive path is a drive path and a host
/// is a UNC path everywhere, as for .NET's `Uri.LocalPath`. Where the host has
/// `to_file_path`, the local path of a path of the host is the path it gives.
#[test]
fn a_file_uri_converts_to_a_path_on_every_target() {
    use url::Url;

    let local_path = |text: &str| StorageProviderHelpers::local_path(&Url::parse(text).unwrap());

    // Paths with slashes, escaped and unescaped.
    assert_eq!("/home/a b/c.txt", local_path("file:///home/a%20b/c.txt"));
    assert_eq!("/home/Stahování/x.txt", local_path("file:///home/Stahov%C3%A1n%C3%AD/x.txt"));
    assert_eq!("/home/100%.txt", local_path("file:///home/100%25.txt"));
    // A drive.
    assert_eq!("C:\\dir\\a b.txt", local_path("file:///C:/dir/a%20b.txt"));
    assert_eq!("C:", local_path("file:///C:"));
    assert_eq!("C:\\", local_path("file:///C:/"));
    // A host is a UNC path; `localhost` is no host in a URL.
    assert_eq!("\\\\server\\share\\a.txt", local_path("file://server/share/a.txt"));
    assert_eq!("/a.txt", local_path("file://localhost/a.txt"));
    // Any other scheme gives its unescaped path.
    assert_eq!("/document/a b", local_path("content://com.example/document/a%20b"));

    // The path of the host, where the host has one, agrees with `to_file_path`.
    #[cfg(all(unix, not(target_arch = "wasm32")))]
    for text in ["file:///home/a%20b/c.txt", "file:///tmp/100%25.txt", "file:///"] {
        let url = Url::parse(text).unwrap();
        assert_eq!(url.to_file_path().unwrap(), std::path::PathBuf::from(StorageProviderHelpers::local_path(&url)));
    }
    #[cfg(windows)]
    for text in ["file:///C:/dir/a%20b.txt", "file://server/share/a.txt"] {
        let url = Url::parse(text).unwrap();
        assert_eq!(url.to_file_path().unwrap(), std::path::PathBuf::from(StorageProviderHelpers::local_path(&url)));
    }

    // A path converts to a URL and back.
    for (path, is_directory, url) in [
        ("/home/a b/[x].txt", false, "file:///home/a%20b/%5Bx%5D.txt"),
        ("/home/user/", true, "file:///home/user/"),
        ("C:\\Work\\a#b.txt", false, "file:///C:/Work/a%23b.txt"),
        ("D:\\", false, "file:///D:/"),
    ] {
        let uri = StorageProviderHelpers::uri_from_file_path(path, is_directory);
        assert_eq!(url, uri.as_str());
        assert_eq!(Some(path), StorageProviderHelpers::try_get_path_from_file_uri(Some(&uri)).as_deref());
    }
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
