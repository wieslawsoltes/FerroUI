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

use super::unicode_trie_builder_constants::{
    DATA_GRANULARITY, DATA_MASK, INDEX_1_OFFSET, INDEX_2_MASK, INDEX_SHIFT, LSCP_INDEX_2_OFFSET,
    OMITTED_BMP_INDEX_1_LENGTH, SHIFT_1, SHIFT_2,
};

/// A read only view over a serialized two stage code point trie.
#[derive(Clone, Copy, Debug)]
pub(crate) struct UnicodeTrie<'a> {
    data: &'a [u32],
    high_start: i32,
    error_value: u32,
}

impl<'a> UnicodeTrie<'a> {
    #[inline(always)]
    pub(crate) const fn new(data: &'a [u32], high_start: i32, error_value: u32) -> Self {
        Self { data, high_start, error_value }
    }

    #[allow(dead_code)] // upstream member, only the lookups are used so far
    #[inline]
    pub(crate) const fn data(&self) -> &'a [u32] {
        self.data
    }

    #[allow(dead_code)] // upstream member, only the lookups are used so far
    #[inline]
    pub(crate) const fn high_start(&self) -> i32 {
        self.high_start
    }

    #[allow(dead_code)] // upstream member, only the lookups are used so far
    #[inline]
    pub(crate) const fn error_value(&self) -> u32 {
        self.error_value
    }

    /// Get the value for a code point as stored in the trie.
    #[inline(always)]
    pub(crate) fn get(&self, code_point: u32) -> u32 {
        let data = self.data;
        let mut index: u32;

        if code_point < 0x0d800 || (code_point > 0x0dbff && code_point <= 0x0ffff) {
            // Ordinary BMP code point, excluding leading surrogates.
            // BMP uses a single level lookup.  BMP index starts at offset 0 in the Trie2 index.
            // 16 bit data is stored in the index array itself.
            index = data[(code_point >> SHIFT_2) as usize];
            index = (index << INDEX_SHIFT) + (code_point & DATA_MASK as u32);
            return data[index as usize];
        }

        if code_point <= 0xffff {
            // Lead Surrogate Code Point.  A Separate index section is stored for
            // lead surrogate code units and code points.
            //   The main index has the code unit data.
            //   For this function, we need the code point data.
            // Note: this expression could be refactored for slightly improved efficiency, but
            //       surrogate code points will be so rare in practice that it's not worth it.
            index = data[(LSCP_INDEX_2_OFFSET as u32 + ((code_point - 0xd800) >> SHIFT_2)) as usize];
            index = (index << INDEX_SHIFT) + (code_point & DATA_MASK as u32);
            return data[index as usize];
        }

        if code_point < self.high_start as u32 {
            // Supplemental code point, use two-level lookup.
            index = (INDEX_1_OFFSET - OMITTED_BMP_INDEX_1_LENGTH) as u32 + (code_point >> SHIFT_1);
            index = data[index as usize];
            index += (code_point >> SHIFT_2) & INDEX_2_MASK as u32;
            index = data[index as usize];
            index = (index << INDEX_SHIFT) + (code_point & DATA_MASK as u32);
            return data[index as usize];
        }

        if code_point <= 0x10ffff {
            return data[data.len() - DATA_GRANULARITY as usize];
        }

        // Fall through.  The code point is outside of the legal range of 0..0x10ffff.
        self.error_value
    }
}
