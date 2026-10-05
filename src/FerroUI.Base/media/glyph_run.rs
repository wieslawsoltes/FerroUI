use std::cell::{Cell, Ref, RefCell};
use std::ops::Deref;
use std::rc::Rc;

use crate::media::i_immutable_glyph_run_reference::{release_platform_impl, ImmutableGlyphRunReference};
use crate::media::text_formatting::unicode::Codepoint;
use crate::media::text_formatting::{GlyphInfo, ShapedBuffer};
use crate::media::{CharacterHit, GlyphRunMetrics, GlyphTypeface, IImmutableGlyphRunReference, Typeface};
use crate::platform::{render_interface, IGlyphRunImpl};
use crate::utilities::ReadOnlyMemory;
use crate::{Point, Rect, Size};

/// The glyphs of a glyph run: either an owned list or a shaped buffer
/// (upstream: `IReadOnlyList<GlyphInfo>`).
#[derive(Clone)]
pub enum GlyphInfoList {
    /// A plain list of glyphs.
    List(Rc<[GlyphInfo]>),
    /// The glyphs of a shaped buffer.
    ShapedBuffer(Rc<ShapedBuffer>),
}

/// A borrow of the glyphs of a [`GlyphInfoList`].
pub enum GlyphInfosRef<'a> {
    Slice(&'a [GlyphInfo]),
    Shaped(Ref<'a, [GlyphInfo]>),
}

impl Deref for GlyphInfosRef<'_> {
    type Target = [GlyphInfo];

    #[inline]
    fn deref(&self) -> &[GlyphInfo] {
        match self {
            GlyphInfosRef::Slice(slice) => slice,
            GlyphInfosRef::Shaped(shaped) => shaped,
        }
    }
}

impl GlyphInfoList {
    /// An empty list.
    pub fn empty() -> Self {
        GlyphInfoList::List(Rc::from(Vec::new()))
    }

    /// Borrows the glyphs.
    #[inline]
    pub fn borrow(&self) -> GlyphInfosRef<'_> {
        match self {
            GlyphInfoList::List(list) => GlyphInfosRef::Slice(list),
            GlyphInfoList::ShapedBuffer(buffer) => GlyphInfosRef::Shaped(buffer.glyph_infos()),
        }
    }

    /// The number of glyphs.
    pub fn count(&self) -> usize {
        match self {
            GlyphInfoList::List(list) => list.len(),
            GlyphInfoList::ShapedBuffer(buffer) => buffer.length(),
        }
    }
}

impl From<Vec<GlyphInfo>> for GlyphInfoList {
    fn from(glyph_infos: Vec<GlyphInfo>) -> Self {
        GlyphInfoList::List(Rc::from(glyph_infos))
    }
}

impl From<Rc<ShapedBuffer>> for GlyphInfoList {
    fn from(buffer: Rc<ShapedBuffer>) -> Self {
        GlyphInfoList::ShapedBuffer(buffer)
    }
}

/// `list.BinarySearch(value, comparer)` for a cluster: `Ok(index)` of a glyph
/// with that cluster or `Err(insertion index)`.
fn binary_search_cluster(glyph_infos: &[GlyphInfo], cluster: i32, ascending: bool) -> Result<usize, usize> {
    if ascending {
        glyph_infos.binary_search_by(|probe| probe.glyph_cluster.cmp(&cluster))
    } else {
        glyph_infos.binary_search_by(|probe| cluster.cmp(&probe.glyph_cluster))
    }
}

/// Represents a sequence of glyphs from a single face of a single font at a
/// single size, and with a single rendering style.
pub struct GlyphRun {
    glyph_typeface: Rc<GlyphTypeface>,
    platform_impl: RefCell<Option<Rc<dyn IGlyphRunImpl>>>,
    font_rendering_em_size: Cell<f64>,
    bidi_level: Cell<i32>,
    glyph_run_metrics: Cell<Option<GlyphRunMetrics>>,
    characters: RefCell<ReadOnlyMemory<u16>>,
    glyph_infos: RefCell<GlyphInfoList>,
    baseline_origin: Cell<Option<Point>>,
    /// If true, character index and cluster are similar.
    has_one_char_per_cluster: Cell<bool>,
}

/// Glyph runs compare by reference.
impl PartialEq for GlyphRun {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl GlyphRun {
    /// Creates a glyph run from glyph indices: every glyph is its own cluster
    /// and gets the font's horizontal advance.
    pub fn from_glyph_indices(
        glyph_typeface: Rc<GlyphTypeface>,
        font_rendering_em_size: f64,
        characters: ReadOnlyMemory<u16>,
        glyph_indices: &[u16],
        baseline_origin: Option<Point>,
        bidi_level: i32,
    ) -> Rc<Self> {
        let glyph_infos = Self::create_glyph_infos(glyph_indices, font_rendering_em_size, &glyph_typeface);

        let run = Self::new(
            glyph_typeface,
            font_rendering_em_size,
            characters,
            GlyphInfoList::from(glyph_infos),
            baseline_origin,
            bidi_level,
        );

        run.has_one_char_per_cluster.set(true);

        run
    }

    /// Creates a glyph run.
    pub fn new(
        glyph_typeface: Rc<GlyphTypeface>,
        font_rendering_em_size: f64,
        characters: ReadOnlyMemory<u16>,
        glyph_infos: GlyphInfoList,
        baseline_origin: Option<Point>,
        bidi_level: i32,
    ) -> Rc<Self> {
        Rc::new(Self {
            glyph_typeface,
            platform_impl: RefCell::new(None),
            font_rendering_em_size: Cell::new(font_rendering_em_size),
            bidi_level: Cell::new(bidi_level),
            glyph_run_metrics: Cell::new(None),
            characters: RefCell::new(characters),
            glyph_infos: RefCell::new(glyph_infos),
            baseline_origin: Cell::new(baseline_origin),
            has_one_char_per_cluster: Cell::new(false),
        })
    }

    /// Creates a glyph run that only wraps platform resources.
    #[allow(dead_code)] // used by the rendering pipeline (glyph run references)
    pub(crate) fn from_platform_impl(platform_impl: Rc<dyn IGlyphRunImpl>) -> Rc<Self> {
        let baseline_origin = platform_impl.baseline_origin();

        Rc::new(Self {
            glyph_typeface: Typeface::default_typeface().glyph_typeface(),
            platform_impl: RefCell::new(Some(platform_impl)),
            font_rendering_em_size: Cell::new(0.0),
            bidi_level: Cell::new(0),
            glyph_run_metrics: Cell::new(None),
            characters: RefCell::new(ReadOnlyMemory::empty()),
            glyph_infos: RefCell::new(GlyphInfoList::empty()),
            baseline_origin: Cell::new(Some(baseline_origin)),
            has_one_char_per_cluster: Cell::new(false),
        })
    }

    fn create_glyph_infos(
        glyph_indices: &[u16],
        font_rendering_em_size: f64,
        glyph_typeface: &GlyphTypeface,
    ) -> Vec<GlyphInfo> {
        let mut glyph_infos = vec![GlyphInfo::default(); glyph_indices.len()];

        let scale = font_rendering_em_size / glyph_typeface.metrics().design_em_height as f64;

        // Batch call to get all advances at once.
        let mut stack_advances = [0u16; 256];
        let mut heap_advances;
        let advances: &mut [u16] = if glyph_indices.len() <= 256 {
            &mut stack_advances[..glyph_indices.len()]
        } else {
            heap_advances = vec![0u16; glyph_indices.len()];
            &mut heap_advances
        };

        if glyph_typeface.try_get_horizontal_glyph_advances(glyph_indices, advances) {
            for (i, glyph_info) in glyph_infos.iter_mut().enumerate() {
                *glyph_info = GlyphInfo::new(glyph_indices[i], i as i32, advances[i] as f64 * scale);
            }
        }

        glyph_infos
    }

    /// Gets the glyph typeface for the glyph run.
    pub fn glyph_typeface(&self) -> &Rc<GlyphTypeface> {
        &self.glyph_typeface
    }

    /// Gets the em size used for rendering the glyph run.
    pub fn font_rendering_em_size(&self) -> f64 {
        self.font_rendering_em_size.get()
    }

    /// Sets the em size used for rendering the glyph run.
    pub fn set_font_rendering_em_size(&self, value: f64) {
        self.invalidate();
        self.font_rendering_em_size.set(value);
    }

    /// Gets the conservative bounding box of the glyph run.
    pub fn bounds(&self) -> Rect {
        let metrics = self.metrics();
        Rect::from_position_size(
            Point::new(self.baseline_origin().x, 0.0),
            Size::new(metrics.width_including_trailing_whitespace, metrics.height),
        )
    }

    /// Gets the ink bounds of the glyph run.
    pub fn ink_bounds(&self) -> Rect {
        if self.glyph_infos.borrow().count() == 0 {
            Rect::default()
        } else {
            self.platform_impl().bounds()
        }
    }

    /// Gets the metrics of the glyph run.
    pub fn metrics(&self) -> GlyphRunMetrics {
        if let Some(metrics) = self.glyph_run_metrics.get() {
            return metrics;
        }

        let metrics = self.create_glyph_run_metrics();
        self.glyph_run_metrics.set(Some(metrics));
        metrics
    }

    /// Gets the baseline origin of the glyph run.
    pub fn baseline_origin(&self) -> Point {
        match self.baseline_origin.get() {
            Some(origin) => origin,
            None => Point::new(0.0, self.metrics().baseline),
        }
    }

    /// Sets the baseline origin of the glyph run.
    pub fn set_baseline_origin(&self, value: Point) {
        self.invalidate();
        self.baseline_origin.set(Some(value));
    }

    /// Gets the list of UTF-16 code points that represent the Unicode content of the glyph run.
    pub fn characters(&self) -> ReadOnlyMemory<u16> {
        self.characters.borrow().clone()
    }

    /// Sets the list of UTF-16 code points that represent the Unicode content of the glyph run.
    pub fn set_characters(&self, value: ReadOnlyMemory<u16>) {
        self.invalidate();
        *self.characters.borrow_mut() = value;
    }

    /// Gets the list of glyphs to use to render this run.
    pub fn glyph_infos(&self) -> GlyphInfoList {
        self.glyph_infos.borrow().clone()
    }

    /// Sets the list of glyphs to use to render this run.
    pub fn set_glyph_infos(&self, value: GlyphInfoList) {
        self.invalidate();
        *self.glyph_infos.borrow_mut() = value;
        self.has_one_char_per_cluster.set(false);
    }

    /// Gets the bidirectional nesting level of the glyph run.
    pub fn bidi_level(&self) -> i32 {
        self.bidi_level.get()
    }

    /// Sets the bidirectional nesting level of the glyph run.
    pub fn set_bidi_level(&self, value: i32) {
        self.invalidate();
        self.bidi_level.set(value);
    }

    /// Gets the scale of the current glyph typeface.
    pub(crate) fn scale(&self) -> f64 {
        self.font_rendering_em_size() / self.glyph_typeface.metrics().design_em_height as f64
    }

    /// Returns `true` if the text direction is left-to-right. Otherwise, returns `false`.
    pub fn is_left_to_right(&self) -> bool {
        (self.bidi_level.get() & 1) == 0
    }

    /// The platform implementation of the glyph run.
    pub fn platform_impl(&self) -> Rc<dyn IGlyphRunImpl> {
        if let Some(platform_impl) = self.platform_impl.borrow().as_ref() {
            return platform_impl.clone();
        }

        self.create_glyph_run_impl()
    }

    /// Obtains geometry for the glyph run.
    pub fn build_geometry(&self) -> crate::Ref<crate::media::Geometry> {
        let geometry_impl = render_interface().build_glyph_run_geometry(self);

        crate::media::PlatformGeometry::new(geometry_impl).upcast()
    }

    /// Retrieves the offset from the leading edge of the glyph run to the
    /// leading or trailing edge of a caret stop containing the specified
    /// character hit.
    pub fn get_distance_from_character_hit(&self, character_hit: CharacterHit) -> f64 {
        let list = self.glyph_infos.borrow();
        let glyph_infos = list.borrow();

        if glyph_infos.is_empty() {
            return 0.0;
        }

        let character_index = character_hit.first_character_index() + character_hit.trailing_length();
        let mut is_trailing_hit = character_hit.trailing_length() > 0;

        let mut distance = 0.0;
        let metrics = self.metrics();

        if self.is_left_to_right() {
            if character_index < metrics.first_cluster {
                return 0.0;
            }

            if character_index > metrics.last_cluster {
                return self.bounds().width;
            }

            let mut glyph_index = self.find_glyph_index_in(&glyph_infos, character_index);

            let current_cluster = glyph_infos[glyph_index].glyph_cluster;

            let in_cluster_hit = current_cluster < character_index;

            if in_cluster_hit {
                // Move to the end of the cluster.
                while glyph_index < glyph_infos.len() {
                    if glyph_infos[glyph_index].glyph_cluster > character_index {
                        break;
                    }
                    glyph_index += 1;
                }

                is_trailing_hit = false;
            }

            if is_trailing_hit {
                while glyph_index + 1 < glyph_infos.len()
                    && glyph_infos[glyph_index + 1].glyph_cluster == current_cluster
                {
                    glyph_index += 1;
                }
            }

            for glyph_info in &glyph_infos[..glyph_index] {
                distance += glyph_info.glyph_advance;
            }

            distance
        } else {
            // RightToLeft
            let glyph_index = self.find_glyph_index_in(&glyph_infos, character_index);

            if character_index > metrics.last_cluster {
                return 0.0;
            }

            if character_index <= metrics.first_cluster {
                return self.bounds().width;
            }

            for glyph_info in &glyph_infos[glyph_index + 1..] {
                distance += glyph_info.glyph_advance;
            }

            self.bounds().width - distance
        }
    }

    /// Retrieves the character hit information that corresponds to the
    /// specified distance into the glyph run. The flag tells whether the
    /// distance was inside the run.
    pub fn get_character_hit_from_distance(&self, distance: f64) -> (CharacterHit, bool) {
        let metrics = self.metrics();
        let is_left_to_right = self.is_left_to_right();

        // Before
        if distance <= 0.0 {
            let (first_character_hit, _) = self.find_nearest_character_hit(if is_left_to_right {
                metrics.first_cluster
            } else {
                metrics.last_cluster
            });

            return (
                if is_left_to_right {
                    CharacterHit::new(first_character_hit.first_character_index())
                } else {
                    first_character_hit
                },
                false,
            );
        }

        let bounds_width = self.bounds().width;

        // After
        if distance >= bounds_width {
            let (last_character_hit, _) = self.find_nearest_character_hit(if is_left_to_right {
                metrics.last_cluster
            } else {
                metrics.first_cluster
            });

            return (
                if is_left_to_right {
                    last_character_hit
                } else {
                    CharacterHit::new(last_character_hit.first_character_index())
                },
                false,
            );
        }

        let mut character_index = 0;

        // Within
        let mut current_x = 0.0;

        {
            let list = self.glyph_infos.borrow();
            let glyph_infos = list.borrow();

            if is_left_to_right {
                for glyph_info in glyph_infos.iter() {
                    let advance = glyph_info.glyph_advance;

                    character_index = glyph_info.glyph_cluster;

                    if current_x + advance > distance {
                        break;
                    }

                    current_x += advance;
                }
            } else {
                current_x = bounds_width;

                for glyph_info in glyph_infos.iter().rev() {
                    let advance = glyph_info.glyph_advance;

                    character_index = glyph_info.glyph_cluster;

                    let offset_x = current_x - advance;

                    if offset_x < distance {
                        break;
                    }

                    current_x -= advance;
                }
            }
        }

        let (character_hit, width) = self.find_nearest_character_hit(character_index);

        let delta = width / 2.0;

        let offset = if is_left_to_right {
            round_to_3(distance - current_x)
        } else {
            round_to_3(current_x - distance)
        };

        let is_trailing = offset > delta;

        (
            if is_trailing { character_hit } else { CharacterHit::new(character_hit.first_character_index()) },
            true,
        )
    }

    /// Retrieves the next valid caret character hit in the logical direction in the glyph run.
    pub fn get_next_caret_character_hit(&self, character_hit: CharacterHit) -> CharacterHit {
        if character_hit.trailing_length() == 0 {
            let (character_hit, _) = self.find_nearest_character_hit(character_hit.first_character_index());

            if character_hit.first_character_index() == self.metrics().last_cluster {
                return character_hit;
            }

            return CharacterHit::new(character_hit.first_character_index() + character_hit.trailing_length());
        }

        self.find_nearest_character_hit(character_hit.first_character_index() + character_hit.trailing_length()).0
    }

    /// Retrieves the previous valid caret character hit in the logical direction in the glyph run.
    pub fn get_previous_caret_character_hit(&self, character_hit: CharacterHit) -> CharacterHit {
        let index = if character_hit.trailing_length() > 0 {
            character_hit.first_character_index()
        } else {
            character_hit.first_character_index() - 1
        };

        let (previous_character_hit, _) = self.find_nearest_character_hit(index);

        CharacterHit::new(previous_character_hit.first_character_index())
    }

    /// Finds a glyph index for given character index.
    pub fn find_glyph_index(&self, character_index: i32) -> usize {
        let list = self.glyph_infos.borrow();
        let glyph_infos = list.borrow();
        self.find_glyph_index_in(&glyph_infos, character_index)
    }

    fn find_glyph_index_in(&self, glyph_infos: &[GlyphInfo], mut character_index: i32) -> usize {
        if glyph_infos.is_empty() {
            return 0;
        }

        if self.has_one_char_per_cluster.get() {
            return (character_index.max(0) as usize).min(glyph_infos.len() - 1);
        }

        let metrics = self.metrics();
        let is_left_to_right = self.is_left_to_right();

        if character_index > metrics.last_cluster {
            return if is_left_to_right { glyph_infos.len() - 1 } else { 0 };
        }

        if character_index < metrics.first_cluster {
            return if is_left_to_right { 0 } else { glyph_infos.len() - 1 };
        }

        let mut start = binary_search_cluster(glyph_infos, character_index, is_left_to_right);

        if start.is_err() {
            while character_index > 0 && start.is_err() {
                character_index -= 1;

                start = binary_search_cluster(glyph_infos, character_index, is_left_to_right);
            }
        }

        let Ok(mut start) = start else {
            return 0;
        };

        if is_left_to_right {
            while start > 0 && glyph_infos[start - 1].glyph_cluster == glyph_infos[start].glyph_cluster {
                start -= 1;
            }
        } else {
            while start + 1 < glyph_infos.len()
                && glyph_infos[start + 1].glyph_cluster == glyph_infos[start].glyph_cluster
            {
                start += 1;
            }
        }

        start.min(glyph_infos.len() - 1)
    }

    /// Finds the nearest character hit at given index. Also returns the width
    /// of the found cluster.
    pub fn find_nearest_character_hit(&self, index: i32) -> (CharacterHit, f64) {
        let mut width = 0.0;

        let list = self.glyph_infos.borrow();
        let glyph_infos = list.borrow();
        let metrics = self.metrics();
        let characters_length = self.characters.borrow().len() as i32;

        if glyph_infos.is_empty() {
            return (CharacterHit::with_trailing_length(metrics.first_cluster, characters_length), width);
        }

        let glyph_index = self.find_glyph_index_in(&glyph_infos, index);

        if self.has_one_char_per_cluster.get() {
            width = glyph_infos[glyph_index].glyph_advance;

            return (CharacterHit::with_trailing_length(glyph_index as i32, 1), width);
        }

        let is_left_to_right = self.is_left_to_right();

        let cluster = glyph_infos[glyph_index].glyph_cluster;

        let mut next_cluster = cluster;

        let mut current_index = glyph_index;

        while next_cluster == cluster {
            width += glyph_infos[current_index].glyph_advance;

            if is_left_to_right {
                current_index += 1;

                if current_index == glyph_infos.len() {
                    break;
                }
            } else {
                if current_index == 0 {
                    break;
                }

                current_index -= 1;
            }

            next_cluster = glyph_infos[current_index].glyph_cluster;
        }

        let mut cluster_length = (next_cluster - cluster).max(0);

        if cluster == metrics.last_cluster && cluster_length == 0 {
            let mut character_length = 0;

            let mut current_cluster = metrics.first_cluster;

            if is_left_to_right {
                for glyph_info in glyph_infos.iter().skip(1) {
                    next_cluster = glyph_info.glyph_cluster;

                    if current_cluster > cluster {
                        break;
                    }

                    let length = next_cluster - current_cluster;

                    character_length += length;

                    current_cluster = next_cluster;
                }
            } else {
                for glyph_info in glyph_infos.iter().rev() {
                    next_cluster = glyph_info.glyph_cluster;

                    if current_cluster > cluster {
                        break;
                    }

                    let length = next_cluster - current_cluster;

                    character_length += length;

                    current_cluster = next_cluster;
                }
            }

            if characters_length != 0 {
                cluster_length = characters_length - character_length;
            } else {
                cluster_length = 1;
            }
        }

        (CharacterHit::with_trailing_length(cluster, cluster_length), width)
    }

    fn create_glyph_run_metrics(&self) -> GlyphRunMetrics {
        let list = self.glyph_infos.borrow();
        let characters = self.characters.borrow();

        let (mut first_cluster, mut last_cluster) = {
            let glyph_infos = list.borrow();

            if characters.is_empty() || glyph_infos.is_empty() {
                (0, 0)
            } else {
                (glyph_infos[0].glyph_cluster, glyph_infos[glyph_infos.len() - 1].glyph_cluster)
            }
        };

        let is_reversed = first_cluster > last_cluster;

        if !self.is_left_to_right() {
            std::mem::swap(&mut first_cluster, &mut last_cluster);
        }

        let font_metrics = self.glyph_typeface.metrics();
        let scale = self.scale();

        let height = font_metrics.line_spacing() as f64 * scale;

        // A shaped buffer memoizes its total advance; ask it before borrowing
        // the glyphs.
        let shaped_total = match &*list {
            GlyphInfoList::ShapedBuffer(shaped_buffer) => Some(shaped_buffer.total_glyph_advance()),
            GlyphInfoList::List(_) => None,
        };

        let glyph_infos = list.borrow();

        let (trailing_whitespace_length, new_line_length, glyph_count) =
            Self::get_trailing_whitespace_length(&glyph_infos, characters.span(), is_reversed);

        let width_including_trailing_whitespace =
            shaped_total.unwrap_or_else(|| glyph_infos.iter().map(|glyph_info| glyph_info.glyph_advance).sum());

        let mut width = width_including_trailing_whitespace;

        if is_reversed {
            for glyph_info in &glyph_infos[..glyph_count.min(glyph_infos.len())] {
                width -= glyph_info.glyph_advance;
            }
        } else {
            for glyph_info in &glyph_infos[glyph_infos.len() - glyph_count.min(glyph_infos.len())..] {
                width -= glyph_info.glyph_advance;
            }
        }

        let ascent = font_metrics.ascent as f64 * scale;
        let line_gap = font_metrics.line_gap as f64 * scale;
        let baseline = -ascent + line_gap * 0.5;

        GlyphRunMetrics {
            baseline,
            width,
            width_including_trailing_whitespace,
            height,
            new_line_length,
            trailing_whitespace_length,
            first_cluster,
            last_cluster,
        }
    }

    /// Returns `(trailing whitespace length, new line length, glyph count)`.
    fn get_trailing_whitespace_length(
        glyph_infos: &[GlyphInfo],
        characters_span: &[u16],
        is_reversed: bool,
    ) -> (i32, i32, usize) {
        if is_reversed {
            return Self::get_trailing_whitespace_length_right_to_left(glyph_infos, characters_span);
        }

        let mut glyph_count = 0usize;
        let mut new_line_length = 0;
        let mut trailing_whitespace_length = 0;

        if !characters_span.is_empty() {
            let mut character_index = characters_span.len() as i32 - 1;

            let mut i = glyph_infos.len() as i32 - 1;
            while i >= 0 {
                let current_cluster = glyph_infos[i as usize].glyph_cluster;
                if character_index < 0 {
                    break;
                }
                let (mut codepoint, mut character_length) = Codepoint::read_at(characters_span, character_index as usize);

                character_index -= character_length as i32;

                if !codepoint.is_white_space() {
                    break;
                }

                let mut cluster_length = 1;

                while i - 1 >= 0 {
                    let next_cluster = glyph_infos[(i - 1) as usize].glyph_cluster;

                    if current_cluster == next_cluster {
                        cluster_length += 1;
                        i -= 1;

                        if character_index >= 0 {
                            (codepoint, character_length) =
                                Codepoint::read_at(characters_span, character_index as usize);

                            character_index -= character_length as i32;
                        }

                        continue;
                    }

                    break;
                }

                if codepoint.is_break_char() {
                    new_line_length += cluster_length;
                }

                trailing_whitespace_length += cluster_length;

                glyph_count += 1;

                i -= 1;
            }
        }

        (trailing_whitespace_length, new_line_length, glyph_count)
    }

    fn get_trailing_whitespace_length_right_to_left(
        glyph_infos: &[GlyphInfo],
        characters_span: &[u16],
    ) -> (i32, i32, usize) {
        let mut glyph_count = 0usize;
        let mut new_line_length = 0;
        let mut trailing_whitespace_length = 0;

        if !characters_span.is_empty() {
            let mut character_index = characters_span.len() as i32 - 1;

            for i in 0..glyph_infos.len() {
                let current_cluster = glyph_infos[i].glyph_cluster;

                if character_index < 0 {
                    break;
                }

                let (codepoint, _) = Codepoint::read_at(characters_span, character_index as usize);

                if !codepoint.is_white_space() {
                    break;
                }

                let mut cluster_length = 1;

                let mut j = i;

                while j + 1 < glyph_infos.len() {
                    j += 1;
                    let next_cluster = glyph_infos[j].glyph_cluster;

                    if current_cluster == next_cluster {
                        cluster_length += 1;

                        continue;
                    }

                    break;
                }

                character_index -= cluster_length;

                if codepoint.is_break_char() {
                    new_line_length += cluster_length;
                }

                trailing_whitespace_length += cluster_length;

                glyph_count += cluster_length as usize;
            }
        }

        (trailing_whitespace_length, new_line_length, glyph_count)
    }

    /// Drops the cached platform resources and metrics (upstream `Set<T>`).
    fn invalidate(&self) {
        release_platform_impl(self.platform_impl.borrow_mut().take());
        self.glyph_run_metrics.set(None);
    }

    fn create_glyph_run_impl(&self) -> Rc<dyn IGlyphRunImpl> {
        let platform_impl = {
            let list = self.glyph_infos.borrow();
            let glyph_infos = list.borrow();

            render_interface().create_glyph_run(
                &self.glyph_typeface,
                self.font_rendering_em_size(),
                &glyph_infos,
                self.baseline_origin(),
            )
        };

        *self.platform_impl.borrow_mut() = Some(platform_impl.clone());

        platform_impl
    }

    /// Releases the platform resources of the run.
    pub fn dispose(&self) {
        release_platform_impl(self.platform_impl.borrow_mut().take());
    }

    /// Gets the intersections of specified upper and lower limit.
    pub fn get_intersections(&self, lower_limit: f32, upper_limit: f32) -> Vec<f32> {
        self.platform_impl().get_intersections(lower_limit, upper_limit)
    }

    /// Creates an immutable reference to the platform resources of this run.
    pub fn try_create_immutable_glyph_run_reference(&self) -> Option<Rc<dyn IImmutableGlyphRunReference>> {
        Some(Rc::new(ImmutableGlyphRunReference::new(Some(self.platform_impl()))))
    }
}

/// `Math.Round(value, 3)` (banker's rounding at the third decimal).
fn round_to_3(value: f64) -> f64 {
    let scaled = value * 1000.0;
    let rounded = scaled.round();
    // Round half to even, as `Math.Round` does by default.
    let result = if (scaled - scaled.trunc()).abs() == 0.5 && rounded % 2.0 != 0.0 { rounded - scaled.signum() } else { rounded };
    result / 1000.0
}
