//! Studio command authoring uses the same source plans as the XAML language server.
use super::{DesignerWorkspace, project::document_identity};
use crate::{DesignerSession, PreviewStatus, design_time::DesignTimeSnapshot};
use ferroui_controls::{Button, Flyout, PlacementMode};
use ferroui_designer_core::{NodeId, SourceSpan};
use ferroui_designer_language::{
    CommandBindingEdit, CommandBindingModel, CommandBindingRequest, Range,
};
use std::rc::Rc;

#[path = "workspace_command_picker.rs"]
mod picker;
#[cfg(all(test, feature = "mcp"))]
#[path = "workspace_command_bindings_tests.rs"]
mod tests;

#[derive(Clone)]
pub(crate) struct CommandSite {
    pub uri: String,
    pub request: CommandBindingRequest,
    #[cfg(feature = "mcp")]
    pub scope_epoch: u64,
    snapshot: DesignTimeSnapshot,
}

impl DesignerWorkspace {
    pub(super) fn connect_command_bindings(self: &Rc<Self>) {
        let anchor = self.root.get_control::<Button>("EditCommandBinding");
        let flyout = Flyout::new();
        flyout.set_placement(PlacementMode::Left);
        flyout.popup().set_should_use_overlay_layer(true);
        flyout.flyout_presenter_classes().add("designer-chrome");
        let workspace = Rc::downgrade(self);
        let popup = flyout.downgrade();
        flyout
            .opening(move |_| {
                if let (Some(workspace), Some(popup)) = (workspace.upgrade(), popup.upgrade()) {
                    picker::render(&workspace, &popup);
                }
            })
            .detach();
        anchor.set_flyout(Some(flyout.upcast()));
    }

    pub(super) fn refresh_command_binding(&self) {
        let session = self.session.borrow();
        let visible = !session.uses_rust_source()
            && matches!(session.selection(), [node] if session.document().ast().node(*node)
                .is_some_and(|node| node.kind == ferroui_designer_core::NodeKind::Element));
        self.root
            .get_control::<Button>("EditCommandBinding")
            .set_is_visible(visible);
    }

    pub(crate) fn command_site(self: &Rc<Self>, node: NodeId) -> Result<CommandSite, String> {
        if self.pending_source.get() {
            return Err("Apply the current source draft before editing command bindings".into());
        }
        self.synchronize_language_service()?;
        let session = self.session.borrow();
        let document = session.document();
        if !ready_xaml(&session) {
            return Err("Wait for the current XAML design before editing command bindings".into());
        }
        if session.definition_scope().is_some_and(|scope| {
            node != scope.node && !document.ast().is_descendant_of(node, scope.node)
        }) {
            return Err("Choose a control inside the current definition".into());
        }
        let target = document
            .ast()
            .node(node)
            .ok_or("Choose an authored command target")?;
        let uri = document_identity(session.document_uri())?;
        let language = self.try_language_service()?;
        let request = CommandBindingRequest {
            version: language
                .version(&uri)
                .ok_or("Index the current XAML buffer first")?,
            position: language
                .index(&uri)
                .and_then(|index| index.position(target.name_span.start))
                .map_err(|error| error.to_string())?,
            prefix: String::new(),
        };
        Ok(CommandSite {
            uri,
            request,
            #[cfg(feature = "mcp")]
            scope_epoch: session.definition_scope_epoch(),
            snapshot: DesignTimeSnapshot::capture(&session),
        })
    }

    pub(crate) fn command_model(
        &self,
        site: &CommandSite,
        prefix: String,
    ) -> Result<CommandBindingModel, String> {
        self.check_command_site(site)?;
        self.try_language_service()?
            .command_bindings(
                &site.uri,
                &CommandBindingRequest {
                    prefix,
                    ..site.request.clone()
                },
            )
            .map_err(|error| error.to_string())
    }

    pub(crate) fn apply_command_binding(
        self: &Rc<Self>,
        site: &CommandSite,
        change: &CommandBindingEdit,
    ) -> Result<(), String> {
        self.check_command_site(site)?;
        let plan = self
            .try_language_service()?
            .plan_command_binding(&site.uri, &site.request, change)
            .map_err(|error| error.to_string())?;
        if plan.document_changes.is_empty() {
            return Ok(());
        }
        self.apply_language_edits(plan)
    }

    fn reveal_command_source(&self, site: &CommandSite, range: Range) -> Result<(), String> {
        self.check_command_site(site)?;
        let span = {
            let language = self.try_language_service()?;
            language
                .index(&site.uri)
                .and_then(|index| index.span(range))
                .map_err(|error| error.to_string())?
        };
        self.select_source_span(SourceSpan::new(span.start, span.end));
        Ok(())
    }

    fn check_command_site(&self, site: &CommandSite) -> Result<(), String> {
        let session = self.session.borrow();
        if self.pending_source.get() || !ready_xaml(&session) || !site.snapshot.is_current(&session)
        {
            return Err("The source, preview, or selection changed. Reopen Command Binding".into());
        }
        Ok(())
    }
}

fn ready_xaml(session: &DesignerSession) -> bool {
    !session.uses_rust_source()
        && matches!(session.preview_status(), PreviewStatus::Ready { revision } if *revision == session.document().revision())
}
