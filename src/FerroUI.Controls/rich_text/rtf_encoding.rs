//! Byte-to-Unicode boundary for RTF text, font tables and field instructions.
//!
//! RTF `ansicpg`, `fcharset` and `cpg` choose encodings independently of the
//! document's language. A multibyte sequence may span raw text and hex escapes,
//! but a formatting/scope boundary must not hide an incomplete sequence.

//! The legacy code pages come from `encoding_rs` with the cargo feature `rtf-codepages`
//! (`docs/porting/winui-rich-text.md`, decision D2). Without it, RTF text bytes are read
//! in the ANSI code page 1252 only, and other text comes from Unicode escapes; a
//! document that declares another code page fails with `UnsupportedCodePage`.

use super::RtfError;

#[cfg(feature = "rtf-codepages")]
pub(super) use codepages::{validate_code_page, EncodedText};
#[cfg(not(feature = "rtf-codepages"))]
pub(super) use ansi::{validate_code_page, EncodedText};

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

#[cfg(feature = "rtf-codepages")]
mod codepages {
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

    pub(crate) fn validate_code_page(code_page: i32) -> Result<(), RtfError> {
        encoding(code_page).map(|_| ())
    }

    /// Keeps at most the selected codec's incomplete byte sequence. Output is
    /// emitted from a fixed stack buffer, so a large run requires no second copy
    /// of the input and no temporary UTF-8 string.
    #[derive(Default)]
    pub(crate) struct EncodedText {
        active: Option<(i32, Decoder)>,
    }

    impl EncodedText {
        pub(crate) fn push(
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

        pub(crate) fn finish(
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
}

#[cfg(not(feature = "rtf-codepages"))]
mod ansi {
    use super::RtfError;

    /// The characters of the bytes 0x80 to 0x9F in the code page 1252 (`None`: unassigned).
    const WINDOWS_1252_HIGH: [Option<char>; 32] = [
        Some('\u{20AC}'), None, Some('\u{201A}'), Some('\u{0192}'), Some('\u{201E}'), Some('\u{2026}'),
        Some('\u{2020}'), Some('\u{2021}'), Some('\u{02C6}'), Some('\u{2030}'), Some('\u{0160}'),
        Some('\u{2039}'), Some('\u{0152}'), None, Some('\u{017D}'), None, None, Some('\u{2018}'),
        Some('\u{2019}'), Some('\u{201C}'), Some('\u{201D}'), Some('\u{2022}'), Some('\u{2013}'),
        Some('\u{2014}'), Some('\u{02DC}'), Some('\u{2122}'), Some('\u{0161}'), Some('\u{203A}'),
        Some('\u{0153}'), None, Some('\u{017E}'), Some('\u{0178}'),
    ];

    pub(crate) fn validate_code_page(code_page: i32) -> Result<(), RtfError> {
        if code_page == 1252 {
            Ok(())
        } else {
            Err(RtfError::UnsupportedCodePage(code_page))
        }
    }

    /// The code page 1252 is a single-byte code page: no sequence spans two pushes.
    #[derive(Default)]
    pub(crate) struct EncodedText;

    impl EncodedText {
        pub(crate) fn push(
            &mut self,
            code_page: i32,
            bytes: &[u8],
            mut emit: impl FnMut(&[u16]) -> Result<(), RtfError>,
        ) -> Result<(), RtfError> {
            validate_code_page(code_page)?;
            let mut output = [0_u16; 512];
            for chunk in bytes.chunks(output.len()) {
                for (slot, &byte) in output.iter_mut().zip(chunk) {
                    let character = match byte {
                        0x80..=0x9F => WINDOWS_1252_HIGH[usize::from(byte - 0x80)]
                            .ok_or(RtfError::InvalidEncodedText { code_page })?,
                        _ => char::from(byte),
                    };
                    *slot = character as u16;
                }
                emit(&output[..chunk.len()])?;
            }
            Ok(())
        }

        pub(crate) fn finish(&mut self, _emit: impl FnMut(&[u16]) -> Result<(), RtfError>) -> Result<(), RtfError> {
            Ok(())
        }
    }
}
