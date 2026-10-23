//! Literal stylesheet fixtures from the RTF grammar, rather than writer-generated
//! input. Named style identity is deliberately distinct from resolved appearance.

use super::{RtfError, RtfLimits, RtfReader, RtfWriter};
use crate::documents::{DocumentSnapshot, LineSpacingRule, TabAlignment, TabLeader};
use crate::rich_text::TextSetOptions;
use crate::RichEditBox;
use ferroui_base::media::{FontStyle, FontWeight, TextAlignment};
use ferroui_base::threading::Dispatcher;

fn read(text: &str) -> DocumentSnapshot {
    RtfReader::read_str(text).unwrap()
}

#[test]
fn rich_rtf_style_forward_bases_and_direct_overrides_resolve_in_order() {
    let _scope = Dispatcher::unit_test_scope();
    let document = read(concat!(
        r"{\rtf1{\stylesheet",
        r"{\s2\sbasedon1\i\fs36 Derived;}",
        r"{\s1\sbasedon0\b\qc\li360 Base;}",
        r"{\s0\sbasedon222\fs24\ql Normal;}}",
        r"\s2\b0 first\par\s1 second}"
    ));
    assert_eq!(document.text(), "first\nsecond");
    let first = &document.paragraphs[0];
    assert_eq!(first.format.text_alignment, Some(TextAlignment::Center));
    assert_eq!(first.format.left_indent, Some(24.0));
    assert_eq!(first.inlines[0].format.font_size, Some(24.0));
    assert_eq!(
        first.inlines[0].format.font_weight,
        Some(FontWeight::Normal)
    );
    assert_eq!(first.inlines[0].format.font_style, Some(FontStyle::Italic));
    let second = &document.paragraphs[1].inlines[0].format;
    assert_eq!(second.font_size, Some(16.0));
    assert_eq!(second.font_weight, Some(FontWeight::Bold));
    assert_eq!(second.font_style, Some(FontStyle::Normal));
}

#[test]
fn rich_rtf_style_additive_character_format_keeps_the_paragraph_font() {
    let _scope = Dispatcher::unit_test_scope();
    let document = read(concat!(
        r"{\rtf1{\stylesheet{\s1\b\fs36\qr Heading;}",
        r"{\*\cs10\additive Default Paragraph Font;}",
        r"{\*\cs11\sbasedon10\additive\i Emphasis;}}",
        r"\s1 A{\cs11 B}C}"
    ));
    assert_eq!(document.text(), "ABC");
    let paragraph = &document.paragraphs[0];
    assert_eq!(paragraph.format.text_alignment, Some(TextAlignment::Right));
    assert_eq!(paragraph.inlines.len(), 3);
    for inline in &paragraph.inlines {
        assert_eq!(inline.format.font_size, Some(24.0));
        assert_eq!(inline.format.font_weight, Some(FontWeight::Bold));
    }
    assert_eq!(
        paragraph.inlines[0].format.font_style,
        Some(FontStyle::Normal)
    );
    assert_eq!(
        paragraph.inlines[1].format.font_style,
        Some(FontStyle::Italic)
    );
    assert_eq!(
        paragraph.inlines[2].format.font_style,
        Some(FontStyle::Normal)
    );
}

#[test]
fn rich_rtf_style_nonadditive_character_format_resets_only_character_properties() {
    let _scope = Dispatcher::unit_test_scope();
    let document = read(concat!(
        r"{\rtf1{\stylesheet{\s1\b\fs36\qr\li120 Heading;}",
        r"{\*\cs11\i Emphasis;}}\s1 A{\cs11 B}C}"
    ));
    let paragraph = &document.paragraphs[0];
    assert_eq!(paragraph.format.text_alignment, Some(TextAlignment::Right));
    assert_eq!(paragraph.format.left_indent, Some(8.0));
    assert_eq!(
        paragraph.inlines[1].format.font_weight,
        Some(FontWeight::Normal)
    );
    assert_eq!(paragraph.inlines[1].format.font_size, Some(16.0));
    assert_eq!(
        paragraph.inlines[1].format.font_style,
        Some(FontStyle::Italic)
    );
    assert_eq!(
        paragraph.inlines[2].format.font_weight,
        Some(FontWeight::Bold)
    );
    assert_eq!(paragraph.inlines[2].format.font_size, Some(24.0));
}

#[test]
fn rich_rtf_style_font_charset_drives_scoped_body_decoding() {
    let _scope = Dispatcher::unit_test_scope();
    let document = read(concat!(
        r"{\rtf1\ansi\deff0{\fonttbl{\f0\fcharset0 Latin;}{\f1\fcharset238 Polish;}}",
        r"{\stylesheet{\s0\f0 Normal;}{\s1\sbasedon0\f1\b Polish;}}",
        r"\s1 \'a5\par\s0 \'a5}"
    ));
    assert_eq!(document.text(), "Ą\n¥");
    assert_eq!(
        document.paragraphs[0].inlines[0]
            .format
            .font_family
            .as_ref()
            .unwrap()
            .name(),
        "Polish"
    );
    assert_eq!(
        document.paragraphs[1].inlines[0]
            .format
            .font_family
            .as_ref()
            .unwrap()
            .name(),
        "Latin"
    );
}

#[test]
fn rich_rtf_style_metrics_tabs_and_empty_paragraph_defaults_use_shared_decoder() {
    let _scope = Dispatcher::unit_test_scope();
    let document = read(concat!(
        r"{\rtf1\deftab960{\stylesheet{\s1\b\qc\li-120\ri240\fi-60",
        r"\sb120\sa180\sl360\slmult1\tqr\tldot\tx1440 Metrics;}}",
        r"\s1\par\s1 text\tab right}"
    ));
    assert_eq!(document.text(), "\ntext\tright");
    for paragraph in &document.paragraphs {
        let format = &paragraph.format;
        assert_eq!(format.text_alignment, Some(TextAlignment::Center));
        assert_eq!(format.default_tab_stop, Some(64.0));
        assert_eq!(format.left_indent, Some(-8.0));
        assert_eq!(format.right_indent, Some(16.0));
        assert_eq!(format.text_indent, Some(-4.0));
        assert_eq!(format.line_spacing_rule, Some(LineSpacingRule::Multiple));
        assert_eq!(format.line_spacing, Some(1.5));
        let stop = format.tab_stops.as_ref().unwrap().get(0).unwrap();
        assert_eq!(stop.alignment, TabAlignment::Right);
        assert_eq!(stop.leader, TabLeader::Dots);
        assert_eq!(stop.position, 96.0);
    }
    assert_eq!(
        document.paragraphs[0].format.character.font_weight,
        Some(FontWeight::Bold)
    );
}

#[test]
fn rich_rtf_style_names_and_keycodes_never_become_document_content() {
    let _scope = Dispatcher::unit_test_scope();
    let document = read(concat!(
        r"{\rtf1{\stylesheet{\s0\fs24 Normal;}",
        r"{\s1\sbasedon0\b{\*\keycode\ctrl\shift x}N\u261?me;}",
        r"{\*\vendorstyles\s0\fs99 Invisible extension;}",
        r"{\*\ts8\tsrowd\trgaph108 Table;}",
        r"{\*\ds9 Section;}}\s1 body}"
    ));
    assert_eq!(document.text(), "body");
    assert_eq!(
        document.paragraphs[0].inlines[0].format.font_weight,
        Some(FontWeight::Bold)
    );
}

#[test]
fn rich_rtf_style_optional_default_definition_and_unused_cycles_are_safe() {
    let _scope = Dispatcher::unit_test_scope();
    let document = read(concat!(
        r"{\rtf1{\stylesheet{\fs30\qc;}{\s2\sbasedon3 Cycle A;}",
        r"{\s3\sbasedon2 Cycle B;}}\s0 body}"
    ));
    assert_eq!(document.text(), "body");
    assert_eq!(
        document.paragraphs[0].format.text_alignment,
        Some(TextAlignment::Center)
    );
    assert_eq!(
        document.paragraphs[0].inlines[0].format.font_size,
        Some(20.0)
    );
    assert_eq!(read(r"{\rtf1\s0 ordinary}").text(), "ordinary");
}

#[test]
fn rich_rtf_style_selected_invalid_graphs_fail_without_silent_format_loss() {
    let _scope = Dispatcher::unit_test_scope();
    for (source, expected) in [
        (
            r"{\rtf1{\stylesheet{\s1\sbasedon1 Self;}}\s1 x}",
            RtfError::InvalidParameter("cyclic style inheritance"),
        ),
        (
            r"{\rtf1{\stylesheet{\s1\sbasedon2 A;}{\s2\sbasedon1 B;}}\s1 x}",
            RtfError::InvalidParameter("cyclic style inheritance"),
        ),
        (
            r"{\rtf1{\stylesheet{\s1\sbasedon7 Missing;}}\s1 x}",
            RtfError::InvalidParameter("style reference"),
        ),
        (
            r"{\rtf1\cs7 x}",
            RtfError::InvalidParameter("style reference"),
        ),
        (
            r"{\rtf1{\stylesheet{\s1\b Paragraph;}}\cs1 x}",
            RtfError::InvalidParameter("style kind"),
        ),
        (
            r"{\rtf1{\stylesheet{\*\cs1\qc Bad;}}\cs1 x}",
            RtfError::UnsupportedFeature("paragraph properties in character styles"),
        ),
        (
            r"{\rtf1{\stylesheet{\*\ts1\tsrowd Table;}}\ts1 x}",
            RtfError::UnsupportedFeature("section or table styles"),
        ),
        (
            r"{\rtf1{\stylesheet{\*\ds1 Section;}}\ds1 x}",
            RtfError::UnsupportedFeature("section or table styles"),
        ),
    ] {
        assert_eq!(RtfReader::read_str(source), Err(expected), "{source}");
    }
}

#[test]
fn rich_rtf_style_duplicate_and_out_of_range_handles_are_rejected() {
    let _scope = Dispatcher::unit_test_scope();
    for (source, expected) in [
        (
            r"{\rtf1{\stylesheet{\s1 A;}{\s1 B;}}x}",
            RtfError::InvalidParameter("duplicate style handle"),
        ),
        (
            r"{\rtf1{\stylesheet{\s1\s2 Both;}}x}",
            RtfError::InvalidParameter("multiple style handles"),
        ),
        (
            r"{\rtf1{\stylesheet{\s65536 Large;}}x}",
            RtfError::InvalidParameter("style handle"),
        ),
        (
            r"{\rtf1{\stylesheet{\s-32769 Negative;}}x}",
            RtfError::InvalidParameter("style handle"),
        ),
        (r"{\rtf1\s x}", RtfError::InvalidParameter("style handle")),
    ] {
        assert_eq!(RtfReader::read_str(source), Err(expected), "{source}");
    }
}

#[test]
fn rich_rtf_style_expansion_and_inheritance_have_independent_work_bounds() {
    let _scope = Dispatcher::unit_test_scope();
    let mut deep = String::from(r"{\rtf1{\stylesheet{\s0\b Normal;}");
    for id in 1..20 {
        deep.push_str(&format!(r"{{\s{id}\sbasedon{}\i Style;}}", id - 1));
    }
    deep.push_str(r"}\s19 text}");
    assert_eq!(
        RtfReader::read_with_limits(
            deep.as_bytes(),
            RtfLimits {
                max_depth: 8,
                ..Default::default()
            }
        ),
        Err(RtfError::LimitExceeded("style inheritance"))
    );

    let mut amplified = String::from(r"{\rtf1{\stylesheet{\s1 ");
    for _ in 0..40 {
        amplified.push_str(r"\b\i");
    }
    amplified.push_str(" Style;}}");
    for _ in 0..30 {
        amplified.push_str(r"\s1 x");
    }
    amplified.push('}');
    assert_eq!(
        RtfReader::read_with_limits(
            amplified.as_bytes(),
            RtfLimits {
                max_tokens: 300,
                ..Default::default()
            }
        ),
        Err(RtfError::LimitExceeded("style expansion"))
    );
}

#[test]
fn rich_rtf_style_unsupported_metrics_do_not_bypass_direct_property_validation() {
    let _scope = Dispatcher::unit_test_scope();
    for source in [
        r"{\rtf1{\stylesheet{\s1\fs0 Invalid;}}\s1 x}",
        r"{\rtf1{\stylesheet{\s1\charscalex80 Unsupported;}}\s1 x}",
        r"{\rtf1{\stylesheet{\s1\f9 Missing font;}}\s1 x}",
        r"{\rtf1{\stylesheet{\s1\sl-240\slmult1 Invalid spacing;}}\s1 x}",
    ] {
        assert!(RtfReader::read_str(source).is_err(), "{source}");
    }
}

#[test]
fn rich_rtf_style_resolved_formats_survive_canonical_rtf_round_trip() {
    let _scope = Dispatcher::unit_test_scope();
    let first = read(concat!(
        r"{\rtf1{\stylesheet{\s0\fs24 Normal;}{\s1\sbasedon0\b\qc Heading;}",
        r"{\*\cs2\additive\i Emphasis;}}\s1 A{\cs2 B}C\par\s0 text}"
    ));
    let second = read(&RtfWriter::write(&first).unwrap());
    let third = read(&RtfWriter::write(&second).unwrap());
    assert_eq!(second.text(), first.text());
    assert_eq!(
        second.paragraphs[0].format.text_alignment,
        first.paragraphs[0].format.text_alignment
    );
    assert_eq!(
        second.paragraphs[0].inlines[1].format.font_style,
        Some(FontStyle::Italic)
    );
    assert_eq!(second, third);
}

#[test]
fn rich_rtf_style_import_rejects_invalid_graph_before_mutation_and_undoes_once() {
    let _scope = Dispatcher::unit_test_scope();
    let control = RichEditBox::new();
    control.set_text("before").unwrap();
    let text = control.text_document();
    let original = control.document().snapshot();
    let invalid = r"{\rtf1{\stylesheet{\s1\sbasedon1 Cycle;}}\s1 replacement}";
    assert!(text
        .set_text_with_options(TextSetOptions::FORMAT_RTF, invalid)
        .is_err());
    assert_eq!(control.document().snapshot(), original);
    text.set_text_with_options(
        TextSetOptions::FORMAT_RTF,
        r"{\rtf1{\stylesheet{\s1\b Strong;}}\s1 replacement}",
    )
    .unwrap();
    assert_eq!(control.text(), "replacement");
    assert_eq!(
        control.document().snapshot().paragraphs[0].inlines[0]
            .format
            .font_weight,
        Some(FontWeight::Bold)
    );
    assert!(text.undo().unwrap());
    assert_eq!(control.document().snapshot(), original);
    assert!(text.redo().unwrap());
    assert_eq!(control.text(), "replacement");
}
