use crate::*;

#[cfg(all(
    feature = "native-onig",
    not(feature = "portable-regex"),
    not(target_arch = "wasm32")
))]
#[test]
fn ordered_native_groups_preserve_tokens_stacks_and_work_budgets() {
    let grouped_grammar = grammar(r#"{
        "scopeName": "source.test",
        "patterns": [
            {"match":"\\Ahead","name":"first"},
            {"match":"head","name":"later"},
            {"begin":"\\[","end":"\\]","name":"list","patterns":[
                {"match":"\\G😀","name":"anchored"},
                {"match":"(?<=😀)([a-z]+)\\1","name":"repeated","captures":{"1":{"name":"pair"}}},
                {"match":"\\]","name":"child.end"}
            ]},
            {"match":"zz","name":"host"},
            {"match":"(?=!)","name":"zero"}
        ],
        "injections":{"L:source.test":{"patterns":[
            {"match":"z","name":"injection.first"},
            {"match":"zz","name":"injection.second"}
        ]}}
    }"#);
    for max_steps in (0..40).chain([1000]) {
        let options = TokenizeOptions {
            max_steps,
            ..TokenizeOptions::default()
        };
        let mut state = None;
        for line in ["head [😀abab", "] zz !", "head"] {
            let grouped = grouped_grammar
                .tokenize_line_with_options(line, state.as_ref(), options, None)
                .unwrap();
            let individual = grouped_grammar
                .tokenize_line_with_options(line, state.as_ref(), options, Some(&|| false))
                .unwrap();
            assert_eq!(grouped.tokens, individual.tokens, "{line:?}, budget {max_steps}");
            assert_eq!(grouped.rule_stack, individual.rule_stack, "{line:?}, budget {max_steps}");
            assert_eq!(grouped.stopped_early, individual.stopped_early, "{line:?}, budget {max_steps}");
            state = Some(grouped.rule_stack);
        }
    }

    let invalid = grammar(r#"{
        "scopeName":"source.test","patterns":[{"match":"a"},{"match":"("}]
    }"#);
    assert!(matches!(
        invalid.tokenize_line("a", None),
        Err(Error::Regex { pattern, .. }) if pattern == "("
    ));
}

#[test]
fn compiled_grammar_exposes_metadata_and_the_active_resolved_end_rule() {
    let grammar = grammar(
        r#"{"name":"Example","scopeName":"source.test","fileTypes":[".example","other","..rc"],
        "patterns":[{"name":"string.delimited","begin":"<<([A-Z]+)","end":"^\\1$"}]}"#,
    );
    assert_eq!(grammar.name(), Some("Example"));
    assert_eq!(grammar.file_types(), ["example", "other", ".rc"]);
    let root = grammar.initial_state();
    assert_eq!(root.end_rule(), None);
    let continuation = grammar.tokenize_line("<<END", Some(&root)).unwrap().rule_stack;
    assert_ne!(continuation.rule_id(), root.rule_id());
    assert_eq!(continuation.end_rule(), Some("^END$"));
    let closed = grammar.tokenize_line("END", Some(&continuation)).unwrap().rule_stack;
    assert_eq!(closed.rule_id(), root.rule_id());
    assert_eq!(closed.end_rule(), None);
}

#[test]
fn tokenizer_prefix_uses_checked_utf16_offsets() {
    let grammar = grammar(r#"{"scopeName":"source.test"}"#);
    let tokenizer: &dyn ITokenizationSupport = &grammar;
    let line = LineText::new("a😀z\r\n");
    let options = TokenizeOptions::default();
    let result = tokenizer.tokenize_at(line.clone(), None, options, 50, Some(3)).unwrap();
    assert_eq!((result.tokens[0].start_index, result.tokens[0].end_index), (50, 53));
    assert!(matches!(
        tokenizer.tokenize_at(line.clone(), None, options, 0, Some(2)),
        Err(Error::InvalidOffset(2))
    ));
    assert!(matches!(
        tokenizer.tokenize_at(line, None, options, usize::MAX, None),
        Err(Error::InvalidOffset(usize::MAX))
    ));
}

fn grammar(source: &str) -> Grammar {
    let mut registry = Registry::new();
    registry.add_grammar_json(source).unwrap();
    registry.load_grammar("source.test").unwrap()
}

#[test]
fn injection_ties_use_declaration_order_and_the_match_position() {
    // Token scope sequences recorded with the pinned reference grammar engine.
    let mut registry = Registry::new();
    const HOST: &str = r#"{"scopeName":"source.test","patterns":[{"match":"x","name":"host"}]}"#;
    const LOCAL: &str = r#"{
        "scopeName":"source.test",
        "patterns":[{"match":"x","name":"host"}],
        "injections":{
          "L:source.test":{"patterns":[{"match":"x","name":"local.first"},{"match":"x","name":"local.first.fallback"}]},
          "L:source":{"match":"x","name":"local.second"}
        }
    }"#;
    for suffix in ["z", "a"] {
        registry
            .add_grammar_json(&format!(
                r#"{{"scopeName":"source.{suffix}","injectionSelector":"L:source.test","patterns":[{{"match":"x","name":"external.{suffix}"}}]}}"#,
            ))
            .unwrap();
    }
    for (host, injections, input, expected) in [
        (LOCAL, &[][..], "x", &[(0, 1, "local.first")][..]),
        (
            LOCAL,
            &[],
            " x",
            &[(0, 1, "source.test"), (1, 2, "local.second")],
        ),
        (
            HOST,
            &["source.z", "source.a"],
            "x",
            &[(0, 1, "external.z")],
        ),
        (
            HOST,
            &["source.a", "source.z"],
            "x",
            &[(0, 1, "external.a")],
        ),
        (
            HOST,
            &["source.z", "source.a"],
            " x",
            &[(0, 1, "source.test"), (1, 2, "external.a")],
        ),
        (
            LOCAL,
            &["source.z", "source.a"],
            "x",
            &[(0, 1, "local.first")],
        ),
        (
            LOCAL,
            &["source.z", "source.a"],
            " x",
            &[(0, 1, "source.test"), (1, 2, "external.a")],
        ),
    ] {
        registry.add_grammar_json(host).unwrap();
        registry.set_injections("source.test", injections.iter().map(|name| (*name).into()));
        let grammar = registry.load_grammar("source.test").unwrap();
        assert_scoped_ranges(&grammar, input, expected);
    }
}

#[test]
fn injection_matches_compete_with_host_patterns_and_end_rules() {
    let host = grammar(
        r#"{
        "scopeName":"source.test",
        "patterns":[{"match":"x","name":"host"},{"match":"y","name":"host"}],
        "injections":{
            "R:source.test":{"match":"x","name":"injected.right"},
            "L:source.test":{"match":"x","name":"injected.left"}
        }
    }"#,
    );
    assert_scoped_ranges(&host, "x", &[(0, 1, "injected.left")]);
    assert_scoped_ranges(&host, " x", &[(0, 1, "source.test"), (1, 2, "host")]);

    let host = grammar(
        r#"{
        "scopeName":"source.test",
        "patterns":[
            {"begin":"a","end":"x","name":"meta.block","applyEndPatternLast":true},
            {"match":"y","name":"host"}
        ],
        "injections":{
            "R:source.test - meta.block":{"match":"x","name":"injected.right"},
            "R:meta.block":{"match":"x","name":"injected.right"}
        }
    }"#,
    );
    assert_scoped_ranges(&host, "xy", &[(0, 1, "injected.right"), (1, 2, "host")]);
    let closed = host.tokenize_line("ax", None).unwrap();
    assert_eq!(
        closed.tokens.last().unwrap().scopes.last().unwrap(),
        "meta.block"
    );
    assert_eq!(closed.rule_stack.rule_id(), host.initial_state().rule_id());
}

#[test]
fn large_injection_sets_keep_equal_priorities_in_declaration_order() {
    let injections = (0..24)
        .map(|index| {
            let pattern = if index == 0 { "q" } else { "x" };
            format!(
                r#""L:source.test - unused.{index}":{{"match":"{pattern}","name":"local.{index}"}}"#
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let host = grammar(&format!(
        r#"{{"scopeName":"source.test","injections":{{{injections}}}}}"#,
    ));
    assert_scoped_ranges(&host, "x", &[(0, 1, "local.1")]);
    assert_scoped_ranges(&host, " x", &[(0, 1, "source.test"), (1, 2, "local.23")]);
}

#[test]
fn plist_grammar_reads_complete_keys_and_regex_text_across_xml_nodes() {
    let host = grammar(
        r#"<plist version="1.0"><dict>
        <key>scope<!-- split key -->Name</key><string>source.<!-- split scope -->test</string>
        <key>patterns</key><array><dict>
            <key>match</key><string>a<!-- split regex --><![CDATA[&]]>b</string>
            <key>name</key><string>keyword.legacy</string>
        </dict><dict>
            <key>match</key><string>(x)(y)</string>
            <key>captures</key><array>
                <dict><key>name</key><string>capture.plist.first</string></dict>
                <dict><key>name</key><string>capture.plist.second</string></dict>
            </array>
        </dict></array>
    </dict></plist>"#,
    );
    assert_scoped_ranges(&host, "a&b", &[(0, 3, "keyword.legacy")]);
    assert_scoped_ranges(&host, "xy", &[(0, 1, "capture.plist.first"), (1, 2, "capture.plist.second")]);
    assert_scoped_ranges(&host, "a", &[(0, 1, "source.test")]);
}

#[test]
fn plist_grammar_injections_preserve_dictionary_declaration_order() {
    let host = grammar(
        r#"<?xml version="1.0"?><plist version="1.0"><dict>
        <key>scopeName</key><string>source.test</string>
        <key>injections</key><dict>
            <key>L:source.test</key><dict><key>match</key><string>x</string><key>name</key><string>local.first</string></dict>
            <key>L:source</key><dict><key>match</key><string>x</string><key>name</key><string>local.second</string></dict>
        </dict>
    </dict></plist>"#,
    );
    assert_scoped_ranges(&host, "x", &[(0, 1, "local.first")]);
    assert_scoped_ranges(
        &host,
        " x",
        &[(0, 1, "source.test"), (1, 2, "local.second")],
    );
}

fn assert_scoped_ranges(grammar: &Grammar, line: &str, expected: &[(usize, usize, &str)]) {
    let result = grammar.tokenize_line(line, None).unwrap();
    assert_token_ranges(&result.tokens, line, expected);
}

fn assert_token_ranges(tokens: &[Token], line: &str, expected: &[(usize, usize, &str)]) {
    let actual: Vec<_> = tokens
        .iter()
        .map(|token| {
            (
                token.start_index,
                token.end_index,
                token.scopes.last().unwrap().as_str(),
            )
        })
        .collect();
    assert_eq!(actual, expected, "input {line:?}");
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
    let mut raw = registry.load_raw_theme("base.json").unwrap();
    raw.document["include"] = serde_json::json!("missing.json");
    assert_eq!(
        registry.compile_theme(raw).unwrap_err(),
        Error::MissingResource("missing.json".into()),
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
fn model_events_publish_completed_ranges_while_guard_is_alive() {
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
    drop(subscription);
    model.force_tokenization(2).unwrap();
    assert_eq!(events.borrow().len(), 1);
    let sink = events.clone();
    drop(model.tokens_changed(move |event| sink.borrow_mut().push(event.clone())));
    model.invalidate_lines(0..1).unwrap();
    model.retokenize(10).unwrap();
    assert_eq!(events.borrow().len(), 1);
}

#[test]
fn capture_retokenization_starts_without_a_begin_anchor() {
    let grammar = grammar(
        r#"{
        "scopeName":"source.test",
        "patterns":[{
            "match":"<((?:x|\\[x\\]))>",
            "captures":{"1":{"patterns":[
                {"match":"\\Gx$","name":"unexpected.capture.anchor"},
                {"match":"(?<=\\A<)x$","name":"unexpected.document.anchor"},
                {"begin":"\\[","end":"\\]","patterns":[
                    {"match":"\\Gx","name":"nested.begin.anchor"}
                ]},
                {"match":"x","name":"capture.body"}
            ]}}
        }]
    }"#,
    );
    for (line, start, scope) in [
        ("<x>", 1, "capture.body"),
        ("😀 <x>", 4, "capture.body"),
        ("😀 <[x]>", 5, "nested.begin.anchor"),
    ] {
        let result = grammar.tokenize_line(line, None).unwrap();
        let token = result
            .tokens
            .iter()
            .find(|token| token.start_index == start)
            .unwrap();
        assert_eq!(token.end_index, start + 1);
        assert_eq!(token.scopes, ["source.test", scope], "{line:?}");
    }
}

#[test]
fn document_anchor_is_available_at_line_start_until_the_first_advance() {
    let grammar = grammar(
        r#"{
        "scopeName":"source.test",
        "patterns":[
            {"match":"^(x)!","captures":{"1":{"patterns":[
                {"match":"\\Ax$","name":"document.first"},
                {"match":"x","name":"capture.body"}
            ]}}},
            {"match":"^a","name":"prefix"},
            {"match":"(?<=\\Aa)B","name":"unexpected.document.anchor"},
            {"match":"B","name":"after.prefix"}
        ]
    }"#,
    );
    let first = grammar.tokenize_line("x!", None).unwrap();
    assert_eq!(first.tokens[0].scopes, ["source.test", "document.first"]);
    let later = grammar
        .tokenize_line("x!", Some(&first.rule_stack))
        .unwrap();
    assert_eq!(later.tokens[0].scopes, ["source.test", "capture.body"]);
    let advanced = grammar.tokenize_line("aB", None).unwrap();
    assert_eq!(advanced.tokens[1].scopes, ["source.test", "after.prefix"]);
}

#[test]
fn model_without_a_grammar_accepts_edits_without_scheduling_tokenization() {
    use std::{cell::Cell, rc::Rc};

    let mut model = TokenizationModel::default();
    let notifications = Rc::new(Cell::new(0));
    let sink = notifications.clone();
    let _subscription = model.tokens_changed(move |_| sink.set(sink.get() + 1));
    model.clear_grammar();
    model.set_lines(&vec!["first\r\n", "second"]).unwrap();
    model.set_text("first\r\nsecond\r\nthird");
    model.replace_lines(1..2, [String::from("edited")]).unwrap();
    model.invalidate_lines(0..3).unwrap();

    assert!(model.grammar().is_none());
    assert_eq!(model.line(1), Some("edited"));
    assert_eq!(model.pending_line(), None);
    assert!(!model.is_line_invalid(1));
    let update = model
        .force_tokenization_with_options(2, TokenizeOptions::default(), Some(&|| {
            panic!("disabled tokenization must not enter the scanner")
        }))
        .unwrap();
    assert_eq!(update.lines, 0..0);
    assert!(!update.pending);
    assert!(model.tokens(2).is_none());
    assert!(matches!(
        model.force_tokenization(3),
        Err(Error::InvalidLine(3))
    ));
    assert_eq!(notifications.get(), 0);
}

#[test]
fn clearing_the_active_grammar_emits_one_full_range_and_restores_fresh_tokenization() {
    use std::{cell::RefCell, rc::Rc};

    let first = grammar(r#"{"scopeName":"source.test","patterns":[{"begin":"start","end":"stop","name":"block.old"}]}"#);
    let second = grammar(r#"{"scopeName":"source.test","patterns":[{"match":"edited","name":"keyword.new"}]}"#);
    let mut model = TokenizationModel::new(first.clone());
    model.set_text("start\nbody\nstop");
    assert!(model.force_tokenization(0).unwrap().pending);
    assert!(model.tokens(0).unwrap().tokens[0]
        .scopes
        .iter()
        .any(|scope| scope == "block.old"));
    let revision = model.revision();
    model.set_grammar(first.clone());
    assert_eq!(model.revision(), revision);
    assert_eq!(model.pending_line(), Some(1));

    let events = Rc::new(RefCell::new(Vec::new()));
    let sink = events.clone();
    let _subscription = model.tokens_changed(move |event| sink.borrow_mut().push(event.clone()));
    let interface: &mut dyn ITMModel = &mut model;
    interface.clear_grammar();
    interface.clear_grammar();
    assert!(interface.grammar().is_none());
    assert!(interface.tokens(0).is_none());
    assert_eq!(model.pending_line(), None);
    assert_eq!(
        *events.borrow(),
        [ModelTokensChangedEvent {
            ranges: vec![0..3],
            revision: revision + 1,
            line_count: 3,
            pending: false,
        }]
    );

    model.replace_lines(1..2, [String::from("edited")]).unwrap();
    assert_eq!(events.borrow().len(), 1);
    model.set_grammar(second);
    assert_eq!(model.pending_line(), Some(0));
    let completed = model.force_tokenization(2).unwrap();
    assert_eq!(completed.lines, 0..3);
    assert!(!completed.pending);
    assert_eq!(events.borrow().len(), 2);
    assert_eq!(model.tokens(0).unwrap().tokens[0].scopes, ["source.test"]);
    assert_eq!(
        model.tokens(1).unwrap().tokens[0].scopes,
        ["source.test", "keyword.new"]
    );

    // Even a grammar replaced before its first scheduled batch must clear the view.
    events.borrow_mut().clear();
    for _ in 0..50 {
        model.set_grammar(first.clone());
        model.clear_grammar();
    }
    assert_eq!(events.borrow().len(), 50);
    assert!(events
        .borrow()
        .iter()
        .all(|event| event.ranges == [0..3] && !event.pending));
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
                .folding_rules()
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
            configuration
                .enter_rules()
                .unwrap_or_else(|e| panic!("{}: {e}", language.id));
            configuration
                .editing_indentation_rules()
                .unwrap_or_else(|e| panic!("{}: {e}", language.id));
            configuration
                .word_at_offset("", 0)
                .unwrap_or_else(|e| panic!("{}: {e}", language.id));
        }
    }
}

#[test]
fn host_packages_supply_language_detection_configuration_and_cross_package_grammars() {
    use std::rc::Rc;

    let catalog = BundledCatalog::with_packages(
        ThemeName::DarkPlus,
        &[
            LanguagePackage {
                name: "sample-markup",
                manifest: r#"{"name":"sample-markup","contributes":{
                    "languages":[{
                        "id":"sample-markup","aliases":["Sample Markup"],"extensions":[".smp"],
                        "filenamePatterns":["*.sample.view"],"firstLine":"^#!sample-markup",
                        "configuration":"./language.json"
                    }],
                    "grammars":[{
                        "language":"sample-markup","scopeName":"source.sample-markup",
                        "path":"./syntaxes/main.json",
                        "embeddedLanguages":{"meta.sample-value":"sample-value"}
                    }],
                    "snippets":[{"language":"sample-markup","path":"./snippets.json"}]
                }}"#,
                resources: &[
                    (r"syntaxes\main.json", r#"{"scopeName":"source.sample-markup","patterns":[
                        {"include":"source.sample-value"},{"match":"!","name":"punctuation.main"}
                    ]}"#),
                    ("language.json", r##"{"comments":{"lineComment":"#"},"brackets":[["(",")"]]}"##),
                    ("snippets.json", r#"{"Value":{"prefix":"value","body":"value ${1:name}"}}"#),
                ],
            },
            LanguagePackage {
                name: "sample-values",
                manifest: r#"{"name":"sample-values","contributes":{"grammars":[
                    {"scopeName":"source.sample-value","path":"value.json"},
                    {"scopeName":"source.sample-alert","path":"alert.json","injectTo":["source.sample-markup"]}
                ]}}"#,
                resources: &[
                    ("value.json", r#"{"scopeName":"source.sample-value","patterns":[
                        {"match":"[a-z]+","name":"meta.sample-value"}
                    ]}"#),
                    ("alert.json", r#"{"scopeName":"source.sample-alert",
                        "injectionSelector":"L:source.sample-markup","patterns":[
                            {"match":"!","name":"punctuation.alert"}
                        ]}"#),
                ],
            },
        ],
    )
    .unwrap();
    let language = catalog.language_by_extension(".SMP").unwrap();
    assert_eq!(language.id, "sample-markup");
    assert_eq!(catalog.language_by_alias("Sample Markup").unwrap().id, language.id);
    assert_eq!(catalog.language_by_filename("example.sample.view").unwrap().id, language.id);
    assert_eq!(catalog.language_by_first_line("#!sample-markup").unwrap().unwrap().id, language.id);
    assert_eq!(language.configuration.as_ref().unwrap().line_comment(), Some("#"));
    assert_eq!(language.typed_snippets().unwrap().snippets["Value"].prefixes, ["value"]);
    let scope = language.scope_name.clone().unwrap();
    let configuration = catalog
        .grammar_configuration(&scope, |name| (name == "sample-value").then_some(9))
        .unwrap();
    assert_eq!(configuration.embedded_languages["meta.sample-value"], 9);

    let catalog = Rc::new(catalog);
    let lazy = catalog.clone().lazy_registry().unwrap();
    assert_eq!(lazy.scope_names().count(), 0);
    let eager = catalog.registry().unwrap();
    for registry in [&lazy, &eager] {
        let grammar = registry.load_grammar(&scope).unwrap();
        let result = grammar.tokenize_line("word!", None).unwrap();
        assert_eq!(result.tokens.len(), 2);
        assert_eq!(result.tokens[0].scopes.last().unwrap(), "meta.sample-value");
        assert_eq!(result.tokens[1].scopes.last().unwrap(), "punctuation.alert");
    }
    assert_eq!(lazy.scope_names().count(), 3);
}

#[test]
fn filename_and_extension_choose_the_same_language_for_shared_suffixes() {
    let catalog = BundledCatalog::with_packages(
        ThemeName::DarkPlus,
        &[
            LanguagePackage {
                name: "shared-z",
                manifest: r#"{"name":"shared-z","contributes":{"languages":[
                    {"id":"shared-z","extensions":[".port-syntax",".schema.port-syntax"]}
                ]}}"#,
                resources: &[],
            },
            LanguagePackage {
                name: "shared-a",
                manifest: r#"{"name":"shared-a","contributes":{"languages":[
                    {"id":"shared-a","extensions":[".PORT-SYNTAX"]}
                ]}}"#,
                resources: &[],
            },
        ],
    )
    .unwrap();

    for (filename, extension, expected) in [
        ("/work/日本語.PORT-SYNTAX", ".port-syntax", "shared-a"),
        ("C:\\work\\model.SCHEMA.PORT-SYNTAX", "schema.port-syntax", "shared-z"),
    ] {
        assert_eq!(catalog.language_by_extension(extension).unwrap().id, expected);
        assert_eq!(catalog.language_by_filename(filename).unwrap().id, expected);
    }
}

#[test]
fn host_packages_report_missing_ambiguous_and_outside_resources() {
    use std::rc::Rc;

    const MANIFEST: &str = r#"{"contributes":{"grammars":[
        {"scopeName":"source.sample","path":"./syntax.json"}
    ]}}"#;
    fn package<'a>(resources: &'a [(&'a str, &'a str)]) -> LanguagePackage<'a> {
        LanguagePackage {
            name: "sample",
            manifest: MANIFEST,
            resources,
        }
    }
    assert!(matches!(
        BundledCatalog::with_packages(ThemeName::DarkPlus, &[package(&[])]),
        Err(Error::MissingResource(path)) if path == "grammars/sample/syntax.json"
    ));
    for path in ["../syntax.json", "/syntax.json", r"C:\syntax.json", r"\syntax.json"] {
        assert!(matches!(
            BundledCatalog::with_packages(ThemeName::DarkPlus, &[package(&[(path, "{}")])]),
            Err(Error::InvalidPackage(_))
        ));
    }
    assert!(matches!(
        BundledCatalog::with_packages(
            ThemeName::DarkPlus,
            &[package(&[("syntax.json", "{}"), ("./syntax.json", "{}")])],
        ),
        Err(Error::InvalidPackage(_))
    ));
    let source = [("syntax.json", r#"{"scopeName":"source.wrong"}"#)];
    let catalog = Rc::new(BundledCatalog::with_packages(ThemeName::DarkPlus, &[package(&source)]).unwrap());
    assert!(matches!(catalog.registry(), Err(Error::InvalidPackage(_))));
    let registry = catalog.lazy_registry().unwrap();
    assert!(matches!(registry.load_grammar("source.sample"), Err(Error::InvalidPackage(_))));
    assert_eq!(registry.scope_names().count(), 0);
    assert!(matches!(
        BundledCatalog::with_packages(
            ThemeName::DarkPlus,
            &[package(&source), LanguagePackage { name: "duplicate", ..package(&source) }],
        ),
        Err(Error::InvalidPackage(_))
    ));
}

#[cfg(feature = "bundled-grammars")]
#[test]
fn host_package_can_replace_a_bundled_package_without_using_its_files() {
    let manifest = r#"{"contributes":{
        "languages":[{"id":"ini","extensions":[".ini"],"configuration":"custom.json"}],
        "grammars":[{"language":"ini","scopeName":"source.ini","path":"custom-syntax.json"}]
    }}"#;
    let catalog = BundledCatalog::with_packages(
        ThemeName::DarkPlus,
        &[LanguagePackage {
            name: "ini",
            manifest,
            resources: &[
                ("custom.json", r#"{"comments":{"lineComment":"!"}}"#),
                ("custom-syntax.json", r#"{"scopeName":"source.ini","patterns":[{"match":"x","name":"host.ini"}]}"#),
            ],
        }],
    )
    .unwrap();
    let ini = catalog.language_by_id("ini").unwrap();
    assert_eq!(ini.configuration.as_ref().unwrap().line_comment(), Some("!"));
    assert_eq!(catalog.languages().iter().filter(|language| language.id == "ini").count(), 1);
    assert!(catalog.language_by_extension(".json").is_some());
    let registry = catalog.registry().unwrap();
    let result = registry.load_grammar("source.ini").unwrap().tokenize_line("x", None).unwrap();
    assert_eq!(result.tokens[0].scopes.last().unwrap(), "host.ini");
}

#[test]
fn enter_rules_use_first_matching_context_and_regex_flags() {
    let configuration = LanguageConfiguration::from_json(r#"{
      "onEnterRules":[
        {"beforeText":{"pattern":"begin$","flags":"gi"},"afterText":"^end",
         "previousLineText":{"pattern":"\u001f.prev$","flags":"s"},
         "action":{"indent":"indentOutdent","appendText":" 😀 "}},
        {"beforeText":{"pattern":"begin$","flags":"iy"},"action":{"indent":"outdent"}},
        {"beforeText":{"pattern":"begin$","flags":"i"},"action":{"indent":"indent"}}
      ]
    }"#).unwrap();
    let rules = configuration.enter_rules().unwrap();
    for _ in 0..2 {
        let action = rules.action("text BEGIN", "end", "\u{1f}\u{2028}prev").unwrap().unwrap();
        assert_eq!(action.indent, IndentAction::IndentOutdent);
        assert_eq!(action.append_text.as_deref(), Some(" 😀 "));
    }
    assert_eq!(rules.action("BEGIN", "next", "").unwrap().unwrap().indent, IndentAction::Outdent);
    assert_eq!(rules.action("text BEGIN", "end", "other").unwrap().unwrap().indent, IndentAction::Indent);
    assert!(rules.action("finish", "end", "").unwrap().is_none());
    assert!(LanguageConfiguration::from_json(r#"{"onEnterRules":[{"beforeText":{"pattern":"a","flags":"z"}}]}"#)
        .unwrap().enter_rules().is_err());
}

#[cfg(feature = "bundled-grammars")]
#[test]
fn bundled_javascript_enter_rules_match_the_reference_contexts() {
    let catalog = BundledCatalog::new().unwrap();
    let language = catalog.language_by_id("javascript").unwrap();
    let rules = language.configuration.as_ref().unwrap().enter_rules().unwrap();
    // The expected actions also come from evaluating the pinned package with
    // JavaScript RegExp, including precedence between the two comment openers.
    for (before, after, previous, indent, append, remove) in [
        ("\t/** 🌐", "  */", "", IndentAction::IndentOutdent, Some(" * "), None),
        ("\t/** 🌐", "", "", IndentAction::None, Some(" * "), None),
        ("\t * more", " suffix", "\t/** note", IndentAction::None, Some("* "), None),
        ("\t */", "", "\t * more", IndentAction::None, None, Some(1)),
        ("\tcase 1:", " break;", "", IndentAction::Indent, None, None),
    ] {
        let action = rules.action(before, after, previous).unwrap().unwrap();
        assert_eq!((action.indent, action.append_text.as_deref(), action.remove_text), (indent, append, remove));
    }
    assert!(rules.action("\tcase 1:", " default:", "").unwrap().is_none());
}

#[test]
fn configuration_expressions_use_javascript_anchors_whitespace_and_flags() {
    let configuration = LanguageConfiguration::from_json(
        r#"{
          "onEnterRules":[
            {"beforeText":"^tail","action":{"indent":"outdent"}},
            {"beforeText":{"pattern":"^tail","flags":"m"},"action":{"indent":"indent"}}
          ],
          "indentationRules":{
            "increaseIndentPattern":{"pattern":"^[[a-z]--[x]]+$","flags":"v"},
            "decreaseIndentPattern":"^\\s+end$"
          }
        }"#,
    )
    .unwrap();
    let enter = configuration.enter_rules().unwrap();
    assert_eq!(
        enter
            .action("head\u{2028}tail", "", "")
            .unwrap()
            .unwrap()
            .indent,
        IndentAction::Indent
    );
    assert_eq!(
        enter.action("tail", "", "").unwrap().unwrap().indent,
        IndentAction::Outdent
    );
    let indent = configuration.editing_indentation_rules().unwrap().unwrap();
    assert!(indent.should_increase("abc").unwrap());
    assert!(!indent.should_increase("ax").unwrap());
    assert!(indent.should_decrease("\u{feff}end").unwrap());
    assert!(!indent.should_decrease("\u{0085}end").unwrap());

    for flags in ["gg", "uv", "z"] {
        let invalid = LanguageConfiguration::from_json(&format!(
            r#"{{"wordPattern":{{"pattern":"a","flags":"{flags}"}}}}"#
        ))
        .unwrap();
        assert!(matches!(
            invalid.word_at_offset("a", 0),
            Err(Error::Regex { .. })
        ));
    }
}

#[test]
fn configured_words_use_utf16_and_advance_past_empty_matches() {
    let range = |start_index, end_index| {
        Some(WordRange {
            start_index,
            end_index,
        })
    };
    for (pattern, flags, line, offset, expected) in [
        (".", "", "😀", 2, range(1, 2)),
        (".", "u", "😀", 2, range(0, 2)),
        (r"\w+", "", "é abc", 1, None),
        (r"\w+", "", "é abc", 3, range(2, 5)),
        ("(?=a)|b", "", "ab", 2, range(1, 2)),
        ("x", "y", " x", 2, None),
    ] {
        let source = serde_json::json!({"wordPattern": {"pattern": pattern, "flags": flags}});
        let config = LanguageConfiguration::from_json(&source.to_string()).unwrap();
        assert_eq!(config.word_at_offset(line, offset).unwrap(), expected);
    }
}

#[test]
fn effective_pairs_resolve_fallbacks_exclusions_and_longest_openers() {
    let config = LanguageConfiguration::from_json(r#"{
      "brackets":[["(",")"]], "autoCloseBefore":"!"
    }"#).unwrap();
    let rules = config.language_pairs().unwrap();
    assert_eq!(rules.auto_closing, rules.surrounding);
    assert!(rules.allows_following("!", false));
    assert!(rules.allows_following(")", false));
    assert!(!rules.allows_following(" ", false));

    let config = LanguageConfiguration::from_json(r#"{
      "brackets":[["(",")"]], "surroundingPairs":[],
      "autoClosingPairs":[["\"","\""],
        {"open":"f\"","close":"\"","notIn":["comment","regex"],"beforeText":{"pattern":"^f\"$","flags":"i"}}]
    }"#).unwrap();
    let rules = config.language_pairs().unwrap();
    assert!(rules.surrounding.is_empty());
    let pair = rules.opening_pair("\"", &["f", "f"]).unwrap().unwrap();
    assert_eq!(pair.open, "f\"");
    assert!(!pair.allows_token_type(StandardTokenType::Comment));
    assert!(!pair.allows_token_type(StandardTokenType::RegEx));
    assert!(pair.allows_token_type(StandardTokenType::String));
    assert_eq!(rules.opening_pair("\"", &["f", "of"]).unwrap().unwrap().open, "\"");
    assert!(rules.opening_pair("(", &[""]).unwrap().is_none());
}

#[cfg(feature = "bundled-grammars")]
#[test]
fn bundled_filename_patterns_override_generic_extensions() {
    let catalog = BundledCatalog::new().unwrap();
    assert_eq!(
        catalog
            .languages()
            .iter()
            .map(|language| language.filename_patterns.len())
            .sum::<usize>(),
        15
    );
    for (filename, expected) in [
        ("/work/Dockerfile.release", "dockerfile"),
        ("C:\\work\\CONTAINERFILE.Debug", "dockerfile"),
        ("Jenkinsfile.integration", "groovy"),
        ("/home/user/.config/git/config", "properties"),
        ("C:\\work\\project\\.git\\config", "properties"),
        (".git/config", "properties"),
        ("compose.yml", "dockercompose"),
        ("compose.yaml", "dockercompose"),
        ("/work/compose.production.yml", "dockercompose"),
        ("/work/compose.production.yaml", "dockercompose"),
        ("my-docker-compose-test.yml", "dockercompose"),
        ("MY-DOCKER-COMPOSE-TEST.YAML", "dockercompose"),
        ("tsconfig.app.json", "jsonc"),
        ("jsconfig.editor.json", "jsonc"),
        ("tsconfig-node.json", "jsonc"),
        ("jsconfig-web.json", "jsonc"),
        ("regular.json", "json"),
        ("ordinary.yaml", "yaml"),
        ("tsconfig.json", "jsonc"),
        ("Dockerfile", "dockerfile"),
        ("lib.rs", "rust"),
    ] {
        assert_eq!(
            catalog
                .language_by_filename(filename)
                .map(|language| language.id.as_str()),
            Some(expected),
            "{filename}"
        );
    }
    assert!(catalog
        .language_by_filename("unknown.extension-not-registered")
        .is_none());
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
fn match_rule_takes_precedence_over_begin_and_uses_match_captures() {
    let host = grammar(
        r##"{"scopeName":"source.test","patterns":[{
            "match":"x","begin":"<","end":">","name":"token",
            "contentName":"meta.block",
            "captures":{"0":{"name":"match.capture"}},
            "beginCaptures":{"0":{"name":"begin.capture"}}
        }]}"##,
    );
    let result = host.tokenize_line("x rest", None).unwrap();
    assert_eq!(result.rule_stack.depth(), 0);
    assert_token_ranges(
        &result.tokens,
        "x rest",
        &[(0, 1, "match.capture"), (1, 6, "source.test")],
    );
    assert_scoped_ranges(&host, "<", &[(0, 1, "source.test")]);
}

#[test]
fn begin_while_rule_ignores_an_end_pattern_and_its_captures() {
    let host = grammar(
        r##"{"scopeName":"source.test","patterns":[{
            "begin":"^>","while":"^>","end":"!","name":"meta.quote",
            "contentName":"comment.quote",
            "whileCaptures":{"0":{"name":"continuation.marker"}},
            "endCaptures":{"0":{"name":"unused.end"}}
        }]}"##,
    );
    let mut state = None;
    let lines: &[(&str, usize, &[(usize, usize, &str)])] = &[
        (">a!", 1, &[(0, 1, "meta.quote"), (1, 3, "comment.quote")]),
        (
            ">b!",
            1,
            &[(0, 1, "continuation.marker"), (1, 3, "comment.quote")],
        ),
        ("plain", 0, &[(0, 5, "source.test")]),
    ];
    for &(line, depth, expected) in lines {
        let result = host.tokenize_line(line, state.as_ref()).unwrap();
        assert_eq!(result.rule_stack.depth(), depth, "input {line:?}");
        assert_token_ranges(&result.tokens, line, expected);
        state = Some(result.rule_stack);
    }
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
fn rules_with_only_missing_patterns_cannot_enter_the_tokenizer_stack() {
    let mut registry = Registry::new();
    registry
        .add_grammar_json(r##"{
            "scopeName":"source.target",
            "patterns":[
                {"begin":"\\[","end":"\\]","name":"bad.end","patterns":[{"include":"#missing"}]},
                {"begin":"^>","while":"^ ","name":"bad.while","patterns":[{"include":"#missing"}]},
                {"begin":"\\{","end":"\\}","name":"bad.wrapper","patterns":[{"patterns":[{"include":"#missing"}]}]},
                {"begin":"\\(","end":"\\)","name":"bad.external","patterns":[{"include":"source.other#missing"}]},
                {"match":"ok","name":"keyword.ok"}
            ],
            "injections":{"L:source.target":{"begin":"<","end":">","name":"bad.injection","patterns":[{"include":"#missing"}]}}
        }"##)
        .unwrap();
    registry
        .add_grammar_json(r#"{"scopeName":"source.other","patterns":[{"match":"external"}]}"#)
        .unwrap();
    let grammar = registry.load_grammar("source.target").unwrap();
    let mut state = None;
    for line in ["[ok", ">ok", "{ok", "(ok", "<ok", " ok"] {
        let result = grammar.tokenize_line(line, state.as_ref()).unwrap();
        assert!(!result.stopped_early, "{line}");
        assert_eq!(result.rule_stack.depth(), 0, "{line}");
        assert_token_ranges(&result.tokens, line, &[(0, 1, "source.target"), (1, 3, "keyword.ok")]);
        state = Some(result.rule_stack);
    }
    registry
        .add_grammar_json(r#"{"scopeName":"source.other","repository":{"missing":{"match":"ok","name":"keyword.external"}}}"#)
        .unwrap();
    let reloaded = registry.load_grammar("source.target").unwrap();
    assert_scoped_ranges(&reloaded, "(ok)", &[(0, 1, "bad.external"), (1, 3, "keyword.external"), (3, 4, "bad.external")]);
    assert_scoped_ranges(&grammar, "(ok)", &[(0, 1, "source.target"), (1, 3, "keyword.ok"), (3, 4, "source.target")]);
}

#[test]
fn missing_patterns_leave_empty_mixed_and_recursive_rules_usable() {
    let grammar = grammar(r##"{
        "scopeName":"source.test",
        "patterns":[
            {"begin":"\\[","end":"\\]","name":"meta.empty"},
            {"begin":"<","end":">","name":"meta.empty-group","patterns":[{"patterns":[]}]},
            {"begin":"\\{","end":"\\}","name":"meta.mixed","patterns":[{"include":"#missing"},{"match":"ok","name":"keyword.inner"}]},
            {"include":"#recursive"}
        ],
        "repository":{"recursive":{"begin":"\\(","end":"\\)","name":"meta.recursive","patterns":[{"include":"#recursive"},{"include":"#missing"}]}}
    }"##);
    assert_scoped_ranges(&grammar, "[ok]", &[(0, 4, "meta.empty")]);
    assert_scoped_ranges(&grammar, "<ok>", &[(0, 4, "meta.empty-group")]);
    assert_scoped_ranges(&grammar, "{ok}", &[(0, 1, "meta.mixed"), (1, 3, "keyword.inner"), (3, 4, "meta.mixed")]);
    let before = grammar.tokenize_line("((", None).unwrap();
    assert_eq!(before.rule_stack.depth(), 2);
    let after = grammar.tokenize_line("ok))", Some(&before.rule_stack)).unwrap();
    assert_eq!(after.rule_stack.depth(), 0);
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
fn raw_theme_can_be_edited_and_round_tripped_before_compilation() {
    let mut raw = RawTheme::parse(r##"{
        // Keep authoring data alongside the styles.
        "name": "Host theme",
        "include": "base.tmTheme",
        "colors": { "editor.foreground": "#555", "editor.background": null },
        "settings": [{
            "name": "Named rule",
            "scope": ["keyword", "constant"],
            "settings": { "foreground": "#123", "fontStyle": "", "custom": [true, null, 2] },
        }],
        "custom": { "resource": "untouched" },
    }"##).unwrap();
    raw.document["settings"][0]["settings"]["foreground"] = serde_json::json!("#456");
    let serialized = serde_json::to_string(&raw).unwrap();
    let restored = RawTheme::parse(&serialized).unwrap();
    assert_eq!(restored, raw);
    assert_eq!(restored.document["settings"][0]["name"], "Named rule");
    assert_eq!(
        restored.document["settings"][0]["settings"]["custom"],
        serde_json::json!([true, null, 2])
    );
    assert!(restored.document["colors"]["editor.background"].is_null());

    let base = RawTheme::parse(r#"<plist version="1.0"><dict>
        <key>name</key><string>Base theme</string>
        <key>settings</key><array><dict>
            <key>name</key><string>Comment rule</string>
            <key>scope</key><string>comment</string>
            <key>settings</key><dict>
                <key>foreground</key><string>#abc</string>
                <key>fontStyle</key><string>italic</string>
            </dict>
        </dict></array>
        <key>extensions</key><array><false/><integer>4</integer></array>
    </dict></plist>"#).unwrap();
    assert_eq!(base.document["settings"][0]["name"], "Comment rule");
    assert_eq!(base.document["extensions"], serde_json::json!([false, 4]));
    assert_eq!(
        RawTheme::parse(&serde_json::to_string(&base).unwrap()).unwrap(),
        base
    );
    let theme = Theme::from_raw_with_resolver(restored, &|path| {
        assert_eq!(path, "base.tmTheme");
        Ok(base.clone())
    }).unwrap();
    assert_eq!(theme.name.as_deref(), Some("Host theme"));
    assert_eq!(
        theme.match_scopes(&["constant.language".into()]).foreground,
        Color::parse("#456")
    );
    let comment = theme.match_scopes(&["comment.line".into()]);
    assert_eq!(comment.foreground, Color::parse("#abc"));
    assert_eq!(comment.font_style, FontStyle::ITALIC);
}

#[test]
fn empty_theme_include_does_not_request_a_resource() {
    let theme = Theme::from_json(r##"{
        "include": "",
        "colors": {"editor.foreground": "#123456"},
        "tokenColors": [{"scope": "keyword", "settings": {"foreground": "#778899"}}]
    }"##).unwrap();
    assert_eq!(theme.colors["editor.foreground"], Color::rgba(0x12, 0x34, 0x56, 255));
    assert_eq!(theme.match_scopes(&["keyword".into()]).foreground, Color::parse("#778899"));
}

#[test]
fn deep_theme_resources_preserve_distinct_include_and_token_color_defaults() {
    let root = r##"{
        "include": "0",
        "name": "root",
        "colors": {"editor.foreground": "#246"},
        "tokenColors": "token-palette"
    }"##;
    let theme = Theme::from_json_with_resolver(root, &|name| {
        if name == "token-palette" {
            return Ok(r##"{
                "include": "4096",
                "tokenColors": [{"settings": {"foreground": "#abc"}}]
            }"##.into());
        }
        if name == "4096" {
            return Ok(r##"{
                "colors": {"editor.background": "#123"},
                "tokenColors": [
                    {"settings": {"background": "#123"}},
                    {"scope": "keyword", "settings": {"fontStyle": "italic"}}
                ]
            }"##.into());
        }
        let next = name.parse::<usize>().unwrap() + 1;
        Ok(format!(r#"{{"include": "{next}"}}"#))
    }).unwrap();

    assert_eq!(theme.name.as_deref(), Some("root"));
    assert_eq!(theme.colors.get("editor.foreground").copied(), Color::parse("#246"));
    assert_eq!(theme.colors.get("editor.background").copied(), Color::parse("#123"));
    let scopes = ["source.test".into(), "keyword.control".into()];
    assert_eq!(theme.match_scopes(&scopes).foreground, Color::parse("#abc"));
    assert_eq!(theme.match_scopes(&scopes).background, Color::parse("#123"));
    assert_eq!(theme.encoded_style(&scopes).foreground, Color::parse("#abc"));
    assert_eq!(theme.encoded_style(&scopes).background, Color::parse("#fff"));
    assert_eq!(theme.match_scopes(&scopes).font_style, FontStyle::ITALIC);
}

#[test]
fn theme_cycle_covers_both_include_and_token_color_references() {
    let error = Theme::from_json_with_resolver(r#"{"include": "base"}"#, &|name| match name {
        "base" => Ok(r#"{"tokenColors": "palette"}"#.into()),
        "palette" => Ok(r#"{"include": "base"}"#.into()),
        _ => Err(Error::MissingResource(name.into())),
    }).unwrap_err();
    assert_eq!(error, Error::ResourceCycle("base".into()));
}

#[test]
fn theme_combines_legacy_settings_and_token_colors() {
    let theme = Theme::from_json(
        r##"{
            "settings": [
                {"scope":"keyword", "settings":{"fontStyle":"bold"}},
                {"scope":"storage", "settings":{"foreground":"#112233"}},
                {"scope":"comment", "settings":{"foreground":"#00ff00"}},
                {"scope":"constant", "settings":{"foreground":"#654321"}},
                {"scope":"support", "settings":{"foreground":"#445566"}},
                {"scope":"entity", "settings":{"foreground":"#223344"}}
            ],
            "tokenColors": [
                {"scope":"keyword", "settings":{"foreground":"#123456"}},
                {"scope":"string", "settings":{"foreground":"#aabbcc"}},
                {"scope":"comment", "settings":{"foreground":"#ccaa11"}},
                {"scope":"constant", "settings":{"foreground":"#332211"}},
                {"scope":"entity", "settings":{"foreground":"#ff2200"}},
                {"scope":"storage", "settings":{"foreground":"#222222"}}
            ]
        }"##,
    )
    .unwrap();

    for (scope, foreground, font_style) in [
        ("keyword.control", "#123456", FontStyle::BOLD),
        ("storage.type", "#222222", FontStyle::NONE),
        ("string.quoted", "#aabbcc", FontStyle::NONE),
        ("comment", "#ccaa11", FontStyle::NONE),
        ("constant", "#332211", FontStyle::NONE),
        ("entity", "#223344", FontStyle::NONE),
        ("support", "#445566", FontStyle::NONE),
    ] {
        let style = theme.match_scopes(&[scope.into()]);
        assert_eq!(style.foreground, Color::parse(foreground), "{scope}");
        assert_eq!(style.font_style, font_style, "{scope}");
    }
}

#[test]
fn theme_scope_shapes_preserve_defaults_and_scoped_entries() {
    let theme = Theme::from_json(
        r##"{"settings":[
            {"settings":{"foreground":"#ff0000"}},
            {"scope":"", "settings":{"foreground":"#00ff00"}},
            {"scope":[], "settings":{"foreground":"#0000ff"}},
            {"scope":"  ", "settings":{"background":"#111111"}},
            {"scope":["", "keyword"], "settings":{"fontStyle":"bold"}},
            {"scope":"comment", "settings":{"fontStyle":"italic"}}
        ]}"##,
    )
    .unwrap();
    assert_eq!(
        theme.defaults(),
        TokenStyle {
            foreground: Color::parse("#ff0000"),
            background: Color::parse("#111111"),
            font_style: FontStyle::BOLD,
        }
    );
    assert_eq!(
        theme
            .match_scopes(&["comment".into(), "keyword".into()])
            .font_style,
        FontStyle::BOLD,
    );
}

#[test]
fn theme_prefers_more_specific_matching_parent_paths() {
    let theme = Theme::from_json(
        r##"{"settings":[
            {"scope":"source.outer meta.near keyword.control", "settings":{"foreground":"#aabbcc"}},
            {"scope":"source.outer meta.deep meta.far keyword.control", "settings":{"foreground":"#112233"}},
            {"scope":"meta.y string", "settings":{"foreground":"#654321"}},
            {"scope":"meta.longest string", "settings":{"foreground":"#123456"}},
            {"scope":"string.quoted", "settings":{"foreground":"#654322"}}
        ]}"##,
    )
    .unwrap();
    let cases: &[(&[&str], &str)] = &[
        (
            &[
                "source.outer",
                "meta.deep",
                "meta.far",
                "meta.near",
                "keyword.control",
            ],
            "#112233",
        ),
        (
            &["source.outer", "meta.far", "meta.near", "keyword.control"],
            "#aabbcc",
        ),
        (&["meta.longest", "meta.y", "string.plain"], "#123456"),
        (&["meta.longest", "meta.y", "string.quoted"], "#654322"),
    ];
    for (scopes, foreground) in cases {
        let scopes = scopes
            .iter()
            .map(|scope| (*scope).to_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            theme.match_scopes(&scopes).foreground,
            Color::parse(foreground),
            "{scopes:?}"
        );
    }
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
fn selector_path_negation_and_recovery_match_the_pinned_builder() {
    let cases: &[(&str, &[&str], bool)] = &[
        ("- a b", &[], true),
        ("- a b", &["a"], true),
        ("- a b", &["b"], true),
        ("- a b", &["b", "a"], true),
        ("- a b", &["a.parent", "meta.other", "b.child"], false),
        ("a a", &["a.one"], false),
        ("a a", &["a.one", "a.two"], true),
        ("R:(foo | bar", &["bar"], true),
        ("foo) , L:bar", &["bar"], false),
        ("foo) , L:bar", &["foo"], true),
        ("(| a)", &["a"], false),
        ("-", &[], false),
        ("- -", &[], true),
        ("foo -", &["foo"], false),
        ("a () b", &["a"], true),
    ];
    for (source, scopes, expected) in cases {
        let selector = ScopeSelector::parse(source).unwrap();
        assert_eq!(selector.matches(scopes), *expected, "{source} {scopes:?}");
        assert_eq!(selector.specificity(scopes).is_some(), *expected);
    }

    let branches = ScopeSelector::parse_many("foo, R: , L:bar,").unwrap();
    assert_eq!(branches.len(), 2);
    assert_eq!(branches[0].priority, SelectorPriority::Normal);
    assert!(branches[0].matches(&["foo"]));
    assert_eq!(branches[1].priority, SelectorPriority::Left);
    assert!(branches[1].matches(&["bar"]));
    assert!(ScopeSelector::parse_many("()").unwrap().is_empty());
    assert!(ScopeSelector::parse("L:-a b").unwrap().matches(&["a"]));

    let deeply_nested = "(".repeat(256) + "foo";
    assert!(matches!(
        ScopeSelector::parse(&deeply_nested),
        Err(Error::InvalidSelector(_))
    ));
}

#[test]
fn selector_custom_predicate_receives_whole_paths_and_short_circuits() {
    let branches = ScopeSelector::parse_many("R:x y - blocked, L:(off | on)").unwrap();
    let mut paths = Vec::new();
    let mut matches_path = |names: &[String], input: &u8| {
        let path = names.join(" ");
        let matched = match path.as_str() {
            "x y" => input & 3 == 3,
            "blocked" => input & 4 != 0,
            "off" => false,
            "on" => input & 8 != 0,
            _ => panic!("unexpected selector path {path}"),
        };
        paths.push(path);
        matched
    };
    assert!(!branches[0].matches_with(&0, &mut matches_path));
    assert!(branches[0].matches_with(&3, &mut matches_path));
    assert!(!branches[0].matches_with(&7, &mut matches_path));
    assert!(!branches[1].matches_with(&0, &mut matches_path));
    assert!(branches[1].matches_with(&8, &mut matches_path));
    assert_eq!(
        paths,
        ["x y", "x y", "blocked", "x y", "blocked", "off", "on", "off", "on"]
    );
}

#[test]
fn injections_exclude_the_entire_path_and_load_recoverable_selectors() {
    for selector in [
        "L:source.test - meta.container comment",
        "L:(source.test - meta.container comment",
    ] {
        let grammar = grammar(
            &serde_json::json!({
                "scopeName": "source.test",
                "patterns": [{
                    "begin": "a", "end": "A", "name": "meta.container",
                    "patterns": [{"begin": "#", "end": "#", "name": "comment.line"}]
                }],
                "injections": { (selector): {"match": "x", "name": "injected"} }
            })
            .to_string(),
        );
        assert_scoped_ranges(
            &grammar,
            "xax#x#xAx",
            &[
                (0, 1, "injected"),
                (1, 2, "meta.container"),
                (2, 3, "injected"),
                (3, 6, "comment.line"),
                (6, 7, "injected"),
                (7, 8, "meta.container"),
                (8, 9, "injected"),
            ],
        );
    }
}

#[test]
fn raw_plist_theme_keeps_real_and_opaque_metadata() {
    let source = r##"<plist version="1.0"><dict>
        <key>name</key><string>Legacy</string>
        <key>scopeName</key><string>source.legacy</string>
        <key>metadata</key><dict>
            <key>spacing</key><real> 1.25 </real>
            <key>revision</key><integer> 2 </integer>
            <key>created</key><date>2026-11-20T12:34:56Z</date>
            <key>preview</key><data>ZX<!-- split data -->hhbXBsZQ==</data>
        </dict>
        <key>settings</key><array><dict>
            <key>scope</key><string>keyword</string>
            <key>settings</key><dict>
                <key>foreground</key><string>#aabbcc</string>
            </dict>
        </dict></array>
    </dict></plist>"##;
    let raw = RawTheme::parse(source).unwrap();
    assert_eq!(
        raw.document["metadata"],
        serde_json::json!({
            "spacing": 1.25,
            "revision": 2,
            "created": "2026-11-20T12:34:56Z",
            "preview": "ZXhhbXBsZQ=="
        })
    );
    let json = serde_json::to_string(&raw).unwrap();
    assert_eq!(RawTheme::parse(&json).unwrap(), raw);
    let theme = Theme::from_raw(raw).unwrap();
    assert_eq!(
        theme.match_scopes(&["keyword.control".into()]).foreground,
        Color::parse("#aabbcc")
    );
    let grammar = RawGrammar::from_json(source).unwrap();
    assert_eq!(grammar.scope_name, "source.legacy");

    let non_finite = source.replace(" 1.25 ", "NaN");
    assert!(matches!(
        RawTheme::parse(&non_finite),
        Err(Error::Json(message)) if message.contains("non-finite plist real")
    ));
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
fn token_type_matches_the_first_boundary_word_in_each_scope() {
    use StandardTokenType::{Comment, Other, RegEx, String as StringToken};
    for (scope, expected) in [
        ("punctuation.definition.string.begin", StringToken),
        ("meta.comment.block", Comment),
        ("source.regex-token", RegEx),
        ("comment.string", Comment),
        ("string.meta.embedded", StringToken),
        ("custom.meta.embedded.block.string", Other),
        ("commentary", Other),
        ("xcomment", Other),
        ("_comment", Other),
        ("comment2", Other),
        ("Comment", Other),
    ] {
        assert_eq!(
            EncodedTokenAttributes::infer_token_type(&[scope.into()]),
            expected,
            "{scope}",
        );
    }
    // An unrecognized inner scope inherits its parent. An embedded scope
    // explicitly resets that parent type, and a deeper typed scope overrides it.
    for (scopes, expected) in [
        (vec!["comment.block", "invalid.illegal"], Comment),
        (
            vec!["comment.block", "custom.meta.embedded.block.string", "invalid.illegal"],
            Other,
        ),
        (
            vec!["comment.block", "custom.meta.embedded", "punctuation.string.end"],
            StringToken,
        ),
    ] {
        let scopes: Vec<_> = scopes.into_iter().map(str::to_owned).collect();
        assert_eq!(EncodedTokenAttributes::infer_token_type(&scopes), expected);
    }
}

#[test]
fn token_type_boundaries_follow_dotnet_unicode_categories_and_utf16() {
    use StandardTokenType::{Comment, RegEx};
    // .NET boundary words: letters, nonspacing marks, decimal digits,
    // connector punctuation, and the two join controls. These prevent a match.
    for character in ['é', 'Ω', '中', '\u{0301}', '\u{0663}', '\u{203f}', '\u{200c}', '\u{200d}'] {
        for scope in [format!("{character}regex"), format!("regex{character}")] {
            assert_eq!(
                EncodedTokenAttributes::infer_token_type(&["comment".into(), scope.clone()]),
                Comment,
                "no word boundary in {scope:?}",
            );
        }
    }
    // Spacing/enclosing marks, letter/other numbers and supplementary letters
    // are not boundary-word UTF-16 code units in .NET. Rust's Unicode \w set
    // would incorrectly include several of them.
    for character in ['\u{0903}', '\u{20dd}', '\u{2160}', '\u{00b2}', '𐐀', '😀', '-'] {
        for scope in [format!("{character}regex"), format!("regex{character}")] {
            assert_eq!(
                EncodedTokenAttributes::infer_token_type(&["comment".into(), scope.clone()]),
                RegEx,
                "word boundary in {scope:?}",
            );
        }
    }
}

#[test]
fn included_token_defaults_are_independent_of_the_canvas_palette() {
    let theme = Theme::from_json_with_resolver(
        r##"{"include":"base","colors":{"editor.background":"#101010"}}"##,
        &|_| {
            Ok(r##"{
            "colors":{"editor.foreground":"#ffffff","editor.background":"#000000"},
            "tokenColors":[{"settings":{"foreground":"#d0d0d0","background":"#202020"}}]
        }"##
            .into())
        },
    )
    .unwrap();
    assert_eq!(
        theme.colors["editor.background"],
        Color::parse("#101010").unwrap()
    );
    assert_eq!(theme.defaults().foreground, Color::parse("#d0d0d0"));
    assert_eq!(
        theme.style_overrides(&[]).background,
        Color::parse("#202020")
    );
}

#[test]
fn encoded_theme_styles_use_token_defaults_and_scoped_overrides() {
    let grammar = grammar(
        r#"{"scopeName":"source.test","patterns":[
        {"match":"red","name":"keyword.control.probe"},
        {"match":"bold","name":"entity.name.probe"},
        {"match":"reset","name":"entity.name.reset"}
    ]}"#,
    );
    let base = r##"{
        "colors":{"editor.foreground":"#c4d0dd","editor.background":"#202830"},
        "tokenColors":[
            {"settings":{"foreground":"#def123","background":"#123456","fontStyle":"italic"}},
            {"scope":"entity","settings":{"foreground":"#234567","fontStyle":"bold"}}
        ]
    }"##;
    let themes = [
        (
            r##"{
                "colors":{"editor.foreground":"#c4d0dd","editor.background":"#202830"},
                "tokenColors":[{"scope":"keyword","settings":{"foreground":"#ee5500"}}]
            }"##,
            vec![
                (0, "#ee5500", FontStyle::NONE),
                (3, "#000000", FontStyle::NONE),
            ],
            "#ffffff",
        ),
        (
            r##"{"tokenColors":[
                {"settings":{"foreground":"#def123","background":"#123456","fontStyle":"italic"}},
                {"scope":"keyword","settings":{"foreground":"#ee5500"}},
                {"scope":"entity","settings":{"foreground":"#234567","fontStyle":"bold"}},
                {"scope":"entity.name.reset","settings":{"fontStyle":""}}
            ]}"##,
            vec![
                (0, "#ee5500", FontStyle::ITALIC),
                (3, "#def123", FontStyle::ITALIC),
                (4, "#234567", FontStyle::BOLD),
                (8, "#def123", FontStyle::ITALIC),
                (9, "#234567", FontStyle::NONE),
                (14, "#def123", FontStyle::ITALIC),
            ],
            "#123456",
        ),
        (
            r##"{"include":"base","tokenColors":[
                {"scope":"keyword","settings":{"foreground":"#ee5500"}}
            ]}"##,
            vec![
                (0, "#ee5500", FontStyle::NONE),
                (3, "#000000", FontStyle::NONE),
                (4, "#234567", FontStyle::BOLD),
                (8, "#000000", FontStyle::NONE),
                (9, "#234567", FontStyle::BOLD),
                (14, "#000000", FontStyle::NONE),
            ],
            "#ffffff",
        ),
    ];
    for (source, expected, background) in themes {
        let theme = Theme::from_json_with_resolver(source, &|_| Ok(base.to_owned())).unwrap();
        let mut colors = ColorMap::new();
        let result = grammar
            .tokenize_line2(
                "red bold reset plain",
                None,
                &theme,
                &mut colors,
                &GrammarConfiguration::default(),
            )
            .unwrap();
        assert_eq!(result.tokens.len(), expected.len() * 2, "{source}");
        for (pair, (index, foreground, font_style)) in result.tokens.chunks_exact(2).zip(expected) {
            let metadata = EncodedTokenAttributes(pair[1]);
            assert_eq!(pair[0], index);
            assert_eq!(
                colors.get_color(metadata.foreground()),
                Color::parse(foreground)
            );
            assert_eq!(
                colors.get_color(u16::from(metadata.background())),
                Color::parse(background)
            );
            assert_eq!(metadata.font_style(), font_style);
        }
    }
}

#[test]
fn theme_scope_lists_keep_whitespace_defaults_at_the_top_level() {
    for (scope, has_default) in [
        ("keyword, ,entity.name", true),
        (",,keyword, ,", true),
        ("keyword, ", true),
        ("keyword,,entity.name", false),
        ("(keyword, ,entity.name)", false),
        ("keyword, ,(entity.name, variable)", true),
    ] {
        let source = serde_json::json!({
            "tokenColors": [
                {"settings": {"foreground": "#101010", "background": "#eeeeee"}},
                {"scope": scope, "settings": {"foreground": "#334455", "fontStyle": "italic"}}
            ]
        });
        let theme = Theme::from_json(&source.to_string()).unwrap();
        let token = ["source.test".into(), "text.plain".into()];
        let expected = (
            Color::parse(if has_default { "#334455" } else { "#101010" }),
            Color::parse("#eeeeee"),
            if has_default { FontStyle::ITALIC } else { FontStyle::NONE },
        );
        for style in [theme.match_scopes(&token), theme.encoded_style(&token)] {
            assert_eq!((style.foreground, style.background, style.font_style), expected, "{scope}");
        }
        let keyword = ["source.test".into(), "keyword".into()];
        assert_eq!(theme.match_scopes(&keyword).foreground, Color::parse("#334455"), "{scope}");
    }
}

#[test]
fn theme_parent_specificity_measures_utf16_in_both_style_paths() {
    for parent in ["meta.λλ", "meta.𐐀"] {
        let source = serde_json::json!({
            "tokenColors": [
                {"scope": "meta.abc keyword", "settings": {"foreground": "#112233"}},
                {"scope": format!("{parent} keyword"), "settings": {"foreground": "#778899"}}
            ]
        });
        let theme = Theme::from_json(&source.to_string()).unwrap();
        let scopes = ["source.test", "meta.abc", parent, "keyword", "keyword"].map(str::to_owned);
        assert_eq!(theme.match_scopes(&scopes).foreground, Color::parse("#112233"));
        assert_eq!(theme.encoded_style(&scopes).foreground, Color::parse("#112233"));
    }
}

#[test]
fn encoded_theme_rules_resolve_per_scope_and_inherit_prefixes() {
    let grammar = grammar(
        r#"{"scopeName":"source.test","patterns":[
            {"match":"^q","name":"meta.outer entity.name"},
            {"match":"^r","name":"meta.outer entity.name.other"},
            {"match":"^s","name":"meta.outer entity.name entity.name"},
            {"match":"^t","name":"meta.outer meta.long meta.short entity.name entity.name"},
            {"match":"^u","name":"meta.outer meta.long meta.short entity.name"}
        ]}"#,
    );
    let base = r##"{"tokenColors":[{
        "scope":"entity.name","settings":{"foreground":"#aabbcc","background":"#b0c0d0","fontStyle":"bold"}
    }]}"##;
    let inherited = ("#aabbcc", "#b0c0d0", FontStyle::BOLD);
    let underline = ("#000000", "#ffffff", FontStyle::UNDERLINE);
    let italic = ("#000000", "#ffffff", FontStyle::ITALIC);
    let bold = ("#123456", "#ffffff", FontStyle::BOLD);
    let parent = ("#123456", "#b0c0d0", FontStyle::BOLD);
    let extended = ("#123456", "#ffffff", FontStyle::ITALIC);
    let excluded = ("#123456", "#ffffff", FontStyle::NONE);
    let cases = [
        (
            r##"{"include":"base","tokenColors":[{"scope":"entity","settings":{"fontStyle":"underline"}}]}"##,
            [inherited; 5],
        ),
        (
            r##"{"include":"base","tokenColors":[{"scope":"entity.name","settings":{"fontStyle":"underline"}}]}"##,
            [underline, inherited, underline, underline, underline],
        ),
        (
            r##"{"include":"base","tokenColors":[
            {"scope":"entity","settings":{"foreground":"#cc8800"}},
            {"scope":"entity.name","settings":{"fontStyle":"underline"}}
        ]}"##,
            [("#cc8800", "#ffffff", FontStyle::UNDERLINE); 5],
        ),
        (
            r##"{"include":"base","tokenColors":[{"scope":"meta.outer","settings":{"foreground":"#111111","fontStyle":"italic"}}]}"##,
            [inherited; 5],
        ),
        (
            r##"{"tokenColors":[
            {"scope":"entity.name, keyword","settings":{"fontStyle":"bold"}},
            {"scope":"entity.name","settings":{"background":"#b0c0d0"}},
            {"scope":"entity","settings":{"foreground":"#abcdef","fontStyle":"italic"}}
        ]}"##,
            [("#abcdef", "#b0c0d0", FontStyle::BOLD); 5],
        ),
        (
            r##"{"tokenColors":[{"scope":"entity.name","settings":{"fontStyle":"italic"}}]}"##,
            [
                italic,
                ("#000000", "#ffffff", FontStyle::NONE),
                italic,
                italic,
                italic,
            ],
        ),
        (
            r##"{"tokenColors":[
                {"scope":"entity","settings":{"foreground":"#123456","fontStyle":"bold"}},
                {"scope":"meta.short entity.name","settings":{"background":"#b0c0d0"}},
                {"scope":"meta.long entity.name","settings":{"fontStyle":"italic"}}
            ]}"##,
            // The declared parent path also applies when the target appears once.
            [bold, bold, bold, parent, parent],
        ),
        (
            r##"{"tokenColors":[
                {"scope":"entity","settings":{"foreground":"#123456"}},
                {"scope":"entity - meta.short","settings":{"fontStyle":"italic"}}
            ]}"##,
            [extended, extended, extended, excluded, excluded],
        ),
    ];
    for (source, styles) in cases {
        let theme = Theme::from_json_with_resolver(source, &|_| Ok(base.to_owned())).unwrap();
        let mut colors = ColorMap::new();
        for (line, (foreground, background, font_style)) in
            ["q", "r", "s", "t", "u"].into_iter().zip(styles)
        {
            let result = grammar
                .tokenize_line2(
                    line,
                    None,
                    &theme,
                    &mut colors,
                    &GrammarConfiguration::default(),
                )
                .unwrap();
            assert_eq!(result.tokens.len(), 2, "{source} {line}");
            assert_eq!(result.tokens[0], 0, "{source} {line}");
            let metadata = EncodedTokenAttributes(result.tokens[1]);
            assert_eq!(
                colors.get_color(metadata.foreground()),
                Color::parse(foreground),
                "{source} {line}"
            );
            assert_eq!(
                colors.get_color(u16::from(metadata.background())),
                Color::parse(background),
                "{source} {line}"
            );
            assert_eq!(metadata.font_style(), font_style, "{source} {line}");
        }
    }
}

#[test]
fn encoded_theme_redefinitions_inherit_conditional_prefix_fields() {
    let theme = Theme::from_json(
        r##"{"tokenColors":[
            {"scope":"meta.block keyword.control","settings":{"background":"#778899"}},
            {"scope":"keyword.control","settings":{"foreground":"#445566"}},
            {"scope":"meta.block keyword","settings":{"foreground":"#aabbcc","fontStyle":"italic"}},
            {"scope":"keyword","settings":{"foreground":"#112233","fontStyle":"bold"}}
        ]}"##,
    )
    .unwrap();
    for (parent, foreground, background, font_style) in [
        ("meta.block.demo", "#aabbcc", "#778899", FontStyle::ITALIC),
        ("meta.other.demo", "#445566", "#ffffff", FontStyle::BOLD),
    ] {
        let scopes = ["source.test", parent, "keyword.control.flow"].map(str::to_owned);
        let style = theme.encoded_style(&scopes);
        assert_eq!(style.foreground, Color::parse(foreground), "{parent}");
        assert_eq!(style.background, Color::parse(background), "{parent}");
        assert_eq!(style.font_style, font_style, "{parent}");
    }
}

#[test]
fn encoded_token_types_preserve_component_matches_and_embedded_reset() {
    use StandardTokenType::{Comment, Other, RegEx, String as StringToken};
    let grammar = grammar(
        r##"{"scopeName":"source.test","patterns":[{"match":"r","name":"meta.regex.token"},{"match":"s","name":"punctuation.definition.string.begin"},{"begin":"\\[","end":"\\]","name":"comment.block","patterns":[{"match":"x","name":"custom.meta.embedded.block.string"}]}]}"##,
    );
    let result = grammar
        .tokenize_line2(
            "rs[x]", None, &Theme::default(), &mut ColorMap::new(),
            &GrammarConfiguration::default(),
        )
        .unwrap();
    let types: Vec<_> = result.tokens.chunks_exact(2)
        .map(|pair| (pair[0], EncodedTokenAttributes(pair[1]).token_type()))
        .collect();
    assert_eq!(types, vec![(0, RegEx), (1, StringToken), (2, Comment), (3, Other), (4, Comment)]);
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
fn nested_capture_tokenization_preserves_the_first_emitted_ranges() {
    use serde_json::json;

    for (outer, first, second) in [
        (
            json!({"name": "outer"}),
            "source.test token inner inner.content styled.inner",
            "source.test token outer",
        ),
        (
            json!({
                "name": "outer",
                "contentName": "outer.content",
                "patterns": [{"match": "x", "name": "styled.outer"}]
            }),
            "source.test token outer outer.content styled.outer",
            "source.test token outer outer.content",
        ),
    ] {
        let source = json!({
            "scopeName": "source.test",
            "patterns": [{
                "match": "((x)y)(z)",
                "name": "token",
                "captures": {
                    "1": outer,
                    "2": {
                        "name": "inner",
                        "contentName": "inner.content",
                        "patterns": [{"match": "x", "name": "styled.inner"}]
                    },
                    "3": {"name": "tail"}
                }
            }]
        });
        let result = grammar(&source.to_string())
            .tokenize_line("xyz", None)
            .unwrap();
        let actual: Vec<_> = result
            .tokens
            .iter()
            .map(|token| (token.start_index, token.end_index, token.scopes.join(" ")))
            .collect();
        assert_eq!(
            actual,
            [
                (0, 1, first.to_owned()),
                (1, 2, second.to_owned()),
                (2, 3, "source.test token tail".to_owned()),
            ]
        );
    }
}

#[test]
fn overlapping_retokenized_capture_only_styles_its_unemitted_suffix() {
    let host = grammar(
        r##"{"scopeName":"source.test","patterns":[{"match":"(?=(xy))(xyz)","name":"token",
            "captures":{
                "1":{"name":"prefix","patterns":[{"match":"x","name":"styled.first"}]},
                "2":{"name":"whole","patterns":[{"match":"(?<=y)z","name":"styled.suffix"}]}
            }
        }]}"##,
    );
    assert_scoped_ranges(
        &host,
        "xyz",
        &[(0, 1, "styled.first"), (1, 2, "prefix"), (2, 3, "styled.suffix")],
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
    for (end_last, expected_depth) in [
        ("false", 0), ("true", 1), ("0", 0), ("1", 1), ("42", 0), ("-1", 0),
        ("null", 0), (r#""true""#, 0),
    ] {
        let source = format!(
            r##"{{"scopeName":"source.test","patterns":[{{"begin":"a","end":"x","name":"meta.a","applyEndPatternLast":{end_last},"patterns":[{{"match":"x","name":"keyword.x"}}]}}]}}"##
        );
        assert_eq!(
            grammar(&source)
                .tokenize_line("ax", None)
                .unwrap()
                .rule_stack
                .depth(),
            expected_depth,
            "applyEndPatternLast: {end_last}"
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

#[cfg(feature = "bundled-grammars")]
#[test]
fn bundled_html_recognizes_javascript_inside_script_tags() {
    let catalog = BundledCatalog::new().unwrap();
    let registry = catalog.registry().unwrap();
    let grammar = registry.load_grammar("text.html.derivative").unwrap();
    let opener = grammar.tokenize_line("<script>", None).unwrap();
    let javascript = grammar
        .tokenize_line("\twork😀();", Some(&opener.rule_stack))
        .unwrap();
    let has_script_scope = |token: &Token| token.scopes.iter().any(|scope| scope == "source.js");
    assert!(
        javascript.tokens.iter().any(|token| {
            token.start_index <= 1 && 1 < token.end_index && has_script_scope(token)
        }),
        "{:?}",
        javascript.tokens
    );
    let closer = grammar
        .tokenize_line("</script>", Some(&javascript.rule_stack))
        .unwrap();
    let html = grammar
        .tokenize_line("<p>&copy;</p>", Some(&closer.rule_stack))
        .unwrap();
    assert!(!html.tokens.iter().any(has_script_scope));
    assert!(html.tokens.iter().any(|token| {
        token
            .scopes
            .iter()
            .any(|scope| scope == "constant.character.entity.named.copy.html")
    }));
}

#[cfg(all(feature = "bundled-grammars", not(feature = "portable-regex")))]
#[test]
fn token_stream_matches_official_upstream_corpus() {
    let catalog = BundledCatalog::new().unwrap();
    let registry = catalog.registry().unwrap();
    let catalog_corpus: serde_json::Value =
        serde_json::from_str(include_str!("tests/fixtures/catalog-tokens.json")).unwrap();
    let covered: std::collections::BTreeSet<_> = catalog_corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| case["scope"].as_str().unwrap())
        .collect();
    let bundled = catalog
        .grammar_contributions()
        .iter()
        .map(|grammar| grammar.scope_name.as_str())
        .collect();
    assert_eq!(covered, bundled);

    let language_corpus: serde_json::Value =
        serde_json::from_str(include_str!("tests/fixtures/upstream-tokens.json")).unwrap();
    let cases = language_corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .chain(catalog_corpus["cases"].as_array().unwrap());
    let mut failures = Vec::new();
    for case in cases {
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

#[test]
fn includes_shared_by_many_paths_expand_once() {
    // Two includes of the next rule at each of 40 levels: 2^40 paths to one match.
    let mut repository = String::new();
    for level in 0..40 {
        let next = level + 1;
        repository.push_str(&format!(
            r##""r{level}":{{"patterns":[{{"include":"#r{next}"}},{{"include":"#r{next}"}}]}},"##
        ));
    }
    repository.push_str(r##""r40":{"match":"x","name":"keyword.x"}"##);
    let source = format!(
        r##"{{"scopeName":"source.test","patterns":[{{"include":"#r0"}}],"repository":{{{repository}}}}}"##
    );
    let result = grammar(&source).tokenize_line("x", None).unwrap();
    assert_eq!(result.tokens[0].scopes.last().unwrap(), "keyword.x");
}

#[test]
fn a_long_include_chain_does_not_use_the_stack() {
    let levels = 50_000;
    let mut repository = String::new();
    for level in 0..levels {
        let next = level + 1;
        repository.push_str(&format!(r##""r{level}":{{"include":"#r{next}"}},"##));
    }
    repository.push_str(&format!(r##""r{levels}":{{"match":"x","name":"keyword.x"}}"##));
    let source = format!(
        r##"{{"scopeName":"source.test","patterns":[{{"include":"#r0"}}],"repository":{{{repository}}}}}"##
    );
    let result = grammar(&source).tokenize_line("x", None).unwrap();
    assert_eq!(result.tokens[0].scopes.last().unwrap(), "keyword.x");
}

#[test]
fn a_deeply_nested_plist_is_an_error() {
    let levels = 200_000;
    let source = format!(
        r#"<plist version="1.0">{}{}</plist>"#,
        "<array>".repeat(levels),
        "</array>".repeat(levels)
    );
    assert!(RawGrammar::from_json(&source).is_err());
}

#[test]
fn a_partial_pass_does_not_converge_before_the_lines_it_has_not_reached() {
    let grammar = grammar(
        r##"{"scopeName":"source.test","patterns":[
            {"begin":"/\\*","end":"\\*/","name":"comment.block"},
            {"match":"x","name":"keyword.x"}]}"##,
    );
    let mut model = TokenizationModel::new(grammar);
    model.set_text(&vec!["x"; 100].join("\n"));
    assert!(!model.retokenize(100).unwrap().pending);

    model.replace_lines(0..1, [String::from("/*")]).unwrap();
    assert!(model.retokenize(10).unwrap().pending);
    model.replace_lines(5..6, [String::from("y")]).unwrap();
    assert!(!model.retokenize(100).unwrap().pending);
    let last = &model.tokens(99).unwrap().tokens[0];
    assert!(last.scopes.iter().any(|scope| scope == "comment.block"), "{:?}", last.scopes);
}

#[test]
fn a_line_over_the_work_budget_does_not_stop_the_lines_after_it() {
    let grammar = grammar(r#"{"scopeName":"source.test","patterns":[{"match":"x","name":"keyword.x"}]}"#);
    let mut model = TokenizationModel::new(grammar);
    model.set_text("xxxx\nxxxx");
    let options = TokenizeOptions { max_steps: 1, ..TokenizeOptions::default() };
    let result = model.retokenize_with_options(100, options, None).unwrap();
    assert!(result.stopped_early && !result.pending);
    assert!(model.tokens(1).is_some());
}

#[test]
fn a_keep_inside_a_lookahead_does_not_slice_backwards() {
    let grammar = grammar(
        r#"{"scopeName":"source.test","patterns":[{"match":"(?=a\\K)","name":"entity.$0"}]}"#,
    );
    // Either backend may refuse the expression; neither may panic.
    let _ = grammar.tokenize_line("a", None);
}

#[test]
fn a_bad_injection_selector_of_one_grammar_does_not_stop_the_others() {
    let mut registry = Registry::new();
    registry
        .add_grammar_json(r#"{"scopeName":"source.good","patterns":[{"match":"a","name":"keyword"}]}"#)
        .unwrap();
    registry
        .add_grammar_json(r#"{"scopeName":"text.bad","injectionSelector":"(((","patterns":[]}"#)
        .unwrap();
    let grammar = registry.load_grammar("source.good").unwrap();
    let line = grammar.tokenize_line("a", None).unwrap();
    assert!(line.tokens.iter().any(|token| token.scopes.iter().any(|scope| scope == "keyword")));
}
