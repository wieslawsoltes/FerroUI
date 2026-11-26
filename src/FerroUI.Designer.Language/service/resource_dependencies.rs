//! Dependent diagnostics follow the same indexed XAML include graph as resource lookup.
use super::LanguageService;
use std::collections::BTreeSet;

impl LanguageService {
    pub(crate) fn uses_resource_document(&self, owner: &str, changed: &str) -> bool {
        let mut pending = vec![owner.to_owned()];
        let mut seen = BTreeSet::new();
        while let Some(uri) = pending.pop() {
            if uri == changed {
                return true;
            }
            if !seen.insert(uri.clone()) {
                continue;
            }
            if let Some(state) = self.documents.get(&uri) {
                pending.extend(
                    state
                        .resource_edges
                        .values()
                        .flatten()
                        .filter_map(|edge| edge.source.as_deref())
                        .filter_map(|source| self.resource_uri(&uri, source).ok()),
                );
            }
        }
        false
    }
}
