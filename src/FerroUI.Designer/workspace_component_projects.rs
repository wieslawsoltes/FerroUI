//! Component discovery consumes indexed documents and retained editor buffers. It
//! never reloads a file behind an unsaved tab or reparses an unchanged AST snapshot.
use super::{ComponentUi, DesignerDefinitionAsset, DesignerWorkspace, LibrarySelection};
use ferroui_designer_core::{Ast, Document, NodeId};
use ferroui_designer_tools::{ComponentCatalog, ComponentKind, component_fragment};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;
use std::sync::Arc;

#[derive(Default)]
pub(super) struct ComponentDocumentIndex {
    explicit_documents: BTreeMap<String, BTreeSet<String>>,
    explicit_assets: BTreeSet<String>,
    automatic: BTreeMap<String, IndexedDocument>,
}

struct IndexedDocument {
    ast: Arc<Ast>,
    valid: bool,
    assets: BTreeSet<String>,
}

pub(super) struct PreparedComponentDocument {
    uri: String,
    document: Rc<Document>,
    assets: Vec<(DesignerDefinitionAsset, NodeId)>,
}

impl ComponentDocumentIndex {
    pub(super) fn claim_explicit_asset(&mut self, id: &str) {
        self.explicit_assets.insert(id.into());
    }
}

impl ComponentUi {
    fn remove_document_assets(&mut self, assets: BTreeSet<String>) {
        for id in assets {
            if !self.documents.explicit_assets.contains(&id) {
                self.definitions.remove(&id);
                self.sources.remove(&id);
            }
        }
    }

    fn retire_missing_library_selection(&mut self) {
        if matches!(&self.selection, Some(LibrarySelection::Definition(id)) if !self.definitions.contains_key(id))
        {
            self.selection = None;
            self.editor_open = false;
            self.create_open = false;
        }
        self.stamp = None;
    }
}

pub(super) fn prepare_component_document(
    uri: String,
    document: Rc<Document>,
    automatic: bool,
) -> Result<PreparedComponentDocument, String> {
    let assets = ComponentCatalog::from_document(&document)
        .map_err(|error| error.to_string())?
        .entries
        .into_iter()
        .filter(|entry| match entry.kind {
            ComponentKind::UserControl | ComponentKind::CustomControl => {
                entry.node == document.ast().root()
            }
            ComponentKind::ControlTheme | ComponentKind::DataTemplate => true,
            ComponentKind::ControlTemplate => entry.target_type.is_some() || entry.key.is_some(),
            ComponentKind::Style => entry
                .target_type
                .as_deref()
                .is_some_and(|selector| !selector.contains('^')),
            _ => false,
        })
        .map(|entry| {
            let source =
                component_fragment(&document, entry.node).map_err(|error| error.to_string())?;
            let asset = DesignerDefinitionAsset {
                id: format!(
                    "{}{uri}#{}",
                    if automatic { "project:" } else { "" },
                    entry.node
                ),
                label: entry.display_name,
                kind: entry.kind,
                target_type: entry.target_type,
                source,
                origin_uri: Some(uri.clone()),
            };
            // Each fragment is an element from this already validated catalog;
            // reparsing every fragment would duplicate the original parse work.
            Ok((asset, entry.node))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(PreparedComponentDocument {
        uri,
        document,
        assets,
    })
}

impl DesignerWorkspace {
    /// Publish a privately prepared source document without recursively refreshing
    /// the Library. Batch registration calls this only after every input validates.
    pub(super) fn replace_component_document_snapshot(
        &self,
        prepared: PreparedComponentDocument,
        automatic: bool,
    ) -> usize {
        let PreparedComponentDocument {
            uri,
            document,
            assets,
        } = prepared;
        let mut state = self.components.borrow_mut();
        if let Some(old) = state.documents.automatic.remove(&uri) {
            state.remove_document_assets(old.assets);
        }
        if !automatic {
            if let Some(old) = state.documents.explicit_documents.remove(&uri) {
                state.remove_document_assets(old);
            }
        }
        let mut ids = BTreeSet::new();
        for (asset, node) in assets {
            if state.documents.explicit_assets.contains(&asset.id) {
                continue;
            }
            ids.insert(asset.id.clone());
            state.sources.insert(
                asset.id.clone(),
                super::source_import::DefinitionSource {
                    document: document.clone(),
                    node,
                },
            );
            state.definitions.insert(asset.id.clone(), asset);
        }
        let count = ids.len();
        if automatic {
            state.documents.automatic.insert(
                uri,
                IndexedDocument {
                    ast: document.ast_snapshot(),
                    valid: true,
                    assets: ids,
                },
            );
        } else {
            state.documents.explicit_documents.insert(uri, ids);
        }
        state.retire_missing_library_selection();
        let browser = state.browser.clone();
        drop(state);
        if let Some(browser) = browser {
            browser.cancel_pending_drag();
        }
        count
    }

    /// Refresh from the language index, then overlay authoritative retained tabs and
    /// the active source buffer. The source snapshots retain their AST identities.
    pub(super) fn sync_project_components(&self) {
        let tools = self.source_tools.borrow();
        let project = self.project.borrow();
        let session = self.session.borrow();
        let mut documents = tools
            .service
            .uris()
            .filter_map(|uri| {
                tools
                    .service
                    .document(uri)
                    .map(|document| (uri.to_owned(), document))
            })
            .collect::<BTreeMap<_, _>>();
        for (uri, tab) in &project.tabs {
            documents.insert(uri.clone(), &tab.document);
        }
        let active = super::super::project::document_identity(session.document_uri())
            .unwrap_or_else(|_| session.document_uri().to_owned());
        documents.insert(active, session.document());
        let obsolete = self
            .components
            .borrow()
            .documents
            .automatic
            .keys()
            .filter(|uri| !documents.contains_key(*uri))
            .cloned()
            .collect::<Vec<_>>();
        for uri in obsolete {
            let mut state = self.components.borrow_mut();
            if let Some(old) = state.documents.automatic.remove(&uri) {
                state.remove_document_assets(old.assets);
                state.retire_missing_library_selection();
            }
        }
        for (uri, document) in documents {
            let ast = document.ast_snapshot();
            let valid = document.is_valid();
            let unchanged = {
                let state = self.components.borrow();
                state.documents.explicit_documents.contains_key(&uri)
                    || state
                        .documents
                        .automatic
                        .get(&uri)
                        .is_some_and(|old| old.valid == valid && Arc::ptr_eq(&old.ast, &ast))
            };
            if unchanged {
                continue;
            }
            if valid {
                let mut snapshot = document.clone();
                snapshot.set_history_limit(0);
                if let Ok(prepared) =
                    prepare_component_document(uri.clone(), Rc::new(snapshot), true)
                {
                    self.replace_component_document_snapshot(prepared, true);
                    continue;
                }
            }
            // Invalid dirty buffers must not expose a stale accepted definition as
            // current. Keep the tab and source untouched; repaired text reappears.
            let mut state = self.components.borrow_mut();
            if let Some(old) = state.documents.automatic.remove(&uri) {
                state.remove_document_assets(old.assets);
            }
            state.documents.automatic.insert(
                uri,
                IndexedDocument {
                    ast,
                    valid,
                    assets: BTreeSet::new(),
                },
            );
            state.retire_missing_library_selection();
            let browser = state.browser.clone();
            drop(state);
            if let Some(browser) = browser {
                browser.cancel_pending_drag();
            }
        }
    }
}

#[cfg(test)]
#[path = "workspace_component_projects_tests.rs"]
mod tests;
