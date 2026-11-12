//! The text shaper of the backend: the OpenType shaper of the engine.

use crate::notes;
use crate::typeface::ProGpuTypeface;
use ferroui_base::media::fonts::OpenTypeTag;
use ferroui_base::media::text_formatting::unicode::Codepoint;
use ferroui_base::media::text_formatting::{GlyphInfo, ShapedBuffer, TextShaperOptions};
use ferroui_base::media::{GlyphTypeface, ITextShaperTypeface};
use ferroui_base::platform::ITextShaperImpl;
use ferroui_base::utilities::{current_locale, Locale, ReadOnlyMemory};
use ferroui_base::Vector;
use progpu_native_sys::progpu::*;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::io::Read;
use std::ptr;
use std::rc::Rc;
use std::sync::Arc;

/// What a trailing line break is shaped as: a character the shaper draws as nothing and that is
/// a cluster of its own. (A zero width non joiner would not do: the engine forms its clusters of
/// whole graphemes, and a joiner extends the grapheme before it.)
const ZERO_WIDTH_SPACE: u32 = 0x200B;
const REPLACEMENT_CHARACTER: u32 = 0xFFFD;
/// The characters around the shaped text the shaper is told of: enough for joining scripts.
const CONTEXT_SCALARS: usize = 5;

/// The buffers of a shaping call, reused between calls.
#[derive(Default)]
struct Buffers {
    input: Vec<progpu_native_text_scalar>,
    before: Vec<progpu_native_text_scalar>,
    after: Vec<progpu_native_text_scalar>,
    glyphs: Vec<progpu_native_text_shaping_glyph>,
    scratch: Vec<u64>,
}

ferroui_base::ferro_thread_local! {
    static BUFFERS: RefCell<Buffers> = RefCell::new(Buffers::default());
}

/// The shaping context of the engine for one glyph typeface: its font, parsed once, and the
/// shaping plans made on it.
pub struct ProGpuShaperTypeface {
    context: Cell<*mut progpu_native_text_context>,
    coordinates: Arc<[i16]>,
    units_per_em: f64,
}

impl ProGpuShaperTypeface {
    fn new(glyph_typeface: &GlyphTypeface) -> Self {
        let platform_typeface = glyph_typeface.platform_typeface();
        let units_per_em = f64::from(glyph_typeface.metrics().design_em_height);
        let mut context = ptr::null_mut();
        let mut coordinates: Arc<[i16]> = Arc::from(Vec::new());
        let mut create = |bytes: &[u8], face_index: u32| {
            if bytes.is_empty() {
                return;
            }
            // SAFETY: the bytes are borrowed for the call (the context copies them); the context
            // is written by the call and checked.
            let status = unsafe {
                progpu_native_text_context_create(PROGPU_NATIVE_ABI_VERSION, bytes.as_ptr(), bytes.len(), face_index, ptr::null(), 0, &mut context)
            };
            if status != PROGPU_NATIVE_STATUS_SUCCESS {
                context = ptr::null_mut();
            }
        };
        match ProGpuTypeface::try_get(&**platform_typeface) {
            Some(typeface) => {
                let (bytes, face_index) = typeface.face().data();
                create(bytes, face_index);
                coordinates = typeface.coordinates().clone();
            }
            None => {
                // A typeface of another font manager: the shaper reads the bytes of its font.
                let mut bytes = Vec::new();
                if let Some(mut stream) = platform_typeface.try_get_stream() {
                    if stream.read_to_end(&mut bytes).is_err() {
                        bytes.clear();
                    }
                }
                create(&bytes, 0);
            }
        }
        if context.is_null() {
            notes::failure("the text shaper of ProGPU cannot read the font of a typeface: its text is shaped to nothing");
        }
        ProGpuShaperTypeface { context: Cell::new(context), coordinates, units_per_em }
    }
}

impl ITextShaperTypeface for ProGpuShaperTypeface {
    fn dispose(&self) {
        let context = self.context.replace(ptr::null_mut());
        if !context.is_null() {
            // SAFETY: the context is owned here and released once.
            unsafe { progpu_native_text_context_destroy(context) };
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl Drop for ProGpuShaperTypeface {
    fn drop(&mut self) {
        self.dispose();
    }
}

/// The scalar at `index` of UTF-16 `text` and its length in code units; an unpaired surrogate
/// reads as the replacement character.
fn scalar_at(text: &[u16], index: usize) -> (u32, usize) {
    let unit = u32::from(text[index]);
    if (0xD800..0xDC00).contains(&unit) {
        if let Some(&low) = text.get(index + 1) {
            let low = u32::from(low);
            if (0xDC00..0xE000).contains(&low) {
                return (0x10000 + ((unit - 0xD800) << 10) + (low - 0xDC00), 2);
            }
        }
        return (REPLACEMENT_CHARACTER, 1);
    }
    if (0xDC00..0xE000).contains(&unit) {
        return (REPLACEMENT_CHARACTER, 1);
    }
    (unit, 1)
}

fn scalar(code_point: u32, input_index: usize, input_length: usize) -> progpu_native_text_scalar {
    progpu_native_text_scalar {
        code_point,
        input_index: input_index as u32,
        input_length: input_length as u16,
        canonical_combining_class: 0,
        reserved: 0,
        script: 0,
    }
}

/// The scalars of `text[start..end]`, their input indices counted from `origin`.
fn push_scalars(scalars: &mut Vec<progpu_native_text_scalar>, text: &[u16], start: usize, end: usize, origin: usize) {
    let mut index = start;
    while index < end {
        let (code_point, length) = scalar_at(&text[..end], index);
        scalars.push(scalar(code_point, index - origin, length));
        index += length;
    }
}

/// Replaces a trailing line break by a zero width space so that it shapes to nothing; a `\r\n`
/// pair becomes one scalar, one cluster.
fn merge_break_pair(scalars: &mut Vec<progpu_native_text_scalar>) {
    let Some(last) = scalars.last().copied() else {
        return;
    };
    if !Codepoint::new(last.code_point).is_break_char() {
        return;
    }
    let length = scalars.len();
    if length > 1 && scalars[length - 2].code_point == '\r' as u32 && last.code_point == '\n' as u32 {
        scalars.pop();
        let first = &mut scalars[length - 2];
        first.code_point = ZERO_WIDTH_SPACE;
        first.input_length += last.input_length;
    } else {
        scalars[length - 1].code_point = ZERO_WIDTH_SPACE;
    }
}

/// The OpenType language tag of a culture: its primary language, resolved by the engine.
fn language_tag(culture: &Locale) -> u32 {
    if *culture == Locale::UNKNOWN {
        return 0;
    }
    let language = culture.id.language.as_str();
    let mut tag = 0u32;
    // SAFETY: the language is borrowed for the call; the tag is written by it.
    let status = unsafe { progpu_native_text_resolve_language_tag(language.as_ptr().cast(), language.len(), &mut tag) };
    if status == PROGPU_NATIVE_STATUS_SUCCESS {
        tag
    } else {
        0
    }
}

fn features(options: &TextShaperOptions) -> Vec<progpu_native_text_feature> {
    let Some(font_features) = options.font_features() else {
        return Vec::new();
    };
    font_features
        .iter()
        .map(|feature| progpu_native_text_feature {
            tag: OpenTypeTag::parse(&feature.tag).value(),
            value: feature.value as u32,
            start: feature.start as u32,
            end: feature.end as u32,
        })
        .collect()
}

/// A text shaper on the OpenType shaper of the engine.
#[derive(Default)]
pub struct ProGpuTextShaper;

impl ProGpuTextShaper {
    /// Creates the text shaper.
    pub fn new() -> Self {
        Self
    }
}

impl ITextShaperImpl for ProGpuTextShaper {
    fn shape_text(&self, text: &ReadOnlyMemory<u16>, options: &TextShaperOptions) -> Rc<ShapedBuffer> {
        let glyph_typeface = options.glyph_typeface();
        let font_rendering_em_size = options.font_rendering_em_size();
        let bidi_level = options.bidi_level();
        let empty = || ShapedBuffer::new(text.clone(), 0, glyph_typeface.clone(), font_rendering_em_size, bidi_level);

        if text.is_empty() {
            return empty();
        }

        let text_shaper_typeface = glyph_typeface.text_shaper_typeface();
        let typeface = text_shaper_typeface
            .as_any()
            .downcast_ref::<ProGpuShaperTypeface>()
            .unwrap_or_else(|| panic!("The provided GlyphTypeface is not supported by this text shaper."));
        let context = typeface.context.get();
        if context.is_null() || typeface.units_per_em <= 0.0 {
            return empty();
        }

        // The shaper needs the surrounding characters to shape joining scripts.
        let (containing_text, start): (&[u16], usize) = match text.owner() {
            Some(owner) => (owner, text.offset_in_owner()),
            None => (text.span(), 0),
        };
        let end = start + text.len();

        let current_culture;
        let culture = match options.culture() {
            Some(culture) => culture,
            None => {
                current_culture = current_locale();
                &current_culture
            }
        };
        let features = features(options);

        BUFFERS.with(|buffers| {
            let mut buffers = buffers.borrow_mut();
            let Buffers { input, before, after, glyphs, scratch } = &mut *buffers;

            input.clear();
            push_scalars(input, containing_text, start, end, start);
            merge_break_pair(input);

            // The characters before the text, from the nearest: a few are enough.
            before.clear();
            let mut context_start = start;
            for _ in 0..CONTEXT_SCALARS {
                if context_start == 0 {
                    break;
                }
                context_start -= 1;
                if context_start > 0 && (0xDC00..0xE000).contains(&containing_text[context_start]) && (0xD800..0xDC00).contains(&containing_text[context_start - 1]) {
                    context_start -= 1;
                }
            }
            push_scalars(before, containing_text, context_start, start, context_start);

            after.clear();
            let mut context_end = end;
            for _ in 0..CONTEXT_SCALARS {
                if context_end >= containing_text.len() {
                    break;
                }
                context_end += scalar_at(containing_text, context_end).1;
            }
            push_scalars(after, containing_text, end, context_end, end);

            // SAFETY: plain data; the fields the call reads are set below.
            let mut request: progpu_native_text_shape_request = unsafe { core::mem::zeroed() };
            request.struct_size = core::mem::size_of::<progpu_native_text_shape_request>() as u32;
            request.abi_version = PROGPU_NATIVE_ABI_VERSION;
            request.input = input.as_ptr();
            request.input_count = input.len() as u32;
            if !before.is_empty() {
                request.pre_context = before.as_ptr();
                request.pre_context_count = before.len() as u32;
            }
            if !after.is_empty() {
                request.post_context = after.as_ptr();
                request.post_context_count = after.len() as u32;
            }
            if !features.is_empty() {
                request.features = features.as_ptr();
                request.feature_count = features.len() as u32;
            }
            if !typeface.coordinates.is_empty() {
                request.normalized_coordinates = typeface.coordinates.as_ptr();
                request.normalized_coordinate_count = typeface.coordinates.len() as u32;
            }
            request.language = language_tag(culture);
            request.direction = if (bidi_level & 1) == 0 { PROGPU_NATIVE_TEXT_DIRECTION_LEFT_TO_RIGHT } else { PROGPU_NATIVE_TEXT_DIRECTION_RIGHT_TO_LEFT } as u32;
            request.cluster_level = PROGPU_NATIVE_TEXT_CLUSTER_MONOTONE_GRAPHEMES as u32;

            // SAFETY: plain data, sized for the call.
            let mut requirements: progpu_native_text_shape_requirements = unsafe { core::mem::zeroed() };
            requirements.struct_size = core::mem::size_of::<progpu_native_text_shape_requirements>() as u32;
            // SAFETY: the request borrows the buffers above for the call.
            if unsafe { progpu_native_text_context_get_shape_requirements(context, &request, &mut requirements) } != PROGPU_NATIVE_STATUS_SUCCESS {
                notes::failure(&format!("the text shaper of ProGPU refused a text (error {})", requirements.error_code));
                return empty();
            }
            let glyph_capacity = requirements.glyph_capacity as usize;
            if glyphs.len() < glyph_capacity {
                // SAFETY: plain data.
                glyphs.resize(glyph_capacity, unsafe { core::mem::zeroed() });
            }
            let scratch_words = (requirements.scratch_bytes as usize).div_ceil(8);
            if scratch.len() < scratch_words {
                scratch.resize(scratch_words, 0);
            }
            // SAFETY: plain data, sized for the call.
            let mut result: progpu_native_text_shape_result = unsafe { core::mem::zeroed() };
            result.struct_size = core::mem::size_of::<progpu_native_text_shape_result>() as u32;
            // SAFETY: the outputs are as large as the requirements say; the scratch is aligned
            // to eight bytes.
            let status = unsafe {
                progpu_native_text_context_shape(
                    context,
                    &request,
                    glyphs.as_mut_ptr(),
                    glyph_capacity as u32,
                    scratch.as_mut_ptr().cast(),
                    scratch_words * 8,
                    &mut result,
                )
            };
            if status != PROGPU_NATIVE_STATUS_SUCCESS {
                notes::failure(&format!("the text shaper of ProGPU failed on a text (error {})", result.error_code));
                return empty();
            }

            let count = result.glyph_count as usize;
            let text_scale = font_rendering_em_size / typeface.units_per_em;
            let shaped_buffer = ShapedBuffer::new(text.clone(), count, glyph_typeface.clone(), font_rendering_em_size, bidi_level);

            for (index, glyph) in glyphs[..count].iter().enumerate() {
                let mut glyph_index = glyph.glyph_id as u16;
                // The cluster is the input index of its first character: an offset in `text`.
                let glyph_cluster = glyph.cluster;
                let mut glyph_advance = f64::from(glyph.advance_x) * text_scale + options.letter_spacing();
                // The shaper gives its offsets with Y down already.
                let glyph_offset = Vector::new(f64::from(glyph.offset_x) * text_scale, f64::from(glyph.offset_y) * text_scale);

                let original = start + glyph_cluster.max(0) as usize;
                if original < containing_text.len() && containing_text[original] == '\t' as u16 {
                    glyph_index = glyph_typeface.character_to_glyph_map().get_glyph(' ' as i32);

                    if options.incremental_tab_width() > 0.0 {
                        glyph_advance = options.incremental_tab_width();
                    } else {
                        let advance = glyph_typeface.try_get_horizontal_glyph_advance(glyph_index).unwrap_or(0);
                        glyph_advance = 4.0 * f64::from(advance) * text_scale;
                    }
                }

                shaped_buffer.set(index, GlyphInfo::with_offset(glyph_index, glyph_cluster, glyph_advance, glyph_offset));
            }

            shaped_buffer
        })
    }

    fn create_typeface(&self, glyph_typeface: &GlyphTypeface) -> Rc<dyn ITextShaperTypeface> {
        Rc::new(ProGpuShaperTypeface::new(glyph_typeface))
    }
}
