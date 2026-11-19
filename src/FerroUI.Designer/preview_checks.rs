use crate::runtime::RuntimePreview;
use crate::{DesignerSession, PreviewStatus};
use ferroui_base::{Ref, Visual};
use ferroui_designer_core::{Document, Node, NodeId, NodeKind, SourceSpan};
use ferroui_designer_rsx::RsxSourceMap;
use ferroui_designer_tools::authored_layout_property_span;
use ferroui_inspect::{MeasuredOverflow, check_layout_values, check_visual_with_layout};
use serde::Serialize;

#[cfg(test)]
#[path = "preview_checks_tests.rs"]
mod tests;

#[derive(Clone, Debug, Serialize)]
pub struct PreviewFinding {
    pub node: NodeId,
    pub control: String,
    pub name: Option<String>,
    pub code: &'static str,
    pub message: String,
    /// A compact caption for a docked list or narrow stage picker.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Canonical name of the authored layout field for value findings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property: Option<&'static str>,
    /// Byte range in the checked source. Rust hosts map this through the accepted invocation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<SourceSpan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub overflow: Option<PreviewOverflow>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PreviewOverflow {
    pub boundary: String,
    pub left: f64,
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
}

impl From<MeasuredOverflow> for PreviewOverflow {
    fn from(value: MeasuredOverflow) -> Self {
        Self {
            boundary: value.boundary,
            left: value.left,
            top: value.top,
            right: value.right,
            bottom: value.bottom,
        }
    }
}

impl PreviewFinding {
    /// Resolve the same finding to the source of a statically mapped Rust preview.
    pub fn map_rsx_source(&mut self, document: &Document, map: &RsxSourceMap) {
        self.source = document.ast().node(self.node).and_then(|node| {
            let offset = node.span.start;
            match self.property {
                Some(property) => map.property(offset, property),
                None => map.control(offset).map(|(_, name)| name),
            }
        });
    }

    fn for_node(node: &Node, code: &'static str, message: String) -> Self {
        Self {
            node: node.id,
            control: node.local_name().into(),
            name: node.xaml_name().map(str::to_owned),
            code,
            message,
            summary: None,
            property: None,
            source: Some(node.name_span),
            overflow: None,
        }
    }
}

impl DesignerSession {
    /// Check the current native authoring scope and return links to mapped source nodes.
    pub fn preview_checks(&self) -> Result<Vec<PreviewFinding>, String> {
        let viewport = self.preview_host().upcast::<Visual>();
        let mut findings = check_preview(self.document(), &self.preview, &viewport)?;
        if let Some(map) = self.rust_map() {
            for finding in &mut findings {
                finding.map_rsx_source(self.document(), map);
            }
        }
        Ok(findings)
    }
}

pub(crate) fn check_preview(
    document: &Document,
    preview: &RuntimePreview,
    viewport: &Ref<Visual>,
) -> Result<Vec<PreviewFinding>, String> {
    if !matches!(preview.status, PreviewStatus::Ready { revision } if revision == document.revision())
    {
        return Err("Preview checks require the current accepted native frame".into());
    }
    if viewport.bounds().is_empty() || !viewport.is_effectively_visible() {
        return Err("Preview checks require a visible laid-out artboard".into());
    }
    let mut findings = Vec::new();
    for node in document
        .ast()
        .nodes_in_source_order()
        .filter(|node| node.is_element())
    {
        let Some(visual) = preview.visual(node.id) else {
            continue;
        };
        let Some(checks) = check_visual_with_layout(&visual, viewport) else {
            continue;
        };
        findings.extend(
            checks
                .accessibility
                .into_iter()
                .map(|issue| PreviewFinding::for_node(node, issue.code(), issue.message())),
        );
        if let Some(overflow) = checks.overflow {
            let code = if overflow.clipped {
                "clippedLayout"
            } else {
                "layoutOverflow"
            };
            let mut finding = PreviewFinding::for_node(node, code, overflow.message());
            finding.summary = Some(overflow.summary());
            finding.overflow = Some(overflow.into());
            findings.push(finding);
        }
        let parent = authored_parent(document, node).and_then(|owner| preview.visual(owner));
        let native_parent = visual.visual_parent();
        let layout_parent = parent
            .as_ref()
            .filter(|parent| native_parent.as_ref() == Some(*parent));
        let source = |property: &str| authored_layout_property_span(document, node, property);
        for value in check_layout_values(&visual, layout_parent, |property| {
            source(property).is_some()
        }) {
            let mut finding =
                PreviewFinding::for_node(node, "ineffectiveLayoutValue", value.message);
            finding.summary = Some(value.summary);
            finding.property = Some(value.property);
            finding.source = source(value.property);
            findings.push(finding);
        }
    }
    findings.sort_by_key(|finding| finding.code == "ineffectiveLayoutValue");
    Ok(findings)
}

fn authored_parent(document: &Document, node: &Node) -> Option<NodeId> {
    let ast = document.ast();
    let mut parent = ast.node(node.parent?)?;
    while parent.kind == NodeKind::PropertyElement {
        parent = ast.node(parent.parent?)?;
    }
    parent.is_element().then_some(parent.id)
}
