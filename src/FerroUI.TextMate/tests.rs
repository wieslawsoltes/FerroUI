use crate::*;

fn grammar(source: &str) -> Grammar {
    let mut registry = Registry::new();
    registry.add_grammar_json(source).unwrap();
    registry.load_grammar("source.test").unwrap()
}

#[test]
fn utf16_tokens_capture_scopes_and_multiline_stack() {
    let grammar = grammar(
        r##"{"scopeName":"source.test","patterns":[{"begin":"/\\*","end":"\\*/","name":"comment.block","beginCaptures":{"0":{"name":"punctuation.begin"}}},{"match":"(foo)(bar)","name":"entity","captures":{"1":{"name":"first"},"2":{"name":"second"}}}]}"##,
    );
    let first = grammar.tokenize_line("😀 foobar /*", None).unwrap();
    assert_eq!(
        first
            .tokens
            .iter()
            .find(|t| t.scopes.last().is_some_and(|s| s == "first"))
            .unwrap()
            .start_index,
        3
    );
    assert_eq!(first.rule_stack.depth(), 1);
    let second = grammar
        .tokenize_line("text */ foobar", Some(&first.rule_stack))
        .unwrap();
    assert_eq!(
        second.tokens[0].scopes,
        vec!["source.test", "comment.block"]
    );
    assert_eq!(second.rule_stack.depth(), 0);
}

#[test]
fn heredoc_end_back_references_are_literal_and_persist() {
    let grammar = grammar(
        r##"{"scopeName":"source.test","patterns":[{"begin":"<<(\\S+)","end":"^\\1$","name":"string.heredoc"}]}"##,
    );
    let first = grammar.tokenize_line("<<a.b", None).unwrap();
    let second = grammar
        .tokenize_line("axb", Some(&first.rule_stack))
        .unwrap();
    assert_eq!(second.rule_stack.depth(), 1);
    assert_eq!(
        grammar
            .tokenize_line("a.b", Some(&second.rule_stack))
            .unwrap()
            .rule_stack
            .depth(),
        0
    );
}

#[test]
fn zero_length_cycles_terminate_and_work_budget_is_observable() {
    let grammar = grammar(
        r##"{"scopeName":"source.test","patterns":[{"begin":"(?=x)","end":"(?=x)","name":"meta.empty"}]}"##,
    );
    assert!(grammar.tokenize_line("xxx", None).unwrap().tokens.len() <= 2);
    assert!(
        grammar
            .tokenize_line_with_options(
                "xxx",
                None,
                TokenizeOptions {
                    max_steps: 1,
                    ..Default::default()
                },
                None
            )
            .unwrap()
            .stopped_early
    );
}

#[test]
fn while_conditions_pop_at_first_unmatched_line() {
    let grammar = grammar(
        r##"{"scopeName":"source.test","patterns":[{"begin":"^>","while":"^>","name":"markup.quote","whileCaptures":{"0":{"name":"punctuation.quote"}}}]}"##,
    );
    let first = grammar.tokenize_line("> hello", None).unwrap();
    let second = grammar
        .tokenize_line("> again", Some(&first.rule_stack))
        .unwrap();
    assert_eq!(second.rule_stack.depth(), 1);
    assert!(second.tokens[0]
        .scopes
        .iter()
        .any(|s| s == "punctuation.quote"));
    let third = grammar
        .tokenize_line("done", Some(&second.rule_stack))
        .unwrap();
    assert_eq!(third.rule_stack.depth(), 0);
}

#[test]
fn nested_repository_and_external_includes() {
    let mut registry = Registry::new();
    registry.add_grammar_json(r##"{"scopeName":"source.test","patterns":[{"include":"source.other#word"},{"include":"#local"}],"repository":{"local":{"match":"\\d+","name":"constant.numeric"}}}"##).unwrap();
    registry.add_grammar_json(r##"{"scopeName":"source.other","repository":{"word":{"match":"word","name":"keyword.external"}}}"##).unwrap();
    let result = registry
        .load_grammar("source.test")
        .unwrap()
        .tokenize_line("word 42", None)
        .unwrap();
    assert_eq!(result.tokens[0].scopes.last().unwrap(), "keyword.external");
    assert_eq!(
        result.tokens.last().unwrap().scopes.last().unwrap(),
        "constant.numeric"
    );
}

#[test]
fn theme_specificity_reset_comments_and_includes() {
    let theme = Theme::from_json_with_resolver(r##"{"include":"base","tokenColors":[{"scope":"source.rust keyword.control","settings":{"foreground":"#ff0000","fontStyle":""}},],}"##, &|_| Ok(r##"{"tokenColors":[{"scope":"keyword","settings":{"foreground":"#00ff00","fontStyle":"bold"}}]}"##.into())).unwrap();
    let style = theme.match_scopes(&["source.rust".into(), "keyword.control.rust".into()]);
    assert_eq!(style.foreground, Color::parse("#ff0000"));
    assert_eq!(style.font_style, FontStyle::NONE);
    assert_eq!(
        theme.match_scopes(&["keyword.other".into()]).font_style,
        FontStyle::BOLD
    );
    assert!(read_json("{/* unterminated").is_err());
    assert_eq!(
        read_json(
            r#"{"url":"http://a/*b*/",// comment
      "x":[1,2,],}"#
        )
        .unwrap()["url"],
        "http://a/*b*/"
    );
}

#[test]
fn selector_ancestry_exclusion_and_alternatives() {
    let selector =
        ScopeSelector::parse("L:source.rust (string | comment) - comment.block").unwrap();
    assert!(selector.matches(&["source.rust", "meta.x", "string.quoted"]));
    assert!(!selector.matches(&["source.rust", "comment.block"]));
    assert!(!ScopeSelector::parse("source.rust string")
        .unwrap()
        .matches(&["string", "source.rust"]));
    assert!(!ScopeSelector::parse("string")
        .unwrap()
        .matches(&["stringlike"]));
}

#[test]
fn selectors_preserve_upstream_matcher_cases_and_branch_priorities() {
    let cases: &[(&str, &[&str], bool)] = &[
        ("foo", &["foo"], true),
        ("foo", &["bar"], false),
        ("- foo", &["foo"], false),
        ("- foo", &["bar"], true),
        ("- - foo", &["bar"], false),
        ("bar foo", &["foo"], false),
        ("bar foo", &["bar"], false),
        ("bar foo", &["bar", "foo"], true),
        ("bar - foo", &["bar"], true),
        ("bar - foo", &["foo", "bar"], false),
        ("bar - foo", &["foo"], false),
        ("bar, foo", &["foo"], true),
        ("bar, foo", &["bar"], true),
        ("bar, foo", &["bar", "foo"], true),
        ("bar, -foo", &["bar", "foo"], true),
        ("bar, -foo", &["yo"], true),
        ("bar, -foo", &["foo"], false),
        ("(foo)", &["foo"], true),
        ("(foo - bar)", &["foo"], true),
        ("(foo - bar)", &["foo", "bar"], false),
        ("foo bar - (yo man)", &["foo", "bar"], true),
        ("foo bar - (yo man)", &["foo", "bar", "yo"], true),
        ("foo bar - (yo man)", &["foo", "bar", "yo", "man"], false),
        ("foo bar - (yo | man)", &["foo", "bar", "yo", "man"], false),
        ("foo bar - (yo | man)", &["foo", "bar", "yo"], false),
        ("(a || b)", &["unrelated"], false),
        ("(a ,, b)", &["b"], true),
        ("", &["anything"], false),
        ("- - -foo", &["foo"], false),
        (
            "entity.attribute-name",
            &["entity.attribute-name.html"],
            true,
        ),
    ];
    for (selector, scopes, expected) in cases {
        assert_eq!(
            ScopeSelector::parse(selector).unwrap().matches(scopes),
            *expected,
            "{selector:?} on {scopes:?}"
        );
    }
    let branches = ScopeSelector::parse_many("R: a, L: b, c").unwrap();
    assert_eq!(
        branches.iter().map(|s| s.priority).collect::<Vec<_>>(),
        [
            SelectorPriority::Right,
            SelectorPriority::Left,
            SelectorPriority::Normal
        ]
    );
}

#[test]
fn legacy_plist_themes_and_token_color_cycles_are_handled() {
    let source = r##"<?xml version="1.0"?><plist version="1.0"><dict>
      <key>name</key><string>Legacy</string><key>settings</key><array><dict>
      <key>scope</key><string>keyword</string><key>settings</key><dict>
      <key>foreground</key><string>#aabbcc</string><key>fontStyle</key><string>bold</string>
      </dict></dict></array></dict></plist>"##;
    let theme = Theme::from_json(source).unwrap();
    let style = theme.match_scopes(&["keyword.control".into()]);
    assert_eq!(style.foreground, Color::parse("#aabbcc"));
    assert_eq!(style.font_style, FontStyle::BOLD);
    assert!(matches!(
        Theme::from_json_with_resolver(r#"{"tokenColors":"loop"}"#, &|_| Ok(
            r#"{"tokenColors":"loop"}"#.into()
        )),
        Err(Error::ResourceCycle(_))
    ));
}

#[test]
fn left_injection_precedes_the_host_end_rule() {
    let grammar = grammar(
        r##"{"scopeName":"source.test","patterns":[{"begin":"a","end":"x","name":"meta.a"}],"injections":{"R:source.other, L:meta.a":{"match":"x","name":"injected"}}}"##,
    );
    let result = grammar.tokenize_line("ax", None).unwrap();
    assert_eq!(result.rule_stack.depth(), 1);
    assert_eq!(
        result.tokens.last().unwrap().scopes.last().unwrap(),
        "injected"
    );
}

#[test]
fn theme_preserves_upstream_scope_color_regressions() {
    let theme = Theme::from_json_with_resolver(
        include_str!("tests/fixtures/theme-dark-plus.json"),
        &|_| Ok(include_str!("tests/fixtures/theme-dark-vs.json").into()),
    )
    .unwrap();
    let cases: &[(&[&str], &str)] = &[
        (&["keyword.control"], "#C586C0"),
        (
            &[
                "keyword.control.directive.include.c",
                "meta.preprocessor.include.c",
            ],
            "#C586C0",
        ),
        (
            &["string.quoted.double.php", "variable.other.php"],
            "#9CDCFE",
        ),
        (
            &["meta.embedded.block.html", "comment.line.double-slash.js"],
            "#6A9955",
        ),
        (
            &[
                "text.html.basic",
                "meta.embedded.block.html",
                "source.js",
                "comment.line.double-slash.js",
                "punctuation.definition.comment.js",
            ],
            "#6A9955",
        ),
        (
            &[
                "source.json",
                "meta.structure.dictionary.json",
                "string.json",
                "support.type.property-name.json",
            ],
            "#9CDCFE",
        ),
        (
            &[
                "text.html.basic",
                "meta.embedded.block.html",
                "meta.tag.metadata.script.start.html",
                "entity.name.tag.html",
            ],
            "#569CD6",
        ),
    ];
    for (scopes, color) in cases {
        let scopes = scopes.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        assert_eq!(
            theme.match_scopes(&scopes).foreground,
            Color::parse(color),
            "{scopes:?}"
        );
    }
}

#[cfg(feature = "bundled-grammars")]
#[test]
fn bundled_first_line_detection_uses_shebang_patterns() {
    let catalog = BundledCatalog::new().unwrap();
    assert_eq!(
        catalog
            .language_by_first_line("#!/usr/bin/env python3")
            .unwrap()
            .unwrap()
            .id,
        "python"
    );
    assert!(catalog
        .language_by_first_line("an ordinary first line")
        .unwrap()
        .is_none());
}

#[test]
fn model_retokenizes_until_state_convergence() {
    let grammar = grammar(
        r##"{"scopeName":"source.test","patterns":[{"begin":"/\\*","end":"\\*/","name":"comment"}]}"##,
    );
    let mut model = TokenizationModel::new(grammar);
    model.set_text("a\nb\nc\nd\ne");
    assert!(!model.retokenize(100).unwrap().pending);
    model.replace_lines(1..2, ["/*".into()]).unwrap();
    assert!(model.retokenize(1).unwrap().pending);
    assert!(model.tokens(2).is_none());
    model.retokenize(100).unwrap();
    assert_eq!(
        model.tokens(4).unwrap().tokens[0].scopes.last().unwrap(),
        "comment"
    );
    model.replace_lines(2..3, ["*/".into()]).unwrap();
    model.retokenize(100).unwrap();
    assert_eq!(
        model.tokens(4).unwrap().tokens[0].scopes,
        vec!["source.test"]
    );
    model.replace_lines(0..1, ["z".into()]).unwrap();
    assert_eq!(model.retokenize(100).unwrap().lines, 0..2);
}

#[test]
fn state_cannot_be_used_by_an_unrelated_grammar() {
    let a = grammar(r#"{"scopeName":"source.test"}"#);
    let b = grammar(r#"{"scopeName":"source.test"}"#);
    assert!(matches!(
        a.tokenize_line("", Some(&b.initial_state())),
        Err(Error::InvalidState)
    ));
}

#[test]
fn metadata_round_trips_all_bit_fields() {
    let metadata =
        EncodedTokenAttributes::new(255, StandardTokenType::RegEx, true, FontStyle(15), 511, 255);
    assert_eq!(metadata.0, u32::MAX);
    assert_eq!(metadata.foreground(), 511);
    assert_eq!(metadata.language_id(), 255);
    assert_eq!(metadata.background(), 255);
    assert_eq!(metadata.font_style(), FontStyle(15));
}

#[test]
fn encoded_tokens_apply_embedded_languages_overrides_and_brackets() {
    let grammar = grammar(
        r##"{"scopeName":"source.test","patterns":[{"match":"word","name":"meta.embedded.block string.quoted"}]}"##,
    );
    let mut colors = ColorMap::new();
    let mut config = GrammarConfiguration {
        initial_language: 1,
        ..Default::default()
    };
    config.embedded_languages.insert("meta.embedded".into(), 2);
    config.token_types.push((
        ScopeSelector::parse("meta.embedded string").unwrap(),
        StandardTokenType::Other,
    ));
    config
        .balanced_bracket_selectors
        .push(ScopeSelector::parse("*").unwrap());
    config
        .unbalanced_bracket_selectors
        .push(ScopeSelector::parse("string").unwrap());
    let result = grammar
        .tokenize_line2("word plain", None, &Theme::default(), &mut colors, &config)
        .unwrap();
    assert_eq!(result.tokens.len(), 4);
    let embedded = EncodedTokenAttributes(result.tokens[1]);
    assert_eq!(embedded.language_id(), 2);
    assert_eq!(embedded.token_type(), StandardTokenType::Other);
    assert!(!embedded.contains_balanced_brackets());
    let plain = EncodedTokenAttributes(result.tokens[3]);
    assert_eq!(plain.language_id(), 1);
    assert!(plain.contains_balanced_brackets());
    assert_eq!(
        colors.get_color(plain.foreground()),
        Color::parse("#000000")
    );
}

#[test]
fn injections_and_retokenized_captures_preserve_host_scopes() {
    let grammar = grammar(
        r##"{"scopeName":"source.test","patterns":[{"match":"\\[(\\w+)\\]","name":"meta.brackets","captures":{"1":{"name":"content","patterns":[{"match":"word","name":"keyword"}]}}}],"injections":{"L:source.test":{"match":"TODO","name":"comment.todo"}}}"##,
    );
    let result = grammar.tokenize_line("[word] TODO", None).unwrap();
    assert_eq!(
        result.tokens[1].scopes,
        vec!["source.test", "meta.brackets", "content", "keyword"]
    );
    assert_eq!(
        result.tokens.last().unwrap().scopes,
        vec!["source.test", "comment.todo"]
    );
}

#[test]
fn dollar_capture_names_support_case_transforms() {
    let grammar = grammar(
        r##"{"scopeName":"source.test","patterns":[{"match":"(ABC)","name":"entity.${1:/downcase}"}]}"##,
    );
    assert_eq!(
        grammar.tokenize_line("ABC", None).unwrap().tokens[0]
            .scopes
            .last()
            .unwrap(),
        "entity.abc"
    );
}

#[test]
fn cancellation_does_not_publish_a_partial_incremental_state() {
    let grammar = grammar(r#"{"scopeName":"source.test"}"#);
    let mut model = TokenizationModel::new(grammar);
    model.set_text("a\nb");
    let result = model
        .retokenize_with_options(100, TokenizeOptions::default(), Some(&|| true))
        .unwrap();
    assert!(result.pending && result.stopped_early);
    assert_eq!(result.lines, 0..0);
    assert!(model.tokens(0).is_none());
    assert!(!model.retokenize(100).unwrap().pending);
}

#[test]
fn end_rule_precedence_and_apply_end_pattern_last_are_observable() {
    for (end_last, expected_depth) in [(false, 0), (true, 1)] {
        let source = format!(
            r##"{{"scopeName":"source.test","patterns":[{{"begin":"a","end":"x","name":"meta.a","applyEndPatternLast":{end_last},"patterns":[{{"match":"x","name":"keyword.x"}}]}}]}}"##
        );
        assert_eq!(
            grammar(&source)
                .tokenize_line("ax", None)
                .unwrap()
                .rule_stack
                .depth(),
            expected_depth
        );
    }
}

#[cfg(feature = "bundled-grammars")]
#[test]
fn bundled_catalog_loads_real_grammars_languages_and_themes() {
    let catalog = BundledCatalog::new().unwrap();
    assert!(catalog.languages().len() >= 50);
    assert_eq!(catalog.scope_by_extension(".rs"), Some("source.rust"));
    let registry = catalog.registry().unwrap();
    for (scope, source) in [
        ("source.rust", "fn main() { let x = 42; }"),
        (
            "source.cs",
            "using System; class Foo { int Bar() { return 42; } }",
        ),
        ("source.json", r#"{"hello": true, "value": 42}"#),
        ("source.python", "def main(x): # comment"),
        ("source.js", "const x = 42;"),
    ] {
        let grammar = registry.load_grammar(scope).unwrap();
        // Full-dialect bundle conformance is exercised with real Oniguruma.
        // The explicitly selected portable backend is a restricted fallback.
        if grammar.backend() == RegexBackend::PortableOnigurumaMode && scope != "source.json" {
            continue;
        }
        let result = match grammar.tokenize_line(source, None) {
            Ok(result) => result,
            Err(Error::Regex { .. } | Error::RegexLimit(_))
                if grammar.backend() == RegexBackend::PortableOnigurumaMode =>
            {
                continue
            }
            Err(error) => panic!("{scope}: {error}"),
        };
        assert!(!result.stopped_early, "{scope}");
        assert!(result.tokens.iter().any(|t| t.scopes.len() > 1), "{scope}");
    }
    for theme in ThemeName::ALL {
        catalog
            .theme(*theme)
            .unwrap_or_else(|e| panic!("{theme:?}: {e}"));
    }
}

#[cfg(all(feature = "bundled-grammars", not(feature = "portable-regex")))]
#[test]
fn token_stream_matches_official_upstream_corpus() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("tests/fixtures/upstream-tokens.json")).unwrap();
    let catalog = BundledCatalog::new().unwrap();
    let registry = catalog.registry().unwrap();
    let mut failures = Vec::new();
    for case in corpus["cases"].as_array().unwrap() {
        let scope = case["scope"].as_str().unwrap();
        let grammar = registry.load_grammar(scope).unwrap();
        let mut state = None;
        for (number, line) in case["lines"].as_array().unwrap().iter().enumerate() {
            let text = line["text"].as_str().unwrap();
            let result = grammar
                .tokenize_line(text, state.as_ref())
                .unwrap_or_else(|e| panic!("{scope}, line {number}: {e}"));
            let expected: Vec<Token> = serde_json::from_value(line["tokens"].clone()).unwrap();
            if result.tokens != expected
                || result.rule_stack.depth() != line["depth"].as_u64().unwrap() as usize
            {
                failures.push(format!("{scope}, line {number}: {text:?}\nexpected: {expected:?} depth={}\nactual: {:?} depth={}", line["depth"], result.tokens, result.rule_stack.depth()));
            }
            state = Some(result.rule_stack);
        }
    }
    assert!(
        failures.is_empty(),
        "{} differential mismatches:\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
