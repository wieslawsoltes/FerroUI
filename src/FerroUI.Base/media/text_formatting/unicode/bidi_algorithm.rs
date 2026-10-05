// Copyright (c) Six Labors.
// Licensed under the Apache License, Version 2.0.
// Ported from: https://github.com/SixLabors/Fonts/

use std::collections::HashMap;

use super::bidi_class::BidiClass;
use super::bidi_data::{clear_then_reset_if_too_large, BidiData};
use super::bidi_paired_bracket_type::BidiPairedBracketType;

/// Maximum pairing depth for paired brackets
const MAX_PAIRED_BRACKET_DEPTH: usize = 63;

/// Implementation of Unicode bidirectional algorithm (UAX #9)
/// <https://unicode.org/reports/tr9/>
///
/// The Bidi algorithm uses a number of memory arrays for resolved
/// types, level information, bracket types, x9 removal maps and
/// more...
///
/// This implementation of the BiDi algorithm has been designed
/// to reduce memory pressure by re-using the same
/// work buffers, so instances of this class should be re-used
/// as much as possible.
///
/// The caller's tables (original classes, paired bracket types and values,
/// and the optional output levels) are borrowed for the duration of a
/// `process` call instead of being stored as slices like upstream does.
#[derive(Debug)]
pub struct BidiAlgorithm {
    /// Whether the state is clean and can be reused without a reset.
    has_clean_state: bool,

    /// Try if the incoming data is known to contain brackets
    has_brackets: bool,

    /// True if the incoming data is known to contain embedding runs
    has_embeddings: bool,

    /// True if the incoming data is known to contain isolating runs
    has_isolates: bool,

    /// Mapping of isolate start/end pairs
    ///
    /// Maps the start index to the end index. (Upstream keeps a two directional
    /// dictionary but only ever reads the forward direction.)
    isolate_pairs: HashMap<usize, usize>,

    /// The working BiDi classes
    working_classes: Vec<BidiClass>,

    /// The resolved levels (empty when the caller supplied the output buffer)
    resolved_levels: Vec<i8>,

    /// The resolve paragraph embedding level
    paragraph_embedding_level: i8,

    /// The status stack used during resolution of explicit
    /// embedding and isolating runs
    status_stack: Vec<Status>,

    /// Mapping used to virtually remove characters for rule X9
    x9_map: Vec<usize>,

    /// Re-usable list of level runs
    level_runs: Vec<LevelRun>,

    /// Mapping for the current isolating sequence, built
    /// by joining level runs from the x9 map.
    isolated_run_mapping: Vec<usize>,

    /// A stack of pending isolate openings used by find_isolate_pairs()
    pending_isolate_openings: Vec<usize>,

    /// Reusable list of pending opening brackets used by the
    /// locate_paired_brackets method
    pending_opening_brackets: Vec<usize>,

    /// Resolved list of paired brackets
    paired_brackets: Vec<BracketPair>,
}

impl Default for BidiAlgorithm {
    fn default() -> Self {
        Self::new()
    }
}

impl BidiAlgorithm {
    /// Initializes a new instance of the [`BidiAlgorithm`] class.
    pub fn new() -> Self {
        Self {
            has_clean_state: true,
            has_brackets: false,
            has_embeddings: false,
            has_isolates: false,
            isolate_pairs: HashMap::new(),
            working_classes: Vec::new(),
            resolved_levels: Vec::new(),
            paragraph_embedding_level: 0,
            status_stack: Vec::new(),
            x9_map: Vec::new(),
            level_runs: Vec::new(),
            isolated_run_mapping: Vec::new(),
            pending_isolate_openings: Vec::new(),
            pending_opening_brackets: Vec::new(),
            paired_brackets: Vec::new(),
        }
    }

    /// Gets the resolved levels.
    ///
    /// Empty when the last `process_classes` call was given an output buffer
    /// (the levels were written there).
    #[inline]
    pub fn resolved_levels(&self) -> &[i8] {
        &self.resolved_levels
    }

    /// Gets the resolved paragraph embedding level
    #[inline]
    pub fn resolved_paragraph_embedding_level(&self) -> i32 {
        self.paragraph_embedding_level as i32
    }

    /// Process data from a BiDiData instance
    ///
    /// * `data` - The BiDi Unicode data.
    ///
    /// The classes of `data` are updated in place by rule N0 (nonspacing marks
    /// after resolved brackets take the bracket direction), as upstream.
    pub fn process(&mut self, data: &mut BidiData) {
        let paragraph_embedding_level = data.paragraph_embedding_level();
        let has_brackets = data.has_brackets();
        let has_embeddings = data.has_embeddings();
        let has_isolates = data.has_isolates();
        let (classes, paired_bracket_types, paired_bracket_values) = data.parts_mut();

        self.process_classes(
            classes,
            paired_bracket_types,
            paired_bracket_values,
            paragraph_embedding_level,
            has_brackets,
            has_embeddings,
            has_isolates,
            None,
        );
    }

    /// Processes Bidi Data
    ///
    /// Panics when `out_levels` is given and its length differs from the input data.
    #[allow(clippy::too_many_arguments)]
    pub fn process_classes(
        &mut self,
        types: &mut [BidiClass],
        paired_bracket_types: &[BidiPairedBracketType],
        paired_bracket_values: &[i32],
        paragraph_embedding_level: i8,
        has_brackets: Option<bool>,
        has_embeddings: Option<bool>,
        has_isolates: Option<bool>,
        out_levels: Option<&mut [i8]>,
    ) {
        // Reset state
        self.reset();

        if types.is_empty() {
            return;
        }

        self.has_clean_state = false;

        // Setup original types and working types
        self.working_classes.extend_from_slice(types);

        // Store things we know
        self.has_brackets = has_brackets.unwrap_or(paired_bracket_types.len() == types.len());
        self.has_embeddings = has_embeddings.unwrap_or(true);
        self.has_isolates = has_isolates.unwrap_or(true);

        // Find all isolate pairs
        self.find_isolate_pairs(types);

        // Resolve the paragraph embedding level
        if paragraph_embedding_level == 2 {
            self.paragraph_embedding_level = self.resolve_embedding_level_at(types, 0);
        } else {
            self.paragraph_embedding_level = paragraph_embedding_level;
        }

        // Create resolved levels buffer
        match out_levels {
            Some(out_levels) => {
                if out_levels.len() != types.len() {
                    panic!("Out levels must be the same length as the input data");
                }

                self.resolve(types, paired_bracket_types, paired_bracket_values, out_levels);
            }
            None => {
                // The buffer is moved out while the algorithm runs so that it can be
                // borrowed next to the other work buffers; its capacity is kept.
                let mut resolved_levels = std::mem::take(&mut self.resolved_levels);
                resolved_levels.clear();
                resolved_levels.resize(types.len(), self.paragraph_embedding_level);

                self.resolve(types, paired_bracket_types, paired_bracket_values, &mut resolved_levels);

                self.resolved_levels = resolved_levels;
            }
        }
    }

    /// The steps of `process_classes` that follow the buffer set up.
    fn resolve(
        &mut self,
        original_classes: &mut [BidiClass],
        paired_bracket_types: &[BidiPairedBracketType],
        paired_bracket_values: &[i32],
        resolved_levels: &mut [i8],
    ) {
        // Resolve explicit embedding levels (Rules X1-X8)
        self.resolve_explicit_embedding_levels(original_classes, resolved_levels);

        // Build the rule X9 map
        self.build_x9_removal_map(original_classes);

        // Process all isolated run sequences
        self.process_isolated_run_sequences(original_classes, paired_bracket_types, paired_bracket_values, resolved_levels);

        // Reset whitespace levels
        self.reset_whitespace_levels(original_classes, resolved_levels);

        // Clean up
        self.assign_levels_to_code_points_removed_by_x9(original_classes, resolved_levels);
    }

    /// Resolve the paragraph embedding level if not explicitly passed
    /// by the caller. Also used by rule X5c for FSI isolating sequences.
    ///
    /// * `data` - The data to be evaluated; it must be the data of the last
    ///   `process` call (or a prefix of it), because the isolate pairs found by
    ///   that call are used to skip isolates.
    ///
    /// Returns the resolved embedding level
    pub fn resolve_embedding_level(&self, data: &[BidiClass]) -> i8 {
        self.resolve_embedding_level_at(data, 0)
    }

    /// `data_start` is the offset of `data` inside the processed classes
    /// (the `Start` of the slice upstream).
    fn resolve_embedding_level_at(&self, data: &[BidiClass], data_start: usize) -> i8 {
        // P2
        let mut i = 0;
        while i < data.len() {
            match data[i] {
                BidiClass::LeftToRight => {
                    // P3
                    return 0;
                }

                BidiClass::ArabicLetter | BidiClass::RightToLeft => {
                    // P3
                    return 1;
                }

                BidiClass::FirstStrongIsolate | BidiClass::LeftToRightIsolate | BidiClass::RightToLeftIsolate => {
                    // Skip isolate pairs
                    // (Because we're working with a slice, we need to adjust the indices
                    //  we're using for the isolatePairs map)
                    match self.isolate_pairs.get(&(data_start + i)) {
                        // An end before the slice cannot be expressed; the scan would
                        // restart from a negative index upstream.
                        Some(&end) => i = end.saturating_sub(data_start),
                        None => i = data.len(),
                    }
                }

                _ => {}
            }

            i += 1;
        }

        // P3
        0
    }

    /// Build a list of matching isolates for a directionality slice
    /// Implements BD9
    fn find_isolate_pairs(&mut self, original_classes: &[BidiClass]) {
        // Redundant?
        if !self.has_isolates {
            return;
        }

        // Lets double check this as we go and clear the flag
        // if there actually aren't any isolate pairs as this might
        // mean we can skip some later steps
        self.has_isolates = false;

        // BD9...
        self.pending_isolate_openings.clear();

        for (i, &t) in original_classes.iter().enumerate() {
            match t {
                BidiClass::LeftToRightIsolate | BidiClass::RightToLeftIsolate | BidiClass::FirstStrongIsolate => {
                    self.pending_isolate_openings.push(i);
                    self.has_isolates = true;
                }
                BidiClass::PopDirectionalIsolate => {
                    if let Some(opening) = self.pending_isolate_openings.pop() {
                        self.isolate_pairs.insert(opening, i);
                    }

                    self.has_isolates = true;
                }
                _ => {}
            }
        }
    }

    /// Resolve the explicit embedding levels from the original
    /// data.  Implements rules X1 to X8.
    fn resolve_explicit_embedding_levels(&mut self, original_classes: &[BidiClass], resolved_levels: &mut [i8]) {
        // Redundant?
        if !self.has_isolates && !self.has_embeddings {
            return;
        }

        // Work variables
        self.status_stack.clear();
        let mut overflow_isolate_count = 0;
        let mut overflow_embedding_count = 0;
        let mut valid_isolate_count = 0;

        // Constants
        const MAX_STACK_DEPTH: i8 = 125;

        // Rule X1 - setup initial state
        self.status_stack.clear();

        // Neutral
        self.status_stack.push(Status::new(self.paragraph_embedding_level, BidiClass::OtherNeutral, false));

        #[inline(always)]
        fn peek(stack: &[Status]) -> Status {
            *stack.last().expect("the status stack is never empty")
        }

        #[inline(always)]
        fn next_odd_level(level: i8) -> i8 {
            ((level as i32 + 1) | 1) as i8
        }

        #[inline(always)]
        fn next_even_level(level: i8) -> i8 {
            ((level as i32 + 2) & !1) as i8
        }

        // Process all characters
        for i in 0..original_classes.len() {
            match original_classes[i] {
                BidiClass::RightToLeftEmbedding => {
                    // Rule X2
                    let new_level = next_odd_level(peek(&self.status_stack).embedding_level);
                    if new_level <= MAX_STACK_DEPTH && overflow_isolate_count == 0 && overflow_embedding_count == 0 {
                        self.status_stack.push(Status::new(new_level, BidiClass::OtherNeutral, false));
                        resolved_levels[i] = new_level;
                    } else if overflow_isolate_count == 0 {
                        overflow_embedding_count += 1;
                    }
                }

                BidiClass::LeftToRightEmbedding => {
                    // Rule X3
                    let new_level = next_even_level(peek(&self.status_stack).embedding_level);
                    if new_level < MAX_STACK_DEPTH && overflow_isolate_count == 0 && overflow_embedding_count == 0 {
                        self.status_stack.push(Status::new(new_level, BidiClass::OtherNeutral, false));
                        resolved_levels[i] = new_level;
                    } else if overflow_isolate_count == 0 {
                        overflow_embedding_count += 1;
                    }
                }

                BidiClass::RightToLeftOverride => {
                    // Rule X4
                    let new_level = next_odd_level(peek(&self.status_stack).embedding_level);
                    if new_level <= MAX_STACK_DEPTH && overflow_isolate_count == 0 && overflow_embedding_count == 0 {
                        self.status_stack.push(Status::new(new_level, BidiClass::RightToLeft, false));
                        resolved_levels[i] = new_level;
                    } else if overflow_isolate_count == 0 {
                        overflow_embedding_count += 1;
                    }
                }

                BidiClass::LeftToRightOverride => {
                    // Rule X5
                    let new_level = next_even_level(peek(&self.status_stack).embedding_level);
                    if new_level <= MAX_STACK_DEPTH && overflow_isolate_count == 0 && overflow_embedding_count == 0 {
                        self.status_stack.push(Status::new(new_level, BidiClass::LeftToRight, false));
                        resolved_levels[i] = new_level;
                    } else if overflow_isolate_count == 0 {
                        overflow_embedding_count += 1;
                    }
                }

                BidiClass::RightToLeftIsolate | BidiClass::LeftToRightIsolate | BidiClass::FirstStrongIsolate => {
                    // Rule X5a, X5b and X5c
                    let mut resolved_isolate = original_classes[i];

                    if resolved_isolate == BidiClass::FirstStrongIsolate {
                        let end_of_isolate = match self.isolate_pairs.get(&i) {
                            Some(&end_of_isolate) => end_of_isolate,
                            None => original_classes.len(),
                        };

                        // Rule X5c
                        if self.resolve_embedding_level_at(&original_classes[i + 1..end_of_isolate], i + 1) == 1 {
                            resolved_isolate = BidiClass::RightToLeftIsolate;
                        } else {
                            resolved_isolate = BidiClass::LeftToRightIsolate;
                        }
                    }

                    // Replace RLI's level with current embedding level
                    let tos = peek(&self.status_stack);
                    resolved_levels[i] = tos.embedding_level;

                    // Apply override
                    if tos.override_status != BidiClass::OtherNeutral {
                        self.working_classes[i] = tos.override_status;
                    }

                    // Work out new level
                    let new_level = if resolved_isolate == BidiClass::RightToLeftIsolate {
                        next_odd_level(tos.embedding_level)
                    } else {
                        next_even_level(tos.embedding_level)
                    };

                    // Valid?
                    if new_level <= MAX_STACK_DEPTH && overflow_isolate_count == 0 && overflow_embedding_count == 0 {
                        valid_isolate_count += 1;
                        self.status_stack.push(Status::new(new_level, BidiClass::OtherNeutral, true));
                    } else {
                        overflow_isolate_count += 1;
                    }
                }

                BidiClass::BoundaryNeutral => {
                    // Mentioned in rule X6 - "for all types besides ..., BN, ..."
                    // no-op
                }

                BidiClass::PopDirectionalIsolate => {
                    // Rule X6a
                    if overflow_isolate_count > 0 {
                        overflow_isolate_count -= 1;
                    } else if valid_isolate_count != 0 {
                        overflow_embedding_count = 0;
                        while !peek(&self.status_stack).isolate_status {
                            self.status_stack.pop();
                        }

                        self.status_stack.pop();
                        valid_isolate_count -= 1;
                    }

                    let tos = peek(&self.status_stack);
                    resolved_levels[i] = tos.embedding_level;
                    if tos.override_status != BidiClass::OtherNeutral {
                        self.working_classes[i] = tos.override_status;
                    }
                }

                BidiClass::PopDirectionalFormat => {
                    // Rule X7
                    if overflow_isolate_count == 0 {
                        if overflow_embedding_count > 0 {
                            overflow_embedding_count -= 1;
                        } else if !peek(&self.status_stack).isolate_status && self.status_stack.len() >= 2 {
                            self.status_stack.pop();
                        }
                    }
                }

                BidiClass::ParagraphSeparator => {
                    // Rule X8
                    resolved_levels[i] = self.paragraph_embedding_level;
                }

                _ => {
                    // Rule X6
                    let tos = peek(&self.status_stack);
                    resolved_levels[i] = tos.embedding_level;
                    if tos.override_status != BidiClass::OtherNeutral {
                        self.working_classes[i] = tos.override_status;
                    }
                }
            }
        }
    }

    /// Build a map to the original data positions that excludes all
    /// the types defined by rule X9
    fn build_x9_removal_map(&mut self, original_classes: &[BidiClass]) {
        // Reserve room for the x9 map
        self.x9_map.clear();
        self.x9_map.reserve(original_classes.len());

        if self.has_embeddings || self.has_isolates {
            // Build a map the removes all x9 characters
            for (i, &original_class) in original_classes.iter().enumerate() {
                if !is_removed_by_x9(original_class) {
                    self.x9_map.push(i);
                }
            }
        } else {
            self.x9_map.extend(0..original_classes.len());
        }
    }

    /// Find the original character index for an entry in the X9 map
    ///
    /// * `index` - Index in the x9 removal map
    ///
    /// Returns the index to the original data
    #[inline(always)]
    fn map_x9(&self, index: usize) -> usize {
        self.x9_map[index]
    }

    /// Add a new level run
    ///
    /// This method resolves the sos and eos values for the run
    /// and adds the run to the list
    ///
    /// * `start` - The index of the start of the run (in x9 removed units)
    /// * `length` - The length of the run (in x9 removed units)
    /// * `level` - The level of the run
    fn add_level_run(
        &mut self,
        start: usize,
        length: usize,
        level: i32,
        original_classes: &[BidiClass],
        resolved_levels: &[i8],
    ) {
        // Get original indices to first and last character in this run
        let first_char_index = self.map_x9(start);
        let last_char_index = self.map_x9(start + length - 1);

        // Work out sos
        let prev_level = original_classes[..first_char_index]
            .iter()
            .rposition(|&original_class| !is_removed_by_x9(original_class))
            .map_or(self.paragraph_embedding_level as i32, |i| resolved_levels[i] as i32);
        let sos = direction_from_level(prev_level.max(level));

        // Work out eos
        let last_type = self.working_classes[last_char_index];
        let next_level = if is_isolate_start(last_type) {
            self.paragraph_embedding_level as i32
        } else {
            let mut i = last_char_index + 1;
            while i < original_classes.len() && is_removed_by_x9(original_classes[i]) {
                i += 1;
            }

            if i >= original_classes.len() {
                self.paragraph_embedding_level as i32
            } else {
                resolved_levels[i] as i32
            }
        };

        let eos = direction_from_level(next_level.max(level));

        // Add the run
        self.level_runs.push(LevelRun::new(start, length, level, sos, eos));
    }

    /// Find all runs of the same level, populating the level_runs
    /// collection
    fn find_level_runs(&mut self, original_classes: &[BidiClass], resolved_levels: &[i8]) {
        let mut current_level: i32 = -1;
        let mut run_start = 0;
        for i in 0..self.x9_map.len() {
            let level = resolved_levels[self.map_x9(i)] as i32;
            if level == current_level {
                continue;
            }

            if current_level != -1 {
                self.add_level_run(run_start, i - run_start, current_level, original_classes, resolved_levels);
            }

            current_level = level;
            run_start = i;
        }

        // Don't forget the final level run
        if current_level != -1 {
            self.add_level_run(
                run_start,
                self.x9_map.len() - run_start,
                current_level,
                original_classes,
                resolved_levels,
            );
        }
    }

    /// Given a character index, find the level run that starts at that position
    ///
    /// * `index` - The index into the original (unmapped) data
    ///
    /// Returns the index of the run that starts at that index
    fn find_run_for_index(&self, index: usize) -> usize {
        for i in 0..self.level_runs.len() {
            // Passed index is for the original non-x9 filtered data, however
            // the level run ranges are for the x9 filtered data.  Convert before
            // comparing
            if self.map_x9(self.level_runs[i].start) == index {
                return i;
            }
        }

        panic!("Internal error");
    }

    /// Determine and the process all isolated run sequences
    fn process_isolated_run_sequences(
        &mut self,
        original_classes: &mut [BidiClass],
        paired_bracket_types: &[BidiPairedBracketType],
        paired_bracket_values: &[i32],
        resolved_levels: &mut [i8],
    ) {
        // Find all runs with the same level
        self.find_level_runs(original_classes, resolved_levels);

        // Process them one at a time by first building
        // a mapping using slices from the x9 map for each
        // run section that needs to be joined together to
        // form an complete run.  That full run mapping
        // will be placed in isolated_run_mapping and then
        // processed by process_isolated_run_sequence().
        while !self.level_runs.is_empty() {
            // Clear the mapping
            self.isolated_run_mapping.clear();

            // Combine mappings from this run and all runs that continue on from it
            let mut run_index = 0;
            let mut eos;
            let sos = self.level_runs[0].sos;
            let level = self.level_runs[0].level;
            loop {
                // Get the run
                // Remove this run as we've now processed it
                let r = self.level_runs.remove(run_index);

                // The eos of the isolating run is the eos of the
                // last level run that comprises it.
                eos = r.eos;

                // Add the x9 map indices for the run range to the mapping
                // for this isolated run
                self.isolated_run_mapping.extend_from_slice(&self.x9_map[r.start..r.start + r.length]);

                // Get the last character and see if it's an isolating run with a matching
                // PDI and concatenate that run to this one
                let last_character_index = self.isolated_run_mapping[self.isolated_run_mapping.len() - 1];
                let last_type = original_classes[last_character_index];
                let next_run_index =
                    if is_isolate_start(last_type) { self.isolate_pairs.get(&last_character_index).copied() } else { None };

                match next_run_index {
                    Some(next_run_index) => {
                        // Find the continuing run index
                        run_index = self.find_run_for_index(next_run_index);
                    }
                    None => break,
                }
            }

            // Process this isolated run
            self.process_isolated_run_sequence(
                sos,
                eos,
                level,
                original_classes,
                paired_bracket_types,
                paired_bracket_values,
                resolved_levels,
            );
        }
    }

    /// Process a single isolated run sequence, where the character sequence
    /// mapping is currently held in isolated_run_mapping.
    #[allow(clippy::too_many_arguments)]
    fn process_isolated_run_sequence(
        &mut self,
        sos: BidiClass,
        eos: BidiClass,
        run_level: i32,
        original_classes: &mut [BidiClass],
        paired_bracket_types: &[BidiPairedBracketType],
        paired_bracket_values: &[i32],
        resolved_levels: &mut [i8],
    ) {
        let has_brackets = self.has_brackets;

        // Create mappings onto the underlying data
        let mut run = IsolatedRun {
            map: &self.isolated_run_mapping,
            resolved_classes: &mut self.working_classes,
            original_classes,
            levels: resolved_levels,
            paired_bracket_types,
            paired_bracket_values,
            level: run_level,
            direction: direction_from_level(run_level),
            length: self.isolated_run_mapping.len(),
        };
        let run_length = run.length;

        // Rule W1
        // Also, set hasXX flags
        let mut previous_class = sos;

        const ISOLATE_MASK: u32 = (1u32 << BidiClass::LeftToRightIsolate as i32)
            | (1u32 << BidiClass::RightToLeftIsolate as i32)
            | (1u32 << BidiClass::FirstStrongIsolate as i32)
            | (1u32 << BidiClass::PopDirectionalIsolate as i32);

        const W_RULES_MASK: u32 = (1u32 << BidiClass::EuropeanNumber as i32)
            | (1u32 << BidiClass::ArabicLetter as i32)
            | (1u32 << BidiClass::EuropeanSeparator as i32)
            | (1u32 << BidiClass::CommonSeparator as i32)
            | (1u32 << BidiClass::ArabicNumber as i32)
            | (1u32 << BidiClass::EuropeanTerminator as i32);

        let mut w_rules: u32 = 0;

        for i in 0..run_length {
            let resolved_class = run.resolved_class(i);

            if resolved_class == BidiClass::NonspacingMark {
                run.set_resolved_class(i, previous_class);
            } else {
                let class_bit = 1u32 << resolved_class as i32;

                if (class_bit & ISOLATE_MASK) != 0 {
                    previous_class = BidiClass::OtherNeutral;
                } else {
                    w_rules |= class_bit & W_RULES_MASK;
                    previous_class = resolved_class;
                }
            }
        }

        // By tracking the types of characters known to be in the current run, we can
        // skip some of the rules that we know won't apply.
        let has_en = (w_rules & (1u32 << BidiClass::EuropeanNumber as i32)) != 0;
        let has_al = (w_rules & (1u32 << BidiClass::ArabicLetter as i32)) != 0;
        let has_es = (w_rules & (1u32 << BidiClass::EuropeanSeparator as i32)) != 0;
        let has_cs = (w_rules & (1u32 << BidiClass::CommonSeparator as i32)) != 0;
        let mut has_an = (w_rules & (1u32 << BidiClass::ArabicNumber as i32)) != 0;
        let has_et = (w_rules & (1u32 << BidiClass::EuropeanTerminator as i32)) != 0;

        // Rule W2
        if has_en {
            for i in 0..run_length {
                if run.resolved_class(i) != BidiClass::EuropeanNumber {
                    continue;
                }

                for j in (0..i).rev() {
                    let resolved_class = run.resolved_class(j);

                    match resolved_class {
                        BidiClass::LeftToRight | BidiClass::RightToLeft | BidiClass::ArabicLetter => {
                            if resolved_class == BidiClass::ArabicLetter {
                                run.set_resolved_class(i, BidiClass::ArabicNumber);
                                has_an = true;
                            }

                            break;
                        }
                        _ => {}
                    }
                }
            }
        }

        // Rule W3
        if has_al {
            for i in 0..run_length {
                if run.resolved_class(i) == BidiClass::ArabicLetter {
                    run.set_resolved_class(i, BidiClass::RightToLeft);
                }
            }
        }

        // Rule W4
        if (has_es || has_cs) && (has_en || has_an) {
            let mut i = 1;
            while i + 1 < run_length {
                let resolved_class = run.resolved_class(i);

                if resolved_class == BidiClass::EuropeanSeparator {
                    let previous_separator_class = run.resolved_class(i - 1);
                    let next_separator_class = run.resolved_class(i + 1);

                    if previous_separator_class == BidiClass::EuropeanNumber
                        && next_separator_class == BidiClass::EuropeanNumber
                    {
                        // ES between EN and EN
                        run.set_resolved_class(i, BidiClass::EuropeanNumber);
                    }
                } else if resolved_class == BidiClass::CommonSeparator {
                    let previous_separator_class = run.resolved_class(i - 1);
                    let next_separator_class = run.resolved_class(i + 1);

                    if (previous_separator_class == BidiClass::ArabicNumber
                        && next_separator_class == BidiClass::ArabicNumber)
                        || (previous_separator_class == BidiClass::EuropeanNumber
                            && next_separator_class == BidiClass::EuropeanNumber)
                    {
                        // CS between (AN and AN) or (EN and EN)
                        run.set_resolved_class(i, previous_separator_class);
                    }
                }

                i += 1;
            }
        }

        // Rule W5
        if has_et && has_en {
            let mut i = 0;
            while i < run_length {
                if run.resolved_class(i) != BidiClass::EuropeanTerminator {
                    i += 1;
                    continue;
                }

                // Locate end of sequence
                let sequence_start = i;
                let mut sequence_end = i;

                while sequence_end < run_length && run.resolved_class(sequence_end) == BidiClass::EuropeanTerminator {
                    sequence_end += 1;
                }

                // Preceded by, or followed by EN?
                if (if sequence_start == 0 { sos } else { run.resolved_class(sequence_start - 1) })
                    == BidiClass::EuropeanNumber
                    || (if sequence_end == run_length { eos } else { run.resolved_class(sequence_end) })
                        == BidiClass::EuropeanNumber
                {
                    // Change the entire range
                    while i < sequence_end {
                        run.set_resolved_class(i, BidiClass::EuropeanNumber);
                        i += 1;
                    }
                }

                // continue at end of sequence
                i = sequence_end;
                i += 1;
            }
        }

        // Rule W6
        if has_es || has_et || has_cs {
            for i in 0..run_length {
                match run.resolved_class(i) {
                    BidiClass::EuropeanSeparator | BidiClass::EuropeanTerminator | BidiClass::CommonSeparator => {
                        run.set_resolved_class(i, BidiClass::OtherNeutral);
                    }
                    _ => {}
                }
            }
        }

        // Rule W7.
        if has_en {
            let mut previous_strong_class = sos;

            for i in 0..run_length {
                let resolved_class = run.resolved_class(i);

                match resolved_class {
                    BidiClass::EuropeanNumber => {
                        // If prev strong type was an L change this to L too
                        if previous_strong_class == BidiClass::LeftToRight {
                            run.set_resolved_class(i, BidiClass::LeftToRight);
                        }
                    }

                    BidiClass::LeftToRight | BidiClass::RightToLeft => {
                        // Remember previous strong type (NB: AL should already be changed to R)
                        previous_strong_class = resolved_class;
                    }
                    _ => {}
                }
            }
        }

        // Rule N0 - process bracket pairs
        if has_brackets {
            run.locate_paired_brackets(&mut self.pending_opening_brackets, &mut self.paired_brackets);

            for i in 0..self.paired_brackets.len() {
                let paired_bracket = self.paired_brackets[i];
                let mut strong_direction = run.inspect_paired_bracket(paired_bracket);

                // Case "d" - no strong types in the brackets, ignore
                if strong_direction == BidiClass::OtherNeutral {
                    continue;
                }

                // Case "b" - strong type found that matches the embedding direction
                if (strong_direction == BidiClass::LeftToRight || strong_direction == BidiClass::RightToLeft)
                    && strong_direction == run.direction
                {
                    run.set_paired_bracket_direction(paired_bracket, strong_direction);
                    continue;
                }

                // Case "c" - found opposite strong type found, look before to establish context
                strong_direction = run.inspect_before_paired_bracket(paired_bracket, sos);
                if strong_direction == run.direction || strong_direction == BidiClass::OtherNeutral {
                    strong_direction = run.direction;
                }

                run.set_paired_bracket_direction(paired_bracket, strong_direction);
            }
        }

        // Rules N1 and N2 - resolve neutral types
        let mut i = 0;
        while i < run_length {
            let resolved_class = run.resolved_class(i);

            if is_neutral_class(resolved_class) {
                // Locate end of sequence
                let seq_start = i;
                let mut seq_end = i;

                while seq_end < run_length && is_neutral_class(run.resolved_class(seq_end)) {
                    seq_end += 1;
                }

                // Work out the preceding class
                let class_before = if seq_start == 0 {
                    sos
                } else {
                    match run.resolved_class(seq_start - 1) {
                        BidiClass::ArabicNumber | BidiClass::EuropeanNumber => BidiClass::RightToLeft,
                        class_before => class_before,
                    }
                };

                // Work out the following class
                let class_after = if seq_end == run_length {
                    eos
                } else {
                    match run.resolved_class(seq_end) {
                        BidiClass::ArabicNumber | BidiClass::EuropeanNumber => BidiClass::RightToLeft,
                        class_after => class_after,
                    }
                };

                // Work out the final resolved type
                let final_resolve_class = if class_before == class_after {
                    // Rule N1
                    class_before
                } else {
                    // Rule N2
                    run.direction
                };

                // Apply changes
                for j in seq_start..seq_end {
                    run.set_resolved_class(j, final_resolve_class);
                }

                // continue after this run
                i = seq_end;
            }

            i += 1;
        }

        // Rules I1 and I2 - resolve implicit types
        if (run.level & 0x01) == 0 {
            // Rule I1 - even
            for i in 0..run_length {
                let resolved_class = run.resolved_class(i);
                let current_run_level = run.level_mut(i);

                match resolved_class {
                    BidiClass::RightToLeft => {
                        *current_run_level = current_run_level.wrapping_add(1);
                    }
                    BidiClass::ArabicNumber | BidiClass::EuropeanNumber => {
                        *current_run_level = current_run_level.wrapping_add(2);
                    }
                    _ => {}
                }
            }
        } else {
            // Rule I2 - odd
            for i in 0..run_length {
                let resolved_class = run.resolved_class(i);
                let current_run_level = run.level_mut(i);

                if resolved_class != BidiClass::RightToLeft {
                    *current_run_level = current_run_level.wrapping_add(1);
                }
            }
        }
    }

    /// Resets whitespace levels. Implements rule L1
    fn reset_whitespace_levels(&self, original_classes: &[BidiClass], resolved_levels: &mut [i8]) {
        for i in 0..resolved_levels.len() {
            let original_class = original_classes[i];

            match original_class {
                BidiClass::ParagraphSeparator | BidiClass::SegmentSeparator => {
                    // Rule L1, clauses one and two.
                    resolved_levels[i] = self.paragraph_embedding_level;

                    // Rule L1, clause three.
                    for j in (0..i).rev() {
                        if is_whitespace(original_classes[j]) {
                            // including format codes
                            resolved_levels[j] = self.paragraph_embedding_level;
                        } else {
                            break;
                        }
                    }
                }
                _ => {}
            }
        }

        // Rule L1, clause four.
        for j in (0..resolved_levels.len()).rev() {
            if is_whitespace(original_classes[j]) {
                // including format codes
                resolved_levels[j] = self.paragraph_embedding_level;
            } else {
                break;
            }
        }
    }

    /// Assign levels to any characters that would be have been
    /// removed by rule X9.  The idea is to keep level runs together
    /// that would otherwise be broken by an interfering isolate/embedding
    /// control character.
    fn assign_levels_to_code_points_removed_by_x9(&mut self, original_classes: &[BidiClass], resolved_levels: &mut [i8]) {
        // Redundant?
        if !self.has_isolates && !self.has_embeddings {
            return;
        }

        // No-op?
        if self.working_classes.is_empty() {
            return;
        }

        // Fix up first character
        if resolved_levels[0] < 0 {
            resolved_levels[0] = self.paragraph_embedding_level;
        }

        if is_removed_by_x9(original_classes[0]) {
            self.working_classes[0] = original_classes[0];
        }

        for i in 1..self.working_classes.len() {
            let original_class = original_classes[i];

            if is_removed_by_x9(original_class) {
                self.working_classes[i] = original_class;
                resolved_levels[i] = resolved_levels[i - 1];
            }
        }
    }

    /// Resets the bidi algorithm to a clean state.
    pub fn reset(&mut self) {
        if self.has_clean_state {
            return;
        }

        self.has_brackets = false;
        self.has_embeddings = false;
        self.has_isolates = false;

        // 16 bytes per entry, like the key/value pairs upstream counts.
        if self.isolate_pairs.capacity().saturating_mul(std::mem::size_of::<(usize, usize)>()) > 1024 * 1024 {
            self.isolate_pairs = HashMap::new();
        } else {
            self.isolate_pairs.clear();
        }

        clear_then_reset_if_too_large(&mut self.working_classes);
        clear_then_reset_if_too_large(&mut self.resolved_levels);
        self.paragraph_embedding_level = 0;
        clear_then_reset_if_too_large(&mut self.status_stack);
        clear_then_reset_if_too_large(&mut self.x9_map);
        clear_then_reset_if_too_large(&mut self.level_runs);
        clear_then_reset_if_too_large(&mut self.isolated_run_mapping);
        clear_then_reset_if_too_large(&mut self.pending_isolate_openings);
        clear_then_reset_if_too_large(&mut self.pending_opening_brackets);
        clear_then_reset_if_too_large(&mut self.paired_brackets);

        self.has_clean_state = true;
    }
}

/// The isolating run sequence currently being processed: views of the
/// per-codepoint tables through the run's index mapping (the mapped array
/// slices and the run level/direction/length fields upstream).
struct IsolatedRun<'a> {
    /// Mapping for the isolating sequence onto the underlying data
    map: &'a [usize],

    /// The resolved (working) types
    resolved_classes: &'a mut [BidiClass],

    /// The original types
    original_classes: &'a mut [BidiClass],

    /// The resolved levels
    levels: &'a mut [i8],

    /// The paired bracket types
    paired_bracket_types: &'a [BidiPairedBracketType],

    /// The paired bracket values
    paired_bracket_values: &'a [i32],

    /// The level of the isolating run
    level: i32,

    /// The direction of the isolating run
    direction: BidiClass,

    /// The length of the isolating run
    length: usize,
}

impl IsolatedRun<'_> {
    #[inline(always)]
    fn resolved_class(&self, index: usize) -> BidiClass {
        self.resolved_classes[self.map[index]]
    }

    #[inline(always)]
    fn set_resolved_class(&mut self, index: usize, value: BidiClass) {
        self.resolved_classes[self.map[index]] = value;
    }

    #[inline(always)]
    fn original_class(&self, index: usize) -> BidiClass {
        self.original_classes[self.map[index]]
    }

    #[inline(always)]
    fn set_original_class(&mut self, index: usize, value: BidiClass) {
        self.original_classes[self.map[index]] = value;
    }

    #[inline(always)]
    fn level_mut(&mut self, index: usize) -> &mut i8 {
        &mut self.levels[self.map[index]]
    }

    #[inline(always)]
    fn paired_bracket_type(&self, index: usize) -> BidiPairedBracketType {
        self.paired_bracket_types[self.map[index]]
    }

    #[inline(always)]
    fn paired_bracket_value(&self, index: usize) -> i32 {
        self.paired_bracket_values[self.map[index]]
    }

    /// Locate all pair brackets in the current isolating run
    ///
    /// Leaves a sorted list of BracketPairs in `paired_brackets`
    fn locate_paired_brackets(&self, pending_opening_brackets: &mut Vec<usize>, paired_brackets: &mut Vec<BracketPair>) {
        // Clear work collections
        pending_opening_brackets.clear();
        paired_brackets.clear();

        // Since sorting is expensive if called often
        // and since we will rarely have many
        // items in this list (most paragraphs will only have a handful of bracket
        // pairs - if that), we use a simple linear lookup and insert most of the
        // time.  If there are more that `SORT_LIMIT` paired brackets we abort th
        // linear searching/inserting and sort at the end.
        const SORT_LIMIT: usize = 8;

        // Process all characters in the run, looking for paired brackets
        for i in 0..self.length {
            // Ignore non-neutral characters
            if self.resolved_class(i) != BidiClass::OtherNeutral {
                continue;
            }

            match self.paired_bracket_type(i) {
                BidiPairedBracketType::Open => {
                    if pending_opening_brackets.len() == MAX_PAIRED_BRACKET_DEPTH {
                        break;
                    }

                    pending_opening_brackets.insert(0, i);
                }

                BidiPairedBracketType::Close => {
                    // see if there is a match
                    for j in 0..pending_opening_brackets.len() {
                        if self.paired_bracket_value(i) != self.paired_bracket_value(pending_opening_brackets[j]) {
                            continue;
                        }

                        // Add this paired bracket set
                        let opener = pending_opening_brackets[j];
                        if paired_brackets.len() < SORT_LIMIT {
                            let mut ppi = 0;
                            while ppi < paired_brackets.len() && paired_brackets[ppi].opening_index < opener {
                                ppi += 1;
                            }

                            paired_brackets.insert(ppi, BracketPair::new(opener, i));
                        } else {
                            paired_brackets.push(BracketPair::new(opener, i));
                        }

                        // remove up to and including matched opener
                        pending_opening_brackets.drain(0..=j);
                        break;
                    }
                }

                BidiPairedBracketType::None => {}
            }
        }

        // Is a sort pending?
        if paired_brackets.len() > SORT_LIMIT {
            paired_brackets.sort_unstable_by_key(|pair| pair.opening_index);
        }
    }

    /// Inspect a paired bracket set and determine its strong direction
    ///
    /// * `bracket_pair` - The paired bracket to be inspected
    ///
    /// Returns the direction of the bracket set content
    fn inspect_paired_bracket(&self, bracket_pair: BracketPair) -> BidiClass {
        let direction_from_level = direction_from_level(self.level);
        let mut direction_opposite = BidiClass::OtherNeutral;
        for i in bracket_pair.opening_index + 1..bracket_pair.closing_index {
            let dir = get_strong_class_n0(self.resolved_class(i));
            if dir == BidiClass::OtherNeutral {
                continue;
            }

            if dir == direction_from_level {
                return dir;
            }

            direction_opposite = dir;
        }

        direction_opposite
    }

    /// Look for a strong type before a paired bracket
    ///
    /// * `bracket_pair` - The paired bracket set to be inspected
    /// * `sos` - The sos in case nothing found before the bracket
    ///
    /// Returns the strong direction before the brackets
    fn inspect_before_paired_bracket(&self, bracket_pair: BracketPair, sos: BidiClass) -> BidiClass {
        for i in (0..bracket_pair.opening_index).rev() {
            let direction = get_strong_class_n0(self.resolved_class(i));
            if direction != BidiClass::OtherNeutral {
                return direction;
            }
        }

        sos
    }

    /// Sets the direction of a bracket pair, including setting the direction of
    /// NSM's inside the brackets and following.
    ///
    /// * `paired_bracket` - The paired brackets
    /// * `direction` - The resolved direction for the bracket pair
    fn set_paired_bracket_direction(&mut self, paired_bracket: BracketPair, direction: BidiClass) {
        // Set the direction of the brackets
        self.set_resolved_class(paired_bracket.opening_index, direction);
        self.set_resolved_class(paired_bracket.closing_index, direction);

        // Set the directionality of NSM's inside the brackets
        // BN  characters (such as ZWJ or ZWSP) that appear between the base bracket character
        // and the nonspacing mark should be ignored.
        for i in paired_bracket.opening_index + 1..paired_bracket.closing_index {
            if self.original_class(i) == BidiClass::NonspacingMark {
                self.set_original_class(i, direction);
            } else if self.original_class(i) != BidiClass::BoundaryNeutral {
                break;
            }
        }

        // Set the directionality of NSM's following the brackets
        for i in paired_bracket.closing_index + 1..self.length {
            if self.original_class(i) == BidiClass::NonspacingMark {
                self.set_original_class(i, direction);
            } else if self.original_class(i) != BidiClass::BoundaryNeutral {
                break;
            }
        }
    }
}

#[inline(always)]
fn is_isolate_start(class: BidiClass) -> bool {
    const MASK: u32 = (1u32 << BidiClass::LeftToRightIsolate as i32)
        | (1u32 << BidiClass::RightToLeftIsolate as i32)
        | (1u32 << BidiClass::FirstStrongIsolate as i32);
    ((1u32 << class as i32) & MASK) != 0
}

/// Check if a directionality type represents whitespace
#[inline(always)]
fn is_whitespace(bi_di_class: BidiClass) -> bool {
    const MASK: u32 = (1u32 << BidiClass::LeftToRightEmbedding as i32)
        | (1u32 << BidiClass::RightToLeftEmbedding as i32)
        | (1u32 << BidiClass::LeftToRightOverride as i32)
        | (1u32 << BidiClass::RightToLeftOverride as i32)
        | (1u32 << BidiClass::PopDirectionalFormat as i32)
        | (1u32 << BidiClass::LeftToRightIsolate as i32)
        | (1u32 << BidiClass::RightToLeftIsolate as i32)
        | (1u32 << BidiClass::FirstStrongIsolate as i32)
        | (1u32 << BidiClass::PopDirectionalIsolate as i32)
        | (1u32 << BidiClass::BoundaryNeutral as i32)
        | (1u32 << BidiClass::WhiteSpace as i32);
    ((1u32 << bi_di_class as i32) & MASK) != 0
}

/// Convert a level to a direction where odd is RTL and
/// even is LTR
///
/// * `level` - The level to convert
///
/// Returns a directionality
#[inline(always)]
fn direction_from_level(level: i32) -> BidiClass {
    if (level & 0x1) == 0 {
        BidiClass::LeftToRight
    } else {
        BidiClass::RightToLeft
    }
}

/// Helper to check if a directionality is removed by rule X9
///
/// * `bi_di_class` - The bidi type to check
///
/// Returns true if rule X9 would remove this character; otherwise false
#[inline(always)]
fn is_removed_by_x9(bi_di_class: BidiClass) -> bool {
    const MASK: u32 = (1u32 << BidiClass::LeftToRightEmbedding as i32)
        | (1u32 << BidiClass::RightToLeftEmbedding as i32)
        | (1u32 << BidiClass::LeftToRightOverride as i32)
        | (1u32 << BidiClass::RightToLeftOverride as i32)
        | (1u32 << BidiClass::PopDirectionalFormat as i32)
        | (1u32 << BidiClass::BoundaryNeutral as i32);
    ((1u32 << bi_di_class as i32) & MASK) != 0
}

/// Check if a directionality is neutral for rules N1 and N2
#[inline(always)]
fn is_neutral_class(direction: BidiClass) -> bool {
    const MASK: u32 = (1u32 << BidiClass::ParagraphSeparator as i32)
        | (1u32 << BidiClass::SegmentSeparator as i32)
        | (1u32 << BidiClass::WhiteSpace as i32)
        | (1u32 << BidiClass::OtherNeutral as i32)
        | (1u32 << BidiClass::RightToLeftIsolate as i32)
        | (1u32 << BidiClass::LeftToRightIsolate as i32)
        | (1u32 << BidiClass::FirstStrongIsolate as i32)
        | (1u32 << BidiClass::PopDirectionalIsolate as i32);
    ((1u32 << direction as i32) & MASK) != 0
}

/// Maps a direction to a strong class for rule N0
///
/// * `direction` - The direction to map
///
/// Returns a strong direction - R, L or ON
#[inline(always)]
fn get_strong_class_n0(direction: BidiClass) -> BidiClass {
    match direction {
        BidiClass::EuropeanNumber | BidiClass::ArabicNumber | BidiClass::ArabicLetter | BidiClass::RightToLeft => {
            BidiClass::RightToLeft
        }
        BidiClass::LeftToRight => BidiClass::LeftToRight,
        _ => BidiClass::OtherNeutral,
    }
}

/// Hold the start and end index of a pair of brackets
#[derive(Clone, Copy, Debug)]
struct BracketPair {
    /// The index of the opening bracket
    opening_index: usize,

    /// The index of the closing bracket
    closing_index: usize,
}

impl BracketPair {
    /// Initializes a new instance of the [`BracketPair`] struct.
    ///
    /// * `opening_index` - Index of the opening bracket
    /// * `closing_index` - Index of the closing bracket
    #[inline]
    fn new(opening_index: usize, closing_index: usize) -> Self {
        Self { opening_index, closing_index }
    }
}

/// Status stack entry used while resolving explicit
/// embedding levels
#[derive(Clone, Copy, Debug)]
struct Status {
    embedding_level: i8,
    override_status: BidiClass,
    isolate_status: bool,
}

impl Status {
    #[inline]
    fn new(embedding_level: i8, override_status: BidiClass, isolate_status: bool) -> Self {
        Self { embedding_level, override_status, isolate_status }
    }
}

/// Provides information about a level run - a continuous
/// sequence of equal levels.
#[derive(Clone, Copy, Debug)]
struct LevelRun {
    start: usize,
    length: usize,
    level: i32,
    sos: BidiClass,
    eos: BidiClass,
}

impl LevelRun {
    #[inline]
    fn new(start: usize, length: usize, level: i32, sos: BidiClass, eos: BidiClass) -> Self {
        Self { start, length, level, sos, eos }
    }
}
