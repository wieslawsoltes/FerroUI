//! Native spacing tools for Studio and source-editing preview clients.
use super::{ensure_revision, integer, schema as s, text, DesignerMcp};
use crate::workspace_spacing::{Adjustment, Handle, SpacingModel};
use crate::{DesignerSession, LayoutEditPreview};
use ferroui_designer_core::Edit;
use ferroui_mcp::UiToolProvider;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};

const HELPERS: &str = "designer_spacing_helpers";
const DRAFT: &str = "designer_spacing_draft";
const EDIT: &str = "designer_spacing_edit";
static NEXT_MODEL: AtomicU64 = AtomicU64::new(1);

pub(super) fn handles(name: &str) -> bool {
    matches!(name, HELPERS | DRAFT | EDIT)
}

pub(super) fn tools() -> [Value; 3] {
    let adjustment = json!({"oneOf":[
        s::object(json!({"delta":s::object(json!({"x":s::number(),"y":s::number()}), &["x","y"])}), &["delta"]),
        s::object(json!({"value":s::number()}), &["value"])
    ]});
    let mut fields = json!({"model":s::id(),"handle":s::string(),"adjustment":adjustment});
    let draft = s::object(fields.clone(), &["model", "handle", "adjustment"]);
    fields["expected_revision"] = s::revision();
    [
        s::tool(HELPERS, "Measure gap, padding and margin handles for the current authored selection. Coordinates and drag deltas use artboard units.", s::object(json!({}), &[]), false),
        s::tool(DRAFT, "Validate a spacing adjustment and return its draft position and value without changing source. Use a current model with an exact value or an artboard drag delta.", draft, false),
        s::tool(EDIT, "Apply a spacing adjustment from a current model in one source transaction.", s::object(fields, &["model", "handle", "adjustment", "expected_revision"]), true),
    ]
}

#[derive(Default)]
pub(super) struct Cache {
    current: RefCell<Option<SpacingModel>>,
}

impl Cache {
    pub(super) fn call(
        &self,
        session: &mut DesignerSession,
        name: &str,
        args: &Value,
    ) -> Result<Value, String> {
        if name == HELPERS {
            if let Some(snapshot) = self
                .current
                .borrow()
                .as_ref()
                .filter(|snapshot| snapshot.is_current(session))
            {
                return Ok(snapshot.catalog(session));
            }
            let model = NEXT_MODEL.fetch_add(1, Ordering::Relaxed);
            let snapshot = SpacingModel::capture(session, model);
            let result = snapshot.catalog(session);
            self.current.replace(Some(snapshot));
            return Ok(result);
        }
        let (handle, preview) = self.plan(session, args)?;
        let changed = !preview.source_edits(session)?.is_empty();
        if name == EDIT {
            session.commit_layout_preview(&preview)?;
        }
        Ok(
            json!({"handle":handle,"description":preview.description(),"changed":changed,
            "revision":session.document().revision()}),
        )
    }

    fn plan(
        &self,
        session: &DesignerSession,
        args: &Value,
    ) -> Result<(Handle, LayoutEditPreview), String> {
        let model = integer(args, "model")?;
        let cache = self.current.borrow();
        let snapshot = cache
            .as_ref()
            .filter(|snapshot| snapshot.model == model)
            .ok_or("The spacing model changed. Refresh the handles before adjusting one")?;
        let adjustment: Adjustment = serde_json::from_value(args["adjustment"].clone())
            .map_err(|error| error.to_string())?;
        snapshot.plan(session, &text(args, "handle")?, adjustment)
    }
}

impl DesignerMcp {
    /// Drop measured geometry on a new accepted frame, including presentation-only frames.
    pub fn invalidate_spacing(&self) {
        self.spacing.current.replace(None);
    }

    /// Prepare the checked spacing transaction for an editor that owns the source and undo.
    pub fn spacing_source_plan(&self, args: &Value) -> Result<(Value, Vec<Edit>), String> {
        self.validate(EDIT, args)?;
        let owner = self.session()?;
        let session = owner
            .try_borrow()
            .map_err(|_| "Designer is already processing a transaction")?;
        ensure_revision(&session, args)?;
        let (handle, preview) = self.spacing.plan(&session, args)?;
        Ok((json!(handle), preview.source_edits(&session)?))
    }
}
