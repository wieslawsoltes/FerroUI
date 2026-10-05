/// Helper for looking up unicode character class information.
///
/// This file contains only the packing-layout constants; the trie-backed
/// lookup helpers live in `unicode_data_lookups.rs`. The split mirrors the
/// upstream one, where the generator of the tables consumes the constants
/// without needing the generated tries (which it produces).
pub struct UnicodeData;

// The constants are `int` upstream; they are only ever combined with the
// unsigned trie values, so they are `u32` here.
#[allow(dead_code)] // the *_BITS constants only document the layout at runtime
impl UnicodeData {
    // ── UnicodeDataTrie field layout ──────────────────────────────────────────────────
    //   bits  0– 5: GeneralCategory  (6)
    //   bits  6–13: Script             (8)
    //   bits 14–20: ScriptExtensions   (7)
    pub(crate) const CATEGORY_BITS: u32 = 6;
    pub(crate) const SCRIPT_BITS: u32 = 8;
    pub(crate) const SCRIPTEXTENSIONS_BITS: u32 = 7;

    pub(crate) const SCRIPT_SHIFT: u32 = Self::CATEGORY_BITS;
    pub(crate) const SCRIPTEXTENSIONS_SHIFT: u32 = Self::CATEGORY_BITS + Self::SCRIPT_BITS;

    pub(crate) const CATEGORY_MASK: u32 = (1 << Self::CATEGORY_BITS) - 1;
    pub(crate) const SCRIPT_MASK: u32 = (1 << Self::SCRIPT_BITS) - 1;
    pub(crate) const SCRIPTEXTENSIONS_MASK: u32 = (1 << Self::SCRIPTEXTENSIONS_BITS) - 1;

    // ── SegmentationTrie field layout ──────────────────────────────────────────────
    //   bits  0– 4: GraphemeBreak      (5)
    //   bits  5– 6: IndicConjunctBreak (2)
    //   bits  7–11: WordBreak          (5)
    //   bits 12–17: LineBreak          (6)
    //   bits 18–22: SentenceBreak      (5)
    //   bit     23: Emoji
    //   bit     24: Emoji_Presentation
    //   bit     25: Default_Ignorable_Code_Point
    pub(crate) const GRAPHEMEBREAK_BITS: u32 = 5;
    pub(crate) const INDICCONJUNCTBREAK_BITS: u32 = 2;
    pub(crate) const WORDBREAK_BITS: u32 = 5;
    pub(crate) const LINEBREAK_BITS: u32 = 6;
    pub(crate) const SENTENCEBREAK_BITS: u32 = 5;

    pub(crate) const GRAPHEMEBREAK_SHIFT: u32 = 0;
    pub(crate) const INDICCONJUNCTBREAK_SHIFT: u32 = Self::GRAPHEMEBREAK_BITS;
    pub(crate) const WORDBREAK_SHIFT: u32 = Self::GRAPHEMEBREAK_BITS + Self::INDICCONJUNCTBREAK_BITS;
    pub(crate) const LINEBREAK_SHIFT: u32 =
        Self::GRAPHEMEBREAK_BITS + Self::INDICCONJUNCTBREAK_BITS + Self::WORDBREAK_BITS;
    pub(crate) const SENTENCEBREAK_SHIFT: u32 =
        Self::GRAPHEMEBREAK_BITS + Self::INDICCONJUNCTBREAK_BITS + Self::WORDBREAK_BITS + Self::LINEBREAK_BITS;

    pub(crate) const GRAPHEMEBREAK_MASK: u32 = (1 << Self::GRAPHEMEBREAK_BITS) - 1;
    pub(crate) const INDICCONJUNCTBREAK_MASK: u32 = (1 << Self::INDICCONJUNCTBREAK_BITS) - 1;
    pub(crate) const WORDBREAK_MASK: u32 = (1 << Self::WORDBREAK_BITS) - 1;
    pub(crate) const LINEBREAK_MASK: u32 = (1 << Self::LINEBREAK_BITS) - 1;
    pub(crate) const SENTENCEBREAK_MASK: u32 = (1 << Self::SENTENCEBREAK_BITS) - 1;

    // Single-bit properties packed into the spare bits of the segmentation word. Emoji and
    // Emoji_Presentation come from the same emoji-data.txt the grapheme break data is read from.
    pub(crate) const EMOJI_SHIFT: u32 = Self::SENTENCEBREAK_SHIFT + Self::SENTENCEBREAK_BITS;
    pub(crate) const EMOJIPRESENTATION_SHIFT: u32 = Self::EMOJI_SHIFT + 1;
    pub(crate) const DEFAULTIGNORABLE_SHIFT: u32 = Self::EMOJIPRESENTATION_SHIFT + 1;

    pub(crate) const EMOJI_FLAG: u32 = 1 << Self::EMOJI_SHIFT;
    pub(crate) const EMOJIPRESENTATION_FLAG: u32 = 1 << Self::EMOJIPRESENTATION_SHIFT;
    pub(crate) const DEFAULTIGNORABLE_FLAG: u32 = 1 << Self::DEFAULTIGNORABLE_SHIFT;

    // ── BiDiTrie field layout ─────────────────────────────────────────────────────────
    //   bits  0–15: BiDiPairedBracket codepoint (16)
    //   bits 16–17: BiDiPairedBracketType       (2)
    //   bits 18–22: BiDiClass                   (5)
    pub(crate) const BIDIPAIREDBRACKED_BITS: u32 = 16;
    pub(crate) const BIDIPAIREDBRACKEDTYPE_BITS: u32 = 2;
    pub(crate) const BIDICLASS_BITS: u32 = 5;

    pub(crate) const BIDIPAIREDBRACKEDTYPE_SHIFT: u32 = Self::BIDIPAIREDBRACKED_BITS;
    pub(crate) const BIDICLASS_SHIFT: u32 = Self::BIDIPAIREDBRACKED_BITS + Self::BIDIPAIREDBRACKEDTYPE_BITS;

    pub(crate) const BIDIPAIREDBRACKED_MASK: u32 = (1 << Self::BIDIPAIREDBRACKED_BITS) - 1;
    pub(crate) const BIDIPAIREDBRACKEDTYPE_MASK: u32 = (1 << Self::BIDIPAIREDBRACKEDTYPE_BITS) - 1;
    pub(crate) const BIDICLASS_MASK: u32 = (1 << Self::BIDICLASS_BITS) - 1;
}
