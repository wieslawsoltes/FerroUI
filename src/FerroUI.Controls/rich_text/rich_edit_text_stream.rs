//! Portable counterparts of the TOM document/range stream methods. The WinRT
//! random-access ABI is replaced by the Read/Write traits used by FerroUI storage.
use super::*;
use std::io::{self, Read, Write};

/// Explicit encoding of plain text. Native stream encodings are not established
/// by the pinned WinUI implementation, which delegates to an external text engine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TextStreamEncoding {
    #[default]
    Utf8,
    Utf16LittleEndian,
    Utf16BigEndian,
}
ferroui_base::ferro_markup_enum!(TextStreamEncoding {
    Utf8, Utf16LittleEndian, Utf16BigEndian,
}, { namespace: "FerroUI.Controls" });

/// Stream policy supplied explicitly at each call. A matching leading BOM is
/// accepted and removed on input; a contradictory BOM is an encoding error.
/// RTF uses its own byte encoding and requires the default Utf8/no-BOM policy,
/// without transcoding its byte payload. The byte bound also applies to output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextStreamOptions {
    pub encoding: TextStreamEncoding,
    pub write_byte_order_mark: bool,
    /// Maximum bytes consumed or emitted. Zero permits only an empty stream.
    pub max_bytes: usize,
}
impl Default for TextStreamOptions {
    fn default() -> Self {
        Self {
            encoding: TextStreamEncoding::Utf8,
            write_byte_order_mark: false,
            max_bytes: super::super::RtfLimits::default().max_bytes,
        }
    }
}

impl RichEditTextDocument {
    /// Loads from the reader's current position. I/O, decoding, and parsing finish
    /// before document/default/history changes or composition cancellation. The
    /// caller retains ownership of the reader; it is never sought or closed.
    pub fn load_from_stream(
        &self,
        options: TextSetOptions,
        reader: &mut dyn Read,
        stream: TextStreamOptions,
    ) -> Result<(), TextOperationError> {
        validate_set_options(options)?;
        validate_stream_policy(options.contains(TextSetOptions::FORMAT_RTF), stream)?;
        let bytes = read_bounded(reader, stream.max_bytes)?;
        if options.contains(TextSetOptions::FORMAT_RTF) {
            let snapshot = read_rtf(&bytes, stream.max_bytes)?;
            self.replace_document(
                &snapshot,
                options.contains(TextSetOptions::CHECK_TEXT_LIMIT),
            )?;
        } else {
            self.set_text_with_options(options, &decode_plain(&bytes, stream.encoding)?)?;
        }
        Ok(())
    }

    /// Writes an encoded snapshot at the writer's current position. Validation and
    /// encoding finish before the first write. An I/O error may leave a written
    /// prefix; this method never seeks, truncates, flushes, or closes the writer.
    pub fn save_to_stream(
        &self,
        options: TextGetOptions,
        writer: &mut dyn Write,
        stream: TextStreamOptions,
    ) -> Result<(), TextOperationError> {
        validate_get_options(options)?;
        let rtf = options.contains(TextGetOptions::FORMAT_RTF);
        validate_stream_policy(rtf, stream)?;
        let text = self.get_text(options)?;
        write_encoded(writer, &text, rtf, stream)
    }
}

impl RichEditTextRange {
    /// Replaces this live range from the reader's current position. The complete
    /// payload is validated first; the editor's independent selection is rebased
    /// and the replacement becomes one undoable operation.
    pub fn set_text_via_stream(
        &self,
        options: TextSetOptions,
        reader: &mut dyn Read,
        stream: TextStreamOptions,
    ) -> Result<(), TextOperationError> {
        validate_set_options(options)?;
        validate_stream_policy(options.contains(TextSetOptions::FORMAT_RTF), stream)?;
        let bytes = read_bounded(reader, stream.max_bytes)?;
        if options.contains(TextSetOptions::FORMAT_RTF) {
            let snapshot = read_rtf(&bytes, stream.max_bytes)?;
            self.replace_with_options(options, "", Some(&snapshot))?;
        } else {
            self.set_text_with_options(options, &decode_plain(&bytes, stream.encoding)?)?;
        }
        Ok(())
    }

    /// Writes this range without moving its endpoints or changing editor history.
    /// The stream's position, lifetime, and completion/flush policy belong to the
    /// caller, as in FerroUI's other Read/Write-based storage APIs.
    pub fn get_text_via_stream(
        &self,
        options: TextGetOptions,
        writer: &mut dyn Write,
        stream: TextStreamOptions,
    ) -> Result<(), TextOperationError> {
        validate_get_options(options)?;
        let rtf = options.contains(TextGetOptions::FORMAT_RTF);
        validate_stream_policy(rtf, stream)?;
        let text = self.get_text(options)?;
        write_encoded(writer, &text, rtf, stream)
    }
}

fn validate_stream_policy(rtf: bool, stream: TextStreamOptions) -> Result<(), TextOperationError> {
    if rtf && (stream.encoding != TextStreamEncoding::Utf8 || stream.write_byte_order_mark) {
        return Err(TextOperationError::InvalidOptions(
            "RTF streams use raw codec bytes and require the Utf8/no-BOM stream policy.",
        ));
    }
    Ok(())
}

fn read_bounded(reader: &mut dyn Read, limit: usize) -> Result<Vec<u8>, TextOperationError> {
    let mut result = Vec::new();
    let mut buffer = [0; 8192];
    loop {
        // Read at most one extra byte to distinguish an exact-size payload from
        // an oversized one without consuming the rest of an arbitrary stream.
        let request = buffer
            .len()
            .min(limit.saturating_sub(result.len()).saturating_add(1));
        let length = match reader.read(&mut buffer[..request]) {
            Ok(0) => return Ok(result),
            Ok(length) => length,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
        };
        if length > limit.saturating_sub(result.len()) {
            return Err(TextOperationError::StreamLimitExceeded { limit });
        }
        result
            .try_reserve(length)
            .map_err(|error| io::Error::new(io::ErrorKind::OutOfMemory, error))?;
        result.extend_from_slice(&buffer[..length]);
    }
}

fn read_rtf(bytes: &[u8], max_bytes: usize) -> Result<DocumentSnapshot, RtfError> {
    RtfReader::read_with_limits(
        bytes,
        super::super::RtfLimits {
            max_bytes,
            ..Default::default()
        },
    )
}

fn bom(encoding: TextStreamEncoding) -> &'static [u8] {
    match encoding {
        TextStreamEncoding::Utf8 => &[0xef, 0xbb, 0xbf],
        TextStreamEncoding::Utf16LittleEndian => &[0xff, 0xfe],
        TextStreamEncoding::Utf16BigEndian => &[0xfe, 0xff],
    }
}

fn decode_plain(bytes: &[u8], encoding: TextStreamEncoding) -> Result<String, TextOperationError> {
    let mut bytes = bytes;
    for candidate in [
        TextStreamEncoding::Utf8,
        TextStreamEncoding::Utf16LittleEndian,
        TextStreamEncoding::Utf16BigEndian,
    ] {
        if let Some(content) = bytes.strip_prefix(bom(candidate)) {
            if candidate != encoding {
                return Err(TextOperationError::InvalidEncoding(
                    "The stream BOM contradicts the selected text encoding.",
                ));
            }
            bytes = content;
            break;
        }
    }
    if encoding == TextStreamEncoding::Utf8 {
        return String::from_utf8(bytes.to_vec())
            .map_err(|_| TextOperationError::InvalidEncoding("The stream is not valid UTF-8."));
    }
    if bytes.len() % 2 != 0 {
        return Err(TextOperationError::InvalidEncoding(
            "A UTF-16 stream must contain complete two-byte code units.",
        ));
    }
    let units: Vec<_> = bytes
        .chunks_exact(2)
        .map(|pair| {
            let pair = [pair[0], pair[1]];
            if encoding == TextStreamEncoding::Utf16LittleEndian {
                u16::from_le_bytes(pair)
            } else {
                u16::from_be_bytes(pair)
            }
        })
        .collect();
    String::from_utf16(&units).map_err(|_| {
        TextOperationError::InvalidEncoding("The UTF-16 stream contains an unpaired surrogate.")
    })
}

fn write_encoded(
    writer: &mut dyn Write,
    text: &str,
    rtf: bool,
    stream: TextStreamOptions,
) -> Result<(), TextOperationError> {
    let prefix = if stream.write_byte_order_mark {
        bom(stream.encoding)
    } else {
        &[]
    };
    let content_length = if rtf || stream.encoding == TextStreamEncoding::Utf8 {
        Some(text.len())
    } else {
        text.encode_utf16().count().checked_mul(2)
    };
    let length = content_length
        .and_then(|length| length.checked_add(prefix.len()))
        .filter(|length| *length <= stream.max_bytes)
        .ok_or(TextOperationError::StreamLimitExceeded {
            limit: stream.max_bytes,
        })?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length)
        .map_err(|error| io::Error::new(io::ErrorKind::OutOfMemory, error))?;
    bytes.extend_from_slice(prefix);
    if rtf || stream.encoding == TextStreamEncoding::Utf8 {
        bytes.extend_from_slice(text.as_bytes());
    } else {
        for unit in text.encode_utf16() {
            let encoded = if stream.encoding == TextStreamEncoding::Utf16LittleEndian {
                unit.to_le_bytes()
            } else {
                unit.to_be_bytes()
            };
            bytes.extend_from_slice(&encoded);
        }
    }
    writer.write_all(&bytes)?;
    Ok(())
}

#[cfg(test)]
#[path = "rich_edit_text_stream_tests.rs"]
mod tests;
