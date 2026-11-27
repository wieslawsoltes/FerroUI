//! From the bytes of a font file to a sanitized OpenType font.

use crate::sfnt::{SanitizeOptions, Sanitizer, APPLE_TRUE, COLLECTION, OPEN_TYPE_CFF, TRUETYPE};
use crate::woff::{Woff1Decoder, WOFF1_SIGNATURE};
use crate::FontDecodeError;

const WOFF2_SIGNATURE: u32 = u32::from_be_bytes(*b"wOF2");

/// The container of a font file, told from its first bytes. CSS treats `format()` as a hint, so
/// this is what decides how a file is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontContainer {
    OpenType,
    Collection,
    Woff,
    Woff2,
}

/// The container of a font file; `None` when it is none of the known ones.
pub fn sniff_container(data: &[u8]) -> Option<FontContainer> {
    let signature: [u8; 4] = data.get(..4)?.try_into().ok()?;
    match u32::from_be_bytes(signature) {
        TRUETYPE | APPLE_TRUE | OPEN_TYPE_CFF => Some(FontContainer::OpenType),
        COLLECTION => Some(FontContainer::Collection),
        WOFF1_SIGNATURE => Some(FontContainer::Woff),
        WOFF2_SIGNATURE => Some(FontContainer::Woff2),
        _ => None,
    }
}

enum Stage<'a> {
    Woff(Box<Woff1Decoder<'a>>),
    SanitizeInput(Box<Sanitizer<&'a [u8]>>),
    SanitizeDecoded(Box<Sanitizer<Vec<u8>>>),
    Finished,
}

/// Decodes and sanitizes a font in steps, so that a thread that must stay responsive can give
/// it a budget at a time.
pub struct FontDecoder<'a> {
    stage: Stage<'a>,
    options: SanitizeOptions,
}

impl<'a> FontDecoder<'a> {
    /// Starts decoding a font file. Headers are validated here, before any large allocation.
    pub fn new(data: &'a [u8], options: SanitizeOptions) -> Result<Self, FontDecodeError> {
        if data.len() > options.limits.max_encoded_size {
            return Err(FontDecodeError::LimitExceeded("encoded size"));
        }
        let stage = match sniff_container(data).ok_or(FontDecodeError::UnknownFormat)? {
            FontContainer::OpenType | FontContainer::Collection => {
                Stage::SanitizeInput(Box::new(Sanitizer::new(data, options)?))
            }
            FontContainer::Woff => Stage::Woff(Box::new(Woff1Decoder::new(data, &options.limits)?)),
            FontContainer::Woff2 => return Err(FontDecodeError::Unsupported("WOFF2")),
        };
        Ok(Self { stage, options })
    }

    /// Does up to about `budget` bytes of work. Returns the font when it is finished, after
    /// which the decoder has nothing more to give.
    pub fn step(&mut self, budget: usize) -> Result<Option<Vec<u8>>, FontDecodeError> {
        let budget = budget.max(1);
        match &mut self.stage {
            Stage::Woff(decoder) => {
                if decoder.step(budget)? {
                    let Stage::Woff(decoder) = std::mem::replace(&mut self.stage, Stage::Finished) else {
                        return Err(FontDecodeError::Malformed("the decoder changed stage"));
                    };
                    let font = decoder.into_font();
                    self.stage = Stage::SanitizeDecoded(Box::new(Sanitizer::new(font, self.options)?));
                }
                Ok(None)
            }
            Stage::SanitizeInput(sanitizer) => Self::finish_with(sanitizer.step(budget), &mut self.stage),
            Stage::SanitizeDecoded(sanitizer) => Self::finish_with(sanitizer.step(budget), &mut self.stage),
            Stage::Finished => Err(FontDecodeError::Malformed("the decoder has already finished")),
        }
    }

    fn finish_with(
        result: Result<Option<Vec<u8>>, FontDecodeError>,
        stage: &mut Stage<'a>,
    ) -> Result<Option<Vec<u8>>, FontDecodeError> {
        if !matches!(result, Ok(None)) {
            *stage = Stage::Finished;
        }
        result
    }
}

/// Decodes and sanitizes a font in one go.
pub fn decode_font(data: &[u8], options: &SanitizeOptions) -> Result<Vec<u8>, FontDecodeError> {
    let mut decoder = FontDecoder::new(data, *options)?;
    loop {
        if let Some(font) = decoder.step(usize::MAX)? {
            return Ok(font);
        }
    }
}
