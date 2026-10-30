use super::{RtfError, RtfLimits, RtfReader, RtfWriter};
use crate::documents::{CharacterFormat, InlineSnapshotKind, RichTextDocument, TextGravity};
use ferroui_base::media::{FontFamily, FontWeight};
use ferroui_base::threading::Dispatcher;

#[test]
fn rich_edit_rtf_code_pages_decode_independent_multilingual_fixtures() {
    let _scope = Dispatcher::unit_test_scope();
    // Literal bytes also checked with independent platform code-page tables.
    let cases: &[(i32, &[u8], &str)] = &[
        (874, br"\'e4\'b7\'c2", "ไทย"),
        (932, br"\'93\'fa\'96\'7b\'8c\'ea", "日本語"),
        (936, br"\'d6\'d0\'ce\'c4", "中文"),
        (949, br"\'c7\'d1\'b1\'db", "한글"),
        (950, br"\'a4\'a4\'a4\'e5", "中文"),
        (1250, br"Za\'bf\'f3\'b3\'e6", "Zażółć"),
        (1251, br"\'cf\'f0\'e8\'e2\'e5\'f2", "Привет"),
        (1252, br"\'80\'e9", "€é"),
        (1253, br"\'c5\'eb\'eb\'dc\'e4\'e1", "Ελλάδα"),
        (1254, br"\'d0\'fd\'fe", "Ğış"),
        (1255, br"\'f9\'ec\'e5\'ed", "שלום"),
        (1256, br"\'e3\'d1\'cd\'c8\'c7", "مرحبا"),
        (1257, br"\'c2\'feu", "Āžu"),
        (1258, br"Vi\'ea\'f2t", "Viê\u{323}t"),
        (10000, br"caf\'8e", "café"),
        (10007, br"\'8f\'f0\'e8\'e2\'e5\'f2", "Привет"),
        (65001, br"A\'f0\'9f\'98\'80\'e3\'81\'82", "A😀あ"),
    ];
    for (page, bytes, expected) in cases {
        let mut input = format!("{{\\rtf1\\ansi\\ansicpg{page} ").into_bytes();
        input.extend_from_slice(bytes);
        input.push(b'}');
        let snapshot = RtfReader::read(&input).unwrap();
        assert_eq!(snapshot.text(), *expected, "code page {page}");
        let output = RtfWriter::write(&snapshot).unwrap();
        assert_eq!(RtfReader::read_str(&output).unwrap().text(), *expected);
    }
}

#[test]
fn rich_edit_rtf_multibyte_sequences_span_raw_and_escaped_tokens() {
    let _scope = Dispatcher::unit_test_scope();
    let input = b"{\\rtf1\\ansicpg932 \\'82\xa0\x82\\'a2}";
    assert_eq!(RtfReader::read(input).unwrap().text(), "あい");
    let input = b"{\\rtf1\\ansicpg65001 \xf0\\'9f\x98\\'80}";
    assert_eq!(RtfReader::read(input).unwrap().text(), "😀");
}

#[test]
fn rich_edit_rtf_multibyte_escaped_delimiters_are_text() {
    let _scope = Dispatcher::unit_test_scope();
    assert_eq!(
        RtfReader::read(br"{\rtf1\ansicpg932 \'95\'5c\'96\'7b}")
            .unwrap()
            .text(),
        "表本"
    );
}

#[test]
fn rich_edit_rtf_font_charsets_are_scoped_and_plain_restores_default_font() {
    let _scope = Dispatcher::unit_test_scope();
    let snapshot = RtfReader::read(br"{\rtf1\ansi\ansicpg1250\deff0{\fonttbl{\f0\fcharset238 Latin;}{\f1\fcharset204 Cyrillic;}{\f2\fcharset128 Japanese;}}\'b3{\f1\b \'cf\'f0}{\f2 \'93\'fa}\'bf\f1\'e8\plain\'b3}").unwrap();
    assert_eq!(snapshot.text(), "łПр日żиł");
    let model = RichTextDocument::new();
    model.restore(&snapshot).unwrap();
    // Probe the character after each offset; the caret-format API intentionally
    // uses the preceding run at a boundary.
    let character = |offset| {
        CharacterFormat::effective(&model.element_at(offset, TextGravity::Forward).unwrap())
    };
    assert_eq!(character(0).font_family, Some(FontFamily::new("Latin")));
    assert_eq!(character(1).font_family, Some(FontFamily::new("Cyrillic")));
    assert_eq!(character(1).font_weight, Some(FontWeight::Bold));
    assert_eq!(character(3).font_family, Some(FontFamily::new("Japanese")));
    assert_eq!(character(4).font_family, Some(FontFamily::new("Latin")));
    assert_eq!(character(6).font_family, Some(FontFamily::new("Latin")));
}

#[test]
fn rich_edit_rtf_font_code_page_overrides_charset_in_either_order() {
    let _scope = Dispatcher::unit_test_scope();
    for declaration in [
        r"\fcharset0\cpg1251",
        r"\cpg1251\fcharset0",
        r"\fcharset255\cpg1251",
        r"\cpg1251\fcharset255",
        r"\fcharset130\cpg1251",
        r"\cpg1251\fcharset130",
        r"\fcharset2\cpg1251",
        r"\cpg1251\fcharset2",
        r"\fcharset999\cpg1251",
        r"\cpg1251\fcharset999",
    ] {
        let input = format!(r"{{\rtf1\deff0{{\fonttbl{{\f0{declaration} Cyrillic;}}}}\'cf\'f0}}");
        assert_eq!(RtfReader::read_str(&input).unwrap().text(), "Пр");
    }
    for (charset, page) in [(255, 437), (130, 1361)] {
        let input = format!(r"{{\rtf1{{\fonttbl{{\f0\fcharset{charset} Unsupported;}}}}text}}");
        assert!(matches!(
            RtfReader::read_str(&input),
            Err(RtfError::UnsupportedCodePage(actual)) if actual == page
        ));
    }
    for charset in [2, 999] {
        // Check name-byte decoding and finalization without any legacy bytes.
        for name in ["Unsupported", r"\uc0\u83", ""] {
            let input = format!(r"{{\rtf1{{\fonttbl{{\f0\fcharset{charset} {name};}}}}text}}");
            assert!(matches!(
                RtfReader::read_str(&input),
                Err(RtfError::UnsupportedFeature("font character set"))
            ));
        }
    }
}

#[test]
fn rich_edit_rtf_font_code_page_override_keeps_multibyte_name_decoder_continuous() {
    let _scope = Dispatcher::unit_test_scope();
    for declaration in [
        r"\fcharset2\cpg932",
        r"\cpg932\fcharset2",
        r"\fcharset999\cpg932",
        r"\cpg932\fcharset999",
    ] {
        let mut input = format!(r"{{\rtf1\deff0{{\fonttbl{{\f0{declaration} ").into_bytes();
        input.extend_from_slice(b"\\'96\xbe\x92\\'a9;}}\\'93\xfa\x96\\'7b}");
        let snapshot = RtfReader::read(&input).unwrap();
        assert_eq!(snapshot.text(), "日本", "{declaration}");
        let model = RichTextDocument::new();
        model.restore(&snapshot).unwrap();
        assert_eq!(
            model.character_format_at(0).unwrap().font_family,
            Some(FontFamily::new("明朝")),
            "{declaration}"
        );
    }
}

#[test]
fn rich_edit_rtf_default_charset_uses_declared_document_code_page() {
    let _scope = Dispatcher::unit_test_scope();
    let input = br"{\rtf1\ansicpg1250\deff0{\fonttbl{\f0\fcharset1 Default;}{\f1\fcharset0 ANSI;}}\'b3{\f1\'b3}\'b3}";
    assert_eq!(RtfReader::read(input).unwrap().text(), "ł³ł");
}

#[test]
fn rich_edit_rtf_code_page_applies_to_font_names_and_field_destinations() {
    let _scope = Dispatcher::unit_test_scope();
    let snapshot = RtfReader::read(br#"{\rtf1\ansicpg1252\deff0{\fonttbl{\f0\fcharset128 \'96\'be\'92\'a9;}}{\field{\*\fldinst HYPERLINK "https://example.org/\'93\'fa"}{\fldrslt \'96\'7b}}}"#).unwrap();
    let model = RichTextDocument::new();
    model.restore(&snapshot).unwrap();
    assert_eq!(snapshot.text(), "本");
    assert_eq!(
        model.character_format_at(0).unwrap().font_family,
        Some(FontFamily::new("明朝"))
    );
    assert!(matches!(&snapshot.paragraphs[0].inlines[0].kind,
        InlineSnapshotKind::Hyperlink { navigate_uri: Some(uri), .. }
        if uri == "https://example.org/日"));
}

#[test]
fn rich_edit_rtf_unicode_fallback_bytes_do_not_reach_the_legacy_decoder() {
    let _scope = Dispatcher::unit_test_scope();
    let snapshot = RtfReader::read(br"{\rtf1\ansicpg932\uc2 \u26085\'93\'fa\'96\'7b}").unwrap();
    assert_eq!(snapshot.text(), "日本");
    let snapshot = RtfReader::read(br"{\rtf1\ansicpg65001\uc1 \u-10179?\u-8704?}").unwrap();
    assert_eq!(snapshot.text(), "😀");
}

#[test]
fn rich_edit_rtf_incomplete_sequences_fail_at_every_semantic_boundary() {
    let _scope = Dispatcher::unit_test_scope();
    for input in [
        br"{\rtf1\ansicpg932 \'82}".as_slice(),
        br"{\rtf1\ansicpg932 \'82\b\'a0}".as_slice(),
        br"{\rtf1\ansicpg932 \'82{\'a0}}".as_slice(),
        br"{\rtf1\ansicpg932 {\'82}\'a0}".as_slice(),
        br"{\rtf1\ansicpg932 \'82\u12354?}".as_slice(),
        br"{\rtf1\ansicpg932\deff0{\fonttbl{\f0 \'82;}}x}".as_slice(),
        br#"{\rtf1\ansicpg932{\field{\*\fldinst HYPERLINK "\'82"}{\fldrslt x}}}"#.as_slice(),
    ] {
        assert!(
            matches!(
                RtfReader::read(input),
                Err(RtfError::InvalidEncodedText { code_page: 932 })
            ),
            "{}",
            String::from_utf8_lossy(input)
        );
    }
}

#[test]
fn rich_edit_rtf_invalid_utf8_is_rejected_without_replacement() {
    let _scope = Dispatcher::unit_test_scope();
    for bytes in [
        br"\'c0\'af".as_slice(),
        br"\'ed\'a0\'80",
        br"\'f4\'90\'80\'80",
        br"\'e3\'81",
    ] {
        let mut input = br"{\rtf1\ansicpg65001 ".to_vec();
        input.extend_from_slice(bytes);
        input.push(b'}');
        assert!(matches!(
            RtfReader::read(&input),
            Err(RtfError::InvalidEncodedText { code_page: 65001 })
        ));
    }
}

#[test]
fn rich_edit_rtf_code_page_changes_restore_with_group_state() {
    let _scope = Dispatcher::unit_test_scope();
    let input = br"{\rtf1\ansicpg1250 \'b3{\ansicpg1251\'cf}\'bf}";
    assert_eq!(RtfReader::read(input).unwrap().text(), "łПż");
    assert_eq!(
        RtfReader::read(br"{\rtf1\mac caf\'8e}").unwrap().text(),
        "café"
    );
}

#[test]
fn rich_edit_rtf_code_page_limits_count_decoded_utf16_units() {
    let _scope = Dispatcher::unit_test_scope();
    let input = br"{\rtf1\ansicpg65001 \'f0\'9f\'98\'80}";
    assert_eq!(
        RtfReader::read_with_limits(
            input,
            RtfLimits {
                max_text_length: 2,
                ..RtfLimits::default()
            }
        )
        .unwrap()
        .text(),
        "😀"
    );
    assert!(matches!(
        RtfReader::read_with_limits(
            input,
            RtfLimits {
                max_text_length: 1,
                ..RtfLimits::default()
            }
        ),
        Err(RtfError::LimitExceeded("text length"))
    ));
}

#[test]
fn rich_edit_rtf_long_multibyte_runs_cross_decoder_output_buffers() {
    let _scope = Dispatcher::unit_test_scope();
    let text = "日本😀".repeat(1025);
    let input = format!("{{\\rtf1\\ansicpg65001 {text}}}");
    let snapshot = RtfReader::read_str(&input).unwrap();
    assert_eq!(snapshot.text(), text);
    assert_eq!(snapshot.paragraphs[0].inlines.len(), 1);
}

#[test]
fn rich_edit_rtf_ignored_destination_does_not_inject_hyperlink_target_text() {
    let _scope = Dispatcher::unit_test_scope();
    let snapshot = RtfReader::read(br#"{\rtf1{\field{\*\fldinst HYPERLINK "https://example.org/{\*\unknown hidden}safe"}{\fldrslt link}}}"#).unwrap();
    assert!(matches!(&snapshot.paragraphs[0].inlines[0].kind,
        InlineSnapshotKind::Hyperlink { navigate_uri: Some(uri), .. }
        if uri == "https://example.org/safe"));
}
