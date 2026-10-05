// Copyright (c) Six Labors.
// Licensed under the Apache License, Version 2.0.
// Ported from: https://github.com/SixLabors/Fonts/

use super::bidi_class::BidiClass;
use super::bidi_paired_bracket_type::BidiPairedBracketType;
use super::codepoint::Codepoint;
use super::codepoint_enumerator::CodepointEnumerator;

// 1MB, arbitrary, that's 512K characters or 128K object references on x64
const MAX_KEPT_BUFFER_SIZE_IN_BYTES: usize = 1024 * 1024;

/// Clears a reusable work buffer, releasing its memory when it grew too large
/// to be worth keeping (the formatting buffer helper of the text formatting
/// module upstream; kept local so the `unicode` module is self-contained).
pub(super) fn clear_then_reset_if_too_large<T>(buffer: &mut Vec<T>) {
    buffer.clear();

    if std::mem::size_of::<T>().saturating_mul(buffer.capacity()) > MAX_KEPT_BUFFER_SIZE_IN_BYTES {
        *buffer = Vec::new();
    }
}

/// Represents a unicode string and all associated attributes
/// for each character required for the bidirectional Unicode algorithm
///
/// To avoid allocations, this class is designed to be reused.
#[derive(Debug)]
pub struct BidiData {
    has_clean_state: bool,
    classes: Vec<BidiClass>,
    paired_bracket_types: Vec<BidiPairedBracketType>,
    paired_bracket_values: Vec<i32>,
    saved_classes: Vec<BidiClass>,
    saved_paired_bracket_types: Vec<BidiPairedBracketType>,
    temp_level_buffer: Vec<i8>,

    paragraph_embedding_level: i8,
    has_brackets: Option<bool>,
    has_embeddings: Option<bool>,
    has_isolates: Option<bool>,
    length: usize,
}

impl Default for BidiData {
    fn default() -> Self {
        Self::new()
    }
}

impl BidiData {
    pub const fn new() -> Self {
        Self {
            has_clean_state: true,
            classes: Vec::new(),
            paired_bracket_types: Vec::new(),
            paired_bracket_values: Vec::new(),
            saved_classes: Vec::new(),
            saved_paired_bracket_types: Vec::new(),
            temp_level_buffer: Vec::new(),
            paragraph_embedding_level: 0,
            has_brackets: None,
            has_embeddings: None,
            has_isolates: None,
            length: 0,
        }
    }

    #[inline]
    pub fn paragraph_embedding_level(&self) -> i8 {
        self.paragraph_embedding_level
    }

    #[inline]
    pub fn set_paragraph_embedding_level(&mut self, value: i8) {
        self.paragraph_embedding_level = value;
    }

    #[inline]
    pub fn has_brackets(&self) -> Option<bool> {
        self.has_brackets
    }

    #[inline]
    pub fn has_embeddings(&self) -> Option<bool> {
        self.has_embeddings
    }

    #[inline]
    pub fn has_isolates(&self) -> Option<bool> {
        self.has_isolates
    }

    /// Gets the length of the data held by the BidiData
    ///
    /// This is the number of codepoints appended (a surrogate pair counts once).
    #[inline]
    pub fn length(&self) -> usize {
        self.length
    }

    /// Gets the bidi character type of each code point
    #[inline]
    pub fn classes(&self) -> &[BidiClass] {
        &self.classes[..self.length.min(self.classes.len())]
    }

    /// The mutable view of [`BidiData::classes`] (the slice is writable upstream).
    #[inline]
    pub fn classes_mut(&mut self) -> &mut [BidiClass] {
        let length = self.length.min(self.classes.len());
        &mut self.classes[..length]
    }

    /// Gets the paired bracket type for each code point
    #[inline]
    pub fn paired_bracket_types(&self) -> &[BidiPairedBracketType] {
        &self.paired_bracket_types[..self.length.min(self.paired_bracket_types.len())]
    }

    /// The mutable view of [`BidiData::paired_bracket_types`] (the slice is writable upstream).
    #[inline]
    pub fn paired_bracket_types_mut(&mut self) -> &mut [BidiPairedBracketType] {
        let length = self.length.min(self.paired_bracket_types.len());
        &mut self.paired_bracket_types[..length]
    }

    /// Gets the paired bracket value for code point
    ///
    /// The paired bracket values are the code points
    /// of each character where the opening code point
    /// is replaced with the closing code point for easier
    /// matching.  Also, bracket code points are mapped
    /// to their canonical equivalents
    #[inline]
    pub fn paired_bracket_values(&self) -> &[i32] {
        &self.paired_bracket_values[..self.length]
    }

    /// The three per-codepoint tables at once, with the classes writable: the
    /// bidi algorithm reads the bracket tables while it updates the classes.
    #[inline]
    pub(crate) fn parts_mut(&mut self) -> (&mut [BidiClass], &[BidiPairedBracketType], &[i32]) {
        // The tables are shorter than `length` only after `restore_types` without a
        // matching `save_types` (upstream then keeps exposing stale data).
        let length = self.length.min(self.classes.len());
        (
            &mut self.classes[..length],
            &self.paired_bracket_types[..self.length.min(self.paired_bracket_types.len())],
            &self.paired_bracket_values[..self.length],
        )
    }

    /// Appends text to the bidi data.
    ///
    /// * `text` - The text to process.
    pub fn append(&mut self, text: &[u16]) {
        self.has_clean_state = false;

        // One entry per code unit is reserved, as upstream does; only one entry
        // per codepoint is used.
        self.classes.resize(self.classes.len() + text.len(), BidiClass::LeftToRight);
        self.paired_bracket_types.resize(self.paired_bracket_types.len() + text.len(), BidiPairedBracketType::None);
        self.paired_bracket_values.resize(self.paired_bracket_values.len() + text.len(), 0);

        // Resolve the BidiCharacterType, paired bracket type and paired
        // bracket values for all code points

        let mut i = self.length;

        const EMBEDDING_MASK: u32 = (1u32 << BidiClass::LeftToRightEmbedding as i32)
            | (1u32 << BidiClass::LeftToRightOverride as i32)
            | (1u32 << BidiClass::RightToLeftEmbedding as i32)
            | (1u32 << BidiClass::RightToLeftOverride as i32)
            | (1u32 << BidiClass::PopDirectionalFormat as i32);

        const ISOLATE_MASK: u32 = (1u32 << BidiClass::LeftToRightIsolate as i32)
            | (1u32 << BidiClass::RightToLeftIsolate as i32)
            | (1u32 << BidiClass::FirstStrongIsolate as i32)
            | (1u32 << BidiClass::PopDirectionalIsolate as i32);

        let mut code_point_enumerator = CodepointEnumerator::new(text);

        while let Some(codepoint) = code_point_enumerator.move_next() {
            // Look up BiDiClass
            let dir = codepoint.bi_di_class();

            self.classes[i] = dir;

            let dir_bit = 1u32 << dir as i32;

            if self.has_embeddings.is_none() && (dir_bit & EMBEDDING_MASK) != 0 {
                self.has_embeddings = Some(true);
            }

            if self.has_isolates.is_none() && (dir_bit & ISOLATE_MASK) != 0 {
                self.has_isolates = Some(true);
            }

            // Lookup paired bracket types
            let pbt = codepoint.paired_bracket_type();

            self.paired_bracket_types[i] = pbt;

            if pbt == BidiPairedBracketType::Open {
                // Opening bracket types can never have a null pairing.
                let paired = codepoint.try_get_paired_bracket().unwrap_or_default();

                self.paired_bracket_values[i] = Codepoint::get_canonical_type(paired).value() as i32;

                self.has_brackets = Some(true);
            } else if pbt == BidiPairedBracketType::Close {
                self.paired_bracket_values[i] = Codepoint::get_canonical_type(codepoint).value() as i32;

                self.has_brackets = Some(true);
            }

            i += 1;
        }

        self.length = i;
    }

    /// Save the Types and PairedBracketTypes of this BiDiData
    ///
    /// This is used when processing embedded style runs with
    /// BiDiClass overrides. Text layout process saves the data,
    /// overrides the style runs to neutral, processes the bidi
    /// data for the entire paragraph and then restores this data
    /// before processing the embedded runs.
    pub fn save_types(&mut self) {
        self.has_clean_state = false;

        // Capture the types data
        self.saved_classes.clear();
        self.saved_classes.extend_from_slice(&self.classes);
        self.saved_paired_bracket_types.clear();
        self.saved_paired_bracket_types.extend_from_slice(&self.paired_bracket_types);
    }

    /// Restore the data saved by SaveTypes
    pub fn restore_types(&mut self) {
        self.has_clean_state = false;

        self.classes.clear();
        self.classes.extend_from_slice(&self.saved_classes);
        self.paired_bracket_types.clear();
        self.paired_bracket_types.extend_from_slice(&self.saved_paired_bracket_types);
    }

    /// Gets a temporary level buffer. Used by the text layout process when
    /// resolving style runs with different BiDiClass.
    ///
    /// * `length` - Length of the required buffer
    ///
    /// Returns a level buffer of that length (zeroed here; uninitialized upstream).
    pub fn get_temp_level_buffer(&mut self, length: usize) -> &mut [i8] {
        self.temp_level_buffer.clear();
        self.temp_level_buffer.resize(length, 0);
        &mut self.temp_level_buffer
    }

    /// Resets the bidi data to a clean state.
    pub fn reset(&mut self) {
        if self.has_clean_state {
            return;
        }

        clear_then_reset_if_too_large(&mut self.classes);
        clear_then_reset_if_too_large(&mut self.paired_bracket_types);
        clear_then_reset_if_too_large(&mut self.paired_bracket_values);
        clear_then_reset_if_too_large(&mut self.saved_classes);
        clear_then_reset_if_too_large(&mut self.saved_paired_bracket_types);
        clear_then_reset_if_too_large(&mut self.temp_level_buffer);

        self.paragraph_embedding_level = 0;
        self.has_brackets = Some(false);
        self.has_embeddings = Some(false);
        self.has_isolates = Some(false);

        self.length = 0;

        self.has_clean_state = true;
    }
}
