use super::*;
use serde_json::json;
use std::cell::Cell;
use std::rc::Rc;

struct CounterTools(Rc<Cell<u64>>);
impl UiToolProvider for CounterTools {
    fn tools(&self) -> Vec<Value> {
        vec![json!({
            "name":"designer_counter", "description":"Test UI-thread-owned extension",
            "inputSchema":{"type":"object","properties":{"amount":{"type":"integer"}},"required":["amount"],"additionalProperties":false},
            "annotations":{"readOnlyHint":false}
        })]
    }
    fn validate(&self, _: &str, args: &Value) -> Result<(), String> {
        Dispatcher::ui_thread().verify_access();
        if args["amount"].as_u64().is_some_and(|amount| amount <= 10) {
            Ok(())
        } else {
            Err("amount must be an integer from 0 through 10".into())
        }
    }
    fn call(&self, _: &str, args: &Value, _: Permissions) -> Result<Value, String> {
        Dispatcher::ui_thread().verify_access();
        self.0.set(self.0.get() + args["amount"].as_u64().unwrap());
        Ok(json!({"value":self.0.get()}))
    }
}

fn invoke(client: Client, args: Value) -> Result<Value, String> {
    invoke_tool(client, "designer_counter", args)
}
fn invoke_tool(client: Client, name: &str, args: Value) -> Result<Value, String> {
    let name = name.to_owned();
    let worker =
        std::thread::spawn(move || client.call(&name, args, &Arc::new(AtomicBool::new(false))));
    while !worker.is_finished() {
        Dispatcher::ui_thread().run_jobs(None);
        std::thread::sleep(Duration::from_millis(1));
    }
    worker.join().unwrap()
}
fn settings(write: bool) -> Options {
    Options {
        capture_logs: false,
        permissions: Permissions {
            allow_property_writes: write,
            allow_input: false,
        },
        ..Options::default()
    }
}

#[test]
fn inspector_only_server_handle_preserves_send_contract() {
    fn assert_send<T: Send>() {}
    assert_send::<ServerHandle>();
}

#[test]
fn extension_is_scoped_to_server_and_uses_ui_thread_validation_and_permissions() {
    let _scope = Dispatcher::unit_test_scope();
    let a = Rc::new(Cell::new(0));
    let b = Rc::new(Cell::new(100));
    let first = serve_with_tools(settings(true), Rc::new(CounterTools(a.clone()))).unwrap();
    let second = serve_with_tools(settings(true), Rc::new(CounterTools(b.clone()))).unwrap();
    let denied = serve_with_tools(settings(false), Rc::new(CounterTools(a.clone()))).unwrap();
    assert_eq!(
        first.client.tools().len(),
        ferroui_inspect::tools().len() + 1
    );
    assert_eq!(
        invoke(first.client.clone(), json!({"amount":3})).unwrap()["value"],
        3
    );
    assert_eq!(
        invoke(second.client.clone(), json!({"amount":4})).unwrap()["value"],
        104
    );
    assert!(invoke(denied.client.clone(), json!({"amount":1}))
        .unwrap_err()
        .contains("allow_property_writes"));
    assert!(invoke(first.client.clone(), json!({"amount":11}))
        .unwrap_err()
        .contains("integer"));
    assert!(first
        .client
        .validate("designer_counter", &json!({"amount":1,"unknown":true}))
        .is_err());
    assert_eq!(a.get(), 3);
    assert_eq!(b.get(), 104);
    let builtin = serve(settings(false)).unwrap();
    assert!(builtin
        .client
        .validate("designer_counter", &json!({"amount":1}))
        .is_err());
}

#[test]
fn stopping_extension_server_cancels_queued_mutations() {
    let _scope = Dispatcher::unit_test_scope();
    let value = Rc::new(Cell::new(0));
    let server = serve_with_tools(settings(true), Rc::new(CounterTools(value.clone()))).unwrap();
    let client = server.client.clone();
    let worker = std::thread::spawn(move || {
        client.call(
            "designer_counter",
            json!({"amount":5}),
            &Arc::new(AtomicBool::new(false)),
        )
    });
    let started = Instant::now();
    while !Dispatcher::ui_thread().has_jobs_with_priority(DispatcherPriority::BACKGROUND) {
        assert!(!worker.is_finished() && started.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(1));
    }
    drop(server);
    assert!(worker.join().unwrap().is_err());
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(value.get(), 0);
}

#[test]
fn extension_catalog_cannot_override_inspector_contracts() {
    let _scope = Dispatcher::unit_test_scope();
    struct Shadow;
    impl UiToolProvider for Shadow {
        fn tools(&self) -> Vec<Value> {
            ferroui_inspect::tools()
        }
        fn validate(&self, _: &str, _: &Value) -> Result<(), String> {
            Ok(())
        }
        fn call(&self, _: &str, _: &Value, _: Permissions) -> Result<Value, String> {
            unreachable!()
        }
    }
    assert!(serve_with_tools(settings(true), Rc::new(Shadow)).is_err());
}

#[test]
fn readonly_extensions_keep_application_timeouts_and_independent_permissions() {
    let _scope = Dispatcher::unit_test_scope();
    struct ReadTools;
    impl UiToolProvider for ReadTools {
        fn tools(&self) -> Vec<Value> {
            vec![json!({
                "name":"application_read",
                "inputSchema":{"type":"object","properties":{"timeout_ms":{"type":"integer"}},"required":["timeout_ms"],"additionalProperties":false},
                "annotations":{"readOnlyHint":true}
            })]
        }
        fn validate(&self, _: &str, args: &Value) -> Result<(), String> {
            Dispatcher::ui_thread().verify_access();
            args["timeout_ms"]
                .as_u64()
                .map(|_| ())
                .ok_or_else(|| "timeout_ms must be an integer".into())
        }
        fn call(&self, _: &str, args: &Value, permissions: Permissions) -> Result<Value, String> {
            Dispatcher::ui_thread().verify_access();
            Ok(
                json!({"timeout_ms":args["timeout_ms"],"writes":permissions.allow_property_writes,"input":permissions.allow_input}),
            )
        }
    }
    let server = serve_with_tools(
        Options {
            permissions: Permissions {
                allow_property_writes: false,
                allow_input: true,
            },
            ..settings(false)
        },
        Rc::new(ReadTools),
    )
    .unwrap();
    let result = invoke_tool(
        server.client.clone(),
        "application_read",
        json!({"timeout_ms":u64::MAX}),
    )
    .unwrap();
    assert_eq!(result["timeout_ms"], u64::MAX);
    assert_eq!(result["writes"], false);
    assert_eq!(result["input"], true);
    assert!(invoke_tool(
        server.client.clone(),
        "set_property",
        json!({"node_id":1,"property":"Width","value":100})
    )
    .unwrap_err()
    .contains("disabled"));
}

#[test]
fn a_disposed_registration_never_routes_to_a_new_provider() {
    let _scope = Dispatcher::unit_test_scope();
    let a = Rc::new(Cell::new(0));
    let provider = Rc::new(CounterTools(a.clone()));
    let first = extensions::Registration::new(provider.clone()).unwrap();
    let old_catalog = first.catalog.clone();
    let permissions = Permissions {
        allow_property_writes: true,
        allow_input: false,
    };
    assert_eq!(
        old_catalog
            .execute("designer_counter", &json!({"amount":2}), permissions)
            .unwrap()["value"],
        2
    );
    drop(first);
    // Keeping the original application provider alive must not keep its registration callable.
    let b = Rc::new(Cell::new(100));
    let second = extensions::Registration::new(Rc::new(CounterTools(b.clone()))).unwrap();
    assert!(old_catalog
        .execute("designer_counter", &json!({"amount":3}), permissions)
        .unwrap_err()
        .contains("disposed"));
    assert_eq!(
        second
            .catalog
            .execute("designer_counter", &json!({"amount":4}), permissions)
            .unwrap()["value"],
        104
    );
    assert_eq!(a.get(), 2);
    assert_eq!(Rc::strong_count(&provider), 1);
}

#[test]
fn cancelling_a_queued_extension_never_applies_it_when_dispatcher_resumes() {
    let _scope = Dispatcher::unit_test_scope();
    let value = Rc::new(Cell::new(0));
    let server = serve_with_tools(settings(true), Rc::new(CounterTools(value.clone()))).unwrap();
    let client = server.client.clone();
    let cancelled = Arc::new(AtomicBool::new(false));
    let worker_cancelled = cancelled.clone();
    let worker = std::thread::spawn(move || {
        client.call("designer_counter", json!({"amount":7}), &worker_cancelled)
    });
    let started = Instant::now();
    while !Dispatcher::ui_thread().has_jobs_with_priority(DispatcherPriority::BACKGROUND) {
        assert!(!worker.is_finished() && started.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(1));
    }
    cancelled.store(true, Ordering::Release);
    assert!(worker.join().unwrap().unwrap_err().contains("cancelled"));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(value.get(), 0);
    assert_eq!(
        invoke(server.client.clone(), json!({"amount":1})).unwrap()["value"],
        1
    );
}
