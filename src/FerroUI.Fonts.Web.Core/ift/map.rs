//! The patch map tables `IFT ` and `IFTX`, format 2 (section 5.3 of the standard), and which of
//! their entries a target needs (section 4.3).

use super::sparse::decode_sparse_bit_set;
use super::template::{expand_template, validate_template, EntryId};
use crate::sfnt::{ReadBe, Tag};
use crate::FontDecodeError;
use std::collections::HashSet;
use std::ops::Range;

const MALFORMED: FontDecodeError = FontDecodeError::Malformed("patch map");
/// The nodes of the sparse bit sets of one table, a limit on the work a hostile table can ask for.
const MAX_SET_NODES: usize = 1 << 22;
/// The header up to the URL template.
const HEADER_LENGTH: usize = 35;

const HAS_FEATURES: u8 = 1 << 0;
const HAS_CHILDREN: u8 = 1 << 1;
const HAS_IDS: u8 = 1 << 2;
const HAS_FORMAT: u8 = 1 << 3;
const HAS_CODE_POINTS: u8 = 1 << 4;
const HAS_BIAS: u8 = 1 << 5;
pub(super) const IGNORED: u8 = 1 << 6;
const RESERVED: u8 = 1 << 7;

/// The patch format of glyph-keyed patches; 1 and 2 are table-keyed.
pub(super) const GLYPH_KEYED: u8 = 3;

/// A segment of the design space: an axis and an inclusive range, in 16.16.
type DesignSegment = (Tag, i32, i32);

#[derive(Debug)]
enum EntryIds {
    Numbers(Vec<u32>),
    Strings(Vec<Range<u32>>),
}

#[derive(Debug)]
struct PatchEntry {
    /// Where `formatFlags` is in the table.
    flags_offset: u32,
    ignored: bool,
    format: u8,
    ids: EntryIds,
    code_points: Vec<(u32, u32)>,
    features: Vec<Tag>,
    design_space: Vec<DesignSegment>,
    children: Vec<u32>,
    conjunctive: bool,
}

/// A parsed patch map.
#[derive(Debug)]
pub struct PatchMap {
    tag: Tag,
    compatibility_id: [u8; 16],
    template: Vec<u8>,
    id_strings: Vec<u8>,
    cff_charstrings: Option<u32>,
    cff2_charstrings: Option<u32>,
    entries: Vec<PatchEntry>,
}

struct Cursor<'a> {
    data: &'a [u8],
    position: usize,
}

impl Cursor<'_> {
    fn u8(&mut self) -> Result<u8, FontDecodeError> {
        let value = self.data.u8_at(self.position).ok_or(FontDecodeError::Truncated)?;
        self.position += 1;
        Ok(value)
    }

    fn u16(&mut self) -> Result<u16, FontDecodeError> {
        let value = self.data.u16_at(self.position).ok_or(FontDecodeError::Truncated)?;
        self.position += 2;
        Ok(value)
    }

    fn u24(&mut self) -> Result<u32, FontDecodeError> {
        let bytes = self.data.get(self.position..self.position + 3).ok_or(FontDecodeError::Truncated)?;
        self.position += 3;
        Ok(u32::from(bytes[0]) << 16 | u32::from(bytes[1]) << 8 | u32::from(bytes[2]))
    }

    fn i24(&mut self) -> Result<i32, FontDecodeError> {
        Ok(((self.u24()? << 8) as i32) >> 8)
    }

    fn u32(&mut self) -> Result<u32, FontDecodeError> {
        let value = self.data.u32_at(self.position).ok_or(FontDecodeError::Truncated)?;
        self.position += 4;
        Ok(value)
    }

    fn tag(&mut self) -> Result<Tag, FontDecodeError> {
        Ok(self.u32()?.to_be_bytes())
    }
}

/// The id the next entry continues from.
#[derive(Clone)]
enum LastId {
    Number(u32),
    String(Range<u32>),
}

/// Parses a patch map table. Every entry is validated, ignored ones included, as child entries
/// count them.
pub fn parse_patch_map(tag: Tag, table: &[u8]) -> Result<PatchMap, FontDecodeError> {
    let mut cursor = Cursor { data: table, position: 0 };
    if cursor.u8()? != 2 {
        return Err(FontDecodeError::Unsupported("patch map format"));
    }
    cursor.position = 4;
    let flags = cursor.u8()?;
    let compatibility_id = table.get(5..21).ok_or(FontDecodeError::Truncated)?;
    let compatibility_id: [u8; 16] = compatibility_id.try_into().map_err(|_| MALFORMED)?;
    cursor.position = 21;
    let default_format = cursor.u8()?;
    if !(1..=3).contains(&default_format) {
        return Err(MALFORMED);
    }
    let entry_count = cursor.u24()? as usize;
    let entries_offset = cursor.u32()? as usize;
    let strings_offset = cursor.u32()? as usize;
    let template_length = cursor.u16()? as usize;
    debug_assert_eq!(cursor.position, HEADER_LENGTH);
    let template = table.get(HEADER_LENGTH..HEADER_LENGTH + template_length).ok_or(FontDecodeError::Truncated)?;
    validate_template(template)?;
    cursor.position = HEADER_LENGTH + template_length;
    let cff_charstrings = if flags & 1 != 0 { Some(cursor.u32()?) } else { None };
    let cff2_charstrings = if flags & 2 != 0 { Some(cursor.u32()?) } else { None };
    let id_strings = if strings_offset == 0 { &[][..] } else { table.get(strings_offset..).ok_or(MALFORMED)? };
    // Each entry takes at least one byte, so a count the table cannot hold fails here.
    if entry_count > table.len().saturating_sub(entries_offset) {
        return Err(FontDecodeError::Truncated);
    }
    let mut parser = EntryParser {
        cursor: Cursor { data: table, position: entries_offset },
        id_strings,
        uses_strings: strings_offset != 0,
        string_position: 0,
        last: if strings_offset == 0 { LastId::Number(0) } else { LastId::String(0..0) },
        default_format,
        nodes: MAX_SET_NODES,
    };
    let mut entries = Vec::with_capacity(entry_count.min(4096));
    for index in 0..entry_count {
        entries.push(parser.entry(index)?);
    }
    Ok(PatchMap {
        tag,
        compatibility_id,
        template: template.to_vec(),
        id_strings: id_strings.to_vec(),
        cff_charstrings,
        cff2_charstrings,
        entries,
    })
}

struct EntryParser<'a> {
    cursor: Cursor<'a>,
    id_strings: &'a [u8],
    uses_strings: bool,
    string_position: usize,
    last: LastId,
    default_format: u8,
    nodes: usize,
}

impl EntryParser<'_> {
    fn entry(&mut self, index: usize) -> Result<PatchEntry, FontDecodeError> {
        let flags_offset = u32::try_from(self.cursor.position).map_err(|_| MALFORMED)?;
        let flags = self.cursor.u8()?;
        if flags & RESERVED != 0 {
            return Err(MALFORMED);
        }
        let (mut features, mut design_space) = (Vec::new(), Vec::new());
        if flags & HAS_FEATURES != 0 {
            let count = self.cursor.u8()?;
            features.reserve(count as usize);
            for _ in 0..count {
                features.push(self.cursor.tag()?);
            }
            let count = self.cursor.u16()? as usize;
            if count.saturating_mul(12) > self.cursor.data.len().saturating_sub(self.cursor.position) {
                return Err(FontDecodeError::Truncated);
            }
            design_space.reserve(count);
            for _ in 0..count {
                let tag = self.cursor.tag()?;
                let (start, end) = (self.cursor.u32()? as i32, self.cursor.u32()? as i32);
                if start > end {
                    return Err(MALFORMED);
                }
                design_space.push((tag, start, end));
            }
        }
        let (mut children, mut conjunctive) = (Vec::new(), false);
        if flags & HAS_CHILDREN != 0 {
            let mode = self.cursor.u8()?;
            conjunctive = mode & 0x80 != 0;
            let count = mode & 0x7F;
            children.reserve(count as usize);
            for _ in 0..count {
                let child = self.cursor.u24()?;
                if child as usize >= index {
                    return Err(MALFORMED);
                }
                children.push(child);
            }
        }
        let ids = self.ids(flags)?;
        let format = if flags & HAS_FORMAT != 0 { self.cursor.u8()? } else { self.default_format };
        if !(1..=3).contains(&format) {
            return Err(MALFORMED);
        }
        let mut code_points = Vec::new();
        if flags & (HAS_CODE_POINTS | HAS_BIAS) != 0 {
            let bias = match (flags & HAS_CODE_POINTS != 0, flags & HAS_BIAS != 0) {
                (false, true) => u32::from(self.cursor.u16()?),
                (true, true) => self.cursor.u24()?,
                _ => 0,
            };
            let rest = self.cursor.data.get(self.cursor.position..).ok_or(FontDecodeError::Truncated)?;
            let (ranges, used) = decode_sparse_bit_set(rest, bias, &mut self.nodes)?;
            self.cursor.position += used;
            code_points = ranges;
        }
        Ok(PatchEntry {
            flags_offset,
            ignored: flags & IGNORED != 0,
            format,
            ids,
            code_points,
            features,
            design_space,
            children,
            conjunctive,
        })
    }

    fn ids(&mut self, flags: u8) -> Result<EntryIds, FontDecodeError> {
        if !self.uses_strings {
            let LastId::Number(mut id) = self.last else {
                return Err(MALFORMED);
            };
            let mut ids = Vec::with_capacity(1);
            if flags & HAS_IDS != 0 {
                loop {
                    let delta = self.cursor.i24()?;
                    id = u32::try_from(i64::from(id) + 1 + i64::from(delta >> 1)).map_err(|_| MALFORMED)?;
                    ids.push(id);
                    if delta & 1 == 0 {
                        break;
                    }
                }
            } else {
                id = id.checked_add(1).ok_or(MALFORMED)?;
                ids.push(id);
            }
            self.last = LastId::Number(id);
            return Ok(EntryIds::Numbers(ids));
        }
        let LastId::String(mut range) = self.last.clone() else {
            return Err(MALFORMED);
        };
        let mut ids = Vec::with_capacity(1);
        if flags & HAS_IDS != 0 {
            loop {
                let value = self.cursor.u24()?;
                let length = (value & 0x7F_FFFF) as usize;
                let end = self.string_position.checked_add(length).ok_or(MALFORMED)?;
                if end > self.id_strings.len() {
                    return Err(FontDecodeError::Truncated);
                }
                let (first, last) = (u32::try_from(self.string_position), u32::try_from(end));
                range = first.map_err(|_| MALFORMED)?..last.map_err(|_| MALFORMED)?;
                ids.push(range.clone());
                self.string_position = end;
                if value & 0x80_0000 == 0 {
                    break;
                }
            }
        } else {
            ids.push(range.clone());
        }
        self.last = LastId::String(range);
        Ok(EntryIds::Strings(ids))
    }
}

impl PatchMap {
    /// The tag of the table this map was read from.
    pub fn tag(&self) -> Tag {
        self.tag
    }

    /// The id that ties patches to this map.
    pub fn compatibility_id(&self) -> &[u8; 16] {
        &self.compatibility_id
    }

    pub(super) fn charstrings_offset(&self, tag: &Tag) -> Option<u32> {
        match tag {
            b"CFF " => self.cff_charstrings,
            b"CFF2" => self.cff2_charstrings,
            _ => None,
        }
    }

    /// Whether an entry that is still active needs a table-keyed patch.
    pub fn needs_table_keyed_patch(&self) -> bool {
        self.entries.iter().any(|entry| !entry.ignored && entry.format != GLYPH_KEYED)
    }

    fn urls(&self, entry: &PatchEntry) -> impl Iterator<Item = Result<String, FontDecodeError>> + '_ {
        let ids: Vec<EntryId<'_>> = match &entry.ids {
            EntryIds::Numbers(ids) => ids.iter().map(|id| EntryId::Number(*id)).collect(),
            EntryIds::Strings(ranges) => ranges
                .iter()
                .map(|range| EntryId::Bytes(&self.id_strings[range.start as usize..range.end as usize]))
                .collect(),
        };
        ids.into_iter().map(|id| expand_template(&self.template, id))
    }

    /// Sets the ignored bit of the first entry whose first URL is `url` ("Remove Entries from a
    /// Patch Map", section 5.3.2). The length of the table does not change.
    pub(super) fn remove_entry(&self, table: &mut [u8], url: &str) -> Result<(), FontDecodeError> {
        for entry in &self.entries {
            let first = self.urls(entry).next().transpose()?;
            if first.as_deref() == Some(url) {
                let flags = table.get_mut(entry.flags_offset as usize).ok_or(MALFORMED)?;
                *flags |= IGNORED;
                return Ok(());
            }
        }
        Ok(())
    }
}

/// What a client wants to render: the subset definition of section 3.1.
#[derive(Clone, Debug, Default)]
pub struct SubsetTarget {
    code_points: Vec<(u32, u32)>,
    features: Vec<Tag>,
    design_space: Vec<DesignSegment>,
}

impl SubsetTarget {
    /// A target of these code points, layout features and design space segments.
    pub fn new(
        code_points: impl IntoIterator<Item = u32>,
        features: impl IntoIterator<Item = Tag>,
        design_space: Vec<(Tag, i32, i32)>,
    ) -> Self {
        let mut points: Vec<u32> = code_points.into_iter().collect();
        points.sort_unstable();
        points.dedup();
        let mut ranges: Vec<(u32, u32)> = Vec::new();
        for point in points {
            match ranges.last_mut() {
                Some(range) if range.1 + 1 == point => range.1 = point,
                _ => ranges.push((point, point)),
            }
        }
        Self { code_points: ranges, features: features.into_iter().collect(), design_space }
    }

    fn intersects(&self, entry: &PatchEntry) -> bool {
        let code_points = entry.code_points.is_empty() || ranges_overlap(&entry.code_points, &self.code_points);
        let features =
            entry.features.is_empty() || entry.features.iter().any(|feature| self.features.contains(feature));
        let design_space = entry.design_space.is_empty()
            || entry.design_space.iter().any(|(tag, start, end)| {
                self.design_space.iter().any(|(axis, first, last)| axis == tag && start <= last && first <= end)
            });
        code_points && features && design_space
    }
}

fn ranges_overlap(a: &[(u32, u32)], b: &[(u32, u32)]) -> bool {
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        if a[i].1 < b[j].0 {
            i += 1;
        } else if b[j].1 < a[i].0 {
            j += 1;
        } else {
            return true;
        }
    }
    false
}

/// Whether each entry intersects the target, children included. Children come before their
/// parent, so one pass is enough and a hostile graph cannot make this exponential.
fn matching_entries(map: &PatchMap, target: &SubsetTarget) -> Vec<bool> {
    let mut matches: Vec<bool> = Vec::with_capacity(map.entries.len());
    for entry in &map.entries {
        let own = target.intersects(entry);
        let children = entry.children.is_empty()
            || if entry.conjunctive {
                entry.children.iter().all(|child| matches[*child as usize])
            } else {
                entry.children.iter().any(|child| matches[*child as usize])
            };
        matches.push(own && children);
    }
    matches
}

/// The URLs, as the map writes them, of the patches a target needs, in the order of the maps.
/// `skip` says which were already applied or failed.
pub fn patches_to_load(
    maps: &[PatchMap],
    target: &SubsetTarget,
    skip: impl Fn(&str) -> bool,
) -> Result<Vec<String>, FontDecodeError> {
    let mut seen = HashSet::new();
    let mut urls = Vec::new();
    for map in maps {
        let matches = matching_entries(map, target);
        for (entry, matched) in map.entries.iter().zip(matches) {
            if !matched || entry.ignored {
                continue;
            }
            for url in map.urls(entry) {
                let url = url?;
                if !skip(&url) && seen.insert(url.clone()) {
                    urls.push(url);
                }
            }
        }
    }
    Ok(urls)
}
