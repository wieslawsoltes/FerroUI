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
// SAFETY: an IDWriteFontFace is immutable and DirectWrite permits concurrent access to font objects.
unsafe impl Sync for NativeFont {}
impl NativeFont {
    pub unsafe fn from_raw(p: ffi::Ptr) -> Arc<Self> {
        let handle = Owned::new(p);
        let mut name = vec![0; 4096];
        ffi::d2_font_name(p, name.as_mut_ptr(), name.len());
        name.truncate(name.iter().position(|b| *b == 0).unwrap_or(name.len()));
        Arc::new(Self {
            handle,
            name: String::from_utf8_lossy(&name).into_owned(),
        })
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
    data: Option<Vec<u8>>,
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
        unsafe {
            let p = ffi::d2_font_table(self.font.raw(), tag.value());
            (!p.is_null()).then(|| ReadOnlyMemory::from_vec(Owned::new(p).bytes()))
        }
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
#[derive(Default)]
pub struct FontManager {
    embedded: RefCell<Vec<Arc<NativeFont>>>,
}
impl FontManager {
    /// Loads a variable font using public DirectWrite axis identifiers (OpenType tags).
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
        unsafe {
            let raw = ffi::d2_font_data(data.as_ptr(), data.len());
            if raw.is_null() {
                return None;
            }
            let original = Owned::new(raw);
            let raw =
                ffi::d2_font_variations(original.0, tags.as_ptr(), values.as_ptr(), tags.len());
            if raw.is_null() {
                return None;
            }
            let font = NativeFont::from_raw(raw);
            self.embedded.borrow_mut().push(font.clone());
            let mut face = Typeface::new(
                font,
                FontWeight::Normal,
                FontStyle::Normal,
                FontStretch::Normal,
            );
            face.simulations = simulations;
            face.data = Some(data);
            Some(Rc::new(face))
        }
    }
    fn system(
        name: &str,
        weight: FontWeight,
        style: FontStyle,
        stretch: FontStretch,
    ) -> Option<Arc<NativeFont>> {
        let name = CString::new(name).ok()?;
        unsafe {
            let p = ffi::d2_font(name.as_ptr(), weight.0, style as i32, stretch as i32);
            (!p.is_null()).then(|| NativeFont::from_raw(p))
        }
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
        .unwrap()
        .name
        .clone()
    }
    fn get_installed_font_family_names(&self, _: bool) -> Vec<String> {
        unsafe {
            String::from_utf8_lossy(&Owned::new(ffi::d2_font_names()).bytes())
                .lines()
                .map(str::to_owned)
                .collect()
        }
    }
    fn try_create_glyph_typeface(
        &self,
        name: &str,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        let font = self
            .embedded
            .borrow()
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(name))
            .cloned()
            .or_else(|| Self::system(name, weight, style, stretch))?;
        // The native lookup rejects absent families. Do not compare the returned
        // English display name with a valid localized name accepted by DirectWrite.
        Some(Rc::new(Typeface::new(font, weight, style, stretch)))
    }
    fn try_match_character(
        &self,
        codepoint: i32,
        style: FontStyle,
        weight: FontWeight,
        stretch: FontStretch,
        name: Option<&str>,
        culture: Option<&Locale>,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        let ch = char::from_u32(codepoint as u32)?;
        let mut buf = [0; 2];
        let text = ch.encode_utf16(&mut buf);
        let font = Self::system(name.unwrap_or(""), weight, style, stretch)?;
        let language = CString::new(culture.map(ToString::to_string).unwrap_or_default()).ok()?;
        unsafe {
            let p = ffi::d2_font_match(font.raw(), text.as_ptr(), text.len(), language.as_ptr());
            if p.is_null() {
                return None;
            }
            Some(Rc::new(Typeface::new(
                NativeFont::from_raw(p),
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
        let bytes = unsafe { Owned::new(ffi::d2_font_family_faces(family.as_ptr())).bytes() };
        let faces: Vec<_> = String::from_utf8_lossy(&bytes)
            .lines()
            .filter_map(|line| {
                let values: Vec<i32> = line.split(',').filter_map(|v| v.parse().ok()).collect();
                (values.len() == 3).then(|| {
                    ferroui_base::media::Typeface::from_name_with_style(
                        name,
                        match values[1] {
                            1 => FontStyle::Italic,
                            2 => FontStyle::Oblique,
                            _ => FontStyle::Normal,
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
        let native = face
            .platform_typeface()
            .as_any()
            .downcast_ref::<Typeface>()
            .expect("DirectWrite requires a Direct2D typeface");
        let mut glyphs: Vec<ffi::Glyph> = Vec::with_capacity(text.len());
        extern "C" fn emit(p: ffi::Ptr, g: ffi::Glyph) {
            unsafe { (&mut *(p as *mut Vec<ffi::Glyph>)).push(g) }
        }
        let features = options.font_features();
        let mut tags = Vec::new();
        let mut values = Vec::new();
        let mut feature_starts = Vec::new();
        let mut feature_ends = Vec::new();
        if let Some(features) = features {
            for f in features.iter() {
                tags.push(OpenTypeTag::parse(&f.tag).value());
                values.push(f.value as i32);
                feature_starts.push(f.start as u32);
                feature_ends.push(f.end as u32);
            }
        }
        // Preserve the paragraph's UTF-16 joining context across formatter slices.
        // The adapter uses the original delimiters to exclude unrelated paragraphs.
        let units = text.owner().map(|v| v.as_ref()).unwrap_or_default();
        let language = CString::new(
            options
                .culture()
                .map(ToString::to_string)
                .unwrap_or_default(),
        )
        .unwrap();
        unsafe {
            assert_ne!(
                ffi::d2_shape(
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
                    feature_starts.as_ptr(),
                    feature_ends.as_ptr(),
                    tags.len(),
                    &mut glyphs as *mut _ as ffi::Ptr,
                    emit,
                ),
                0,
                "DirectWrite shaping failed"
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
            if matches!(
                text.span().get(g.cluster as usize),
                Some(10 | 13 | 0x2028 | 0x2029)
            ) {
                id = face.character_to_glyph_map().get_glyph(32);
            }
            let last_in_cluster = glyphs
                .get(i + 1)
                .is_none_or(|next| next.cluster != g.cluster);
            let control = matches!(
                text.span().get(g.cluster as usize),
                Some(10 | 13 | 0x2028 | 0x2029 | 0x200c | 0x200d)
            );
            let mut advance = if control {
                0.
            } else {
                g.advance
                    + if last_in_cluster {
                        options.letter_spacing()
                    } else {
                        0.
                    }
            };
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
