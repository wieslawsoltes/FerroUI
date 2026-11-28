use super::{RawGrammar, Registry, Theme, Token, read_json};
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    name: String,
    scope: String,
    lines: Vec<Line>,
}

#[derive(Deserialize)]
struct Line {
    text: String,
    tokens: Vec<Token>,
}

pub(super) fn verify() -> Result<usize, Box<dyn std::error::Error>> {
    let mut registry = Registry::new();
    for source in [
        include_str!("../fixtures/pinned-grammars/batchfile.tmLanguage.json"),
        include_str!("../fixtures/pinned-grammars/csharp.tmLanguage.json"),
        include_str!("../fixtures/pinned-grammars/css.tmLanguage.json"),
        include_str!("../fixtures/pinned-grammars/TypeScript.tmLanguage.json"),
    ] {
        registry.add_grammar_json(source)?;
    }
    for (scope, source) in [
        ("template.ng", include_str!("../fixtures/pinned-grammars/template.ng.json")),
        ("styles.ng", include_str!("../fixtures/pinned-grammars/styles.ng.json")),
    ] {
        // The reference locator supplies these injection names separately.
        let mut grammar = read_json(source)?;
        grammar["scopeName"] = scope.into();
        registry.add_grammar(serde_json::from_value::<RawGrammar>(grammar)?);
    }
    let cases: Vec<Case> =
        serde_json::from_str(include_str!("../fixtures/pinned-grammars/expected.json"))?;
    for case in &cases {
        registry.set_injections(&case.scope, ["template.ng".into(), "styles.ng".into()]);
    }
    let mut count = 0;
    for case in cases {
        let grammar = registry.load_grammar(&case.scope)?;
        let mut state = None;
        for (index, line) in case.lines.into_iter().enumerate() {
            let result = grammar.tokenize_line(&line.text, state.as_ref())?;
            if result.stopped_early || result.tokens != line.tokens {
                return Err(format!(
                    "{} ({}) line {}: expected {:?}, received {:?}, stopped_early={}",
                    case.name, case.scope, index + 1, line.tokens, result.tokens, result.stopped_early,
                ).into());
            }
            state = Some(result.rule_stack);
            count += 1;
        }
    }

    let theme = Theme::from_json(include_str!("../fixtures/theme-dark-vs.json"))?;
    if theme
        .style_overrides(&["support.type.property-name.css".into()])
        .foreground
        .is_none()
    {
        return Err("Dark VS has no foreground rule for the reference CSS property scope".into());
    }
    Ok(count)
}

pub(super) fn verify_scope_captures() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = Registry::new();
    registry.add_grammar_json(
        r#"{
            "scopeName": "source.capture-probe",
            "patterns": [{
                "match": "(\\.MiX)(?:!)?(empty)?",
                "name": "meta.$0 entity.$1 entity.${1:/downcase} entity.${1:/upcase} missing.$7 nested.${1:/bad-$1} invalid.${1} invalid.${1:/upcase tail.$1 optional.$2"
            }]
        }"#,
    )?;
    let grammar = registry.load_grammar("source.capture-probe")?;
    for (text, whole_match, optional) in [
        (".MiX!", "meta.MiX!", "optional."),
        (".MiXempty", "meta.MiXempty", "optional.empty"),
    ] {
        let expected = Token {
            start_index: 0,
            end_index: text.encode_utf16().count(),
            scopes: [
                "source.capture-probe",
                whole_match,
                "entity.MiX",
                "entity.mix",
                "entity.MIX",
                "missing.$7",
                "nested.${1:/bad-MiX}",
                "invalid.${1}",
                "invalid.${1:/upcase",
                "tail.MiX",
                optional,
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        };
        let result = grammar.tokenize_line(text, None)?;
        if result.stopped_early || result.tokens.as_slice() != std::slice::from_ref(&expected) {
            return Err(format!(
                "{text:?}: expected {expected:?}, received {:?}, stopped_early={}",
                result.tokens, result.stopped_early,
            )
            .into());
        }
    }
    Ok(())
}

pub(super) fn verify_capture_overrides() -> Result<(), Box<dyn std::error::Error>> {
    // The pinned reference includes the synthetic newline and adjacent ranges
    // with equal scopes. These expectations use FerroUI's normalized ranges.
    const SCOPE: &str = "source.capture-override";
    const SHARED: &str = "keyword.shared";
    const SPECIFIC: &str = "keyword.specific";
    for (kind, field, text, depth) in [
        ("end", "beginCaptures", "A", 1),
        ("end", "endCaptures", "B", 0),
        ("while", "whileCaptures", "B", 1),
    ] {
        for (override_value, expected_capture) in [
            (None, Some(SHARED)),
            (Some(serde_json::json!({})), None),
            (Some(serde_json::Value::Null), Some(SHARED)),
            (
                Some(serde_json::json!({"1": {"name": SPECIFIC}})),
                Some(SPECIFIC),
            ),
        ] {
            let mut rule = serde_json::json!({
                "begin": "^(A)", kind: "^(B)", "name": "meta.block",
                "captures": {"1": {"name": SHARED}},
            });
            if let Some(value) = &override_value {
                rule[field] = value.clone();
            }
            let source = serde_json::json!({"scopeName": SCOPE, "patterns": [rule]});
            let mut registry = Registry::new();
            registry.add_grammar_json(&source.to_string())?;
            let grammar = registry.load_grammar(SCOPE)?;
            let state = if text == "B" {
                Some(grammar.tokenize_line("A", None)?.rule_stack)
            } else {
                None
            };
            let result = grammar.tokenize_line(text, state.as_ref())?;
            let expected = Token {
                start_index: 0,
                end_index: 1,
                scopes: [SCOPE, "meta.block"]
                    .into_iter()
                    .chain(expected_capture)
                    .map(str::to_owned)
                    .collect(),
            };
            if result.stopped_early
                || result.rule_stack.depth() != depth
                || result.tokens.as_slice() != std::slice::from_ref(&expected)
            {
                return Err(format!(
                    "{field}={override_value:?}: expected {expected:?} at depth {depth}, received {:?} at depth {}, stopped_early={}",
                    result.tokens, result.rule_stack.depth(), result.stopped_early,
                )
                .into());
            }
        }
    }

    let mut registry = Registry::new();
    registry.add_grammar_json(
        r#"{
            "scopeName": "source.null-collections",
            "fileTypes": null,
            "repository": null,
            "injections": null,
            "patterns": [{
                "name": "keyword.match",
                "match": "A",
                "patterns": null,
                "repository": null,
                "captures": null
            }]
        }"#,
    )?;
    let grammar = registry.load_grammar("source.null-collections")?;
    let result = grammar.tokenize_line("A", None)?;
    let expected = Token {
        start_index: 0,
        end_index: 1,
        scopes: vec!["source.null-collections".into(), "keyword.match".into()],
    };
    if result.stopped_early || result.tokens.as_slice() != std::slice::from_ref(&expected) {
        return Err(format!("null collections: expected {expected:?}, received {result:?}").into());
    }
    Ok(())
}

pub(super) fn verify_capture_lists() -> Result<(), Box<dyn std::error::Error>> {
    let mut registry = Registry::new();
    registry.add_grammar_json(r#"{
        "scopeName":"source.test",
        "patterns":[
            {"match":"(a)(b)","captures":[{"name":"cap.first"},{"name":"cap.second"}]},
            {"begin":"(b)","end":"(e)","name":"block.end",
             "beginCaptures":[{"name":"cap.begin"}],"endCaptures":[{"name":"cap.end"}]},
            {"begin":"(s)","while":"^(w)","name":"block.while",
             "whileCaptures":[{"name":"cap.while"}]},
            {"begin":"(z)","end":"(q)","name":"block.overrides",
             "captures":[{"name":"cap.shared"}],"beginCaptures":[],"endCaptures":null}
        ]
    }"#)?;
    let grammar = registry.load_grammar("source.test")?;
    let mut previous = None;
    for (line, expected) in [
        ("ab", &["cap.first", "cap.second"][..]),
        ("b", &["cap.begin"][..]),
        ("e", &["cap.end"][..]),
        ("s", &["block.while"][..]),
        ("w", &["cap.while"][..]),
        ("x", &["source.test"][..]),
        ("z", &["block.overrides"][..]),
        ("q", &["cap.shared"][..]),
    ] {
        let result = grammar.tokenize_line(line, previous.as_ref())?;
        let ranges = result.tokens.iter()
            .map(|token| (token.start_index, token.end_index, token.scopes.last().map(String::as_str)));
        let expected = expected.iter().enumerate().map(|(index, scope)| (index, index + 1, Some(*scope)));
        if result.stopped_early || !ranges.eq(expected) {
            return Err(format!("capture lists for {line:?}: {result:?}").into());
        }
        previous = Some(result.rule_stack);
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn capture_names_preserve_missing_and_malformed_references() {
    verify_scope_captures().unwrap();
}

#[cfg(test)]
#[test]
fn explicit_capture_maps_replace_shared_captures() {
    verify_capture_overrides().unwrap();
}

#[cfg(test)]
#[test]
fn capture_lists_follow_group_order_and_preserve_overrides() {
    verify_capture_lists().unwrap();
}

#[cfg(test)]
#[test]
fn pinned_grammar_token_ranges_and_scopes_match() {
    assert_eq!(verify().unwrap(), 11);
}
