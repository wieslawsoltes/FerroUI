//! Compiled with wasm-bindgen and executed by `tests/oracle/check-wasm.mjs`.

#[cfg(target_arch = "wasm32")]
use ferroui_textmate::{RawGrammar, Registry, Theme, Token, read_json};
#[cfg(target_arch = "wasm32")]
#[path = "../tests/oracle/dynamic_delimiters.rs"]
mod dynamic_delimiters;
#[cfg(target_arch = "wasm32")]
#[path = "../tests/oracle/pinned_grammar_cases.rs"]
mod pinned_grammar_cases;
#[cfg(target_arch = "wasm32")]
#[path = "../tests/oracle/while_conditions.rs"]
mod while_conditions;
#[cfg(target_arch = "wasm32")]
#[path = "../tests/oracle/zero_width.rs"]
mod zero_width;

#[cfg(target_arch = "wasm32")]
mod browser {
    use ferroui_textmate::{
        initialize_browser_oniguruma, BundledCatalog, Error, LanguageConfiguration, LanguagePackage,
        Registry, ThemeName, Token, TokenizationModel, TokenizeOptions, WordRange,
    };
    use std::{cell::RefCell, rc::Rc};
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    pub fn verify_initialization_error() -> std::result::Result<(), JsValue> {
        let mut registry = Registry::new();
        registry
            .add_grammar_json(r#"{"scopeName":"source.test","patterns":[{"match":"x"}]}"#)
            .map_err(error)?;
        let grammar = registry.load_grammar("source.test").map_err(error)?;
        let cancelled = grammar
            .tokenize_line_with_options("x", None, TokenizeOptions::default(), Some(&|| true))
            .map_err(error)?;
        if !cancelled.stopped_early {
            return Err(JsValue::from_str("cancellation did not stop the scan"));
        }
        if !matches!(
            grammar.tokenize_line("x", None),
            Err(Error::BackendNotInitialized)
        ) {
            return Err(JsValue::from_str(
                "scanner used before initialization did not report its initialization error",
            ));
        }
        Ok(())
    }

    #[wasm_bindgen]
    pub async fn verify_host_contracts() -> std::result::Result<String, JsValue> {
        initialize_browser_oniguruma().await.map_err(error)?;
        let catalog = Rc::new(BundledCatalog::new().map_err(error)?);
        for (path, expected) in [
            ("C:\\work\\tsconfig.app.json", "jsonc"),
            ("project/.git/config", "properties"),
            ("compose.dev.yaml", "dockercompose"),
        ] {
            if catalog
                .language_by_filename(path)
                .map(|language| language.id.as_str())
                != Some(expected)
            {
                return Err(JsValue::from_str(
                    "browser filename pattern detection failed",
                ));
            }
        }
        let registry = catalog.lazy_registry().map_err(error)?;
        if registry.scope_names().count() != 0 {
            return Err(JsValue::from_str("lazy catalog loaded eagerly"));
        }
        let grammar = registry.load_grammar("source.json").map_err(error)?;
        if registry.scope_names().count() >= 79 {
            return Err(JsValue::from_str(
                "lazy catalog did not restrict the dependency closure",
            ));
        }
        let mut model = TokenizationModel::default();
        model.set_text("{\n\"emoji\": \"😀\"\n}");
        let events = Rc::new(RefCell::new(Vec::new()));
        let sink = events.clone();
        let subscription = model.tokens_changed(move |event| sink.borrow_mut().push(event.clone()));
        if model.force_tokenization(1).map_err(error)?.pending || !events.borrow().is_empty() {
            return Err(JsValue::from_str(
                "the model scheduled work before a grammar was installed",
            ));
        }
        model.set_grammar(grammar.clone());
        let first = model.force_tokenization(1).map_err(error)?;
        if first.lines != (0..2) || !first.pending || events.borrow().len() != 1 {
            return Err(JsValue::from_str(
                "incremental model did not publish its completed prefix",
            ));
        }
        model.clear_grammar();
        model.clear_grammar();
        if model.grammar().is_some()
            || model.tokens(0).is_some()
            || model.pending_line().is_some()
            || events.borrow().len() != 2
            || events.borrow()[1].ranges != [0..3]
            || events.borrow()[1].pending
        {
            return Err(JsValue::from_str(
                "clearing the grammar did not invalidate all visible tokens",
            ));
        }
        drop(subscription);
        model.set_grammar(grammar);
        model.force_tokenization(2).map_err(error)?;
        if events.borrow().len() != 2 || model.tokens(2).is_none() {
            return Err(JsValue::from_str(
                "model subscription disposal or cache completion failed",
            ));
        }
        let configuration = LanguageConfiguration::from_json(
            r#"{"wordPattern":{"pattern":"[a-z😀]+","flags":"gi"}}"#,
        )
        .map_err(error)?;
        if configuration
            .word_at_offset("ABC 😀word +", 7)
            .map_err(error)?
            != Some(WordRange {
                start_index: 4,
                end_index: 10,
            })
        {
            return Err(JsValue::from_str(
                "browser word lookup did not preserve UTF-16 coordinates",
            ));
        }
        Ok("lazy registry, filename globs, model grammar lifecycle and Unicode word lookup pass through Rust/Wasm".into())
    }

    fn error(error: Error) -> JsValue {
        JsValue::from_str(&error.to_string())
    }

    #[wasm_bindgen]
    pub async fn verify_pinned_grammar_cases() -> std::result::Result<String, JsValue> {
        initialize_browser_oniguruma().await.map_err(error)?;
        let count = super::pinned_grammar_cases::verify()
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        super::pinned_grammar_cases::verify_scope_captures()
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        super::pinned_grammar_cases::verify_capture_overrides()
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        super::pinned_grammar_cases::verify_capture_lists()
            .map_err(|error| JsValue::from_str(&error.to_string()))?;
        let while_lines = super::while_conditions::verify().map_err(|error| JsValue::from_str(&error))?;
        let zero_width_lines =
            super::zero_width::verify().map_err(|error| JsValue::from_str(&error))?;
        let delimiters =
            super::dynamic_delimiters::verify().map_err(|error| JsValue::from_str(&error))?;
        Ok(format!(
            "{count} pinned reference test lines, capture fixtures, {while_lines} while-rule lines, {zero_width_lines} zero-width lines and {delimiters} delimiter fixtures pass"
        ))
    }

    #[wasm_bindgen]
    pub async fn verify_host_package() -> std::result::Result<String, JsValue> {
        initialize_browser_oniguruma().await.map_err(error)?;
        let catalog = Rc::new(BundledCatalog::with_packages(
            ThemeName::DarkPlus,
            &[LanguagePackage {
                name: "sample",
                manifest: r#"{"contributes":{
                    "languages":[{"id":"sample","extensions":[".sample"],"configuration":"./language.json"}],
                    "grammars":[{"language":"sample","scopeName":"source.sample","path":"./syntax.json"}]
                }}"#,
                resources: &[
                    ("language.json", r#"{"wordPattern":"[a-z]+"}"#),
                    ("syntax.json", r#"{"scopeName":"source.sample","patterns":[{"match":"letter","name":"keyword.sample"}]}"#),
                ],
            }],
        ).map_err(error)?);
        let language = catalog.language_by_extension(".SAMPLE")
            .ok_or_else(|| JsValue::from_str("host package extension was not indexed"))?;
        let scope = language.scope_name.as_deref()
            .ok_or_else(|| JsValue::from_str("host package scope was not found"))?;
        let registry = catalog.clone().lazy_registry().map_err(error)?;
        let result = registry.load_grammar(scope).map_err(error)?
            .tokenize_line("letter", None).map_err(error)?;
        if language.configuration.as_ref().and_then(LanguageConfiguration::word_pattern) != Some("[a-z]+")
            || result.tokens.first().and_then(|token| token.scopes.last()).map(String::as_str) != Some("keyword.sample")
        {
            return Err(JsValue::from_str("host package did not apply its configuration and grammar"));
        }
        Ok("host-supplied language package indexes and tokenizes through Rust/Wasm".into())
    }

    #[wasm_bindgen]
    pub async fn verify_grouped_scans() -> std::result::Result<String, JsValue> {
        initialize_browser_oniguruma().await.map_err(error)?;
        let mut registry = Registry::new();
        registry
            .add_grammar_json(
                r#"{
            "scopeName":"source.groups",
            "patterns":[
                {"match":"\\Ahead","name":"first"},
                {"match":"head","name":"ordinary"},
                {"begin":"\\[","end":"\\]","name":"brackets","patterns":[
                    {"match":"\\G😀","name":"anchored"},
                    {"match":"(?<=😀)([a-z]+)\\1","name":"repeat","captures":{"1":{"name":"pair"}}},
                    {"match":"\\]","name":"child-end"}
                ]},
                {"begin":"\\(","end":"\\)","applyEndPatternLast":true,"name":"parens","patterns":[
                    {"match":"\\Gz","name":"head-z"},
                    {"match":"\\)","name":"child-end"}
                ]},
                {"match":"z+","name":"host-z"},
                {"match":"(?=!)","name":"empty"}
            ],
            "injections":{"L:source.groups":{"patterns":[
                {"match":"z","name":"injection-first"},
                {"match":"zz","name":"injection-second"}
            ]}}
        }"#,
            )
            .map_err(error)?;
        registry.add_grammar_json(r#"{"scopeName":"source.keep","patterns":[
            {"match":"a\\Kb","name":"keep"},{"match":"a","name":"first"},{"match":"b","name":"second"}
        ]}"#).map_err(error)?;
        registry
            .add_grammar_json(
                r#"{"scopeName":"source.malformed","patterns":[
            {"match":"x"},{"match":"("}
        ]}"#,
            )
            .map_err(error)?;

        let never_cancel = || false;
        for (scope, lines) in [
            ("source.groups", &["head [😀abab] zz !", "head (z) zz"][..]),
            ("source.keep", &["ab", "xabab"][..]),
        ] {
            let grammar = registry.load_grammar(scope).map_err(error)?;
            for max_steps in (0..40).chain([1000]) {
                let options = TokenizeOptions {
                    max_steps,
                    ..TokenizeOptions::default()
                };
                let mut state = None;
                for line in lines {
                    let grouped = grammar
                        .tokenize_line_with_options(line, state.as_ref(), options, None)
                        .map_err(error)?;
                    let individual = grammar
                        .tokenize_line_with_options(
                            line,
                            state.as_ref(),
                            options,
                            Some(&never_cancel),
                        )
                        .map_err(error)?;
                    if grouped.tokens != individual.tokens
                        || grouped.rule_stack != individual.rule_stack
                        || grouped.stopped_early != individual.stopped_early
                    {
                        return Err(JsValue::from_str(&format!(
                            "candidate scans differ for {line:?} with budget {max_steps}"
                        )));
                    }
                    state = Some(grouped.rule_stack);
                }
            }
        }

        let malformed = registry.load_grammar("source.malformed").map_err(error)?;
        let grouped = malformed.tokenize_line("x", None).err();
        let individual = malformed
            .tokenize_line_with_options("x", None, TokenizeOptions::default(), Some(&never_cancel))
            .err();
        if grouped != individual
            || !matches!(grouped, Some(Error::Regex { pattern, .. }) if pattern == "(")
        {
            return Err(JsValue::from_str(
                "grouped scanner lost the invalid pattern diagnostic",
            ));
        }
        Ok("grouped and individual scans agree on tokens, anchors, priority, budgets and invalid patterns".into())
    }

    #[wasm_bindgen]
    pub async fn verify_upstream_corpus(corpus: String) -> std::result::Result<String, JsValue> {
        initialize_browser_oniguruma()
            .await
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let value: serde_json::Value =
            serde_json::from_str(&corpus).map_err(|e| JsValue::from_str(&e.to_string()))?;
        let catalog = BundledCatalog::new().map_err(|e| JsValue::from_str(&e.to_string()))?;
        let registry = catalog
            .registry()
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
        let mut lines = 0;
        for case in value["cases"]
            .as_array()
            .ok_or_else(|| JsValue::from_str("missing cases"))?
        {
            let scope = case["scope"]
                .as_str()
                .ok_or_else(|| JsValue::from_str("missing scope"))?;
            let grammar = registry
                .load_grammar(scope)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;
            let mut state = None;
            for (index, line) in case["lines"]
                .as_array()
                .ok_or_else(|| JsValue::from_str("missing lines"))?
                .iter()
                .enumerate()
            {
                let text = line["text"]
                    .as_str()
                    .ok_or_else(|| JsValue::from_str("missing line text"))?;
                let result = grammar
                    .tokenize_line(text, state.as_ref())
                    .map_err(|e| JsValue::from_str(&format!("{scope} line {index}: {e}")))?;
                let expected: Vec<Token> = serde_json::from_value(line["tokens"].clone())
                    .map_err(|e| JsValue::from_str(&e.to_string()))?;
                if result.tokens != expected
                    || result.rule_stack.depth() as u64
                        != line["depth"].as_u64().unwrap_or(u64::MAX)
                {
                    return Err(JsValue::from_str(&format!("{scope} line {index}: token stream differs\nexpected: {expected:?}\nactual: {:?}", result.tokens)));
                }
                state = Some(result.rule_stack);
                lines += 1;
            }
        }
        Ok(format!(
            "{lines} lines match the upstream token stream through Rust + browser Oniguruma"
        ))
    }
}
