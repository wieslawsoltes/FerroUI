use super::DesignerWorkspace;
use crate::{
    design_handoff::{Insertion, Resolved, Review, ReviewContext},
    raster_asset_fragment, DesignerSession, OutlinePlacement, PlacementOrigin, PlacementTarget,
};
use ferroui_designer_core::NodeId;
use ferroui_designer_project::{
    design_handoff::{fingerprint, Access, ManifestFile},
    file_path,
};
use ferroui_designer_rsx::RsxCatalog;
use std::{path::Path, rc::Rc};

#[path = "workspace_design_handoff_view.rs"]
mod view;

impl DesignerWorkspace {
    pub(super) fn connect_design_handoff(self: &Rc<Self>) {
        view::connect(self);
    }

    pub(crate) fn open_handoff_review(
        self: &Rc<Self>,
        path: Option<(&Path, Access)>,
    ) -> Result<(), String> {
        view::open(self, path)
    }

    pub(crate) fn review_handoff(&self, path: &Path, access: Access) -> Result<Review, String> {
        self.with_handoff(path, access, |context| context.review())
    }

    pub(crate) fn check_handoff(
        &self,
        path: &Path,
        access: Access,
        id: &str,
        review: &str,
    ) -> Result<Resolved, String> {
        self.with_handoff(path, access, |context| context.checked(id, review))
    }

    pub(crate) fn copy_handoff(
        &self,
        path: &Path,
        access: Access,
        id: &str,
        review: &str,
    ) -> Result<String, String> {
        self.with_handoff(path, access, |context| context.copy(id, review))
    }

    pub(crate) fn place_handoff(
        &self,
        resolved: Resolved,
        revision: u64,
        node: NodeId,
        position: OutlinePlacement,
    ) -> Result<(), String> {
        if self.pending_source.get() {
            return Err("Apply the pending source before placing a mapped control".into());
        }
        let mut session = self.session.borrow_mut();
        if session.document().revision() != revision || !session.can_manipulate() {
            return Err("Choose a placement in the current design revision".into());
        }
        let source = match resolved
            .row
            .insertion
            .ok_or("Choose a mapped control or raster")?
        {
            Insertion::Asset { control } => {
                let path = file_path(session.document_uri()).map_err(|error| error.to_string())?;
                self.handoff_toolbox(&path)?
                    .into_iter()
                    .find(|asset| asset.id == control)
                    .ok_or("The control left the active Toolbox")?
                    .source
            }
            Insertion::Image { source } => {
                session.validate_raster_asset(&source)?;
                raster_asset_fragment(&source)?
            }
        };
        place(&mut session, &source, revision, node, position)
    }

    fn with_handoff<R>(
        &self,
        path: &Path,
        access: Access,
        resolve: impl FnOnce(&ReviewContext<'_>) -> Result<R, String>,
    ) -> Result<R, String> {
        let solution = self
            .solution()
            .ok_or("Open a solution to review design handoff")?;
        let manifest =
            ManifestFile::read(&solution.root, path, access).map_err(|error| error.to_string())?;
        let session = self.session.borrow();
        let uri = session.document_uri();
        let document = file_path(uri).map_err(|error| error.to_string())?;
        let active = super::project::document_identity(uri)?;
        let toolbox = self.handoff_toolbox(&document)?;
        let project = self.project.borrow();
        let draft = |uri: &str| {
            if uri == active {
                if self.pending_source.get() {
                    return Err(
                        "Apply the pending source before using the active definition".into(),
                    );
                }
                return Ok(Some(session.authored_source().into()));
            }
            Ok(project
                .tabs
                .get(uri)
                .filter(|tab| tab.source() != tab.saved)
                .map(|tab| tab.source().into()))
        };
        let registered = self.rust_catalog(uri);
        let rsx = session
            .uses_rust_source()
            .then(|| match registered.as_deref() {
                Some(catalog) => catalog,
                None => RsxCatalog::built_in(),
            });
        let revision = fingerprint(&(
            &active,
            session.document().revision(),
            session.definition_scope_epoch(),
            session.authored_source(),
            self.pending_source.get(),
        ));
        let context = ReviewContext {
            manifest: &manifest,
            document: uri,
            revision: &revision,
            toolbox: &toolbox,
            rsx,
            packaged: session.preview.packaged_assets(),
            draft: &draft,
        };
        resolve(&context)
    }
}

fn place(
    session: &mut DesignerSession,
    source: &str,
    revision: u64,
    node: NodeId,
    position: OutlinePlacement,
) -> Result<(), String> {
    let target = PlacementTarget::Outline { node, position };
    let plan = session.plan_placement(PlacementOrigin::Asset(source), target)?;
    if session.uses_rust_source() && ferroui_designer_rsx::is_child_list_edit(&plan.edits) {
        let after = matches!(position, OutlinePlacement::After).then_some(node);
        session.apply_rust_placement(revision, &plan.description, plan.edits, after, &[])
    } else {
        session.apply_visual(&plan.description, plan.edits)
    }
}
