use super::DesignerWorkspace;
use super::project::{Tab, document_identity};
use ferroui_designer_core::{Edit, NodeId, SourceSpan};
use ferroui_designer_language::{
    HandlerDraft, HandlerPlan, HandlerRequest, HandlerTarget, LineIndex,
};
#[cfg(feature = "mcp")]
use serde_json::{Value, json};
use std::rc::Rc;

#[cfg(all(test, feature = "mcp"))]
#[path = "workspace_event_handlers_tests.rs"]
mod tests;

pub(super) struct HandlerContext {
    uri: String,
    revision: u64,
    request: HandlerRequest,
    target: HandlerTarget,
    rust: Tab,
}

impl HandlerContext {
    fn draft(&self) -> HandlerDraft {
        HandlerDraft {
            uri: self.target.rust_uri.clone(),
            version: self.rust.document.revision() as i64,
            source: self.rust.source().to_owned(),
        }
    }
}

impl DesignerWorkspace {
    pub(super) fn connect_event_handlers(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.root
            .get_control::<ferroui_controls::Button>("CreateEventHandler")
            .click(move |_, _| {
                let Some(workspace) = weak.upgrade() else {
                    return;
                };
                if workspace.pending_source.get() {
                    workspace.apply_source();
                }
                let result = workspace.handler_context(None).and_then(|context| {
                    let name = context.target.suggested_name.clone();
                    let plan = workspace.commit_event_handler(context, &name)?;
                    workspace.reveal_event_declaration(&plan)
                });
                workspace.report(result);
            });
    }

    #[cfg(feature = "mcp")]
    pub(crate) fn inspect_event_handler(
        self: &Rc<Self>,
        node: NodeId,
        event: &str,
        name: Option<&str>,
    ) -> Result<Value, String> {
        let context = self.handler_context(Some((node, event)))?;
        let service = self.try_language_service()?;
        let draft = context.draft();
        let plan = name
            .map(|name| service.plan_handler(&context.uri, &context.request, &draft, name))
            .transpose()
            .map_err(|e| e.to_string())?;
        let mut result = json!(context.target);
        result["rustVersion"] = json!(draft.version);
        if let Some(plan) = plan {
            result["plan"] = json!(plan);
        } else {
            result["declaration"] = json!(
                service
                    .handler_definition(&context.uri, &context.request, Some(&draft))
                    .map_err(|e| e.to_string())?
            );
        }
        Ok(result)
    }

    #[cfg(feature = "mcp")]
    pub(crate) fn scaffold_event_handler(
        self: &Rc<Self>,
        node: NodeId,
        event: &str,
        name: &str,
        expected_rust_version: i64,
    ) -> Result<HandlerPlan, String> {
        let context = self.handler_context(Some((node, event)))?;
        if context.rust.document.revision() as i64 != expected_rust_version {
            return Err("The Rust buffer changed. Read the event-handler target again".into());
        }
        self.commit_event_handler(context, name)
    }

    fn handler_context(
        self: &Rc<Self>,
        target: Option<(NodeId, &str)>,
    ) -> Result<HandlerContext, String> {
        if self.pending_source.get() {
            return Err("Apply the current source draft before scaffolding an event".into());
        }
        self.synchronize_language_service()?;
        let (uri, revision, offset) = {
            let session = self.session.borrow();
            if session.uses_rust_source() || !session.document().is_valid() {
                return Err("Open the authored XAML view to scaffold a Rust handler".into());
            }
            let document = session.document();
            let offset = if let Some((id, _)) = target {
                document
                    .ast()
                    .node(id)
                    .ok_or("Select an authored event target")?
                    .name_span
                    .start
            } else {
                super::source_tools::byte_from_utf16(document.source(), self.source.caret_index())
            };
            (
                document_identity(session.document_uri())?,
                document.revision(),
                offset,
            )
        };
        let service = self.try_language_service()?;
        let request = HandlerRequest {
            version: service
                .version(&uri)
                .ok_or("Index the active XAML buffer first")?,
            position: service
                .index(&uri)
                .and_then(|index| index.position(offset))
                .map_err(|e| e.to_string())?,
            event: target.map(|(_, event)| event.to_owned()),
        };
        let target = service
            .handler_target(&uri, &request)
            .map_err(|e| e.to_string())?;
        drop(service);
        let rust = self.handler_tab(&target.rust_uri)?;
        Ok(HandlerContext {
            uri,
            revision,
            request,
            target,
            rust,
        })
    }

    fn handler_tab(&self, uri: &str) -> Result<Tab, String> {
        let catalog = self.rust_catalog(uri);
        let mut state = self.project.borrow_mut();
        let workspace = state
            .workspace
            .as_ref()
            .ok_or("Open the owning Cargo project first")?;
        let current = workspace
            .read_source(uri)
            .map_err(|error| error.to_string())?;
        if let Some(tab) = state
            .tabs
            .get(uri)
            .or_else(|| state.linked_history.closed(uri))
        {
            if tab.saved != current {
                return Err("The Rust file changed on disk. Reopen it before editing".into());
            }
            return Ok(tab.clone());
        }
        let (document, rust) = crate::rsx_document::RustDocument::open(current.clone(), catalog)
            .map_err(|e| e.to_string())?;
        let tab = Tab {
            preview_uri: uri.into(),
            document,
            rust: Some(rust),
            saved: current,
            file: None,
            layers: None,
            viewport: None,
            guides: None,
        };
        // Retain the exact source revision inspected by MCP or Studio until apply.
        state.tabs.insert(uri.into(), tab.clone());
        Ok(tab)
    }

    fn commit_event_handler(
        self: &Rc<Self>,
        mut context: HandlerContext,
        name: &str,
    ) -> Result<HandlerPlan, String> {
        {
            let session = self.session.borrow();
            session
                .ensure_writable()
                .map_err(|error| error.to_string())?;
            if session.collaboration_actor().is_some() {
                return Err(
                    "Use the local project workspace for an edit spanning XAML and Rust".into(),
                );
            }
            if session.document().revision() != context.revision {
                return Err("The XAML buffer changed".into());
            }
        }
        let draft = context.draft();
        let plan = self
            .try_language_service()?
            .plan_handler(&context.uri, &context.request, &draft, name)
            .map_err(|e| e.to_string())?;
        let [markup, code] = plan.edit.document_changes.as_slice() else {
            return Err("Expected a checked two-file edit".into());
        };
        let mut project = self.project.borrow_mut();
        let label = project
            .linked_history
            .label(&format!("Create {name} handler"))?;
        drop(project);
        let modified_rust = ferroui_designer_language::apply_text_edits(&draft.source, &code.edits)
            .map_err(|e| e.to_string())?;
        context
            .rust
            .rust
            .as_mut()
            .ok_or("The code-behind tab has no Rust source")?
            .update(
                &mut context.rust.document,
                modified_rust.into(),
                &label,
                false,
            )
            .map_err(|e| e.to_string())?;
        if !markup.edits.is_empty() {
            let mut session = self.session.borrow_mut();
            let source = ferroui_designer_language::apply_text_edits(
                session.document().source(),
                &markup.edits,
            )
            .map_err(|error| error.to_string())?;
            session
                .apply_at(
                    context.revision,
                    &label,
                    vec![Edit::ReplaceSource { source }],
                )
                .map_err(|error| error.to_string())?;
        }
        let mut project = self.project.borrow_mut();
        let uri = context.target.rust_uri;
        project.linked_history.take_closed(&uri);
        project.tabs.insert(uri.clone(), context.rust);
        if !markup.edits.is_empty() {
            project.linked_history.link(label, vec![context.uri, uri]);
        }
        drop(project);
        self.rebuild_document_tabs();
        self.refresh(true);
        Ok(plan)
    }

    fn reveal_event_declaration(self: &Rc<Self>, plan: &HandlerPlan) -> Result<(), String> {
        self.activate_document(&plan.declaration.uri)?;
        let span = {
            let session = self.session.borrow();
            let index = LineIndex::new(session.authored_source());
            let range = plan.declaration.range;
            SourceSpan::new(
                index.offset(range.start).map_err(|e| e.to_string())?,
                index.offset(range.end).map_err(|e| e.to_string())?,
            )
        };
        self.select_source_span(span);
        Ok(())
    }
}
