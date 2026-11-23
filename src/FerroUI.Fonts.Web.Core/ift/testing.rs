//! An encoder of incremental fonts for tests: fixtures are built at test time from fonts of the
//! repository, so no font file has to be added.

use crate::sfnt::{assemble, ReadBe, Tag, TRUETYPE};
use crate::woff2::testing::font_tables;
use crate::WebFontLimits;
use read_fonts::{FontRef, TableProvider};
use std::collections::BTreeSet;

/// The compatibility id of the fixtures.
pub const FIXTURE_ID: [u8; 16] = *b"incremental font";

/// An incremental font and its patches, by the URL its map writes for them.
pub struct IncrementalFixture {
    pub font: Vec<u8>,
    pub patches: Vec<(String, Vec<u8>)>,
}

/// `font` with one more table.
pub fn add_table(font: &[u8], tag: Tag, table: Vec<u8>) -> Vec<u8> {
    let mut tables = font_tables(font);
    tables.push((tag, table));
    let mut references: Vec<(Tag, &[u8])> = tables.iter().map(|(tag, data)| (*tag, data.as_slice())).collect();
    assemble(TRUETYPE, &mut references, &WebFontLimits::DEFAULT).unwrap()
}

/// A sparse bit set with a branch factor of 4.
pub fn encode_sparse_bit_set(values: &[u32]) -> Vec<u8> {
    let values: BTreeSet<u32> = values.iter().copied().collect();
    let Some(&max) = values.last() else {
        return vec![0b01];
    };
    let mut height = 1u32;
    while 4u64.pow(height) <= u64::from(max) {
        height += 1;
    }
    let mut output = vec![0b01 | (height as u8) << 2];
    let (mut buffer, mut bits) = (0u32, 0u32);
    for depth in 1..=height {
        let shift = 2 * (height - depth + 1);
        let parents: BTreeSet<u32> = values.iter().map(|value| (u64::from(*value) >> shift) as u32).collect();
        for parent in parents {
            let mut node = 0u32;
            for value in &values {
                if (u64::from(*value) >> shift) as u32 == parent {
                    node |= 1 << ((value >> (2 * (height - depth))) & 3);
                }
            }
            buffer |= node << bits;
            bits += 4;
            while bits >= 8 {
                output.push(buffer as u8);
                buffer >>= 8;
                bits -= 8;
            }
        }
    }
    if bits > 0 {
        output.push(buffer as u8);
    }
    output
}

/// A format 2 patch map with one entry per group of code points, with ids 1, 2 and so on.
pub fn patch_map(
    id: &[u8; 16],
    template: &[u8],
    groups: &[&[u32]],
    format: u8,
    charstrings: Option<(u8, u32)>,
) -> Vec<u8> {
    let extra = if charstrings.is_some() { 4 } else { 0 };
    let entries_at = 35 + template.len() + extra;
    let mut table = vec![2, 0, 0, 0, charstrings.map_or(0, |(flag, _)| flag)];
    table.extend_from_slice(id);
    table.push(format);
    table.extend_from_slice(&(groups.len() as u32).to_be_bytes()[1..]);
    table.extend_from_slice(&(entries_at as u32).to_be_bytes());
    table.extend_from_slice(&0u32.to_be_bytes());
    table.extend_from_slice(&(template.len() as u16).to_be_bytes());
    table.extend_from_slice(template);
    if let Some((_, offset)) = charstrings {
        table.extend_from_slice(&offset.to_be_bytes());
    }
    for group in groups {
        table.push(1 << 4);
        table.extend_from_slice(&encode_sparse_bit_set(group));
    }
    table
}

/// A glyph-keyed patch of the given tables, each with the same glyphs in ascending order.
pub fn glyph_patch(
    id: &[u8; 16],
    tables: &[(Tag, Vec<(u16, Vec<u8>)>)],
    compress: &dyn Fn(&[u8]) -> Vec<u8>,
) -> Vec<u8> {
    let glyphs: Vec<u16> = tables.first().map_or(Vec::new(), |(_, data)| data.iter().map(|(id, _)| *id).collect());
    let mut body = Vec::new();
    body.extend_from_slice(&(glyphs.len() as u32).to_be_bytes());
    body.push(tables.len() as u8);
    for glyph in &glyphs {
        body.extend_from_slice(&glyph.to_be_bytes());
    }
    for (tag, _) in tables {
        body.extend_from_slice(tag);
    }
    let mut offset = body.len() + (glyphs.len() * tables.len() + 1) * 4;
    for (_, data) in tables {
        for (_, bytes) in data {
            body.extend_from_slice(&(offset as u32).to_be_bytes());
            offset += bytes.len();
        }
    }
    body.extend_from_slice(&(offset as u32).to_be_bytes());
    for (_, data) in tables {
        for (_, bytes) in data {
            body.extend_from_slice(bytes);
        }
    }
    let mut patch = b"ifgk".to_vec();
    patch.extend_from_slice(&[0; 5]);
    patch.extend_from_slice(id);
    patch.extend_from_slice(&(body.len() as u32).to_be_bytes());
    patch.extend_from_slice(&compress(&body));
    patch
}

/// The component glyphs of a composite glyph.
fn components(glyph: &[u8]) -> Vec<u16> {
    let mut found = Vec::new();
    if glyph.len() < 10 || glyph.i16_at(0).unwrap() >= 0 {
        return found;
    }
    let mut at = 10;
    loop {
        let flags = glyph.u16_at(at).unwrap();
        found.push(glyph.u16_at(at + 2).unwrap());
        at += 4 + if flags & 1 != 0 { 4 } else { 2 };
        at += match flags {
            flags if flags & 0x08 != 0 => 2,
            flags if flags & 0x40 != 0 => 4,
            flags if flags & 0x80 != 0 => 8,
            _ => 0,
        };
        if flags & 0x20 == 0 {
            return found;
        }
    }
}

fn glyph_range(loca: &[u8], long: bool, glyph: usize) -> (usize, usize) {
    if long {
        (loca.u32_at(glyph * 4).unwrap() as usize, loca.u32_at(glyph * 4 + 4).unwrap() as usize)
    } else {
        (loca.u16_at(glyph * 2).unwrap() as usize * 2, loca.u16_at(glyph * 2 + 2).unwrap() as usize * 2)
    }
}

/// `font` with the glyphs of each group of code points left empty, an `IFT ` table with one entry
/// per group, and the patches that bring them back. The patches of two groups can share a glyph.
pub fn incremental_font(font: &[u8], groups: &[&[u32]], compress: &dyn Fn(&[u8]) -> Vec<u8>) -> IncrementalFixture {
    let reader = FontRef::new(font).unwrap();
    let cmap = reader.cmap().unwrap();
    let mut tables = font_tables(font);
    let table = |tables: &[(Tag, Vec<u8>)], tag: &Tag| tables.iter().find(|(name, _)| name == tag).unwrap().1.clone();
    let long = table(&tables, b"head").u16_at(50).unwrap() != 0;
    let glyf = table(&tables, b"glyf");
    let loca = table(&tables, b"loca");
    let count = table(&tables, b"maxp").u16_at(4).unwrap() as usize;
    let gvar = tables.iter().find(|(name, _)| name == b"gvar").map(|(_, data)| data.clone());
    let closure = |group: &[u32]| {
        let mut set = BTreeSet::new();
        let mut pending: Vec<u16> =
            group.iter().filter_map(|point| cmap.map_codepoint(*point)).map(|glyph| glyph.to_u32() as u16).collect();
        while let Some(glyph) = pending.pop() {
            if set.insert(glyph) {
                let (start, end) = glyph_range(&loca, long, glyph as usize);
                pending.extend(components(&glyf[start..end]));
            }
        }
        set
    };
    let sets: Vec<BTreeSet<u16>> = groups.iter().map(|group| closure(group)).collect();
    let removed: BTreeSet<u16> = sets.iter().flatten().copied().collect();
    let gvar_glyph = |glyph: usize| {
        gvar.as_ref().map(|gvar| {
            let (long, data) = (gvar.u16_at(14).unwrap() & 1 != 0, gvar.u32_at(16).unwrap() as usize);
            let (start, end) = glyph_range(&gvar[20..], long, glyph);
            gvar[data + start..data + end].to_vec()
        })
    };
    let mut patches = Vec::new();
    for (index, set) in sets.iter().enumerate() {
        let outlines = set.iter().map(|glyph| {
            let (start, end) = glyph_range(&loca, long, *glyph as usize);
            (*glyph, glyf[start..end].to_vec())
        });
        let mut patch_tables = vec![(*b"glyf", outlines.collect::<Vec<_>>())];
        if gvar.is_some() {
            let deltas = set.iter().map(|glyph| (*glyph, gvar_glyph(*glyph as usize).unwrap()));
            patch_tables.push((*b"gvar", deltas.collect()));
        }
        let id = (index + 1) as u8;
        let name = super::template::expand_template(&[128], super::template::EntryId::Number(id.into())).unwrap();
        let url = format!("p/{name}");
        patches.push((url, glyph_patch(&FIXTURE_ID, &patch_tables, compress)));
    }
    let (mut new_glyf, mut new_loca) = (Vec::new(), Vec::new());
    for glyph in 0..count {
        let offset = new_glyf.len();
        if long {
            new_loca.extend_from_slice(&(offset as u32).to_be_bytes());
        } else {
            new_loca.extend_from_slice(&((offset / 2) as u16).to_be_bytes());
        }
        if !removed.contains(&(glyph as u16)) {
            let (start, end) = glyph_range(&loca, long, glyph);
            new_glyf.extend_from_slice(&glyf[start..end]);
            if !long && new_glyf.len() % 2 != 0 {
                new_glyf.push(0);
            }
        }
    }
    if long {
        new_loca.extend_from_slice(&(new_glyf.len() as u32).to_be_bytes());
    } else {
        new_loca.extend_from_slice(&((new_glyf.len() / 2) as u16).to_be_bytes());
    }
    for (tag, data) in &mut tables {
        match &*tag {
            b"glyf" => *data = std::mem::take(&mut new_glyf),
            b"loca" => *data = std::mem::take(&mut new_loca),
            b"gvar" => {
                let source = gvar.as_ref().unwrap();
                let glyphs = source.u16_at(12).unwrap() as usize;
                let long = source.u16_at(14).unwrap() & 1 != 0;
                let start = source.u32_at(16).unwrap() as usize;
                let (mut offsets, mut bytes) = (Vec::new(), Vec::new());
                for glyph in 0..glyphs {
                    if long {
                        offsets.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
                    } else {
                        offsets.extend_from_slice(&((bytes.len() / 2) as u16).to_be_bytes());
                    }
                    if !removed.contains(&(glyph as u16)) {
                        bytes.extend_from_slice(&gvar_glyph(glyph).unwrap());
                        if !long && bytes.len() % 2 != 0 {
                            bytes.push(0);
                        }
                    }
                }
                if long {
                    offsets.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
                } else {
                    offsets.extend_from_slice(&((bytes.len() / 2) as u16).to_be_bytes());
                }
                let size = if long { 4 } else { 2 };
                let mut new = source[..20 + (glyphs + 1) * size].to_vec();
                new[20..].copy_from_slice(&offsets);
                new.extend_from_slice(&source[20 + (glyphs + 1) * size..start]);
                new.extend_from_slice(&bytes);
                *data = new;
            }
            _ => {}
        }
    }
    tables.push((*b"IFT ", patch_map(&FIXTURE_ID, &[2, b'p', b'/', 128], groups, 3, None)));
    let mut references: Vec<(Tag, &[u8])> = tables.iter().map(|(tag, data)| (*tag, data.as_slice())).collect();
    let font = assemble(TRUETYPE, &mut references, &WebFontLimits::DEFAULT).unwrap();
    IncrementalFixture { font, patches }
}
