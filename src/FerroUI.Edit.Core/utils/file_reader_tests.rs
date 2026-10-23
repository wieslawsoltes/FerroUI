use super::*;
use crate::document::{DocumentTextWriter, TextDocument, Utf16TextWriter};
use crate::utils::{DecodingErrorPolicy, FileTextReader};
use std::cell::Cell;
use std::io::{BufRead, Cursor, ErrorKind};
use std::rc::Rc;

struct ChunkedRead {
    bytes: Vec<u8>,
    position: usize,
    chunk: usize,
    read_bytes: Rc<Cell<usize>>,
    interrupt: bool,
    fail_at: Option<usize>,
}

impl ChunkedRead {
    fn new(bytes: impl Into<Vec<u8>>, chunk: usize) -> Self {
        Self {
            bytes: bytes.into(),
            position: 0,
            chunk,
            read_bytes: Rc::new(Cell::new(0)),
            interrupt: false,
            fail_at: None,
        }
    }
}

impl Read for ChunkedRead {
    fn read(&mut self, destination: &mut [u8]) -> io::Result<usize> {
        if destination.is_empty() {
            return Ok(0);
        }
        if std::mem::take(&mut self.interrupt) {
            return Err(io::Error::from(ErrorKind::Interrupted));
        }
        if self.fail_at == Some(self.position) {
            self.fail_at = None;
            return Err(io::Error::from(ErrorKind::WouldBlock));
        }
        let count = destination
            .len()
            .min(self.chunk)
            .min(self.bytes.len() - self.position);
        destination[..count].copy_from_slice(&self.bytes[self.position..self.position + count]);
        self.position += count;
        self.read_bytes.set(self.read_bytes.get() + count);
        Ok(count)
    }
}

fn encoded(text: &str, encoding: TextEncoding, bom: bool) -> Vec<u8> {
    let mut bytes = if bom {
        encoding.byte_order_mark().to_vec()
    } else {
        Vec::new()
    };
    match encoding {
        TextEncoding::Utf8 => bytes.extend_from_slice(text.as_bytes()),
        TextEncoding::Utf16Le | TextEncoding::Utf16Be => {
            for unit in text.encode_utf16() {
                bytes.extend_from_slice(&if encoding == TextEncoding::Utf16Le {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                });
            }
        }
        TextEncoding::Utf32Le | TextEncoding::Utf32Be => {
            for scalar in text.chars() {
                bytes.extend_from_slice(&if encoding == TextEncoding::Utf32Le {
                    (scalar as u32).to_le_bytes()
                } else {
                    (scalar as u32).to_be_bytes()
                });
            }
        }
        _ => panic!("The fixture only encodes Unicode"),
    }
    bytes
}

#[test]
fn unicode_boms_override_defaults_and_short_reads_preserve_supplementary_text() {
    let text = "A😀\r\nβ\u{feff}Z";
    for encoding in [
        TextEncoding::Utf8,
        TextEncoding::Utf16Le,
        TextEncoding::Utf16Be,
        TextEncoding::Utf32Le,
        TextEncoding::Utf32Be,
    ] {
        for chunk in 1..=5 {
            let mut source = ChunkedRead::new(encoded(text, encoding, true), chunk);
            source.interrupt = true;
            let mut reader = FileReader::open_stream(source, TextEncoding::Latin1).unwrap();
            assert_eq!(reader.encoding(), encoding);
            assert_eq!(
                reader.byte_order_mark_length(),
                encoding.byte_order_mark().len()
            );
            assert!(reader.has_byte_order_mark());
            let mut actual = Vec::new();
            let mut unit = [0];
            while reader.read_into(&mut unit).unwrap() != 0 {
                actual.push(unit[0]);
            }
            assert_eq!(actual, text.encode_utf16().collect::<Vec<_>>());
            assert_eq!(reader.peek().unwrap(), None);
            assert!(reader.read_to_end().unwrap().is_empty());
        }
    }
}

#[test]
fn bom_only_files_and_ambiguous_or_partial_prefixes_match_the_reference_path() {
    for encoding in [
        TextEncoding::Utf8,
        TextEncoding::Utf16Le,
        TextEncoding::Utf16Be,
        TextEncoding::Utf32Le,
        TextEncoding::Utf32Be,
    ] {
        let mut reader =
            FileReader::open_stream(encoding.byte_order_mark(), TextEncoding::Ascii).unwrap();
        assert_eq!(reader.encoding(), encoding);
        assert!(reader.read_to_end_utf16().unwrap().is_empty());
    }
    for (bytes, expected) in [
        (&[0xef, 0xbb][..], "�"),
        (&[0xef, 0xbb, b'x'][..], "�x"),
        (&[0, 0, b'A'][..], "\0\0A"),
    ] {
        let mut reader = FileReader::open_stream(bytes, TextEncoding::Latin1).unwrap();
        assert_eq!(reader.encoding(), TextEncoding::Utf8);
        assert!(!reader.has_byte_order_mark());
        assert_eq!(reader.read_to_end().unwrap(), expected);
    }
    let mut reader = FileReader::open_stream(&[0xff, 0xfe, 0][..], TextEncoding::Latin1).unwrap();
    assert_eq!(reader.encoding(), TextEncoding::Utf16Le);
    assert_eq!(reader.read_to_end().unwrap(), "�");
}

#[test]
fn heuristic_distinguishes_ascii_utf8_and_legacy_fallback_without_inventing_a_bom() {
    for (bytes, default, expected_encoding, text) in [
        (&b"Az"[..], TextEncoding::Latin1, TextEncoding::Latin1, "Az"),
        (&b"Az"[..], TextEncoding::Utf16Le, TextEncoding::Utf8, "Az"),
        (
            "é😀".as_bytes(),
            TextEncoding::Latin1,
            TextEncoding::Utf8,
            "é😀",
        ),
        (
            &b"caf\xe9 "[..],
            TextEncoding::Latin1,
            TextEncoding::Latin1,
            "café ",
        ),
        (
            &b"caf\xe9"[..],
            TextEncoding::Latin1,
            TextEncoding::Utf8,
            "caf�",
        ),
        (
            &b"a\x80\x93\x94\x81"[..],
            TextEncoding::Windows1252,
            TextEncoding::Windows1252,
            "a€“”\u{81}",
        ),
        (
            &b"a\xff"[..],
            TextEncoding::Ascii,
            TextEncoding::Ascii,
            "a?",
        ),
        (
            &b"a\xff"[..],
            TextEncoding::Utf16Be,
            TextEncoding::Utf8,
            "a�",
        ),
        (
            &b"a\xff"[..],
            TextEncoding::Utf32Be,
            TextEncoding::Utf8,
            "a�",
        ),
        // The source's structural probe accepts scalar-invalid 3-byte forms;
        // the Unicode decoder subsequently replaces their maximal subparts.
        (
            &[0xe0, 0x80, 0x80][..],
            TextEncoding::Latin1,
            TextEncoding::Utf8,
            "���",
        ),
    ] {
        let mut reader = FileReader::open_stream(bytes, default).unwrap();
        assert_eq!(reader.encoding(), expected_encoding);
        assert!(!reader.has_byte_order_mark());
        assert_eq!(reader.read_to_end().unwrap(), text);
    }
}

#[test]
fn zero_and_one_byte_inputs_keep_the_supplied_default_encoding() {
    for encoding in [
        TextEncoding::Utf8,
        TextEncoding::Utf16Le,
        TextEncoding::Utf32Be,
        TextEncoding::Latin1,
        TextEncoding::Ascii,
    ] {
        let mut empty = FileReader::open_stream(&[][..], encoding).unwrap();
        assert_eq!(empty.encoding(), encoding);
        assert_eq!(empty.read().unwrap(), None);
        let mut one = FileReader::open_stream(&[0xe9][..], encoding).unwrap();
        assert_eq!(one.encoding(), encoding);
        assert_eq!(
            one.read_to_end().unwrap(),
            match encoding {
                TextEncoding::Latin1 => "é",
                TextEncoding::Ascii => "?",
                _ => "�",
            }
        );
    }
}

#[test]
fn detection_is_capped_and_replay_decodes_a_scalar_crossing_the_probe_boundary() {
    let mut input = vec![b'a'; FileReader::DETECTION_LIMIT - 1];
    input.extend_from_slice("😀Z".as_bytes());
    let source = ChunkedRead::new(input, 3073);
    let count = source.read_bytes.clone();
    let mut reader = FileReader::open_stream(source, TextEncoding::Latin1).unwrap();
    assert_eq!(count.get(), FileReader::DETECTION_LIMIT);
    assert_eq!(reader.encoding(), TextEncoding::Utf8);
    let result = reader.read_to_end().unwrap();
    assert_eq!(result.len(), FileReader::DETECTION_LIMIT + 4);
    assert!(
        result[..FileReader::DETECTION_LIMIT - 1]
            .bytes()
            .all(|b| b == b'a')
    );
    assert_eq!(&result[FileReader::DETECTION_LIMIT - 1..], "😀Z");
    let mut input = vec![b'a'; FileReader::DETECTION_LIMIT];
    input.push(0xff);
    let source = ChunkedRead::new(input, 4096);
    let count = source.read_bytes.clone();
    let mut reader = FileReader::open_stream(source, TextEncoding::Latin1).unwrap();
    assert_eq!(count.get(), FileReader::DETECTION_LIMIT);
    assert_eq!(reader.encoding(), TextEncoding::Latin1);
    assert!(reader.read_to_end().unwrap().ends_with('ÿ'));
}

#[test]
fn malformed_utf8_matches_standard_maximal_subpart_replacement_across_chunks() {
    for input in [
        &b"a\xc0\xafz"[..],
        &[0xe0, 0x80, 0x80],
        &[0xed, 0xa0, 0x80],
        &[0xf4, 0x90, 0x80, 0x80],
        &[0xf5, 0x80, 0x80, 0x80],
        &[0xe1, 0x80, b'A'],
        &[0xf0, 0x90, 0x80],
        &[0xe2],
        &b"\x80\xbfABC\xe2\x82\xac\xf0\x9f\x98\x80"[..],
    ] {
        for chunk in 1..=4 {
            let mut reader = FileTextReader::new(
                ChunkedRead::new(input, chunk),
                TextEncoding::Utf8,
                DecodingErrorPolicy::Replace,
            );
            assert_eq!(
                reader.read_to_end().unwrap(),
                String::from_utf8_lossy(input)
            );
        }
    }
}

#[test]
fn malformed_utf16_and_utf32_replace_invalid_units_and_truncated_words() {
    for encoding in [TextEncoding::Utf16Le, TextEncoding::Utf16Be] {
        let units = [0xd800_u16, 0x41, 0xdc00, 0xd83d, 0xde00, 0xd800];
        let mut bytes = Vec::new();
        for unit in units {
            bytes.extend_from_slice(&if encoding == TextEncoding::Utf16Le {
                unit.to_le_bytes()
            } else {
                unit.to_be_bytes()
            });
        }
        bytes.push(0x61);
        let mut reader = FileTextReader::new(
            ChunkedRead::new(bytes, 1),
            encoding,
            DecodingErrorPolicy::Replace,
        );
        assert_eq!(reader.read_to_end().unwrap(), "�A�😀��");
    }
    for encoding in [TextEncoding::Utf32Le, TextEncoding::Utf32Be] {
        let mut bytes = Vec::new();
        for scalar in [0xd800_u32, 0x110000, 0x41, 0x1f600] {
            bytes.extend_from_slice(&if encoding == TextEncoding::Utf32Le {
                scalar.to_le_bytes()
            } else {
                scalar.to_be_bytes()
            });
        }
        bytes.extend_from_slice(&[1, 2, 3]);
        let mut reader = FileTextReader::new(
            ChunkedRead::new(bytes, 1),
            encoding,
            DecodingErrorPolicy::Replace,
        );
        assert_eq!(reader.read_to_end().unwrap(), "��A😀�");
    }
}

#[test]
fn strict_reads_preserve_completed_output_and_report_the_encoded_byte_offset() {
    let options = FileReaderOptions {
        default_encoding: TextEncoding::Utf8,
        error_policy: DecodingErrorPolicy::Strict,
    };
    let mut reader =
        FileReader::open_stream_with_options(&b"\xef\xbb\xbfA\xffB"[..], options).unwrap();
    let mut units = [0; 8];
    assert_eq!(reader.read_into(&mut units).unwrap(), 1);
    assert_eq!(units[0], b'A' as u16);
    assert_eq!(reader.read_into(&mut []).unwrap(), 0);
    let error = reader.read_into(&mut units).unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidData);
    assert!(error.to_string().contains("byte offset 4"));
    // Strict errors consume the offending maximal subpart; a caller that
    // intentionally recovers can continue with the following text.
    assert_eq!(reader.read_to_end().unwrap(), "B");
    let mut reader = FileTextReader::new(
        &b"A\xffB"[..],
        TextEncoding::Utf8,
        DecodingErrorPolicy::Strict,
    )
    .into_utf8();
    let mut output = [0; 8];
    assert_eq!(reader.read(&mut output).unwrap(), 1);
    assert_eq!(output[0], b'A');
    assert_eq!(
        reader.read(&mut output).unwrap_err().kind(),
        ErrorKind::InvalidData
    );
    assert_eq!(reader.read(&mut output).unwrap(), 1);
    assert_eq!(output[0], b'B');
    assert_eq!(reader.read(&mut output).unwrap(), 0);
}

#[test]
fn io_errors_mid_scalar_retain_partial_input_and_do_not_replay_previous_output() {
    for utf8_adapter in [false, true] {
        let mut source = ChunkedRead::new("A😀B".as_bytes(), 1);
        source.fail_at = Some(3);
        let mut reader =
            FileTextReader::new(source, TextEncoding::Utf8, DecodingErrorPolicy::Replace);
        if utf8_adapter {
            let mut reader = reader.into_utf8();
            assert_eq!(reader.fill_buf().unwrap(), b"A");
            reader.consume(1);
            assert_eq!(reader.fill_buf().unwrap_err().kind(), ErrorKind::WouldBlock);
            assert_eq!(reader.fill_buf().unwrap(), "😀".as_bytes());
            assert_eq!(reader.fill_buf().unwrap(), "😀".as_bytes());
            let mut actual = String::new();
            reader.read_to_string(&mut actual).unwrap();
            assert_eq!(actual, "😀B");
        } else {
            let mut buffer = [0; 8];
            assert_eq!(reader.read_into(&mut buffer).unwrap(), 1);
            assert_eq!(buffer[0], b'A' as u16);
            assert_eq!(
                reader.read_into(&mut buffer).unwrap_err().kind(),
                ErrorKind::WouldBlock
            );
            assert_eq!(reader.read_to_end().unwrap(), "😀B");
        }
    }
}

#[test]
fn utf16_peek_lines_and_explicit_utf8_projection_share_the_unread_cursor() {
    let bytes = encoded("😀\r\n\rB\nC\u{2028}Z", TextEncoding::Utf16Be, true);
    let mut reader =
        FileReader::open_stream(ChunkedRead::new(bytes, 1), TextEncoding::Latin1).unwrap();
    assert_eq!(reader.peek().unwrap(), Some(0xd83d));
    assert_eq!(reader.peek().unwrap(), Some(0xd83d));
    assert_eq!(reader.read_line().unwrap().as_deref(), Some("😀"));
    assert_eq!(reader.read_line().unwrap().as_deref(), Some(""));
    assert_eq!(reader.read_line().unwrap().as_deref(), Some("B"));
    assert_eq!(reader.read_line().unwrap().as_deref(), Some("C\u{2028}Z"));
    assert_eq!(reader.read_line().unwrap(), None);
    let mut reader = FileTextReader::new(
        "😀X".as_bytes(),
        TextEncoding::Utf8,
        DecodingErrorPolicy::Replace,
    );
    assert_eq!(reader.read().unwrap(), Some(0xd83d));
    let mut projected = reader.into_utf8();
    let mut result = Vec::new();
    let mut one_byte = [0];
    while projected.read(&mut one_byte).unwrap() != 0 {
        result.push(one_byte[0]);
    }
    assert_eq!(result, "�X".as_bytes());
}

#[test]
fn explicit_encoding_does_not_probe_or_strip_an_in_band_bom() {
    for encoding in [
        TextEncoding::Utf16Le,
        TextEncoding::Utf16Be,
        TextEncoding::Utf32Le,
        TextEncoding::Utf32Be,
    ] {
        let source = ChunkedRead::new(encoded("A😀", encoding, true), 1);
        let calls = source.read_bytes.clone();
        let mut reader = FileTextReader::new(source, encoding, DecodingErrorPolicy::Replace);
        assert_eq!(calls.get(), 0);
        assert!(!reader.has_byte_order_mark());
        assert_eq!(reader.read_to_end().unwrap(), "\u{feff}A😀");
    }
}

#[test]
fn utf16_sink_streaming_uses_existing_document_writer_and_caller_undo_group() {
    let text = format!("{}😀\r\nend", "a".repeat(4095));
    let source: Box<dyn Read> = Box::new(ChunkedRead::new(
        encoded(&text, TextEncoding::Utf32Be, true),
        3,
    ));
    let mut reader = FileReader::open_stream(source, TextEncoding::Latin1).unwrap();
    let document = TextDocument::from_text("prefixsuffix");
    let group = document.begin_update().unwrap();
    let mut writer = DocumentTextWriter::new(document.clone(), 6).unwrap();
    assert_eq!(
        reader.copy_to(&mut writer).unwrap(),
        text.encode_utf16().count()
    );
    group.finish().unwrap();
    assert_eq!(document.text(), format!("prefix{text}suffix"));
    assert_eq!(document.undo_count(), 1);
    document.undo().unwrap();
    assert_eq!(document.text(), "prefixsuffix");

    struct FailingSink {
        calls: usize,
        accepted: Vec<u16>,
    }
    impl Utf16TextWriter for FailingSink {
        fn write_utf16(&mut self, text: &[u16]) -> io::Result<()> {
            self.calls += 1;
            if self.calls == 2 {
                return Err(io::Error::from(ErrorKind::WriteZero));
            }
            self.accepted.extend_from_slice(text);
            Ok(())
        }
    }
    let mut reader = FileReader::open_stream(text.as_bytes(), TextEncoding::Utf8).unwrap();
    let mut sink = FailingSink {
        calls: 0,
        accepted: Vec::new(),
    };
    assert_eq!(
        reader.copy_to(&mut sink).unwrap_err().kind(),
        ErrorKind::WriteZero
    );
    assert_eq!(sink.accepted.len(), 4096);
    assert_eq!(
        sink.accepted,
        text.encode_utf16().take(4096).collect::<Vec<_>>()
    );
}

#[test]
fn seekable_validation_and_borrowed_stream_ownership_are_explicit() {
    let mut cursor = Cursor::new(b"before".to_vec());
    cursor.set_position(2);
    let error = FileReader::open_seekable_stream(&mut cursor, TextEncoding::Utf8)
        .err()
        .unwrap();
    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert_eq!(cursor.position(), 2);
    assert_eq!(
        FileReader::read_stream_content(&mut cursor, TextEncoding::Utf8).unwrap(),
        "fore"
    );
    cursor.set_position(0);
    let mut reader = FileReader::open_seekable_stream(&mut cursor, TextEncoding::Utf8).unwrap();
    assert_eq!(reader.read_to_end().unwrap(), "before");
    drop(reader);
    assert_eq!(cursor.position(), 6);
}

#[test]
#[cfg(not(target_family = "wasm"))]
fn path_helpers_decode_real_files_and_propagate_file_errors() {
    struct TempFile(std::path::PathBuf);
    impl Drop for TempFile {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let file = TempFile(std::env::temp_dir().join(format!(
        "ferroedit-file-reader-{}-{unique}.txt",
        std::process::id()
    )));
    std::fs::write(
        &file.0,
        encoded("from file 😀", TextEncoding::Utf16Le, true),
    )
    .unwrap();
    assert_eq!(
        FileReader::read_file_content(&file.0, TextEncoding::Latin1).unwrap(),
        "from file 😀"
    );
    let mut reader =
        FileReader::open_file_with_options(&file.0, FileReaderOptions::default()).unwrap();
    assert_eq!(reader.encoding(), TextEncoding::Utf16Le);
    assert_eq!(reader.read_to_end().unwrap(), "from file 😀");
    drop(reader);
    std::fs::remove_file(&file.0).unwrap();
    assert_eq!(
        FileReader::open_file(&file.0, TextEncoding::Utf8)
            .err()
            .unwrap()
            .kind(),
        ErrorKind::NotFound
    );
}
