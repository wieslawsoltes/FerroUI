use super::*;
use crate::utils::{RopeTextReader, Utf16Rope};
use std::io::{self, Read};

#[test]
fn reader_retains_old_text_without_retaining_mutable_document() {
    let document = TextDocument::from_text("before😀after");
    let source: &dyn ITextSource = &document;
    let mut reader = source.create_reader_range(6, 2).unwrap();
    let mut whole = source.create_reader();
    document.set_text("changed").unwrap();
    drop(document);
    assert_eq!(reader.peek(), Some(0xd83d));
    assert_eq!(reader.peek(), Some(0xd83d));
    assert_eq!(reader.read(), Some(0xd83d));
    assert_eq!(reader.position_utf16(), 1);
    assert_eq!(reader.read(), Some(0xde00));
    assert_eq!(reader.read(), None);
    assert_eq!(reader.peek(), None);
    assert_eq!(whole.read_to_end(), "before😀after");
    assert!(whole.read_to_end_utf16().is_empty());
}

#[test]
fn short_reads_block_reads_chunks_and_clones_keep_independent_cursors() {
    let units: Vec<u16> = (0..4096).map(|i| (i * 13) as u16).collect();
    let rope = Utf16Rope::from_utf16(&units);
    let mut reader = RopeTextReader::from_range(&rope, 503, 3000).unwrap();
    let mut destination = [0; 700];
    assert_eq!(reader.read_into(&mut []), 0);
    let first = reader.read_into(&mut destination);
    assert!(first > 0 && first < destination.len());
    assert_eq!(&destination[..first], &units[503..503 + first]);
    let mut cloned = reader.clone();
    let consumed = reader.read_block(&mut destination);
    assert_eq!(consumed, 700);
    assert_eq!(destination, &units[503 + first..1203 + first]);
    assert_eq!(cloned.read(), Some(units[503 + first]));
    let mut actual = Vec::new();
    while let Some(chunk) = reader.read_chunk() {
        actual.extend_from_slice(&chunk);
    }
    assert_eq!(actual, units[1203 + first..3503]);
    assert_eq!(reader.position_utf16(), 3000);
    assert_eq!(reader.len(), 0);
    assert!(RopeTextReader::from_range(&rope, usize::MAX, 1).is_none());
    assert!(RopeTextReader::from_range(&rope, 1, usize::MAX).is_none());
}

#[test]
fn line_reader_consumes_cross_leaf_crlf_and_preserves_invalid_utf16() {
    let mut units = vec![b'x' as u16; 1024];
    units[511] = 13;
    units[512] = 10;
    units[513] = 0xd800;
    units[514] = 13;
    units[515] = 10;
    units[516] = 10;
    let snapshot = TextSnapshot::from_utf16(&units);
    let mut reader = snapshot.create_reader_range(509, 8).unwrap();
    assert_eq!(reader.read_line_utf16(), Some(vec![120, 120]));
    assert_eq!(reader.read_line_utf16(), Some(vec![0xd800]));
    assert_eq!(reader.read_line_utf16(), Some(vec![]));
    assert_eq!(reader.read_line_utf16(), None);
    let mut reader = TextSnapshot::from_text("\rA\nB\r\nC").create_reader();
    assert_eq!(reader.read_line().as_deref(), Some(""));
    assert_eq!(reader.read_line().as_deref(), Some("A"));
    assert_eq!(reader.read_line().as_deref(), Some("B"));
    assert_eq!(reader.read_line().as_deref(), Some("C"));
    assert_eq!(reader.read_line(), None);
}

#[test]
fn contiguous_memory_shares_leaf_and_split_ranges_copy_only_requested_text() {
    let document = TextDocument::from_utf16(&(0..2048).collect::<Vec<u16>>());
    let source: &dyn ITextSource = &document;
    let a = source.get_text_as_memory(10, 50).unwrap();
    let b = source.get_text_as_memory(30, 50).unwrap();
    assert!(a.shares_storage_with(&b));
    assert_eq!(&*a, &(10..60).collect::<Vec<u16>>());
    let across = source.get_text_as_memory(500, 40).unwrap();
    assert!(!a.shares_storage_with(&across));
    assert_eq!(&*across, &(500..540).collect::<Vec<u16>>());
    assert_eq!(&*a.slice(5, 3).unwrap(), &[15, 16, 17]);
    assert!(a.slice(1, usize::MAX).is_none());
    document.set_text("replacement").unwrap();
    assert_eq!(a[0], 10);
    assert_eq!(across[0], 500);
    assert!(TextSnapshot::empty()
        .get_text_as_memory(0, 0)
        .unwrap()
        .is_empty());
}

#[test]
fn utf8_stream_handles_one_byte_reads_and_surrogates_across_leaves() {
    let mut units = vec![b'x' as u16; 1024];
    units[510] = 0xd800;
    units[511] = 0xd83d;
    units[512] = 0xde00;
    units[513] = 0xdc00;
    units[514] = 0x20ac;
    let snapshot = TextSnapshot::from_utf16(&units);
    let mut reader = snapshot.create_reader_range(510, 6).unwrap().into_utf8();
    assert_eq!(Read::read(&mut reader, &mut []).unwrap(), 0);
    let mut bytes = Vec::new();
    let mut byte = [0];
    while Read::read(&mut reader, &mut byte).unwrap() != 0 {
        bytes.push(byte[0]);
    }
    assert_eq!(String::from_utf8(bytes).unwrap(), "�😀�€x");
    let mut output = Vec::new();
    assert_eq!(
        snapshot.write_utf8_range_to(&mut output, 511, 2).unwrap(),
        4
    );
    assert_eq!(output, "😀".as_bytes());
    output.clear();
    snapshot.write_utf8_range_to(&mut output, 512, 1).unwrap();
    assert_eq!(output, "�".as_bytes());
}

struct MutatingSink {
    document: TextDocument,
    units: Vec<u16>,
    calls: usize,
    fail_at: Option<usize>,
}
impl Utf16TextWriter for MutatingSink {
    fn write_utf16(&mut self, text: &[u16]) -> io::Result<()> {
        self.calls += 1;
        if self.fail_at == Some(self.calls) {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "test sink"));
        }
        if self.calls == 1 {
            self.document.set_text("sink changed source").unwrap();
        }
        self.units.extend_from_slice(text);
        Ok(())
    }
}

#[test]
fn streaming_writer_snapshots_before_reentrant_sink_and_propagates_failures() {
    let units: Vec<u16> = (0..2048).collect();
    let document = TextDocument::from_utf16(&units);
    let mut sink = MutatingSink {
        document: document.clone(),
        units: Vec::new(),
        calls: 0,
        fail_at: None,
    };
    (&document as &dyn ITextSource)
        .write_text_to(&mut sink)
        .unwrap();
    assert_eq!(sink.units, units);
    assert!(sink.calls >= 4);
    assert_eq!(document.text(), "sink changed source");
    let snapshot = TextSnapshot::from_utf16(&units);
    sink.units.clear();
    sink.calls = 0;
    sink.fail_at = Some(2);
    assert_eq!(
        snapshot.write_text_to(&mut sink).unwrap_err().kind(),
        io::ErrorKind::BrokenPipe
    );
    assert_eq!(sink.calls, 2);
    assert_eq!(sink.units, units[..512]);
    let calls = sink.calls;
    assert_eq!(
        snapshot
            .write_text_range_to(&mut sink, 1, usize::MAX)
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(sink.calls, calls);
}

#[test]
fn document_writer_can_copy_same_document_losslessly_in_one_undo_group() {
    let units: Vec<u16> = (0..1800)
        .map(|i| if i % 3 == 0 { 0xd800 } else { 97 })
        .collect();
    let document = TextDocument::from_utf16(&units);
    let mut writer = DocumentTextWriter::new(document.clone(), units.len()).unwrap();
    {
        let _group = document.begin_update().unwrap();
        (&document as &dyn ITextSource)
            .write_text_to(&mut writer)
            .unwrap();
    }
    assert_eq!(
        document.create_snapshot().to_utf16(),
        [units.clone(), units.clone()].concat()
    );
    assert_eq!(writer.insertion_offset(), 3600);
    document.undo().unwrap();
    assert_eq!(document.create_snapshot().to_utf16(), units);
}

#[test]
fn ordinal_search_crosses_chunks_and_handles_overlapping_matches() {
    let snapshot = TextSnapshot::from_text(&format!(
        "{}ABABA{}ABABA",
        "x".repeat(510),
        "y".repeat(1200)
    ));
    assert_eq!(snapshot.index_of("ABA", 400, 1320).unwrap(), Some(510));
    assert_eq!(
        snapshot.last_index_of("ABA", 400, 1320).unwrap(),
        Some(1717)
    );
    assert_eq!(snapshot.index_of("too long", 0, 2).unwrap(), None);
    assert_eq!(snapshot.index_of("", 512, 2).unwrap(), Some(512));
    assert_eq!(snapshot.last_index_of("", 512, 2).unwrap(), Some(514));
    assert_eq!(snapshot.index_of("BAB", 512, 2).unwrap(), None);
    assert_eq!(snapshot.index_of("ABA", 512, 3).unwrap(), Some(512));
}

#[test]
fn lossless_search_finds_raw_surrogate_halves_and_character_sets() {
    let snapshot = TextSnapshot::from_utf16(&[0xd800, 97, 0xd83d, 0xde00, 0xd800, 0, 0xffff]);
    let source: &dyn ITextSource = &snapshot;
    assert_eq!(source.index_of_char(0xd800, 0, 7).unwrap(), Some(0));
    assert_eq!(source.last_index_of_char(0xd800, 0, 7).unwrap(), Some(4));
    assert_eq!(source.index_of_any(&[0, 0xde00], 0, 7).unwrap(), Some(3));
    assert_eq!(source.index_of_any(&[], 0, 7).unwrap(), None);
    assert_eq!(
        source
            .index_of_any(&[1, 2, 3, 4, 5, 6, 7, 8, 0xffff], 0, 7)
            .unwrap(),
        Some(6)
    );
    assert_eq!(
        source
            .index_of_utf16(&[0xde00], 0, 7, TextComparison::Ordinal)
            .unwrap(),
        Some(3)
    );
    assert_eq!(
        source
            .last_index_of_utf16(&[0xd800], 0, 7, TextComparison::Ordinal)
            .unwrap(),
        Some(4)
    );
}

#[test]
fn ordinal_ignore_case_uses_simple_mapping_without_expansion_or_normalization() {
    for (a, b, equal) in [
        ("café", "CAFÉ", true),
        ("σςΣ", "ΣΣΣ", true),
        ("ᾀ", "ᾈ", true),
        ("ß", "SS", false),
        ("ß", "ẞ", false),
        ("ﬀ", "FF", false),
        ("ı", "I", false),
        ("ſ", "S", false),
        ("İ", "i", false),
        ("µ", "Μ", true),
        ("é", "e\u{301}", false),
        ("K", "K", false),
    ] {
        let snapshot = TextSnapshot::from_text(a);
        assert_eq!(
            snapshot
                .index_of_with_comparison(
                    b,
                    0,
                    snapshot.text_length(),
                    TextComparison::OrdinalIgnoreCase
                )
                .unwrap(),
            equal.then_some(0),
            "{a} / {b}"
        );
    }
}

#[test]
fn supplementary_casing_crosses_chunks_and_preserves_utf16_positions() {
    let mut units = vec![b'x' as u16; 1024];
    units[511] = 0xd801;
    units[512] = 0xdc28; // U+10428 DESERET SMALL LETTER LONG I
    units[900] = 0xd801;
    units[901] = 0xdc00; // U+10400 uppercase
    let snapshot = TextSnapshot::from_utf16(&units);
    assert_eq!(
        snapshot
            .index_of_with_comparison("𐐀", 0, 1024, TextComparison::OrdinalIgnoreCase)
            .unwrap(),
        Some(511)
    );
    assert_eq!(
        snapshot
            .last_index_of_with_comparison("𐐨", 0, 1024, TextComparison::OrdinalIgnoreCase)
            .unwrap(),
        Some(900)
    );
    assert_eq!(
        snapshot
            .index_of_with_comparison("𐐀", 512, 300, TextComparison::OrdinalIgnoreCase)
            .unwrap(),
        None
    );
    assert_eq!(snapshot.index_of("𐐀", 0, 1024).unwrap(), Some(900));
}

#[test]
fn malformed_queries_match_raw_half_surrogates_at_candidate_boundaries() {
    let snapshot = TextSnapshot::from_utf16(&[0xd801, 0xdc28, 97, 0xd801, 0xdc00, 65]);
    let compare = TextComparison::OrdinalIgnoreCase;
    assert_eq!(
        snapshot.index_of_utf16(&[0xdc28], 0, 6, compare).unwrap(),
        Some(1)
    );
    assert_eq!(
        snapshot.index_of_utf16(&[0xdc00], 0, 6, compare).unwrap(),
        Some(4)
    );
    assert_eq!(
        snapshot
            .index_of_utf16(&[0xdc28, 65], 0, 6, compare)
            .unwrap(),
        Some(1)
    );
    assert_eq!(
        snapshot
            .index_of_utf16(&[65, 0xd801], 0, 6, compare)
            .unwrap(),
        Some(2)
    );
    assert_eq!(
        snapshot
            .last_index_of_utf16(&[0xd801], 0, 6, compare)
            .unwrap(),
        Some(3)
    );
    assert_eq!(
        snapshot
            .index_of_utf16(&[0xd801, 0xdc00, 97, 0xd801], 0, 6, compare)
            .unwrap(),
        Some(0)
    );
}

#[test]
fn source_operations_validate_ranges_even_for_empty_queries_and_sinks() {
    let source: &dyn ITextSource = &TextSnapshot::from_text("abc");
    for (offset, length) in [(4, 0), (usize::MAX, 0), (1, usize::MAX)] {
        assert!(source.create_reader_range(offset, length).is_err());
        assert!(source.get_text_as_memory(offset, length).is_err());
        assert!(source.index_of("", offset, length).is_err());
        assert!(source.last_index_of("", offset, length).is_err());
        assert!(source.index_of_any(&[], offset, length).is_err());
        assert!(source.index_of_char(97, offset, length).is_err());
        assert!(source.last_index_of_char(97, offset, length).is_err());
    }
    assert_eq!(
        source.get_text_segment(&SimpleSegment::new(1, 2)).unwrap(),
        "bc"
    );
    assert_eq!(source.create_reader_range(3, 0).unwrap().read(), None);
    assert_eq!(source.index_of_char(97, 3, 0).unwrap(), None);
    assert_eq!(source.index_of("", 3, 0).unwrap(), Some(3));
}

#[test]
fn randomized_ordinal_search_matches_flat_utf16_reference() {
    let mut seed = 0x915bd710u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed as usize
    };
    let alphabet = [65, 97, 66, 98, 0xd801, 0xdc00, 0xdc28, 0xd800, 0, 13, 10];
    let units: Vec<u16> = (0..2100)
        .map(|_| alphabet[next() % alphabet.len()])
        .collect();
    let snapshot = TextSnapshot::from_utf16(&units);
    for _ in 0..1400 {
        let offset = next() % (units.len() + 1);
        let length = next() % (units.len() - offset + 1);
        let start = next() % units.len();
        let count = (next() % 9).min(units.len() - start);
        let needle = &units[start..start + count];
        let haystack = &units[offset..offset + length];
        let (first, last) = if needle.is_empty() {
            (Some(offset), Some(offset + length))
        } else if needle.len() > haystack.len() {
            (None, None)
        } else {
            (
                haystack
                    .windows(needle.len())
                    .position(|x| x == needle)
                    .map(|i| i + offset),
                haystack
                    .windows(needle.len())
                    .rposition(|x| x == needle)
                    .map(|i| i + offset),
            )
        };
        assert_eq!(
            snapshot
                .index_of_utf16(needle, offset, length, TextComparison::Ordinal)
                .unwrap(),
            first
        );
        assert_eq!(
            snapshot
                .last_index_of_utf16(needle, offset, length, TextComparison::Ordinal)
                .unwrap(),
            last
        );
    }
}

#[test]
fn readers_and_retained_memory_transfer_across_threads() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<RopeTextReader>();
    send_sync::<crate::utils::Utf16Memory>();
    let document = TextDocument::from_text("thread snapshot 😀");
    let reader = document.create_reader();
    let memory = document.get_text_as_memory(0, 6).unwrap();
    document.set_text("new").unwrap();
    let output = std::thread::spawn(move || {
        let mut reader = reader;
        (reader.read_to_end(), memory.to_vec())
    })
    .join()
    .unwrap();
    assert_eq!(output.0, "thread snapshot 😀");
    assert_eq!(output.1, "thread".encode_utf16().collect::<Vec<_>>());
}

#[test]
fn randomized_case_insensitive_search_matches_independent_candidate_comparison() {
    // Deliberately includes raw surrogate halves as well as complete Deseret
    // pairs, so range boundaries and malformed-needle fallback are exercised.
    fn equivalent(a: &[u16], b: &[u16]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        let mut position = 0;
        while position < a.len() {
            if a[position] == 0xd801
                && b[position] == 0xd801
                && position + 1 < a.len()
                && (0xdc00..=0xdc4f).contains(&a[position + 1])
                && (0xdc00..=0xdc4f).contains(&b[position + 1])
            {
                let fold = |x: u16| if x >= 0xdc28 { x - 0x28 } else { x };
                if fold(a[position + 1]) != fold(b[position + 1]) {
                    return false;
                }
                position += 2;
            } else {
                let fold = |x: u16| if (97..=122).contains(&x) { x - 32 } else { x };
                if fold(a[position]) != fold(b[position]) {
                    return false;
                }
                position += 1;
            }
        }
        true
    }
    let mut seed = 0x51e3735u64;
    let mut next = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed as usize
    };
    let alphabet = [65, 97, 66, 98, 0xd801, 0xdc00, 0xdc28, 0xd800, 0, 13, 10];
    let units: Vec<u16> = (0..1300)
        .map(|_| alphabet[next() % alphabet.len()])
        .collect();
    let snapshot = TextSnapshot::from_utf16(&units);
    for _ in 0..800 {
        let offset = next() % (units.len() + 1);
        let length = next() % (units.len() - offset + 1);
        let start = next() % units.len();
        let count = (next() % 8 + 1).min(units.len() - start);
        let needle = &units[start..start + count];
        let haystack = &units[offset..offset + length];
        let (first, last) = if needle.len() > haystack.len() {
            (None, None)
        } else {
            (
                haystack
                    .windows(needle.len())
                    .position(|x| equivalent(x, needle))
                    .map(|i| i + offset),
                haystack
                    .windows(needle.len())
                    .rposition(|x| equivalent(x, needle))
                    .map(|i| i + offset),
            )
        };
        let comparison = TextComparison::OrdinalIgnoreCase;
        assert_eq!(
            snapshot
                .index_of_utf16(needle, offset, length, comparison)
                .unwrap(),
            first,
            "{needle:x?}"
        );
        assert_eq!(
            snapshot
                .last_index_of_utf16(needle, offset, length, comparison)
                .unwrap(),
            last,
            "{needle:x?}"
        );
    }
}

#[test]
fn rope_source_constructor_and_shared_rope_are_independent_values() {
    let rope = Utf16Rope::from_utf16(&[97, 0xd800, 98]);
    let source = RopeTextSource::from_rope(rope);
    let changed = source
        .get_rope()
        .replace(0, 1, &Utf16Rope::from_text("xyz"))
        .unwrap();
    assert_eq!(source.to_utf16(), [97, 0xd800, 98]);
    assert_eq!(changed.to_utf16(), [120, 121, 122, 0xd800, 98]);
}
