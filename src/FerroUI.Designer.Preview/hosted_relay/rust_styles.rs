//! Checked Rust style source operations against the accepted app frame.
use super::styles::{Action, Edit, RustEdit};
use crate::hosted_animation::{checked_source_patch, source_patch};
use crate::rsx_animation::AuthoredRsxFrame;
use ferroui_designer_core::TextPatch;
use ferroui_designer_language::LineIndex;
use serde_json::{json, Value};

impl AuthoredRsxFrame {
    pub(super) fn style_action(&self, action: Action) -> Result<Value, String> {
        let uri = &self.uri;
        match action {
            Action::Files => Ok(json!({"documents":[{"address":uri,"uri":uri}]})),
            Action::Catalog { address } => {
                self.require_style_source(&address, None)?;
                let catalog =
                    ferroui_designer::rust_style_report(self.source(), &self.projection()?, None)?;
                Ok(json!({"uri":uri,"catalog":catalog}))
            }
            Action::Read { target } => {
                self.require_style_source(&target.address, None)?;
                let node = usize::try_from(target.node).map_err(|error| error.to_string())?;
                let definition = ferroui_designer::rust_style_report(
                    self.source(),
                    &self.projection()?,
                    Some(node),
                )?;
                Ok(json!({"uri":uri,"definition":definition}))
            }
            Action::Plan {
                target,
                edit: Edit::Rust(RustEdit::Rust { edit }),
                source,
            } => {
                self.require_style_source(&target.address, Some(&source))?;
                if u64::try_from(edit.target()).ok() != Some(target.node) {
                    return Err(
                        "Use matching style or owner offsets from the accepted Rust frame".into(),
                    );
                }
                let patches = self.projection()?.plan_style(&edit)?;
                let updated =
                    TextPatch::apply_all(&source, patches).map_err(|error| error.to_string())?;
                let patch = checked_source_patch(&source, &updated)?.map(source_patch);
                Ok(json!({"uri":uri,"patches":patch.into_iter().collect::<Vec<_>>()}))
            }
            Action::Source { address } => {
                self.require_style_source(&address, None)?;
                Ok(json!({"uri":uri}))
            }
            Action::Locate {
                address,
                range,
                source,
            } => {
                self.require_style_source(&address, Some(&source))?;
                LineIndex::new(&source)
                    .span(range)
                    .map_err(|error| error.to_string())?;
                Ok(json!({"uri":uri,"range":range}))
            }
            Action::Plan { .. } | Action::Resources { .. } => {
                Err("Choose a mapped Rust style action for this preview".into())
            }
        }
    }

    fn require_style_source(&self, address: &str, source: Option<&str>) -> Result<(), String> {
        if address != self.uri || source.is_some_and(|source| source != self.source()) {
            return Err(
                "The Rust buffer changed. Wait for its accepted application preview.".into(),
            );
        }
        Ok(())
    }
}
