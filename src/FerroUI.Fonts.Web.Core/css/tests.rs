use super::*;

fn only(text: &str) -> FontFaceDescriptors {
    let mut faces = parse_stylesheet(text);
    assert_eq!(faces.len(), 1, "{text}");
    faces.remove(0)
}

#[test]
fn a_complete_rule() {
    let face = only(
        r#"/* a comment */ @font-face {
            font-family: "Open Sans";
            src: local("Open Sans"), url(/f/os.woff2) format("woff2") tech(variations, color-COLRv1),
                 url('/f/os.woff') format(woff);
            font-style: oblique 10deg 20deg;
            font-weight: 300 800;
            font-stretch: 75% 125%;
            unicode-range: U+0-FF, U+131, U+4??;
            font-display: swap;
            font-feature-settings: "smcp", "liga" off, "cv01" 3;
            font-variation-settings: "wght" 450, "XHGT" 0.7;
            font-named-instance: "Light";
            font-language-override: "TRK";
            ascent-override: 90%;
            descent-override: 20%;
            line-gap-override: 0%;
            size-adjust: 105%;
        }"#,
    );
    assert_eq!(face.family, "Open Sans");
    assert_eq!(face.sources.len(), 3);
    assert_eq!(face.sources[0], FontSource::Local { name: "Open Sans".into() });
    let FontSource::Url { url, format, techs } = &face.sources[1] else { panic!("a url") };
    assert_eq!(url, "/f/os.woff2");
    assert_eq!(*format, Some(FontFormat::Woff2));
    assert!(techs.contains(FontTech::Variations) && techs.contains(FontTech::ColorColrV1));
    assert_eq!(face.style, FontStyleDescriptor::Oblique { start: 10.0, end: 20.0 });
    assert_eq!(face.weight, Some(FontWeightRange { start: 300.0, end: 800.0 }));
    assert_eq!(face.width, Some(FontWidthRange { start: 75.0, end: 125.0 }));
    assert!(face.unicode_range.contains(0x41) && face.unicode_range.contains(0x4ff));
    assert!(!face.unicode_range.contains(0x132));
    assert_eq!(face.display, FontDisplay::Swap);
    assert_eq!(face.feature_settings[1], FontFeatureSetting { tag: *b"liga", value: 0 });
    assert_eq!(face.feature_settings[2].value, 3);
    assert_eq!(face.variation_settings[0], FontVariationSetting { tag: *b"wght", value: 450.0 });
    assert_eq!(face.named_instance.as_deref(), Some("Light"));
    assert_eq!(face.language_override.as_deref(), Some("TRK"));
    assert_eq!(face.ascent_override, MetricOverride::Value { first: 0.9, second: None });
    assert_eq!(face.line_gap_override, MetricOverride::Value { first: 0.0, second: None });
    assert!((face.size_adjust - 1.05).abs() < 1e-6);
}

#[test]
fn initial_values() {
    let face = only("@font-face { font-family: A; src: url(a.ttf) }");
    assert_eq!(face.style, FontStyleDescriptor::Auto);
    assert_eq!((face.weight, face.width), (None, None));
    assert_eq!(face.unicode_range.ranges(), &[(0, 0x10FFFF)]);
    assert_eq!(face.display, FontDisplay::Auto);
    assert_eq!(face.size_adjust, 1.0);
}

#[test]
fn a_rule_needs_a_family_and_a_source() {
    assert!(parse_stylesheet("@font-face { src: url(a.ttf) }").is_empty());
    assert!(parse_stylesheet("@font-face { font-family: A }").is_empty());
    assert!(parse_stylesheet("@font-face { font-family: serif; src: url(a.ttf) }").is_empty());
    assert_eq!(parse_font_face_block("src: url(a)"), Err(CssParseError::MissingFamily));
    assert_eq!(parse_font_face_block("font-family: A; src: nope"), Err(CssParseError::MissingSource));
}

#[test]
fn other_rules_are_skipped() {
    let faces = parse_stylesheet(
        r#"@import "x.css"; body { color: red; } @media (min-width: 1px) { a { b: c } }
           @font-face { font-family: One; src: url(1.ttf) } p:hover { x: "}" }
           @font-face { font-family: Two; src: url(2.ttf) }"#,
    );
    let names: Vec<&str> = faces.iter().map(|face| face.family.as_str()).collect();
    assert_eq!(names, ["One", "Two"]);
}

#[test]
fn an_invalid_declaration_keeps_the_previous_value() {
    let face = only(
        "@font-face { font-family: A; src: url(a); font-weight: 500; font-weight: 5000; \
         font-display: never; unknown-thing: 1; size-adjust: -5%; }",
    );
    assert_eq!(face.weight, Some(FontWeightRange { start: 500.0, end: 500.0 }));
    assert_eq!(face.display, FontDisplay::Auto);
    assert_eq!(face.size_adjust, 1.0);
}

#[test]
fn the_last_valid_family_and_src_win() {
    let face = only("@font-face { font-family: A; font-family: 1; src: url(a); src: nope; }");
    assert_eq!(face.family, "A");
    assert!(matches!(&face.sources[0], FontSource::Url { url, .. } if url == "a"));
}

#[test]
fn a_descriptor_name_may_be_escaped() {
    let face = only(r"@font-face { font-f\61mily: A; src: url(a); FONT-DISPLAY: swap }");
    assert_eq!(face.family, "A");
    assert_eq!(face.display, FontDisplay::Swap);
}

#[test]
fn family_names() {
    assert_eq!(parse_family_name("Open   Sans").unwrap(), "Open Sans");
    assert_eq!(parse_family_name("'serif'").unwrap(), "serif");
    assert_eq!(parse_family_name("my serif").unwrap(), "my serif");
    assert_eq!(parse_family_name(r"F\6f o").unwrap(), "Foo");
    for invalid in ["", "serif", "INHERIT", "default", "1a", "a, b", "\"\""] {
        assert!(parse_family_name(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn src_skips_what_it_cannot_parse() {
    let sources = parse_src(
        "url(a) format(nonsense), url(b) format(\"woff2-variations\"), garbage, local(), \
         url(c) tech(unknown), local(My Font), url(d) format(woff) tech(incremental)",
    )
    .unwrap();
    assert_eq!(sources.len(), 3);
    let FontSource::Url { format, techs, .. } = &sources[0] else { panic!("a url") };
    assert_eq!(*format, Some(FontFormat::Woff2));
    assert!(techs.contains(FontTech::Variations));
    assert_eq!(sources[1], FontSource::Local { name: "My Font".into() });
    assert!(parse_src("nope, url(a) format(nope)").is_err());
    assert!(parse_src("").is_err());
}

#[test]
fn src_keeps_commas_of_data_urls() {
    let sources = parse_src("url(data:font/woff2;base64,AAAA) format(woff2), url(\"data:a,b\")").unwrap();
    assert_eq!(sources.len(), 2);
    assert!(matches!(&sources[0], FontSource::Url { url, .. } if url == "data:font/woff2;base64,AAAA"));
    assert!(matches!(&sources[1], FontSource::Url { url, .. } if url == "data:a,b"));
}

#[test]
fn source_support() {
    let variations = FontTechs::NONE.with(FontTech::Variations);
    let sources = parse_src("url(a) format(woff2) tech(variations), url(b) format(svg), url(c) tech(incremental)");
    let sources = sources.unwrap();
    assert!(sources[0].is_supported(variations));
    assert!(!sources[0].is_supported(FontTechs::NONE));
    assert!(!sources[1].is_supported(variations));
    assert!(!sources[2].is_supported(variations));
}

#[test]
fn unicode_ranges() {
    let set = parse_unicode_range("u+26, U+0-7F , U+0025-00FF, U+4??, U+1F600").unwrap();
    assert_eq!(set.ranges(), &[(0, 0xFF), (0x400, 0x4FF), (0x1F600, 0x1F600)]);
    assert!(set.intersects_text("\u{1F600}"));
    assert!(!set.intersects_text("\u{3042}"));
    assert_eq!(parse_unicode_range("U+??????").unwrap_err(), CssParseError::InvalidValue("unicode-range"));
    assert_eq!(parse_unicode_range("U+10????").unwrap().ranges(), &[(0x100000, 0x10FFFF)]);
    assert_eq!(parse_unicode_range("U+/* c */41").unwrap().ranges(), &[(0x41, 0x41)]);
    for invalid in ["", "U+", "41", "U+110000", "U+7F-0", "U+1234567", "U+4?4", "U+0-?", "U+0,", "U+G"] {
        assert!(parse_unicode_range(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn styles() {
    assert_eq!(parse_font_style("ITALIC").unwrap(), FontStyleDescriptor::Italic);
    assert_eq!(parse_font_style("oblique").unwrap(), FontStyleDescriptor::Oblique { start: 14.0, end: 14.0 });
    assert_eq!(
        parse_font_style("oblique 20deg -5deg").unwrap(),
        FontStyleDescriptor::Oblique { start: -5.0, end: 20.0 }
    );
    assert_eq!(parse_font_style("oblique 0").unwrap(), FontStyleDescriptor::Oblique { start: 0.0, end: 0.0 });
    assert_eq!(parse_font_style("oblique 0.25turn").unwrap(), FontStyleDescriptor::Oblique { start: 90.0, end: 90.0 });
    for invalid in ["", "bold", "italic 5deg", "oblique 91deg", "oblique 5", "oblique 1deg 2deg 3deg"] {
        assert!(parse_font_style(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn weights_and_widths() {
    assert_eq!(parse_font_weight("auto").unwrap(), None);
    assert_eq!(parse_font_weight("bold normal").unwrap(), Some(FontWeightRange { start: 400.0, end: 700.0 }));
    assert_eq!(parse_font_weight("1 1000").unwrap(), Some(FontWeightRange { start: 1.0, end: 1000.0 }));
    for invalid in ["", "0", "1001", "bolder", "100 200 300", "50%"] {
        assert!(parse_font_weight(invalid).is_err(), "{invalid}");
    }
    assert_eq!(parse_font_width("auto").unwrap(), None);
    assert_eq!(parse_font_width("condensed expanded").unwrap(), Some(FontWidthRange { start: 75.0, end: 125.0 }));
    assert_eq!(parse_font_width("150% 50%").unwrap(), Some(FontWidthRange { start: 50.0, end: 150.0 }));
    for invalid in ["", "-5%", "100", "wide", "1% 2% 3%"] {
        assert!(parse_font_width(invalid).is_err(), "{invalid}");
    }
}

#[test]
fn settings() {
    assert!(parse_feature_settings("normal").unwrap().is_empty());
    assert!(parse_variation_settings("NORMAL").unwrap().is_empty());
    for invalid in ["", "liga", "\"lig\"", "\"ligaa\"", "\"liga\" -1", "\"liga\" 1.5", "\"liga\",", "\"l\u{e9}ga\""] {
        assert!(parse_feature_settings(invalid).is_err(), "{invalid}");
    }
    for invalid in ["", "\"wght\"", "wght 400", "\"wgh\" 1", "\"wght\" 1 2", "\"wght\" 1,"] {
        assert!(parse_variation_settings(invalid).is_err(), "{invalid}");
    }
    assert_eq!(parse_variation_settings("\"wght\" -1.5e2").unwrap()[0].value, -150.0);
}

#[test]
fn metric_overrides() {
    assert_eq!(parse_metric_override("normal").unwrap(), MetricOverride::Normal);
    assert_eq!(parse_metric_override("50% 25%").unwrap(), MetricOverride::Value { first: 0.5, second: Some(0.25) });
    for invalid in ["", "-1%", "10", "1% 2% 3%", "auto"] {
        assert!(parse_metric_override(invalid).is_err(), "{invalid}");
    }
    assert_eq!(parse_size_adjust("0%").unwrap(), 0.0);
    assert!(parse_size_adjust("normal").is_err());
}

#[test]
fn named_instance_and_language() {
    assert_eq!(parse_named_instance("auto").unwrap(), None);
    assert!(parse_named_instance("Light").is_err());
    assert_eq!(parse_language_override("normal").unwrap(), None);
    assert!(parse_language_override("\"TOOLONG\"").is_err());
}

#[test]
fn strings_with_escapes_and_bad_tokens() {
    let face = only("@font-face { font-family: \"A\\\"B\\42\"; src: url(\"a\\29.ttf\"); }");
    assert_eq!(face.family, "A\"BB");
    assert!(matches!(&face.sources[0], FontSource::Url { url, .. } if url == "a).ttf"));
    // A string broken by a newline ends there, and drops that declaration only.
    let face = only("@font-face { font-family: A; font-named-instance: \"x\n; src: url(a); }");
    assert_eq!(face.named_instance, None);
    // The quote after the break opens a new string, which swallows the rest of the rule, as
    // the tokenizer of CSS Syntax requires.
    assert!(parse_stylesheet("@font-face { font-family: A; font-named-instance: \"x\ny\"; src: url(a); }").is_empty());
}

#[test]
fn unterminated_input_does_not_panic() {
    for text in [
        "@font-face",
        "@font-face {",
        "@font-face { font-family: \"A",
        "@font-face { src: url(",
        "@font-face { src: url(\"a",
        "/* comment",
        "@font-face { font-family: A; src: url(a) format(",
        "@font-face { font-family: A; src: local(",
        "\\",
        "@\\",
        "url(\\",
        "}}}}))))",
        "@font-face { font-family: \u{e9}\u{1F600}; src: url(\u{1F600}); }",
    ] {
        let _ = parse_stylesheet(text);
        let _ = parse_src(text);
        let _ = parse_unicode_range(text);
        let _ = parse_font_style(text);
    }
}

#[test]
fn every_prefix_of_a_rule_parses_without_a_panic() {
    let rule = "@font-face{font-family:\"\u{e9}x\";src:url(a\\29) format(\"woff2\") tech(variations),local(B C);\
                unicode-range:U+0-FF,U+4??;font-weight:100 900;font-style:oblique 1deg 2deg;}";
    for end in (0..=rule.len()).filter(|end| rule.is_char_boundary(*end)) {
        let _ = parse_stylesheet(&rule[..end]);
        let _ = parse_font_face_block(&rule[..end]);
    }
}
