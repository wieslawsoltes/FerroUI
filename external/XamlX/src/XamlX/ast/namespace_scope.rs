//! Lexical XML namespace declarations shared by parsed and transformed nodes.
//! Deviation (DEVIATIONS.md, XAML namespace declarations): upstream resolves prefixes lexically
//! while parsing but keeps no scope on the nodes; the port keeps it for the runtime.
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use crate::parsers::XamlLoadCondition;

/// An immutable lexical scope, shared through `Arc` so that the nodes that carry it stay `Send`
/// and unwind-safe, as they were before the port kept scopes on them.
/// Nodes without new declarations share their parent's handle;
/// child scopes store only their declarations until a namespace service needs a complete map.
#[derive(Debug)]
pub struct XamlNamespaceScope {
    parent: Option<Arc<Self>>,
    declarations: HashMap<String, String>,
    flattened: OnceLock<HashMap<String, String>>,
    /// The condition of a conditional namespace that guards the nodes of this scope until the
    /// document loads (`parsers::conditional_namespace`); none for every other scope.
    load_condition: Option<XamlLoadCondition>,
}

impl XamlNamespaceScope {
    pub fn new(parent: Option<Arc<Self>>, declarations: HashMap<String, String>) -> Arc<Self> {
        if declarations.is_empty() {
            if let Some(parent) = &parent {
                return parent.clone();
            }
        }
        Arc::new(Self {
            parent,
            declarations,
            flattened: OnceLock::new(),
            load_condition: None,
        })
    }

    /// A scope with the declarations of `parent`, whose nodes are guarded by `condition`.
    pub fn with_load_condition(parent: Option<Arc<Self>>, condition: XamlLoadCondition) -> Arc<Self> {
        Arc::new(Self {
            parent,
            declarations: HashMap::new(),
            flattened: OnceLock::new(),
            load_condition: Some(condition),
        })
    }

    /// The conditions that guard the nodes of this scope, the outermost first.
    pub fn load_conditions(&self) -> Vec<&XamlLoadCondition> {
        let mut conditions = self.parent.as_ref().map(|parent| parent.load_conditions()).unwrap_or_default();
        if let Some(condition) = &self.load_condition {
            if !conditions.contains(&condition) {
                conditions.push(condition);
            }
        }
        conditions
    }

    pub fn lookup(&self, alias: &str) -> Option<&str> {
        self.declarations
            .get(alias)
            .map(String::as_str)
            .or_else(|| self.parent.as_ref().and_then(|parent| parent.lookup(alias)))
    }

    /// All declarations visible here, including inherited declarations and local shadowing.
    pub fn aliases(&self) -> &HashMap<String, String> {
        self.flattened.get_or_init(|| {
            let mut aliases = self
                .parent
                .as_ref()
                .map(|parent| parent.aliases().clone())
                .unwrap_or_default();
            aliases.extend(self.declarations.clone());
            aliases
        })
    }
}
