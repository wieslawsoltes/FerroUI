// RichTextKit
// Copyright © 2019 Topten Software. All Rights Reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may
// not use this product except in compliance with the License. You may obtain
// a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations
// under the License.
// Ported from: https://github.com/foliojs/unicode-trie
// Copied from: https://github.com/toptensoftware/RichTextKit

//! The layout constants of the trie (the constants half of the builder upstream).

// Shift size for getting the index-1 table offset.
pub(crate) const SHIFT_1: i32 = 6 + 5;

// Shift size for getting the index-2 table offset.
pub(crate) const SHIFT_2: i32 = 5;

// Difference between the two shift sizes,
// for getting an index-1 offset from an index-2 offset. 6=11-5
pub(super) const SHIFT_1_2: i32 = SHIFT_1 - SHIFT_2;

// Number of index-1 entries for the BMP. 32=0x20
// This part of the index-1 table is omitted from the serialized form.
pub(crate) const OMITTED_BMP_INDEX_1_LENGTH: i32 = 0x10000 >> SHIFT_1;

// Number of code points per index-1 table entry. 2048=0x800
pub(super) const CP_PER_INDEX_1_ENTRY: i32 = 1 << SHIFT_1;

// Number of entries in an index-2 block. 64=0x40
pub(super) const INDEX_2_BLOCK_LENGTH: i32 = 1 << SHIFT_1_2;

// Mask for getting the lower bits for the in-index-2-block offset. */
pub(crate) const INDEX_2_MASK: i32 = INDEX_2_BLOCK_LENGTH - 1;

// Number of entries in a data block. 32=0x20
pub(super) const DATA_BLOCK_LENGTH: i32 = 1 << SHIFT_2;

// Mask for getting the lower bits for the in-data-block offset.
pub(crate) const DATA_MASK: i32 = DATA_BLOCK_LENGTH - 1;

// Shift size for shifting left the index array values.
// Increases possible data size with 16-bit index values at the cost
// of compactability.
// This requires data blocks to be aligned by DATA_GRANULARITY.
pub(crate) const INDEX_SHIFT: i32 = 2;

// The alignment size of a data block. Also the granularity for compaction.
pub(crate) const DATA_GRANULARITY: i32 = 1 << INDEX_SHIFT;

// The BMP part of the index-2 table is fixed and linear and starts at offset 0.
// Length=2048=0x800=0x10000>>SHIFT_2.
pub(super) const INDEX_2_OFFSET: i32 = 0;

// The part of the index-2 table for U+D800..U+DBFF stores values for
// lead surrogate code _units_ not code _points_.
// Values for lead surrogate code _points_ are indexed with this portion of the table.
// Length=32=0x20=0x400>>SHIFT_2. (There are 1024=0x400 lead surrogates.)
pub(crate) const LSCP_INDEX_2_OFFSET: i32 = 0x10000 >> SHIFT_2;
pub(super) const LSCP_INDEX_2_LENGTH: i32 = 0x400 >> SHIFT_2;

// Count the lengths of both BMP pieces. 2080=0x820
pub(super) const INDEX_2_BMP_LENGTH: i32 = LSCP_INDEX_2_OFFSET + LSCP_INDEX_2_LENGTH;

// The 2-byte UTF-8 version of the index-2 table follows at offset 2080=0x820.
// Length 32=0x20 for lead bytes C0..DF, regardless of SHIFT_2.
pub(super) const UTF8_2B_INDEX_2_OFFSET: i32 = INDEX_2_BMP_LENGTH;
pub(super) const UTF8_2B_INDEX_2_LENGTH: i32 = 0x800 >> 6; // U+0800 is the first code point after 2-byte UTF-8

// The index-1 table, only used for supplementary code points, at offset 2112=0x840.
// Variable length, for code points up to highStart, where the last single-value range starts.
// Maximum length 512=0x200=0x100000>>SHIFT_1.
// (For 0x100000 supplementary code points U+10000..U+10ffff.)
//
// The part of the index-2 table for supplementary code points starts
// after this index-1 table.
//
// Both the index-1 table and the following part of the index-2 table
// are omitted completely if there is only BMP data.
pub(crate) const INDEX_1_OFFSET: i32 = UTF8_2B_INDEX_2_OFFSET + UTF8_2B_INDEX_2_LENGTH;
pub(super) const MAX_INDEX_1_LENGTH: i32 = 0x100000 >> SHIFT_1;

// The illegal-UTF-8 data block follows the ASCII block, at offset 128=0x80.
// Used with linear access for single bytes 0..0xbf for simple error handling.
// Length 64=0x40, not DATA_BLOCK_LENGTH.
pub(super) const BAD_UTF8_DATA_OFFSET: i32 = 0x80;

// The start of non-linear-ASCII data blocks, at offset 192=0xc0.
// !!!!
pub(super) const DATA_START_OFFSET: i32 = 0xc0;

// The null data block.
// Length 64=0x40 even if DATA_BLOCK_LENGTH is smaller,
// to work with 6-bit trail bytes from 2-byte UTF-8.
pub(super) const DATA_NULL_OFFSET: i32 = DATA_START_OFFSET;

// The start of allocated data blocks.
pub(super) const NEW_DATA_START_OFFSET: i32 = DATA_NULL_OFFSET + 0x40;

// The start of data blocks for U+0800 and above.
// Below, compaction uses a block length of 64 for 2-byte UTF-8.
// From here on, compaction uses DATA_BLOCK_LENGTH.
// Data values for 0x780 code points beyond ASCII.
pub(super) const DATA_0800_OFFSET: i32 = NEW_DATA_START_OFFSET + 0x780;

// Start with allocation of 16k data entries. */
pub(super) const INITIAL_DATA_LENGTH: i32 = 1 << 14;

// Grow about 8x each time.
pub(super) const MEDIUM_DATA_LENGTH: i32 = 1 << 17;

// Maximum length of the runtime data array.
// Limited by 16-bit index values that are left-shifted by INDEX_SHIFT,
// and by uint16_t UTrie2Header.shiftedDataLength.
pub(super) const MAX_DATA_LENGTH_RUNTIME: i32 = 0xffff << INDEX_SHIFT;

pub(super) const INDEX_1_LENGTH: i32 = 0x110000 >> SHIFT_1;

// Maximum length of the build-time data array.
// One entry per 0x110000 code points, plus the illegal-UTF-8 block and the null block,
// plus values for the 0x400 surrogate code units.
pub(super) const MAX_DATA_LENGTH_BUILDTIME: i32 = 0x110000 + 0x40 + 0x40 + 0x400;

// At build time, leave a gap in the index-2 table,
// at least as long as the maximum lengths of the 2-byte UTF-8 index-2 table
// and the supplementary index-1 table.
// Round up to INDEX_2_BLOCK_LENGTH for proper compacting.
pub(super) const INDEX_GAP_OFFSET: i32 = INDEX_2_BMP_LENGTH;
pub(super) const INDEX_GAP_LENGTH: i32 =
    ((UTF8_2B_INDEX_2_LENGTH + MAX_INDEX_1_LENGTH) + INDEX_2_MASK) & !INDEX_2_MASK;

// Maximum length of the build-time index-2 array.
// Maximum number of Unicode code points (0x110000) shifted right by SHIFT_2,
// plus the part of the index-2 table for lead surrogate code points,
// plus the build-time index gap,
// plus the null index-2 block.)
pub(super) const MAX_INDEX_2_LENGTH: i32 =
    (0x110000 >> SHIFT_2) + LSCP_INDEX_2_LENGTH + INDEX_GAP_LENGTH + INDEX_2_BLOCK_LENGTH;

// The null index-2 block, following the gap in the index-2 table.
pub(super) const INDEX_2_NULL_OFFSET: i32 = INDEX_GAP_OFFSET + INDEX_GAP_LENGTH;

// The start of allocated index-2 blocks.
pub(super) const INDEX_2_START_OFFSET: i32 = INDEX_2_NULL_OFFSET + INDEX_2_BLOCK_LENGTH;

// Maximum length of the runtime index array.
// Limited by its own 16-bit index values, and by uint16_t UTrie2Header.indexLength.
// (The actual maximum length is lower,
// (0x110000>>SHIFT_2)+UTF8_2B_INDEX_2_LENGTH+MAX_INDEX_1_LENGTH.)
pub(super) const MAX_INDEX_LENGTH: i32 = 0xffff;
