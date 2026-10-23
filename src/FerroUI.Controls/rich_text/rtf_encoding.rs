//! Byte-to-Unicode boundary for RTF text, font tables and field instructions.
//!
//! RTF `ansicpg`, `fcharset` and `cpg` choose encodings independently of the
//! document's language. A multibyte sequence may span raw text and hex escapes,
//! but a formatting/scope boundary must not hide an incomplete sequence.

use super::RtfError;
use encoding_rs::{Decoder, DecoderResult, Encoding};

fn encoding(code_page: i32) -> Result<&'static Encoding, RtfError> {
    Ok(match code_page {
        874 => encoding_rs::WINDOWS_874,
        932 => encoding_rs::SHIFT_JIS,
        936 => encoding_rs::GBK,
        949 => encoding_rs::EUC_KR,
        950 => encoding_rs::BIG5,
        1250 => encoding_rs::WINDOWS_1250,
        1251 => encoding_rs::WINDOWS_1251,
        1252 => encoding_rs::WINDOWS_1252,
        1253 => encoding_rs::WINDOWS_1253,
        1254 => encoding_rs::WINDOWS_1254,
        1255 => encoding_rs::WINDOWS_1255,
        1256 => encoding_rs::WINDOWS_1256,
        1257 => encoding_rs::WINDOWS_1257,
        1258 => encoding_rs::WINDOWS_1258,
        10000 => encoding_rs::MACINTOSH,
        10007 => encoding_rs::X_MAC_CYRILLIC,
        65001 => encoding_rs::UTF_8,
        _ => return Err(RtfError::UnsupportedCodePage(code_page)),
    })
}

pub(super) fn validate_code_page(code_page: i32) -> Result<(), RtfError> {
    encoding(code_page).map(|_| ())
}

/// RTF's charset identifiers are not Windows code-page identifiers. `DEFAULT`
/// retains the document's explicitly declared ANSI code page; `ANSI` is 1252.
pub(super) fn font_code_page(charset: i32) -> Result<Option<i32>, RtfError> {
    let code_page = match charset {
        0 => 1252,
        1 => return Ok(None),
        77 => 10000,
        128 => 932,
        129 => 949,
        130 => 1361,
        134 => 936,
        136 => 950,
        161 => 1253,
        162 => 1254,
        163 => 1258,
        177 => 1255,
        178 => 1256,
        186 => 1257,
        204 => 1251,
        222 => 874,
        238 => 1250,
        255 => 437,
        _ => return Err(RtfError::UnsupportedFeature("font character set")),
    };
    // A following explicit `cpg` may override this declaration before any
    // bytes are decoded. Validate the effective codec at the byte boundary.
    Ok(Some(code_page))
}

/// Keeps at most the selected codec's incomplete byte sequence. Output is
/// emitted from a fixed stack buffer, so a large run requires no second copy
/// of the input and no temporary UTF-8 string.
#[derive(Default)]
pub(super) struct EncodedText {
    active: Option<(i32, Decoder)>,
}

impl EncodedText {
    pub(super) fn push(
        &mut self,
        code_page: i32,
        bytes: &[u8],
        emit: impl FnMut(&[u16]) -> Result<(), RtfError>,
    ) -> Result<(), RtfError> {
        if bytes.is_empty() {
            return Ok(());
        }
        if self.active.is_none() {
            self.active = Some((
                code_page,
                encoding(code_page)?.new_decoder_without_bom_handling(),
            ));
        }
        let (active_page, decoder) = self.active.as_mut().expect("initialized above");
        if *active_page != code_page {
            // Every syntax transition finishes the preceding byte run first.
            return Err(RtfError::InvalidParameter("unclosed code-page transition"));
        }
        Self::decode(decoder, code_page, bytes, false, emit)
    }

    pub(super) fn finish(
        &mut self,
        emit: impl FnMut(&[u16]) -> Result<(), RtfError>,
    ) -> Result<(), RtfError> {
        if let Some((code_page, mut decoder)) = self.active.take() {
            Self::decode(&mut decoder, code_page, &[], true, emit)?;
        }
        Ok(())
    }

    fn decode(
        decoder: &mut Decoder,
        code_page: i32,
        mut bytes: &[u8],
        last: bool,
        mut emit: impl FnMut(&[u16]) -> Result<(), RtfError>,
    ) -> Result<(), RtfError> {
        let mut output = [0_u16; 512];
        loop {
            let (result, consumed, written) =
                decoder.decode_to_utf16_without_replacement(bytes, &mut output, last);
            emit(&output[..written])?;
            bytes = &bytes[consumed..];
            match result {
                DecoderResult::InputEmpty => return Ok(()),
                DecoderResult::OutputFull => {}
                DecoderResult::Malformed(_, _) => {
                    return Err(RtfError::InvalidEncodedText { code_page });
                }
            }
        }
    }
}
