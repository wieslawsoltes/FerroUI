use crate::document::TextSnapshot;
use std::fmt;
use std::io::{self, BufWriter, Write};

const WINDOWS_1252_C1: [char; 32] = [
    '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8d}', 'Ž', '\u{8f}',
    '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9d}', 'ž', 'Ÿ',
];

pub(super) fn decode_windows_1252(byte: u8) -> char {
    match byte {
        0x80..=0x9f => WINDOWS_1252_C1[usize::from(byte - 0x80)],
        _ => char::from(byte),
    }
}

ferroui_system::styled_value!(TextEncoding);

/// Encodings supported by the editor's dependency-free file reader.
/// Byte-order marks are reported separately from the selected encoding.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextEncoding {
    #[default]
    Utf8,
    Utf16Le,
    Utf16Be,
    Utf32Le,
    Utf32Be,
    Ascii,
    Latin1,
    Windows1252,
}

impl TextEncoding {
    pub const fn is_unicode(self) -> bool {
        matches!(
            self,
            Self::Utf8 | Self::Utf16Le | Self::Utf16Be | Self::Utf32Le | Self::Utf32Be
        )
    }

    pub const fn is_ascii_compatible(self) -> bool {
        matches!(
            self,
            Self::Utf8 | Self::Ascii | Self::Latin1 | Self::Windows1252
        )
    }

    pub const fn byte_order_mark(self) -> &'static [u8] {
        match self {
            Self::Utf8 => &[0xef, 0xbb, 0xbf],
            Self::Utf16Le => &[0xff, 0xfe],
            Self::Utf16Be => &[0xfe, 0xff],
            Self::Utf32Le => &[0xff, 0xfe, 0, 0],
            Self::Utf32Be => &[0, 0, 0xfe, 0xff],
            Self::Ascii | Self::Latin1 | Self::Windows1252 => &[],
        }
    }

    /// Writes an immutable snapshot and flushes the caller's stream. An
    /// unpaired surrogate or an unrepresentable scalar yields InvalidData.
    pub fn write_snapshot(
        self,
        snapshot: &TextSnapshot,
        target: impl Write,
        write_bom: bool,
    ) -> io::Result<()> {
        let mut writer = BufWriter::new(target);
        if write_bom {
            writer.write_all(self.byte_order_mark())?;
        }
        for scalar in char::decode_utf16(snapshot.create_reader()) {
            let scalar =
                scalar.map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            match self {
                Self::Utf8 => writer.write_all(scalar.encode_utf8(&mut [0; 4]).as_bytes())?,
                Self::Utf16Le | Self::Utf16Be => {
                    for &mut unit in scalar.encode_utf16(&mut [0; 2]) {
                        let bytes = if self == Self::Utf16Le {
                            unit.to_le_bytes()
                        } else {
                            unit.to_be_bytes()
                        };
                        writer.write_all(&bytes)?;
                    }
                }
                Self::Utf32Le => writer.write_all(&(scalar as u32).to_le_bytes())?,
                Self::Utf32Be => writer.write_all(&(scalar as u32).to_be_bytes())?,
                Self::Ascii | Self::Latin1 | Self::Windows1252 => {
                    let byte = self.single_byte(scalar).ok_or_else(|| {
                        io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("{scalar:?} is not representable in {self}"),
                        )
                    })?;
                    writer.write_all(&[byte])?;
                }
            }
        }
        writer.flush()
    }

    fn single_byte(self, scalar: char) -> Option<u8> {
        match (self, u8::try_from(scalar as u32).ok()) {
            (Self::Ascii, Some(byte @ 0..=0x7f))
            | (Self::Latin1, Some(byte))
            | (Self::Windows1252, Some(byte @ (0..=0x7f | 0xa0..=0xff))) => Some(byte),
            (Self::Windows1252, _) => WINDOWS_1252_C1
                .iter()
                .position(|&mapped| mapped == scalar)
                .map(|index| 0x80 + index as u8),
            _ => None,
        }
    }
}

impl fmt::Display for TextEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Utf8 => "UTF-8",
            Self::Utf16Le => "UTF-16 LE",
            Self::Utf16Be => "UTF-16 BE",
            Self::Utf32Le => "UTF-32 LE",
            Self::Utf32Be => "UTF-32 BE",
            Self::Ascii => "ASCII",
            Self::Latin1 => "ISO-8859-1",
            Self::Windows1252 => "Windows-1252",
        })
    }
}

/// Malformed encoded input either produces replacement characters or an
/// `InvalidData` IO error. ASCII replacement uses `?`; Unicode uses U+FFFD.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DecodingErrorPolicy {
    #[default]
    Replace,
    Strict,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FileReaderOptions {
    pub default_encoding: TextEncoding,
    pub error_policy: DecodingErrorPolicy,
}
