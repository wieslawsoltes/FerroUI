//! Lexical XML namespace declarations shared by parsed and transformed nodes.
use std::cell::OnceCell;
use std::collections::HashMap;
use std::rc::Rc;

/// An immutable lexical scope. Nodes without new declarations share their parent's handle;
/// child scopes store only their declarations until a namespace service needs a complete map.
#[derive(Debug)]
pub struct XamlNamespaceScope {
    parent: Option<Rc<Self>>,
    declarations: HashMap<String, String>,
    flattened: OnceCell<HashMap<String, String>>,
}

impl XamlNamespaceScope {
    pub fn new(parent: Option<Rc<Self>>, declarations: HashMap<String, String>) -> Rc<Self> {
        if declarations.is_empty() {
            if let Some(parent) = &parent {
                return parent.clone();
            }
        }
        Rc::new(Self {
            parent,
            declarations,
            flattened: OnceCell::new(),
        })
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
