//! OpenType table readers.

mod big_endian_binary_reader;
mod decycler;
mod encoding_id_extensions;
mod encoding_ids;
mod feature_list_table;
mod font_version;
mod head_table;
mod horizontal_header_table;
mod invalid_font_table_exception;
mod known_name_ids;
mod loca_table;
mod maxp_table;
mod meta_table;
mod missing_font_table_exception;
mod os2_table;
mod panose;
mod platform_id;
mod post_table;
mod script_list_table;
mod vertical_header_table;

pub mod cmap;
pub mod glyf;
pub mod metrics;
pub mod name;

#[cfg(any(test, feature = "testing"))]
pub(crate) mod testing;

#[cfg(test)]
mod head_table_tests;
#[cfg(test)]
mod loca_table_tests;
#[cfg(test)]
mod maxp_table_tests;
#[cfg(test)]
mod os2_table_tests;

pub use big_endian_binary_reader::{BigEndianBinaryReader, FontTableError};
pub use decycler::{CycleGuard, Decycler, DecyclerError, DecyclerException};
pub use encoding_id_extensions::{Encoding, EncodingIDExtensions};
pub use encoding_ids::EncodingIDs;
pub use feature_list_table::FeatureListTable;
pub use font_version::FontVersion;
pub use head_table::{FontDirectionHint, GlyphDataFormat, HeadFlags, HeadTable, IndexToLocFormat, MacStyleFlags};
pub use horizontal_header_table::HorizontalHeaderTable;
pub use invalid_font_table_exception::InvalidFontTableException;
pub use known_name_ids::KnownNameIds;
pub use loca_table::LocaTable;
pub use maxp_table::MaxpTable;
pub use meta_table::MetaTable;
pub use missing_font_table_exception::MissingFontTableException;
pub use os2_table::{FontSelectionFlags, OS2Table};
pub use panose::{
    Panose, PanoseArmStyle, PanoseContrast, PanoseFamilyKind, PanoseLetterform, PanoseMidline, PanoseProportion,
    PanoseSerifStyle, PanoseStrokeVariation, PanoseWeight, PanoseXHeight,
};
pub use platform_id::PlatformID;
pub use post_table::PostTable;
pub use script_list_table::ScriptListTable;
pub use vertical_header_table::VerticalHeaderTable;
