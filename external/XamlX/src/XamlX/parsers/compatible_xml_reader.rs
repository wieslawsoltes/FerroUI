//! Port of `Parsers/CompatibleXmlReader.cs`.
//!
//! Upstream wraps a `System.Xml.XmlReader` and filters what `XDocument.Load` gets to see. The
//! Rust parser walks an XML tree instead of pulling reader events, so only the behavior of the
//! wrapper is ported: markup-compatibility (`mc:Ignorable`) scopes and the remapping of
//! "compatible" namespaces. The XML walker asks the questions in document order, which keeps
//! the order-dependent parts (namespaces become "known" when they are first mapped) intact.

use std::collections::{HashMap, HashSet};

pub(crate) const MARKUP_COMPATIBILITY_NAMESPACE: &str =
    "http://schemas.openxmlformats.org/markup-compatibility/2006";

pub(crate) struct CompatibleXmlReader {
    compatible: HashMap<String, String>,
    nsmap: HashMap<String, String>,
    known_namespaces: HashSet<String>,
    scope_stack: Vec<Vec<String>>,
    scope_ignore: Vec<String>,
}

impl CompatibleXmlReader {
    pub fn new(compatible: Option<&HashMap<String, String>>) -> Self {
        Self {
            compatible: compatible.cloned().unwrap_or_default(),
            nsmap: HashMap::new(),
            known_namespaces: HashSet::new(),
            scope_stack: Vec::new(),
            scope_ignore: Vec::new(),
        }
    }

    /// `PushScope(string prefixes)`: `lookup_namespace` resolves a prefix at the current element
    /// and returns the already mapped namespace.
    pub fn push_scope(
        &mut self,
        prefixes: &str,
        mut lookup_namespace: impl FnMut(&mut Self, &str) -> Option<String>,
    ) {
        let mut new_ignore = self.scope_ignore.clone();
        for prefix in prefixes.split(' ').filter(|p| !p.is_empty()) {
            if let Some(ns) = lookup_namespace(self, prefix) {
                if !new_ignore.contains(&ns) {
                    new_ignore.push(ns);
                }
            }
        }

        let previous = std::mem::replace(&mut self.scope_ignore, new_ignore);
        self.scope_stack.push(previous);
    }

    /// `PopScopeIfNeeded()`; the caller knows whether the element pushed a scope.
    pub fn pop_scope(&mut self) {
        if let Some(previous) = self.scope_stack.pop() {
            self.scope_ignore = previous;
        }
    }

    pub fn should_ignore(&self, ns: &str) -> bool {
        if ns == MARKUP_COMPATIBILITY_NAMESPACE {
            return true;
        }
        if self.known_namespaces.contains(ns) {
            return false;
        }
        self.scope_ignore.iter().any(|i| i == ns)
    }

    pub fn get_mapped(&mut self, ns: &str) -> String {
        if let Some(rv) = self.nsmap.get(ns) {
            return rv.clone();
        }

        let mapped = match self.compatible.get(ns) {
            Some(mapped) => {
                self.known_namespaces.insert(mapped.clone());
                mapped.clone()
            }
            None => ns.to_string(),
        };

        self.nsmap.insert(ns.to_string(), mapped.clone());
        mapped
    }
}
