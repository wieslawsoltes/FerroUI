// Independent regressions for the Rust stream adaptation of the four TOM APIs.
use super::*;
use ferroui_base::media::FontWeight;
use ferroui_base::threading::Dispatcher;
use std::io::Cursor;

fn services(text: &str) -> RichEditTextDocument {
    RichEditTextDocument::new(RichTextDocument::from_text(text))
}

fn policy(encoding: TextStreamEncoding) -> TextStreamOptions {
    TextStreamOptions {
        encoding,
        ..Default::default()
    }
}

fn encoded(text: &str, encoding: TextStreamEncoding, with_bom: bool) -> Vec<u8> {
    let mut result = if with_bom {
        bom(encoding).to_vec()
    } else {
        Vec::new()
    };
    match encoding {
        TextStreamEncoding::Utf8 => result.extend_from_slice(text.as_bytes()),
        TextStreamEncoding::Utf16LittleEndian => {
            result.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
        }
        TextStreamEncoding::Utf16BigEndian => {
            result.extend(text.encode_utf16().flat_map(u16::to_be_bytes));
        }
    }
    result
}

#[test]
fn stream_encodings_round_trip_scalars_and_matching_byte_order_marks() {
    let _scope = Dispatcher::unit_test_scope();
    let text = "café\n𝄞😀\0tail";
    for encoding in [
        TextStreamEncoding::Utf8,
        TextStreamEncoding::Utf16LittleEndian,
        TextStreamEncoding::Utf16BigEndian,
    ] {
        for with_bom in [false, true] {
            let bytes = encoded(text, encoding, with_bom);
            let document = services("before");
            document
                .load_from_stream(
                    TextSetOptions::NONE,
                    &mut bytes.as_slice(),
                    policy(encoding),
                )
                .unwrap();
            assert_eq!(document.text(), text);
            let mut output = Vec::new();
            document
                .save_to_stream(
                    TextGetOptions::USE_LF,
                    &mut output,
                    TextStreamOptions {
                        write_byte_order_mark: with_bom,
                        ..policy(encoding)
                    },
                )
                .unwrap();
            assert_eq!(output, bytes);
        }
    }
}

#[test]
fn malformed_stream_encodings_preserve_composition_selection_and_history() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("committed");
    document.set_text("undoable").unwrap();
    document.editor().set_preedit("preview", None).unwrap();
    let before = document.document().snapshot();
    let selection = document.selection().selection();
    let revision = document.document().revision();
    let cases = [
        (vec![0xc3, 0x28], TextStreamEncoding::Utf8),
        (vec![0xff, 0xfe, b'x', 0], TextStreamEncoding::Utf8),
        (
            vec![0xef, 0xbb, 0xbf, b'x'],
            TextStreamEncoding::Utf16LittleEndian,
        ),
        (
            vec![0xfe, 0xff, 0, b'x'],
            TextStreamEncoding::Utf16LittleEndian,
        ),
        (
            vec![0xff, 0xfe, b'x', 0],
            TextStreamEncoding::Utf16BigEndian,
        ),
        (vec![b'x'], TextStreamEncoding::Utf16LittleEndian),
        (vec![0, 0xd8], TextStreamEncoding::Utf16LittleEndian),
        (vec![0xdc, 0], TextStreamEncoding::Utf16BigEndian),
    ];
    for (bytes, encoding) in cases {
        assert!(matches!(
            document.load_from_stream(
                TextSetOptions::NONE,
                &mut bytes.as_slice(),
                policy(encoding)
            ),
            Err(TextOperationError::InvalidEncoding(_))
        ));
        assert_eq!(document.document().snapshot(), before);
        assert_eq!(document.document().revision(), revision);
        assert_eq!(document.selection().selection(), selection);
        assert!(document.editor().is_composing());
    }
    document.editor().cancel_composition().unwrap();
    assert!(document.undo().unwrap());
    assert_eq!(document.text(), "committed");
}

#[test]
fn invalid_range_stream_preserves_live_endpoints_and_composition() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc");
    document.editor().set_preedit("preview", None).unwrap();
    let range = document.get_range(1, 3).unwrap();
    let before = document.document().snapshot();
    assert!(matches!(
        range.set_text_via_stream(
            TextSetOptions::NONE,
            &mut [0xed, 0xa0, 0x80].as_slice(),
            TextStreamOptions::default()
        ),
        Err(TextOperationError::InvalidEncoding(_))
    ));
    assert_eq!(range.range(), 1..3);
    assert_eq!(document.document().snapshot(), before);
    assert!(document.editor().is_composing());
}

#[test]
fn raw_rtf_streams_preserve_ansi_bytes_and_supported_formatting() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("before");
    let bytes = b"{\\rtf1\\ansi\\b caf\xe9}";
    document
        .load_from_stream(
            TextSetOptions::FORMAT_RTF,
            &mut bytes.as_slice(),
            TextStreamOptions::default(),
        )
        .unwrap();
    assert_eq!(document.text(), "café");
    assert_eq!(
        document
            .document()
            .character_format_at(0)
            .unwrap()
            .font_weight,
        Some(FontWeight::Bold)
    );
    let mut output = Vec::new();
    document
        .save_to_stream(
            TextGetOptions::FORMAT_RTF,
            &mut output,
            TextStreamOptions::default(),
        )
        .unwrap();
    let restored = services("");
    restored
        .load_from_stream(
            TextSetOptions::FORMAT_RTF,
            &mut output.as_slice(),
            TextStreamOptions::default(),
        )
        .unwrap();
    assert_eq!(restored.text(), "café");
    assert_eq!(
        restored
            .document()
            .character_format_at(0)
            .unwrap()
            .font_weight,
        Some(FontWeight::Bold)
    );
    assert!(document.undo().unwrap());
    assert_eq!(document.text(), "before");
}

#[test]
fn malformed_rtf_stream_is_validated_before_any_edit() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc");
    document.editor().set_preedit("preview", None).unwrap();
    let before = document.document().snapshot();
    let range = document.get_range(1, 2).unwrap();
    let bytes = b"{\\rtf1 incomplete";
    assert!(matches!(
        document.load_from_stream(
            TextSetOptions::FORMAT_RTF,
            &mut bytes.as_slice(),
            TextStreamOptions::default()
        ),
        Err(TextOperationError::Rtf(_))
    ));
    assert!(matches!(
        range.set_text_via_stream(
            TextSetOptions::FORMAT_RTF,
            &mut bytes.as_slice(),
            TextStreamOptions::default()
        ),
        Err(TextOperationError::Rtf(_))
    ));
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(range.range(), 1..2);
    assert!(document.editor().is_composing());
}

struct UntouchedStream;
impl Read for UntouchedStream {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        panic!("invalid options must not read")
    }
}
impl Write for UntouchedStream {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        panic!("invalid options must not write")
    }
    fn flush(&mut self) -> io::Result<()> {
        panic!("stream methods must not flush")
    }
}

#[test]
fn invalid_stream_flags_and_rtf_encoding_policy_do_not_access_the_stream() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc");
    let range = document.document_range().unwrap();
    let mut stream = UntouchedStream;
    for policy in [
        policy(TextStreamEncoding::Utf16LittleEndian),
        TextStreamOptions {
            write_byte_order_mark: true,
            ..Default::default()
        },
    ] {
        assert!(document
            .load_from_stream(TextSetOptions::FORMAT_RTF, &mut stream, policy)
            .is_err());
        assert!(document
            .save_to_stream(TextGetOptions::FORMAT_RTF, &mut stream, policy)
            .is_err());
        assert!(range
            .set_text_via_stream(TextSetOptions::FORMAT_RTF, &mut stream, policy)
            .is_err());
        assert!(range
            .get_text_via_stream(TextGetOptions::FORMAT_RTF, &mut stream, policy)
            .is_err());
    }
    assert!(document
        .load_from_stream(
            TextSetOptions::from_bits_retain(2),
            &mut stream,
            TextStreamOptions::default()
        )
        .is_err());
    assert!(document
        .save_to_stream(
            TextGetOptions::NO_HIDDEN,
            &mut stream,
            TextStreamOptions::default()
        )
        .is_err());
    assert!(range
        .set_text_via_stream(
            TextSetOptions::UNHIDE,
            &mut stream,
            TextStreamOptions::default()
        )
        .is_err());
    assert!(range
        .get_text_via_stream(
            TextGetOptions::USE_LF | TextGetOptions::USE_CRLF,
            &mut stream,
            TextStreamOptions::default()
        )
        .is_err());
}

#[test]
fn input_byte_limit_accepts_exact_payload_and_consumes_only_one_overflow_byte() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("before");
    let policy = TextStreamOptions {
        max_bytes: 3,
        ..Default::default()
    };
    let mut oversized = Cursor::new(b"0123456789");
    assert!(matches!(
        document.load_from_stream(TextSetOptions::NONE, &mut oversized, policy),
        Err(TextOperationError::StreamLimitExceeded { limit: 3 })
    ));
    assert_eq!(oversized.position(), 4);
    assert_eq!(document.text(), "before");
    assert!(!document.can_undo());
    document
        .load_from_stream(TextSetOptions::NONE, &mut b"abc".as_slice(), policy)
        .unwrap();
    assert_eq!(document.text(), "abc");
    let zero = TextStreamOptions {
        max_bytes: 0,
        ..Default::default()
    };
    document
        .load_from_stream(TextSetOptions::NONE, &mut b"".as_slice(), zero)
        .unwrap();
    assert_eq!(document.text(), "");
    assert!(matches!(
        document.load_from_stream(TextSetOptions::NONE, &mut b"x".as_slice(), zero),
        Err(TextOperationError::StreamLimitExceeded { limit: 0 })
    ));
}

struct InterruptedReader {
    bytes: Cursor<Vec<u8>>,
    interrupted: bool,
    fail_at_end: bool,
}
impl Read for InterruptedReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(io::ErrorKind::Interrupted.into());
        }
        if self.fail_at_end && self.bytes.position() == self.bytes.get_ref().len() as u64 {
            return Err(io::ErrorKind::ConnectionReset.into());
        }
        self.bytes.read(buffer)
    }
}

#[test]
fn interrupted_reads_retry_and_io_failure_after_a_prefix_is_transactional() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("before");
    for fail_at_end in [true, false] {
        let mut reader = InterruptedReader {
            bytes: Cursor::new(b"after".to_vec()),
            interrupted: false,
            fail_at_end,
        };
        let result = document.load_from_stream(
            TextSetOptions::NONE,
            &mut reader,
            TextStreamOptions::default(),
        );
        if fail_at_end {
            assert!(
                matches!(result, Err(TextOperationError::Io(error)) if error.kind() == io::ErrorKind::ConnectionReset)
            );
            assert_eq!(document.text(), "before");
            assert!(!document.can_undo());
        } else {
            result.unwrap();
            assert_eq!(document.text(), "after");
            assert!(document.undo().unwrap());
            assert_eq!(document.text(), "before");
        }
    }
}

struct PartialWriter {
    bytes: Vec<u8>,
    fail_after: Option<usize>,
    interrupted: bool,
}
impl Write for PartialWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if !self.interrupted {
            self.interrupted = true;
            return Err(io::ErrorKind::Interrupted.into());
        }
        let remaining = self
            .fail_after
            .map_or(usize::MAX, |limit| limit.saturating_sub(self.bytes.len()));
        if remaining == 0 {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        let written = remaining.min(2).min(buffer.len());
        self.bytes.extend_from_slice(&buffer[..written]);
        Ok(written)
    }
    fn flush(&mut self) -> io::Result<()> {
        panic!("the caller owns flush policy")
    }
}

#[test]
fn writes_retry_interrupts_and_partial_writes_without_flushing() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("café😀");
    let mut writer = PartialWriter {
        bytes: Vec::new(),
        fail_after: None,
        interrupted: false,
    };
    document
        .save_to_stream(
            TextGetOptions::USE_LF,
            &mut writer,
            TextStreamOptions::default(),
        )
        .unwrap();
    assert_eq!(writer.bytes, "café😀".as_bytes());
    assert!(!document.can_undo());
}

#[test]
fn write_failure_exposes_the_prefix_and_preserves_the_document_and_range() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("before");
    let range = document.get_range(1, 5).unwrap();
    let before = document.document().snapshot();
    let mut writer = PartialWriter {
        bytes: Vec::new(),
        fail_after: Some(3),
        interrupted: false,
    };
    assert!(
        matches!(range.get_text_via_stream(TextGetOptions::NONE, &mut writer, TextStreamOptions::default()), Err(TextOperationError::Io(error)) if error.kind() == io::ErrorKind::BrokenPipe)
    );
    assert_eq!(writer.bytes, b"efo");
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(range.range(), 1..5);
    assert!(!document.can_undo());
}

#[test]
fn output_byte_limit_includes_bom_and_fails_before_the_first_write() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("😀");
    let range = document.document_range().unwrap();
    let policy = TextStreamOptions {
        encoding: TextStreamEncoding::Utf16LittleEndian,
        write_byte_order_mark: true,
        max_bytes: 5,
    };
    assert!(matches!(
        range.get_text_via_stream(TextGetOptions::NONE, &mut UntouchedStream, policy),
        Err(TextOperationError::StreamLimitExceeded { limit: 5 })
    ));
    let mut output = Vec::new();
    range
        .get_text_via_stream(
            TextGetOptions::NONE,
            &mut output,
            TextStreamOptions {
                max_bytes: 6,
                ..policy
            },
        )
        .unwrap();
    assert_eq!(output, [0xff, 0xfe, 0x3d, 0xd8, 0, 0xde]);
    let empty = services("");
    empty
        .save_to_stream(
            TextGetOptions::USE_LF,
            &mut UntouchedStream,
            TextStreamOptions {
                max_bytes: 0,
                ..Default::default()
            },
        )
        .unwrap();
}

#[test]
fn streams_use_the_current_position_without_seeking_or_truncating() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("before");
    let mut input = Cursor::new(b"prefixbody".to_vec());
    input.set_position(6);
    document
        .load_from_stream(
            TextSetOptions::NONE,
            &mut input,
            TextStreamOptions::default(),
        )
        .unwrap();
    assert_eq!(document.text(), "body");
    assert_eq!(input.position(), 10);
    let mut output = Cursor::new(b"PREFIX........TAIL".to_vec());
    output.set_position(6);
    document
        .save_to_stream(
            TextGetOptions::USE_LF,
            &mut output,
            TextStreamOptions::default(),
        )
        .unwrap();
    assert_eq!(output.position(), 10);
    assert_eq!(output.into_inner(), b"PREFIXbody....TAIL");
}

#[test]
fn streamed_range_replacement_rebases_selection_and_is_one_undo_unit() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abcd");
    document.selection().set_range(4, 4).unwrap();
    let range = document.get_range(1, 3).unwrap();
    let bytes = encoded("😀Q", TextStreamEncoding::Utf16BigEndian, false);
    range
        .set_text_via_stream(
            TextSetOptions::NONE,
            &mut bytes.as_slice(),
            policy(TextStreamEncoding::Utf16BigEndian),
        )
        .unwrap();
    assert_eq!(document.text(), "a😀Qd");
    assert_eq!(range.range(), 1..4);
    assert_eq!(document.selection().selection(), EditorSelection::caret(5));
    assert!(document.undo().unwrap());
    assert_eq!(document.text(), "abcd");
    assert_eq!(document.selection().selection(), EditorSelection::caret(4));
    assert!(!document.can_undo());
}

#[test]
fn streamed_range_rtf_retains_format_and_check_text_limit_uses_utf16() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abcd");
    document.editor().set_max_length(5);
    let range = document.get_range(1, 3).unwrap();
    let rtf = b"{\\rtf1\\ansi\\b X\\u-10179?\\u-8704?YZ}";
    range
        .set_text_via_stream(
            TextSetOptions::FORMAT_RTF | TextSetOptions::CHECK_TEXT_LIMIT,
            &mut rtf.as_slice(),
            TextStreamOptions::default(),
        )
        .unwrap();
    assert_eq!(document.text(), "aX😀d");
    let TextFormatValue::Uniform(format) = range.character_format().unwrap() else {
        panic!("the inserted RTF range has one character format");
    };
    assert_eq!(format.font_weight, Some(FontWeight::Bold));
    assert_eq!(document.editor().max_length(), 5);
    let mut output = Vec::new();
    range
        .get_text_via_stream(
            TextGetOptions::FORMAT_RTF,
            &mut output,
            TextStreamOptions::default(),
        )
        .unwrap();
    assert_eq!(RtfReader::read(&output).unwrap().text(), "X😀");
}

#[test]
fn stream_output_snapshot_is_stable_during_writer_reentrancy() {
    let _scope = Dispatcher::unit_test_scope();
    struct MutatingWriter {
        document: RichEditTextDocument,
        output: Vec<u8>,
    }
    impl Write for MutatingWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.document.set_text("changed from writer").unwrap();
            self.output.extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            panic!("no implicit flush")
        }
    }
    let document = services("snapshot");
    let mut writer = MutatingWriter {
        document: document.clone(),
        output: Vec::new(),
    };
    document
        .save_to_stream(
            TextGetOptions::USE_LF,
            &mut writer,
            TextStreamOptions::default(),
        )
        .unwrap();
    assert_eq!(writer.output, b"snapshot");
    assert_eq!(document.text(), "changed from writer");
}
