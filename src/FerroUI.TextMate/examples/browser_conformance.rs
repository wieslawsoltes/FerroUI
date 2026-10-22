//! Compiled with wasm-bindgen and executed by `tests/oracle/check-wasm.mjs`.

#[cfg(target_arch = "wasm32")]
mod browser {
    use ferroui_textmate::{
        initialize_browser_oniguruma, BundledCatalog, Error, LanguageConfiguration, Registry,
        Token, TokenizationModel, WordRange,
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
        let mut model = TokenizationModel::new(grammar);
        model.set_text("{\n\"emoji\": \"😀\"\n}");
        let events = Rc::new(RefCell::new(Vec::new()));
        let sink = events.clone();
        let subscription = model.tokens_changed(move |event| sink.borrow_mut().push(event.clone()));
        let first = model.force_tokenization(1).map_err(error)?;
        if first.lines != (0..2) || !first.pending || events.borrow().len() != 1 {
            return Err(JsValue::from_str(
                "incremental model did not publish its completed prefix",
            ));
        }
        subscription.dispose();
        model.force_tokenization(2).map_err(error)?;
        if events.borrow().len() != 1 || model.tokens(2).is_none() {
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
        Ok("lazy registry, incremental model events/disposal and Unicode word lookup pass through Rust/Wasm".into())
    }

    fn error(error: Error) -> JsValue {
        JsValue::from_str(&error.to_string())
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
