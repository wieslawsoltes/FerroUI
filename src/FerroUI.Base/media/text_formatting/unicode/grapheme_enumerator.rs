// This source file is adapted from the .NET cross-platform runtime project.
// (https://github.com/dotnet/runtime/)
//
// Licensed under the MIT License, courtesy of The .NET Foundation.

use super::codepoint::Codepoint;
use super::grapheme::Grapheme;
use super::grapheme_break_class::GraphemeBreakClass;
use super::indic_conjunct_break_class::IndicConjunctBreakClass;
use super::unicode_data::UnicodeData;

/// Enumerates the grapheme clusters of UTF-16 text.
#[derive(Clone, Debug)]
pub struct GraphemeEnumerator<'a> {
    text: &'a [u16],
    current_code_unit_offset: usize,
    code_unit_length_of_current_codepoint: usize,
    current_codepoint: Codepoint,
    current_indic_conjunct_break_type: IndicConjunctBreakClass,

    /// Will be [`GraphemeBreakClass::Other`] if invalid data or EOF reached.
    /// Caller shouldn't need to special-case this since the normal rules will halt on this condition.
    current_type: GraphemeBreakClass,
}

/// The GB9c state: only codepoints already consumed into the current cluster
/// can make a following InCB=Consonant join instead of starting a new cluster.
#[derive(Clone, Copy)]
struct IndicConjunctState {
    has_linker: bool,
    has_base: bool,
}

impl IndicConjunctState {
    #[inline(always)]
    fn consume(&mut self, indic_conjunct_break_type: IndicConjunctBreakClass) {
        match indic_conjunct_break_type {
            IndicConjunctBreakClass::Consonant => {
                self.has_base = true;
                self.has_linker = false;
            }
            IndicConjunctBreakClass::Linker => {
                if self.has_base {
                    self.has_linker = true;
                }
            }
            IndicConjunctBreakClass::Extend => {}
            _ => {
                self.has_base = false;
                self.has_linker = false;
            }
        }
    }
}

impl<'a> GraphemeEnumerator<'a> {
    #[inline]
    pub const fn new(text: &'a [u16]) -> Self {
        Self {
            text,
            current_code_unit_offset: 0,
            code_unit_length_of_current_codepoint: 0,
            current_codepoint: Codepoint::REPLACEMENT_CODEPOINT,
            current_indic_conjunct_break_type: IndicConjunctBreakClass::None,
            current_type: GraphemeBreakClass::Other,
        }
    }

    /// Moves to the next [`Grapheme`].
    ///
    /// Returns `None` at the end of the text.
    pub fn move_next(&mut self) -> Option<Grapheme> {
        let start_offset = self.current_code_unit_offset;

        if start_offset >= self.text.len() {
            return None;
        }

        // Algorithm given at https://www.unicode.org/reports/tr29/#Grapheme_Cluster_Boundary_Rules.

        if start_offset == 0 {
            self.read_next_codepoint();
        }

        let first_codepoint = self.current_codepoint;

        // The block is left early where upstream jumps to its `Return` label.
        'rules: {
            // First, consume as many Prepend scalars as we can (rule GB9b).
            if self.current_type == GraphemeBreakClass::Prepend {
                loop {
                    self.read_next_codepoint();
                    if self.current_type != GraphemeBreakClass::Prepend {
                        break;
                    }
                }

                // There were only Prepend scalars in the text
                if self.current_code_unit_offset >= self.text.len() {
                    break 'rules;
                }
            }

            // Next, make sure we're not about to violate control character restrictions.
            // Essentially, if we saw Prepend data, we can't have Control | CR | LF data afterward (rule GB5).
            if self.current_code_unit_offset > start_offset {
                const CONTROL_CR_LF_MASK: u32 = (1u32 << GraphemeBreakClass::Control as i32)
                    | (1u32 << GraphemeBreakClass::CR as i32)
                    | (1u32 << GraphemeBreakClass::LF as i32);

                if ((1u32 << self.current_type as i32) & CONTROL_CR_LF_MASK) != 0 {
                    break 'rules;
                }
            }

            // Now begin the main state machine.

            let mut indic = IndicConjunctState { has_linker: false, has_base: false };

            indic.consume(self.current_indic_conjunct_break_type);

            let mut previous_cluster_break_type = self.current_type;

            self.read_next_codepoint();

            // Each iteration is one `case` of the upstream switch; `goto case X`
            // becomes assigning X and looping, `break` leaves the loop.
            loop {
                match previous_cluster_break_type {
                    GraphemeBreakClass::CR => {
                        if self.current_type != GraphemeBreakClass::LF {
                            break 'rules; // rules GB3 & GB4 (only <LF> can follow <CR>)
                        }

                        self.read_next_codepoint();
                        previous_cluster_break_type = GraphemeBreakClass::LF;
                    }

                    GraphemeBreakClass::Control | GraphemeBreakClass::LF => {
                        break 'rules; // rule GB4 (no data after Control | LF)
                    }

                    GraphemeBreakClass::L => {
                        if self.current_type == GraphemeBreakClass::L {
                            self.read_next_codepoint(); // rule GB6 (L x L)
                            previous_cluster_break_type = GraphemeBreakClass::L;
                        } else if self.current_type == GraphemeBreakClass::V {
                            self.read_next_codepoint(); // rule GB6 (L x V)
                            previous_cluster_break_type = GraphemeBreakClass::V;
                        } else if self.current_type == GraphemeBreakClass::LV {
                            self.read_next_codepoint(); // rule GB6 (L x LV)
                            previous_cluster_break_type = GraphemeBreakClass::LV;
                        } else if self.current_type == GraphemeBreakClass::LVT {
                            self.read_next_codepoint(); // rule GB6 (L x LVT)
                            previous_cluster_break_type = GraphemeBreakClass::LVT;
                        } else {
                            break;
                        }
                    }

                    GraphemeBreakClass::LV | GraphemeBreakClass::V => {
                        if self.current_type == GraphemeBreakClass::V {
                            self.read_next_codepoint(); // rule GB7 (LV | V x V)
                            previous_cluster_break_type = GraphemeBreakClass::V;
                        } else if self.current_type == GraphemeBreakClass::T {
                            self.read_next_codepoint(); // rule GB7 (LV | V x T)
                            previous_cluster_break_type = GraphemeBreakClass::T;
                        } else {
                            break;
                        }
                    }

                    GraphemeBreakClass::LVT | GraphemeBreakClass::T => {
                        if self.current_type == GraphemeBreakClass::T {
                            self.read_next_codepoint(); // rule GB8 (LVT | T x T)
                            previous_cluster_break_type = GraphemeBreakClass::T;
                        } else {
                            break;
                        }
                    }

                    GraphemeBreakClass::ExtendedPictographic => {
                        // Attempt processing extended pictographic (rules GB11, GB9).
                        // First, drain any Extend scalars that might exist
                        while self.current_type == GraphemeBreakClass::Extend {
                            self.read_next_codepoint();
                        }

                        // Now see if there's a ZWJ + extended pictograph again.
                        if self.current_type != GraphemeBreakClass::ZWJ {
                            break;
                        }

                        self.read_next_codepoint();
                        if self.current_type != GraphemeBreakClass::ExtendedPictographic {
                            break;
                        }

                        self.read_next_codepoint();
                        previous_cluster_break_type = GraphemeBreakClass::ExtendedPictographic;
                    }

                    GraphemeBreakClass::RegionalIndicator => {
                        // We've consumed a single RI scalar. Try to consume another (to make it a pair).

                        if self.current_type == GraphemeBreakClass::RegionalIndicator {
                            self.read_next_codepoint();
                        }

                        // Standlone RI scalars (or a single pair of RI scalars) can only be followed by trailers.

                        break; // nothing but trailers after the final RI
                    }

                    _ => break,
                }
            }

            const GB9_MASK: u32 = (1u32 << GraphemeBreakClass::Extend as i32)
                | (1u32 << GraphemeBreakClass::ZWJ as i32)
                | (1u32 << GraphemeBreakClass::SpacingMark as i32);

            // rules GB9, GB9a
            // Keep trailers with the current cluster, and feed them into the GB9c
            // state so InCB=Extend stays transparent between consonants and linkers.
            while ((1u32 << self.current_type as i32) & GB9_MASK) != 0 {
                indic.consume(self.current_indic_conjunct_break_type);
                self.read_next_codepoint();
            }

            // GB9c keeps Indic conjunct clusters together once a consonant has been
            // followed by a linker, with any GB9 extenders allowed on both sides.
            while indic.has_base
                && indic.has_linker
                && self.current_indic_conjunct_break_type == IndicConjunctBreakClass::Consonant
            {
                indic.consume(self.current_indic_conjunct_break_type);
                self.read_next_codepoint();

                while ((1u32 << self.current_type as i32) & GB9_MASK) != 0 {
                    indic.consume(self.current_indic_conjunct_break_type);
                    self.read_next_codepoint();
                }
            }
        }

        let grapheme_length = self.current_code_unit_offset - start_offset;

        Some(Grapheme::new(first_codepoint, start_offset, grapheme_length)) // rules GB2, GB999
    }

    fn read_next_codepoint(&mut self) {
        // For ill-formed subsequences (like unpaired UTF-16 surrogate code points), we rely on
        // the decoder's default behavior of interpreting these ill-formed subsequences as
        // equivalent to U+FFFD REPLACEMENT CHARACTER. This code point has a boundary property
        // of Other (XX), which matches the modifications made to UAX#29, Rev. 35.
        // See: https://www.unicode.org/reports/tr29/tr29-35.html#Modifications
        // This change is also reflected in the UCD files. For example, Unicode 11.0's UCD file
        // https://www.unicode.org/Public/11.0.0/ucd/auxiliary/GraphemeBreakProperty.txt
        // has the line "D800..DFFF    ; Control # Cs [2048] <surrogate-D800>..<surrogate-DFFF>",
        // but starting with Unicode 12.0 that line has been removed.
        //
        // If a later version of the Unicode Standard further modifies this guidance we should reflect
        // that here.

        self.current_code_unit_offset += self.code_unit_length_of_current_codepoint;

        let (codepoint, count) = Codepoint::read_at(self.text, self.current_code_unit_offset);
        self.current_codepoint = codepoint;
        self.code_unit_length_of_current_codepoint = count;

        self.current_type = self.current_codepoint.grapheme_break_class();
        self.current_indic_conjunct_break_type = UnicodeData::get_indic_conjunct_break_class(self.current_codepoint.value());
    }
}

impl Iterator for GraphemeEnumerator<'_> {
    type Item = Grapheme;

    #[inline]
    fn next(&mut self) -> Option<Grapheme> {
        self.move_next()
    }
}
