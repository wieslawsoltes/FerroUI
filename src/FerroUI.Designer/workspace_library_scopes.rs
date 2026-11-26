use super::{ComponentLibraryFilter, ComponentLibraryItem, DesignerWorkspace};
use crate::library_scopes::{LibraryObject, LibraryScope};
use crate::DesignerSession;
use ferroui_designer_core::Ast;
use ferroui_designer_tools::ComponentEntry;
use std::{rc::Rc, sync::Arc};

#[derive(Default)]
pub(super) struct ScopeCache {
    uri: String,
    revision: u64,
    scope_epoch: u64,
    ast: Option<Arc<Ast>>,
    groups: Rc<Vec<LibraryScope>>,
}

impl ScopeCache {
    pub(super) fn update(
        &mut self,
        session: &DesignerSession,
        definitions: &[ComponentEntry],
    ) -> Rc<Vec<LibraryScope>> {
        if !self.current(session) {
            self.uri = session.document_uri().to_owned();
            self.revision = session.document().revision();
            self.scope_epoch = session.definition_scope_epoch();
            self.ast = Some(session.document().ast_snapshot());
            self.groups = Rc::new(session.library_scopes(definitions).unwrap_or_default());
        }
        self.groups.clone()
    }

    fn current(&self, session: &DesignerSession) -> bool {
        self.uri == session.document_uri()
            && self.revision == session.document().revision()
            && self.scope_epoch == session.definition_scope_epoch()
            && self
                .ast
                .as_ref()
                .is_some_and(|ast| Arc::ptr_eq(ast, &session.document().ast_snapshot()))
    }

    pub(super) fn source_only(&self, id: &str) -> Option<&LibraryObject> {
        self.groups
            .iter()
            .flat_map(|scope| &scope.objects)
            .find(|object| object.id == id && object.definition.is_none())
    }
}

pub(super) fn source_items(
    scopes: &[LibraryScope],
) -> impl Iterator<Item = ComponentLibraryItem> + '_ {
    scopes.iter().flat_map(|scope| {
        scope
            .objects
            .iter()
            .filter(|object| object.definition.is_none())
            .map(|object| {
                let filter = match object.kind {
                    "Template" => ComponentLibraryFilter::Templates,
                    "Style" => ComponentLibraryFilter::Themes,
                    "Resource" => ComponentLibraryFilter::Resources,
                    "Include" => ComponentLibraryFilter::Dictionaries,
                    _ => ComponentLibraryFilter::All,
                };
                let detail = object.detail.as_ref().map_or_else(
                    || scope.path.clone(),
                    |detail| format!("{} · {detail}", scope.path),
                );
                ComponentLibraryItem {
                    id: object.id.clone(),
                    title: object.title.clone(),
                    kind: object.kind.into(),
                    detail,
                    filter,
                    node: None,
                    asset_id: None,
                    swatch: None,
                    usages: 0,
                }
            })
    })
}

impl DesignerWorkspace {
    pub(super) fn reveal_library_source(
        self: &Rc<Self>,
        item: &ComponentLibraryItem,
    ) -> Option<Result<(), String>> {
        let state = self.components.borrow();
        let object = state.scopes.source_only(&item.id)?.clone();
        {
            let session = self.session.borrow();
            if self.pending_source.get()
                || !session.authored_is_valid()
                || !state.scopes.current(&session)
            {
                return Some(Err(
                    "The owning scope changed. Reopen its Library entry.".into()
                ));
            }
        }
        drop(state);
        if let Some(uri) = &object.source_uri {
            return Some(self.reveal_rust_module_span(uri, object.source));
        }
        self.select_source_span(object.source);
        Some(Ok(()))
    }
}
