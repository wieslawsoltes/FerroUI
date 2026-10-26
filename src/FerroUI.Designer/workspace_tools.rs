//! Native geometry adapters for portable layout planners. The manipulation surface reports
//! artboard coordinates; each planner receives measured geometry in its panel's local space.
use crate::{DesignGesture, DesignerSession};
use ferroui_base::{Matrix, Ref, Visual};
use ferroui_controls::Grid;
use ferroui_designer_core::{Document, Edit, NodeId, NodeKind};
use ferroui_designer_tools::{
    Affine, Alignment, Axis, GridLines, LayoutAction, LayoutContext, Point, Rect, ResolvedLayout,
};
use std::collections::{BTreeMap, BTreeSet};

struct Measurement {
    bounds: BTreeMap<NodeId, Rect>,
    grid_lines: BTreeMap<NodeId, GridLines>,
    resolved: BTreeMap<NodeId, ResolvedLayout>,
    to_artboard: Affine,
    to_parent: Affine,
    visual: Ref<Visual>,
}
impl Measurement {
    fn context<'a>(&'a self, document: &'a Document) -> LayoutContext<'a> {
        LayoutContext::new(document, &self.bounds)
            .with_grid_lines(&self.grid_lines)
            .with_resolved_layout(&self.resolved)
    }
    fn delta(&self, delta: Point) -> Result<Point, String> {
        let origin = self
            .to_parent
            .transform(Point::new(0.0, 0.0))
            .map_err(|e| e.to_string())?;
        let end = self.to_parent.transform(delta).map_err(|e| e.to_string())?;
        Ok(Point::new(end.x - origin.x, end.y - origin.y))
    }
}
fn affine(matrix: Matrix) -> Result<Affine, String> {
    if matrix.m13 != 0.0 || matrix.m23 != 0.0 || matrix.m33 != 1.0 {
        return Err("Perspective transforms require a custom layout adapter".into());
    }
    let result = Affine {
        m11: matrix.m11,
        m12: matrix.m12,
        m21: matrix.m21,
        m22: matrix.m22,
        dx: matrix.m31,
        dy: matrix.m32,
    };
    result.validate().map_err(|e| e.to_string())?;
    Ok(result)
}
fn parent(session: &DesignerSession, node: NodeId) -> Result<NodeId, String> {
    let mut parent = session
        .document()
        .ast()
        .node(node)
        .ok_or("Unknown element")?
        .parent;
    while let Some(id) = parent {
        let element = session.document().ast().node(id).ok_or("Unknown parent")?;
        if element.kind != NodeKind::PropertyElement {
            return Ok(id);
        }
        parent = element.parent;
    }
    Err("The root is positioned by the artboard; select a child to move".into())
}
fn boundaries(lengths: impl Iterator<Item = f64>, spacing: f64, extent: f64) -> Vec<f64> {
    let lengths = lengths.collect::<Vec<_>>();
    if lengths.is_empty() {
        return vec![0.0, extent.max(0.0)];
    }
    let mut result = vec![0.0];
    let mut offset = 0.0;
    for (i, length) in lengths.iter().enumerate() {
        offset += *length;
        if i + 1 < lengths.len() {
            offset += spacing;
        }
        result.push(offset);
    }
    result
}
fn children(document: &Document, parent: NodeId) -> Vec<NodeId> {
    let mut result = vec![];
    for id in document.ast().element_children(parent) {
        if document
            .ast()
            .node(id)
            .is_some_and(|node| node.kind == NodeKind::PropertyElement)
        {
            result.extend(document.ast().element_children(id));
        } else {
            result.push(id);
        }
    }
    result
}
fn measure(
    session: &DesignerSession,
    parent: NodeId,
    child: Option<NodeId>,
) -> Result<Measurement, String> {
    let authored = session.preview.visual(parent).ok_or(
        "The selected parent has no live layout; repair its preview or edit its template in source",
    )?;
    // Margin and Width/Height are interpreted by the actual layout parent, which may be a
    // ContentPresenter inside the source parent's template or a scaled Viewbox container.
    let visual = child
        .and_then(|node| session.preview.visual(node))
        .and_then(|visual| visual.visual_parent())
        .unwrap_or_else(|| authored.clone());
    let artboard = session.preview.host.clone().upcast::<Visual>();
    let to_artboard = affine(
        visual
            .transform_to_visual(&artboard)
            .ok_or("The parent is not attached to the artboard")?,
    )?;
    let to_parent = to_artboard.inverse().map_err(|e| e.to_string())?;
    let mut bounds = BTreeMap::new();
    let mut resolved = BTreeMap::new();
    let size = visual.bounds();
    bounds.insert(parent, Rect::new(0.0, 0.0, size.width, size.height));
    for id in children(session.document(), parent) {
        if let Some(control) = session.preview.visual(id) {
            let layout_parent = control.visual_parent().unwrap_or_else(|| visual.clone());
            if let Some(transform) = layout_parent.transform_to_visual(&visual) {
                // Transform the layout rectangle from its parent, excluding its own
                // RenderTransform while preserving presenter offsets and ancestor scaling.
                let rect = control.bounds().transform_to_aabb(transform);
                bounds.insert(id, Rect::new(rect.x, rect.y, rect.width, rect.height));
                if let Some(layout) = control.cast::<ferroui_base::layout::Layoutable>() {
                    let margin = layout.margin();
                    let slot = layout
                        .previous_arrange()
                        .map(|rect| rect.transform_to_aabb(transform))
                        .map(|rect| Rect::new(rect.x, rect.y, rect.width, rect.height));
                    resolved.insert(
                        id,
                        ResolvedLayout {
                            margin: [margin.left, margin.top, margin.right, margin.bottom],
                            slot,
                            visible: layout.is_visible(),
                        },
                    );
                }
            }
        }
    }
    let mut grid_lines = BTreeMap::new();
    if let Some(grid) = authored.cast::<Grid>() {
        grid_lines.insert(
            parent,
            GridLines {
                rows: boundaries(
                    grid.row_definitions()
                        .snapshot()
                        .iter()
                        .map(|r| r.actual_height()),
                    grid.row_spacing(),
                    grid.bounds().height,
                ),
                columns: boundaries(
                    grid.column_definitions()
                        .snapshot()
                        .iter()
                        .map(|c| c.actual_width()),
                    grid.column_spacing(),
                    grid.bounds().width,
                ),
            },
        );
    }
    Ok(Measurement {
        bounds,
        grid_lines,
        resolved,
        to_artboard,
        to_parent,
        visual,
    })
}
fn plan_moves(
    session: &DesignerSession,
    deltas: BTreeMap<NodeId, Point>,
) -> Result<Vec<Edit>, String> {
    let mut measured = BTreeMap::new();
    let mut actions = Vec::new();
    for (node, delta) in deltas {
        let source_parent = parent(session, node)?;
        let visual = session
            .preview
            .visual(node)
            .and_then(|node| node.visual_parent())
            .ok_or("The selected control is not attached to a layout parent")?;
        let runtime_parent = ferroui_inspect::Inspector::shared()
            .borrow_mut()
            .node_id(&visual);
        let key = (source_parent, runtime_parent);
        if !measured.contains_key(&key) {
            measured.insert(key, measure(session, source_parent, Some(node))?);
        }
        let delta = measured[&key].delta(delta)?;
        actions.push((key, LayoutAction::Move { node, delta }));
    }
    let contexts = measured
        .iter()
        .map(|(key, measurement)| (*key, measurement.context(session.document())))
        .collect::<BTreeMap<_, _>>();
    session
        .layout_tools()
        .plan_batch(
            actions
                .into_iter()
                .map(|(key, action)| (&contexts[&key], action)),
        )
        .map_err(|e| e.to_string())
}

pub(crate) fn commit(session: &mut DesignerSession, gesture: DesignGesture) -> Result<(), String> {
    let mut edits = Vec::new();
    match gesture {
        DesignGesture::Move { nodes, delta } => {
            edits.extend(plan_moves(
                session,
                outermost(session.document(), &nodes)
                    .into_iter()
                    .map(|node| (node, Point::new(delta.x, delta.y)))
                    .collect(),
            )?);
        }
        DesignGesture::Resize {
            node,
            before,
            after,
        } => {
            let measured = measure(session, parent(session, node)?, Some(node))?;
            let transform = measured.to_artboard;
            let visual = session
                .preview
                .visual(node)
                .ok_or("No live layout for selected element")?;
            let own = affine(
                visual
                    .transform_to_visual(&measured.visual)
                    .ok_or("Detached element")?,
            )?;
            if transform.m12.abs() > 1e-9
                || transform.m21.abs() > 1e-9
                || transform.m11 <= 0.0
                || transform.m22 <= 0.0
                || own.m12.abs() > 1e-9
                || own.m21.abs() > 1e-9
                || (own.m11 - 1.0).abs() > 1e-9
                || (own.m22 - 1.0).abs() > 1e-9
            {
                return Err("Resize rotated, skewed, mirrored or individually scaled controls through Width/Height and the transform editor".into());
            }
            let original = *measured
                .bounds
                .get(&node)
                .ok_or("Missing measured layout")?;
            let delta = measured.delta(Point::new(after.x - before.x, after.y - before.y))?;
            let after = Rect::new(
                original.x + delta.x,
                original.y + delta.y,
                after.width / transform.m11,
                after.height / transform.m22,
            );
            edits.extend(
                session
                    .layout_tools()
                    .plan(
                        &measured.context(session.document()),
                        LayoutAction::Resize {
                            node,
                            before: original,
                            after,
                        },
                    )
                    .map_err(|e| e.to_string())?,
            );
        }
        DesignGesture::Rotate { node, degrees } => {
            if !degrees.is_finite() {
                return Err("Rotation must be finite".into());
            }
            let measured = measure(session, parent(session, node)?, Some(node))?;
            let visual = session
                .preview
                .visual(node)
                .ok_or("No live rotation target")?;
            let own = affine(
                visual
                    .transform_to_visual(&measured.visual)
                    .ok_or("Detached rotation target")?,
            )?;
            let conformal = |matrix: Affine| {
                let x = matrix.m11 * matrix.m11 + matrix.m12 * matrix.m12;
                let y = matrix.m21 * matrix.m21 + matrix.m22 * matrix.m22;
                matrix.m11 * matrix.m22 - matrix.m12 * matrix.m21 > 0.0
                    && (x - y).abs() < 1e-7 * x.max(y).max(1.0)
                    && (matrix.m11 * matrix.m21 + matrix.m12 * matrix.m22).abs()
                        < 1e-7 * x.max(y).max(1.0)
            };
            let origin = visual
                .render_transform_origin()
                .to_pixels(visual.bounds().size());
            if !conformal(measured.to_artboard)
                || !conformal(own)
                || (origin.x - visual.bounds().width * 0.5).abs() > 0.001
                || (origin.y - visual.bounds().height * 0.5).abs() > 0.001
            {
                return Err("Use RenderTransform to rotate controls with a custom origin, mirroring, skew or nonuniform scaling".into());
            }
            let element = session
                .document()
                .ast()
                .node(node)
                .ok_or("Unknown element")?;
            if element
                .children
                .iter()
                .filter_map(|id| session.document().ast().node(*id))
                .any(|node| node.name.ends_with(".RenderTransform"))
            {
                return Err("Edit the existing RenderTransform property element in XAML before adding a rotation".into());
            }
            let original = element.attribute("RenderTransform").unwrap_or("");
            if original.starts_with('{') {
                return Err(
                    "The transform is bound; edit its binding or animate the bound value".into(),
                );
            }
            edits.push(Edit::SetAttribute {
                node,
                name: "RenderTransform".into(),
                value: format!("{original} rotate({degrees}deg)").trim().to_owned(),
            });
        }
        DesignGesture::Insert { source, position } => {
            let (source, declaration) =
                crate::preview_source::embed_native_fragment(session.document(), &source)?;
            let mut planning_document = session.document().clone();
            if let Some(declaration) = declaration {
                planning_document
                    .apply("Declare native asset namespace", vec![declaration.clone()])
                    .map_err(|error| error.to_string())?;
                edits.push(declaration);
            }
            let mut target = None;
            if let Some(root) = session.preview.root() {
                let host = session.preview.host.clone().upcast::<Visual>();
                if let Some(point) = host.translate_point(position, &root) {
                    let mut visual = root.get_visual_at(point);
                    while let Some(current) = visual {
                        if let Some(id) = session.preview.source_id(&current) {
                            target = Some(id);
                            break;
                        }
                        visual = current.visual_parent();
                    }
                }
            }
            let mut target = target.unwrap_or(session.document().ast().root());
            let mut error = "No eligible drop target exists at this point".to_owned();
            loop {
                let element = session
                    .document()
                    .ast()
                    .node(target)
                    .ok_or("Drop target no longer exists")?;
                if session.layout_tools().supports(&element.name) {
                    let plan = measure(session, target, None).and_then(|measured| {
                        let position = measured
                            .to_parent
                            .transform(Point::new(position.x, position.y))
                            .map_err(|e| e.to_string())?;
                        session
                            .layout_tools()
                            .plan(
                                &measured.context(&planning_document),
                                LayoutAction::Drop {
                                    parent: target,
                                    source: source.clone(),
                                    position,
                                },
                            )
                            .map_err(|e| e.to_string())
                    });
                    match plan {
                        Ok(plan) => {
                            edits.extend(plan);
                            break;
                        }
                        Err(reason) => error = reason,
                    }
                }
                target = element.parent.ok_or(error.clone())?;
            }
        }
    }
    if !edits.is_empty() {
        session.apply_visual("Manipulate selection", edits)?;
    }
    Ok(())
}
fn artboard_bounds(session: &DesignerSession) -> Result<Vec<(NodeId, Rect)>, String> {
    let relative = session.preview.host.clone().upcast::<Visual>();
    outermost(session.document(), session.selection())
        .into_iter()
        .map(|node| {
            let rect = session
                .preview
                .bounds(node, &relative)
                .ok_or("Selection has no live layout")?;
            Ok((node, Rect::new(rect.x, rect.y, rect.width, rect.height)))
        })
        .collect()
}
fn apply_deltas(
    session: &mut DesignerSession,
    deltas: BTreeMap<NodeId, Point>,
    label: &str,
) -> Result<(), String> {
    let edits = plan_moves(session, deltas)?;
    session.apply_visual(label, edits)
}
pub(crate) fn align(session: &mut DesignerSession, alignment: Alignment) -> Result<(), String> {
    let deltas = ferroui_designer_tools::align(&artboard_bounds(session)?, alignment)
        .map_err(|e| e.to_string())?;
    apply_deltas(session, deltas, "Align selection")
}
pub(crate) fn distribute(session: &mut DesignerSession, axis: Axis) -> Result<(), String> {
    let deltas = ferroui_designer_tools::distribute(&artboard_bounds(session)?, axis)
        .map_err(|e| e.to_string())?;
    apply_deltas(session, deltas, "Distribute selection")
}
fn outermost(document: &Document, nodes: &[NodeId]) -> Vec<NodeId> {
    let selected = nodes.iter().copied().collect::<BTreeSet<_>>();
    nodes
        .iter()
        .copied()
        .filter(|id| {
            let mut parent = document.ast().node(*id).and_then(|node| node.parent);
            while let Some(id) = parent {
                if selected.contains(&id) {
                    return false;
                }
                parent = document.ast().node(id).and_then(|node| node.parent);
            }
            true
        })
        .collect()
}

pub(crate) fn duplicate(session: &mut DesignerSession) -> Result<(), String> {
    let document = session.document();
    let mut edits = Vec::new();
    let mut names = document
        .ast()
        .elements()
        .filter_map(|node| node.attribute("Name").or_else(|| node.attribute("x:Name")))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    for id in outermost(document, session.selection()) {
        let node = document.ast().node(id).ok_or("Unknown selection")?;
        let owner = self::parent(session, id)?;
        let owner_type = document.ast().node(owner).ok_or("Missing parent")?;
        if !session
            .layout_tools()
            .adapter(&owner_type.name)
            .is_some_and(|adapter| adapter.capabilities().reorder)
        {
            return Err("Duplicate requires a panel that accepts multiple authored children; place the control in a panel first".into());
        }
        let parent = node
            .parent
            .ok_or("The document root cannot be duplicated")?;
        let mut text = document.valid_source()[node.span.start..node.span.end].to_owned();
        let mut patches = Vec::new();
        for descendant in document
            .ast()
            .elements()
            .filter(|item| item.span.start >= node.span.start && item.span.end <= node.span.end)
        {
            for attribute in descendant
                .attributes
                .iter()
                .filter(|attribute| matches!(attribute.name.as_str(), "Name" | "x:Name"))
            {
                let mut suffix = 1;
                let unique = loop {
                    let candidate = format!("{}_Copy{suffix}", attribute.value);
                    if names.insert(candidate.clone()) {
                        break candidate;
                    }
                    suffix += 1;
                };
                patches.push((
                    attribute.value_span.start - node.span.start,
                    attribute.value_span.end - node.span.start,
                    unique,
                ));
            }
        }
        patches.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
        for (start, end, replacement) in patches {
            text.replace_range(start..end, &replacement);
        }
        edits.push(Edit::InsertChild {
            parent,
            index: document.ast().element_children(parent).len(),
            source: text,
        });
    }
    session
        .apply("Duplicate selection", edits)
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub(crate) fn delete(session: &mut DesignerSession) -> Result<(), String> {
    let root = session.document().ast().root();
    let ids = outermost(session.document(), session.selection())
        .into_iter()
        .filter(|id| *id != root);
    let edits = ids.map(|node| Edit::RemoveNode { node }).collect();
    session
        .apply("Delete selection", edits)
        .map_err(|error| error.to_string())?;
    Ok(())
}
