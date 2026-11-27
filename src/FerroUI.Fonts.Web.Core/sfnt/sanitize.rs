//! The sanitizer: a font from the network goes in, a new font goes out that holds only what
//! passed validation. Modeled on the OpenType Sanitizer of the browsers.

use super::deep::{TableCheck, TableContext, Verdict};
use super::glyf::{validate_glyph, write_without_instructions, ComponentGraph, GlyphLayout};
use super::tables::*;
use super::{assemble, read_directory, ReadBe, TableRecord, Tag, OPEN_TYPE_CFF};
use crate::{FontDecodeError, WebFontLimits};

/// What the sanitizer does besides validating.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SanitizeOptions {
    pub limits: WebFontLimits,
    /// The face of a collection to keep. A collection comes out as a single font.
    pub face_index: u32,
    /// Removes the TrueType programs (`fpgm`, `prep`, `cvt `, `cvar`) and the instructions of
    /// each glyph. Off by default, as in browsers: hinting changes how text looks.
    pub strip_hinting: bool,
    /// Keeps the patch maps of an incremental font (`IFT `, `IFTX`), which are validated. Off by
    /// default: a font that was not declared with `tech(incremental)` has no use for them.
    pub keep_patch_maps: bool,
}

/// The tables that are kept when they pass. Anything else is dropped, including `DSIG`, which no
/// longer holds for a rewritten font, and the tables there is no validator for (`JSTF`, `MATH`,
/// `VDMX`, `LTSH`, `PCLT`, `EBSC`).
const KEPT_TABLES: [&Tag; 43] = [
    b"BASE", b"CBDT", b"CBLC", b"CFF ", b"CFF2", b"COLR", b"CPAL", b"EBDT", b"EBLC", b"GDEF", b"GPOS", b"GSUB",
    b"HVAR", b"MVAR", b"OS/2", b"STAT", b"SVG ", b"VORG", b"VVAR", b"avar", b"cmap", b"cvar", b"cvt ", b"fpgm",
    b"fvar", b"gasp", b"glyf", b"gvar", b"hdmx", b"head", b"hhea", b"hmtx", b"kern", b"loca", b"ltag", b"maxp",
    b"meta", b"name", b"post", b"prep", b"sbix", b"vhea", b"vmtx",
];

/// The tables a font cannot be used without: when one of them fails, the font is refused.
const REQUIRED_TABLES: [&Tag; 7] = [b"CFF ", b"CFF2", b"cmap", b"head", b"hhea", b"hmtx", b"maxp"];

/// Tables that mean nothing without another: `(table, what it needs)`.
const DEPENDENCIES: [(&Tag, &Tag); 12] = [
    (b"COLR", b"CPAL"),
    (b"CBDT", b"CBLC"),
    (b"CBLC", b"CBDT"),
    (b"EBDT", b"EBLC"),
    (b"EBLC", b"EBDT"),
    (b"vhea", b"vmtx"),
    (b"vmtx", b"vhea"),
    (b"avar", b"fvar"),
    (b"gvar", b"fvar"),
    (b"HVAR", b"fvar"),
    (b"MVAR", b"fvar"),
    (b"VVAR", b"fvar"),
];

/// The tables that only a valid `fvar` gives a meaning to.
const VARIATION_TABLES: [&Tag; 6] = [b"avar", b"cvar", b"gvar", b"HVAR", b"MVAR", b"VVAR"];
const HINTING_TABLES: [&Tag; 4] = [b"cvar", b"cvt ", b"fpgm", b"prep"];

enum Output {
    Keep,
    Replace(Vec<u8>),
    Drop,
}

enum Stage {
    Tables {
        next: usize,
    },
    Glyphs {
        next: u32,
    },
    /// The content of each table that is still kept.
    Contents {
        next: usize,
        check: Option<Box<TableCheck>>,
    },
    Finish,
    Done,
}

/// The state of the walk over `glyf`.
struct GlyphWalk {
    glyf: TableRecord,
    loca: TableRecord,
    long_loca: bool,
    graph: ComponentGraph,
    /// With `strip_hinting`, the rewritten table and its new offsets.
    stripped: Option<(Vec<u8>, Vec<u32>)>,
}

/// A sanitizer that runs in steps, so that a thread that must stay responsive can give it a
/// budget at a time.
pub struct Sanitizer<D> {
    data: D,
    options: SanitizeOptions,
    version: u32,
    records: Vec<TableRecord>,
    outputs: Vec<Output>,
    glyph_count: u16,
    fvar_axes: Option<u16>,
    walk: Option<GlyphWalk>,
    stage: Stage,
}

impl<D: AsRef<[u8]>> Sanitizer<D> {
    /// Reads the directory and the tables everything else depends on. `data` is the font, owned
    /// or borrowed.
    pub fn new(data: D, options: SanitizeOptions) -> Result<Self, FontDecodeError> {
        let bytes = data.as_ref();
        if bytes.len() > options.limits.max_decoded_size {
            return Err(FontDecodeError::LimitExceeded("decoded size"));
        }
        let (version, records) = read_directory(bytes, options.face_index, &options.limits)?;
        let table = |tag: &Tag| {
            records.iter().find(|record| record.tag == *tag).map(|record| &bytes[record.offset..][..record.length])
        };
        let required = |tag: &'static Tag| table(tag).ok_or(FontDecodeError::MissingTable(*tag));
        let head = parse_head(required(b"head")?).ok_or(FontDecodeError::Malformed("head"))?;
        let glyph_count = parse_maxp(required(b"maxp")?).ok_or(FontDecodeError::Malformed("maxp"))?;
        let metrics =
            parse_metrics_header(required(b"hhea")?, glyph_count).ok_or(FontDecodeError::Malformed("hhea"))?;
        if required(b"hmtx")?.len() < metrics_length(metrics, glyph_count) {
            return Err(FontDecodeError::Malformed("hmtx"));
        }
        required(b"cmap")?;
        let has = |tag: &Tag| table(tag).is_some_and(|table| !table.is_empty());
        if has(b"glyf") != has(b"loca") {
            return Err(FontDecodeError::Malformed("glyf and loca must come together"));
        }
        if version == OPEN_TYPE_CFF && !has(b"CFF ") && !has(b"CFF2") {
            return Err(FontDecodeError::MissingTable(*b"CFF "));
        }
        let outlines = [b"glyf", b"CFF ", b"CFF2", b"CBDT", b"sbix", b"EBDT", b"SVG "];
        if !outlines.iter().any(|tag| has(tag)) {
            return Err(FontDecodeError::Malformed("the font has no glyphs"));
        }
        let fvar_axes = table(b"fvar").and_then(parse_fvar);
        let walk = match (
            records.iter().find(|record| record.tag == *b"glyf" && record.length > 0),
            records.iter().find(|record| record.tag == *b"loca" && record.length > 0),
        ) {
            (Some(glyf), Some(loca)) => {
                let expected = (glyph_count as usize + 1) * if head.long_loca { 4 } else { 2 };
                if loca.length < expected {
                    return Err(FontDecodeError::Malformed("loca"));
                }
                let stripped = options
                    .strip_hinting
                    .then(|| (Vec::with_capacity(glyf.length), Vec::with_capacity(glyph_count as usize + 1)));
                Some(GlyphWalk {
                    glyf: *glyf,
                    loca: *loca,
                    long_loca: head.long_loca,
                    graph: ComponentGraph::default(),
                    stripped,
                })
            }
            _ => None,
        };
        let outputs = records.iter().map(|_| Output::Keep).collect();
        Ok(Self {
            data,
            options,
            version,
            records,
            outputs,
            glyph_count,
            fvar_axes,
            walk,
            stage: Stage::Tables { next: 0 },
        })
    }

    fn table_bytes(&self, tag: &Tag) -> Option<&[u8]> {
        let data = self.data.as_ref();
        self.records.iter().find(|record| record.tag == *tag).map(|record| &data[record.offset..][..record.length])
    }

    /// Decides what happens to one table.
    fn check_table(&self, record: &TableRecord) -> Result<Output, FontDecodeError> {
        let table = &self.data.as_ref()[record.offset..][..record.length];
        let tag = &record.tag;
        if self.options.keep_patch_maps && (*tag == crate::ift::IFT_TABLE || *tag == crate::ift::IFTX_TABLE) {
            crate::ift::parse_patch_map(*tag, table)?;
            return Ok(Output::Keep);
        }
        if !KEPT_TABLES.contains(&tag) {
            return Ok(Output::Drop);
        }
        if self.options.strip_hinting && HINTING_TABLES.contains(&tag) {
            return Ok(Output::Drop);
        }
        if VARIATION_TABLES.contains(&tag) && self.fvar_axes.is_none() {
            return Ok(Output::Drop);
        }
        let keep_if = |valid: bool| if valid { Output::Keep } else { Output::Drop };
        Ok(match tag {
            b"cmap" => {
                let keep = valid_cmap_records(table).ok_or(FontDecodeError::Malformed("cmap"))?;
                if keep.len() == table.u16_at(2).unwrap_or(0) as usize {
                    Output::Keep
                } else {
                    Output::Replace(rewrite_cmap(table, &keep))
                }
            }
            b"hmtx" => {
                let metrics = self.table_bytes(b"hhea").and_then(|hhea| parse_metrics_header(hhea, self.glyph_count));
                let metrics = metrics.ok_or(FontDecodeError::Malformed("hhea"))?;
                let length = metrics_length(metrics, self.glyph_count);
                if table.len() == length {
                    Output::Keep
                } else {
                    Output::Replace(table[..length].to_vec())
                }
            }
            b"vhea" => keep_if(self.valid_vertical()),
            b"vmtx" => keep_if(self.valid_vertical()),
            b"name" => keep_if(valid_name(table)),
            b"OS/2" => keep_if(valid_os2(table)),
            b"post" => keep_if(valid_post(table)),
            b"GDEF" | b"GPOS" | b"GSUB" => keep_if(valid_layout_header(tag, table)),
            b"fvar" => keep_if(self.fvar_axes.is_some()),
            b"gvar" => keep_if(self.fvar_axes.is_some_and(|axes| valid_gvar(table, axes, self.glyph_count))),
            b"COLR" => keep_if(valid_colr(table) && self.table_bytes(b"CPAL").is_some_and(valid_cpal)),
            b"CPAL" => keep_if(valid_cpal(table)),
            b"sbix" => keep_if(valid_sbix(table, self.glyph_count)),
            b"CBDT" => keep_if(self.table_bytes(b"CBLC").is_some()),
            b"CBLC" => keep_if(self.table_bytes(b"CBDT").is_some()),
            b"EBDT" => keep_if(self.table_bytes(b"EBLC").is_some()),
            b"EBLC" => keep_if(self.table_bytes(b"EBDT").is_some()),
            b"SVG " => keep_if(valid_svg(table)),
            b"CFF " => {
                if !valid_cff(table) {
                    return Err(FontDecodeError::Malformed("CFF"));
                }
                Output::Keep
            }
            b"CFF2" => {
                if !valid_cff2(table) {
                    return Err(FontDecodeError::Malformed("CFF2"));
                }
                Output::Keep
            }
            _ => Output::Keep,
        })
    }

    fn valid_vertical(&self) -> bool {
        let (Some(vhea), Some(vmtx)) = (self.table_bytes(b"vhea"), self.table_bytes(b"vmtx")) else {
            return false;
        };
        parse_metrics_header(vhea, self.glyph_count)
            .is_some_and(|metrics| vmtx.len() >= metrics_length(metrics, self.glyph_count))
    }

    fn loca_entry(&self, walk: &GlyphWalk, index: usize) -> Option<usize> {
        let loca = &self.data.as_ref()[walk.loca.offset..][..walk.loca.length];
        if walk.long_loca {
            loca.u32_at(index * 4).map(|value| value as usize)
        } else {
            loca.u16_at(index * 2).map(|value| value as usize * 2)
        }
    }

    /// Walks glyphs until `budget` bytes were looked at.
    fn step_glyphs(&mut self, mut next: u32, budget: &mut usize) -> Result<Stage, FontDecodeError> {
        let Some(mut walk) = self.walk.take() else {
            return Ok(Stage::Finish);
        };
        let glyph_count = self.glyph_count as usize;
        let glyf = &self.data.as_ref()[walk.glyf.offset..][..walk.glyf.length];
        const MALFORMED: FontDecodeError = FontDecodeError::Malformed("loca");
        while (next as usize) < glyph_count {
            if *budget == 0 {
                self.walk = Some(walk);
                return Ok(Stage::Glyphs { next });
            }
            let index = next as usize;
            let start = self.loca_entry(&walk, index).ok_or(MALFORMED)?;
            let end = self.loca_entry(&walk, index + 1).ok_or(MALFORMED)?;
            if start > end || end > glyf.len() {
                return Err(MALFORMED);
            }
            let glyph = &glyf[start..end];
            let layout = validate_glyph(glyph, index as u16, glyph_count, &mut walk.graph)?;
            if let Some((output, offsets)) = walk.stripped.as_mut() {
                offsets.push(output.len() as u32);
                if !matches!(layout, GlyphLayout::Empty) {
                    write_without_instructions(glyph, &layout, output);
                    // Padding keeps every offset expressible in the short format of `loca`.
                    output.resize((output.len() + 3) & !3, 0);
                }
            }
            *budget = budget.saturating_sub(glyph.len().max(16));
            next += 1;
        }
        walk.graph.check(glyph_count, self.options.limits.max_component_depth)?;
        if let Some((output, mut offsets)) = walk.stripped.take() {
            offsets.push(output.len() as u32);
            let mut loca = Vec::with_capacity(offsets.len() * 4);
            for offset in &offsets {
                loca.extend_from_slice(&offset.to_be_bytes());
            }
            let mut head = self.table_bytes(b"head").ok_or(FontDecodeError::MissingTable(*b"head"))?.to_vec();
            head[50..52].copy_from_slice(&1i16.to_be_bytes());
            for (tag, table) in [(b"glyf", output), (b"loca", loca), (b"head", head)] {
                if let Some(index) = self.records.iter().position(|record| record.tag == *tag) {
                    self.outputs[index] = Output::Replace(table);
                }
            }
        }
        Ok(Stage::Finish)
    }

    fn finish(&mut self) -> Result<Vec<u8>, FontDecodeError> {
        let data = self.data.as_ref();
        let mut tables: Vec<(Tag, &[u8])> = Vec::with_capacity(self.records.len());
        for (record, output) in self.records.iter().zip(self.outputs.iter()) {
            match output {
                Output::Keep => tables.push((record.tag, &data[record.offset..][..record.length])),
                Output::Replace(table) => tables.push((record.tag, table)),
                Output::Drop => {}
            }
        }
        assemble(self.version, &mut tables, &self.options.limits)
    }

    /// Validates up to about `budget` bytes. Returns whether all of the font was validated.
    pub(crate) fn advance(&mut self, budget: usize) -> Result<bool, FontDecodeError> {
        let mut budget = budget.max(1);
        loop {
            match std::mem::replace(&mut self.stage, Stage::Done) {
                Stage::Tables { next } => {
                    let Some(record) = self.records.get(next).copied() else {
                        self.stage = Stage::Glyphs { next: 0 };
                        continue;
                    };
                    self.outputs[next] = self.check_table(&record)?;
                    budget = budget.saturating_sub(record.length.max(16));
                    self.stage = Stage::Tables { next: next + 1 };
                }
                Stage::Glyphs { next } => {
                    self.stage = self.step_glyphs(next, &mut budget)?;
                    if matches!(self.stage, Stage::Finish) {
                        self.stage = Stage::Contents { next: 0, check: None };
                    }
                }
                Stage::Contents { next, check } => {
                    self.stage = self.step_contents(next, check, &mut budget)?;
                    if matches!(self.stage, Stage::Finish) {
                        self.finish_contents()?;
                        return Ok(true);
                    }
                }
                Stage::Finish => {
                    self.stage = Stage::Finish;
                    return Ok(true);
                }
                Stage::Done => return Err(FontDecodeError::Malformed("the sanitizer has already finished")),
            }
            if budget == 0 {
                return Ok(false);
            }
        }
    }

    /// The bytes a table will be written with.
    fn output_bytes(&self, index: usize) -> Option<&[u8]> {
        let record = self.records.get(index)?;
        match self.outputs.get(index)? {
            Output::Keep => Some(&self.data.as_ref()[record.offset..][..record.length]),
            Output::Replace(table) => Some(table),
            Output::Drop => None,
        }
    }

    /// Validates the content of the kept tables, one at a time, until the budget is spent.
    fn step_contents(
        &mut self,
        mut next: usize,
        mut check: Option<Box<TableCheck>>,
        budget: &mut usize,
    ) -> Result<Stage, FontDecodeError> {
        let palette_entries = (0..self.records.len())
            .find(|index| self.records[*index].tag == *b"CPAL")
            .and_then(|index| self.output_bytes(index))
            .and_then(|table| table.u16_at(2));
        let axis_count = self.fvar_axes.unwrap_or(0);
        let context = TableContext { glyph_count: self.glyph_count, axis_count, palette_entries };
        while next < self.records.len() {
            if *budget == 0 {
                return Ok(Stage::Contents { next, check });
            }
            let record = self.records[next];
            let verdict = match self.output_bytes(next) {
                None => Verdict::Valid,
                Some(table) => {
                    let state = check.get_or_insert_with(|| Box::new(TableCheck::new(record.tag, table.len())));
                    state.step(table, &context, budget)
                }
            };
            match verdict {
                Verdict::Pending => return Ok(Stage::Contents { next, check }),
                Verdict::Valid => {}
                Verdict::Invalid => {
                    if REQUIRED_TABLES.contains(&&record.tag) {
                        return Err(FontDecodeError::Malformed("a required table did not validate"));
                    }
                    self.outputs[next] = Output::Drop;
                }
            }
            check = None;
            next += 1;
        }
        Ok(Stage::Finish)
    }

    /// Drops what lost a table it depends on, and refuses a font left without glyphs.
    fn finish_contents(&mut self) -> Result<(), FontDecodeError> {
        let kept = |this: &Self, tag: &Tag| {
            this.records
                .iter()
                .position(|record| record.tag == *tag)
                .is_some_and(|index| !matches!(this.outputs[index], Output::Drop))
        };
        loop {
            let mut changed = false;
            for (table, needs) in DEPENDENCIES {
                if kept(self, table) && !kept(self, needs) {
                    if let Some(index) = self.records.iter().position(|record| record.tag == *table) {
                        self.outputs[index] = Output::Drop;
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }
        let outlines: [&Tag; 7] = [b"glyf", b"CFF ", b"CFF2", b"CBDT", b"sbix", b"EBDT", b"SVG "];
        if !outlines.iter().any(|tag| kept(self, tag)) {
            return Err(FontDecodeError::Malformed("no glyph table validated"));
        }
        Ok(())
    }

    /// Whether the validated font keeps every table exactly as it came.
    pub(crate) fn keeps_everything(&self) -> bool {
        self.outputs.iter().all(|output| matches!(output, Output::Keep))
    }

    /// Does up to about `budget` bytes of work. Returns the font when it is finished.
    pub fn step(&mut self, budget: usize) -> Result<Option<Vec<u8>>, FontDecodeError> {
        if !self.advance(budget)? {
            return Ok(None);
        }
        let font = self.finish()?;
        self.stage = Stage::Done;
        Ok(Some(font))
    }
}

impl Sanitizer<Vec<u8>> {
    /// The font after validation. A font this crate wrote itself, in canonical layout and with
    /// its checksums, is handed on without a copy when every table was kept.
    pub(crate) fn into_font(mut self) -> Result<Vec<u8>, FontDecodeError> {
        if self.keeps_everything() {
            return Ok(self.data);
        }
        self.finish()
    }
}

/// Sanitizes a font in one go.
pub fn sanitize(data: &[u8], options: &SanitizeOptions) -> Result<Vec<u8>, FontDecodeError> {
    let mut sanitizer = Sanitizer::new(data, *options)?;
    loop {
        if let Some(font) = sanitizer.step(usize::MAX)? {
            return Ok(font);
        }
    }
}
