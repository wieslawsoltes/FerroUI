//! Incremental Font Transfer (W3C Candidate Recommendation Draft, 13 October 2026), the client
//! side, for **glyph-keyed patches only**.
//!
//! An incremental font is an ordinary font with some glyphs left empty and one or two patch map
//! tables that say which patch holds what. Table-keyed patches use the LZ77 dictionary of Shared
//! Brotli, which the decoder of this project does not have, so a font that still needs one is
//! refused as a whole by [`read_patch_maps`] and the face moves on to its next `src` entry. It is
//! never decoded approximately.
//!
//! Everything here reads untrusted bytes: it validates, it does not panic, and every allocation
//! is bounded by [`WebFontLimits`].

mod map;
mod patch;
mod sparse;
mod template;

#[cfg(any(test, feature = "testing"))]
pub mod testing;
#[cfg(test)]
mod tests;

pub use map::{parse_patch_map, patches_to_load, PatchMap, SubsetTarget};
pub use patch::GlyphPatchJob;

use crate::sfnt::{read_directory, Tag};
use crate::{FontDecodeError, WebFontLimits};

/// The patch map table every incremental font has.
pub const IFT_TABLE: Tag = *b"IFT ";
/// The second, optional patch map table.
pub const IFTX_TABLE: Tag = *b"IFTX";

/// The layout features a target includes by default (Appendix A of the standard).
pub const DEFAULT_FEATURES: &[Tag] = &[
    *b"abvf", *b"abvm", *b"abvs", *b"akhn", *b"blwf", *b"blwm", *b"blws", *b"calt", *b"ccmp", *b"cfar", *b"chws",
    *b"cjct", *b"clig", *b"cswh", *b"curs", *b"dist", *b"dnom", *b"dtls", *b"fin2", *b"fin3", *b"fina", *b"flac",
    *b"frac", *b"half", *b"haln", *b"init", *b"isol", *b"jalt", *b"kern", *b"liga", *b"ljmo", *b"locl", *b"ltra",
    *b"ltrm", *b"mark", *b"med2", *b"medi", *b"mkmk", *b"mset", *b"nukt", *b"numr", *b"pref", *b"pres", *b"pstf",
    *b"psts", *b"rand", *b"rclt", *b"rkrf", *b"rlig", *b"rphf", *b"rtla", *b"rtlm", *b"rvrn", *b"ssty", *b"stch",
    *b"tjmo", *b"valt", *b"vatu", *b"vchw", *b"vert", *b"vjmo", *b"vkrn", *b"vpal", *b"vrt2", *b"vrtr",
];

/// The most patches one font may take in one run of the extension algorithm (section 4.3).
pub const MAX_PATCHES: usize = 2000;

/// The patch maps of a font, `IFT ` first. Empty when the font is not incremental. Fails when the
/// maps are invalid, when both have the same compatibility id, and when an entry still needs a
/// table-keyed patch.
pub fn read_patch_maps(font: &[u8], limits: &WebFontLimits) -> Result<Vec<PatchMap>, FontDecodeError> {
    let (_, records) = read_directory(font, 0, limits)?;
    let table = |tag: &Tag| {
        records.iter().find(|record| record.tag == *tag).map(|record| &font[record.offset..][..record.length])
    };
    let Some(first) = table(&IFT_TABLE) else {
        return Ok(Vec::new());
    };
    patch_maps_from_tables(first, table(&IFTX_TABLE))
}

/// The patch maps of a font from its `IFT ` and `IFTX` tables, with the checks of
/// [`read_patch_maps`].
pub fn patch_maps_from_tables(first: &[u8], second: Option<&[u8]>) -> Result<Vec<PatchMap>, FontDecodeError> {
    let mut maps = vec![parse_patch_map(IFT_TABLE, first)?];
    if let Some(second) = second {
        let second = parse_patch_map(IFTX_TABLE, second)?;
        if second.compatibility_id() == maps[0].compatibility_id() {
            return Err(FontDecodeError::Malformed("IFT and IFTX have the same compatibility id"));
        }
        maps.push(second);
    }
    if maps.iter().any(PatchMap::needs_table_keyed_patch) {
        return Err(FontDecodeError::Unsupported("table-keyed patches"));
    }
    Ok(maps)
}
