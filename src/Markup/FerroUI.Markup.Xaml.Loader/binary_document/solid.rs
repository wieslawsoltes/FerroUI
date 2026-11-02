//! Solid FXP pages group like columns across documents and pool their strings.
//! Expansion reconstructs complete stored FXD images, including original local
//! string indices, graph references, source positions and namespace scopes.
use super::{check_path, invalid, take, uint, ThemePack, MAGIC, MAX_DOCUMENTS, MAX_PACK_BYTES};
use crate::binary_document::{
    decode::read_uint,
    encode::{encode_graph, put, EncodedGraph},
    ASSIGN, DIRECTIVE, MAX_DOCUMENT_BYTES, MAX_NODES, OBJECT, PROPERTY, TEXT, TYPE,
};
use crate::compiler_extensions::FerroXamlIlCompiler;
use std::{
    collections::{BTreeSet, HashMap},
    io::Write,
};
use xamlx::exceptions::XamlResult;

const VERSION: u8 = 2;
const MAX_GLOBAL_STRINGS: usize = 1 << 20;
const COLUMN_COUNT: usize = 5; // document metadata, tags, lines, positions, scopes
const OPCODE_COUNT: usize = 6;

pub fn is_solid(bytes: &[u8]) -> bool {
    bytes.starts_with(MAGIC) && bytes.get(4) == Some(&VERSION)
}

fn sized(out: &mut Vec<u8>, bytes: &[u8]) {
    put(out, bytes.len() as u32);
    out.extend_from_slice(bytes);
}

fn slice<'a>(bytes: &mut &'a [u8]) -> XamlResult<&'a [u8]> {
    let len = read_uint(bytes)? as usize;
    take(bytes, len)
}

fn strings(mut bytes: &[u8]) -> impl Iterator<Item = XamlResult<&[u8]>> {
    std::iter::from_fn(move || (!bytes.is_empty()).then(|| slice(&mut bytes)))
}

fn skip_ints(input: &mut &[u8], count: u32) -> XamlResult<()> {
    if count as usize > input.len() {
        return Err(invalid("truncated operand list"));
    }
    for _ in 0..count {
        read_uint(input)?;
    }
    Ok(())
}

/// Consume an FXD operand row without resolving references or allocating AST nodes.
fn record<'a>(input: &mut &'a [u8], tag: u8) -> XamlResult<&'a [u8]> {
    let row = *input;
    let (fixed, lists) = match tag {
        TYPE => (3, 1),
        PROPERTY | TEXT => (3, 0),
        OBJECT => (1, 2),
        ASSIGN | DIRECTIVE => (2, 1),
        _ => return Err(invalid("unknown solid opcode")),
    };
    skip_ints(input, fixed)?;
    for _ in 0..lists {
        let count = read_uint(input)?;
        skip_ints(input, count)?;
    }
    Ok(&row[..row.len() - input.len()])
}

fn checked_size(sum: usize, add: usize, limit: usize) -> XamlResult<usize> {
    sum.checked_add(add)
        .filter(|n| *n <= limit)
        .ok_or_else(|| invalid("solid expansion exceeds limit"))
}

fn graph_size(graph: &EncodedGraph) -> usize {
    let c = graph.columns.as_ref().expect("columnar encoder");
    let mut header = Vec::new();
    put(&mut header, graph.string_count);
    6 + header.len()
        + graph.strings.len()
        + graph.metadata.len()
        + graph.records.len()
        + c.tags.len()
        + c.lines.len()
        + c.positions.len()
        + c.scopes.len()
}

/// Compile a complete file for distribution. Random-access editors can use the
/// independently compressed v1 representation via `compile_documents`.
pub fn compile_solid_documents(documents: &[(&str, &str)]) -> XamlResult<Vec<u8>> {
    if documents.is_empty() || documents.len() > MAX_DOCUMENTS {
        return Err(invalid("invalid document count"));
    }
    let mut sources = documents.to_vec();
    sources.sort_unstable_by_key(|&(name, _)| name);
    let mut documents = Vec::with_capacity(sources.len());
    let mut total = 13;
    for (name, source) in sources {
        check_path(name)?;
        if documents.last().is_some_and(|&(prev, _)| prev == name) {
            return Err(invalid("duplicate path"));
        }
        let graph = FerroXamlIlCompiler::parse_source_syntax(source)
            .and_then(|doc| encode_graph(&doc, true))
            .map_err(|err| invalid(&format!("{name}: {}", err.message())))?;
        let size = graph_size(&graph);
        if size > MAX_DOCUMENT_BYTES {
            return Err(invalid("document exceeds limit"));
        }
        total = checked_size(total, 6 + name.len() + size, MAX_PACK_BYTES)?;
        documents.push((name, graph));
    }
    let mut pool = BTreeSet::new();
    for (_, graph) in &documents {
        for s in strings(&graph.strings) {
            pool.insert(s?);
        }
    }
    if pool.len() > MAX_GLOBAL_STRINGS {
        return Err(invalid("too many distinct strings"));
    }
    let mut payload = Vec::new();
    put(&mut payload, pool.len() as u32);
    let mut ids = HashMap::with_capacity(pool.len());
    let mut previous: &[u8] = &[];
    for (id, text) in pool.into_iter().enumerate() {
        let common = previous
            .iter()
            .zip(text)
            .take_while(|(a, b)| a == b)
            .count();
        put(&mut payload, common as u32);
        sized(&mut payload, &text[common..]);
        ids.insert(text, id as i32);
        previous = text;
    }
    put(&mut payload, documents.len() as u32);
    let mut local = Vec::new();
    let mut columns: [Vec<u8>; COLUMN_COUNT] = Default::default();
    let mut operands: [Vec<u8>; OPCODE_COUNT] = Default::default();
    for (path, graph) in &documents {
        sized(&mut payload, path.as_bytes());
        put(&mut payload, graph_size(graph) as u32);
        put(&mut payload, graph.string_count);
        let mut previous = 0;
        for text in strings(&graph.strings) {
            let id = ids[text?];
            let delta = id - previous;
            put(&mut local, ((delta << 1) ^ (delta >> 31)) as u32);
            previous = id;
        }
        let c = graph.columns.as_ref().expect("columnar encoder");
        for (dest, lane) in
            columns
                .iter_mut()
                .zip([&graph.metadata, &c.tags, &c.lines, &c.positions, &c.scopes])
        {
            put(&mut payload, lane.len() as u32);
            dest.extend_from_slice(lane);
        }
        let mut remaining = graph.records.as_slice();
        for &tag in &c.tags {
            operands[tag as usize].extend_from_slice(record(&mut remaining, tag)?);
        }
        debug_assert!(remaining.is_empty());
    }
    sized(&mut payload, &local);
    for lane in columns.into_iter().chain(operands) {
        sized(&mut payload, &lane);
    }
    if payload.len() > MAX_PACK_BYTES {
        return Err(invalid("solid payload too large"));
    }
    let mut output = MAGIC.to_vec();
    output.push(VERSION);
    output.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    let mut z = flate2::write::ZlibEncoder::new(output, flate2::Compression::best());
    z.write_all(&payload).map_err(|e| invalid(&e.to_string()))?;
    let output = z.finish().map_err(|e| invalid(&e.to_string()))?;
    if output.len() > MAX_PACK_BYTES {
        return Err(invalid("solid image too large"));
    }
    Ok(output)
}

struct Entry<'a> {
    path: &'a str,
    size: usize,
    strings: usize,
    columns: [usize; COLUMN_COUNT],
}

/// Expand a solid archive to one indexed v1 image. No constructors run here.
/// Graph validation is performed by the FXD decoder when the host opens a
/// document, or for the entire revision via `ThemePack::validate`.
pub fn expand_solid(image: &[u8]) -> XamlResult<Vec<u8>> {
    if !is_solid(image) || image.len() > MAX_PACK_BYTES {
        return Err(invalid("invalid solid signature or size"));
    }
    let mut compressed = &image[5..];
    let size = uint(&mut compressed)?;
    if size > MAX_PACK_BYTES {
        return Err(invalid("solid payload too large"));
    }
    let mut payload = Vec::new();
    // Inflate in bounded steps so a tiny invalid header cannot immediately reserve
    // its entire alleged uncompressed size.
    let mut inflater = flate2::Decompress::new(true);
    let mut chunk = [0; 16 * 1024];
    loop {
        let before = inflater.total_in() as usize;
        let produced = inflater.total_out();
        let status = inflater
            .decompress(
                &compressed[before..],
                &mut chunk,
                flate2::FlushDecompress::None,
            )
            .map_err(|_| invalid("invalid solid compression"))?;
        let n = (inflater.total_out() - produced) as usize;
        checked_size(payload.len(), n, size)?;
        payload.extend_from_slice(&chunk[..n]);
        if status == flate2::Status::StreamEnd {
            break;
        }
        if n == 0 && inflater.total_in() as usize == before {
            return Err(invalid("incomplete solid compression"));
        }
    }
    if payload.len() != size || inflater.total_in() != compressed.len() as u64 {
        return Err(invalid("solid compression length mismatch"));
    }
    reconstruct(&payload)
}

fn reconstruct(mut data: &[u8]) -> XamlResult<Vec<u8>> {
    let count = read_uint(&mut data)? as usize;
    if count > MAX_GLOBAL_STRINGS || count > data.len() / 2 {
        return Err(invalid("invalid global pool"));
    }
    let mut pool: Vec<Vec<u8>> = Vec::with_capacity(count);
    let mut text_bytes = 0;
    for _ in 0..count {
        let prefix = read_uint(&mut data)? as usize;
        let suffix = slice(&mut data)?;
        let prev = pool.last().map_or(&[][..], Vec::as_slice);
        let head = prev
            .get(..prefix)
            .ok_or_else(|| invalid("invalid shared prefix"))?;
        let len = checked_size(prefix, suffix.len(), MAX_DOCUMENT_BYTES)?;
        text_bytes = checked_size(text_bytes, len, MAX_PACK_BYTES)?;
        let mut text = Vec::with_capacity(len);
        text.extend_from_slice(head);
        text.extend_from_slice(suffix);
        if pool.last().is_some_and(|prev| prev >= &text) || std::str::from_utf8(&text).is_err() {
            return Err(invalid("malformed or unsorted global strings"));
        }
        pool.push(text);
    }
    let count = read_uint(&mut data)? as usize;
    if count == 0 || count > MAX_DOCUMENTS || count > data.len() / 8 {
        return Err(invalid("invalid document count"));
    }
    let mut entries = Vec::with_capacity(count);
    let mut total = 13;
    let mut index = Vec::new();
    let mut previous = None;
    for _ in 0..count {
        let path = std::str::from_utf8(slice(&mut data)?).map_err(|_| invalid("malformed path"))?;
        check_path(path)?;
        if previous.is_some_and(|p| p >= path) {
            return Err(invalid("unsorted document paths"));
        }
        previous = Some(path);
        let size = read_uint(&mut data)? as usize;
        let strings = read_uint(&mut data)? as usize;
        if size < 6 || size > MAX_DOCUMENT_BYTES || strings > MAX_NODES || strings > size {
            return Err(invalid("invalid document sizes"));
        }
        total = checked_size(total, 6 + path.len() + size, MAX_PACK_BYTES)?;
        let mut columns = [0; COLUMN_COUNT];
        let mut bytes = 6;
        for n in &mut columns {
            *n = read_uint(&mut data)? as usize;
            bytes = checked_size(bytes, *n, size)?;
        }
        if columns[1] > MAX_NODES {
            return Err(invalid("too many nodes"));
        }
        index.extend_from_slice(&(path.len() as u16).to_le_bytes());
        index.extend_from_slice(path.as_bytes());
        index.extend_from_slice(&(size as u32).to_le_bytes());
        entries.push(Entry {
            path,
            size,
            strings,
            columns,
        });
    }
    let mut local = slice(&mut data)?;
    let mut columns = [&[][..]; COLUMN_COUNT];
    let mut operands = [&[][..]; OPCODE_COUNT];
    for lane in columns.iter_mut().chain(&mut operands) {
        *lane = slice(&mut data)?;
    }
    if !data.is_empty() {
        return Err(invalid("trailing solid payload"));
    }
    let mut output = MAGIC.to_vec();
    output.push(super::PACK_VERSION);
    output.extend_from_slice(&(count as u32).to_le_bytes());
    output.extend_from_slice(&(index.len() as u32).to_le_bytes());
    output.extend(index);
    for entry in entries {
        let start = output.len();
        output.extend_from_slice(b"FXD\0\x01\x02");
        put(&mut output, entry.strings as u32);
        let mut id = 0i64;
        for _ in 0..entry.strings {
            let d = read_uint(&mut local)?;
            id += (i64::from(d >> 1)) ^ -i64::from(d & 1);
            let text = usize::try_from(id)
                .ok()
                .and_then(|i| pool.get(i))
                .ok_or_else(|| invalid("invalid local string id"))?;
            checked_size(output.len() - start, text.len() + 1, entry.size)?;
            sized(&mut output, text);
        }
        let mut tags = &[][..];
        for (i, n) in entry.columns.into_iter().enumerate() {
            let lane = take(&mut columns[i], n)?;
            checked_size(output.len() - start, n, entry.size)?;
            output.extend_from_slice(lane);
            if i == 1 {
                tags = lane;
            }
        }
        for &tag in tags {
            let lane = operands
                .get_mut(tag as usize)
                .ok_or_else(|| invalid("unknown opcode"))?;
            let row = record(lane, tag)?;
            checked_size(output.len() - start, row.len(), entry.size)?;
            output.extend_from_slice(row);
        }
        if output.len() - start != entry.size {
            return Err(invalid(&format!(
                "incorrect reconstructed size for {}",
                entry.path
            )));
        }
    }
    if !local.is_empty() || columns.iter().chain(&operands).any(|lane| !lane.is_empty()) {
        return Err(invalid("unused solid column data"));
    }
    debug_assert_eq!(output.len(), total);
    ThemePack::open(&output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binary_document::{decode, encode_stored};

    #[test]
    fn solid_reconstructs_ordinary_images_independent_of_input_order() {
        let first = r#"<Panel xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><x:String x:Key="α🔷"> повтор </x:String><Button Content="{Binding Title}"/></Panel>"#;
        let second = r#"<Panel xmlns="https://github.com/ferroui"><Panel.Children><TextBlock Text="same"/></Panel.Children></Panel>"#;
        let a = compile_solid_documents(&[("B.xaml", second), ("A.xaml", first)]).unwrap();
        assert_eq!(
            a,
            compile_solid_documents(&[("A.xaml", first), ("B.xaml", second)]).unwrap()
        );
        assert!(is_solid(&a));
        let expanded = expand_solid(&a).unwrap();
        let pack = ThemePack::open(&expanded).unwrap();
        for (path, source) in [("A.xaml", first), ("B.xaml", second)] {
            let expected = FerroXamlIlCompiler::parse_source_syntax(source).unwrap();
            let stored = encode_stored(&decode(pack.document(path).unwrap()).unwrap()).unwrap();
            assert_eq!(encode_stored(&expected).unwrap(), stored);
        }
        assert!(compile_solid_documents(&[("A", first), ("A", first)]).is_err());
    }

    #[test]
    fn corrupt_solid_envelopes_fail_before_graph_construction() {
        let good = compile_solid_documents(&[("A.xaml", "<Panel/>")]).unwrap();
        for i in 0..good.len() {
            assert!(expand_solid(&good[..i]).is_err());
        }
        let mut trailing = good.clone();
        trailing.push(0);
        assert!(expand_solid(&trailing).is_err());
        let mut wrong = good;
        wrong[5..9].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(expand_solid(&wrong).is_err());
        // A nonzero prefix before the first global string cannot be reconstructed.
        assert!(reconstruct(&[1, 1, 0]).is_err());
        assert!(reconstruct(&[0xff, 0xff, 0xff, 0xff, 0x0f]).is_err());
    }
}
