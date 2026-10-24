use std::fmt;

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
