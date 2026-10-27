//! Native geometry adapters for portable layout planners. The manipulation surface reports
//! artboard coordinates; each planner receives measured geometry in its panel's local space.
use crate::{DesignGesture, DesignerSession};
use ferroui_base::{Matrix, Ref, Visual};
use ferroui_controls::Grid;
use ferroui_designer_core::{Document, Edit, NodeId, NodeKind};
use ferroui_designer_tools::{
    Affine, Alignment, Axis, GridLines, LayoutAction, LayoutContext, Point, Rect, ResolvedLayout,
};
use std::collections::{BTreeMap, BTreeSet, btree_map::Entry};

fn effective_properties(visual: &Visual) -> BTreeMap<String, String> {
    ferroui_inspect::Inspector::properties(visual)
        .into_iter()
        .map(|info| (info.name, ferroui_inspect::display_value(Some(&info.value))))
        .collect()
}
fn is_custom_panel(session: &DesignerSession, node: NodeId) -> bool {
    let Some(element) = session.document().ast().node(node) else {
        return false;
    };
    let Some(visual) = session.preview.visual(node) else {
        return false;
    };
    let Some(panel) = visual.cast::<ferroui_controls::Panel>() else {
        return false;
    };
    if panel.is_items_host() {
        return false;
    }
    let qualified_custom = element.namespace_uri.as_deref().is_some_and(|uri| {
        !matches!(
            uri,
            "https://github.com/ferroui"
                | "using:FerroUI.Controls"
                | "clr-namespace:FerroUI.Controls;assembly=FerroUI.Controls"
        )
    }) && session
        .layout_tools()
        .adapter_exact(&element.name)
        .is_none()
        && matches!(
            element.local_name(),
            "Panel"
                | "Canvas"
                | "Grid"
                | "StackPanel"
                | "WrapPanel"
                | "DockPanel"
                | "UniformGrid"
                | "RelativePanel"
                | "FlexPanel"
        );
    !session.layout_tools().supports(&element.name)
        || qualified_custom
        || visual.get_type().name() != element.local_name()
}

/// Discoverable native fallback: unknown Panel subclasses retain their layout algorithm;
/// manipulation authors a render adjustment, while child insertion/order uses Panel.Children.
impl DesignerSession {
    /// Plans a panel-specific edit against the current, measured native preview.
    /// The returned edits share the same coordinate and expression rules as pointer tools.
    pub fn plan_layout_action(&self, action: LayoutAction) -> Result<Vec<Edit>, String> {
        self.ensure_writable().map_err(|error| error.to_string())?;
        if !self.can_manipulate() {
            return Err("Layout editing requires a successful preview of the current XAML".into());
        }
        let (owner, child) = match &action {
            LayoutAction::GridTrack { panel, .. } | LayoutAction::PanelProperty { panel, .. } => {
                (*panel, None)
            }
            LayoutAction::Drop { parent, .. } => (*parent, None),
            LayoutAction::Move { node, .. }
            | LayoutAction::Resize { node, .. }
            | LayoutAction::Reorder { node, .. }
            | LayoutAction::GridCell { node, .. }
            | LayoutAction::Dock { node, .. }
            | LayoutAction::Relative { node, .. } => (parent(self, *node)?, Some(*node)),
        };
        let measurement = measure(self, owner, child)?;
        self.layout_tools()
            .plan(&measurement.context(self.document()), action)
            .map_err(|error| error.to_string())
    }

    /// Applies one measured layout operation as one undoable transaction. Hosts can use this
    /// boundary for track/cell editors, docking pickers and registered panel property tools.
    pub fn apply_layout_action(&mut self, action: LayoutAction, label: &str) -> Result<(), String> {
        let edits = self.plan_layout_action(action)?;
        if edits.is_empty() {
            return Ok(());
        }
        self.apply_visual(label, edits)
    }

    pub fn runtime_layout_capabilities(
        &self,
        node: NodeId,
    ) -> Option<ferroui_designer_tools::LayoutCapabilities> {
        let element = self.document().ast().node(node)?;
        if self
            .preview
            .visual(node)
            .and_then(|visual| visual.cast::<ferroui_controls::Panel>())
            .is_some_and(|panel| panel.is_items_host())
        {
            return Some(ferroui_designer_tools::LayoutCapabilities {
                move_items: false,
                resize_items: false,
                reorder: false,
                drop_items: false,
                generated_items: true,
            });
        }
        if is_custom_panel(self, node) {
            return Some(ferroui_designer_tools::LayoutCapabilities {
                move_items: true,
                resize_items: true,
                reorder: true,
                drop_items: true,
                generated_items: false,
            });
        }
        self.layout_tools()
            .adapter(&element.name)
            .map(|adapter| adapter.capabilities())
    }
}

fn visual_adjustment(
    session: &DesignerSession,
    node: NodeId,
    adjustment: Matrix,
) -> Result<Vec<Edit>, String> {
    affine(adjustment)?;
    let visual = session
        .preview
        .visual(node)
        .ok_or("No live transform target")?;
    let parent = visual
        .visual_parent()
        .ok_or("The root transform belongs to its preview host")?;
    let to_artboard = crate::affine_manipulation::affine(
        parent
            .transform_to_visual(&session.preview.host)
            .ok_or("Detached layout parent")?,
    )
    .ok_or("The ancestor transform must be affine")?;
    let inverse = crate::affine_manipulation::inverse(to_artboard)
        .ok_or("The ancestor transform is singular")?;
    let local = crate::affine_manipulation::affine(
        visual
            .transform_to_visual(&parent)
            .ok_or("Detached transform target")?,
    )
    .ok_or("The target transform must be affine")?;
    let wanted = local * to_artboard * adjustment * inverse;
    let render = crate::affine_manipulation::render_for_local(
        wanted,
        visual.bounds(),
        visual.render_transform_origin(),
        visual.has_mirror_transform(),
    )
    .ok_or("The target transform is singular")?;
    transform_edits(
        session.document(),
        node,
        visual
            .render_transform()
            .map(|t| t.value())
            .unwrap_or(Matrix::IDENTITY),
        render,
    )
}

fn transform_edits(
    document: &Document,
    node: NodeId,
    before: Matrix,
    after: Matrix,
) -> Result<Vec<Edit>, String> {
    let element = document
        .ast()
        .node(node)
        .ok_or("Unknown transform target")?;
    let correction = crate::affine_manipulation::inverse(before).map(|inverse| inverse * after);
    if let Some(property) = element
        .children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .find(|node| node.name.ends_with(".RenderTransform"))
    {
        let original = document
            .ast()
            .element_children(property.id)
            .first()
            .and_then(|id| document.ast().node(*id))
            .ok_or("RenderTransform must contain a transform object")?;
        let correction = correction.ok_or("Cannot compose a singular authored transform")?;
        let original_source = &document.valid_source()[original.span.start..original.span.end];
        // The wrapper lives at this transform's lexical scope, which can differ from
        // the document root. An unused prefix keeps the original subtree's default
        // namespace intact and avoids a separate root edit invalidating the text span.
        const NS: &str = "https://github.com/ferroui";
        let namespaces = document.ast().namespaces(property.id);
        let (prefix, declaration) = if namespaces.get("").is_some_and(|uri| uri == NS) {
            (String::new(), String::new())
        } else if let Some((prefix, _)) = namespaces.iter().find(|(_, uri)| uri.as_str() == NS) {
            (format!("{prefix}:"), String::new())
        } else {
            let mut prefix = "designTransform".to_owned();
            let mut suffix = 1;
            while namespaces.contains_key(&prefix) {
                prefix = format!("designTransform{suffix}");
                suffix += 1;
            }
            (format!("{prefix}:"), format!(" xmlns:{prefix}=\"{NS}\""))
        };
        let fragment = format!(
            "<{prefix}TransformGroup{declaration}>{original_source}<{prefix}MatrixTransform Matrix=\"{}\" /></{prefix}TransformGroup>",
            crate::affine_manipulation::matrix_text(correction)
        );
        return Ok(vec![Edit::Text {
            span: original.span,
            replacement: fragment,
        }]);
    }
    let value = if let Some(original) = element.attribute("RenderTransform") {
        if original.trim_start().starts_with('{') {
            return Err("RenderTransform is bound. Use the property editor to author a local transform or adjust the binding source".into());
        }
        correction
            .map(|correction| {
                format!(
                    "{original} matrix({})",
                    crate::affine_manipulation::matrix_text(correction)
                )
            })
            .unwrap_or_else(|| {
                format!("matrix({})", crate::affine_manipulation::matrix_text(after))
            })
    } else {
        format!("matrix({})", crate::affine_manipulation::matrix_text(after))
    };
    Ok(vec![Edit::SetAttribute {
        node,
        name: "RenderTransform".into(),
        value,
    }])
}

/// Keep the visible rectangle/transform when a tree move changes a native Canvas parent.
/// Flow panels continue to own their children's layout. Canvas positions are normalized to
/// leading anchors; automatic dimensions are materialized from the accepted native layout.
pub(crate) fn canvas_reparent_edits(
    session: &DesignerSession,
    nodes: &[NodeId],
    destination: NodeId,
) -> Result<Vec<Edit>, String> {
    let Some(canvas) = session
        .preview
        .visual(destination)
        .and_then(|visual| visual.cast::<ferroui_controls::Canvas>())
    else {
        return Ok(Vec::new());
    };
    if is_custom_panel(session, destination) {
        return Ok(Vec::new());
    }
    let canvas = canvas.upcast::<Visual>();
    let artboard = session.preview.host.clone().upcast::<Visual>();
    // Compose through the common ancestor explicitly. Matrix order matters for sibling
    // parents with rotation, shear or reflection; translations alone hide this error.
    let artboard_to_canvas = canvas
        .transform_to_visual(&artboard)
        .and_then(crate::affine_manipulation::inverse)
        .ok_or("The new Canvas must have an invertible affine transform to the artboard")?;
    let mut edits = Vec::new();
    for node in nodes {
        if parent(session, *node)? == destination {
            continue;
        }
        let visual = session
            .preview
            .visual(*node)
            .ok_or("The dragged element has no live layout")?;
        let layout = visual
            .cast::<ferroui_base::layout::Layoutable>()
            .ok_or("The dragged element has no native layout properties")?;
        let bounds = visual.bounds();
        if bounds.width <= 0.0 || bounds.height <= 0.0 {
            return Err("Measure the artboard before moving this element to a Canvas".into());
        }
        let layout_parent = visual
            .visual_parent()
            .ok_or("The dragged element is detached")?;
        let parent_transform = layout_parent
            .transform_to_visual(&artboard)
            .and_then(crate::affine_manipulation::affine)
            .map(|matrix| matrix * artboard_to_canvas)
            .ok_or("The new Canvas must share an affine visual root with the selection")?;
        let wanted = visual
            .transform_to_visual(&artboard)
            .and_then(crate::affine_manipulation::affine)
            .map(|matrix| matrix * artboard_to_canvas)
            .ok_or("The dragged element must have an affine transform")?;
        let position = ferroui_base::Point::new(bounds.x, bounds.y).transform(parent_transform);
        let next_bounds =
            ferroui_base::Rect::new(position.x, position.y, bounds.width, bounds.height);
        let render = crate::affine_manipulation::render_for_local(
            wanted,
            next_bounds,
            visual.render_transform_origin(),
            visual.has_mirror_transform(),
        )
        .ok_or("The dragged element's transform is singular")?;
        let margin = layout.margin();
        let left = position.x - margin.left;
        let top = position.y - margin.top;
        if ![
            left,
            top,
            bounds.width,
            bounds.height,
            render.m11,
            render.m12,
            render.m21,
            render.m22,
            render.m31,
            render.m32,
        ]
        .iter()
        .all(|value| value.is_finite())
        {
            return Err("The new Canvas position must have finite geometry".into());
        }
        for name in ["Canvas.Left", "Canvas.Top", "Canvas.Right", "Canvas.Bottom"] {
            ensure_literal_property(session.document(), *node, name)?;
        }
        edits.extend([
            Edit::SetAttribute {
                node: *node,
                name: "Canvas.Left".into(),
                value: left.to_string(),
            },
            Edit::SetAttribute {
                node: *node,
                name: "Canvas.Top".into(),
                value: top.to_string(),
            },
            Edit::RemoveAttribute {
                node: *node,
                name: "Canvas.Right".into(),
            },
            Edit::RemoveAttribute {
                node: *node,
                name: "Canvas.Bottom".into(),
            },
        ]);
        for (name, automatic, size) in [
            ("Width", layout.width().is_nan(), bounds.width),
            ("Height", layout.height().is_nan(), bounds.height),
        ] {
            if automatic {
                ensure_literal_property(session.document(), *node, name)?;
                edits.push(Edit::SetAttribute {
                    node: *node,
                    name: name.into(),
                    value: size.to_string(),
                });
            }
        }
        let before = visual
            .render_transform()
            .map_or(Matrix::IDENTITY, |transform| transform.value());
        // Translation-only parent changes do not introduce redundant transform attributes.
        let unchanged = [
            before.m11 - render.m11,
            before.m12 - render.m12,
            before.m21 - render.m21,
            before.m22 - render.m22,
            before.m31 - render.m31,
            before.m32 - render.m32,
        ]
        .iter()
        .all(|difference| difference.abs() < 1e-9);
        if !unchanged {
            edits.extend(transform_edits(session.document(), *node, before, render)?);
        }
    }
    Ok(snapshot_edits_first(edits))
}

/// Raw source spans address the planning snapshot. Apply disjoint replacements from the
/// end before identity-based attributes or structure can move those spans. Visual planners
/// only replace transforms of outermost selected nodes, so these intervals do not overlap.
fn snapshot_edits_first(edits: Vec<Edit>) -> Vec<Edit> {
    let (mut source, semantic): (Vec<_>, Vec<_>) = edits
        .into_iter()
        .partition(|edit| matches!(edit, Edit::Text { .. }));
    source.sort_by_key(|edit| match edit {
        Edit::Text { span, .. } => std::cmp::Reverse(span.start),
        _ => unreachable!(),
    });
    source.extend(semantic);
    source
}

fn ensure_literal_property(document: &Document, node: NodeId, name: &str) -> Result<(), String> {
    let element = document.ast().node(node).ok_or("Unknown layout target")?;
    if element.attribute(name).is_some_and(|value| {
        value.trim_start().starts_with('{') && !value.trim_start().starts_with("{}")
    }) {
        return Err(format!(
            "{name} is an expression; edit it explicitly before reparenting to a Canvas"
        ));
    }
    if element
        .children
        .iter()
        .filter_map(|id| document.ast().node(*id))
        .any(|child| {
            child.kind == NodeKind::PropertyElement
                && (child.local_name() == name || child.local_name().ends_with(&format!(".{name}")))
        })
    {
        return Err(format!(
            "{name} is a property element; edit it explicitly before reparenting to a Canvas"
        ));
    }
    Ok(())
}

struct Measurement {
    bounds: BTreeMap<NodeId, Rect>,
    grid_lines: BTreeMap<NodeId, GridLines>,
    resolved: BTreeMap<NodeId, ResolvedLayout>,
    properties: BTreeMap<NodeId, BTreeMap<String, String>>,
    to_artboard: Affine,
    to_parent: Affine,
    visual: Ref<Visual>,
}
impl Measurement {
    fn context<'a>(&'a self, document: &'a Document) -> LayoutContext<'a> {
        LayoutContext::new(document, &self.bounds)
            .with_grid_lines(&self.grid_lines)
            .with_resolved_layout(&self.resolved)
            .with_resolved_properties(&self.properties)
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
    let matrix = crate::affine_manipulation::affine(matrix)
        .ok_or("Perspective transforms require a custom layout adapter")?;
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
    if authored
        .cast::<ferroui_controls::Panel>()
        .is_some_and(|panel| panel.is_items_host())
    {
        return Err("Generated item layout is owned by its ItemsControl; edit its item template or source collection".into());
    }
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
    let mut properties = BTreeMap::new();
    properties.insert(parent, effective_properties(&authored));
    let size = visual.bounds();
    bounds.insert(parent, Rect::new(0.0, 0.0, size.width, size.height));
    for id in children(session.document(), parent) {
        if let Some(control) = session.preview.visual(id) {
            properties.insert(id, effective_properties(&control));
            let layout_parent = control.visual_parent().unwrap_or_else(|| visual.clone());
            if let Some(transform) = layout_parent
                .transform_to_visual(&visual)
                .and_then(crate::affine_manipulation::affine)
            {
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
        properties,
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
    let mut custom = Vec::new();
    for (node, delta) in deltas {
        let source_parent = parent(session, node)?;
        if is_custom_panel(session, source_parent) {
            custom.extend(visual_adjustment(
                session,
                node,
                Matrix::create_translation(delta.x, delta.y),
            )?);
            continue;
        }
        let visual = session
            .preview
            .visual(node)
            .and_then(|node| node.visual_parent())
            .ok_or("The selected control is not attached to a layout parent")?;
        let runtime_parent = ferroui_inspect::Inspector::shared()
            .borrow_mut()
            .node_id(&visual);
        let key = (source_parent, runtime_parent);
        let measurement = match measured.entry(key) {
            Entry::Vacant(entry) => entry.insert(measure(session, source_parent, Some(node))?),
            Entry::Occupied(entry) => entry.into_mut(),
        };
        let delta = measurement.delta(delta)?;
        actions.push((key, LayoutAction::Move { node, delta }));
    }
    let contexts = measured
        .iter()
        .map(|(key, measurement)| (*key, measurement.context(session.document())))
        .collect::<BTreeMap<_, _>>();
    let mut edits = session
        .layout_tools()
        .plan_batch(
            actions
                .into_iter()
                .map(|(key, action)| (&contexts[&key], action)),
        )
        .map_err(|e| e.to_string())?;
    edits.extend(custom);
    Ok(snapshot_edits_first(edits))
}

pub(crate) fn commit(session: &mut DesignerSession, gesture: DesignGesture) -> Result<(), String> {
    if !session.can_manipulate() {
        return Err(
            "Visual manipulation requires a successful preview of the current source revision"
                .into(),
        );
    }
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
                if before.width <= 0.0
                    || before.height <= 0.0
                    || after.width <= 0.0
                    || after.height <= 0.0
                {
                    return Err("Resize dimensions must be positive".into());
                }
                let adjustment = Matrix::create_translation(-before.x, -before.y)
                    * Matrix::create_scale(
                        after.width / before.width,
                        after.height / before.height,
                    )
                    * Matrix::create_translation(after.x, after.y);
                let edits = visual_adjustment(session, node, adjustment)?;
                return session.apply_visual("Resize transformed selection", edits);
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
        DesignGesture::ResizeLocal { node, after } => {
            Rect::new(after.x, after.y, after.width, after.height)
                .validate()
                .map_err(|error| error.to_string())?;
            if after.width <= 0.0 || after.height <= 0.0 {
                return Err("Resize dimensions must be positive".into());
            }
            let measured = measure(session, parent(session, node)?, Some(node))?;
            let visual = session
                .preview
                .visual(node)
                .ok_or("No live resize target")?;
            let render = visual
                .render_transform()
                .map(|t| t.value())
                .unwrap_or(Matrix::IDENTITY);
            let original = visual.bounds();
            if is_custom_panel(session, parent(session, node)?) {
                if original.width <= 0.0 || original.height <= 0.0 {
                    return Err("The custom panel must arrange the target to a nonzero size before visual resize".into());
                }
                let geometry = crate::affine_manipulation::Geometry::capture(
                    &visual,
                    &session.preview.host.clone().upcast(),
                )
                .ok_or("The target transform is singular")?;
                let local = Matrix::create_scale(
                    after.width / original.width,
                    after.height / original.height,
                ) * Matrix::create_translation(after.x, after.y);
                edits.extend(visual_adjustment(
                    session,
                    node,
                    geometry.from_surface * local * geometry.to_surface,
                )?);
            } else {
                let resized = crate::affine_manipulation::resized_layout(
                    original,
                    after,
                    visual.render_transform_origin(),
                    visual.has_mirror_transform(),
                    render,
                );
                edits.extend(
                    session
                        .layout_tools()
                        .plan(
                            &measured.context(session.document()),
                            LayoutAction::Resize {
                                node,
                                before: Rect::new(
                                    original.x,
                                    original.y,
                                    original.width,
                                    original.height,
                                ),
                                after: Rect::new(
                                    resized.x,
                                    resized.y,
                                    resized.width,
                                    resized.height,
                                ),
                            },
                        )
                        .map_err(|error| error.to_string())?,
                );
            }
        }
        DesignGesture::Rotate { node, degrees } => {
            if !degrees.is_finite() {
                return Err("Rotation must be finite".into());
            }
            let visual = session
                .preview
                .visual(node)
                .ok_or("No live rotation target")?;
            let geometry = crate::affine_manipulation::Geometry::capture(
                &visual,
                &session.preview.host.clone().upcast(),
            )
            .ok_or("The target transform is singular")?;
            edits.extend(visual_adjustment(
                session,
                node,
                Matrix::create_rotation_at(degrees.to_radians(), geometry.pivot),
            )?);
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
                if is_custom_panel(session, target) {
                    let container = element
                        .children
                        .iter()
                        .filter_map(|id| session.document().ast().node(*id))
                        .find(|node| node.name.ends_with(".Children"))
                        .map_or(target, |node| node.id);
                    edits.push(Edit::InsertChild {
                        parent: container,
                        index: session.document().ast().element_children(container).len(),
                        source: source.clone(),
                    });
                    break;
                }
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
    if !session.can_manipulate() {
        return Err(
            "Alignment requires a successful preview of the current source revision".into(),
        );
    }
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
    let selected = outermost(document, session.selection());
    let mut edits = Vec::new();
    for id in &selected {
        let owner = parent(session, *id)?;
        if !session
            .runtime_layout_capabilities(owner)
            .is_some_and(|capabilities| capabilities.reorder)
        {
            return Err("Duplicate requires a panel accepting multiple authored children".into());
        }
    }
    for (id, source) in duplicate_sources(document, &selected)? {
        let node = document.ast().node(id).ok_or("Unknown duplicate source")?;
        let parent = node
            .parent
            .ok_or("The document root cannot be duplicated")?;
        edits.push(Edit::InsertChild {
            parent,
            index: document.ast().element_children(parent).len(),
            source,
        });
    }
    session
        .apply("Duplicate selection", edits)
        .map_err(|e| e.to_string())?;
    Ok(())
}
fn duplicate_sources(
    document: &Document,
    selected: &[NodeId],
) -> Result<Vec<(NodeId, String)>, String> {
    let roots = selected
        .iter()
        .map(|id| document.ast().node(*id).ok_or("Unknown selection"))
        .collect::<Result<Vec<_>, _>>()?;
    let copied = |span: ferroui_designer_core::SourceSpan| {
        roots
            .iter()
            .any(|root| span.start >= root.span.start && span.end <= root.span.end)
    };
    let occurrences = ferroui_designer_language::name_occurrences(document);
    let mut names = occurrences
        .iter()
        .filter(|item| item.declaration)
        .map(|item| item.name.clone())
        .collect::<BTreeSet<_>>();
    let mut replacements = BTreeMap::new();
    for declaration in occurrences
        .iter()
        .filter(|item| item.declaration && copied(item.span))
    {
        let mut suffix = 1;
        let unique = loop {
            let value = format!("{}_Copy{suffix}", declaration.name);
            if names.insert(value.clone()) {
                break value;
            }
            suffix += 1;
        };
        replacements.insert((declaration.scope, declaration.name.clone()), unique);
    }
    let mut result = Vec::new();
    for root in roots {
        let mut source = document.valid_source()[root.span.start..root.span.end].to_owned();
        let mut changes = Vec::new();
        for occurrence in occurrences
            .iter()
            .filter(|item| item.span.start >= root.span.start && item.span.end <= root.span.end)
        {
            if let Some(name) = replacements.get(&(occurrence.scope, occurrence.name.clone())) {
                changes.push((occurrence.span, name.clone()));
            }
        }
        for comment in document.ast().nodes().filter(|node| {
            node.kind == NodeKind::Comment
                && node.span.start >= root.span.start
                && node.span.end <= root.span.end
        }) {
            let scope = ferroui_designer_language::name_scope(document, comment.id);
            let renames = replacements
                .iter()
                .filter(|((name_scope, _), _)| *name_scope == scope)
                .map(|((_, original), copied)| (original.clone(), copied.clone()))
                .collect::<BTreeMap<_, _>>();
            let original = &document.valid_source()[comment.span.start..comment.span.end];
            // Metadata contains no XML names; a standalone wrapper lets the shared codec
            // rewrite this one comment without introducing inherited xmlns attributes.
            let wrapper = format!("<Metadata>{original}</Metadata>");
            let rewritten = ferroui_designer_animation::rewrite_design_metadata(&wrapper, &renames)
                .map_err(|error| error.to_string())?;
            let rewritten = ferroui_designer_animation::rekey_copied_programs(&rewritten)
                .map_err(|error| error.to_string())?;
            let rewritten = rewritten
                .strip_prefix("<Metadata>")
                .and_then(|source| source.strip_suffix("</Metadata>"))
                .ok_or("The state metadata codec changed its XML wrapper")?;
            if rewritten != original {
                changes.push((comment.span, rewritten.into()));
            }
        }
        changes.sort_by_key(|(span, _)| std::cmp::Reverse(span.start));
        for (span, replacement) in changes {
            source.replace_range(
                span.start - root.span.start..span.end - root.span.start,
                &replacement,
            );
        }
        result.push((root.id, source));
    }
    Ok(result)
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

#[cfg(test)]
mod duplicate_tests {
    use super::*;
    #[test]
    fn duplicate_rewrites_internal_references_across_siblings_but_preserves_external_and_literals()
    {
        let document=Document::new(r##"<Canvas xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml"><Button Name="External"/><Grid Name="Card"><Button Name="Target"/><TextBlock Text="{Binding ElementName=Target, Path=Content}" Tag="{x:Reference External}"/><Grid.Styles><Style Selector="#Target"><Setter Property="Tag" Value="Target"/></Style></Grid.Styles></Grid><Button Name="Peer" Tag="{Binding #Card.Tag}"/></Canvas>"##).unwrap();
        let result = duplicate_sources(
            &document,
            &[
                document.ast().find_name("Card").unwrap(),
                document.ast().find_name("Peer").unwrap(),
            ],
        )
        .unwrap();
        assert!(result[0].1.contains(r#"Name="Target_Copy1""#));
        assert!(result[0].1.contains("ElementName=Target_Copy1"));
        assert!(result[0].1.contains(r##"Selector="#Target_Copy1""##));
        assert!(result[0].1.contains("{x:Reference External}"));
        assert!(result[0].1.contains(r#"Value="Target""#));
        assert!(result[1].1.contains("#Card_Copy1.Tag"));
    }
    #[test]
    fn duplicate_references_resolve_within_each_template_namescope() {
        let document=Document::new(r##"<Grid xmlns="https://github.com/ferroui"><Button Name="Target"/><ContentControl Name="Copy"><ContentControl.Template><ControlTemplate><Grid><Button Name="Target"/><TextBlock Text="{Binding #Target.Content}"/></Grid></ControlTemplate></ContentControl.Template></ContentControl></Grid>"##).unwrap();
        let result =
            duplicate_sources(&document, &[document.ast().find_name("Copy").unwrap()]).unwrap();
        assert!(result[0].1.contains("#Target_Copy1.Content"));
    }
    #[test]
    fn duplicate_keeps_visual_state_metadata_targets_with_the_copied_controls() {
        use ferroui_designer_animation::{StateSetter, VisualState, VisualStateGroup};
        let mut document = Document::new(r#"<Canvas xmlns="https://github.com/ferroui"><Grid Name="Card"><Button Name="Target" /></Grid></Canvas>"#).unwrap();
        let owner = document.ast().find_name("Card").unwrap();
        let group = VisualStateGroup {
            name: "Selection".into(),
            owner,
            states: vec![VisualState {
                name: "Active".into(),
                setters: vec![StateSetter {
                    target: "Target".into(),
                    property: "Opacity".into(),
                    value: "0.5".into(),
                }],
            }],
            transitions: vec![],
        };
        document
            .apply("Create state group", group.plan(&document).unwrap())
            .unwrap();
        let source = duplicate_sources(&document, &[owner]).unwrap().remove(0).1;
        document
            .apply(
                "Duplicate card",
                vec![Edit::InsertChild {
                    parent: document.ast().root(),
                    index: 1,
                    source,
                }],
            )
            .unwrap();
        let copied_owner = document.ast().find_name("Card_Copy1").unwrap();
        let groups = VisualStateGroup::read(&document);
        let copied_group = groups
            .iter()
            .find(|group| group.owner == copied_owner)
            .unwrap();
        assert_eq!(copied_group.states[0].setters[0].target, "Target_Copy1");
        assert_eq!(
            groups
                .iter()
                .find(|group| group.owner == owner)
                .unwrap()
                .states[0]
                .setters[0]
                .target,
            "Target"
        );
        assert_eq!(document.source().matches("xmlns=").count(), 1);
    }
}
