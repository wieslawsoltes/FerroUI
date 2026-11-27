//! From the bytes of a font file to a sanitized OpenType font.

use crate::sfnt::{SanitizeOptions, Sanitizer, APPLE_TRUE, COLLECTION, OPEN_TYPE_CFF, TRUETYPE};
use crate::woff::{Woff1Decoder, WOFF1_SIGNATURE};
use crate::woff2::{BrotliDecoderFactory, Woff2Decoder, WOFF2_SIGNATURE};
use crate::FontDecodeError;

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

enum Stage<D> {
    Woff(Box<Woff1Decoder<D>>),
    Woff2(Box<Woff2Decoder<D>>),
    SanitizeInput(Box<Sanitizer<D>>),
    SanitizeDecoded(Box<Sanitizer<Vec<u8>>>),
    Finished,
}

/// Decodes and sanitizes a font in steps, so that a thread that must stay responsive can give
/// it a budget at a time. `D` holds the bytes of the file: borrowed, or owned so that the
/// decoder can be kept between slices of work.
pub struct FontDecoder<D> {
    stage: Stage<D>,
    options: SanitizeOptions,
}

impl<D: AsRef<[u8]>> FontDecoder<D> {
    /// Starts decoding a font file, without a Brotli decoder: WOFF2 is refused.
    pub fn new(data: D, options: SanitizeOptions) -> Result<Self, FontDecodeError> {
        Self::with_brotli(data, options, None)
    }

    /// Starts decoding a font file. Headers are validated here, before any large allocation.
    /// `brotli` makes the decoders WOFF2 needs.
    pub fn with_brotli(
        data: D,
        options: SanitizeOptions,
        brotli: Option<BrotliDecoderFactory>,
    ) -> Result<Self, FontDecodeError> {
        if data.as_ref().len() > options.limits.max_encoded_size {
            return Err(FontDecodeError::LimitExceeded("encoded size"));
        }
        let stage = match sniff_container(data.as_ref()).ok_or(FontDecodeError::UnknownFormat)? {
            FontContainer::OpenType | FontContainer::Collection => {
                Stage::SanitizeInput(Box::new(Sanitizer::new(data, options)?))
            }
            FontContainer::Woff => Stage::Woff(Box::new(Woff1Decoder::new(data, &options.limits)?)),
            FontContainer::Woff2 => {
                let brotli = brotli.ok_or(FontDecodeError::Unsupported("WOFF2 needs a Brotli decoder"))?;
                Stage::Woff2(Box::new(Woff2Decoder::new(data, &options, brotli)?))
            }
        };
        Ok(Self { stage, options })
    }

    /// Does up to about `budget` bytes of work. Returns the font when it is finished, after
    /// which the decoder has nothing more to give. After an error it has nothing more either.
    pub fn step(&mut self, budget: usize) -> Result<Option<Vec<u8>>, FontDecodeError> {
        let result = self.advance(budget.max(1));
        if !matches!(result, Ok(None)) {
            self.stage = Stage::Finished;
        }
        result
    }

    fn advance(&mut self, budget: usize) -> Result<Option<Vec<u8>>, FontDecodeError> {
        match &mut self.stage {
            Stage::Woff(decoder) => {
                if decoder.step(budget)? {
                    let Stage::Woff(decoder) = std::mem::replace(&mut self.stage, Stage::Finished) else {
                        return Err(FontDecodeError::Malformed("the decoder changed stage"));
                    };
                    self.stage = Stage::SanitizeDecoded(Box::new(Sanitizer::new(decoder.into_font(), self.options)?));
                }
                Ok(None)
            }
            Stage::Woff2(decoder) => {
                if let Some(font) = decoder.step(budget)? {
                    self.stage = Stage::SanitizeDecoded(Box::new(Sanitizer::new(font, self.options)?));
                }
                Ok(None)
            }
            Stage::SanitizeInput(sanitizer) => sanitizer.step(budget),
            Stage::SanitizeDecoded(sanitizer) => {
                if !sanitizer.advance(budget)? {
                    return Ok(None);
                }
                let Stage::SanitizeDecoded(sanitizer) = std::mem::replace(&mut self.stage, Stage::Finished) else {
                    return Err(FontDecodeError::Malformed("the decoder changed stage"));
                };
                sanitizer.into_font().map(Some)
            }
            Stage::Finished => Err(FontDecodeError::Malformed("the decoder has already finished")),
        }
    }
}

/// Decodes and sanitizes a font in one go, without a Brotli decoder.
pub fn decode_font(data: &[u8], options: &SanitizeOptions) -> Result<Vec<u8>, FontDecodeError> {
    decode_font_with_brotli(data, options, None)
}

/// Decodes and sanitizes a font in one go. `brotli` makes the decoders WOFF2 needs.
pub fn decode_font_with_brotli(
    data: &[u8],
    options: &SanitizeOptions,
    brotli: Option<BrotliDecoderFactory>,
) -> Result<Vec<u8>, FontDecodeError> {
    let mut decoder = FontDecoder::with_brotli(data, *options, brotli)?;
    loop {
        if let Some(font) = decoder.step(usize::MAX)? {
            return Ok(font);
        }
    }
}
