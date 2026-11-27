//! Validation of what a table holds: its whole offset graph, and the values that index into
//! something else.
//!
//! The graph is walked with the traversal of `read-fonts`, under a budget, so that subtables
//! shared many times over cannot multiply the work. On top of it come the checks the walk does
//! not make: glyph ids, lookup, feature, layer and palette indices, the order of coverage and
//! class ranges, cycles of color glyphs, and the charstrings of CFF, run with a limit on the
//! work of their subroutines. A table that fails, or that has no validator here, is not kept
//! (`docs/porting/web-fonts.md`, section 7). This is not the OpenType Sanitizer.

use super::{ReadBe, Tag};
use read_fonts::ps::cff::blend::BlendState;
use read_fonts::ps::cff::index::Index;
use read_fonts::ps::cff::CffFontRef;
use read_fonts::ps::cs::{self, CharstringContext, CharstringKind, CommandSink};
use read_fonts::ps::error::Error as CharstringError;
use read_fonts::tables as t;
use read_fonts::tables::gpos::{PairPos, PositionLookup, PositionSubtables, SinglePos, ValueRecord};
use read_fonts::traversal::{FieldType, SomeArray, SomeTable};
use read_fonts::types::{Fixed, GlyphId};
use read_fonts::{FontData, FontRead, FontReadWithArgs, ReadError};
use std::cell::Cell;
use std::collections::HashMap;

/// The deepest nesting of offsets in any table the format defines, with a wide margin.
const MAX_DEPTH: usize = 64;
/// Where the vertices of layers start in the graph of color glyphs.
const LAYER_VERTEX: u64 = 1 << 32;

/// The tables kept without a walk: their content is checked elsewhere (`head`, `maxp`, `hmtx`,
/// `vmtx`, `glyf`, `loca`), or is a program for an interpreter that takes any bytes (`fpgm`,
/// `prep`), or a plain array (`cvt `).
const CHECKED_ELSEWHERE: [&Tag; 9] = [b"cvt ", b"fpgm", b"glyf", b"head", b"hmtx", b"loca", b"maxp", b"prep", b"vmtx"];

/// The largest TrueType program kept.
const MAX_PROGRAM: usize = 1 << 20;

/// What the checks of one table compare values with.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct TableContext {
    pub glyph_count: u16,
    /// The axes of a valid `fvar`.
    pub axis_count: u16,
    /// The entries of a palette in `CPAL`, when the font has a valid one.
    pub palette_entries: Option<u16>,
}

/// How a step of a check ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Verdict {
    Pending,
    Valid,
    Invalid,
}

/// The counts of the table being walked.
#[derive(Clone, Copy, Debug, Default)]
struct Counts {
    lookups: Option<u32>,
    features: Option<u32>,
    layers_v0: u32,
    layers_v1: u32,
}

/// The references between color glyphs and layers of `COLR` version 1.
#[derive(Clone, Debug, Default)]
struct ColorGraph {
    edges: Vec<(u64, u64)>,
    root: Option<u64>,
}

impl ColorGraph {
    fn has_cycle(&mut self) -> bool {
        const ACTIVE: u8 = 1;
        const DONE: u8 = 2;
        self.edges.sort_unstable();
        self.edges.dedup();
        let edges = &self.edges;
        let children = |vertex: u64| {
            let start = edges.partition_point(|edge| edge.0 < vertex);
            let end = edges.partition_point(|edge| edge.0 <= vertex);
            &edges[start..end]
        };
        let mut state: HashMap<u64, u8> = HashMap::new();
        let mut stack: Vec<(u64, usize)> = Vec::new();
        for &(root, _) in edges {
            if state.contains_key(&root) {
                continue;
            }
            state.insert(root, ACTIVE);
            stack.push((root, 0));
            while let Some(top) = stack.last_mut() {
                let (vertex, next) = *top;
                let Some(&(_, child)) = children(vertex).get(next) else {
                    state.insert(vertex, DONE);
                    stack.pop();
                    continue;
                };
                top.1 += 1;
                match state.get(&child) {
                    Some(&ACTIVE) => return true,
                    Some(_) => {}
                    None => {
                        state.insert(child, ACTIVE);
                        stack.push((child, 0));
                    }
                }
            }
        }
        false
    }
}

type Table<'a> = Box<dyn SomeTable<'a> + 'a>;
type Array<'a> = Box<dyn SomeArray<'a> + 'a>;

enum Node<'a> {
    Table(Table<'a>),
    Array(Array<'a>),
}

impl<'a> Node<'a> {
    fn child(&self, index: usize, field: &'static str) -> Option<(&'static str, FieldType<'a>)> {
        match self {
            Self::Table(table) => table.get_field(index).map(|field| (field.name, field.value)),
            Self::Array(array) => array.get(index).map(|value| (field, value)),
        }
    }

    fn type_name(&self) -> &str {
        match self {
            Self::Table(table) => table.type_name(),
            Self::Array(array) => array.type_name(),
        }
    }
}

struct Frame<'a> {
    node: Node<'a>,
    next: usize,
    field: &'static str,
    /// The last value of an array that has to ascend.
    previous: Option<u32>,
}

/// Where a walk stopped: the index of the next child at each depth, and the state of each array.
#[derive(Clone, Debug, Default)]
struct WalkState {
    path: Vec<(usize, Option<u32>)>,
    visits: u64,
    colors: ColorGraph,
    started: bool,
}

fn glyph_value(value: &FieldType<'_>) -> Option<u32> {
    match value {
        FieldType::GlyphId16(glyph) => Some(glyph.to_u16() as u32),
        FieldType::GlyphId24(glyph) => Some(glyph.to_u32()),
        _ => None,
    }
}

fn integer(value: &FieldType<'_>) -> Option<u32> {
    match value {
        FieldType::U8(value) => Some(*value as u32),
        FieldType::U16(value) => Some(*value as u32),
        FieldType::U32(value) => Some(*value),
        _ => glyph_value(value),
    }
}

fn field_value(table: &dyn SomeTable<'_>, name: &str) -> Option<u32> {
    let mut index = 0;
    while let Some(field) = table.get_field(index) {
        if field.name == name {
            return integer(&field.value);
        }
        index += 1;
    }
    None
}

/// Whether the elements of an array are plain numbers no check looks at, so that the walk does
/// not need to visit them one by one.
fn is_plain(value: &FieldType<'_>, field: &str) -> bool {
    let number = matches!(
        value,
        FieldType::I8(_)
            | FieldType::U8(_)
            | FieldType::I16(_)
            | FieldType::U16(_)
            | FieldType::I24(_)
            | FieldType::U24(_)
            | FieldType::I32(_)
            | FieldType::U32(_)
            | FieldType::FWord(_)
            | FieldType::UfWord(_)
            | FieldType::F2Dot14(_)
            | FieldType::Fixed(_)
            | FieldType::Tag(_)
            | FieldType::NameId(_)
    );
    number && !matches!(field, "lookup_list_indices" | "feature_indices" | "palette_index")
}

struct Walker<'c> {
    context: &'c TableContext,
    counts: Counts,
}

impl Walker<'_> {
    /// The checks of one value, by where it is.
    fn check_value(&self, parent: &str, field: &str, value: &FieldType<'_>, state: &mut WalkState) -> bool {
        if let Some(glyph) = glyph_value(value) {
            if glyph >= self.context.glyph_count as u32 {
                return false;
            }
            if field == "glyph_id" && parent == "PaintColrGlyph" {
                if let Some(root) = state.colors.root {
                    state.colors.edges.push((root, glyph as u64));
                }
            }
            return true;
        }
        let Some(number) = integer(value) else {
            return true;
        };
        match field {
            "lookup_list_index" | "lookup_list_indices" => self.counts.lookups.is_none_or(|count| number < count),
            "feature_index" | "feature_indices" => self.counts.features.is_none_or(|count| number < count),
            "required_feature_index" => number == 0xFFFF || self.counts.features.is_none_or(|count| number < count),
            "palette_index" => {
                number == 0xFFFF || self.context.palette_entries.is_some_and(|entries| number < entries as u32)
            }
            _ => true,
        }
    }

    /// The checks of a table or record, as a whole, when the walk enters it.
    fn check_node(&self, node: &Node<'_>, frame_field: &str, state: &mut WalkState) -> bool {
        let Node::Table(table) = node else {
            return true;
        };
        match table.type_name() {
            "BaseGlyph" => {
                let first = field_value(table.as_ref(), "first_layer_index").unwrap_or(u32::MAX);
                let count = field_value(table.as_ref(), "num_layers").unwrap_or(u32::MAX);
                first.checked_add(count).is_some_and(|end| end <= self.counts.layers_v0)
            }
            "BaseGlyphPaint" => {
                state.colors.root = field_value(table.as_ref(), "glyph_id").map(u64::from);
                state.colors.root.is_some()
            }
            "PaintColrLayers" => {
                let first = field_value(table.as_ref(), "first_layer_index").unwrap_or(u32::MAX);
                let count = field_value(table.as_ref(), "num_layers").unwrap_or(u32::MAX);
                let Some(end) = first.checked_add(count).filter(|end| *end <= self.counts.layers_v1) else {
                    return false;
                };
                if let Some(root) = state.colors.root {
                    state.colors.edges.extend((first..end).map(|layer| (root, LAYER_VERTEX + layer as u64)));
                }
                true
            }
            _ => {
                let _ = frame_field;
                true
            }
        }
    }

    /// The order of the ranges of a coverage or class table.
    fn check_range(&self, table: &dyn SomeTable<'_>, previous: &mut Option<u32>) -> bool {
        let start = field_value(table, "start_glyph_id");
        let end = field_value(table, "end_glyph_id");
        let (Some(start), Some(end)) = (start, end) else {
            return false;
        };
        // Shipped fonts have ranges that share their last glyph with the next one, which no reader
        // is harmed by. Anything further out of order is refused.
        if start > end || previous.is_some_and(|last| start < last) {
            return false;
        }
        *previous = Some(end);
        true
    }

    fn enter<'a>(
        &self,
        frames: &mut Vec<Frame<'a>>,
        name: &'static str,
        value: FieldType<'a>,
        state: &mut WalkState,
    ) -> bool {
        let parent = frames.last().map_or("", |frame| frame.node.type_name()).to_owned();
        // The table an element of an array belongs to.
        let owner = match frames.as_slice() {
            [.., owner, array] if matches!(array.node, Node::Array(_)) => owner.node.type_name().to_owned(),
            _ => parent.clone(),
        };
        // The traversal resolves these against the wrong base inside computed arrays, and
        // represents these as opaque: `check_value_records` and `check_tuple_stores` read them.
        if (parent == "ValueRecord" && name.ends_with("_device"))
            || (parent == "Gvar" && name == "glyph_variation_data_offsets")
        {
            return true;
        }
        let node = match value {
            // An offset of zero is null in every reader, also where the format does not call it
            // optional: shipped fonts have it for class definitions and lookup lists.
            FieldType::ResolvedOffset(offset) => match offset.target {
                Ok(target) => Node::Table(target),
                Err(_) => return offset.offset.to_u32() == 0,
            },
            FieldType::ArrayOffset(offset) => match offset.target {
                Ok(target) => Node::Array(target),
                Err(_) => return offset.offset.to_u32() == 0,
            },
            FieldType::StringOffset(offset) => return offset.target.is_ok(),
            FieldType::Record(record) => Node::Table(Box::new(record)),
            FieldType::Array(array) => Node::Array(array),
            // An offset the traversal does not follow is one nothing has checked.
            FieldType::BareOffset(offset) => return offset.to_u32() == 0,
            FieldType::Unknown => return false,
            value => {
                if owner == "CoverageFormat1" && name == "glyph_array" {
                    if let (Some(glyph), Some(top)) = (glyph_value(&value), frames.last_mut()) {
                        // As for ranges: a glyph listed twice is in shipped fonts, and harms no reader.
                        if top.previous.is_some_and(|last| glyph < last) {
                            return false;
                        }
                        top.previous = Some(glyph);
                    }
                }
                return self.check_value(&parent, name, &value, state);
            }
        };
        if frames.len() >= MAX_DEPTH {
            return false;
        }
        if let Node::Table(table) = &node {
            if matches!(table.type_name(), "RangeRecord" | "ClassRangeRecord") {
                let Some(top) = frames.last_mut() else {
                    return false;
                };
                let mut previous = top.previous;
                if !self.check_range(table.as_ref(), &mut previous) {
                    return false;
                }
                top.previous = previous;
            }
        }
        if owner == "LayerList" && name == "paint_offsets" {
            let index = frames.last().map_or(0, |frame| frame.next.saturating_sub(1));
            state.colors.root = Some(LAYER_VERTEX + index as u64);
        }
        if !self.check_node(&node, name, state) {
            return false;
        }
        frames.push(Frame { node, next: 0, field: name, previous: None });
        true
    }

    /// Walks until the budget is spent.
    fn walk<'a>(&self, root: Table<'a>, limit: u64, state: &mut WalkState, budget: &mut usize) -> Verdict {
        let mut frames = vec![Frame { node: Node::Table(root), next: 0, field: "", previous: None }];
        if !state.started {
            state.started = true;
        } else if !self.resume(&mut frames, state) {
            return Verdict::Invalid;
        }
        while let Some(top) = frames.last_mut() {
            if *budget == 0 {
                state.path = frames.iter().map(|frame| (frame.next, frame.previous)).collect();
                return Verdict::Pending;
            }
            state.visits += 1;
            *budget -= 1;
            if state.visits > limit {
                return Verdict::Invalid;
            }
            let index = top.next;
            let Some((name, value)) = top.node.child(index, top.field) else {
                frames.pop();
                continue;
            };
            top.next += 1;
            // An array of plain numbers has nothing further to look at.
            if matches!(top.node, Node::Array(_)) && index == 0 && is_plain(&value, name) {
                frames.pop();
                continue;
            }
            if !self.enter(&mut frames, name, value, state) {
                return Verdict::Invalid;
            }
        }
        if state.colors.has_cycle() {
            return Verdict::Invalid;
        }
        Verdict::Valid
    }

    /// Goes back down to where the walk stopped.
    fn resume<'a>(&self, frames: &mut Vec<Frame<'a>>, state: &WalkState) -> bool {
        for depth in 0..state.path.len() {
            let (next, previous) = state.path[depth];
            let Some(top) = frames.last_mut() else {
                return false;
            };
            top.next = next;
            top.previous = previous;
            if depth + 1 == state.path.len() {
                break;
            }
            let Some(index) = next.checked_sub(1) else {
                return false;
            };
            let Some((name, value)) = top.node.child(index, top.field) else {
                return false;
            };
            let node = match value {
                FieldType::ResolvedOffset(offset) => offset.target.ok().map(Node::Table),
                FieldType::ArrayOffset(offset) => offset.target.ok().map(Node::Array),
                FieldType::Record(record) => Some(Node::Table(Box::new(record) as Table<'a>)),
                FieldType::Array(array) => Some(Node::Array(array)),
                _ => None,
            };
            let Some(node) = node else {
                return false;
            };
            frames.push(Frame { node, next: 0, field: name, previous: None });
        }
        true
    }
}

/// The typed root of a table; `None` when there is no validator for it.
fn root_table<'a>(tag: &Tag, data: &'a [u8], context: &TableContext) -> Option<Result<Table<'a>, ReadError>> {
    fn boxed<'a, T: SomeTable<'a> + 'a>(table: Result<T, ReadError>) -> Result<Table<'a>, ReadError> {
        table.map(|table| Box::new(table) as Table<'a>)
    }
    let font = FontData::new(data);
    let glyphs = context.glyph_count;
    Some(match tag {
        b"BASE" => boxed(t::base::Base::read(font)),
        b"CBDT" => boxed(t::cbdt::Cbdt::read(font)),
        b"CBLC" => boxed(t::cblc::Cblc::read(font)),
        b"COLR" => boxed(t::colr::Colr::read(font)),
        b"CPAL" => boxed(t::cpal::Cpal::read(font)),
        b"EBDT" => boxed(t::ebdt::Ebdt::read(font)),
        b"EBLC" => boxed(t::eblc::Eblc::read(font)),
        b"GDEF" => boxed(t::gdef::Gdef::read(font)),
        b"GPOS" => boxed(t::gpos::Gpos::read(font)),
        b"GSUB" => boxed(t::gsub::Gsub::read(font)),
        b"HVAR" => boxed(t::hvar::Hvar::read(font)),
        b"MVAR" => boxed(t::mvar::Mvar::read(font)),
        b"OS/2" => boxed(t::os2::Os2::read(font)),
        b"STAT" => boxed(t::stat::Stat::read(font)),
        b"SVG " => boxed(t::svg::Svg::read(font)),
        b"VORG" => boxed(t::vorg::Vorg::read(font)),
        b"VVAR" => boxed(t::vvar::Vvar::read(font)),
        b"avar" => boxed(t::avar::Avar::read(font)),
        b"cmap" => boxed(t::cmap::Cmap::read(font)),
        b"fvar" => boxed(t::fvar::Fvar::read(font)),
        b"gasp" => boxed(t::gasp::Gasp::read(font)),
        b"gvar" => boxed(t::gvar::Gvar::read(font)),
        b"hdmx" => boxed(t::hdmx::Hdmx::read_with_args(font, &glyphs)),
        b"hhea" => boxed(t::hhea::Hhea::read(font)),
        b"ltag" => boxed(t::ltag::Ltag::read(font)),
        b"name" => boxed(t::name::Name::read(font)),
        b"post" => boxed(t::post::Post::read(font)),
        b"sbix" => boxed(t::sbix::Sbix::read_with_args(font, &(glyphs as u32).try_into().ok()?)),
        b"vhea" => boxed(t::vhea::Vhea::read(font)),
        _ => return None,
    })
}

fn table_counts(tag: &Tag, data: &[u8]) -> Counts {
    let font = FontData::new(data);
    let mut counts = Counts::default();
    match tag {
        b"GSUB" => {
            if let Ok(table) = t::gsub::Gsub::read(font) {
                counts.lookups = Some(table.lookup_list().map_or(0, |list| list.lookup_count() as u32));
                counts.features = Some(table.feature_list().map_or(0, |list| list.feature_count() as u32));
            }
        }
        b"GPOS" => {
            if let Ok(table) = t::gpos::Gpos::read(font) {
                counts.lookups = Some(table.lookup_list().map_or(0, |list| list.lookup_count() as u32));
                counts.features = Some(table.feature_list().map_or(0, |list| list.feature_count() as u32));
            }
        }
        b"COLR" => {
            if let Ok(table) = t::colr::Colr::read(font) {
                counts.layers_v0 = table.num_layer_records() as u32;
                counts.layers_v1 = table.layer_list().and_then(Result::ok).map_or(0, |list| list.num_layers());
            }
        }
        _ => {}
    }
    counts
}

/// Whether a table holds the fields its version promises. The traversal of `read-fonts` 0.41
/// unwraps exactly these version- and flag-dependent fields, so that a table too short for them
/// would panic. The fields are listed next to the dependency, in
/// `[package.metadata.read-fonts-guards]` of this crate, and a test ties that list to the pin.
pub(super) fn has_versioned_fields(tag: &Tag, data: &[u8]) -> bool {
    let check = || -> Option<bool> {
        let (major, minor) = (data.u16_at(0)?, data.u16_at(2).unwrap_or(0));
        Some(match tag {
            b"BASE" => minor == 0 || data.len() >= 12,
            b"COLR" => major == 0 || data.len() >= 34,
            b"CPAL" => major == 0 || data.len() >= 12 + data.u16_at(4)? as usize * 2 + 12,
            b"GDEF" => (minor < 2 || data.len() >= 14) && (minor < 3 || data.len() >= 18),
            b"GPOS" | b"GSUB" => (minor == 0 || data.len() >= 14) && lookups_have_their_fields(data)?,
            b"STAT" => minor == 0 || data.len() >= 20,
            b"avar" => {
                major < 2 || {
                    let mut offset = 8usize;
                    for _ in 0..data.u16_at(6)? {
                        offset = offset.checked_add(2 + data.u16_at(offset)? as usize * 4)?;
                    }
                    offset.checked_add(8)? <= data.len()
                }
            }
            b"name" => {
                major == 0 || {
                    let tags = 6 + data.u16_at(2)? as usize * 12;
                    tags + 2 + data.u16_at(tags)? as usize * 4 <= data.len()
                }
            }
            b"OS/2" => {
                data.len()
                    >= match major {
                        0 => 68,
                        1 => 86,
                        2..=4 => 96,
                        _ => 100,
                    }
            }
            b"post" => major != 2 || 34 + data.u16_at(32)? as usize * 2 <= data.len(),
            _ => true,
        })
    };
    check().unwrap_or(false)
}

/// `meta`: the data of each record inside the table. It is opaque to every reader.
fn meta_is_valid(data: &[u8]) -> Option<bool> {
    if data.u32_at(0)? != 1 {
        return Some(false);
    }
    for index in 0..data.u32_at(12)? as usize {
        let record = index.checked_mul(12)?.checked_add(16)?;
        let (offset, length) = (data.u32_at(record + 4)? as usize, data.u32_at(record + 8)? as usize);
        if offset.checked_add(length)? > data.len() {
            return Some(false);
        }
    }
    Some(true)
}

/// The glyph names of `post` version 2: one index a glyph, each to a standard name or to one of
/// the Pascal strings that follow, which the traversal does not read.
fn post_names_are_valid(data: &[u8], glyph_count: u16) -> Option<bool> {
    if data.u32_at(0)? != 0x0002_0000 {
        return Some(true);
    }
    let count = data.u16_at(32)?;
    if count != glyph_count {
        return Some(false);
    }
    let mut at = 34 + count as usize * 2;
    let mut strings = 0usize;
    while at < data.len() {
        at = at.checked_add(1 + data[at] as usize)?;
        strings += 1;
    }
    if at != data.len() {
        return Some(false);
    }
    for index in 0..count as usize {
        if data.u16_at(34 + index * 2)? as usize >= 258 + strings {
            return Some(false);
        }
    }
    Some(true)
}

/// Whether each lookup that uses a mark filtering set has the field for it.
fn lookups_have_their_fields(data: &[u8]) -> Option<bool> {
    const USE_MARK_FILTERING_SET: u16 = 0x0010;
    let list = data.u16_at(8)? as usize;
    if list == 0 {
        return Some(true);
    }
    for index in 0..data.u16_at(list)? as usize {
        let lookup = list + data.u16_at(list + 2 + index * 2)? as usize;
        if data.u16_at(lookup + 2)? & USE_MARK_FILTERING_SET != 0 {
            data.u16_at(lookup + 6 + data.u16_at(lookup + 4)? as usize * 2)?;
        }
    }
    Some(true)
}

/// The device offsets of a value record, against the base they are relative to.
fn value_record_is_valid(record: &ValueRecord, base: FontData<'_>) -> bool {
    [
        record.x_placement_device(base),
        record.y_placement_device(base),
        record.x_advance_device(base),
        record.y_advance_device(base),
    ]
    .iter()
    .all(|device| !matches!(device, Some(Err(_))))
}

/// The value records of one lookup of `GPOS`.
fn lookup_value_records_are_valid(lookup: &PositionLookup<'_>) -> Option<bool> {
    let mut valid = true;
    match lookup.subtables().ok()? {
        PositionSubtables::Single(subtables) => {
            for subtable in subtables.iter() {
                match subtable.ok()? {
                    SinglePos::Format1(table) => {
                        valid &= value_record_is_valid(&table.value_record(), table.offset_data())
                    }
                    SinglePos::Format2(table) => {
                        for record in table.value_records().iter() {
                            valid &= value_record_is_valid(&record.ok()?, table.offset_data());
                        }
                    }
                }
            }
        }
        PositionSubtables::Pair(subtables) => {
            for subtable in subtables.iter() {
                match subtable.ok()? {
                    PairPos::Format1(table) => {
                        for set in table.pair_sets().iter() {
                            let set = set.ok()?;
                            for pair in set.pair_value_records().iter() {
                                let pair = pair.ok()?;
                                valid &= value_record_is_valid(pair.value_record1(), set.offset_data());
                                valid &= value_record_is_valid(pair.value_record2(), set.offset_data());
                            }
                        }
                    }
                    PairPos::Format2(table) => {
                        for first in table.class1_records().iter() {
                            let first = first.ok()?;
                            for second in first.class2_records().iter() {
                                let second = second.ok()?;
                                valid &= value_record_is_valid(second.value_record1(), table.offset_data());
                                valid &= value_record_is_valid(second.value_record2(), table.offset_data());
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
    Some(valid)
}

/// Checks the value records of `GPOS`, a lookup at a time.
fn check_value_records(data: &[u8], next: &mut u32, budget: &mut usize) -> Verdict {
    if data.u16_at(8) == Some(0) {
        return Verdict::Valid;
    }
    let Ok(table) = t::gpos::Gpos::read(FontData::new(data)) else {
        return Verdict::Invalid;
    };
    let Ok(list) = table.lookup_list() else {
        return Verdict::Invalid;
    };
    let lookups = list.lookups();
    while (*next as usize) < lookups.len() {
        if *budget == 0 {
            return Verdict::Pending;
        }
        let Ok(lookup) = lookups.get(*next as usize) else {
            return Verdict::Invalid;
        };
        if lookup_value_records_are_valid(&lookup) != Some(true) {
            return Verdict::Invalid;
        }
        *budget = budget.saturating_sub(256);
        *next += 1;
    }
    Verdict::Valid
}

/// Walks packed point numbers; returns the position after them.
fn skip_packed_points(data: &[u8], mut at: usize, end: usize) -> Option<usize> {
    let first = *data.get(at)?;
    at += 1;
    let count = if first & 0x80 != 0 {
        let low = *data.get(at)?;
        at += 1;
        ((first & 0x7F) as usize) << 8 | low as usize
    } else {
        first as usize
    };
    let mut seen = 0;
    while seen < count {
        let control = *data.get(at)?;
        at += 1;
        let run = (control & 0x7F) as usize + 1;
        at = at.checked_add(run * if control & 0x80 != 0 { 2 } else { 1 })?;
        seen += run;
    }
    (seen == count && at <= end).then_some(at)
}

/// Walks packed deltas to the end of their data, which they must not run past.
fn packed_deltas_fit(data: &[u8], mut at: usize, end: usize) -> Option<bool> {
    while at < end {
        let control = *data.get(at)?;
        at += 1;
        let run = (control & 0x3F) as usize + 1;
        at = at.checked_add(match control & 0xC0 {
            0x80 => 0,
            0x40 => run * 2,
            0xC0 => run * 4,
            _ => run,
        })?;
    }
    Some(at == end)
}

/// A tuple variation store: its headers, and the points and deltas of each tuple. `store` is
/// where offsets start from and `headers` where the first header is.
fn tuple_store_is_valid(
    data: &[u8],
    store: usize,
    headers: usize,
    end: usize,
    axes: usize,
    shared: u16,
) -> Option<bool> {
    const SHARED_POINTS: u16 = 0x8000;
    const EMBEDDED_PEAK: u16 = 0x8000;
    const INTERMEDIATE: u16 = 0x4000;
    const PRIVATE_POINTS: u16 = 0x2000;
    let flags = data.u16_at(headers - 4)?;
    let mut serialized = store.checked_add(data.u16_at(headers - 2)? as usize)?;
    if serialized > end {
        return Some(false);
    }
    if flags & SHARED_POINTS != 0 {
        serialized = skip_packed_points(data, serialized, end)?;
    }
    let mut header = headers;
    for _ in 0..flags & 0x0FFF {
        let size = data.u16_at(header)? as usize;
        let index = data.u16_at(header + 2)?;
        header += 4;
        if index & EMBEDDED_PEAK != 0 {
            header += axes * 2;
        } else if index & 0x0FFF >= shared {
            return Some(false);
        }
        if index & INTERMEDIATE != 0 {
            header += axes * 4;
        }
        if header > end {
            return Some(false);
        }
        let tuple_end = serialized.checked_add(size).filter(|tuple_end| *tuple_end <= end)?;
        let deltas =
            if index & PRIVATE_POINTS != 0 { skip_packed_points(data, serialized, tuple_end)? } else { serialized };
        if !packed_deltas_fit(data, deltas, tuple_end)? {
            return Some(false);
        }
        serialized = tuple_end;
    }
    Some(true)
}

/// Checks the tuple variation stores of `gvar`, a glyph at a time.
fn check_gvar_stores(data: &[u8], next: &mut u32, budget: &mut usize) -> Option<Verdict> {
    let axes = data.u16_at(4)? as usize;
    let shared = data.u16_at(6)?;
    let glyphs = data.u16_at(12)? as u32;
    let long = data.u16_at(14)? & 1 != 0;
    let base = data.u32_at(16)? as usize;
    let offset = |glyph: u32| -> Option<usize> {
        let glyph = glyph as usize;
        Some(
            base + if long { data.u32_at(20 + glyph * 4)? as usize } else { data.u16_at(20 + glyph * 2)? as usize * 2 },
        )
    };
    while *next < glyphs {
        if *budget == 0 {
            return Some(Verdict::Pending);
        }
        let (start, end) = (offset(*next)?, offset(*next + 1)?);
        if end > data.len() || start > end {
            return Some(Verdict::Invalid);
        }
        if start < end && tuple_store_is_valid(data, start, start + 4, end, axes, shared) != Some(true) {
            return Some(Verdict::Invalid);
        }
        *budget = budget.saturating_sub((end - start).max(16));
        *next += 1;
    }
    Some(Verdict::Valid)
}

/// Limits the work of charstrings: each subroutine call spends its length.
struct FuelContext<'c, C> {
    inner: &'c C,
    fuel: &'c Cell<u64>,
}

impl<C> FuelContext<'_, C> {
    fn spend<'d>(&self, code: Result<&'d [u8], CharstringError>) -> Result<&'d [u8], CharstringError> {
        let code = code?;
        let cost = code.len() as u64 + 1;
        let left = self.fuel.get().checked_sub(cost).ok_or(CharstringError::Read(ReadError::OutOfBounds))?;
        self.fuel.set(left);
        Ok(code)
    }
}

impl<C: CharstringContext> CharstringContext for FuelContext<'_, C> {
    fn kind(&self) -> CharstringKind {
        self.inner.kind()
    }

    fn seac_components(&self, base: i32, accent: i32) -> Result<[&[u8]; 2], CharstringError> {
        let [base, accent] = self.inner.seac_components(base, accent)?;
        Ok([self.spend(Ok(base))?, self.spend(Ok(accent))?])
    }

    fn global_subr(&self, index: i32) -> Result<&[u8], CharstringError> {
        self.spend(self.inner.global_subr(index))
    }

    fn subr(&self, index: i32) -> Result<&[u8], CharstringError> {
        self.spend(self.inner.subr(index))
    }

    fn weight_vector(&self) -> &[Fixed] {
        self.inner.weight_vector()
    }
}

struct NoOutline;

impl CommandSink for NoOutline {
    fn move_to(&mut self, _: Fixed, _: Fixed) {}
    fn line_to(&mut self, _: Fixed, _: Fixed) {}
    fn curve_to(&mut self, _: Fixed, _: Fixed, _: Fixed, _: Fixed, _: Fixed, _: Fixed) {}
    fn close(&mut self) {}
}

/// Runs the charstrings of `CFF ` or `CFF2` from `next` until the budget is spent.
fn check_charstrings(
    tag: &Tag,
    data: &[u8],
    glyphs: u16,
    next: &mut u32,
    fuel: &Cell<u64>,
    budget: &mut usize,
) -> Verdict {
    let font = if tag == b"CFF2" { CffFontRef::new_cff2(data, None) } else { CffFontRef::new_cff(data, 0, None) };
    let Ok(font) = font else {
        return Verdict::Invalid;
    };
    if font.num_glyphs() != glyphs as u32 {
        return Verdict::Invalid;
    }
    let is_cff2 = tag == b"CFF2";
    let mut current: Option<(u16, Index<'_>)> = None;
    while *next < glyphs as u32 {
        if *budget == 0 {
            return Verdict::Pending;
        }
        let glyph = GlyphId::new(*next);
        let Some(subfont_index) = font.subfont_index(glyph) else {
            return Verdict::Invalid;
        };
        if current.as_ref().is_none_or(|(index, _)| *index != subfont_index) {
            let Ok(subfont) = font.subfont(subfont_index, &[]) else {
                return Verdict::Invalid;
            };
            let subrs = match subfont.subrs_offset() {
                0 => Index::Empty,
                offset => match data.get(offset as usize..).map(|bytes| Index::new(bytes, is_cff2)) {
                    Some(Ok(index)) => index,
                    _ => return Verdict::Invalid,
                },
            };
            current = Some((subfont_index, subrs));
        }
        let Some((_, subrs)) = current.as_ref() else {
            return Verdict::Invalid;
        };
        let Ok(code) = font.charstrings().get(*next as usize) else {
            return Verdict::Invalid;
        };
        let Some(left) = fuel.get().checked_sub(code.len() as u64 + 1) else {
            return Verdict::Invalid;
        };
        fuel.set(left);
        let inner = (data, font.charstrings(), font.global_subrs(), subrs);
        let context = FuelContext { inner: &inner, fuel };
        let blend = font.var_store().and_then(|store| BlendState::new(store.clone(), &[], 0).ok());
        let mut sink = NoOutline;
        if cs::evaluate(&context, blend, code, &mut sink).is_err() {
            return Verdict::Invalid;
        }
        *budget = budget.saturating_sub(code.len().max(16));
        *next += 1;
    }
    Verdict::Valid
}

/// The OpenType form of `kern` with subtables of format 0, the only form the traversal has no
/// walk for and text still uses: sorted pairs of glyphs that exist.
fn valid_kern(data: &[u8], glyphs: u16) -> bool {
    let check = || -> Option<bool> {
        if data.u16_at(0)? != 0 {
            return Some(false);
        }
        let mut offset = 4usize;
        for _ in 0..data.u16_at(2)? {
            let length = data.u16_at(offset + 2)? as usize;
            let format = data.u16_at(offset + 4)? >> 8;
            let pairs = data.u16_at(offset + 6)? as usize;
            if format != 0 || length < 14 + pairs * 6 || offset.checked_add(length)? > data.len() {
                return Some(false);
            }
            let mut previous = None;
            for index in 0..pairs {
                let pair = offset + 14 + index * 6;
                let (left, right) = (data.u16_at(pair)?, data.u16_at(pair + 2)?);
                let key = (left as u32) << 16 | right as u32;
                if left >= glyphs || right >= glyphs || previous.is_some_and(|last| key <= last) {
                    return Some(false);
                }
                previous = Some(key);
            }
            offset += length;
        }
        Some(true)
    };
    check().unwrap_or(false)
}

/// The check of one table, resumable across steps.
#[derive(Clone, Debug)]
pub(super) struct TableCheck {
    tag: Tag,
    walk: WalkState,
    next_glyph: u32,
    fuel: u64,
    /// Whether the checks that come before the walk are done.
    prepared: bool,
}

impl TableCheck {
    pub(super) fn new(tag: Tag, length: usize) -> Self {
        // The work a charstring may do is proportional to the size of the font.
        let fuel = (length as u64).saturating_mul(64).max(1 << 22);
        Self { tag, walk: WalkState::default(), next_glyph: 0, fuel, prepared: false }
    }

    /// Does up to `budget` units of work on the bytes of the table.
    pub(super) fn step(&mut self, data: &[u8], context: &TableContext, budget: &mut usize) -> Verdict {
        let tag = self.tag;
        if CHECKED_ELSEWHERE.contains(&&tag) {
            return match &tag {
                b"cvt " if data.len() % 2 != 0 => Verdict::Invalid,
                b"fpgm" | b"prep" if data.len() > MAX_PROGRAM => Verdict::Invalid,
                _ => Verdict::Valid,
            };
        }
        if tag == *b"meta" {
            return if meta_is_valid(data) == Some(true) { Verdict::Valid } else { Verdict::Invalid };
        }
        if tag == *b"kern" {
            *budget = budget.saturating_sub(data.len() / 16);
            return if valid_kern(data, context.glyph_count) { Verdict::Valid } else { Verdict::Invalid };
        }
        if matches!(&tag, b"CFF " | b"CFF2") {
            let fuel = Cell::new(self.fuel);
            let verdict = check_charstrings(&tag, data, context.glyph_count, &mut self.next_glyph, &fuel, budget);
            self.fuel = fuel.get();
            return verdict;
        }
        if !self.prepared {
            if !has_versioned_fields(&tag, data) {
                return Verdict::Invalid;
            }
            let verdict = match &tag {
                b"GPOS" => check_value_records(data, &mut self.next_glyph, budget),
                b"post" => match post_names_are_valid(data, context.glyph_count) {
                    Some(true) => Verdict::Valid,
                    _ => Verdict::Invalid,
                },
                b"gvar" => check_gvar_stores(data, &mut self.next_glyph, budget).unwrap_or(Verdict::Invalid),
                b"cvar" => match data.u16_at(6).map(|offset| offset as usize <= data.len()) {
                    Some(true) => {
                        let store = tuple_store_is_valid(data, 0, 8, data.len(), context.axis_count as usize, 0);
                        if store == Some(true) {
                            Verdict::Valid
                        } else {
                            Verdict::Invalid
                        }
                    }
                    _ => Verdict::Invalid,
                },
                _ => Verdict::Valid,
            };
            if verdict != Verdict::Valid {
                return verdict;
            }
            self.prepared = true;
            // The traversal has no fields for `cvar`: its store was read above, and is all it holds.
            if tag == *b"cvar" {
                return Verdict::Valid;
            }
        }
        let root = match root_table(&tag, data, context) {
            Some(Ok(root)) => root,
            _ => return Verdict::Invalid,
        };
        let walker = Walker { context, counts: table_counts(&tag, data) };
        // Shared subtables can be visited many times, but not without end.
        let limit = (data.len() as u64).saturating_mul(32).clamp(1 << 16, 1 << 28);
        walker.walk(root, limit, &mut self.walk, budget)
    }
}

#[cfg(test)]
pub(super) fn has_color_cycle(edges: &[(u64, u64)]) -> bool {
    ColorGraph { edges: edges.to_vec(), root: None }.has_cycle()
}
