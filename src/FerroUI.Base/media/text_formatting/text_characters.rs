use std::rc::Rc;

use crate::logging::{LogArea, LogEventLevel, Logger};
use crate::media::fonts::FontFallbackScriptHints;
use crate::media::text_formatting::text_run::{text_run_any, TextRun};
use crate::media::text_formatting::unicode::{
    Codepoint, CodepointEnumerator, GeneralCategory, GraphemeEnumerator, Script,
};
use crate::media::text_formatting::{TextRunProperties, UnshapedTextRun};
use crate::media::{FontManager, GlyphTypeface, Typeface};
use crate::utilities::ReadOnlyMemory;

// NUL characters render nothing but must keep their position in the run. WORD JOINER (U+2060)
// is a zero-width, default-ignorable, non-breaking filler; unlike ZERO WIDTH SPACE (U+200B)
// it introduces no line-break opportunity, matching NUL's lack of break semantics.
const WORD_JOINER: u16 = 0x2060;
const WORD_JOINER_RUN_LENGTH: usize = 8;

thread_local! {
    static WORD_JOINER_RUN: ReadOnlyMemory<u16> = ReadOnlyMemory::from_vec(vec![WORD_JOINER; WORD_JOINER_RUN_LENGTH]);
}

/// A text run that holds text characters.
pub struct TextCharacters {
    text: ReadOnlyMemory<u16>,
    properties: Rc<dyn TextRunProperties>,
}

impl TextCharacters {
    /// Constructs a run for text content from a string.
    ///
    /// Panics when the font rendering em size of the properties is not positive.
    pub fn from_str(text: &str, text_run_properties: Rc<dyn TextRunProperties>) -> Self {
        Self::new(ReadOnlyMemory::<u16>::from_str(text), text_run_properties)
    }

    /// Constructs a run for text content from a memory region.
    ///
    /// Panics when the font rendering em size of the properties is not positive.
    pub fn new(text: ReadOnlyMemory<u16>, text_run_properties: Rc<dyn TextRunProperties>) -> Self {
        if text_run_properties.font_rendering_em_size() <= 0.0 {
            panic!(
                "Invalid FontRenderingEmSize (Parameter 'textRunProperties')\nActual value was {}.",
                text_run_properties.font_rendering_em_size()
            );
        }

        Self { text, properties: text_run_properties }
    }

    /// The run's properties (never absent for text characters).
    pub fn run_properties(&self) -> &Rc<dyn TextRunProperties> {
        &self.properties
    }

    /// Gets a list of [`UnshapedTextRun`].
    // Internal upstream; public so the Skia unit tests reach it.
    pub fn get_shapeable_characters(
        &self,
        mut text: ReadOnlyMemory<u16>,
        bidi_level: i8,
        font_manager: &FontManager,
        previous_properties: &mut Option<Rc<dyn TextRunProperties>>,
        results: &mut Vec<Rc<dyn TextRun>>,
    ) {
        let properties = &self.properties;

        while !text.is_empty() {
            let shapeable_run =
                Self::create_shapeable_run(&text, properties, bidi_level, font_manager, previous_properties.as_ref());

            text = text.slice_from(shapeable_run.length() as usize);

            // Whitespace says nothing about which font the text around it wants, and it belongs to
            // the default typeface whenever that covers it - so a run of pure whitespace must not
            // become the anti-thrashing bias for what follows. Otherwise the words on either side
            // of a space each resolve their fallback from scratch and can land on different fonts.
            if !Self::is_white_space_only(shapeable_run.text_span()) {
                *previous_properties = shapeable_run.properties().cloned();
            }

            results.push(Rc::new(shapeable_run));
        }
    }

    /// Returns whether every codepoint in `text` is whitespace. Returns on the
    /// first codepoint that isn't, so a run of text costs a single lookup.
    fn is_white_space_only(text: &[u16]) -> bool {
        let mut codepoints = CodepointEnumerator::new(text);

        while let Some(codepoint) = codepoints.move_next() {
            if !codepoint.is_white_space() {
                return false;
            }
        }

        true
    }

    /// Creates a shapeable text run with unique properties.
    fn create_shapeable_run(
        text: &ReadOnlyMemory<u16>,
        default_properties: &Rc<dyn TextRunProperties>,
        bidi_level: i8,
        font_manager: &FontManager,
        previous_properties: Option<&Rc<dyn TextRunProperties>>,
    ) -> UnshapedTextRun {
        let default_typeface = default_properties.typeface();
        let default_glyph_typeface = default_properties.cached_glyph_typeface();
        let previous_typeface = previous_properties.map(|properties| properties.typeface());
        let previous_glyph_typeface = previous_properties.map(|properties| properties.cached_glyph_typeface());
        let text_span = text.span();

        let mut count = 0usize;
        let mut codepoints = CodepointEnumerator::new(text_span);

        while let Some(first_codepoint) = codepoints.move_next() {
            if first_codepoint.value() != 0 {
                break;
            }

            count += 1;
        }

        // Detect null terminator
        if count > 0 {
            // Reuse a cached run of WORD JOINERs for the common short case to avoid an allocation.
            let null_replacement = if count <= WORD_JOINER_RUN_LENGTH {
                WORD_JOINER_RUN.with(|run| run.slice(0, count))
            } else {
                ReadOnlyMemory::from_vec(vec![WORD_JOINER; count])
            };

            return UnshapedTextRun::new(null_replacement, default_properties.clone(), bidi_level);
        }

        // The first scalar's script drives both the locale-sensitivity check and the complex-script
        // capability preference below.
        let first_script = Codepoint::read_at(text_span, 0).0.script();

        // The previous run's font is reused as a cheap anti-thrashing bias, but it bypasses the
        // culture-aware fallback scorer. For locale-sensitive scripts (CJK Han unification) skip
        // the reuse when the culture changed between runs so the culture-scored fallback runs
        // instead - otherwise e.g. a zh run's font would pin ja text. Same-culture runs keep the
        // reuse, so the common case pays only a culture comparison.
        let mut allow_previous_typeface = true;

        if let Some(previous_properties) = previous_properties {
            if previous_properties.culture_info() != default_properties.culture_info()
                && FontFallbackScriptHints::is_locale_sensitive(first_script)
            {
                allow_previous_typeface = false;
            }
        }

        // Capability preference (Strategy A): for a complex script, first try fonts that declare
        // the script in GSUB/GPOS (so they can actually shape it), then fall back to cmap-only
        // coverage. Simple scripts use a single cmap tier, so the common path is unchanged.
        let capability_tiers =
            if FontFallbackScriptHints::try_get_complex_shaping_tags(first_script).is_some() { 2 } else { 1 };

        for capability_tier in 0..capability_tiers {
            // Tier 0 (only present when there are two tiers) requires the font to declare shaping
            // support for the script; tier 1 accepts cmap coverage alone (the historical gate).
            let require_shaping_capability = capability_tiers == 2 && capability_tier == 0;

            // When this tier requires shaping capability, constrain the fallback search to fonts
            // that can shape the script (Script::Unknown = the historical, unconstrained search).
            let shaping_constraint = if require_shaping_capability { first_script } else { Script::Unknown };

            // Coverage tiers (full cluster, then base-only) run inside each capability tier. The
            // fallback is resolved once per capability tier because the search constraint differs.
            let mut fallback_typeface: Option<Typeface> = None;
            let mut fallback_glyph_typeface: Option<Rc<GlyphTypeface>> = None;
            let mut fallback_resolved = false;

            // A primary that cannot shape this tier's script is not a valid "return target" for
            // text: handing clusters back to it would block a shaping-capable fallback that merely
            // shares the primary's cmap. It still reclaims the spacing whitespace between the
            // words, which needs no shaping - see try_get_shapeable_length.
            let default_can_shape =
                !require_shaping_capability || default_glyph_typeface.can_shape_script(first_script);

            for pass in 0..2 {
                let require_full_cluster = pass == 0;

                if default_can_shape {
                    if let Some(count) = Self::try_get_shapeable_length(
                        text_span,
                        &default_glyph_typeface,
                        None,
                        false,
                        require_full_cluster,
                    ) {
                        // Primary font: the properties already carry this typeface, so reuse them
                        // directly. This avoids a needless copy and preserves a custom
                        // TextRunProperties implementation that with_typeface would otherwise flatten.
                        return UnshapedTextRun::new(text.slice(0, count), default_properties.clone(), bidi_level);
                    }
                }

                if allow_previous_typeface {
                    if let (Some(previous_glyph_typeface), Some(previous_typeface)) =
                        (&previous_glyph_typeface, previous_typeface)
                    {
                        if !require_shaping_capability || previous_glyph_typeface.can_shape_script(first_script) {
                            if let Some(count) = Self::try_get_shapeable_length(
                                text_span,
                                previous_glyph_typeface,
                                Some(&default_glyph_typeface),
                                default_can_shape,
                                require_full_cluster,
                            ) {
                                return UnshapedTextRun::new(
                                    text.slice(0, count),
                                    default_properties.with_typeface(previous_typeface),
                                    bidi_level,
                                );
                            }
                        }
                    }
                }

                // Resolve the fallback once, after the primary/previous probes fail. It is keyed on
                // the first scalar the primary font cannot render - the base for an unsupported
                // script, or the combining mark for an otherwise-supported cluster - so the search
                // can find a font for the mark, not just the base. Reused by every pass.
                if !fallback_resolved {
                    fallback_resolved = true;

                    let fallback_codepoint = Self::get_fallback_codepoint(text_span, &default_glyph_typeface);

                    fallback_typeface = font_manager.try_match_character_with_script(
                        i32::from(fallback_codepoint),
                        default_typeface.style(),
                        default_typeface.weight(),
                        default_typeface.stretch(),
                        Some(default_typeface.font_family()),
                        default_properties.culture_info(),
                        shaping_constraint,
                    );

                    if let Some(typeface) = &fallback_typeface {
                        fallback_glyph_typeface = font_manager.try_get_glyph_typeface(typeface);

                        if fallback_glyph_typeface.is_none() {
                            // The platform matched a fallback family but its glyph typeface could not
                            // be loaded; the cluster degrades to .notdef. Surface it for diagnosis.
                            if let Some(logger) = Logger::try_get(LogEventLevel::Warning, LogArea::FONTS) {
                                logger.log_with_values(
                                    None,
                                    "Matched fallback typeface {FamilyName} for codepoint U+{Codepoint} but could not load its glyph typeface.",
                                    &[&typeface.font_family().name(), &format_args!("{:04X}", fallback_codepoint.value())],
                                );
                            }
                        }
                    }
                }

                if let (Some(fallback_glyph_typeface), Some(fallback_typeface)) =
                    (&fallback_glyph_typeface, &fallback_typeface)
                {
                    if let Some(count) = Self::try_get_shapeable_length(
                        text_span,
                        fallback_glyph_typeface,
                        Some(&default_glyph_typeface),
                        default_can_shape,
                        require_full_cluster,
                    ) {
                        return UnshapedTextRun::new(
                            text.slice(0, count),
                            default_properties.with_typeface(fallback_typeface),
                            bidi_level,
                        );
                    }
                }
            }
        }

        // No font (not even a last-resort match) covers the first cluster. Coalesce the
        // following clusters that likewise have no home into a single .notdef ("tofu") run,
        // then hand control back so the next run can be selected normally. We must stop as
        // soon as a cluster the primary font - or any fallback - can render is reached;
        // otherwise a renderable cluster following an unmatchable one would be swallowed as
        // tofu too (e.g. a private-use codepoint immediately followed by CJK text).
        let mut count = 0usize;
        let mut enumerator = GraphemeEnumerator::new(text_span);

        while let Some(grapheme) = enumerator.move_next() {
            let first_codepoint = grapheme.first_codepoint();

            if !first_codepoint.is_white_space() {
                // Primary font regained coverage - return to it.
                if default_glyph_typeface.character_to_glyph_map().try_get_glyph(i32::from(first_codepoint)).is_some() {
                    break;
                }

                // A fallback exists for this cluster - stop so the next run can use it. The
                // first cluster is skipped (count == 0): we already know it has no match.
                if count > 0
                    && font_manager
                        .try_match_character(
                            i32::from(first_codepoint),
                            default_typeface.style(),
                            default_typeface.weight(),
                            default_typeface.stretch(),
                            Some(default_typeface.font_family()),
                            default_properties.culture_info(),
                        )
                        .is_some()
                {
                    break;
                }
            }

            count += grapheme.length();
        }

        UnshapedTextRun::new(text.slice(0, count), default_properties.clone(), bidi_level)
    }

    /// Tries to get a shapeable length that is supported by the specified typeface.
    ///
    /// * `text` — the characters to shape.
    /// * `glyph_typeface` — the typeface that is used to find matching characters.
    /// * `default_glyph_typeface` — the default typeface, or `None` when there is none to
    ///   return to (the probe for the default typeface itself).
    /// * `default_can_shape_script` — whether the default typeface can shape this run's script.
    ///   When `false` it only reclaims spacing whitespace, which needs no shaping.
    /// * `require_full_cluster` — when `true`, a grapheme cluster only counts as supported when
    ///   the typeface has a glyph for every scalar it contains (base plus combining marks);
    ///   when `false`, only the base scalar is tested.
    ///
    /// Returns the shapeable length, or `None` when it is zero.
    // Internal upstream; public so the Skia unit tests reach it.
    pub fn try_get_shapeable_length(
        text: &[u16],
        glyph_typeface: &GlyphTypeface,
        default_glyph_typeface: Option<&GlyphTypeface>,
        default_can_shape_script: bool,
        require_full_cluster: bool,
    ) -> Option<usize> {
        let mut length = 0usize;
        let mut script = Script::Unknown;

        if text.is_empty() {
            return None;
        }

        let mut enumerator = GraphemeEnumerator::new(text);

        while let Some(current_grapheme) = enumerator.move_next() {
            let current_codepoint = current_grapheme.first_codepoint();
            let current_script = current_codepoint.script();

            if current_codepoint.value() == 0 {
                // Do not include null terminators
                break;
            }

            let cluster_text = &text[current_grapheme.offset()..current_grapheme.offset() + current_grapheme.length()];

            // A fallback run ends where the default typeface regains coverage, spacing whitespace
            // included - practically every font maps U+0020, so exempting it would let the run
            // shape the following space with the fallback's own advance. A default typeface that
            // cannot shape this script still reclaims that whitespace, which carries no shaping.
            // Only Zs qualifies: control and format codepoints (bidi controls, prepended number
            // signs) keep their cluster with the probed font.
            if let Some(default_glyph_typeface) = default_glyph_typeface {
                if (default_can_shape_script || current_codepoint.general_category() == GeneralCategory::SpaceSeparator)
                    && Self::cluster_is_covered(cluster_text, current_codepoint, default_glyph_typeface, require_full_cluster)
                {
                    break;
                }
            }

            // Stop at the first cluster this typeface can't render. A cluster that only holds a
            // default ignorable (a stray variation selector, say) renders nothing whichever font
            // it lands on, so it never ends the run.
            if Self::needs_glyph(current_codepoint)
                && !Self::cluster_is_covered(cluster_text, current_codepoint, glyph_typeface, require_full_cluster)
            {
                break;
            }

            if current_script != script {
                if script == Script::Unknown
                    || current_script != Script::Common && matches!(script, Script::Common | Script::Inherited)
                {
                    script = current_script;
                } else if current_script != Script::Inherited && current_script != Script::Common {
                    break;
                }
            }

            length += current_grapheme.length();
        }

        (length > 0).then_some(length)
    }

    /// Determines whether the codepoint is one a font is expected to provide a glyph for. Break
    /// chars, control codepoints and default ignorables (variation selectors, joiners, ...) never
    /// render - demanding a glyph for those would reject fonts that cover the cluster's actual
    /// content. Format codepoints are not excluded wholesale: the invisible ones are all default
    /// ignorable, while the rest (prepended concatenation marks like U+0600, interlinear
    /// annotation chars) are visible and do need a glyph.
    fn needs_glyph(codepoint: Codepoint) -> bool {
        !codepoint.is_break_char()
            && codepoint.general_category() != GeneralCategory::Control
            && !codepoint.is_default_ignorable()
    }

    /// Determines whether `glyph_typeface` can render the first grapheme cluster in
    /// `cluster_text`. For a single-scalar cluster, or when `require_full_cluster` is `false`,
    /// only the base scalar is tested. Otherwise every scalar that needs a glyph must be
    /// present, so a base+mark cluster is only covered by a font that has the marks too.
    fn cluster_is_covered(
        cluster_text: &[u16],
        first_codepoint: Codepoint,
        glyph_typeface: &GlyphTypeface,
        require_full_cluster: bool,
    ) -> bool {
        let base_length = if first_codepoint.value() > 0xFFFF { 2 } else { 1 };

        if !require_full_cluster || cluster_text.len() <= base_length {
            return glyph_typeface.character_to_glyph_map().try_get_glyph(i32::from(first_codepoint)).is_some();
        }

        let mut codepoints = CodepointEnumerator::new(cluster_text);

        while let Some(codepoint) = codepoints.move_next() {
            if !Self::needs_glyph(codepoint) {
                continue;
            }

            if glyph_typeface.character_to_glyph_map().try_get_glyph(i32::from(codepoint)).is_none() {
                return false;
            }
        }

        true
    }

    /// Returns the first scalar of the run's first grapheme cluster that
    /// `default_glyph_typeface` cannot render - the base for an unsupported script,
    /// or a combining mark for an otherwise-supported cluster. Keying the fallback search on this
    /// lets it find a font for the mark, not just the base. Falls back to the cluster's first
    /// scalar when every scalar is already covered.
    fn get_fallback_codepoint(text: &[u16], default_glyph_typeface: &GlyphTypeface) -> Codepoint {
        let mut grapheme_enumerator = GraphemeEnumerator::new(text);

        let Some(grapheme) = grapheme_enumerator.move_next() else {
            return Codepoint::REPLACEMENT_CODEPOINT;
        };

        let mut codepoints =
            CodepointEnumerator::new(&text[grapheme.offset()..grapheme.offset() + grapheme.length()]);

        while let Some(codepoint) = codepoints.move_next() {
            if !Self::needs_glyph(codepoint) {
                continue;
            }

            if default_glyph_typeface.character_to_glyph_map().try_get_glyph(i32::from(codepoint)).is_none() {
                return codepoint;
            }
        }

        grapheme.first_codepoint()
    }
}

impl TextRun for TextCharacters {
    fn length(&self) -> i32 {
        self.text.len() as i32
    }

    fn text(&self) -> ReadOnlyMemory<u16> {
        self.text.clone()
    }

    fn text_span(&self) -> &[u16] {
        self.text.span()
    }

    fn properties(&self) -> Option<&Rc<dyn TextRunProperties>> {
        Some(&self.properties)
    }

    text_run_any!();
}
