//! The FerroUI application formats shared between desktop and web clipboards.

use crate::input::DataFormat;
use std::fmt::Write;
use std::rc::Rc;

/// The prefix that the ClipboardItem API adds to custom MIME types.
pub const WEB_PREFIX: &str = "web ";

const APP_PREFIX: &str = "web application/frn-fmt.";

/// Encodes a case-sensitive application identifier as a lowercase MIME name.
/// The representation also records whether the payload is UTF-8 or raw bytes.
pub fn to_web_clipboard_format(identifier: &str, text: bool) -> String {
    let mut name = String::from(APP_PREFIX);
    name.push_str(if text { "text." } else { "bytes." });
    for byte in identifier.bytes() {
        let _ = write!(name, "{byte:02x}");
    }
    name
}

/// Decodes a FerroUI application format from a ClipboardItem type.
pub fn from_web_clipboard_format(name: &str) -> Option<DataFormat> {
    let (representation, encoded) = name.strip_prefix(APP_PREFIX)?.split_once('.')?;
    if encoded.is_empty() || encoded.len() % 2 != 0 {
        return None;
    }
    let identifier = encoded
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok())
        .collect::<Option<Vec<_>>>()?;
    if !identifier
        .iter()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-'))
    {
        return None;
    }
    let identifier = std::str::from_utf8(&identifier).ok()?;
    match representation {
        "text" => Some(DataFormat::create_string_application_format(identifier).into()),
        "bytes" => Some(DataFormat::create_bytes_application_format(identifier).into()),
        _ => None,
    }
}
