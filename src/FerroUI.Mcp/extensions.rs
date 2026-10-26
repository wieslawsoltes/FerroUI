//! UI-thread extensions share the existing authenticated MCP transport and dispatcher.
use crate::Permissions;
use ferroui_base::threading::Dispatcher;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::rc::{Rc, Weak};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// An application-owned set of MCP tools. Every method runs on the creating UI thread.
///
/// The provider can hold `Rc`, controls, designer sessions, and other non-`Send` state. Only
/// its immutable tool descriptions and numeric registration identity cross to transport threads.
/// Descriptions are snapshotted when the server starts; restart to change the catalog. Tool
/// names must not shadow the inspector. All tools without `readOnlyHint: true` additionally
/// require `Options::permissions.allow_property_writes` before the provider is invoked.
/// Providers must describe mutating tools accurately and also check `allow_input` for any
/// tool that synthesizes input; input permission remains independent of authoring permission.
pub trait UiToolProvider {
    fn tools(&self) -> Vec<Value>;
    /// Validate typed arguments before reading or changing application state.
    fn validate(&self, name: &str, arguments: &Value) -> Result<(), String>;
    fn call(
        &self,
        name: &str,
        arguments: &Value,
        permissions: Permissions,
    ) -> Result<Value, String>;
}

ferroui_base::ferro_thread_local! {
    static PROVIDERS: RefCell<BTreeMap<u64, Weak<dyn UiToolProvider>>> = RefCell::new(BTreeMap::new());
}
static NEXT_PROVIDER: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub(crate) struct Catalog {
    id: u64,
    pub tools: Arc<Vec<Value>>,
}

/// Its `Rc` intentionally makes the hosting server handle UI-thread-owned.
pub(crate) struct Registration {
    pub catalog: Catalog,
    _provider: Rc<dyn UiToolProvider>,
}

impl Registration {
    pub fn new(provider: Rc<dyn UiToolProvider>) -> io::Result<Self> {
        Dispatcher::ui_thread().verify_access();
        let tools = provider.tools();
        let invalid = |message| io::Error::new(io::ErrorKind::InvalidInput, message);
        if tools.len() > 256 {
            return Err(invalid("An MCP extension may register at most 256 tools"));
        }
        let mut names: BTreeSet<String> = ferroui_inspect::tools()
            .iter()
            .filter_map(|tool| tool["name"].as_str().map(str::to_owned))
            .collect();
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
                .ok_or_else(|| invalid("Invalid MCP extension tool name"))?;
            if !names.insert(name.to_owned()) {
                return Err(invalid("Duplicate MCP extension or inspector tool name"));
            }
            if tool["inputSchema"]["type"] != "object" {
                return Err(invalid("MCP extension tools need an object inputSchema"));
            }
        }
        let id = NEXT_PROVIDER.fetch_add(1, Ordering::Relaxed);
        PROVIDERS.with(|providers| {
            providers.borrow_mut().insert(id, Rc::downgrade(&provider));
        });
        Ok(Self {
            catalog: Catalog {
                id,
                tools: Arc::new(tools),
            },
            _provider: provider,
        })
    }
}

impl Drop for Registration {
    fn drop(&mut self) {
        PROVIDERS.with(|providers| providers.borrow_mut().remove(&self.catalog.id));
    }
}

impl Catalog {
    pub fn contains(&self, name: &str) -> bool {
        self.tools.iter().any(|tool| tool["name"] == name)
    }

    /// The transport validates the envelope; semantic validation stays with the UI provider.
    pub fn validate_envelope(&self, name: &str, arguments: &Value) -> Result<(), String> {
        let tool = self
            .tools
            .iter()
            .find(|tool| tool["name"] == name)
            .ok_or_else(|| format!("Unknown tool: {name}"))?;
        let args = arguments.as_object().ok_or("arguments must be an object")?;
        let schema = &tool["inputSchema"];
        if let Some(required) = schema["required"].as_array() {
            for key in required.iter().filter_map(Value::as_str) {
                if !args.contains_key(key) {
                    return Err(format!("Missing argument: {key}"));
                }
            }
        }
        if schema["additionalProperties"] == false {
            for key in args.keys() {
                if schema["properties"].get(key).is_none() {
                    return Err(format!("Unknown argument: {key}"));
                }
            }
        }
        Ok(())
    }

    pub fn execute(
        &self,
        name: &str,
        arguments: &Value,
        permissions: Permissions,
    ) -> Result<Value, String> {
        Dispatcher::ui_thread().verify_access();
        self.validate_envelope(name, arguments)?;
        let definition = self.tools.iter().find(|tool| tool["name"] == name).unwrap();
        if definition["annotations"]["readOnlyHint"] != true && !permissions.allow_property_writes {
            return Err("This extension tool requires allow_property_writes".into());
        }
        let provider = PROVIDERS
            .with(|providers| providers.borrow().get(&self.id).and_then(Weak::upgrade))
            .ok_or("The application tool provider has been disposed")?;
        provider.validate(name, arguments)?;
        provider.call(name, arguments, permissions)
    }
}
