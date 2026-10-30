use super::*;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::task::{Context, Wake};

#[derive(Default)]
struct FakeTransport {
    requests: RefCell<Vec<AiHttpRequest>>,
    responses: RefCell<VecDeque<Result<AiHttpResponse, StudioError>>>,
    cancel_on_send: RefCell<Option<AiCancellationToken>>,
}
impl FakeTransport {
    fn push(&self, response: AiHttpResponse) {
        self.responses.borrow_mut().push_back(Ok(response));
    }
}
impl AiTransport for FakeTransport {
    fn send<'a>(
        &'a self,
        request: AiHttpRequest,
        _: &'a AiCancellationToken,
    ) -> AiFuture<'a, AiHttpResponse> {
        self.requests.borrow_mut().push(request);
        if let Some(cancellation) = self.cancel_on_send.borrow().as_ref() {
            cancellation.cancel();
        }
        let response = self
            .responses
            .borrow_mut()
            .pop_front()
            .unwrap_or_else(|| Err(StudioError::Provider("No fake response queued".into())));
        Box::pin(async move { response })
    }
}
fn ready<F: Future>(future: F) -> F::Output {
    let mut future = Box::pin(future);
    match future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(result) => result,
        Poll::Pending => panic!("The fake transport must finish immediately"),
    }
}
fn configured(provider: AiProvider) -> AiProviderConfig {
    AiProviderConfig {
        api_key: "test-secret-only".into(),
        model_id: "selected-model".into(),
        is_enabled: true,
        ..AiProviderConfig::for_provider(provider)
    }
}
fn configured_manager(provider: AiProvider) -> (AiTranslationManager, Rc<FakeTransport>) {
    let transport = Rc::new(FakeTransport::default());
    let mut manager = AiTranslationManager::new(transport.clone());
    manager.configure(configured(provider)).unwrap();
    (manager, transport)
}
fn request(keys: &[&str]) -> AiTranslationRequest {
    AiTranslationRequest {
        source_language: "English".into(),
        target_language: "Polish".into(),
        entries: keys
            .iter()
            .map(|key| AiTranslationRequestEntry {
                key: (*key).into(),
                source_text: format!("Source {key}"),
            })
            .collect(),
        context: "Application controls".into(),
    }
}
fn response(provider: AiProvider, content: &str) -> AiHttpResponse {
    let value = match provider {
        AiProvider::OpenAI => {
            json!({"model":"observed-model","choices":[{"finish_reason":"stop","message":{"content":content}}],"usage":{"total_tokens":20}})
        }
        AiProvider::Claude => {
            json!({"model":"observed-model","stop_reason":"end_turn","content":[{"type":"text","text":content}],"usage":{"input_tokens":9,"output_tokens":11,"cache_creation_input_tokens":2,"cache_read_input_tokens":3}})
        }
        AiProvider::Gemini => {
            json!({"modelVersion":"observed-model","candidates":[{"finishReason":"STOP","content":{"parts":[{"text":content}]}}],"usageMetadata":{"promptTokenCount":8,"candidatesTokenCount":12,"thoughtsTokenCount":10,"totalTokenCount":30}})
        }
    };
    AiHttpResponse {
        status: 200,
        body: value.to_string(),
        duration_ms: 17,
    }
}
fn translation(keys: &[&str]) -> String {
    Value::Array(
        keys.iter()
            .map(|key| json!({"key":key,"text":format!("Translated {key}")}))
            .collect(),
    )
    .to_string()
}

#[test]
fn provider_protocols_encode_exact_inputs_and_decode_validated_output() {
    for provider in AiProvider::ALL {
        let (manager, transport) = configured_manager(provider);
        transport.push(response(provider, &translation(&["B", "A"])));
        let mut request = request(&["A", "B"]);
        request.entries[0].source_text = "Quoted \"text\"\nUnicode 日本語 {0:N2}".into();
        let result =
            ready(manager.translate_with_provider(provider, &request, &AiCancellationToken::new()))
                .unwrap();
        assert_eq!(
            result
                .entries
                .iter()
                .map(|entry| entry.key.as_str())
                .collect::<Vec<_>>(),
            ["A", "B"]
        );
        assert_eq!(result.model_used, "observed-model");
        assert_eq!(result.duration_ms, 17);
        assert_eq!(
            result.tokens_used,
            match provider {
                AiProvider::OpenAI => 20,
                AiProvider::Claude => 25,
                AiProvider::Gemini => 30,
            }
        );
        let sent = transport.requests.borrow();
        assert_eq!(sent.len(), 1);
        let sent = &sent[0];
        assert_eq!(sent.method, AiHttpMethod::Post);
        assert!(!format!("{sent:?}").contains("test-secret-only"));
        assert!(!sent.url.contains("test-secret-only"));
        let body: Value = serde_json::from_str(sent.body.as_ref().unwrap()).unwrap();
        let payload = match provider {
            AiProvider::OpenAI => {
                assert_eq!(sent.url, "https://api.openai.com/v1/chat/completions");
                assert_eq!(sent.headers["authorization"], "Bearer test-secret-only");
                assert_eq!(body["max_completion_tokens"], 4096);
                assert!(body.get("max_tokens").is_none());
                assert_eq!(body["messages"][0]["role"], "system");
                assert!(body["messages"][0]["content"]
                    .as_str()
                    .unwrap()
                    .contains("Application controls"));
                body["messages"][1]["content"].as_str().unwrap()
            }
            AiProvider::Claude => {
                assert_eq!(sent.url, "https://api.anthropic.com/v1/messages");
                assert_eq!(sent.headers["x-api-key"], "test-secret-only");
                assert_eq!(sent.headers["anthropic-version"], "2023-06-01");
                assert_eq!(body["max_tokens"], 4096);
                assert!(body.get("temperature").is_none());
                assert!(body["system"].as_str().unwrap().contains("Polish"));
                body["messages"][0]["content"].as_str().unwrap()
            }
            AiProvider::Gemini => {
                assert_eq!(
                    sent.url,
                    "https://generativelanguage.googleapis.com/v1beta/models/selected-model:generateContent"
                );
                assert_eq!(sent.headers["x-goog-api-key"], "test-secret-only");
                assert!(!sent.url.contains("key="));
                assert_eq!(body["generationConfig"]["maxOutputTokens"], 4096);
                assert!(body["generationConfig"].get("temperature").is_none());
                assert!(body["systemInstruction"]["parts"][0]["text"]
                    .as_str()
                    .unwrap()
                    .contains("placeholders"));
                body["contents"][0]["parts"][0]["text"].as_str().unwrap()
            }
        };
        let payload: Value = serde_json::from_str(payload).unwrap();
        assert_eq!(payload[0]["text"], request.entries[0].source_text);
        assert_eq!(payload[1]["key"], "B");
    }
}

#[test]
fn configuration_defaults_validation_and_atomic_settings_updates() {
    let config: AiProviderConfig = serde_json::from_str(r#"{"provider":"claude"}"#).unwrap();
    assert!(config.model_id.is_empty());
    assert!(config.omit_temperature);
    assert_eq!(config.base_url, AiProvider::Claude.default_base_url());
    assert_eq!(config.temperature, 0.3);
    assert_eq!(config.max_tokens, 4096);
    let config = configured(AiProvider::Gemini);
    assert!(!format!("{config:?}").contains(&config.api_key));
    assert_eq!(
        serde_json::from_str::<AiProviderConfig>(&serde_json::to_string(&config).unwrap()).unwrap(),
        config
    );
    for base_url in [
        "file:///etc/passwd",
        "https://user:secret@example.com",
        "https://example.com/?key=secret",
        "https://example.com/#fragment",
        "not a URL",
    ] {
        let mut invalid = config.clone();
        invalid.base_url = base_url.into();
        assert!(invalid.validate().is_err());
    }
    for temperature in [f64::NAN, -0.1, 2.1] {
        let mut invalid = config.clone();
        invalid.temperature = temperature;
        assert!(invalid.validate().is_err());
    }
    let (mut manager, _) = configured_manager(AiProvider::OpenAI);
    let before = manager.configuration(AiProvider::OpenAI).clone();
    let mut update = before.clone();
    update.max_tokens = 100;
    let mut invalid = configured(AiProvider::Claude);
    invalid.api_key = "bad\nheader".into();
    assert!(manager.apply_settings(&[update, invalid]).is_err());
    assert_eq!(manager.configuration(AiProvider::OpenAI), &before);
    let mut invalid = before;
    invalid.max_tokens = 0;
    assert!(manager.configure(invalid).is_err());
}

#[test]
fn endpoint_composition_preserves_proxy_paths_and_explicit_model_options() {
    let (mut manager, transport) = configured_manager(AiProvider::OpenAI);
    let mut config = configured(AiProvider::OpenAI);
    config.base_url = "https://proxy.example/provider/v1/".into();
    config.legacy_max_tokens = true;
    config.omit_temperature = true;
    manager.configure(config).unwrap();
    transport.push(response(AiProvider::OpenAI, &translation(&["A"])));
    ready(manager.translate_with_provider(
        AiProvider::OpenAI,
        &request(&["A"]),
        &AiCancellationToken::new(),
    ))
    .unwrap();
    let sent = transport.requests.borrow();
    assert_eq!(
        sent[0].url,
        "https://proxy.example/provider/v1/chat/completions"
    );
    let body: Value = serde_json::from_str(sent[0].body.as_ref().unwrap()).unwrap();
    assert_eq!(body["max_tokens"], 4096);
    assert!(body.get("max_completion_tokens").is_none());
    assert!(body.get("temperature").is_none());
}

#[test]
fn malformed_partial_duplicate_and_unrequested_translations_never_succeed() {
    for content in [
        "Not JSON",
        "[]",
        r#"[{"key":"A","text":"A"}]"#,
        r#"[{"key":"A","text":"A"},{"key":"A","text":"Again"}]"#,
        r#"[{"key":"A","text":"A"},{"key":"Other","text":"Other"}]"#,
        r#"[{"key":"A","text":"A"},{"key":"B"}]"#,
        r#"[{"key":"A","text":"A"},{"key":"B","text":"B","confidence":1.1}]"#,
        r#"[{"key":"A","key":"B","text":"A"},{"key":"B","text":"B"}]"#,
        "```json\n[]\n```\nexplanation",
    ] {
        let (manager, transport) = configured_manager(AiProvider::OpenAI);
        transport.push(response(AiProvider::OpenAI, content));
        assert!(
            ready(manager.translate_with_provider(
                AiProvider::OpenAI,
                &request(&["A", "B"]),
                &AiCancellationToken::new()
            ))
            .is_err(),
            "{content}"
        );
    }
    let (manager, transport) = configured_manager(AiProvider::OpenAI);
    transport.push(response(
        AiProvider::OpenAI,
        "an unstructured single translation",
    ));
    assert!(ready(manager.translate_single(
        AiProvider::OpenAI,
        "Hello",
        "en",
        "pl",
        "",
        &AiCancellationToken::new()
    ))
    .is_err());
}

#[test]
fn fences_empty_translations_and_split_content_blocks_are_supported() {
    let (manager, transport) = configured_manager(AiProvider::Claude);
    let mut wire: Value = serde_json::from_str(&response(AiProvider::Claude, "").body).unwrap();
    wire["content"] = json!([{"type":"thinking","thinking":"private"},{"type":"text","text":"```json\n[{\"key\":\"A\","},{"type":"text","text":"\"text\":\"\",\"confidence\":0.8}]\n```"}]);
    transport.push(AiHttpResponse {
        status: 200,
        body: wire.to_string(),
        duration_ms: 1,
    });
    let result = ready(manager.translate_with_provider(
        AiProvider::Claude,
        &request(&["A"]),
        &AiCancellationToken::new(),
    ))
    .unwrap();
    assert_eq!(result.entries[0].translated_text, "");
    assert_eq!(result.entries[0].confidence, 0.8);
    let (manager, transport) = configured_manager(AiProvider::Gemini);
    let wire = json!({"candidates":[{"finishReason":"STOP","content":{"parts":[{"thought":true,"text":"internal reasoning"},{"text":"[{\"key\":\"A\","},{"text":"\"text\":\"Translated\"}]"}]}}]});
    transport.push(AiHttpResponse {
        status: 200,
        body: wire.to_string(),
        duration_ms: 1,
    });
    assert_eq!(
        ready(manager.translate_with_provider(
            AiProvider::Gemini,
            &request(&["A"]),
            &AiCancellationToken::new()
        ))
        .unwrap()
        .entries[0]
            .translated_text,
        "Translated"
    );
}

#[test]
fn http_errors_refusals_truncation_and_invalid_usage_are_propagated() {
    for provider in AiProvider::ALL {
        let (manager, transport) = configured_manager(provider);
        transport.push(AiHttpResponse {
            status: 401,
            body: json!({"error":{"message":"Rejected test-secret-only"}}).to_string(),
            duration_ms: 1,
        });
        let error = ready(manager.translate_with_provider(
            provider,
            &request(&["A"]),
            &AiCancellationToken::new(),
        ))
        .unwrap_err()
        .to_string();
        assert!(error.contains("401"));
        assert!(error.contains("[redacted]"));
        assert!(!error.contains("test-secret-only"));
        let mut wire: Value =
            serde_json::from_str(&response(provider, &translation(&["A"])).body).unwrap();
        match provider {
            AiProvider::OpenAI => wire["choices"][0]["finish_reason"] = json!("length"),
            AiProvider::Claude => wire["stop_reason"] = json!("max_tokens"),
            AiProvider::Gemini => wire["candidates"][0]["finishReason"] = json!("SAFETY"),
        }
        transport.push(AiHttpResponse {
            status: 200,
            body: wire.to_string(),
            duration_ms: 1,
        });
        assert!(ready(manager.translate_with_provider(
            provider,
            &request(&["A"]),
            &AiCancellationToken::new()
        ))
        .is_err());
    }
    let (manager, transport) = configured_manager(AiProvider::OpenAI);
    let mut wire: Value =
        serde_json::from_str(&response(AiProvider::OpenAI, &translation(&["A"])).body).unwrap();
    wire["usage"]["total_tokens"] = json!(-1);
    transport.push(AiHttpResponse {
        status: 200,
        body: wire.to_string(),
        duration_ms: 1,
    });
    assert!(ready(manager.translate_with_provider(
        AiProvider::OpenAI,
        &request(&["A"]),
        &AiCancellationToken::new()
    ))
    .is_err());
}

#[test]
fn batches_report_stable_offsets_and_do_not_return_source_text_on_failure() {
    let (manager, transport) = configured_manager(AiProvider::OpenAI);
    transport.push(response(AiProvider::OpenAI, &translation(&["B", "A"])));
    transport.push(response(AiProvider::OpenAI, &translation(&["C"])));
    let mut updates = Vec::new();
    let result = ready(manager.translate_batches(
        AiProvider::OpenAI,
        &request(&["A", "B", "C"]),
        2,
        &AiCancellationToken::new(),
        &mut |progress, result| {
            updates.push((
                progress,
                result
                    .entries
                    .iter()
                    .map(|entry| entry.key.clone())
                    .collect::<Vec<_>>(),
            ))
        },
    ))
    .unwrap();
    assert_eq!(updates.len(), 2);
    assert_eq!(updates[0].0.start_index, 0);
    assert_eq!(updates[0].0.batch_entries, 2);
    assert_eq!(updates[1].0.start_index, 2);
    assert_eq!(updates[1].0.completed_entries, 3);
    assert_eq!(updates[0].1, ["A", "B"]);
    assert_eq!(result.entries().count(), 3);
    assert_eq!(result.tokens_used, 40);
    assert_eq!(result.duration_ms, 34);
    let empty = ready(manager.translate_batches(
        AiProvider::OpenAI,
        &request(&[]),
        1,
        &AiCancellationToken::new(),
        &mut |_, _| panic!("No empty batch callback"),
    ))
    .unwrap();
    assert!(empty.batches.is_empty());
    assert_eq!(transport.requests.borrow().len(), 2);
    assert!(ready(manager.translate_batches(
        AiProvider::OpenAI,
        &request(&["A"]),
        0,
        &AiCancellationToken::new(),
        &mut |_, _| {}
    ))
    .is_err());
}

#[test]
fn cancellation_stops_before_send_after_send_and_between_batches() {
    let (manager, transport) = configured_manager(AiProvider::OpenAI);
    let cancellation = AiCancellationToken::new();
    cancellation.cancel();
    assert_eq!(
        ready(manager.translate_with_provider(AiProvider::OpenAI, &request(&["A"]), &cancellation)),
        Err(StudioError::Cancelled)
    );
    assert!(transport.requests.borrow().is_empty());
    let cancellation = AiCancellationToken::new();
    *transport.cancel_on_send.borrow_mut() = Some(cancellation.clone());
    transport.push(response(AiProvider::OpenAI, &translation(&["A"])));
    assert_eq!(
        ready(manager.translate_with_provider(AiProvider::OpenAI, &request(&["A"]), &cancellation)),
        Err(StudioError::Cancelled)
    );
    *transport.cancel_on_send.borrow_mut() = None;
    let cancellation = AiCancellationToken::new();
    transport.push(response(AiProvider::OpenAI, &translation(&["A"])));
    let calls = Cell::new(0);
    let result = ready(manager.translate_batches(
        AiProvider::OpenAI,
        &request(&["A", "B"]),
        1,
        &cancellation,
        &mut |_, _| {
            calls.set(calls.get() + 1);
            cancellation.cancel();
        },
    ));
    assert_eq!(result, Err(StudioError::Cancelled));
    assert_eq!(calls.get(), 1);
    assert_eq!(transport.requests.borrow().len(), 2);
}

#[test]
fn cancellation_wakes_pending_transport_and_drops_its_future_without_listener_leaks() {
    struct PendingTransport(Rc<Cell<usize>>);
    struct PendingResponse(Rc<Cell<usize>>);
    impl Future for PendingResponse {
        type Output = Result<AiHttpResponse, StudioError>;
        fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
            Poll::Pending
        }
    }
    impl Drop for PendingResponse {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    impl AiTransport for PendingTransport {
        fn send<'a>(
            &'a self,
            _: AiHttpRequest,
            _: &'a AiCancellationToken,
        ) -> AiFuture<'a, AiHttpResponse> {
            Box::pin(PendingResponse(self.0.clone()))
        }
    }
    #[derive(Default)]
    struct Counter(std::sync::atomic::AtomicUsize);
    impl Wake for Counter {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
        fn wake_by_ref(self: &Arc<Self>) {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }
    let drops = Rc::new(Cell::new(0));
    let mut manager = AiTranslationManager::new(Rc::new(PendingTransport(drops.clone())));
    manager.configure(configured(AiProvider::OpenAI)).unwrap();
    let cancellation = AiCancellationToken::new();
    let request = request(&["A"]);
    let counter = Arc::new(Counter::default());
    let waker = Waker::from(counter.clone());
    let mut future =
        Box::pin(manager.translate_with_provider(AiProvider::OpenAI, &request, &cancellation));
    assert!(future
        .as_mut()
        .poll(&mut Context::from_waker(&waker))
        .is_pending());
    cancellation.cancel();
    assert_eq!(counter.0.load(Ordering::Relaxed), 1);
    assert_eq!(
        future.as_mut().poll(&mut Context::from_waker(&waker)),
        Poll::Ready(Err(StudioError::Cancelled))
    );
    drop(future);
    assert_eq!(drops.get(), 1);
    assert!(cancellation.state.listeners.lock().unwrap().is_empty());
    let cancellation = AiCancellationToken::new();
    let mut future =
        Box::pin(manager.translate_with_provider(AiProvider::OpenAI, &request, &cancellation));
    assert!(future
        .as_mut()
        .poll(&mut Context::from_waker(&waker))
        .is_pending());
    drop(future);
    assert!(cancellation.state.listeners.lock().unwrap().is_empty());
    assert_eq!(drops.get(), 2);
}

#[test]
fn model_discovery_paginates_filters_capabilities_and_propagates_failures() {
    let (manager, transport) = configured_manager(AiProvider::Gemini);
    for value in [
        json!({"models":[{"name":"models/gemini-b","supportedGenerationMethods":["generateContent"]},{"name":"models/embedding","supportedGenerationMethods":["embedContent"]}],"nextPageToken":"cursor/a+b"}),
        json!({"models":[{"name":"models/gemini-a","supportedGenerationMethods":["generateContent"]},{"name":"models/gemini-b","supportedGenerationMethods":["generateContent"]}]}),
    ] {
        transport.push(AiHttpResponse {
            status: 200,
            body: value.to_string(),
            duration_ms: 1,
        });
    }
    let list = ready(manager.list_models(AiProvider::Gemini, &AiCancellationToken::new())).unwrap();
    assert_eq!(list.models, ["gemini-a", "gemini-b"]);
    assert_eq!(list.source, AiModelListSource::Live);
    assert!(list.warning.is_none());
    let requests = transport.requests.borrow();
    let url = Url::parse(&requests[1].url).unwrap();
    assert_eq!(
        url.query_pairs()
            .find(|(key, _)| key == "pageToken")
            .unwrap()
            .1,
        "cursor/a+b"
    );
    drop(requests);
    assert!(ready(manager.list_models(AiProvider::Claude, &AiCancellationToken::new())).is_err());
    assert_eq!(transport.requests.borrow().len(), 2);
    transport.push(AiHttpResponse {
        status: 429,
        body: json!({"error":{"message":"Try later"}}).to_string(),
        duration_ms: 1,
    });
    let error =
        ready(manager.list_models(AiProvider::Gemini, &AiCancellationToken::new())).unwrap_err();
    assert!(error.to_string().contains("429"));
    let cancellation = AiCancellationToken::new();
    cancellation.cancel();
    assert_eq!(
        ready(manager.list_models(AiProvider::Gemini, &cancellation)),
        Err(StudioError::Cancelled)
    );
}

#[test]
fn claude_model_cursor_loops_are_diagnosed_and_best_provider_does_not_retry() {
    let (mut manager, transport) = configured_manager(AiProvider::Claude);
    for _ in 0..2 {
        transport.push(AiHttpResponse {
            status: 200,
            body: json!({"data":[{"id":"claude-live"}],"has_more":true,"last_id":"same"})
                .to_string(),
            duration_ms: 1,
        });
    }
    let error =
        ready(manager.list_models(AiProvider::Claude, &AiCancellationToken::new())).unwrap_err();
    assert!(error.to_string().contains("repeated"));
    assert!(transport.requests.borrow()[1].url.contains("after_id=same"));
    manager.configure(configured(AiProvider::OpenAI)).unwrap();
    transport.push(AiHttpResponse {
        status: 500,
        body: "server failed".into(),
        duration_ms: 1,
    });
    assert!(ready(
        manager.translate_with_best_available(&request(&["A"]), &AiCancellationToken::new())
    )
    .is_err());
    assert_eq!(transport.requests.borrow().len(), 3);
    assert!(transport.requests.borrow()[2].url.contains("openai.com"));
}

#[test]
fn request_validation_rejects_duplicates_without_sending_or_faking_connection_success() {
    let (manager, transport) = configured_manager(AiProvider::OpenAI);
    assert!(ready(manager.translate_with_provider(
        AiProvider::OpenAI,
        &request(&["A", "A"]),
        &AiCancellationToken::new()
    ))
    .is_err());
    assert!(transport.requests.borrow().is_empty());
    transport.push(response(
        AiProvider::OpenAI,
        r#"[{"key":"single","text":"Hola"}]"#,
    ));
    assert!(
        ready(manager.test_connection(AiProvider::OpenAI, &AiCancellationToken::new())).unwrap()
    );
    transport.push(response(AiProvider::OpenAI, "broken JSON"));
    assert!(
        ready(manager.test_connection(AiProvider::OpenAI, &AiCancellationToken::new())).is_err()
    );
}

#[test]
fn discovery_rejects_malformed_pagination_and_excludes_non_text_openai_models() {
    let (manager, transport) = configured_manager(AiProvider::OpenAI);
    transport.push(AiHttpResponse { status: 200, body: json!({"data":[{"id":"gpt-text"},{"id":"text-embedding-model"},{"id":"gpt-audio"},{"id":"gpt-image"}]}).to_string(), duration_ms: 1 });
    let list = ready(manager.list_models(AiProvider::OpenAI, &AiCancellationToken::new())).unwrap();
    assert_eq!(list.models, ["gpt-text"]);
    let (manager, transport) = configured_manager(AiProvider::Claude);
    transport.push(AiHttpResponse {
        status: 200,
        body: json!({"data":[{"id":"claude-live"}],"has_more":"yes"}).to_string(),
        duration_ms: 1,
    });
    let error =
        ready(manager.list_models(AiProvider::Claude, &AiCancellationToken::new())).unwrap_err();
    assert!(error.to_string().contains("pagination"));
    for provider in AiProvider::ALL {
        let (manager, transport) = configured_manager(provider);
        let malformed = if provider == AiProvider::Gemini {
            json!({"models":[{"name":"models/"}]})
        } else {
            json!({"data":[{"id":" "}]})
        };
        transport.push(AiHttpResponse {
            status: 200,
            body: malformed.to_string(),
            duration_ms: 1,
        });
        assert!(ready(manager.list_models(provider, &AiCancellationToken::new())).is_err());
    }
}

#[test]
fn discovery_works_before_explicit_selection_and_never_changes_configuration() {
    let (mut manager, transport) = configured_manager(AiProvider::OpenAI);
    let mut config = configured(AiProvider::OpenAI);
    config.model_id.clear();
    manager.configure(config.clone()).unwrap();
    assert!(!config.is_configured());
    assert!(manager.configured_providers().is_empty());
    let error = ready(manager.translate_with_provider(
        AiProvider::OpenAI,
        &request(&["A"]),
        &AiCancellationToken::new(),
    ))
    .unwrap_err();
    assert!(error.to_string().contains("Select a model"));
    assert!(transport.requests.borrow().is_empty());
    for models in [json!([{ "id": "live-model" }]), json!([])] {
        transport.push(AiHttpResponse {
            status: 200,
            body: json!({ "data": models }).to_string(),
            duration_ms: 1,
        });
    }
    let list = ready(manager.list_models(AiProvider::OpenAI, &AiCancellationToken::new())).unwrap();
    assert_eq!(list.models, ["live-model"]);
    let empty =
        ready(manager.list_models(AiProvider::OpenAI, &AiCancellationToken::new())).unwrap();
    assert!(empty.models.is_empty());
    assert_eq!(empty.source, AiModelListSource::Live);
    assert!(empty.warning.is_none());
    let compatibility = AiTranslationManager::compatibility_model_list(AiProvider::OpenAI);
    assert_eq!(compatibility.source, AiModelListSource::Compatibility);
    assert!(compatibility.warning.unwrap().contains("retired"));
    assert!(!compatibility.models.is_empty());
    assert_eq!(manager.configuration(AiProvider::OpenAI), &config);
    assert_eq!(transport.requests.borrow().len(), 2);
    config.model_id = "host-selected-compatible-model".into();
    config.omit_temperature = false;
    manager.configure(config).unwrap();
    assert_eq!(manager.configured_providers(), [AiProvider::OpenAI]);
    transport.push(response(AiProvider::OpenAI, &translation(&["A"])));
    ready(manager.translate_with_provider(
        AiProvider::OpenAI,
        &request(&["A"]),
        &AiCancellationToken::new(),
    ))
    .unwrap();
    let requests = transport.requests.borrow();
    let body: Value = serde_json::from_str(requests[2].body.as_ref().unwrap()).unwrap();
    assert_eq!(body["model"], "host-selected-compatible-model");
    assert_eq!(body["temperature"], 0.3);
}

#[test]
fn a_failed_later_discovery_page_never_returns_a_partial_success() {
    for provider in [AiProvider::Claude, AiProvider::Gemini] {
        let (manager, transport) = configured_manager(provider);
        let first_page = match provider {
            AiProvider::Claude => {
                json!({"data":[{"id":"discovered-model"}],"has_more":true,"last_id":"cursor"})
            }
            _ => {
                json!({"models":[{"name":"models/discovered-model","supportedGenerationMethods":["generateContent"]}],"nextPageToken":"cursor"})
            }
        };
        transport.push(AiHttpResponse {
            status: 200,
            body: first_page.to_string(),
            duration_ms: 1,
        });
        transport.push(AiHttpResponse {
            status: 503,
            body: json!({"error":{"message":"test-secret-only unavailable"}}).to_string(),
            duration_ms: 1,
        });
        let error = ready(manager.list_models(provider, &AiCancellationToken::new())).unwrap_err();
        assert!(error.to_string().contains("503"));
        assert!(!error.to_string().contains("test-secret-only"));
        assert_eq!(transport.requests.borrow().len(), 2);
    }
}

#[test]
fn cancellation_during_a_later_discovery_page_discards_partial_models() {
    struct PageTransport {
        calls: Cell<usize>,
        pending_dropped: Rc<Cell<bool>>,
    }
    struct PendingPage(Rc<Cell<bool>>);
    impl Future for PendingPage {
        type Output = Result<AiHttpResponse, StudioError>;
        fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
            Poll::Pending
        }
    }
    impl Drop for PendingPage {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }
    impl AiTransport for PageTransport {
        fn send<'a>(
            &'a self,
            _: AiHttpRequest,
            _: &'a AiCancellationToken,
        ) -> AiFuture<'a, AiHttpResponse> {
            self.calls.set(self.calls.get() + 1);
            if self.calls.get() == 1 {
                Box::pin(async {
                    Ok(AiHttpResponse {
                    status: 200,
                    body: json!({"data":[{"id":"already-discovered"}],"has_more":true,"last_id":"cursor"}).to_string(),
                    duration_ms: 1,
                })
                })
            } else {
                Box::pin(PendingPage(self.pending_dropped.clone()))
            }
        }
    }
    let transport = Rc::new(PageTransport {
        calls: Cell::new(0),
        pending_dropped: Rc::new(Cell::new(false)),
    });
    let mut manager = AiTranslationManager::new(transport.clone());
    manager.configure(configured(AiProvider::Claude)).unwrap();
    let token = AiCancellationToken::new();
    let mut future = Box::pin(manager.list_models(AiProvider::Claude, &token));
    assert!(future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
        .is_pending());
    assert_eq!(transport.calls.get(), 2);
    token.cancel();
    assert_eq!(
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop())),
        Poll::Ready(Err(StudioError::Cancelled))
    );
    assert!(transport.pending_dropped.get());
    assert_eq!(transport.calls.get(), 2);
    assert!(token.state.listeners.lock().unwrap().is_empty());
}

#[test]
fn decoding_errors_redact_keys_and_token_totals_cannot_overflow() {
    let (manager, transport) = configured_manager(AiProvider::OpenAI);
    transport.push(response(AiProvider::OpenAI, r#""test-secret-only""#));
    let error = ready(manager.translate_with_provider(
        AiProvider::OpenAI,
        &request(&["A"]),
        &AiCancellationToken::new(),
    ))
    .unwrap_err();
    assert!(!error.to_string().contains("test-secret-only"));
    let (manager, transport) = configured_manager(AiProvider::Claude);
    let mut wire: Value =
        serde_json::from_str(&response(AiProvider::Claude, &translation(&["A"])).body).unwrap();
    wire["usage"]["input_tokens"] = json!(u64::MAX);
    transport.push(AiHttpResponse {
        status: 200,
        body: wire.to_string(),
        duration_ms: 1,
    });
    let error = ready(manager.translate_with_provider(
        AiProvider::Claude,
        &request(&["A"]),
        &AiCancellationToken::new(),
    ))
    .unwrap_err();
    assert!(error.to_string().contains("overflow"));
}
