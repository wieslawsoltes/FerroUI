use crate::ffi::{self, Owned};
use ferroui_base::media::{
    fonts::OpenTypeTag,
    text_formatting::{GlyphInfo, ShapedBuffer, TextShaperOptions},
    *,
};
use ferroui_base::platform::{IFontManagerImpl, ITextShaperImpl};
use ferroui_base::utilities::{Locale, ReadOnlyMemory};
use ferroui_base::Vector;
use std::{any::Any, cell::RefCell, ffi::CString, io::Read, rc::Rc, sync::Arc};

pub struct NativeFont {
    handle: Owned,
    pub name: String,
}
// SAFETY: a CTFont is immutable and Core Text permits concurrent access to font objects.
unsafe impl Sync for NativeFont {}
impl NativeFont {
    /// Takes ownership of a +1 CTFont; `None` for NULL.
    ///
    /// # Safety
    /// `p` is NULL or a +1 CTFont reference that nothing else releases.
    pub unsafe fn from_raw(p: ffi::Ptr) -> Option<Arc<Self>> {
        let handle = Owned::new(p)?;
        let mut name = vec![0; 4096];
        ffi::q_font_name(p, name.as_mut_ptr(), name.len());
        name.truncate(name.iter().position(|b| *b == 0).unwrap_or(name.len()));
        Some(Arc::new(Self {
            handle,
            name: String::from_utf8_lossy(&name).into_owned(),
        }))
    }
    pub fn raw(&self) -> ffi::Ptr {
        self.handle.0
    }
}
pub struct Typeface {
    pub font: Arc<NativeFont>,
    weight: FontWeight,
    style: FontStyle,
    stretch: FontStretch,
    simulations: FontSimulations,
    data: Option<Arc<[u8]>>,
}
impl Typeface {
    fn new(
        font: Arc<NativeFont>,
        weight: FontWeight,
        style: FontStyle,
        stretch: FontStretch,
    ) -> Self {
        Self {
            font,
            weight,
            style,
            stretch,
            simulations: FontSimulations::None,
            data: None,
        }
    }
}
impl IFontMemory for Typeface {
    fn try_get_table(&self, tag: OpenTypeTag) -> Option<ReadOnlyMemory<u8>> {
        // SAFETY: q_font_table returns NULL or a +1 CFData.
        let table = unsafe { Owned::new(ffi::q_font_table(self.font.raw(), tag.value())) }?;
        Some(ReadOnlyMemory::from_vec(table.bytes()))
    }
    fn dispose(&self) {}
}
impl IPlatformTypeface for Typeface {
    fn family_name(&self) -> String {
        self.font.name.clone()
    }
    fn weight(&self) -> FontWeight {
        self.weight
    }
    fn style(&self) -> FontStyle {
        self.style
    }
    fn stretch(&self) -> FontStretch {
        self.stretch
    }
    fn font_simulations(&self) -> FontSimulations {
        self.simulations
    }
    fn try_get_stream(&self) -> Option<Box<dyn Read>> {
        self.data
            .clone()
            .map(|v| Box::new(std::io::Cursor::new(v)) as Box<dyn Read>)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
/// The weight, style and stretch a face declares in its OS/2 table.
fn face_traits(font: &NativeFont) -> (FontWeight, FontStyle, FontStretch) {
    // SAFETY: q_font_table returns NULL or a +1 CFData.
    let os2 = unsafe { ffi::owned_bytes(ffi::q_font_table(font.raw(), u32::from_be_bytes(*b"OS/2"))) };
    let read = |at: usize| os2.get(at..at + 2).map(|b| u16::from_be_bytes([b[0], b[1]]));
    let weight = read(4)
        .filter(|w| *w > 0)
        .map_or(FontWeight::Normal, |w| FontWeight(i32::from(w).clamp(1, 1000)));
    let stretch = read(6)
        .and_then(|w| FontStretch::from_i32(i32::from(w)))
        .unwrap_or(FontStretch::Normal);
    let selection = read(62).unwrap_or(0);
    let style = if selection & 1 != 0 {
        FontStyle::Italic
    } else if selection & (1 << 9) != 0 {
        FontStyle::Oblique
    } else {
        FontStyle::Normal
    };
    (weight, style, stretch)
}
/// How far `actual` is from `desired` under CSS font matching: below 400 lighter weights come
/// first, above 500 heavier ones, and from 400 to 500 weights up to 500, then lighter, then
/// heavier.
fn weight_cost(desired: i32, actual: i32) -> i32 {
    let distance = (actual - desired).abs();
    let tier = if desired < 400 {
        i32::from(actual > desired)
    } else if desired > 500 {
        i32::from(actual < desired)
    } else if actual >= desired && actual <= 500 {
        0
    } else if actual < desired {
        1
    } else {
        2
    };
    tier * 10_000 + distance
}
fn style_cost(desired: FontStyle, actual: FontStyle) -> i32 {
    match (desired, actual) {
        (a, b) if a == b => 0,
        (FontStyle::Normal, _) | (_, FontStyle::Normal) => 2,
        _ => 1, // Italic and oblique stand in for each other.
    }
}
/// A face loaded from a stream, kept for lookups by family name.
struct EmbeddedFace {
    font: Arc<NativeFont>,
    weight: FontWeight,
    style: FontStyle,
    stretch: FontStretch,
    data: Arc<[u8]>,
}
#[derive(Default)]
pub struct FontManager {
    embedded: RefCell<Vec<EmbeddedFace>>,
}
impl FontManager {
    /// Loads a variable font using public Core Text axis identifiers (OpenType tags).
    /// Unknown axes and values outside their declared ranges are rejected.
    pub fn create_variable_typeface_from_stream(
        &self,
        stream: &mut dyn Read,
        simulations: FontSimulations,
        variations: &[(OpenTypeTag, f64)],
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        let mut data = Vec::new();
        stream.read_to_end(&mut data).ok()?;
        let tags: Vec<_> = variations.iter().map(|(t, _)| t.value()).collect();
        let values: Vec<_> = variations.iter().map(|(_, v)| *v).collect();
        // SAFETY: q_font_data and q_font_variations return NULL or a +1 CTFont.
        let font = unsafe {
            let original = Owned::new(ffi::q_font_data(data.as_ptr(), data.len()))?;
            NativeFont::from_raw(ffi::q_font_variations(
                original.0,
                tags.as_ptr(),
                values.as_ptr(),
                tags.len(),
            ))?
        };
        let (mut weight, style, stretch) = face_traits(&font);
        // A weight axis instance is that weight, whatever the default instance declares.
        if let Some((_, value)) = variations
            .iter()
            .find(|(tag, _)| tag.value() == u32::from_be_bytes(*b"wght"))
        {
            weight = FontWeight((value.round() as i32).clamp(1, 1000));
        }
        let data: Arc<[u8]> = data.into();
        {
            let mut embedded = self.embedded.borrow_mut();
            let known = embedded.iter().any(|face| {
                face.font.name.eq_ignore_ascii_case(&font.name)
                    && face.weight == weight
                    && face.style == style
                    && face.stretch == stretch
            });
            if !known {
                embedded.push(EmbeddedFace {
                    font: font.clone(),
                    weight,
                    style,
                    stretch,
                    data: data.clone(),
                });
            }
        }
        let mut face = Typeface::new(font, weight, style, stretch);
        face.simulations = simulations;
        face.data = Some(data);
        Some(Rc::new(face))
    }
    fn system(
        name: &str,
        weight: FontWeight,
        style: FontStyle,
        stretch: FontStretch,
    ) -> Option<Arc<NativeFont>> {
        let name = CString::new(name).ok()?;
        unsafe {
            let p = ffi::q_font(
                name.as_ptr(),
                weight.0,
                i32::from(style != FontStyle::Normal),
                stretch as i32,
            );
            NativeFont::from_raw(p)
        }
    }
    /// The embedded face of `name` nearest to the request: stretch first, then style, then
    /// weight, as CSS font matching orders them.
    fn nearest_embedded(
        &self,
        name: &str,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Typeface> {
        let embedded = self.embedded.borrow();
        let face = embedded
            .iter()
            .filter(|face| face.font.name.eq_ignore_ascii_case(name))
            .min_by_key(|face| {
                (
                    (face.stretch as i32 - stretch as i32).abs(),
                    style_cost(style, face.style),
                    weight_cost(weight.0, face.weight.0),
                )
            })?;
        // The face's own traits: the font collection simulates bold or oblique only when the
        // family has no face of the requested weight or style.
        let mut typeface = Typeface::new(face.font.clone(), face.weight, face.style, face.stretch);
        typeface.data = Some(face.data.clone());
        Some(typeface)
    }
}
impl IFontManagerImpl for FontManager {
    fn get_default_font_family_name(&self) -> String {
        Self::system(
            "",
            FontWeight::Normal,
            FontStyle::Normal,
            FontStretch::Normal,
        )
        .map_or_else(|| "Helvetica".to_owned(), |font| font.name.clone())
    }
    fn get_installed_font_family_names(&self, _: bool) -> Vec<String> {
        // SAFETY: q_font_names returns NULL or a +1 CFData.
        let names = unsafe { ffi::owned_bytes(ffi::q_font_names()) };
        String::from_utf8_lossy(&names)
            .lines()
            .map(str::to_owned)
            .collect()
    }
    fn try_create_glyph_typeface(
        &self,
        name: &str,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        if let Some(face) = self.nearest_embedded(name, style, weight, stretch) {
            return Some(Rc::new(face));
        }
        let font = Self::system(name, weight, style, stretch)?;
        // CTFont creation silently substitutes an unknown family. Preserve the
        // font manager contract so Ferro can try the next requested family.
        if !name.is_empty()
            && name != "Default"
            && name != "$Default"
            && !font.name.eq_ignore_ascii_case(name)
        {
            return None;
        }
        Some(Rc::new(Typeface::new(font, weight, style, stretch)))
    }
    fn try_match_character(
        &self,
        codepoint: i32,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
        name: Option<&str>,
        _: Option<&Locale>,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        let ch = char::from_u32(codepoint as u32)?;
        let mut buf = [0; 2];
        let text = ch.encode_utf16(&mut buf);
        let font = Self::system(name.unwrap_or(""), weight, style, stretch)?;
        unsafe {
            let p = ffi::q_font_match(font.raw(), text.as_ptr(), text.len());
            Some(Rc::new(Typeface::new(
                NativeFont::from_raw(p)?,
                weight,
                style,
                stretch,
            )))
        }
    }
    fn try_create_glyph_typeface_from_stream(
        &self,
        stream: &mut dyn Read,
        simulations: FontSimulations,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        self.create_variable_typeface_from_stream(stream, simulations, &[])
    }

    fn try_get_family_typefaces(&self, name: &str) -> Option<Vec<ferroui_base::media::Typeface>> {
        let family = CString::new(name).ok()?;
        // SAFETY: q_font_family_faces returns NULL or a +1 CFData.
        let bytes = unsafe { ffi::owned_bytes(ffi::q_font_family_faces(family.as_ptr())) };
        let faces: Vec<_> = String::from_utf8_lossy(&bytes)
            .lines()
            .filter_map(|line| {
                let values: Vec<i32> = line.split(',').filter_map(|v| v.parse().ok()).collect();
                (values.len() == 3).then(|| {
                    ferroui_base::media::Typeface::from_name_with_style(
                        name,
                        if values[1] != 0 {
                            FontStyle::Italic
                        } else {
                            FontStyle::Normal
                        },
                        FontWeight(values[0]),
                        FontStretch::from_i32(values[2]).unwrap_or(FontStretch::Normal),
                    )
                })
            })
            .collect();
        (!faces.is_empty()).then_some(faces)
    }
}
struct ShaperTypeface;
impl ITextShaperTypeface for ShaperTypeface {
    fn dispose(&self) {}
    fn as_any(&self) -> &dyn Any {
        self
    }
}
pub struct TextShaper;
impl ITextShaperImpl for TextShaper {
    fn create_typeface(&self, _: &GlyphTypeface) -> Rc<dyn ITextShaperTypeface> {
        Rc::new(ShaperTypeface)
    }
    fn shape_text(
        &self,
        text: &ReadOnlyMemory<u16>,
        options: &TextShaperOptions,
    ) -> Rc<ShapedBuffer> {
        let face = options.glyph_typeface();
        let Some(native) = face.platform_typeface().as_any().downcast_ref::<Typeface>() else {
            crate::warn_once("text shaped with a typeface of another backend is not shown");
            return ShapedBuffer::new(
                text.clone(),
                0,
                face.clone(),
                options.font_rendering_em_size(),
                options.bidi_level(),
            );
        };
        let mut glyphs: Vec<ffi::Glyph> = Vec::new();
        extern "C" fn emit(p: ffi::Ptr, g: ffi::Glyph) {
            unsafe { (&mut *(p as *mut Vec<ffi::Glyph>)).push(g) }
        }
        let features = options.font_features();
        let mut tags = Vec::new();
        let mut values = Vec::new();
        if let Some(features) = features {
            for f in features.iter() {
                tags.push(OpenTypeTag::parse(&f.tag).value());
                values.push(f.value as i32);
            }
        }
        // CR/LF are layout delimiters, not visible glyphs. Core Text consumes ZWNJ without an advance.
        // Preserve surrounding UTF-16 context for joining scripts across styled
        // runs. Core Text emits only the requested line range; clusters stay local.
        let mut units = text.owner().map(|v| v.to_vec()).unwrap_or_default();
        let language = CString::new(
            options
                .culture()
                .map(ToString::to_string)
                .unwrap_or_default(),
        )
        .unwrap_or_default();
        for u in &mut units {
            if *u == 10 || *u == 13 {
                *u = 0x200c;
            }
        }
        unsafe {
            ffi::q_shape(
                native.font.raw(),
                units.as_ptr(),
                units.len(),
                text.offset_in_owner(),
                text.len(),
                language.as_ptr(),
                options.font_rendering_em_size(),
                i32::from(options.bidi_level() & 1 != 0),
                tags.as_ptr(),
                values.as_ptr(),
                tags.len(),
                &mut glyphs as *mut _ as ffi::Ptr,
                emit,
            );
        }
        let shaped = ShapedBuffer::new(
            text.clone(),
            glyphs.len(),
            face.clone(),
            options.font_rendering_em_size(),
            options.bidi_level(),
        );
        for (i, g) in glyphs.iter().enumerate() {
            let mut id = g.glyph;
            let mut advance = g.advance + options.letter_spacing();
            if text.span().get(g.cluster as usize) == Some(&9) {
                id = face.character_to_glyph_map().get_glyph(32);
                advance = if options.incremental_tab_width() > 0.0 {
                    options.incremental_tab_width()
                } else {
                    4.0 * face.try_get_horizontal_glyph_advance(id).unwrap_or(0) as f64
                        * options.font_rendering_em_size()
                        / face.metrics().design_em_height as f64
                };
            }
            shaped.set(
                i,
                GlyphInfo::with_offset(id, g.cluster, advance, Vector::new(g.x, g.y)),
            );
        }
        shaped
    }
}
