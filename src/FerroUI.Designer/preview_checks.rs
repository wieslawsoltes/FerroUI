use crate::{DesignerSession, PreviewStatus};
use ferroui_base::Visual;
use ferroui_designer_core::NodeId;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct PreviewFinding {
    pub node: NodeId,
    pub control: String,
    pub name: Option<String>,
    pub code: &'static str,
    pub message: String,
}

impl DesignerSession {
    /// Check the current native authoring scope and return links to mapped source nodes.
    pub fn preview_checks(&self) -> Result<Vec<PreviewFinding>, String> {
        if !matches!(self.preview_status(), PreviewStatus::Ready { revision } if *revision == self.document().revision())
        {
            return Err("Preview checks require the current accepted native frame".into());
        }
        let viewport = self.preview_host().upcast::<Visual>();
        if viewport.bounds().is_empty() {
            return Err("Preview checks require a laid-out artboard".into());
        }
        let mut findings = Vec::new();
        for node in self
            .document()
            .ast()
            .nodes_in_source_order()
            .filter(|node| node.is_element())
        {
            let Some(visual) = self.preview.visual(node.id) else {
                continue;
            };
            findings.extend(
                ferroui_inspect::check_visual(&visual, &viewport)
                    .into_iter()
                    .map(|finding| PreviewFinding {
                        node: node.id,
                        control: node.local_name().into(),
                        name: node.xaml_name().map(str::to_owned),
                        code: finding.code(),
                        message: finding.message(),
                    }),
            );
        }
        Ok(findings)
    }
}
