use super::*;
use crate::highlighting::{
    HighlightedSection, HighlightingDefinition, HighlightingEngine, HighlightingState,
};
use std::convert::Infallible;
use std::rc::Rc;

fn compiled(xml: &str) -> HighlightingDefinition {
    HighlightingDefinition::load(xml).unwrap()
}

fn highlight(definition: HighlightingDefinition, text: &str) -> Vec<HighlightedSection> {
    HighlightingEngine::new(Rc::new(definition))
        .highlight_line(text, 1, 0, &HighlightingState::default())
        .unwrap()
        .sections
}

fn foreground_at(sections: &[HighlightedSection], offset: usize) -> Option<&str> {
    sections
        .iter()
        .find(|section| section.offset <= offset && offset < section.offset + section.length)
        .and_then(|section| section.color.foreground.as_deref())
}

fn assert_equivalent(left: HighlightingDefinition, right: HighlightingDefinition, lines: &[&str]) {
    let left = HighlightingEngine::new(Rc::new(left));
    let right = HighlightingEngine::new(Rc::new(right));
    let mut left_state = HighlightingState::default();
    let mut right_state = HighlightingState::default();
    let mut offset = 0;
    for (number, text) in lines.iter().enumerate() {
        let l = left
            .highlight_line(text, number + 1, offset, &left_state)
            .unwrap();
        let r = right
            .highlight_line(text, number + 1, offset, &right_state)
            .unwrap();
        assert_eq!(l.sections, r.sections, "line {}: {text}", number + 1);
        assert_eq!(
            left.get_color_stack(&l.end_state).unwrap(),
            right.get_color_stack(&r.end_state).unwrap()
        );
        left_state = l.end_state;
        right_state = r.end_state;
        offset += text.encode_utf16().count() + 1;
    }
}

#[test]
fn tree_preserves_metadata_references_order_and_utf16_positions() {
    let xml = "<SyntaxDefinition name=\"A/B\" extensions=\".a;.b\">\r\n\
        <Property name=\"author\" value=\"😀 &amp; writer\"/><Color name=\"Accent\" foreground=\"Red\" background=\"Blue\" fontFamily=\"Test Font\" fontSize=\"14.5\" fontWeight=\"650\" fontStyle=\"oblique\" underline=\"1\" strikethrough=\"false\" exampleText=\"a &lt; b\"/>\n\
        <RuleSet><Rule color=\"A/B/Accent\">name</Rule></RuleSet></SyntaxDefinition>";
    let syntax = load_xshd(xml).unwrap();
    assert_eq!(syntax.name.as_deref(), Some("A/B"));
    assert_eq!(syntax.extensions, [".a", ".b"]);
    let XshdElement::Property(property) = &syntax.elements[0] else {
        panic!()
    };
    assert_eq!(property.value, "😀 & writer");
    let XshdElement::Color(color) = &syntax.elements[1] else {
        panic!()
    };
    let prefix = xml
        .split("<Color")
        .next()
        .unwrap()
        .rsplit('\n')
        .next()
        .unwrap();
    assert_eq!(
        color.position,
        XshdPosition::new(2, prefix.encode_utf16().count() + 1)
    );
    assert_eq!(color.font_family.as_deref(), Some("Test Font"));
    assert_eq!(color.font_size, Some(14.5));
    assert_eq!(color.font_weight.as_deref(), Some("650"));
    assert_eq!(color.font_style.as_deref(), Some("oblique"));
    assert_eq!(color.underline, Some(true));
    assert_eq!(color.strikethrough, Some(false));
    assert_eq!(color.example_text.as_deref(), Some("a < b"));
    let XshdElement::RuleSet(set) = &syntax.elements[2] else {
        panic!()
    };
    let XshdElement::Rule(rule) = &set.elements[0] else {
        panic!()
    };
    assert_eq!(rule.color, XshdReference::external("A/B", "Accent"));
    assert_eq!(
        syntax.compile().unwrap().properties["author"],
        "😀 & writer"
    );
}

#[test]
fn standard_namespace_prefixes_and_foreign_extensions_are_handled() {
    let xml = format!(
        r#"<s:SyntaxDefinition xmlns:s="{XSHD_NAMESPACE}" xmlns:e="urn:extension"><e:metadata><e:unknown/></e:metadata><s:RuleSet><e:rule/><s:Rule foreground="Red">x</s:Rule></s:RuleSet></s:SyntaxDefinition>"#
    );
    assert_eq!(
        foreground_at(&highlight(compiled(&xml), "x"), 0),
        Some("Red")
    );
    assert!(
        load_xshd("<SyntaxDefinition xmlns=\"urn:wrong\"><RuleSet/></SyntaxDefinition>").is_err()
    );
    assert!(load_xshd("<!DOCTYPE SyntaxDefinition [<!ENTITY value 'bad'>]><SyntaxDefinition><RuleSet/></SyntaxDefinition>").is_err());
}

#[test]
fn parser_rejects_ambiguous_span_members_with_source_locations() {
    for span in [
        r#"<Span begin="x"><Begin>x</Begin></Span>"#,
        r#"<Span begin="x"><End>x</End><End>y</End></Span>"#,
        r#"<Span begin="x" ruleSet="Other"><RuleSet/></Span>"#,
        r#"<Span begin="x"><RuleSet/><RuleSet/></Span>"#,
        r#"<Span begin="x" multiline="maybe"/>"#,
    ] {
        let error = load_xshd(&format!(
            "<SyntaxDefinition>\n<RuleSet>{span}</RuleSet></SyntaxDefinition>"
        ))
        .unwrap_err()
        .to_string();
        assert!(error.contains("line 2"), "{error}");
    }
    for xml in [
        "<SyntaxDefinition><RuleSet name=\"\"/></SyntaxDefinition>",
        "<SyntaxDefinition><RuleSet name=\"a/b\"/></SyntaxDefinition>",
        "<SyntaxDefinition><RuleSet><Unknown/></RuleSet></SyntaxDefinition>",
        "<SyntaxDefinition><RuleSet><Rule><Nested/></Rule></RuleSet></SyntaxDefinition>",
    ] {
        assert!(load_xshd(xml).is_err(), "{xml}");
    }
}

#[test]
fn compiler_validates_duplicate_names_properties_main_sets_and_missing_references() {
    for xml in [
        "<SyntaxDefinition><RuleSet/><RuleSet/></SyntaxDefinition>",
        "<SyntaxDefinition><RuleSet name=\"OnlyNamed\"/></SyntaxDefinition>",
        "<SyntaxDefinition><RuleSet/><RuleSet name=\"A\"/><RuleSet name=\"A\"/></SyntaxDefinition>",
        "<SyntaxDefinition><Color name=\"A\"/><Color name=\"A\"/><RuleSet/></SyntaxDefinition>",
        "<SyntaxDefinition><Property name=\"A\" value=\"1\"/><Property name=\"A\" value=\"2\"/><RuleSet/></SyntaxDefinition>",
        "<SyntaxDefinition><RuleSet><Import ruleSet=\"Missing\"/></RuleSet></SyntaxDefinition>",
        "<SyntaxDefinition><RuleSet><Rule color=\"Missing\">x</Rule></RuleSet></SyntaxDefinition>",
        "<SyntaxDefinition><RuleSet><Keywords/></RuleSet></SyntaxDefinition>",
        "<SyntaxDefinition><RuleSet><Keywords><Word/></Keywords></RuleSet></SyntaxDefinition>",
        "<SyntaxDefinition><RuleSet><Span begin=\"\" end=\"\"/></RuleSet></SyntaxDefinition>",
        "<SyntaxDefinition><RuleSet><Span begin=\"x\" multiline=\"true\"/></RuleSet></SyntaxDefinition>",
    ] { assert!(load_xshd(xml).unwrap().compile().is_err(), "{xml}"); }
}

#[test]
fn imports_preserve_rule_and_span_precedence_at_their_source_position() {
    for import_first in [false, true] {
        for spans in [false, true] {
            let local = if spans {
                r#"<Span begin="x" end="y" foreground="Blue"/>"#
            } else {
                r#"<Rule foreground="Blue">x</Rule>"#
            };
            let imported = if spans {
                r#"<Span begin="x" end="y" foreground="Red"/>"#
            } else {
                r#"<Rule foreground="Red">x</Rule>"#
            };
            let import = r#"<Import ruleSet="Other"/>"#;
            let body = if import_first {
                format!("{import}{local}")
            } else {
                format!("{local}{import}")
            };
            let xml = format!(
                r#"<SyntaxDefinition><RuleSet>{body}</RuleSet><RuleSet name="Other">{imported}</RuleSet></SyntaxDefinition>"#
            );
            let definition = compiled(&xml);
            assert!(definition.rule_sets[definition.main_rule_set]
                .imports
                .is_empty());
            assert_eq!(
                foreground_at(&highlight(definition, "xy"), 0),
                Some(if import_first { "Red" } else { "Blue" })
            );
        }
    }
}

#[test]
fn external_definitions_support_slashes_main_sets_and_forward_colors() {
    let dependency = load_xshd(r#"<SyntaxDefinition name="External/Language"><RuleSet><Rule color="Word">x</Rule></RuleSet><Color name="Word" foreground="Red"/></SyntaxDefinition>"#).unwrap();
    let syntax = load_xshd(r#"<SyntaxDefinition><RuleSet><Import ruleSet="External/Language/"/><Rule color="External/Language/Word">y</Rule></RuleSet></SyntaxDefinition>"#).unwrap();
    let result = highlight(
        syntax.compile_with_dependencies(&[&dependency]).unwrap(),
        "xy",
    );
    assert_eq!(foreground_at(&result, 0), Some("Red"));
    assert_eq!(foreground_at(&result, 1), Some("Red"));
    assert!(syntax.compile().is_err());
    assert!(syntax
        .compile_with_dependencies(&[&dependency, &dependency])
        .is_err());
}

#[test]
fn external_dependency_discovery_visits_delimiters_and_inline_imports() {
    let mut syntax = load_xshd(r#"<SyntaxDefinition><RuleSet><Rule color="Colors/One">x</Rule><Span color="Spans/Body" ruleSet="Embedded/Rules"><Begin color="Edges/Start">\[</Begin><End color="Edges/End">\]</End></Span><Keywords color="Words/Keyword"><Word>if</Word></Keywords></RuleSet></SyntaxDefinition>"#).unwrap();
    syntax.elements.push(XshdElement::Import(XshdImport {
        rule_set: XshdReference::inline(XshdRuleSet {
            elements: vec![XshdElement::Import(XshdImport {
                rule_set: XshdReference::external("Imported", ""),
                ..XshdImport::default()
            })],
            ..XshdRuleSet::default()
        }),
        ..XshdImport::default()
    }));
    assert_eq!(
        syntax
            .external_definitions()
            .into_iter()
            .collect::<Vec<_>>(),
        ["Colors", "Edges", "Embedded", "Imported", "Spans", "Words"]
    );
}

#[test]
fn cyclic_imports_are_errors_but_recursive_spans_remain_valid() {
    for rules in [
        r#"<RuleSet><Import ruleSet=""/></RuleSet>"#,
        r#"<RuleSet><Import ruleSet="A"/></RuleSet><RuleSet name="A"><Import ruleSet="B"/></RuleSet><RuleSet name="B"><Import ruleSet="A"/></RuleSet>"#,
    ] {
        let error = load_xshd(&format!("<SyntaxDefinition>{rules}</SyntaxDefinition>"))
            .unwrap()
            .compile()
            .unwrap_err()
            .to_string();
        assert!(error.contains("cyclic"), "{error}");
    }
    let definition = compiled(
        r#"<SyntaxDefinition><RuleSet><Span begin="\(" end="\)" ruleSet="" foreground="Red" multiline="true"/></RuleSet></SyntaxDefinition>"#,
    );
    let engine = HighlightingEngine::new(Rc::new(definition));
    let line = engine
        .highlight_line("((x))", 1, 0, &HighlightingState::default())
        .unwrap();
    assert_eq!(line.end_state, HighlightingState::default());
    assert_eq!(foreground_at(&line.sections, 2), Some("Red"));
}

#[test]
fn case_inheritance_obeys_explicit_overrides_and_first_translation_context() {
    let xml = r#"<SyntaxDefinition><RuleSet ignoreCase="true"><Import ruleSet="Inherited"/><Span begin="\[" end="\]"><RuleSet ignoreCase="false"><Rule foreground="Blue">x</Rule></RuleSet></Span></RuleSet><RuleSet name="Inherited"><Rule foreground="Red">x</Rule></RuleSet></SyntaxDefinition>"#;
    let sections = highlight(compiled(xml), "X [X x]");
    assert_eq!(foreground_at(&sections, 0), Some("Red"));
    assert_eq!(foreground_at(&sections, 3), None);
    assert_eq!(foreground_at(&sections, 5), Some("Blue"));
    let before = r#"<SyntaxDefinition><RuleSet name="Inherited"><Rule foreground="Red">x</Rule></RuleSet><RuleSet ignoreCase="true"><Import ruleSet="Inherited"/></RuleSet></SyntaxDefinition>"#;
    assert!(highlight(compiled(before), "X").is_empty());
}

#[test]
fn keyword_groups_handle_prefixes_punctuation_and_empty_groups() {
    let definition = compiled(
        r#"<SyntaxDefinition><RuleSet><Keywords foreground="Red"><Word>in</Word><Word>int</Word><Word>integer</Word></Keywords><Keywords foreground="Blue"><Word>.maxstack</Word><Word>_x</Word></Keywords></RuleSet></SyntaxDefinition>"#,
    );
    let sections = highlight(definition, "integer int in inside .maxstack prefix_x");
    assert_eq!(foreground_at(&sections, 0), Some("Red"));
    assert_eq!(foreground_at(&sections, 8), Some("Red"));
    assert_eq!(foreground_at(&sections, 15), None);
    assert_eq!(foreground_at(&sections, 22), Some("Blue"));
    assert_eq!(foreground_at(&sections, 38), Some("Blue"));
}

#[test]
fn writer_preserves_all_color_properties_and_xml_sensitive_text() {
    let xml = r##"<SyntaxDefinition name="Name &amp; &quot;quoted&quot;" extensions=".a;.b"><Property name="description" value="a&#xD;&#xA;&#x9;&lt;b&gt;&amp;😀"/><Color name="Text" foreground="#123456" background="Transparent" fontFamily="Fira Code" fontSize="12.25" fontWeight="semibold" fontStyle="oblique" underline="false" strikethrough="true" exampleText="&lt; &amp; &quot;"/><RuleSet><Rule color="Text">x</Rule></RuleSet></SyntaxDefinition>"##;
    let syntax = load_xshd(xml).unwrap();
    let saved = syntax.to_xml().unwrap();
    assert!(saved.contains("strikethrough=\"true\""));
    assert!(saved.contains("fontFamily=\"Fira Code\""));
    let restored = load_xshd(&saved).unwrap();
    assert_eq!(restored.name, syntax.name);
    let XshdElement::Property(left) = &syntax.elements[0] else {
        panic!()
    };
    let XshdElement::Property(right) = &restored.elements[0] else {
        panic!()
    };
    assert_eq!(left.value, right.value);
    let XshdElement::Color(mut left) = syntax.elements[1].clone() else {
        panic!()
    };
    let XshdElement::Color(mut right) = restored.elements[1].clone() else {
        panic!()
    };
    left.position = XshdPosition::default();
    right.position = XshdPosition::default();
    assert_eq!(left, right);
    assert_eq!(restored.to_xml().unwrap(), saved);
}

#[test]
fn default_regex_mode_and_colored_delimiters_survive_serialization() {
    let red = XshdReference::inline(XshdColor {
        foreground: Some("Red".into()),
        ..XshdColor::default()
    });
    let blue = XshdReference::inline(XshdColor {
        foreground: Some("Blue".into()),
        ..XshdColor::default()
    });
    let syntax = XshdSyntaxDefinition {
        elements: vec![XshdElement::RuleSet(XshdRuleSet {
            elements: vec![
                XshdElement::Rule(XshdRule {
                    regex: Some("a # b".into()),
                    color: red.clone(),
                    ..XshdRule::default()
                }),
                XshdElement::Span(XshdSpan {
                    begin: Some("< #".into()),
                    end: Some("# >".into()),
                    begin_color: blue.clone(),
                    end_color: blue,
                    span_color: red,
                    ..XshdSpan::default()
                }),
            ],
            ..XshdRuleSet::default()
        })],
        ..XshdSyntaxDefinition::default()
    };
    let saved = syntax.to_xml().unwrap();
    assert!(saved.contains("(?-x)"));
    let restored = load_xshd(&saved).unwrap();
    assert_equivalent(
        syntax.compile().unwrap(),
        restored.compile().unwrap(),
        &["a # b", "a b", "< #value# > outside"],
    );
    let sections = highlight(restored.compile().unwrap(), "< #value# > outside");
    assert_eq!(foreground_at(&sections, 0), Some("Blue"));
    assert_eq!(foreground_at(&sections, 4), Some("Red"));
    assert_eq!(foreground_at(&sections, 8), Some("Blue"));
    assert_eq!(foreground_at(&sections, 12), None);
}

#[test]
fn end_patterns_with_explicit_whitespace_options_and_trailing_comments_work() {
    for end in ["x # end comment", "(?-x)x", "(?x)x # end comment"] {
        let xml = format!(
            r#"<SyntaxDefinition><RuleSet><Span begin="&lt;" foreground="Red"><End>{end}</End></Span></RuleSet></SyntaxDefinition>"#
        );
        let syntax = load_xshd(&xml).unwrap();
        let sections = highlight(syntax.compile().unwrap(), "<ax tail");
        assert_eq!(foreground_at(&sections, 2), Some("Red"), "{end}");
        assert_eq!(foreground_at(&sections, 4), None, "{end}");
        assert_equivalent(
            syntax.compile().unwrap(),
            load_xshd(&syntax.to_xml().unwrap())
                .unwrap()
                .compile()
                .unwrap(),
            &["<ax tail", "<abc"],
        );
    }
}

#[test]
fn writer_hoists_inline_imports_without_name_collisions_or_losing_case_context() {
    let inline = XshdRuleSet {
        elements: vec![XshdElement::Rule(XshdRule {
            regex: Some("x".into()),
            color: XshdReference::inline(XshdColor {
                name: Some("InlineColor".into()),
                foreground: Some("Red".into()),
                ..XshdColor::default()
            }),
            ..XshdRule::default()
        })],
        ..XshdRuleSet::default()
    };
    let syntax = XshdSyntaxDefinition {
        elements: vec![
            XshdElement::RuleSet(XshdRuleSet {
                ignore_case: Some(true),
                elements: vec![XshdElement::Import(XshdImport {
                    rule_set: XshdReference::inline(inline),
                    ..XshdImport::default()
                })],
                ..XshdRuleSet::default()
            }),
            XshdElement::RuleSet(XshdRuleSet {
                name: Some("__InlineImport0".into()),
                ..XshdRuleSet::default()
            }),
        ],
        ..XshdSyntaxDefinition::default()
    };
    let saved = syntax.to_xml().unwrap();
    assert!(saved.contains("ruleSet=\"__InlineImport1\""));
    let restored = load_xshd(&saved).unwrap();
    assert_equivalent(
        syntax.compile().unwrap(),
        restored.compile().unwrap(),
        &["X x"],
    );
    assert!(restored
        .compile()
        .unwrap()
        .colors
        .contains_key("InlineColor"));
    assert_eq!(restored.to_xml().unwrap(), saved);
}

#[test]
fn writer_is_transactional_when_xml_characters_or_values_are_invalid() {
    let syntax = load_xshd("<SyntaxDefinition><RuleSet/></SyntaxDefinition>").unwrap();
    let mut writer = SaveXshdVisitor::new();
    writer.write_definition(&syntax).unwrap();
    let saved = writer.as_str().to_owned();
    for bad in ["\0", "\u{1f}", "\u{ffff}"] {
        let mut invalid = syntax.clone();
        invalid.name = Some(bad.into());
        assert!(writer.write_definition(&invalid).is_err());
        assert_eq!(writer.as_str(), saved);
    }
    let mut invalid = syntax;
    invalid.elements.push(XshdElement::Color(XshdColor {
        font_size: Some(f64::NAN),
        ..XshdColor::default()
    }));
    assert!(writer.write_definition(&invalid).is_err());
    assert_eq!(writer.as_str(), saved);
}

#[test]
fn immutable_and_mutable_visitors_visit_properties_and_inline_references() {
    #[derive(Default)]
    struct Visitor {
        seen: Vec<&'static str>,
    }
    impl XshdVisitor for Visitor {
        type Output = usize;
        type Error = Infallible;
        fn visit_rule_set(&mut self, x: &XshdRuleSet) -> Result<usize, Infallible> {
            self.seen.push("set");
            x.accept_elements(self)?;
            Ok(self.seen.len())
        }
        fn visit_color(&mut self, _: &XshdColor) -> Result<usize, Infallible> {
            self.seen.push("color");
            Ok(self.seen.len())
        }
        fn visit_property(&mut self, _: &XshdProperty) -> Result<usize, Infallible> {
            self.seen.push("property");
            Ok(self.seen.len())
        }
        fn visit_keywords(&mut self, x: &XshdKeywords) -> Result<usize, Infallible> {
            self.seen.push("keywords");
            x.color.accept(self)?;
            Ok(self.seen.len())
        }
        fn visit_span(&mut self, x: &XshdSpan) -> Result<usize, Infallible> {
            self.seen.push("span");
            x.begin_color.accept(self)?;
            x.span_color.accept(self)?;
            x.end_color.accept(self)?;
            x.rule_set.accept(self)?;
            Ok(self.seen.len())
        }
        fn visit_import(&mut self, x: &XshdImport) -> Result<usize, Infallible> {
            self.seen.push("import");
            x.rule_set.accept(self)?;
            Ok(self.seen.len())
        }
        fn visit_rule(&mut self, x: &XshdRule) -> Result<usize, Infallible> {
            self.seen.push("rule");
            x.color.accept(self)?;
            Ok(self.seen.len())
        }
    }
    struct Edit;
    impl XshdVisitorMut for Edit {
        type Output = ();
        type Error = Infallible;
        fn visit_rule_set(&mut self, x: &mut XshdRuleSet) -> Result<(), Infallible> {
            x.accept_elements_mut(self)
        }
        fn visit_color(&mut self, x: &mut XshdColor) -> Result<(), Infallible> {
            x.foreground = Some("Blue".into());
            Ok(())
        }
        fn visit_property(&mut self, x: &mut XshdProperty) -> Result<(), Infallible> {
            x.value.push('!');
            Ok(())
        }
        fn visit_keywords(&mut self, x: &mut XshdKeywords) -> Result<(), Infallible> {
            for word in &mut x.words {
                *word = word.to_uppercase();
            }
            x.color.accept_mut(self)?;
            Ok(())
        }
        fn visit_span(&mut self, x: &mut XshdSpan) -> Result<(), Infallible> {
            x.begin_color.accept_mut(self)?;
            x.span_color.accept_mut(self)?;
            x.end_color.accept_mut(self)?;
            x.rule_set.accept_mut(self)?;
            Ok(())
        }
        fn visit_import(&mut self, x: &mut XshdImport) -> Result<(), Infallible> {
            x.rule_set.accept_mut(self)?;
            Ok(())
        }
        fn visit_rule(&mut self, x: &mut XshdRule) -> Result<(), Infallible> {
            x.color.accept_mut(self)?;
            Ok(())
        }
    }
    let mut syntax = load_xshd(r#"<SyntaxDefinition><Property name="A" value="v"/><RuleSet><Keywords foreground="Red"><Word>x</Word></Keywords><Span begin="\[" end="\]"><RuleSet><Rule foreground="Green">y</Rule></RuleSet></Span></RuleSet></SyntaxDefinition>"#).unwrap();
    let mut visitor = Visitor::default();
    syntax.accept_elements(&mut visitor).unwrap();
    assert_eq!(
        visitor.seen,
        ["property", "set", "keywords", "color", "span", "set", "rule", "color"]
    );
    syntax.accept_elements_mut(&mut Edit).unwrap();
    assert_eq!(syntax.compile().unwrap().properties["A"], "v!");
    let sections = highlight(syntax.compile().unwrap(), "x X [y]");
    assert_eq!(foreground_at(&sections, 0), None);
    assert_eq!(foreground_at(&sections, 2), Some("Blue"));
    assert_eq!(foreground_at(&sections, 5), Some("Blue"));
}

#[test]
fn legacy_import_preserves_extensions_external_imports_and_digit_priority() {
    let dependency = load_xshd(r#"<SyntaxDefinition name="Base"><RuleSet><Rule foreground="Blue">base</Rule></RuleSet></SyntaxDefinition>"#).unwrap();
    let syntax = load_xshd(r#"<SyntaxDefinition extensions=".a|.b;.c" extends="Base"><Digits color="Red"/><RuleSets><RuleSet><KeyWords color="Green"><Key word="123"/><Key word=""/></KeyWords><KeyWords/></RuleSet></RuleSets></SyntaxDefinition>"#).unwrap();
    assert_eq!(syntax.extensions, [".a", ".b", ".c"]);
    let definition = syntax.compile_with_dependencies(&[&dependency]).unwrap();
    let sections = highlight(definition, "123 45 base");
    assert_eq!(foreground_at(&sections, 0), Some("Green"));
    assert_eq!(foreground_at(&sections, 4), Some("Red"));
    assert_eq!(foreground_at(&sections, 7), Some("Blue"));
    let reference = load_xshd(
        r#"<SyntaxDefinition><RuleSets><RuleSet reference="Base"/></RuleSets></SyntaxDefinition>"#,
    )
    .unwrap();
    assert_eq!(
        foreground_at(
            &highlight(
                reference.compile_with_dependencies(&[&dependency]).unwrap(),
                "base"
            ),
            0
        ),
        Some("Blue")
    );
}

#[test]
fn legacy_mini_language_matches_only_its_documented_escapes() {
    let check = |expression: &str, text: &str| {
        let pattern =
            super::legacy::import_regex(expression, false, None, XshdPosition::default()).unwrap();
        fancy_regex::Regex::new(&format!("(?x){pattern}"))
            .unwrap()
            .is_match(text)
            .unwrap()
    };
    assert!(check("@@@C", "@!"));
    assert!(!check("@@@C", "@x"));
    assert!(check("@!else@if", "if"));
    assert!(!check("@!if@if", "if"));
    assert!(!check("@-x@foo", "xfoo"));
    assert!(check("@-x@foo", " foo"));
    assert!(check("a # b", "a # b"));
    assert!(!check("a # b", "ab"));
    for expression in ["@", "@Q"] {
        assert!(
            super::legacy::import_regex(expression, false, None, XshdPosition::new(3, 4)).is_err()
        );
    }
}

#[test]
fn legacy_line_and_word_guards_preserve_marker_offsets() {
    for start in [true, false] {
        let xml = format!(
            r#"<SyntaxDefinition><RuleSets><RuleSet><Span color="Red" stopateol="true"><Begin singleword="true" startofline="{start}">if</Begin></Span></RuleSet></RuleSets></SyntaxDefinition>"#
        );
        let syntax = load_xshd(&xml).unwrap();
        let indented = highlight(syntax.compile().unwrap(), "  if x");
        assert_eq!(foreground_at(&indented, 0), None);
        assert_eq!(
            foreground_at(&indented, 2),
            if start { Some("Red") } else { None }
        );
        let after = highlight(syntax.compile().unwrap(), "x if y");
        assert_eq!(
            foreground_at(&after, 2),
            if start { None } else { Some("Red") }
        );
        assert!(highlight(syntax.compile().unwrap(), "  iffy").is_empty());
    }
}

#[test]
fn legacy_escaping_handles_backslashes_doubled_quotes_and_inherited_rules() {
    let xml = r#"<SyntaxDefinition><RuleSets><RuleSet escapecharacter="\"><Span color="Red" stopateol="false"><Begin>&quot;</Begin><End>&quot;</End></Span></RuleSet></RuleSets></SyntaxDefinition>"#;
    let syntax = load_xshd(xml).unwrap();
    let sections = highlight(syntax.compile().unwrap(), "\"a\\\"b\" tail");
    assert_eq!(foreground_at(&sections, 4), Some("Red"));
    assert_eq!(foreground_at(&sections, 8), None);
    let doubled = r#"<SyntaxDefinition><RuleSets><RuleSet><Span color="Red" stopateol="false" escapecharacter="&quot;"><Begin>&quot;</Begin><End color="Blue">&quot;</End></Span></RuleSet></RuleSets></SyntaxDefinition>"#;
    let syntax = load_xshd(doubled).unwrap();
    let sections = highlight(syntax.compile().unwrap(), "\"a\"\"b\" tail");
    assert_eq!(foreground_at(&sections, 2), Some("Red"));
    assert_eq!(foreground_at(&sections, 3), Some("Red"));
    assert_eq!(foreground_at(&sections, 4), Some("Red"));
    assert_eq!(foreground_at(&sections, 5), Some("Blue"));
    assert_eq!(foreground_at(&sections, 8), None);
    assert_equivalent(
        syntax.compile().unwrap(),
        load_xshd(&syntax.to_xml().unwrap())
            .unwrap()
            .compile()
            .unwrap(),
        &["\"a\"\"b\" tail", "\"first", "second\" end"],
    );
}

#[test]
fn legacy_markers_can_include_or_exclude_marker_text_and_whitespace() {
    for include in [true, false] {
        let xml = format!(
            r#"<SyntaxDefinition><RuleSets><RuleSet><MarkFollowing color="Red" markmarker="{include}">@</MarkFollowing><MarkPrevious color="Blue" markmarker="{include}">(</MarkPrevious></RuleSet></RuleSets></SyntaxDefinition>"#
        );
        let sections = highlight(compiled(&xml), "@  name function (");
        assert_eq!(
            foreground_at(&sections, 0),
            if include { Some("Red") } else { None }
        );
        assert_eq!(foreground_at(&sections, 3), Some("Red"));
        assert_eq!(foreground_at(&sections, 8), Some("Blue"));
        assert_eq!(
            foreground_at(&sections, 17),
            if include { Some("Blue") } else { None }
        );
    }
}

#[test]
fn every_bundled_definition_loads_compiles_and_round_trips_through_the_typed_tree() {
    let sources = [
        include_str!("../resources/ASPX.xshd"),
        include_str!("../resources/Boo.xshd"),
        include_str!("../resources/Coco-Mode.xshd"),
        include_str!("../resources/CPP-Mode.xshd"),
        include_str!("../resources/CSharp-Mode.xshd"),
        include_str!("../resources/CSS-Mode.xshd"),
        include_str!("../resources/HTML-Mode.xshd"),
        include_str!("../resources/Java-Mode.xshd"),
        include_str!("../resources/JavaScript-Mode.xshd"),
        include_str!("../resources/Json.xshd"),
        include_str!("../resources/MarkDown-Mode.xshd"),
        include_str!("../resources/MarkDownWithFontSize-Mode.xshd"),
        include_str!("../resources/Patch-Mode.xshd"),
        include_str!("../resources/PHP-Mode.xshd"),
        include_str!("../resources/PowerShell.xshd"),
        include_str!("../resources/Python-Mode.xshd"),
        include_str!("../resources/Tex-Mode.xshd"),
        include_str!("../resources/TSQL-Mode.xshd"),
        include_str!("../resources/VB-Mode.xshd"),
        include_str!("../resources/XML-Mode.xshd"),
        include_str!("../resources/XmlDoc.xshd"),
    ];
    let original: Vec<_> = sources
        .iter()
        .map(|source| load_xshd(source).unwrap())
        .collect();
    let saved: Vec<_> = original
        .iter()
        .map(|syntax| syntax.to_xml().unwrap())
        .collect();
    let restored: Vec<_> = saved
        .iter()
        .map(|source| load_xshd(source).unwrap())
        .collect();
    for index in 0..original.len() {
        let mut required = std::collections::BTreeSet::from([index]);
        let mut pending = vec![index];
        while let Some(current) = pending.pop() {
            let xml = roxmltree::Document::parse(&saved[current]).unwrap();
            for node in xml.descendants().filter(|node| node.is_element()) {
                for attribute in ["ruleSet", "color"] {
                    if let Some((name, _)) = node
                        .attribute(attribute)
                        .and_then(|value| value.rsplit_once('/'))
                    {
                        let dependency = original
                            .iter()
                            .position(|syntax| syntax.name.as_deref() == Some(name))
                            .unwrap();
                        if required.insert(dependency) {
                            pending.push(dependency);
                        }
                    }
                }
            }
        }
        let before_dependencies: Vec<_> = required
            .iter()
            .copied()
            .filter(|i| *i != index)
            .map(|i| &original[i])
            .collect();
        let after_dependencies: Vec<_> = required
            .iter()
            .copied()
            .filter(|i| *i != index)
            .map(|i| &restored[i])
            .collect();
        let before = original[index]
            .compile_with_dependencies(&before_dependencies)
            .unwrap_or_else(|error| {
                panic!("{}: {error}", original[index].name.as_deref().unwrap_or(""))
            });
        let after = restored[index]
            .compile_with_dependencies(&after_dependencies)
            .unwrap();
        assert_equivalent(
            before,
            after,
            &[
                "",
                "// comment",
                "let value = \"😀\";",
                "<tag a=\"x\">",
                "/* first",
                " second */",
                "\"a\"\"b\"",
                "# comment",
                "123 0xAB 1.5e+2",
                "\\command{value}",
            ],
        );
        assert_eq!(restored[index].to_xml().unwrap(), saved[index]);
    }
}
