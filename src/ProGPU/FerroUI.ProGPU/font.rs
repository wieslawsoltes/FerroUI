//! The font stack of the engine, owned: the catalog of the system fonts and a face (its bytes,
//! tables, metrics, names, style variations and unhinted outlines).
//!
//! The engine reads fonts with its own parsers: nothing here goes through a font library of the
//! platform. A catalog and a face are immutable and are shared between threads.

use progpu_native_sys::font::*;
use progpu_native_sys::progpu::{
    progpu_native_glyph_outline_key, progpu_native_path_segment, PROGPU_NATIVE_PATH_SEGMENT_LINE, PROGPU_NATIVE_PATH_SEGMENT_QUADRATIC,
};
use std::collections::HashMap;
use std::ops::Deref;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

static NEXT_FACE_ID: AtomicU64 = AtomicU64::new(1);
/// The numbers of the variation instances of the faces of this process: counted from one and
/// never given a second time, so an outline key names one outline for the life of the process.
static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(1);

/// The shear of a simulated oblique, as the other backends shear it.
pub(crate) const OBLIQUE_SKEW: f32 = 0.3;

/// A requested style: an OpenType weight (1 to 1000), a width class (1 to 9) and a slant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FaceStyle {
    pub weight: i32,
    pub width: i32,
    pub italic: bool,
}

impl FaceStyle {
    fn native(self) -> progpu_native_font_style {
        progpu_native_font_style {
            weight: self.weight,
            width: self.width,
            slant: if self.italic { PROGPU_NATIVE_FONT_SLANT_ITALIC } else { PROGPU_NATIVE_FONT_SLANT_NORMAL },
            optical_size_fixed: 0,
        }
    }
}

/// The axes of a variable face a style was applied to (`progpu_native_font_face_resolve_style`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct AppliedAxes(u32);

impl AppliedAxes {
    /// The weight of the style is the weight of the instance.
    pub fn weight(self) -> bool {
        self.0 & 1 != 0
    }

    /// The slant of the style is the slant of the instance (an italic or a slant axis).
    pub fn slant(self) -> bool {
        self.0 & (4 | 8) != 0
    }
}

/// The unhinted outline of a glyph in design units, Y up, and its control box.
#[derive(Clone, Debug, Default)]
pub(crate) struct GlyphOutline {
    pub segments: Vec<progpu_native_path_segment>,
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

/// An outline a face keeps: the outline and the key the engines know it by
/// ([`crate::outline_log`]).
pub(crate) struct KeptOutline {
    outline: GlyphOutline,
    key: progpu_native_glyph_outline_key,
    /// Whether the outline is in the outline log.
    logged: AtomicBool,
}

impl KeptOutline {
    /// The key of the outline: the variation instance of its face, its glyph and its variant
    /// (the weight and the simulated oblique).
    pub fn key(&self) -> progpu_native_glyph_outline_key {
        self.key
    }

    /// Whether the outline has anything to draw: segments and a box with an area.
    pub fn drawable(&self) -> bool {
        !self.outline.segments.is_empty() && self.outline.max_x > self.outline.min_x && self.outline.max_y > self.outline.min_y
    }

    /// Whether the outline is in the outline log (written by the log, under its lock).
    pub(crate) fn logged(&self) -> &AtomicBool {
        &self.logged
    }
}

impl Deref for KeptOutline {
    type Target = GlyphOutline;

    fn deref(&self) -> &GlyphOutline {
        &self.outline
    }
}

/// The outlines of one variation instance of a face.
struct Instance {
    /// The number the outline keys name the instance by.
    number: u64,
    /// By glyph and variant (`None` for a glyph the face does not have).
    glyphs: HashMap<(u16, u16), Option<Arc<KeptOutline>>>,
}

/// A font face of the engine.
pub(crate) struct FontFace {
    raw: *mut progpu_native_font_face,
    id: u64,
    metrics: progpu_native_font_face_metrics,
    design_box: [f32; 4],
    /// The outlines read so far, by variation instance.
    outlines: Mutex<OutlineCache>,
}

type OutlineCache = HashMap<Box<[i16]>, Instance>;

/// The steps of the weight of an outline: a weight is so many 4096ths of the em its edges move
/// sideways by.
pub(crate) const WEIGHT_STEPS_PER_EM: f32 = 4096.0;

// SAFETY: a face of the engine is immutable and may be used from several threads (the C ABI
// says so); the outline call keeps its storage per thread.
unsafe impl Send for FontFace {}
// SAFETY: as above.
unsafe impl Sync for FontFace {}

impl FontFace {
    fn from_raw(raw: *mut progpu_native_font_face) -> Option<Arc<FontFace>> {
        if raw.is_null() {
            return None;
        }
        // SAFETY: plain data, written by the call.
        let mut metrics: progpu_native_font_face_metrics = unsafe { core::mem::zeroed() };
        // SAFETY: the face is alive; the record is written by the call.
        if unsafe { progpu_native_font_face_get_metrics(raw, &mut metrics) } == 0 {
            // SAFETY: the face is owned here and released once.
            unsafe { progpu_native_font_face_destroy(raw) };
            return None;
        }
        let mut face =
            FontFace { raw, id: NEXT_FACE_ID.fetch_add(1, Ordering::Relaxed), metrics, design_box: [0.0; 4], outlines: Mutex::new(HashMap::new()) };
        face.design_box = face.read_design_box();
        Some(Arc::new(face))
    }

    /// The box of all glyphs the `head` table states, or the em square on the line of the face
    /// when the table states none.
    fn read_design_box(&self) -> [f32; 4] {
        let read = |table: &[u8], offset: usize| table.get(offset..offset + 2).map(|bytes| f32::from(i16::from_be_bytes([bytes[0], bytes[1]])));
        let stated = self.table(u32::from_be_bytes(*b"head")).and_then(|head| Some([read(head, 36)?, read(head, 38)?, read(head, 40)?, read(head, 42)?]));
        match stated {
            Some(stated) if stated[2] > stated[0] && stated[3] > stated[1] => stated,
            _ => [0.0, f32::from(self.metrics.descender), f32::from(self.metrics.units_per_em), f32::from(self.metrics.ascender)],
        }
    }

    /// The box every glyph of the face is in, in design units, Y up: minimum x, minimum y,
    /// maximum x, maximum y.
    pub fn design_box(&self) -> [f32; 4] {
        self.design_box
    }

    /// The face `face_index` of the font file `bytes` (copied), or `None` for bytes that are no
    /// font.
    pub fn from_bytes(bytes: &[u8], face_index: u32) -> Option<Arc<FontFace>> {
        // SAFETY: the bytes are borrowed for the call: the face copies them.
        Self::from_raw(unsafe { progpu_native_font_face_create(bytes.as_ptr().cast(), bytes.len(), face_index) })
    }

    /// The identifier of the face in this process: what the glyph caches key a font with.
    pub fn id(&self) -> u64 {
        self.id
    }

    /// The design units of an em.
    pub fn units_per_em(&self) -> u16 {
        self.metrics.units_per_em
    }

    /// The number of glyphs.
    pub fn glyph_count(&self) -> u16 {
        self.metrics.glyph_count
    }

    /// The weight the face is designed with.
    pub fn weight(&self) -> u16 {
        self.metrics.weight
    }

    /// The width class the face is designed with.
    pub fn width(&self) -> u16 {
        self.metrics.width
    }

    /// Whether the face is designed italic.
    pub fn italic(&self) -> bool {
        self.metrics.italic != 0
    }

    /// The number of variation axes.
    pub fn axis_count(&self) -> usize {
        usize::from(self.metrics.variation_axis_count)
    }

    /// The bytes of the font file and the index of the face in it.
    pub fn data(&self) -> (&[u8], u32) {
        let (mut data, mut size, mut face_index) = (ptr::null(), 0usize, 0u32);
        // SAFETY: the outputs are written by the call; the bytes live as long as the face.
        unsafe {
            if progpu_native_font_face_get_data(self.raw, &mut data, &mut size, &mut face_index) == 0 || data.is_null() {
                return (&[], 0);
            }
            (core::slice::from_raw_parts(data.cast(), size), face_index)
        }
    }

    /// The table `tag` of the face.
    pub fn table(&self, tag: u32) -> Option<&[u8]> {
        let (mut data, mut size) = (ptr::null(), 0usize);
        // SAFETY: the outputs are written by the call; the table lives as long as the face.
        unsafe {
            if progpu_native_font_face_get_table(self.raw, tag, &mut data, &mut size) == 0 || data.is_null() {
                return None;
            }
            Some(core::slice::from_raw_parts(data.cast(), size))
        }
    }

    /// The name `name_id` of the name table (1 family, 4 full name, 16 typographic family).
    pub fn name(&self, name_id: u16) -> Option<String> {
        let mut buffer = [0u8; 256];
        let mut size = 0usize;
        // SAFETY: the buffer is written within its capacity.
        if unsafe { progpu_native_font_face_get_name(self.raw, name_id, buffer.as_mut_ptr().cast(), buffer.len(), &mut size) } == 0 {
            return None;
        }
        let name = String::from_utf8_lossy(&buffer[..size.min(buffer.len())]).into_owned();
        (!name.is_empty()).then_some(name)
    }

    /// The normalized variation coordinates of a style on this face (empty for a static face)
    /// and the axes the style was applied to.
    pub fn resolve_style(&self, style: FaceStyle) -> (Vec<i16>, AppliedAxes) {
        let mut coordinates = vec![0i16; self.axis_count()];
        if coordinates.is_empty() {
            return (coordinates, AppliedAxes::default());
        }
        let (mut written, mut applied) = (0usize, 0u32);
        let request = style.native();
        // SAFETY: the coordinates are written within their capacity.
        let ok = unsafe {
            progpu_native_font_face_resolve_style(self.raw, &request, coordinates.as_mut_ptr(), coordinates.len(), &mut written, &mut applied)
        };
        if ok == 0 {
            return (Vec::new(), AppliedAxes::default());
        }
        coordinates.truncate(written);
        (coordinates, AppliedAxes(applied))
    }

    /// The unhinted outline of `glyph` at the variation `coordinates`. A glyph without an
    /// outline (a space) has no segments; a glyph the face does not have gives `None`.
    pub fn outline(&self, glyph: u16, coordinates: &[i16]) -> Option<GlyphOutline> {
        let (mut segments, mut count) = (ptr::null(), 0usize);
        // SAFETY: plain data, written by the call.
        let mut bounds: progpu_native_font_glyph_bounds = unsafe { core::mem::zeroed() };
        // SAFETY: the segments are borrowed from this thread's storage and copied before the
        // thread's next outline call.
        unsafe {
            let coordinates_pointer = if coordinates.is_empty() { ptr::null() } else { coordinates.as_ptr() };
            if progpu_native_font_face_get_outline(self.raw, glyph, coordinates_pointer, coordinates.len(), &mut segments, &mut count, &mut bounds) == 0 {
                return None;
            }
            let segments = if count == 0 || segments.is_null() { Vec::new() } else { core::slice::from_raw_parts(segments, count).to_vec() };
            Some(GlyphOutline { segments, min_x: bounds.x_min, min_y: bounds.y_min, max_x: bounds.x_max, max_y: bounds.y_max })
        }
    }

    /// The outline of `glyph` at the variation `coordinates` at a text weight of `weight` steps
    /// ([`WEIGHT_STEPS_PER_EM`]; none for the outline as the face has it), sheared for a
    /// simulated oblique with `oblique`: read from the face once and shared after.
    ///
    /// The weight is the growth measured on the text of the platform (docs/porting/
    /// progpu-backend.md, the text quality check): every edge moves sideways by the steps, an
    /// edge that faces up moves up by twice the steps, and an edge that faces down stays where
    /// it is, so a baseline stays a baseline.
    pub fn cached_outline(&self, glyph: u16, coordinates: &[i16], weight: u8, oblique: bool) -> Option<Arc<KeptOutline>> {
        let variant = u16::from(weight) | (u16::from(oblique) << 8);
        let mut outlines = self.outlines.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(found) = outlines.get(coordinates).and_then(|instance| instance.glyphs.get(&(glyph, variant))) {
            return found.clone();
        }
        let outline = self.outline(glyph, coordinates).map(|mut outline| {
            if weight != 0 && !outline.segments.is_empty() {
                let distance = f32::from(weight) * f32::from(self.units_per_em().max(1)) / WEIGHT_STEPS_PER_EM;
                let growth = progpu_native_font_outline_growth { left: distance, bottom: 0.0, right: distance, top: 2.0 * distance };
                // SAFETY: plain data, written by the call.
                let mut bounds: progpu_native_font_glyph_bounds = unsafe { core::mem::zeroed() };
                let mut segments = ptr::null();
                let mut count = 0usize;
                // SAFETY: the source is owned by the outline; the grown segments are borrowed
                // from the calling thread and copied before its next call.
                unsafe {
                    if progpu_native_font_outline_embolden(outline.segments.as_ptr(), outline.segments.len(), &growth, &mut segments, &mut count, &mut bounds) != 0 && count != 0 && !segments.is_null() {
                        outline.segments = core::slice::from_raw_parts(segments, count).to_vec();
                        (outline.min_x, outline.min_y, outline.max_x, outline.max_y) = (bounds.x_min, bounds.y_min, bounds.x_max, bounds.y_max);
                    }
                }
            }
            if oblique && !outline.segments.is_empty() {
                shear(&mut outline, OBLIQUE_SKEW);
            }
            outline
        });
        let instance = match outlines.get_mut(coordinates) {
            Some(instance) => instance,
            None => outlines
                .entry(coordinates.into())
                .or_insert_with(|| Instance { number: NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed), glyphs: HashMap::new() }),
        };
        let key = progpu_native_glyph_outline_key { face: instance.number, glyph: u32::from(glyph), variant: u32::from(variant) };
        let outline = outline.map(|outline| Arc::new(KeptOutline { outline, key, logged: AtomicBool::new(false) }));
        instance.glyphs.insert((glyph, variant), outline.clone());
        outline
    }
}

/// Shears `outline` for a simulated oblique (design units, Y up: what is above the baseline
/// moves right); its box becomes the box of its control points.
fn shear(outline: &mut GlyphOutline, skew: f32) {
    let mut bounds = [f32::MAX, f32::MAX, f32::MIN, f32::MIN];
    for segment in &mut outline.segments {
        let used = if segment.kind == PROGPU_NATIVE_PATH_SEGMENT_LINE as u32 {
            2
        } else if segment.kind == PROGPU_NATIVE_PATH_SEGMENT_QUADRATIC as u32 {
            3
        } else {
            4
        };
        for (index, point) in [&mut segment.p0, &mut segment.p1, &mut segment.p2, &mut segment.p3].into_iter().enumerate() {
            point.x += point.y * skew;
            if index < used {
                bounds = [bounds[0].min(point.x), bounds[1].min(point.y), bounds[2].max(point.x), bounds[3].max(point.y)];
            }
        }
    }
    [outline.min_x, outline.min_y, outline.max_x, outline.max_y] = bounds;
}

impl Drop for FontFace {
    fn drop(&mut self) {
        // The outlines of the face leave the log, and the engines forget them: the numbers of
        // its instances name nothing afterwards.
        let outlines = self.outlines.get_mut().unwrap_or_else(|poisoned| poisoned.into_inner());
        let instances: Vec<u64> = outlines.values().map(|instance| instance.number).collect();
        crate::outline_log::release(&instances);
        // SAFETY: the face is owned here and released once.
        unsafe { progpu_native_font_face_destroy(self.raw) };
    }
}

/// A face of the catalog: what the catalog knows of it without loading it.
#[derive(Clone, Debug)]
pub(crate) struct CatalogFace {
    pub index: u32,
    pub family: String,
    pub identity: u64,
    pub weight: u16,
    pub width: u8,
    pub italic: bool,
}

/// The catalog of the fonts of the system, discovered by the engine.
pub(crate) struct FontCatalog {
    raw: *mut progpu_native_font_catalog,
    loaded: Mutex<HashMap<u32, Arc<FontFace>>>,
}

// SAFETY: a catalog of the engine is immutable and may be read from several threads.
unsafe impl Send for FontCatalog {}
// SAFETY: as above; the loaded faces are behind a lock.
unsafe impl Sync for FontCatalog {}

impl FontCatalog {
    /// The catalog of the system fonts of this process, discovered once.
    pub fn system() -> Option<&'static FontCatalog> {
        static CATALOG: OnceLock<Option<FontCatalog>> = OnceLock::new();
        CATALOG
            .get_or_init(|| {
                // SAFETY: a plain creation; the result is checked.
                let raw = unsafe { progpu_native_font_catalog_create_system() };
                (!raw.is_null()).then(|| FontCatalog { raw, loaded: Mutex::new(HashMap::new()) })
            })
            .as_ref()
    }

    /// The number of faces.
    pub fn face_count(&self) -> u32 {
        // SAFETY: a plain query.
        unsafe { progpu_native_font_catalog_face_count(self.raw) }
    }

    /// The face at `index`.
    pub fn face(&self, index: u32) -> Option<CatalogFace> {
        // SAFETY: plain data, written by the call; its strings live as long as the catalog and
        // are copied here.
        unsafe {
            let mut info: progpu_native_font_face_info = core::mem::zeroed();
            if progpu_native_font_catalog_get_face_info(self.raw, index, &mut info) == 0 {
                return None;
            }
            let family = if info.family_name.is_null() {
                String::new()
            } else {
                String::from_utf8_lossy(core::slice::from_raw_parts(info.family_name.cast(), info.family_name_size)).into_owned()
            };
            Some(CatalogFace {
                index,
                family,
                identity: info.identity,
                weight: info.weight,
                width: info.width,
                italic: u32::from(info.slant) != PROGPU_NATIVE_FONT_SLANT_NORMAL,
            })
        }
    }

    /// The face of `family` nearest to `style`.
    pub fn match_family(&self, family: &str, style: FaceStyle) -> Option<CatalogFace> {
        let mut index = 0u32;
        let request = style.native();
        // SAFETY: the family is borrowed for the call; the index is written by it.
        if unsafe { progpu_native_font_catalog_match_family(self.raw, family.as_ptr().cast(), family.len(), &request, &mut index) } == 0 {
            return None;
        }
        self.face(index)
    }

    /// The face that has `code_point`: of `family` when it has it, else of a family of
    /// `language` (which may be empty), else of any.
    pub fn match_character(&self, family: &str, style: FaceStyle, language: &str, code_point: u32) -> Option<CatalogFace> {
        let (mut index, mut glyph) = (0u32, 0u16);
        let request = style.native();
        // SAFETY: the strings are borrowed for the call; the outputs are written by it.
        let found = unsafe {
            progpu_native_font_catalog_match_character(
                self.raw,
                family.as_ptr().cast(),
                family.len(),
                &request,
                language.as_ptr().cast(),
                language.len(),
                code_point,
                0,
                &mut index,
                &mut glyph,
            )
        };
        if found == 0 {
            return None;
        }
        self.face(index)
    }

    /// The loaded face at `index`: loaded once, shared after.
    pub fn load(&self, index: u32) -> Option<Arc<FontFace>> {
        let mut loaded = self.loaded.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some(face) = loaded.get(&index) {
            return Some(face.clone());
        }
        // SAFETY: an index of the catalog; the result is checked.
        let face = FontFace::from_raw(unsafe { progpu_native_font_catalog_load_face(self.raw, index) })?;
        loaded.insert(index, face.clone());
        Some(face)
    }
}
