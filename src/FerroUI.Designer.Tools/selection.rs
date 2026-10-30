use crate::{
    require, LayoutContext, LayoutRegistry, Point, Rect, Result, SnapIndex, SnapResult,
    SnapSettings, ToolError,
};
use ferroui_designer_core::{Ast, Document, Edit, NodeId};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SelectionMode {
    #[default]
    Replace,
    Add,
    Toggle,
    Subtract,
}
#[derive(Clone, Debug, Default)]
pub struct Selection {
    nodes: BTreeSet<NodeId>,
    primary: Option<NodeId>,
    locked: BTreeSet<NodeId>,
    hidden: BTreeSet<NodeId>,
}
impl Selection {
    pub fn nodes(&self) -> Vec<NodeId> {
        self.nodes.iter().copied().collect()
    }
    pub fn primary(&self) -> Option<NodeId> {
        self.primary
    }
    pub fn set_locked(&mut self, node: NodeId, locked: bool) {
        if locked {
            self.locked.insert(node);
            self.nodes.remove(&node);
        } else {
            self.locked.remove(&node);
        }
        if !self.primary.is_some_and(|id| self.nodes.contains(&id)) {
            self.primary = self.nodes.last().copied();
        }
    }
    pub fn set_hidden(&mut self, node: NodeId, hidden: bool) {
        if hidden {
            self.hidden.insert(node);
            self.nodes.remove(&node);
        } else {
            self.hidden.remove(&node);
        }
        if !self.primary.is_some_and(|id| self.nodes.contains(&id)) {
            self.primary = self.nodes.last().copied();
        }
    }
    pub fn select(&mut self, document: &Document, node: NodeId, mode: SelectionMode) -> Result<()> {
        require(
            document.ast().node(node).is_some_and(|n| n.is_element()),
            "Selection requires an existing element",
        )?;
        require(
            self.selectable(document, node),
            "Element or ancestor is locked/hidden",
        )?;
        if mode == SelectionMode::Replace {
            self.nodes.clear();
        }
        match mode {
            SelectionMode::Replace | SelectionMode::Add => {
                self.nodes.insert(node);
                self.primary = Some(node);
            }
            SelectionMode::Toggle => {
                if !self.nodes.remove(&node) {
                    self.nodes.insert(node);
                    self.primary = Some(node);
                }
            }
            SelectionMode::Subtract => {
                self.nodes.remove(&node);
            }
        }
        if !self.primary.is_some_and(|id| self.nodes.contains(&id)) {
            self.primary = self.nodes.last().copied();
        }
        Ok(())
    }
    pub fn hit_test(
        &self,
        document: &Document,
        point: Point,
        paint_order: &[(NodeId, Rect)],
    ) -> Result<Option<NodeId>> {
        point.validate()?;
        for (node, rect) in paint_order.iter().rev() {
            rect.validate()?;
            if rect.contains(point) && self.selectable(document, *node) {
                return Ok(Some(*node));
            }
        }
        Ok(None)
    }
    pub fn marquee(
        &mut self,
        document: &Document,
        bounds: Rect,
        paint_order: &[(NodeId, Rect)],
        fully_contained: bool,
        mode: SelectionMode,
    ) -> Result<()> {
        bounds.validate()?;
        let mut nodes = vec![];
        let mut seen = BTreeSet::new();
        for (node, rect) in paint_order {
            rect.validate()?;
            let hit = if fully_contained {
                bounds.contains(Point::new(rect.x, rect.y))
                    && bounds.contains(Point::new(rect.right(), rect.bottom()))
            } else {
                bounds.intersects(*rect)
            };
            if hit && self.selectable(document, *node) && seen.insert(*node) {
                nodes.push(*node);
            }
        }
        if mode == SelectionMode::Replace {
            self.nodes.clear();
            self.primary = None;
        }
        for node in nodes {
            self.select(
                document,
                node,
                if mode == SelectionMode::Replace {
                    SelectionMode::Add
                } else {
                    mode
                },
            )?;
        }
        Ok(())
    }
    pub fn prune(&mut self, document: &Document) {
        let keep = self
            .nodes
            .iter()
            .copied()
            .filter(|id| self.selectable(document, *id))
            .collect();
        self.nodes = keep;
        if !self.primary.is_some_and(|id| self.nodes.contains(&id)) {
            self.primary = self.nodes.last().copied();
        }
    }
    fn selectable(&self, document: &Document, node: NodeId) -> bool {
        let mut current = Some(node);
        while let Some(id) = current {
            let Some(element) = document.ast().node(id) else {
                return false;
            };
            if !element.is_element() || self.locked.contains(&id) || self.hidden.contains(&id) {
                return false;
            }
            current = element.parent;
        }
        true
    }
}
/// Remove descendant selections and duplicates so moving a selected group does not move a child
/// twice. Order remains the caller's order; unknown IDs are rejected rather than silently ignored.
pub fn outermost(document: &Document, nodes: &[NodeId]) -> Result<Vec<NodeId>> {
    let selected = nodes.iter().copied().collect::<BTreeSet<_>>();
    let mut result = vec![];
    let mut seen = BTreeSet::new();
    for id in nodes {
        let node = document
            .ast()
            .node(*id)
            .ok_or_else(|| ToolError(format!("Unknown selection {id}")))?;
        require(node.is_element(), "Selection requires elements")?;
        let mut parent = node.parent;
        let mut nested = false;
        while let Some(id) = parent {
            if selected.contains(&id) {
                nested = true;
                break;
            }
            parent = document.ast().node(id).and_then(|n| n.parent);
        }
        if !nested && seen.insert(*id) {
            result.push(*id);
        }
    }
    Ok(result)
}
/// An immutable drag start snapshot. Preview is pure geometry; commit refuses to apply a stale
/// gesture after source or collaboration edits have changed the authored revision.
pub struct MoveGesture {
    revision: u64,
    syntax: Arc<Ast>,
    nodes: Vec<NodeId>,
    start: Point,
    bounds: Rect,
    snap: SnapIndex,
}
impl MoveGesture {
    pub fn begin(
        document: &Document,
        nodes: &[NodeId],
        measurements: &BTreeMap<NodeId, Rect>,
        start: Point,
        snap: SnapSettings,
    ) -> Result<Self> {
        start.validate()?;
        require(
            document.is_valid(),
            "A move gesture requires valid authored XAML",
        )?;
        let nodes = outermost(document, nodes)?;
        require(!nodes.is_empty(), "A move gesture requires a selection")?;
        let mut bounds = None;
        for node in &nodes {
            let rect = *measurements
                .get(node)
                .ok_or_else(|| ToolError("Selection has no measured bounds".into()))?;
            rect.validate()?;
            bounds = Some(bounds.map_or(rect, |b: Rect| b.union(rect)));
        }
        let bounds = bounds.unwrap();
        bounds.validate()?;
        let moving: BTreeSet<_> = nodes
            .iter()
            .flat_map(|node| document.ast().subtree(*node))
            .map(|node| node.id)
            .collect();
        let peers: Vec<_> = measurements
            .iter()
            .filter(|(id, _)| !moving.contains(*id))
            .map(|(_, rect)| *rect)
            .collect();
        Ok(Self {
            revision: document.revision(),
            syntax: document.ast_snapshot(),
            nodes,
            start,
            bounds,
            snap: snap.prepare(&peers)?,
        })
    }
    pub fn preview(&self, pointer: Point) -> Result<SnapResult> {
        pointer.validate()?;
        let moving = self.bounds.translated(Point::new(
            pointer.x - self.start.x,
            pointer.y - self.start.y,
        ))?;
        self.snap.snap(moving)
    }
    pub fn commit(
        &self,
        context: &LayoutContext<'_>,
        registry: &LayoutRegistry,
        pointer: Point,
    ) -> Result<Vec<Edit>> {
        require(
            context.document.revision() == self.revision
                && std::ptr::eq(context.document.ast(), self.syntax.as_ref()),
            "Document changed during the gesture; restart the drag",
        )?;
        let preview = self.preview(pointer)?;
        let delta = Point::new(
            preview.bounds.x - self.bounds.x,
            preview.bounds.y - self.bounds.y,
        );
        registry.move_selection(context, &self.nodes, delta)
    }
}
