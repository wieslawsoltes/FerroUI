//! Isolated background services share MCP's transport without exposing application inspection.
use crate::Permissions;
use serde_json::Value;
use std::collections::BTreeSet;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A request-scoped cancellation/deadline fence. Check immediately before committing durable work.
#[derive(Clone)]
pub struct RequestContext {
    pub permissions: Permissions,
    cancelled: Arc<AtomicBool>,
    stopped: Arc<AtomicBool>,
    started: Instant,
    timeout: Duration,
}
impl RequestContext {
    pub(crate) fn new(
        permissions: Permissions,
        cancelled: Arc<AtomicBool>,
        stopped: Arc<AtomicBool>,
        timeout: Duration,
    ) -> Self {
        Self {
            permissions,
            cancelled,
            stopped,
            started: Instant::now(),
            timeout,
        }
    }
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
            || self.stopped.load(Ordering::Acquire)
            || self.started.elapsed() >= self.timeout
    }
    pub fn check_cancelled(&self) -> Result<(), String> {
        if self.is_cancelled() {
            Err("Request cancelled, deadline exceeded or service stopped".into())
        } else {
            Ok(())
        }
    }
    pub fn remaining(&self) -> Duration {
        self.timeout.saturating_sub(self.started.elapsed())
    }
}

/// A service whose state is safe to access from transport workers. No UI objects may be held.
/// Tool definitions are immutable for the server lifetime. Mutating tools additionally require
/// the server's existing write permission; identity/room ACLs remain the provider's responsibility.
pub trait ServiceToolProvider: Send + Sync + 'static {
    fn tools(&self) -> Vec<Value>;
    fn validate(&self, name: &str, arguments: &Value) -> Result<(), String>;
    fn call(
        &self,
        name: &str,
        arguments: &Value,
        context: &RequestContext,
    ) -> Result<Value, String>;
}

#[derive(Clone)]
pub(crate) struct ServiceCatalog {
    provider: Arc<dyn ServiceToolProvider>,
    tools: Arc<Vec<Value>>,
}
impl ServiceCatalog {
    pub fn new(provider: Arc<dyn ServiceToolProvider>) -> io::Result<Self> {
        let tools = provider.tools();
        let mut names = BTreeSet::new();
        if tools.len() > 256 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "At most 256 service tools may be registered",
            ));
        }
        for tool in &tools {
            let name = tool["name"]
                .as_str()
                .filter(|name| {
                    !name.is_empty()
                        && name.len() <= 128
                        && name
                            .bytes()
                            .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
                })
                .ok_or_else(|| {
                    io::Error::new(io::ErrorKind::InvalidInput, "Invalid service tool name")
                })?;
            if !names.insert(name.to_owned()) || tool["inputSchema"]["type"] != "object" {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Service tools require unique names and object schemas",
                ));
            }
        }
        Ok(Self {
            provider,
            tools: Arc::new(tools),
        })
    }
    pub fn tools(&self) -> &[Value] {
        &self.tools
    }
    pub fn validate(&self, name: &str, args: &Value) -> Result<(), String> {
        let tool = self
            .tools
            .iter()
            .find(|tool| tool["name"] == name)
            .ok_or_else(|| format!("Unknown service tool: {name}"))?;
        let args = args.as_object().ok_or("arguments must be an object")?;
        let schema = &tool["inputSchema"];
        for key in schema["required"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !args.contains_key(key) {
                return Err(format!("Missing argument: {key}"));
            }
        }
        if schema["additionalProperties"] == false {
            for key in args.keys() {
                if schema["properties"].get(key).is_none() {
                    return Err(format!("Unknown argument: {key}"));
                }
            }
        }
        self.provider.validate(name, &Value::Object(args.clone()))
    }
    pub fn execute(
        &self,
        name: &str,
        args: &Value,
        context: &RequestContext,
    ) -> Result<Value, String> {
        self.validate(name, args)?;
        let definition = self
            .tools
            .iter()
            .find(|tool| tool["name"] == name)
            .expect("validated service tool");
        if definition["annotations"]["readOnlyHint"] != true
            && !context.permissions.allow_property_writes
        {
            return Err("This service tool requires allow_property_writes".into());
        }
        context.check_cancelled()?;
        self.provider.call(name, args, context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{protocol::Session, Client};
    use serde_json::json;
    struct Counter(std::sync::atomic::AtomicUsize);
    impl ServiceToolProvider for Counter {
        fn tools(&self) -> Vec<Value> {
            vec![
                json!({"name":"read","inputSchema":{"type":"object","additionalProperties":false},"annotations":{"readOnlyHint":true}}),
                json!({"name":"increment","inputSchema":{"type":"object","additionalProperties":false}}),
            ]
        }
        fn validate(&self, _: &str, _: &Value) -> Result<(), String> {
            Ok(())
        }
        fn call(&self, name: &str, _: &Value, context: &RequestContext) -> Result<Value, String> {
            context.check_cancelled()?;
            if name == "increment" {
                self.0.fetch_add(1, Ordering::SeqCst);
            }
            Ok(json!({"value":self.0.load(Ordering::SeqCst)}))
        }
    }
    fn client(write: bool) -> Client {
        Client {
            dispatcher: None,
            extension: None,
            service: Some(ServiceCatalog::new(Arc::new(Counter(0.into()))).unwrap()),
            permissions: Permissions {
                allow_property_writes: write,
                allow_input: false,
            },
            timeout: Duration::from_secs(2),
            stop: Arc::new(AtomicBool::new(false)),
        }
    }
    #[test]
    fn service_runs_without_dispatcher_and_enforces_permissions_and_cancellation() {
        let client = client(false);
        let cancel = Arc::new(AtomicBool::new(false));
        assert_eq!(client.call("read", json!({}), &cancel).unwrap()["value"], 0);
        assert!(client.call("increment", json!({}), &cancel).is_err());
        assert!(client.validate("get_tree", &json!({})).is_err());
        cancel.store(true, Ordering::Release);
        assert!(client.call("read", json!({}), &cancel).is_err());
        let writable = super::tests::client(true);
        assert_eq!(
            writable
                .call("increment", json!({}), &Arc::new(AtomicBool::new(false)))
                .unwrap()["value"],
            1
        );
    }
    #[test]
    fn service_protocol_exposes_no_inspector_resources_or_log_stream() {
        let client = client(true);
        let session = Session::default();
        let initialize=session.handle_message(&client,json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-03-26"}})).unwrap();
        assert!(initialize["result"]["capabilities"]
            .get("logging")
            .is_none());
        session.handle_message(
            &client,
            json!({"jsonrpc":"2.0","method":"notifications/initialized","params":{}}),
        );
        let resources = session
            .handle_message(
                &client,
                json!({"jsonrpc":"2.0","id":2,"method":"resources/list","params":{}}),
            )
            .unwrap();
        assert_eq!(resources["result"]["resources"], json!([]));
        let logs=session.handle_message(&client,json!({"jsonrpc":"2.0","id":3,"method":"logging/setLevel","params":{"level":"debug"}})).unwrap();
        assert!(logs.get("error").is_some());
    }
}
