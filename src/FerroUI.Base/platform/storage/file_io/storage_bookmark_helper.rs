use super::path;

/// In order to have unique bookmarks across platforms, we prepend a
/// platform specific suffix before native bookmark. And always encoding
/// them in base64 before returning to the user.
///
/// Bookmarks are encoded as:
/// 0-6 - framework prefix with version number
/// 7-15  - platform key
/// 16+ - native bookmark value
/// Which is then encoded in Base64.
///
/// This is an implementation detail of the platform backends.
pub struct StorageBookmarkHelper;

const HEADER_LENGTH: usize = 16;
// The prefix is the one of this framework; the layout of the header (prefix
// with version number, platform key, native bookmark) otherwise follows
// upstream.
const HEADER_PREFIX: &[u8] = b"frn.v1.";
const FAKE_BCL_BOOKMARK_PLATFORM: &[u8] = b"bcl";

/// The outcome of decoding a bookmark.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum DecodeResult {
    Success = 0,
    InvalidFormat,
    InvalidPlatform,
}

const BASE64_ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn to_base64(bytes: &[u8]) -> String {
    let mut result = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        result.push(BASE64_ALPHABET[usize::from(b0 >> 2)] as char);
        result.push(BASE64_ALPHABET[usize::from((b0 & 0x03) << 4 | b1 >> 4)] as char);
        result.push(if chunk.len() > 1 { BASE64_ALPHABET[usize::from((b1 & 0x0F) << 2 | b2 >> 6)] as char } else { '=' });
        result.push(if chunk.len() > 2 { BASE64_ALPHABET[usize::from(b2 & 0x3F)] as char } else { '=' });
    }
    result
}

/// Decodes base64 text; white space is skipped. `None` when the text is not
/// valid base64.
fn try_from_base64_chars(text: &str) -> Option<Vec<u8>> {
    fn value(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some(u32::from(c - b'A')),
            b'a'..=b'z' => Some(u32::from(c - b'a') + 26),
            b'0'..=b'9' => Some(u32::from(c - b'0') + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }

    let mut result = Vec::with_capacity(text.len() / 4 * 3);
    let mut quad = [0u8; 4];
    let mut count = 0;
    let mut finished = false;
    for c in text.bytes() {
        if matches!(c, b' ' | b'\t' | b'\r' | b'\n') {
            continue;
        }
        if finished {
            // Nothing may follow the padding.
            return None;
        }
        quad[count] = c;
        count += 1;
        if count < 4 {
            continue;
        }
        count = 0;

        let v0 = value(quad[0])?;
        let v1 = value(quad[1])?;
        match (quad[2], quad[3]) {
            (b'=', b'=') => {
                result.push((v0 << 2 | v1 >> 4) as u8);
                finished = true;
            }
            (c2, b'=') => {
                let v2 = value(c2)?;
                result.push((v0 << 2 | v1 >> 4) as u8);
                result.push((v1 << 4 | v2 >> 2) as u8);
                finished = true;
            }
            (c2, c3) => {
                let v2 = value(c2)?;
                let v3 = value(c3)?;
                result.push((v0 << 2 | v1 >> 4) as u8);
                result.push((v1 << 4 | v2 >> 2) as u8);
                result.push((v2 << 6 | v3) as u8);
            }
        }
    }

    (count == 0).then_some(result)
}

impl StorageBookmarkHelper {
    /// Encodes the text of a native bookmark of the given platform; `None`
    /// for no bookmark and for an empty one.
    ///
    /// # Panics
    /// See [`encode_bookmark_bytes`](Self::encode_bookmark_bytes).
    pub fn encode_bookmark(platform: &[u8], native_bookmark: Option<&str>) -> Option<String> {
        Self::encode_bookmark_bytes(platform, native_bookmark?.as_bytes())
    }

    /// Encodes a native bookmark of the given platform; `None` for an empty
    /// one.
    ///
    /// # Panics
    /// Panics when the platform name is longer than the header, or does not
    /// fit in the encoded bookmark.
    pub fn encode_bookmark_bytes(platform: &[u8], native_bookmark_bytes: &[u8]) -> Option<String> {
        if native_bookmark_bytes.is_empty() {
            return None;
        }

        if platform.len() > HEADER_LENGTH {
            panic!("Platform name should not be longer than {HEADER_LENGTH} bytes");
        }

        let array_length = HEADER_LENGTH + native_bookmark_bytes.len();
        let mut array = vec![0u8; array_length];

        // Write platform into first 16 bytes.
        array[..HEADER_PREFIX.len()].copy_from_slice(HEADER_PREFIX);
        let platform_end = HEADER_PREFIX.len() + platform.len();
        if platform_end > array_length {
            panic!("Destination is too short.");
        }
        array[HEADER_PREFIX.len()..platform_end].copy_from_slice(platform);

        // Write bookmark bytes.
        array[HEADER_LENGTH..].copy_from_slice(native_bookmark_bytes);

        Some(to_base64(&array))
    }

    /// Decodes a bookmark of the given platform into its native bookmark.
    /// The native bookmark is `Some` exactly when the result is
    /// [`DecodeResult::Success`].
    pub fn try_decode_bookmark(platform: &[u8], base64_bookmark: Option<&str>) -> (DecodeResult, Option<Vec<u8>>) {
        let Some(base64_bookmark) = base64_bookmark else {
            return (DecodeResult::InvalidFormat, None);
        };
        if platform.len() > HEADER_LENGTH || platform.is_empty() || base64_bookmark.encode_utf16().count() % 4 != 0 {
            return (DecodeResult::InvalidFormat, None);
        }

        let Some(decoded_bookmark) = try_from_base64_chars(base64_bookmark) else {
            return (DecodeResult::InvalidFormat, None);
        };

        // A decoded bookmark that is too short to hold what is read from it
        // below is reported as invalid.
        let Some(actual_prefix) = decoded_bookmark.get(..HEADER_PREFIX.len()) else {
            return (DecodeResult::InvalidFormat, None);
        };

        if decoded_bookmark.len() < HEADER_LENGTH
            // Check if decoded string starts with the correct prefix, checking v1 at the same time.
            && HEADER_PREFIX != actual_prefix
        {
            return (DecodeResult::InvalidFormat, None);
        }

        let Some(actual_platform) = decoded_bookmark.get(HEADER_PREFIX.len()..HEADER_PREFIX.len() + platform.len())
        else {
            return (DecodeResult::InvalidFormat, None);
        };
        if actual_platform != platform {
            return (DecodeResult::InvalidPlatform, None);
        }

        match decoded_bookmark.get(HEADER_LENGTH..) {
            Some(native_bookmark) => (DecodeResult::Success, Some(native_bookmark.to_vec())),
            None => (DecodeResult::InvalidFormat, None),
        }
    }

    /// Encodes the local path of a file-system backed storage item as a
    /// bookmark.
    ///
    /// # Panics
    /// Panics when the path is empty.
    pub fn encode_bcl_bookmark(local_path: &str) -> String {
        Self::encode_bookmark(FAKE_BCL_BOOKMARK_PLATFORM, Some(local_path)).expect("The local path must not be empty.")
    }

    /// Decodes the bookmark of a file-system backed storage item into its
    /// local path. Bookmarks of the format used before bookmarks were
    /// encoded (plain paths) are accepted too.
    pub fn try_decode_bcl_bookmark(native_bookmark: &str) -> Option<String> {
        let (decode_result, bytes) = Self::try_decode_bookmark(FAKE_BCL_BOOKMARK_PLATFORM, Some(native_bookmark));
        if decode_result == DecodeResult::Success {
            return Some(String::from_utf8_lossy(&bytes.unwrap_or_default()).into_owned());
        }
        if decode_result == DecodeResult::InvalidFormat
            && !native_bookmark.contains(path::is_invalid_path_char)
            && path::get_directory_name(native_bookmark).is_some_and(|directory| !directory.is_empty())
        {
            // Attempt to restore old BCL bookmarks.
            // Don't check for file existence here, as it will be done at later point when the item is created.
            // Just validate if it looks like a valid file path.
            return Some(native_bookmark.to_owned());
        }

        None
    }
}
