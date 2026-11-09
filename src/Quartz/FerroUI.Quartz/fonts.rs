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
    traits: u32,
    weight: FontWeight,
    style: FontStyle,
    stretch: FontStretch,
    data: Option<Arc<[u8]>>,
}
// SAFETY: a CTFont is immutable and Core Text permits concurrent access to font objects.
unsafe impl Sync for NativeFont {}
impl NativeFont {
    pub unsafe fn from_raw(p: ffi::Ptr) -> Arc<Self> {
        let handle = Owned::new(p);
        let mut name = vec![0; 4096];
        ffi::q_font_name(p, name.as_mut_ptr(), name.len());
        name.truncate(name.iter().position(|b| *b == 0).unwrap_or(name.len()));
        let mut info = [0; 3];
        ffi::q_font_info(p, info.as_mut_ptr());
        Arc::new(Self {
            handle,
            data: None,
            traits: ffi::q_font_traits(p),
            name: String::from_utf8_lossy(&name).into_owned(),
            weight: FontWeight(info[0]),
            style: if info[1] == 0 {
                FontStyle::Normal
            } else {
                FontStyle::Italic
            },
            stretch: FontStretch::from_i32(info[2]).unwrap_or(FontStretch::Normal),
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
    data: Option<Arc<[u8]>>,
}
impl Typeface {
    fn new(
        font: Arc<NativeFont>,
        weight: FontWeight,
        style: FontStyle,
        _stretch: FontStretch,
    ) -> Self {
        let mut simulations = FontSimulations::None;
        if weight.0 >= 600 && font.traits & 2 == 0 {
            simulations |= FontSimulations::Bold;
        }
        if style != FontStyle::Normal && font.traits & 1 == 0 {
            simulations |= FontSimulations::Oblique;
        }
        Self {
            data: font.data.clone(),
            weight: font.weight,
            style: font.style,
            stretch: font.stretch,
            font,
            simulations,
        }
    }
}
impl IFontMemory for Typeface {
    fn try_get_table(&self, tag: OpenTypeTag) -> Option<ReadOnlyMemory<u8>> {
        unsafe {
            let p = ffi::q_font_table(self.font.raw(), tag.value());
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
// CSS font selection prefers weights directionally rather than by absolute distance.
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
#[derive(Default)]
pub struct FontManager {
    embedded: RefCell<Vec<Arc<NativeFont>>>,
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
        unsafe {
            let raw = ffi::q_font_data(data.as_ptr(), data.len());
            if raw.is_null() {
                return None;
            }
            let original = Owned::new(raw);
            let raw =
                ffi::q_font_variations(original.0, tags.as_ptr(), values.as_ptr(), tags.len());
            if raw.is_null() {
                return None;
            }
            let mut font = NativeFont::from_raw(raw);
            let data: Arc<[u8]> = data.into();
            if let Some(native) = Arc::get_mut(&mut font) {
                native.data = Some(data.clone());
                if let Some((_, weight)) = variations
                    .iter()
                    .find(|(tag, _)| tag.value() == u32::from_be_bytes(*b"wght"))
                {
                    native.weight = FontWeight((weight.round() as i32).clamp(1, 1000));
                }
            }
            let mut embedded = self.embedded.borrow_mut();
            if !embedded.iter().any(|f| {
                f.name.eq_ignore_ascii_case(&font.name)
                    && f.weight == font.weight
                    && f.style == font.style
                    && f.stretch == font.stretch
            }) {
                embedded.push(font.clone());
            }
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
            let p = ffi::q_font(
                name.as_ptr(),
                weight.0,
                i32::from(style != FontStyle::Normal),
                stretch as i32,
            );
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
        .map_or_else(|| "Helvetica".to_owned(), |font| font.name.clone())
    }
    fn get_installed_font_family_names(&self, _: bool) -> Vec<String> {
        unsafe {
            String::from_utf8_lossy(&Owned::new(ffi::q_font_names()).bytes())
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
            .filter(|f| f.name.eq_ignore_ascii_case(name))
            .min_by_key(|f| {
                (
                    (f.stretch as i32 - stretch as i32).abs(),
                    style_cost(style, f.style),
                    weight_cost(weight.0, f.weight.0),
                )
            })
            .cloned()
            .or_else(|| Self::system(name, weight, style, stretch))?;
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
        locale: Option<&Locale>,
    ) -> Option<Rc<dyn IPlatformTypeface>> {
        let ch = char::from_u32(codepoint as u32)?;
        let mut buf = [0; 2];
        let text = ch.encode_utf16(&mut buf);
        let font = Self::system(name.unwrap_or(""), weight, style, stretch)?;
        unsafe {
            let locale = CString::new(locale.map(ToString::to_string).unwrap_or_default()).ok()?;
            let p = ffi::q_font_match(font.raw(), text.as_ptr(), text.len(), locale.as_ptr());
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
        let bytes = unsafe { Owned::new(ffi::q_font_family_faces(family.as_ptr())).bytes() };
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
        let native = face
            .platform_typeface()
            .as_any()
            .downcast_ref::<Typeface>()
            .expect("Core Text requires a Quartz typeface");
        let mut glyphs: Vec<ffi::Glyph> = Vec::new();
        extern "C" fn emit(p: ffi::Ptr, g: ffi::Glyph) {
            unsafe { (&mut *(p as *mut Vec<ffi::Glyph>)).push(g) }
        }
        let features = options.font_features();
        let mut tags = Vec::new();
        let mut values = Vec::new();
        let mut starts = Vec::new();
        let mut ends = Vec::new();
        if let Some(features) = features {
            for f in features.iter() {
                tags.push(OpenTypeTag::parse(&f.tag).value());
                values.push(f.value as i32);
                starts.push(f.start as u32);
                ends.push(f.end as u32);
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
        .unwrap();
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
                starts.as_ptr(),
                ends.as_ptr(),
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
