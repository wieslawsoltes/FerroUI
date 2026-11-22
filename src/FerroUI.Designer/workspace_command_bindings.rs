//! Studio command authoring shares checked source plans with the editor and MCP.
use super::{project::document_identity, DesignerWorkspace};
use crate::{design_time::DesignTimeSnapshot, DesignerSession, PreviewStatus};
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::{Button, Flyout, PlacementMode, TextBlock};
use ferroui_designer_core::{NodeId, SourceSpan};
use ferroui_designer_language::{
    CommandBindingEdit, CommandBindingModel, CommandBindingRequest, Range,
};
use std::rc::{Rc, Weak};

#[path = "workspace_command_picker.rs"]
mod picker;
#[path = "workspace_rust_command_picker.rs"]
mod rust_picker;
#[cfg(all(test, feature = "mcp"))]
#[path = "workspace_command_bindings_tests.rs"]
mod tests;

const APPLY: &str = "F1 M1 9L2.5 7.5 6 11 13.5 3.5 15 5 6 14Z";

#[derive(Clone)]
struct Context<Site> {
    workspace: Weak<DesignerWorkspace>,
    flyout: WeakRef<Flyout>,
    site: Site,
    status: WeakRef<TextBlock>,
}

impl<Site> Context<Site> {
    fn new(
        workspace: &Rc<DesignerWorkspace>,
        flyout: &Ref<Flyout>,
        status: &Ref<TextBlock>,
        site: Site,
    ) -> Self {
        Self {
            workspace: Rc::downgrade(workspace),
            flyout: flyout.downgrade(),
            site,
            status: status.downgrade(),
        }
    }

    fn run<T>(
        &self,
        action: impl FnOnce(&Rc<DesignerWorkspace>) -> Result<T, String>,
    ) -> Option<T> {
        match action(&self.workspace.upgrade()?) {
            Ok(result) => Some(result),
            Err(error) => {
                if let Some(status) = self.status.upgrade() {
                    status.set_text(Some(&error));
                    status.set_is_visible(true);
                }
                None
            }
        }
    }

    fn close(&self) {
        if let Some(flyout) = self.flyout.upgrade() {
            flyout.hide();
        }
    }
}

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
        let visible = matches!(session.selection(), [node] if if session.uses_rust_source() {
            session.has_rust_command_slots(*node)
        } else {
            session.document().ast().node(*node)
                .is_some_and(|node| node.kind == ferroui_designer_core::NodeKind::Element)
        });
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
