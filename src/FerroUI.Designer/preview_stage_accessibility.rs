//! Read the automation tree and a native Tab traversal from one accepted stage.
use super::{keyboard, with_runtime_errors, PreviewStageBoard, PreviewStageId, RenderedStage};
use ferroui_base::diagnostics::DesignTextEnvironment;
use ferroui_base::input::InputElement;
use ferroui_base::{FerroObject, Ref, Visual};
use ferroui_designer_core::{Document, NodeId, SourceSpan};
use ferroui_designer_rsx::RsxSourceMap;
use ferroui_inspect::{Inspector, TreeKind};
use serde::Serialize;
use std::collections::BTreeSet;

const MAX_ROWS: usize = 512;

#[derive(Serialize)]
pub struct PreviewStageAccessibility {
    pub tree: Vec<PreviewStageAccessibleNode>,
    pub tab_order: Vec<PreviewStageAccessibleNode>,
    pub truncated: bool,
}

impl PreviewStageAccessibility {
    pub fn map_rsx_source(&mut self, document: &Document, map: &RsxSourceMap) {
        for row in self.tree.iter_mut().chain(&mut self.tab_order) {
            row.source = row.node.and_then(|node| {
                let node = document.ast().node(node)?;
                map.control(node.span.start).map(|(_, name)| name)
            });
        }
    }
}

#[derive(Serialize)]
pub struct PreviewStageAccessibleNode {
    pub runtime_id: u64,
    pub depth: usize,
    pub control: String,
    pub name: String,
    pub role: String,
    pub control_element: bool,
    pub enabled: bool,
    pub offscreen: bool,
    pub node: Option<NodeId>,
    pub source: Option<SourceSpan>,
}

impl PreviewStageBoard {
    pub fn preview_accessibility(
        &self,
        id: PreviewStageId,
    ) -> Result<PreviewStageAccessibility, String> {
        let stage = &self.stages[self.index(id)?];
        let rendered = self.current(stage)?;
        if rendered.preview.host.bounds().is_empty() || !rendered.scope.is_effectively_visible() {
            return Err("Accessibility inspection requires a visible laid-out stage".into());
        }
        let environment = DesignTextEnvironment::get(&rendered.scope);
        DesignTextEnvironment::run(environment.as_ref(), || {
            rendered.source_snapshot.run(|| {
                with_runtime_errors("accessibility inspection", || {
                    let mut report = PreviewStageAccessibility {
                        tree: Vec::new(),
                        tab_order: Vec::new(),
                        truncated: false,
                    };
                    if let Some(root) = rendered.preview.root() {
                        report.read_tree(rendered, root.upcast());
                        report.read_tab_order(rendered);
                    }
                    Ok(report)
                })
            })
        })
    }
}

impl PreviewStageAccessibility {
    fn read_tree(&mut self, rendered: &RenderedStage, root: Ref<FerroObject>) {
        let inspector = Inspector::shared();
        let mut visited = BTreeSet::new();
        let mut remaining = vec![(root, 0)];
        while let Some((object, depth)) = remaining.pop() {
            let Some(row) = row(rendered, &object, depth) else {
                continue;
            };
            if !visited.insert(row.runtime_id) {
                continue;
            }
            if self.tree.len() == MAX_ROWS {
                self.truncated = true;
                break;
            }
            self.tree.push(row);
            let children = inspector.borrow().children(&object, TreeKind::Automation);
            remaining.extend(children.into_iter().rev().map(|child| (child, depth + 1)));
        }
    }

    fn read_tab_order(&mut self, rendered: &RenderedStage) {
        let scope = rendered.scope.clone().upcast::<InputElement>();
        let mut visited = BTreeSet::new();
        let inspector = Inspector::shared();
        let mut previous = None;
        while let Some(input) = keyboard::tab_target(&scope, previous.as_ref(), false) {
            if !visited.insert(inspector.borrow_mut().node_id(&input)) {
                break;
            }
            if visited.len() > MAX_ROWS {
                self.truncated = true;
                break;
            }
            previous = Some(input.clone());
            if let Some(row) = row(rendered, &input.upcast(), 0) {
                self.tab_order.push(row);
            }
        }
    }
}

fn row(
    rendered: &RenderedStage,
    object: &Ref<FerroObject>,
    depth: usize,
) -> Option<PreviewStageAccessibleNode> {
    let peer = Inspector::peer(object).ok()?;
    let owner = Inspector::visual_object(&peer.clone().upcast());
    let node = owner
        .cast::<Visual>()
        .and_then(|visual| rendered.preview.source_id(&visual));
    let source = node
        .and_then(|node| rendered.document.ast().node(node))
        .map(|node| node.name_span);
    Some(PreviewStageAccessibleNode {
        runtime_id: Inspector::shared().borrow_mut().node_id(&peer),
        depth,
        control: owner.get_type().name().to_owned(),
        name: peer.get_name(),
        role: format!("{:?}", peer.get_automation_control_type()),
        control_element: peer.is_control_element(),
        enabled: peer.is_enabled(),
        offscreen: peer.is_offscreen(),
        node,
        source,
    })
}
