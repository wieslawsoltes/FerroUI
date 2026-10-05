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

use super::unicode_trie::UnicodeTrie;
use super::unicode_trie_builder_constants::*;

/// Builds a [`UnicodeTrie`] from individual code point values and ranges.
///
/// Indices are kept as `i32` like upstream: the block maps use negative values
/// as free-list links and gap markers.
pub(crate) struct UnicodeTrieBuilder {
    initial_value: u32,
    error_value: u32,
    index1: Vec<i32>,
    index2: Vec<i32>,
    high_start: i32,
    data: Vec<u32>,
    data_capacity: i32,
    first_free_block: i32,
    is_compacted: bool,
    map: Vec<i32>,
    data_null_offset: i32,
    data_length: i32,
    index2_null_offset: i32,
    index2_length: i32,
    /// The serialized trie produced by [`UnicodeTrieBuilder::freeze`]; the
    /// returned trie borrows it.
    frozen: Vec<u32>,
}

#[inline(always)]
fn ix(value: i32) -> usize {
    value as usize
}

// Only the generator of the data tables and the tests build tries.
#[allow(dead_code)]
impl UnicodeTrieBuilder {
    pub(crate) fn new(initial_value: u32, error_value: u32) -> Self {
        let mut this = Self {
            initial_value,
            error_value,
            index1: vec![0; ix(INDEX_1_LENGTH)],
            index2: vec![0; ix(MAX_INDEX_2_LENGTH)],
            high_start: 0x110000,
            data: vec![0; ix(INITIAL_DATA_LENGTH)],
            data_capacity: INITIAL_DATA_LENGTH,
            first_free_block: 0,
            is_compacted: false,
            // Multi-purpose per-data-block table.
            //
            // Before compacting:
            //
            // Per-data-block reference counters/free-block list.
            //  0: unused
            // >0: reference counter (number of index-2 entries pointing here)
            // <0: next free data block in free-block list
            //
            // While compacting:
            //
            // Map of adjusted indexes, used in compactData() and compactIndex2().
            // Maps from original indexes to new ones.
            map: vec![0; ix(MAX_DATA_LENGTH_BUILDTIME >> SHIFT_2)],
            data_null_offset: 0,
            data_length: 0,
            index2_null_offset: 0,
            index2_length: 0,
            frozen: Vec::new(),
        };

        let mut i: i32 = 0;
        while i < 0x80 {
            this.data[ix(i)] = initial_value;
            i += 1;
        }

        while i < 0xc0 {
            this.data[ix(i)] = error_value;
            i += 1;
        }

        i = DATA_NULL_OFFSET;
        while i < NEW_DATA_START_OFFSET {
            this.data[ix(i)] = initial_value;
            i += 1;
        }

        this.data_null_offset = DATA_NULL_OFFSET;
        this.data_length = NEW_DATA_START_OFFSET;

        // set the index-2 indexes for the 2=0x80>>SHIFT_2 ASCII data blocks
        let mut j: i32 = 0;
        i = 0;
        while j < 0x80 {
            this.index2[ix(i)] = j;
            this.map[ix(i)] = 1;
            i += 1;
            j += DATA_BLOCK_LENGTH;
        }

        // reference counts for the bad-UTF-8-data block
        while j < 0xc0 {
            this.map[ix(i)] = 0;
            i += 1;
            j += DATA_BLOCK_LENGTH;
        }

        // Reference counts for the null data block: all blocks except for the ASCII blocks.
        // Plus 1 so that we don't drop this block during compaction.
        // Plus as many as needed for lead surrogate code points.
        // i==newTrie->dataNullOffset
        this.map[ix(i)] = ((0x110000 >> SHIFT_2) - (0x80 >> SHIFT_2)) + 1 + LSCP_INDEX_2_LENGTH;
        i += 1;
        j += DATA_BLOCK_LENGTH;
        while j < NEW_DATA_START_OFFSET {
            this.map[ix(i)] = 0;
            i += 1;
            j += DATA_BLOCK_LENGTH;
        }

        // set the remaining indexes in the BMP index-2 block
        // to the null data block
        i = 0x80 >> SHIFT_2;
        while i < INDEX_2_BMP_LENGTH {
            this.index2[ix(i)] = DATA_NULL_OFFSET;
            i += 1;
        }

        // Fill the index gap with impossible values so that compaction
        // does not overlap other index-2 blocks with the gap.
        i = 0;
        while i < INDEX_GAP_LENGTH {
            this.index2[ix(INDEX_GAP_OFFSET + i)] = -1;
            i += 1;
        }

        // set the indexes in the null index-2 block
        i = 0;
        while i < INDEX_2_BLOCK_LENGTH {
            this.index2[ix(INDEX_2_NULL_OFFSET + i)] = DATA_NULL_OFFSET;
            i += 1;
        }

        this.index2_null_offset = INDEX_2_NULL_OFFSET;
        this.index2_length = INDEX_2_START_OFFSET;

        // set the index-1 indexes for the linear index-2 block
        j = 0;
        i = 0;
        while i < OMITTED_BMP_INDEX_1_LENGTH {
            this.index1[ix(i)] = j;
            j += INDEX_2_BLOCK_LENGTH;
            i += 1;
        }

        // set the remaining index-1 indexes to the null index-2 block
        while i < INDEX_1_LENGTH {
            this.index1[ix(i)] = INDEX_2_NULL_OFFSET;
            i += 1;
        }

        // Preallocate and reset data for U+0080..U+07ff,
        // for 2-byte UTF-8 which will be compacted in 64-blocks
        // even if DATA_BLOCK_LENGTH is smaller.
        i = 0x80;
        while i < 0x800 {
            this.set(i, initial_value);
            i += DATA_BLOCK_LENGTH;
        }

        this
    }

    /// Panics when the code point is invalid or the trie is already compacted.
    pub(crate) fn set(&mut self, code_point: i32, value: u32) -> &mut Self {
        if !(0..=0x10ffff).contains(&code_point) {
            panic!("Invalid code point");
        }

        if self.is_compacted {
            panic!("Already compacted");
        }

        let block = self.get_data_block(code_point, true);
        self.data[ix(block + (code_point & DATA_MASK))] = value;
        self
    }

    /// `SetRange(start, end, value)` with the default `overwrite: true`.
    pub(crate) fn set_range(&mut self, start: i32, end: i32, value: u32) -> &mut Self {
        self.set_range_with_overwrite(start, end, value, true)
    }

    /// Panics when the range is invalid or the trie is already compacted.
    pub(crate) fn set_range_with_overwrite(&mut self, start: i32, end: i32, value: u32, overwrite: bool) -> &mut Self {
        let mut start = start;

        if (start > 0x10ffff) || (end > 0x10ffff) || (start > end) {
            panic!("Invalid code point");
        }

        if self.is_compacted {
            panic!("Already compacted");
        }

        if !overwrite && (value == self.initial_value) {
            return self; // nothing to do
        }

        let mut limit = end + 1;
        if (start & DATA_MASK) != 0 {
            // set partial block at [start..following block boundary
            let block = self.get_data_block(start, true);

            let next_start = (start + DATA_BLOCK_LENGTH) & !DATA_MASK;
            if next_start <= limit {
                self.fill_block(block, start & DATA_MASK, DATA_BLOCK_LENGTH, value, self.initial_value, overwrite);
                start = next_start;
            } else {
                self.fill_block(block, start & DATA_MASK, limit & DATA_MASK, value, self.initial_value, overwrite);
                return self;
            }
        }

        // number of positions in the last, partial block
        let rest = limit & DATA_MASK;

        // round down limit to a block boundary
        limit &= !DATA_MASK;

        // iterate over all-value blocks
        let mut repeat_block: i32 = if value == self.initial_value { self.data_null_offset } else { -1 };

        while start < limit {
            let mut set_repeat_block = false;

            if (value == self.initial_value) && self.is_in_null_block(start, true) {
                start += DATA_BLOCK_LENGTH; // nothing to do
                continue;
            }

            // get index value
            let mut i2 = self.get_index2_block(start, true);
            i2 += (start >> SHIFT_2) & INDEX_2_MASK;

            let block = self.index2[ix(i2)];
            if self.is_writable_block(block) {
                // already allocated
                if overwrite && (block >= DATA_0800_OFFSET) {
                    // We overwrite all values, and it's not a
                    // protected (ASCII-linear or 2-byte UTF-8) block:
                    // replace with the repeatBlock.
                    set_repeat_block = true;
                } else {
                    // protected block: just write the values into this block
                    self.fill_block(block, 0, DATA_BLOCK_LENGTH, value, self.initial_value, overwrite);
                }
            } else if (self.data[ix(block)] != value) && (overwrite || (block == self.data_null_offset)) {
                // Set the repeatBlock instead of the null block or previous repeat block:
                //
                // If !isWritableBlock() then all entries in the block have the same value
                // because it's the null block or a range block (the repeatBlock from a previous
                // call to utrie2_setRange32()).
                // No other blocks are used multiple times before compacting.
                //
                // The null block is the only non-writable block with the initialValue because
                // of the repeatBlock initialization above. (If value==initialValue, then
                // the repeatBlock will be the null data block.)
                //
                // We set our repeatBlock if the desired value differs from the block's value,
                // and if we overwrite any data or if the data is all initial values
                // (which is the same as the block being the null block, see above).
                set_repeat_block = true;
            }

            if set_repeat_block {
                if repeat_block >= 0 {
                    self.set_index2_entry(i2, repeat_block);
                } else {
                    // create and set and fill the repeatBlock
                    repeat_block = self.get_data_block(start, true);
                    self.write_block(repeat_block, value);
                }
            }

            start += DATA_BLOCK_LENGTH;
        }

        if rest > 0 {
            // set partial block at [last block boundary..limit
            let block = self.get_data_block(start, true);
            self.fill_block(block, 0, rest, value, self.initial_value, overwrite);
        }

        self
    }

    /// `Get(c)` with the default `fromLSCP: true`.
    pub(crate) fn get(&self, c: i32) -> u32 {
        self.get_with(c, true)
    }

    pub(crate) fn get_with(&self, c: i32, from_lscp: bool) -> u32 {
        if !(0..=0x10ffff).contains(&c) {
            return self.error_value;
        }

        if (c >= self.high_start) && (!((0xd800..0xdc00).contains(&c)) || from_lscp) {
            return self.data[ix(self.data_length - DATA_GRANULARITY)];
        }

        let i2 = if (0xd800..0xdc00).contains(&c) && from_lscp {
            (LSCP_INDEX_2_OFFSET - (0xd800 >> SHIFT_2)) + (c >> SHIFT_2)
        } else {
            self.index1[ix(c >> SHIFT_1)] + ((c >> SHIFT_2) & INDEX_2_MASK)
        };

        let block = self.index2[ix(i2)];
        self.data[ix(block + (c & DATA_MASK))]
    }

    /// Compacts the trie and serializes it. The returned trie borrows the
    /// serialized data, which the builder keeps.
    ///
    /// Panics when the trie data is too large.
    pub(crate) fn freeze(&mut self) -> UnicodeTrie<'_> {
        if !self.is_compacted {
            self.compact();
        }

        let all_indexes_length = if self.high_start <= 0x10000 { INDEX_1_OFFSET } else { self.index2_length };

        let data_move = all_indexes_length;

        // are indexLength and dataLength within limits?
        if (all_indexes_length > MAX_INDEX_LENGTH) || // for unshifted indexLength
            ((data_move + self.data_null_offset) > 0xffff) || // for unshifted dataNullOffset
            ((data_move + DATA_0800_OFFSET) > 0xffff) || // for unshifted 2-byte UTF-8 index-2 values
            ((data_move + self.data_length) > MAX_DATA_LENGTH_RUNTIME)
        {
            // for shiftedDataLength
            panic!("Trie data is too large.");
        }

        // calculate the sizes of, and allocate, the index and data arrays
        let index_length = all_indexes_length + self.data_length;
        let mut data: Vec<u32> = Vec::with_capacity(ix(index_length));

        // write the index-2 array values shifted right by INDEX_SHIFT, after adding dataMove
        let mut i: i32 = 0;
        while i < INDEX_2_BMP_LENGTH {
            data.push(((self.index2[ix(i)] + data_move) >> INDEX_SHIFT) as u32);
            i += 1;
        }

        // write UTF-8 2-byte index-2 values, not right-shifted
        i = 0;
        while i < 0xc2 - 0xc0 {
            // C0..C1
            data.push((data_move + BAD_UTF8_DATA_OFFSET) as u32);
            i += 1;
        }

        while i < 0xe0 - 0xc0 {
            // C2..DF
            data.push((data_move + self.index2[ix(i << (6 - SHIFT_2))]) as u32);
            i += 1;
        }

        if self.high_start > 0x10000 {
            let index1_length = (self.high_start - 0x10000) >> SHIFT_1;
            let index2_offset = INDEX_2_BMP_LENGTH + UTF8_2B_INDEX_2_LENGTH + index1_length;

            // write 16-bit index-1 values for supplementary code points
            i = 0;
            while i < index1_length {
                data.push((INDEX_2_OFFSET + self.index1[ix(i + OMITTED_BMP_INDEX_1_LENGTH)]) as u32);
                i += 1;
            }

            // write the index-2 array values for supplementary code points,
            // shifted right by INDEX_SHIFT, after adding dataMove
            i = 0;
            while i < self.index2_length - index2_offset {
                data.push(((data_move + self.index2[ix(index2_offset + i)]) >> INDEX_SHIFT) as u32);
                i += 1;
            }
        }

        // write 16-bit data values
        data.extend_from_slice(&self.data[..ix(self.data_length)]);

        // Upstream allocates exactly `index_length` entries.
        data.resize(ix(index_length), 0);

        self.frozen = data;
        UnicodeTrie::new(&self.frozen, self.high_start, self.error_value)
    }

    fn is_in_null_block(&self, c: i32, for_lscp: bool) -> bool {
        let i2 = if ((c as u32 & 0xfffffc00) == 0xd800) && for_lscp {
            (LSCP_INDEX_2_OFFSET - (0xd800 >> SHIFT_2)) + (c >> SHIFT_2)
        } else {
            self.index1[ix(c >> SHIFT_1)] + ((c >> SHIFT_2) & INDEX_2_MASK)
        };

        let block = self.index2[ix(i2)];
        block == self.data_null_offset
    }

    fn alloc_index2_block(&mut self) -> i32 {
        let new_block = self.index2_length;
        let new_top = new_block + INDEX_2_BLOCK_LENGTH;
        if ix(new_top) > self.index2.len() {
            // Should never occur.
            // Either MAX_BUILD_TIME_INDEX_LENGTH is incorrect,
            // or the code writes more values than should be possible.
            panic!("Internal error in Trie2 creation.");
        }

        self.index2_length = new_top;
        let source = ix(self.index2_null_offset);
        self.index2.copy_within(source..source + ix(INDEX_2_BLOCK_LENGTH), ix(new_block));

        new_block
    }

    fn get_index2_block(&mut self, c: i32, for_lscp: bool) -> i32 {
        if (0xd800..0xdc00).contains(&c) && for_lscp {
            return LSCP_INDEX_2_OFFSET;
        }

        let i1 = c >> SHIFT_1;
        let mut i2 = self.index1[ix(i1)];
        if i2 == self.index2_null_offset {
            i2 = self.alloc_index2_block();
            self.index1[ix(i1)] = i2;
        }

        i2
    }

    fn is_writable_block(&self, block: i32) -> bool {
        (block != self.data_null_offset) && (self.map[ix(block >> SHIFT_2)] == 1)
    }

    fn alloc_data_block(&mut self, copy_block: i32) -> i32 {
        let new_block;
        if self.first_free_block != 0 {
            // get the first free block
            new_block = self.first_free_block;
            self.first_free_block = -self.map[ix(new_block >> SHIFT_2)];
        } else {
            // get a new block from the high end
            new_block = self.data_length;
            let new_top = new_block + DATA_BLOCK_LENGTH;
            if new_top > self.data_capacity {
                // out of memory in the data array
                let capacity = if self.data_capacity < MEDIUM_DATA_LENGTH {
                    MEDIUM_DATA_LENGTH
                } else if self.data_capacity < MAX_DATA_LENGTH_BUILDTIME {
                    MAX_DATA_LENGTH_BUILDTIME
                } else {
                    // Should never occur.
                    // Either MAX_DATA_LENGTH_BUILDTIME is incorrect,
                    // or the code writes more values than should be possible.
                    panic!("Internal error in Trie2 creation.");
                };

                self.data.resize(ix(capacity), 0);
                self.data_capacity = capacity;
            }

            self.data_length = new_top;
        }

        let source = ix(copy_block);
        self.data.copy_within(source..source + ix(DATA_BLOCK_LENGTH), ix(new_block));
        self.map[ix(new_block >> SHIFT_2)] = 0;
        new_block
    }

    fn release_data_block(&mut self, block: i32) {
        // put this block at the front of the free-block chain
        self.map[ix(block >> SHIFT_2)] = -self.first_free_block;
        self.first_free_block = block;
    }

    fn set_index2_entry(&mut self, i2: i32, block: i32) {
        self.map[ix(block >> SHIFT_2)] += 1; // increment first, in case block == oldBlock!
        let old_block = self.index2[ix(i2)];
        self.map[ix(old_block >> SHIFT_2)] -= 1;
        if self.map[ix(old_block >> SHIFT_2)] == 0 {
            self.release_data_block(old_block);
        }

        self.index2[ix(i2)] = block;
    }

    fn get_data_block(&mut self, c: i32, for_lscp: bool) -> i32 {
        let mut i2 = self.get_index2_block(c, for_lscp);
        i2 += (c >> SHIFT_2) & INDEX_2_MASK;

        let old_block = self.index2[ix(i2)];
        if self.is_writable_block(old_block) {
            return old_block;
        }

        // allocate a new data block
        let new_block = self.alloc_data_block(old_block);
        self.set_index2_entry(i2, new_block);
        new_block
    }

    fn fill_block(&mut self, block: i32, start: i32, limit: i32, value: u32, initial_value: u32, overwrite: bool) {
        let range = ix(block + start)..ix(block + limit);
        if overwrite {
            for entry in &mut self.data[range] {
                *entry = value;
            }
        } else {
            for entry in &mut self.data[range] {
                if *entry == initial_value {
                    *entry = value;
                }
            }
        }
    }

    fn write_block(&mut self, block: i32, value: u32) {
        let limit = block + DATA_BLOCK_LENGTH;
        for entry in &mut self.data[ix(block)..ix(limit)] {
            *entry = value;
        }
    }

    fn find_high_start(&self, high_value: u32) -> i32 {
        let mut prev_block;
        let mut prev_i2_block;

        // set variables for previous range
        if high_value == self.initial_value {
            prev_i2_block = self.index2_null_offset;
            prev_block = self.data_null_offset;
        } else {
            prev_i2_block = -1;
            prev_block = -1;
        }

        let prev: i32 = 0x110000;

        // enumerate index-2 blocks
        let mut i1 = INDEX_1_LENGTH;
        let mut c = prev;
        while c > 0 {
            i1 -= 1;
            let i2_block = self.index1[ix(i1)];
            if i2_block == prev_i2_block {
                // the index-2 block is the same as the previous one, and filled with highValue
                c -= CP_PER_INDEX_1_ENTRY;
                continue;
            }

            prev_i2_block = i2_block;
            if i2_block == self.index2_null_offset {
                // this is the null index-2 block
                if high_value != self.initial_value {
                    return c;
                }
                c -= CP_PER_INDEX_1_ENTRY;
            } else {
                // enumerate data blocks for one index-2 block
                let mut i2 = INDEX_2_BLOCK_LENGTH;
                while i2 > 0 {
                    i2 -= 1;
                    let block = self.index2[ix(i2_block + i2)];
                    if block == prev_block {
                        // the block is the same as the previous one, and filled with highValue
                        c -= DATA_BLOCK_LENGTH;
                        continue;
                    }

                    prev_block = block;
                    if block == self.data_null_offset {
                        // this is the null data block
                        if high_value != self.initial_value {
                            return c;
                        }
                        c -= DATA_BLOCK_LENGTH;
                    } else {
                        let mut j = DATA_BLOCK_LENGTH;
                        while j > 0 {
                            j -= 1;
                            let value = self.data[ix(block + j)];
                            if value != high_value {
                                return c;
                            }
                            c -= 1;
                        }
                    }
                }
            }
        }

        // deliver last range
        0
    }

    fn find_same_data_block(&self, data_length: i32, other_block: i32, block_length: i32) -> i32 {
        // ensure that we do not even partially get past dataLength
        let data_length = data_length - block_length;
        let mut block = 0;
        while block <= data_length {
            if equal_sequence(&self.data, block, other_block, block_length) {
                return block;
            }
            block += DATA_GRANULARITY;
        }

        -1
    }

    fn find_same_index2_block(&self, index2_length: i32, other_block: i32) -> i32 {
        // ensure that we do not even partially get past index2Length
        let index2_length = index2_length - INDEX_2_BLOCK_LENGTH;
        let mut block = 0;
        while block <= index2_length {
            if equal_sequence(&self.index2, block, other_block, INDEX_2_BLOCK_LENGTH) {
                return block;
            }
            block += 1;
        }

        -1
    }

    fn compact_data(&mut self) {
        // do not compact linear-ASCII data
        let mut new_start = DATA_START_OFFSET;
        let mut start = 0;
        let mut i = 0;

        while start < new_start {
            self.map[ix(i)] = start;
            i += 1;
            start += DATA_BLOCK_LENGTH;
        }

        // Start with a block length of 64 for 2-byte UTF-8,
        // then switch to DATA_BLOCK_LENGTH.
        let mut block_length = 64;
        let mut block_count = block_length >> SHIFT_2;
        start = new_start;
        while start < self.data_length {
            // start: index of first entry of current block
            // newStart: index where the current block is to be moved
            //           (right after current end of already-compacted data)
            let mut map_index;
            let mut moved_start;
            if start == DATA_0800_OFFSET {
                block_length = DATA_BLOCK_LENGTH;
                block_count = 1;
            }

            // skip blocks that are not used
            if self.map[ix(start >> SHIFT_2)] <= 0 {
                // advance start to the next block
                start += block_length;

                // leave newStart with the previous block!
                continue;
            }

            // search for an identical block
            moved_start = self.find_same_data_block(new_start, start, block_length);
            if moved_start >= 0 {
                // found an identical block, set the other block's index value for the current block
                map_index = start >> SHIFT_2;
                i = block_count;
                while i > 0 {
                    self.map[ix(map_index)] = moved_start;
                    map_index += 1;
                    moved_start += DATA_BLOCK_LENGTH;
                    i -= 1;
                }

                // advance start to the next block
                start += block_length;

                // leave newStart with the previous block!
                continue;
            }

            // see if the beginning of this block can be overlapped with the end of the previous block
            // look for maximum overlap (modulo granularity) with the previous, adjacent block
            let mut overlap = block_length - DATA_GRANULARITY;
            while (overlap > 0) && !equal_sequence(&self.data, new_start - overlap, start, overlap) {
                overlap -= DATA_GRANULARITY;
            }

            if (overlap > 0) || (new_start < start) {
                // some overlap, or just move the whole block
                moved_start = new_start - overlap;
                map_index = start >> SHIFT_2;

                i = block_count;
                while i > 0 {
                    self.map[ix(map_index)] = moved_start;
                    map_index += 1;
                    moved_start += DATA_BLOCK_LENGTH;
                    i -= 1;
                }

                // move the non-overlapping indexes to their new positions
                start += overlap;
                i = block_length - overlap;
                while i > 0 {
                    self.data[ix(new_start)] = self.data[ix(start)];
                    new_start += 1;
                    start += 1;
                    i -= 1;
                }
            } else {
                // no overlap && newStart==start
                map_index = start >> SHIFT_2;
                i = block_count;
                while i > 0 {
                    self.map[ix(map_index)] = start;
                    map_index += 1;
                    start += DATA_BLOCK_LENGTH;
                    i -= 1;
                }

                new_start = start;
            }
        }

        // now adjust the index-2 table
        i = 0;
        while i < self.index2_length {
            // Gap indexes are invalid (-1). Skip over the gap.
            if i == INDEX_GAP_OFFSET {
                i += INDEX_GAP_LENGTH;
            }
            self.index2[ix(i)] = self.map[ix(self.index2[ix(i)] >> SHIFT_2)];
            i += 1;
        }

        self.data_null_offset = self.map[ix(self.data_null_offset >> SHIFT_2)];

        // ensure dataLength alignment
        while (new_start & (DATA_GRANULARITY - 1)) != 0 {
            self.data[ix(new_start)] = self.initial_value;
            new_start += 1;
        }
        self.data_length = new_start;
    }

    fn compact_index2(&mut self) {
        // do not compact linear-BMP index-2 blocks
        let mut new_start = INDEX_2_BMP_LENGTH;
        let mut start = 0;
        let mut i = 0;

        while start < new_start {
            self.map[ix(i)] = start;
            i += 1;
            start += INDEX_2_BLOCK_LENGTH;
        }

        // Reduce the index table gap to what will be needed at runtime.
        new_start += UTF8_2B_INDEX_2_LENGTH + ((self.high_start - 0x10000) >> SHIFT_1);
        start = INDEX_2_NULL_OFFSET;
        while start < self.index2_length {
            // start: index of first entry of current block
            // newStart: index where the current block is to be moved
            //           (right after current end of already-compacted data)

            // search for an identical block
            let moved_start = self.find_same_index2_block(new_start, start);
            if moved_start >= 0 {
                // found an identical block, set the other block's index value for the current block
                self.map[ix(start >> SHIFT_1_2)] = moved_start;

                // advance start to the next block
                start += INDEX_2_BLOCK_LENGTH;

                // leave newStart with the previous block!
                continue;
            }

            // see if the beginning of this block can be overlapped with the end of the previous block
            // look for maximum overlap with the previous, adjacent block
            let mut overlap = INDEX_2_BLOCK_LENGTH - 1;
            while (overlap > 0) && !equal_sequence(&self.index2, new_start - overlap, start, overlap) {
                overlap -= 1;
            }

            if (overlap > 0) || (new_start < start) {
                // some overlap, or just move the whole block
                self.map[ix(start >> SHIFT_1_2)] = new_start - overlap;

                // move the non-overlapping indexes to their new positions
                start += overlap;
                i = INDEX_2_BLOCK_LENGTH - overlap;
                while i > 0 {
                    self.index2[ix(new_start)] = self.index2[ix(start)];
                    new_start += 1;
                    start += 1;
                    i -= 1;
                }
            } else {
                // no overlap && newStart==start
                self.map[ix(start >> SHIFT_1_2)] = start;
                start += INDEX_2_BLOCK_LENGTH;
                new_start = start;
            }
        }

        // now adjust the index-1 table
        i = 0;
        while i < INDEX_1_LENGTH {
            self.index1[ix(i)] = self.map[ix(self.index1[ix(i)] >> SHIFT_1_2)];
            i += 1;
        }

        self.index2_null_offset = self.map[ix(self.index2_null_offset >> SHIFT_1_2)];

        // Ensure data table alignment:
        // Needs to be granularity-aligned for 16-bit trie
        // (so that dataMove will be down-shiftable),
        // and 2-aligned for uint32_t data.

        // Arbitrary value: 0x3fffc not possible for real data.
        while (new_start & ((DATA_GRANULARITY - 1) | 1)) != 0 {
            self.index2[ix(new_start)] = 0x0000ffff << INDEX_SHIFT;
            new_start += 1;
        }

        self.index2_length = new_start;
    }

    fn compact(&mut self) {
        // find highStart and round it up
        let mut high_value = self.get(0x10ffff);
        let mut high_start = self.find_high_start(high_value);
        high_start = (high_start + (CP_PER_INDEX_1_ENTRY - 1)) & !(CP_PER_INDEX_1_ENTRY - 1);
        if high_start == 0x110000 {
            high_value = self.error_value;
        }

        // Set trie->highStart only after utrie2_get32(trie, highStart).
        // Otherwise utrie2_get32(trie, highStart) would try to read the highValue.
        self.high_start = high_start;
        if self.high_start < 0x110000 {
            // Blank out [highStart..10ffff] to release associated data blocks.
            let supp_high_start = if self.high_start <= 0x10000 { 0x10000 } else { self.high_start };
            self.set_range(supp_high_start, 0x10ffff, self.initial_value);
        }

        self.compact_data();

        if self.high_start > 0x10000 {
            self.compact_index2();
        }

        // Store the highValue in the data array and round up the dataLength.
        // Must be done after compactData() because that assumes that dataLength
        // is a multiple of DATA_BLOCK_LENGTH.
        self.data[ix(self.data_length)] = high_value;
        self.data_length += 1;
        while (self.data_length & (DATA_GRANULARITY - 1)) != 0 {
            self.data[ix(self.data_length)] = self.initial_value;
            self.data_length += 1;
        }

        self.is_compacted = true;
    }
}

fn equal_sequence<T: PartialEq>(a: &[T], s: i32, t: i32, length: i32) -> bool {
    let (s, t, length) = (ix(s), ix(t), ix(length));
    a[s..s + length] == a[t..t + length]
}
