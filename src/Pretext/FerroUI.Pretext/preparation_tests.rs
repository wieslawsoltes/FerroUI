//! Ports of all 49 source preparation parity tests.

use std::rc::Rc;

use crate::test_support::{engine, FONT, LINE_HEIGHT, PRE_WRAP};
use crate::*;

fn prepare(text: &str) -> Rc<PreparedTextWithSegments> {
    engine()
        .prepare_with_segments(text, FONT, PrepareOptions::default())
        .unwrap()
}

fn assert_segments(text: &str, expected: &[&str]) {
    let prepared = prepare(text);
    assert_eq!(
        prepared
            .segments()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        expected
    );
}

fn line_texts(lines: &LayoutLinesResult) -> Vec<&str> {
    lines.lines.iter().map(|line| line.text.as_str()).collect()
}

macro_rules! segment_test {
    ($name:ident, $text:expr, $expected:expr) => {
        #[test]
        fn $name() {
            assert_segments($text, $expected);
        }
    };
}

#[test]
fn prepare_whitespace_only_input_stays_empty() {
    let prepared = engine()
        .prepare("  \t\n  ", FONT, PrepareOptions::default())
        .unwrap();
    assert_eq!(
        PretextLayout::layout(&prepared, 200.0, LINE_HEIGHT),
        LayoutResult {
            line_count: 0,
            height: 0.0
        }
    );
}

segment_test!(
    prepare_collapses_ordinary_whitespace_runs_and_trims_edges,
    "  Hello\t \n  World  ",
    &["Hello", " ", "World"]
);

#[test]
fn prepare_pre_wrap_keeps_ordinary_spaces() {
    let prepared = engine()
        .prepare_with_segments("  Hello   World  ", FONT, PRE_WRAP)
        .unwrap();
    assert_eq!(prepared.segments(), &["  ", "Hello", "   ", "World", "  "]);
    assert_eq!(
        prepared.kinds(),
        &[
            SegmentBreakKind::PreservedSpace,
            SegmentBreakKind::Text,
            SegmentBreakKind::PreservedSpace,
            SegmentBreakKind::Text,
            SegmentBreakKind::PreservedSpace
        ]
    );
}

#[test]
fn prepare_pre_wrap_keeps_hard_breaks_as_explicit_segments() {
    let prepared = engine()
        .prepare_with_segments("Hello\nWorld", FONT, PRE_WRAP)
        .unwrap();
    assert_eq!(prepared.segments(), &["Hello", "\n", "World"]);
    assert_eq!(
        prepared.kinds(),
        &[
            SegmentBreakKind::Text,
            SegmentBreakKind::HardBreak,
            SegmentBreakKind::Text
        ]
    );
}

#[test]
fn prepare_pre_wrap_normalizes_cr_lf() {
    let prepared = engine()
        .prepare_with_segments("Hello\r\nWorld", FONT, PRE_WRAP)
        .unwrap();
    assert_eq!(prepared.segments(), &["Hello", "\n", "World"]);
    assert_eq!(
        prepared.kinds(),
        &[
            SegmentBreakKind::Text,
            SegmentBreakKind::HardBreak,
            SegmentBreakKind::Text
        ]
    );
}

#[test]
fn prepare_pre_wrap_keeps_tabs_as_explicit_segments() {
    let prepared = engine()
        .prepare_with_segments("Hello\tWorld", FONT, PRE_WRAP)
        .unwrap();
    assert_eq!(prepared.segments(), &["Hello", "\t", "World"]);
    assert_eq!(
        prepared.kinds(),
        &[
            SegmentBreakKind::Text,
            SegmentBreakKind::Tab,
            SegmentBreakKind::Text
        ]
    );
}

#[test]
fn prepare_keeps_nbsp_as_glue() {
    let prepared = prepare("Hello\u{00a0}world");
    assert_eq!(prepared.segments(), &["Hello\u{00a0}world"]);
    assert_eq!(prepared.kinds(), &[SegmentBreakKind::Text]);
}

#[test]
fn prepare_keeps_standalone_nbsp_visible() {
    let prepared = prepare("\u{00a0}");
    assert_eq!(prepared.segments(), &["\u{00a0}"]);
    assert_eq!(
        PretextLayout::layout(&prepared, 200.0, LINE_HEIGHT),
        LayoutResult {
            line_count: 1,
            height: LINE_HEIGHT
        }
    );
}

#[test]
fn prepare_pre_wrap_keeps_whitespace_only_input_visible() {
    let prepared = engine().prepare("   ", FONT, PRE_WRAP).unwrap();
    assert_eq!(
        PretextLayout::layout(&prepared, 200.0, LINE_HEIGHT),
        LayoutResult {
            line_count: 1,
            height: LINE_HEIGHT
        }
    );
}

#[test]
fn prepare_keeps_nnbsp_as_glue() {
    let prepared = prepare("10\u{202f}000");
    assert_eq!(prepared.segments(), &["10\u{202f}000"]);
    assert_eq!(prepared.kinds(), &[SegmentBreakKind::Text]);
}

#[test]
fn prepare_keeps_word_joiners_as_glue() {
    let prepared = prepare("foo\u{2060}bar");
    assert_eq!(prepared.segments(), &["foo\u{2060}bar"]);
    assert_eq!(prepared.kinds(), &[SegmentBreakKind::Text]);
}

#[test]
fn prepare_treats_zero_width_spaces_as_explicit_breaks() {
    let prepared = prepare("alpha\u{200b}beta");
    assert_eq!(prepared.segments(), &["alpha", "\u{200b}", "beta"]);
    assert_eq!(
        prepared.kinds(),
        &[
            SegmentBreakKind::Text,
            SegmentBreakKind::ZeroWidthBreak,
            SegmentBreakKind::Text
        ]
    );
    assert_eq!(
        PretextLayout::layout(&prepared, prepared.widths()[0] + 0.1, LINE_HEIGHT).line_count,
        2
    );
}

#[test]
fn prepare_treats_soft_hyphens_as_discretionary_breaks() {
    let prepared = prepare("trans\u{00ad}atlantic");
    assert_eq!(prepared.segments(), &["trans", "\u{00ad}", "atlantic"]);
    assert_eq!(
        prepared.kinds(),
        &[
            SegmentBreakKind::Text,
            SegmentBreakKind::SoftHyphen,
            SegmentBreakKind::Text
        ]
    );
    let wide = PretextLayout::layout_with_lines(&prepared, 200.0, LINE_HEIGHT);
    assert_eq!(wide.line_count, 1);
    assert_eq!(line_texts(&wide), ["transatlantic"]);

    let prefixed = prepare("foo trans\u{00ad}atlantic");
    let soft_break_width = (prefixed.widths()[0]
        + prefixed.widths()[1]
        + prefixed.widths()[2]
        + prefixed.discretionary_hyphen_width())
    .max(prefixed.widths()[4])
        + 0.1;
    let narrow = PretextLayout::layout_with_lines(&prefixed, soft_break_width, LINE_HEIGHT);
    assert_eq!(narrow.line_count, 2);
    assert_eq!(line_texts(&narrow), ["foo trans-", "atlantic"]);
    assert_eq!(
        PretextLayout::layout(&prefixed, soft_break_width, LINE_HEIGHT).line_count,
        narrow.line_count
    );

    let continued_width = prefixed.widths()[0]
        + prefixed.widths()[1]
        + prefixed.widths()[2]
        + prefixed.breakable_widths()[4].as_ref().unwrap()[0]
        + prefixed.discretionary_hyphen_width()
        + 0.1;
    let continued = PretextLayout::layout_with_lines(&prefixed, continued_width, LINE_HEIGHT);
    assert_eq!(line_texts(&continued), ["foo trans-a", "tlantic"]);
    assert_eq!(
        PretextLayout::layout(&prefixed, continued_width, LINE_HEIGHT).line_count,
        continued.line_count
    );
}

segment_test!(
    prepare_keeps_closing_punctuation_attached_to_preceding_word,
    "hello.",
    &["hello."]
);
segment_test!(
    prepare_keeps_arabic_punctuation_attached_to_preceding_word,
    "مرحبا، عالم؟",
    &["مرحبا،", " ", "عالم؟"]
);
segment_test!(
    prepare_keeps_arabic_punctuation_plus_mark_clusters_attached,
    "وحوارى بكشء،ٍ من قولهم",
    &["وحوارى", " ", "بكشء،ٍ", " ", "من", " ", "قولهم"]
);
segment_test!(
    prepare_keeps_arabic_no_space_punctuation_clusters_together,
    "فيقول:وعليك السلام",
    &["فيقول:وعليك", " ", "السلام"]
);
segment_test!(
    prepare_keeps_arabic_comma_followed_text_together,
    "همزةٌ،ما كان",
    &["همزةٌ،ما", " ", "كان"]
);
segment_test!(
    prepare_keeps_leading_arabic_combining_marks_with_following_word,
    "كل ِّواحدةٍ",
    &["كل", " ", "ِّواحدةٍ"]
);
segment_test!(
    prepare_keeps_devanagari_danda_attached_to_preceding_word,
    "नमस्ते। दुनिया॥",
    &["नमस्ते।", " ", "दुनिया॥"]
);

#[test]
fn prepare_keeps_myanmar_punctuation_attached_to_preceding_word() {
    let prepared = prepare("ဖြစ်သည်။ နောက်တစ်ခု၊ ကိုက်ချီ၍ ယုံကြည်မိကြ၏။");
    assert_eq!(
        &prepared.segments()[..7],
        &["ဖြစ်သည်။", " ", "နောက်တစ်ခု၊", " ", "ကိုက်", "ချီ၍", " "]
    );
    assert_eq!(prepared.segments().last().unwrap(), "ကြ၏။");
}

segment_test!(
    prepare_keeps_myanmar_possessive_marker_attached_to_following_word,
    "ကျွန်ုပ်၏လက်မဖြင့်",
    &["ကျွန်ုပ်၏လက်မ", "ဖြင့်"]
);
segment_test!(
    prepare_keeps_opening_quotes_attached_to_following_word,
    "“Whenever",
    &["“Whenever"]
);
segment_test!(
    prepare_keeps_apostrophe_led_elisions_attached_to_following_word,
    "“Take ’em downstairs",
    &["“Take", " ", "’em", " ", "downstairs"]
);
segment_test!(
    prepare_keeps_stacked_opening_quotes_attached_to_following_word,
    "invented, “‘George B. Wilson",
    &["invented,", " ", "“‘George", " ", "B.", " ", "Wilson"]
);
segment_test!(
    prepare_treats_ascii_quotes_as_opening_and_closing_glue_by_context,
    "said \"hello\" there",
    &["said", " ", "\"hello\"", " ", "there"]
);
segment_test!(
    prepare_treats_escaped_ascii_quote_clusters_as_glue_by_context,
    "say \\\"hello\\\" there",
    &["say", " ", "\\\"hello\\\"", " ", "there"]
);
segment_test!(
    prepare_keeps_escaped_quote_clusters_attached_through_opening_punctuation,
    "((\\\"\\\"word",
    &["((\\\"\\\"word"]
);
segment_test!(
    prepare_keeps_url_like_runs_together,
    "see https://example.com/reports/q3?lang=ar&mode=full now",
    &[
        "see",
        " ",
        "https://example.com/reports/q3?",
        "lang=ar&mode=full",
        " ",
        "now"
    ]
);
segment_test!(
    prepare_keeps_no_space_ascii_punctuation_chains_together,
    "foo;bar foo:bar foo,bar as;lkdfjals;k",
    &[
        "foo;bar",
        " ",
        "foo:bar",
        " ",
        "foo,bar",
        " ",
        "as;lkdfjals;k"
    ]
);
segment_test!(
    prepare_keeps_numeric_time_ranges_together,
    "window 7:00-9:00 only",
    &["window", " ", "7:00-", "9:00", " ", "only"]
);
segment_test!(
    prepare_splits_hyphenated_numeric_identifiers_at_preferred_boundaries,
    "SSN 420-69-8008 filed",
    &["SSN", " ", "420-", "69-", "8008", " ", "filed"]
);
segment_test!(
    prepare_keeps_unicode_digit_numeric_expressions_together,
    "यह २४×७ सपोर्ट है",
    &["यह", " ", "२४×७", " ", "सपोर्ट", " ", "है"]
);
segment_test!(
    prepare_does_not_attach_opening_punctuation_to_following_whitespace,
    "“ hello",
    &["“", " ", "hello"]
);
segment_test!(
    prepare_keeps_japanese_iteration_marks_attached_to_preceding_kana,
    "棄てゝ行く",
    &["棄", "てゝ", "行", "く"]
);
segment_test!(
    prepare_carries_trailing_cjk_opening_punctuation_forward_across_segment_boundaries,
    "作者はさつき、「下人",
    &["作", "者", "は", "さ", "つ", "き、", "「下", "人"]
);
segment_test!(
    prepare_keeps_em_dashes_breakable,
    "universe—so",
    &["universe", "—", "so"]
);
segment_test!(
    prepare_coalesces_repeated_punctuation_runs_into_single_segment,
    "=== heading ===",
    &["===", " ", "heading", " ", "==="]
);

#[test]
fn prepare_keeps_long_repeated_punctuation_runs_coalesced() {
    let text = "(".repeat(256);
    assert_segments(&text, &[&text]);
}

segment_test!(
    prepare_keeps_repeated_punctuation_runs_attachable_to_trailing_closing_punctuation,
    "((()",
    &["((()"]
);

#[test]
fn prepare_applies_cjk_and_hangul_punctuation_attachment_rules() {
    assert_segments("中文，测试。", &["中", "文，", "测", "试。"]);
    assert_eq!(prepare("테스트입니다.").segments().last().unwrap(), "다.");
}

segment_test!(
    prepare_keeps_non_cjk_glue_connected_runs_intact_before_cjk_text,
    "foo\u{00a0}世界",
    &["foo\u{00a0}", "世", "界"]
);

#[test]
fn prepare_keep_all_keeps_cjk_leading_no_space_runs_cohesive() {
    let layout = engine();
    let options = PrepareOptions {
        word_break: WordBreakMode::KeepAll,
        ..PrepareOptions::default()
    };
    let check = |text: &str, expected: &[&str]| {
        let prepared = layout.prepare_with_segments(text, FONT, options).unwrap();
        assert_eq!(
            prepared
                .segments()
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            expected
        );
    };
    check("中文，测试。", &["中文，", "测试。"]);
    check("한국어테스트", &["한국어테스트"]);
    let text = "漢".repeat(256);
    check(&text, &[&text]);
    for text in ["日本語foo-bar", "日本語foo.bar", "日本語foo—bar"] {
        check(text, &[text]);
    }
    check("foo-bar日本語", &["foo-", "bar", "日本語"]);
    check("foo\u{00a0}世界", &["foo\u{00a0}", "世界"]);
}

#[test]
fn prepare_treats_astral_cjk_ideographs_as_break_units() {
    assert_segments("𠀀𠀁", &["𠀀", "𠀁"]);
    assert_segments("𠀀。", &["𠀀。"]);
    for sample in ["𠀀", "\u{2ebf0}", "\u{31350}", "\u{323b0}"] {
        assert_segments(&format!("{sample}{sample}"), &[sample, sample]);
        let with_stop = format!("{sample}。");
        assert_segments(&with_stop, &[&with_stop]);
    }
}

#[test]
fn prepare_and_prepare_with_segments_agree_on_layout_behavior() {
    let layout = engine();
    let plain = layout
        .prepare("Alpha beta gamma", FONT, PrepareOptions::default())
        .unwrap();
    let rich = layout
        .prepare_with_segments("Alpha beta gamma", FONT, PrepareOptions::default())
        .unwrap();
    for width in [40.0, 80.0, 200.0] {
        assert_eq!(
            PretextLayout::layout(&plain, width, LINE_HEIGHT),
            PretextLayout::layout(&rich, width, LINE_HEIGHT)
        );
    }
}

#[test]
fn prepare_locale_can_be_reset_without_disturbing_later_prepares() {
    let layout = engine();
    layout.set_locale(Some("th"));
    let thai = layout
        .prepare("ภาษาไทยภาษาไทย", FONT, PrepareOptions::default())
        .unwrap();
    assert!(PretextLayout::layout(&thai, 80.0, LINE_HEIGHT).line_count > 0);
    layout.set_locale(None);
    let latin = layout
        .prepare("hello world", FONT, PrepareOptions::default())
        .unwrap();
    assert_eq!(
        PretextLayout::layout(&latin, 200.0, LINE_HEIGHT),
        LayoutResult {
            line_count: 1,
            height: LINE_HEIGHT
        }
    );
}

#[test]
fn prepare_thai_locale_preserves_text_and_uses_locale_aware_word_segmentation() {
    let text = "ภาษาไทยภาษาไทย";
    let layout = engine();
    layout.set_locale(Some("th"));
    let prepared = layout
        .prepare_with_segments(text, FONT, PrepareOptions::default())
        .unwrap();
    assert!(!prepared.segments().is_empty());
    assert!(prepared
        .segments()
        .iter()
        .all(|segment| !segment.is_empty()));
    assert_eq!(prepared.segments().concat(), text);
    // The source permits its native ICU backend to be unavailable. Compiled
    // dictionaries provide this feature on every target of this port.
    assert!(prepared.segments().len() > 1);
    assert!(prepared
        .segments()
        .iter()
        .any(|segment| segment.len() < text.len()));
}

#[test]
fn prepare_pure_ltr_text_skips_rich_bidi_metadata() {
    assert!(prepare("hello world").segment_levels().is_none());
}

#[test]
fn prepare_rich_bidi_metadata_uses_first_strong_character_for_paragraph_direction() {
    let check = |text: &str, expected: &[(&str, i8)]| {
        let prepared = prepare(text);
        let levels = prepared.segment_levels().unwrap();
        assert_eq!(levels.len(), prepared.segments().len());
        let actual: Vec<_> = prepared
            .segments()
            .iter()
            .zip(levels)
            .filter(|(text, _)| text.as_str() != " ")
            .map(|(text, &level)| (text.as_str(), level))
            .collect();
        assert_eq!(actual, expected);
    };
    check("one اثنان three", &[("one", 0), ("اثنان", 1), ("three", 0)]);
    check("123 واحد three", &[("123", 2), ("واحد", 1), ("three", 2)]);
    check("𞤀𞤁 abc", &[("𞤀𞤁", 1), ("abc", 2)]);
}

#[test]
fn profile_prepare_uses_the_injected_clock_and_counts_actual_segments() {
    let layout = engine();
    let mut ticks = [10.0, 13.0, 18.5].into_iter();
    let profile = layout
        .profile_prepare("hello world", FONT, PrepareOptions::default(), || {
            ticks.next().unwrap()
        })
        .unwrap();
    assert_eq!(
        profile,
        PrepareProfile {
            analysis_ms: 3.0,
            measure_ms: 5.5,
            total_ms: 8.5,
            analysis_segments: 3,
            prepared_segments: 3,
            breakable_segments: 2,
        }
    );
}

#[test]
fn profile_prepare_rejects_non_monotonic_or_non_finite_clocks() {
    for values in [
        [2.0, 1.0, 3.0],
        [1.0, f64::NAN, 3.0],
        [0.0, 1.0, f64::INFINITY],
    ] {
        let mut ticks = values.into_iter();
        assert!(engine()
            .profile_prepare("text", FONT, PrepareOptions::default(), || ticks
                .next()
                .unwrap())
            .is_err());
    }
}

#[test]
fn preparation_rejects_invalid_backend_advances() {
    for invalid in [-1.0, f64::NAN, f64::INFINITY, f64::MAX] {
        let layout = PretextLayout::new(Rc::new(DelegateTextMeasurerFactory::new(
            move |_: &str, _: &str| invalid,
        )));
        assert!(matches!(
            layout.prepare("text", FONT, PrepareOptions::default()),
            Err(PretextError::Backend(_))
        ));
    }
}

#[test]
fn reentrant_backend_replacement_does_not_repopulate_old_font_cache() {
    use std::cell::Cell;
    let layout = Rc::new(engine());
    let weak = Rc::downgrade(&layout);
    let replaced = Cell::new(false);
    layout.set_text_measurer_factory(Rc::new(DelegateTextMeasurerFactory::new(
        move |text: &str, _: &str| {
            if !replaced.replace(true) {
                weak.upgrade().unwrap().set_text_measurer_factory(Rc::new(
                    DelegateTextMeasurerFactory::new(|text: &str, _: &str| {
                        text.chars().count() as f64 * 2.0
                    }),
                ));
            }
            text.chars().count() as f64
        },
    )));
    let first = layout
        .prepare("hello", FONT, PrepareOptions::default())
        .unwrap();
    let second = layout
        .prepare("hello", FONT, PrepareOptions::default())
        .unwrap();
    assert_eq!(first.widths(), &[5.0]);
    assert_eq!(second.widths(), &[10.0]);
    assert_eq!(
        PretextLayout::layout(&first, 5.0, LINE_HEIGHT).line_count,
        1
    );
}
