use crate::*;

fn grammar(source: &str) -> Grammar {
    let mut registry = Registry::new();
    registry.add_grammar_json(source).unwrap();
    registry.load_grammar("source.test").unwrap()
}

#[test]
fn registry_lazily_loads_dependencies_injections_and_theme_resources() {
    use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
    struct Locator {
        calls: RefCell<Vec<String>>,
        grammars: BTreeMap<&'static str, &'static str>,
    }
    impl IRegistryOptions for Locator {
        fn get_grammar(&self, scope: &str) -> Result<Option<RawGrammar>> {
            self.calls.borrow_mut().push(scope.into());
            self.grammars
                .get(scope)
                .map(|source| RawGrammar::from_json(source))
                .transpose()
        }
        fn get_injections(&self, scope: &str) -> Result<Vec<String>> {
            Ok(if scope == "source.test" {
                vec!["source.injection".into()]
            } else {
                Vec::new()
            })
        }
        fn get_theme(&self, name: &str) -> Result<Option<String>> {
            Ok((name == "base.json")
                .then(|| r##"{"tokenColors":[{"settings":{"foreground":"#abcdef"}}]}"##.into()))
        }
        fn get_default_theme(&self) -> Result<Option<String>> {
            Ok(Some(r##"{"name":"host theme","include":"base.json","tokenColors":[{"scope":"keyword","settings":{"foreground":"#112233"}}]}"##.into()))
        }
    }
    let locator = Rc::new(Locator {
        calls: RefCell::new(Vec::new()),
        grammars: BTreeMap::from([
            (
                "source.test",
                r##"{"scopeName":"source.test","patterns":[{"include":"source.dep#word"},{"include":"source.missing"}]}"##,
            ),
            (
                "source.dep",
                r##"{"scopeName":"source.dep","patterns":[{"include":"source.test"}],"repository":{"word":{"match":"x","name":"constant.dep"}}}"##,
            ),
            (
                "source.injection",
                r##"{"scopeName":"source.injection","injectionSelector":"L:source","patterns":[{"match":"TODO","name":"comment.todo"}]}"##,
            ),
            ("source.other", r##"{"scopeName":"source.other"}"##),
        ]),
    });
    let registry = Registry::with_options(locator.clone()).unwrap();
    assert_eq!(registry.scope_names().count(), 0);
    let grammar = registry.load_grammar("source.test").unwrap();
    let result = grammar.tokenize_line("x TODO", None).unwrap();
    assert_eq!(result.tokens[0].scopes.last().unwrap(), "constant.dep");
    assert_eq!(
        result.tokens.last().unwrap().scopes.last().unwrap(),
        "comment.todo"
    );
    assert_eq!(
        registry
            .theme()
            .match_scopes(&["keyword".into()])
            .foreground,
        Color::parse("#112233")
    );
    assert_eq!(
        registry
            .load_theme("base.json")
            .unwrap()
            .match_scopes(&["plain".into()])
            .foreground,
        Color::parse("#abcdef")
    );
    assert!(registry
        .dependencies("source.test")
        .unwrap()
        .contains(&"source.injection".into()));
    assert_eq!(
        registry.diagnostics(),
        vec![GrammarLoadDiagnostic {
            scope_name: "source.missing".into(),
            error: Error::MissingGrammar("source.missing".into())
        }]
    );
    let calls = locator.calls.borrow().len();
    registry.load_grammar("source.test").unwrap();
    assert_eq!(locator.calls.borrow().len(), calls);
    let names_before_load = registry.scope_names();
    let other = registry.load_grammar("source.other").unwrap();
    assert_eq!(names_before_load.count(), 3);
    assert_eq!(
        other.tokenize_line("TODO", None).unwrap().tokens[0].scopes,
        vec!["source.other"]
    );
    let from_repository =
        Grammar::from_repository(registry.lookup("source.test").unwrap(), &registry).unwrap();
    assert_eq!(
        from_repository
            .tokenize_line("x TODO", None)
            .unwrap()
            .tokens,
        result.tokens
    );
}

#[test]
fn host_loader_errors_do_not_poison_future_loads() {
    use std::{cell::Cell, rc::Rc};
    struct Locator(Cell<bool>);
    impl IRegistryOptions for Locator {
        fn get_grammar(&self, scope: &str) -> Result<Option<RawGrammar>> {
            if !self.0.replace(true) {
                return Err(Error::MissingResource("not ready".into()));
            }
            RawGrammar::from_json(&format!(r#"{{"scopeName":"{scope}"}}"#)).map(Some)
        }
    }
    let registry = Registry::with_options(Rc::new(Locator(Cell::new(false)))).unwrap();
    assert!(matches!(
        registry.load_grammar("source.test"),
        Err(Error::MissingResource(_))
    ));
    assert!(registry.load_grammar("source.test").is_ok());
    assert!(Registry::with_options(Rc::new(DefaultLocator))
        .unwrap()
        .load_grammar("source.absent")
        .is_err());
}

#[test]
fn line_text_and_tokenizer_contract_preserve_utf16_boundaries() {
    let line = LineText::new("a😀b\r\n");
    assert_eq!(line.len(), 6);
    assert_eq!(line.slice(1..3).unwrap().as_str(), "😀");
    assert!(matches!(line.slice(1..2), Err(Error::InvalidOffset(2))));
    assert_eq!(line.without_line_ending(), "a😀b");
    let owned = line.clone().into_owned();
    assert_eq!(owned, line);
    let grammar =
        grammar(r#"{"scopeName":"source.test","patterns":[{"match":"b$","name":"keyword"}]}"#);
    let tokenizer: &dyn ITokenizationSupport = &grammar;
    assert_eq!(
        tokenizer
            .tokenize(line, None, TokenizeOptions::default())
            .unwrap()
            .tokens,
        grammar.tokenize_line("a😀b", None).unwrap().tokens
    );
}

#[test]
fn model_events_publish_completed_ranges_and_dispose_explicitly() {
    use std::{cell::RefCell, rc::Rc};
    let grammar = grammar(r#"{"scopeName":"source.test"}"#);
    let lines = vec!["a\r\n", "😀\n", "last"];
    let source: &dyn IModelLines = &lines;
    assert_eq!(source.line_length(1), Some(2));
    let mut model = TokenizationModel::from_lines(grammar, source).unwrap();
    assert_eq!(model.line(0), Some("a"));
    let events = Rc::new(RefCell::new(Vec::new()));
    let sink = events.clone();
    let listener: Rc<dyn IModelTokensChangedListener> =
        Rc::new(move |event: &ModelTokensChangedEvent| sink.borrow_mut().push(event.clone()));
    let subscription = model.add_model_tokens_changed_listener(listener);
    let interface: &mut dyn ITMModel = &mut model;
    let result = interface.force_tokenization(1).unwrap();
    assert_eq!(result.lines, 0..2);
    assert!(result.pending);
    assert_eq!(events.borrow()[0].ranges, vec![0..2]);
    assert!(events.borrow()[0].pending);
    assert_eq!(events.borrow()[0].revision, model.revision());
    model
        .retokenize_with_options(10, TokenizeOptions::default(), Some(&|| true))
        .unwrap();
    assert_eq!(events.borrow().len(), 1);
    assert!(model.tokens(2).is_none());
    subscription.dispose();
    subscription.dispose();
    model.force_tokenization(2).unwrap();
    assert_eq!(events.borrow().len(), 1);
    let sink = events.clone();
    drop(model.tokens_changed(move |event| sink.borrow_mut().push(event.clone())));
    model.invalidate_lines(0..1).unwrap();
    model.retokenize(10).unwrap();
    assert_eq!(events.borrow().len(), 2);
    assert_eq!(events.borrow()[1].ranges, vec![0..2]);
    assert!(!events.borrow()[1].pending);
}

#[test]
fn model_events_keep_a_completed_prefix_when_a_later_line_fails() {
    use std::{cell::RefCell, rc::Rc};
    let grammar =
        grammar(r##"{"scopeName":"source.test","patterns":[{"begin":"start","end":"["}]}"##);
    let mut model = TokenizationModel::new(grammar);
    model.set_text("plain\nstart\nlater");
    let ranges = Rc::new(RefCell::new(Vec::new()));
    let sink = ranges.clone();
    let _subscription =
        model.tokens_changed(move |event| sink.borrow_mut().extend(event.ranges.clone()));
    assert!(model.retokenize(10).is_err());
    assert_eq!(*ranges.borrow(), vec![0..1]);
    assert!(model.tokens(0).is_some());
    assert!(model.tokens(1).is_none());
    assert_eq!(model.pending_line(), Some(1));
}

#[test]
fn language_configuration_preserves_typed_rules_words_and_snippets() {
    let configuration = LanguageConfiguration::from_json(r##"{
      "comments":{"lineComment":"//","blockComment":["/*","*/"]},
      "autoCloseBefore":";:.,=}",
      "folding":{"offSide":true,"markers":{"start":"^#region","end":"^#endregion"}},
      "indentationRules":{"increaseIndentPattern":{"pattern":"\\{$","flags":"i"},"decreaseIndentPattern":"^\\}","indentNextLinePattern":"if$"},
      "onEnterRules":[{"beforeText":"\\{$","afterText":{"pattern":"^\\}"},"previousLineText":"if","action":{"indent":"indentOutdent","appendText":" * ","removeText":1}}],
      "autoClosingPairs":[["(",")"],{"open":"\"","close":"\"","notIn":["string","comment"]}],
      "surroundingPairs":[{"open":"/*","close":"*/"}],
      "wordPattern":{"pattern":"[a-z😀]+","flags":"gi"}
    }"##).unwrap();
    assert_eq!(
        configuration.comments().unwrap().unwrap().block_comment,
        Some(("/*".into(), "*/".into()))
    );
    assert!(!configuration.folding().unwrap().unwrap().is_empty());
    let indentation = configuration.indentation_rules().unwrap().unwrap();
    assert!(!indentation.is_empty());
    assert_eq!(indentation.increase.unwrap().flags, "i");
    let rule = &configuration.on_enter_rules().unwrap()[0];
    assert_eq!(rule.action.indent, IndentAction::IndentOutdent);
    assert_eq!(rule.action.remove_text, Some(1));
    let pairs = configuration.auto_closing_pairs().unwrap().pairs;
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[1].not_in, vec!["string", "comment"]);
    assert_eq!(configuration.surrounding_pairs().unwrap()[0].open, "/*");
    assert_eq!(
        configuration.word_at_offset("ABC 😀word +", 7).unwrap(),
        Some(WordRange {
            start_index: 4,
            end_index: 10
        })
    );
    assert!(matches!(
        configuration.word_at_offset("x", 2),
        Err(Error::InvalidOffset(2))
    ));
    let no_words = LanguageConfiguration::from_json(r#"{"wordPattern":"(?=x)"}"#).unwrap();
    assert!(no_words.word_at_offset("xxx", 1).unwrap().is_none());
    let snippets = LanguageSnippets::from_json(r##"{"region":{"prefix":["reg","region"],"body":["#region ${1:name}","$0","#endregion"],"description":"A region","scope":"csharp,cpp"},"line":{"prefix":"x","body":"text"}}"##).unwrap();
    assert_eq!(snippets.snippets["region"].prefixes, vec!["reg", "region"]);
    assert_eq!(snippets.snippets["region"].scopes, vec!["csharp", "cpp"]);
    assert_eq!(snippets.snippets["line"].body, vec!["text"]);
}

#[cfg(feature = "bundled-grammars")]
#[test]
fn bundled_language_settings_decode_through_typed_contracts() {
    let catalog = BundledCatalog::new().unwrap();
    for language in catalog.languages() {
        language.typed_snippets().unwrap();
        if let Some(configuration) = &language.configuration {
            configuration
                .comments()
                .unwrap_or_else(|e| panic!("{}: {e}", language.id));
            configuration
                .folding()
                .unwrap_or_else(|e| panic!("{}: {e}", language.id));
            configuration
                .indentation_rules()
                .unwrap_or_else(|e| panic!("{}: {e}", language.id));
            configuration
                .on_enter_rules()
                .unwrap_or_else(|e| panic!("{}: {e}", language.id));
            configuration
                .auto_closing_pairs()
                .unwrap_or_else(|e| panic!("{}: {e}", language.id));
            configuration
                .surrounding_pairs()
                .unwrap_or_else(|e| panic!("{}: {e}", language.id));
            configuration
                .word_regex()
                .unwrap_or_else(|e| panic!("{}: {e}", language.id));
        }
    }
}

#[test]
fn package_metadata_preserves_contributions_and_maps_host_language_ids() {
    let package = GrammarDefinition::from_json(r##"{
      "name":"sample","displayName":"Sample","version":"1.2.3","license":"MIT",
      "engines":{"vscode":"^1.0","node":">=18"},"scripts":{"update-grammar":"generate"},
      "repository":"https://example.invalid/sample","custom":{"retained":true},
      "contributes":{"languages":[{"id":"sample","mimetypes":["text/x-sample"],"configuration":"config.json"}],
        "grammars":[{"language":"sample","scopeName":"source.sample","path":"grammar.json",
          "embeddedLanguages":{"meta.embedded.js":"javascript","source.unknown":"unknown"},
          "tokenTypes":{"meta.embedded":"other","comment,comment.line":"comment"},
          "balancedBracketScopes":["*"],"unbalancedBracketScopes":["string"]}],
        "snippets":[{"language":"sample","path":"snippets.json"}]}
    }"##).unwrap();
    assert_eq!(package.engines.unwrap().vscode.as_deref(), Some("^1.0"));
    assert_eq!(
        package.repository.unwrap().url,
        "https://example.invalid/sample"
    );
    assert_eq!(package.other["custom"]["retained"], true);
    assert_eq!(
        package.contributes.languages[0].mime_types,
        vec!["text/x-sample"]
    );
    assert_eq!(package.contributes.snippets[0].path, "snippets.json");
    let configuration = package.contributes.grammars[0]
        .configuration(|id| match id {
            "sample" => Some(1),
            "javascript" => Some(2),
            _ => None,
        })
        .unwrap();
    assert_eq!(configuration.initial_language, 1);
    assert_eq!(configuration.embedded_languages.len(), 1);
    assert_eq!(configuration.embedded_languages["meta.embedded.js"], 2);
    assert_eq!(configuration.token_types.len(), 3);
    assert!(configuration.balanced_bracket_selectors[0].matches(&["source.sample"]));
    assert!(configuration.unbalanced_bracket_selectors[0].matches(&["string.quoted"]));
}

#[cfg(feature = "bundled-grammars")]
#[test]
fn bundled_catalog_is_a_lazy_host_locator_and_exposes_package_metadata() {
    use std::rc::Rc;
    let catalog = Rc::new(BundledCatalog::with_default_theme(ThemeName::LightPlus).unwrap());
    assert_eq!(catalog.packages().count(), 51);
    assert_eq!(catalog.grammar_contributions().len(), 79);
    assert_eq!(catalog.language_by_alias("C#").unwrap().id, "csharp");
    assert_eq!(
        catalog.language_by_mime_type("text/html").unwrap().id,
        "html"
    );
    let configuration = catalog
        .grammar_configuration("text.html.derivative", |id| match id {
            "html" => Some(1),
            "javascript" => Some(2),
            _ => None,
        })
        .unwrap();
    assert_eq!(configuration.initial_language, 1);
    assert_eq!(configuration.embedded_languages["source.js"], 2);
    assert_eq!(configuration.token_types.len(), 1);
    assert!(catalog
        .get_injections("source.ts")
        .unwrap()
        .contains(&"documentation.injection.ts".into()));
    for grammar in catalog.grammar_contributions() {
        grammar.configuration(|_| Some(1)).unwrap();
    }
    let registry = catalog.clone().lazy_registry().unwrap();
    assert_eq!(registry.scope_names().count(), 0);
    let grammar = registry.load_grammar("source.json").unwrap();
    assert!(registry.scope_names().count() < 79);
    assert!(
        !grammar
            .tokenize_line(r#"{"ok":true}"#, None)
            .unwrap()
            .stopped_early
    );
    assert_eq!(
        registry.theme().match_scopes(&["source.json".into()]),
        catalog
            .theme(ThemeName::LightPlus)
            .unwrap()
            .match_scopes(&["source.json".into()])
    );
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
