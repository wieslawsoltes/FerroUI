//! Native measurement and transaction boundary for portable panel design-time features.
//! Providers describe panel-local geometry. The bridge owns native hit testing, affine
//! conversion, source identity, content ownership and one-shot transaction validation.
use crate::DesignerSession;
use crate::workspace_tools::{Measurement, is_custom_panel, measure, parent, snapshot_edits_first};
use ferroui_base::{Point, Ref, Visual};
use ferroui_designer_core::{Ast, Edit, NodeId, NodeKind};
use ferroui_designer_tools::{
    Affine, Axis, LayoutAction, LayoutGuideGeometry, LayoutGuideRole, LayoutHelperRequest,
    LayoutHelperScene, LayoutPlacement, Point as LayoutPoint, Rect as LayoutRect,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

#[path = "workspace_layout_helper_frames.rs"]
mod frames;

#[derive(Clone, Debug)]
struct LayoutGuard {
    uri: String,
    revision: u64,
    ast: Arc<Ast>,
    selection: Vec<NodeId>,
    generation: u64,
    providers: u64,
    layer_epoch: u64,
    scope_epoch: u64,
}
impl LayoutGuard {
    fn capture(session: &DesignerSession) -> Self {
        Self {
            uri: session.document_uri().to_owned(),
            revision: session.document().revision(),
            ast: session.document().ast_snapshot(),
            selection: session.selection().to_vec(),
            generation: session.generation(),
            providers: session.layout_tools().revision(),
            layer_epoch: session.layer_view_epoch(),
            scope_epoch: session.definition_scope_epoch(),
        }
    }
    fn validate(&self, session: &DesignerSession) -> Result<(), String> {
        session
            .ensure_writable()
            .map_err(|error| error.to_string())?;
        if !session.can_manipulate()
            || self.uri != session.document_uri()
            || self.revision != session.document().revision()
            || !Arc::ptr_eq(&self.ast, &session.document().ast_snapshot())
            || self.selection != session.selection()
            || self.generation != session.generation()
            || self.providers != session.layout_tools().revision()
            || self.layer_epoch != session.layer_view_epoch()
            || self.scope_epoch != session.definition_scope_epoch()
        {
            return Err(
                "The document, selection or layout provider changed; restart the layout gesture"
                    .into(),
            );
        }
        Ok(())
    }
}

/// Portable panel-local helper geometry and its measured affine mapping to the artboard.
/// Renderers transform vertices explicitly so their pens, labels and hit targets can stay
/// constant in screen space under zoom, rotation, reflection and shear.
#[derive(Clone, Debug)]
pub struct LayoutHelperSnapshot {
    pub scene: LayoutHelperScene,
    pub to_artboard: Affine,
    guard: Option<LayoutGuard>,
    handle_frames: BTreeMap<String, frames::HelperHandleFrame>,
}
impl LayoutHelperSnapshot {
    /// Construct a render-only snapshot for a standalone host. Session-created snapshots
    /// additionally carry a private source identity used for authoring helper gestures.
    pub fn new(scene: LayoutHelperScene, to_artboard: Affine) -> Self {
        Self {
            scene,
            to_artboard,
            guard: None,
            handle_frames: BTreeMap::new(),
        }
    }
    pub fn is_current(&self, session: &DesignerSession) -> bool {
        self.guard
            .as_ref()
            .is_some_and(|guard| guard.validate(session).is_ok())
    }

    /// Convert a logical handle adjustment into artboard displacement. Child margin grips
    /// can live inside an internally scaled presenter with units unlike the outer panel.
    pub fn handle_delta_to_artboard(
        &self,
        handle_index: usize,
        delta: LayoutPoint,
    ) -> Result<LayoutPoint, String> {
        let handle = self
            .scene
            .handles
            .get(handle_index)
            .ok_or("The layout handle is no longer available")?;
        let transform = self
            .handle_frames
            .get(&handle.id)
            .map_or(self.to_artboard, |frame| frame.to_artboard);
        frames::transform_vector(transform, delta)
    }
}

#[derive(Clone, Debug, PartialEq)]
enum PreviewIntent {
    Move(Vec<NodeId>),
    Insert(String),
    Handle(String),
}

/// Immutable, validated authoring result paired with the geometry presented to the user.
/// A rejected result still contains an explicit target and explanation for red feedback.
/// The edit list is private: commit applies the exact accepted preview after checking its
/// source, selection, permissions and provider identity again.
#[derive(Clone, Debug)]
pub struct LayoutEditPreview {
    pub helpers: LayoutHelperSnapshot,
    guard: LayoutGuard,
    edits: Vec<Edit>,
    intent: PreviewIntent,
    rejection: Option<String>,
    label: String,
}
impl LayoutEditPreview {
    pub fn is_accepted(&self) -> bool {
        self.rejection.is_none()
    }
    pub fn rejection(&self) -> Option<&str> {
        self.rejection.as_deref()
    }
    pub fn target(&self) -> NodeId {
        self.helpers.scene.panel
    }
    pub fn description(&self) -> &str {
        &self.label
    }
    pub fn is_current(&self, session: &DesignerSession) -> bool {
        self.guard.validate(session).is_ok()
    }
}

impl DesignerSession {
    /// Measure one selected panel's helpers. The optional pointer is in artboard coordinates;
    /// it is converted into the provider's panel-local space before resolving placement.
    pub fn layout_helper_snapshot(
        &self,
        panel: NodeId,
        pointer: Option<Point>,
        dragged_nodes: &[NodeId],
    ) -> Result<LayoutHelperSnapshot, String> {
        if !self.can_manipulate() {
            return Err("Layout helpers require a writable, current native preview".into());
        }
        let measured = measure(self, panel, None)?;
        snapshot(self, panel, pointer, dragged_nodes, &measured)
    }

    /// Contextual helpers for the selected container, or the common parent of selected
    /// content. Unsupported or unavailable native layout has no idle overlay.
    pub fn selection_layout_helpers(&self) -> Option<LayoutHelperSnapshot> {
        if self.selection().is_empty() || !self.can_manipulate() {
            return None;
        }
        let selected = self.selection();
        let panel = if selected.len() == 1 && selected_container(self, selected[0]) {
            selected[0]
        } else {
            let candidate = parent(self, selected[0]).ok()?;
            if !selected
                .iter()
                .all(|id| parent(self, *id).ok() == Some(candidate))
            {
                return None;
            }
            candidate
        };
        self.layout_helper_snapshot(panel, None, &[]).ok()
    }

    pub fn preview_layout_move(
        &self,
        nodes: &[NodeId],
        position: Point,
        delta: Point,
    ) -> Result<Option<LayoutEditPreview>, String> {
        self.preview_layout_move_cached(nodes, position, delta, None)
    }

    /// Preview insertion/reordering without touching live controls or source. Returning
    /// None preserves ordinary translation inside the same free-layout parent. The cached
    /// variant reuses validation when pointer movement retains the exact semantic edits.
    pub fn preview_layout_move_cached(
        &self,
        nodes: &[NodeId],
        position: Point,
        delta: Point,
        previous: Option<&LayoutEditPreview>,
    ) -> Result<Option<LayoutEditPreview>, String> {
        writable(self)?;
        point(position)?;
        point(delta)?;
        let nodes = ferroui_designer_tools::outermost(self.document(), nodes)
            .map_err(|error| error.to_string())?;
        if nodes.is_empty() {
            return Ok(None);
        }
        for node in &nodes {
            self.ensure_layer_manipulable(*node)?;
        }
        let Some(target) = layout_target(self, position, &nodes) else {
            return Ok(None);
        };
        self.ensure_layer_container_context(target, &nodes)?;
        let same_parent = nodes
            .iter()
            .all(|node| parent(self, *node).ok() == Some(target));
        let element = self
            .document()
            .ast()
            .node(target)
            .ok_or("The layout target disappeared")?;
        let builtin = self
            .layout_tools()
            .is_builtin_adapter_for(self.document(), target)
            && !is_custom_panel(self, target);
        let canvas = builtin
            && self
                .preview
                .visual(target)
                .is_some_and(|visual| visual.cast::<ferroui_controls::Canvas>().is_some());
        if same_parent
            && matches!(element.local_name(), "Canvas" | "Panel" | "RelativePanel")
            && builtin
        {
            return Ok(None);
        }
        let measured = measure(self, target, None)?;
        let mut helpers = snapshot(self, target, Some(position), &nodes, &measured)?;
        let placement = helpers
            .scene
            .placement
            .clone()
            .ok_or("No layout placement was supplied")?;
        let intent = PreviewIntent::Move(nodes.clone());
        if let Some(reason) = &placement.rejection {
            return Ok(Some(rejected(self, helpers, intent, reason.clone())));
        }
        if same_parent
            && placement.attributes.is_empty()
            && !self
                .runtime_layout_capabilities(target)
                .is_some_and(|capabilities| capabilities.reorder)
        {
            return Ok(None);
        }
        // Built-in non-Canvas moves depend on the semantic slot/cell/zone, not the
        // continuously changing pointer or guide geometry. Reuse the already accepted
        // transaction before calling planners that perform their own AST validation.
        // Custom providers and affine Canvas moves can depend on arbitrary coordinates.
        if builtin && !canvas {
            if let Some(previous) =
                previous.filter(|previous| reusable_placement(self, previous, &helpers, &intent))
            {
                return Ok(Some(accepted(
                    self,
                    helpers,
                    intent,
                    previous.edits.clone(),
                    placement.description,
                )));
            }
        }
        let plan = (|| {
            if self.content_target_container(target)? != placement.container {
                return Err("The layout provider chose an incompatible native content slot".into());
            }
            // Retain the pointer grab offset while preserving affine geometry. Include
            // displacement before normalization, which intentionally omits no-op setters.
            let canvas_delta = if canvas {
                measured.delta(LayoutPoint::new(delta.x, delta.y))?
            } else {
                LayoutPoint::new(0.0, 0.0)
            };
            let mut edits =
                self.plan_content_move(&nodes, target, placement.index, canvas_delta)?;
            if !canvas {
                let mut placement_edits = Vec::new();
                for node in &nodes {
                    if !placement.attributes.is_empty() {
                        placement_edits.extend(
                            ferroui_designer_tools::plan_layout_attributes(
                                self.document(),
                                *node,
                                &placement.attributes,
                            )
                            .map_err(|error| error.to_string())?,
                        );
                    }
                }
                placement_edits.extend(edits);
                edits = placement_edits;
            }
            Ok::<_, String>(snapshot_edits_first(edits))
        })();
        let edits = match plan {
            Ok(edits) => edits,
            Err(reason) => return Ok(Some(rejected(self, helpers, intent, reason))),
        };
        if let Err(reason) = validate_edits(self, &helpers, &intent, &edits, previous) {
            return Ok(Some(rejected(self, helpers, intent, reason)));
        }
        // A cross-Canvas move displays the translated selection extent as the landing area.
        // Transform every corner: the local AABB is conservative for rotated/sheared
        // parents, while the surface's separate control ghost preserves the exact shape.
        if canvas {
            let artboard = self.preview.host.clone().upcast::<Visual>();
            let bounds = nodes
                .iter()
                .filter_map(|node| self.preview.bounds(*node, &artboard))
                .reduce(|a, b| a.union(b));
            if let Some(bounds) = bounds {
                let landing = measured
                    .to_parent
                    .bounds(LayoutRect::new(
                        bounds.x + delta.x,
                        bounds.y + delta.y,
                        bounds.width,
                        bounds.height,
                    ))
                    .map_err(|error| error.to_string())?;
                if let Some(placement) = helpers.scene.placement.as_mut() {
                    placement.bounds = landing;
                    placement.description = format!(
                        "Place in Canvas · {}, {}",
                        compact(landing.x),
                        compact(landing.y)
                    );
                }
            }
        }
        let label = helpers
            .scene
            .placement
            .as_ref()
            .map_or(placement.description, |placement| {
                placement.description.clone()
            });
        Ok(Some(accepted(self, helpers, intent, edits, label)))
    }

    pub fn preview_layout_insert(
        &self,
        source: &str,
        position: Point,
    ) -> Result<LayoutEditPreview, String> {
        self.preview_layout_insert_cached(source, position, None)
    }

    pub fn preview_layout_insert_cached(
        &self,
        source: &str,
        position: Point,
        previous: Option<&LayoutEditPreview>,
    ) -> Result<LayoutEditPreview, String> {
        writable(self)?;
        point(position)?;
        let target = layout_target(self, position, &[])
            .ok_or("Choose a content holder or panel inside the artboard")?;
        self.ensure_layer_manipulable(target)?;
        let measured = measure(self, target, None)?;
        let helpers = snapshot(self, target, Some(position), &[], &measured)?;
        let placement = helpers
            .scene
            .placement
            .clone()
            .ok_or("No layout placement was supplied")?;
        let intent = PreviewIntent::Insert(source.to_owned());
        if let Some(reason) = &placement.rejection {
            return Ok(rejected(self, helpers, intent, reason.clone()));
        }
        if let Err(reason) = self.validate_content_insertion(target) {
            return Ok(rejected(self, helpers, intent, reason));
        }
        let cacheable = self
            .layout_tools()
            .is_builtin_adapter_for(self.document(), target)
            || !provider(self, target);
        if let Some(previous) = previous
            .filter(|previous| cacheable && reusable_placement(self, previous, &helpers, &intent))
        {
            return Ok(accepted(
                self,
                helpers,
                intent,
                previous.edits.clone(),
                placement.description,
            ));
        }
        let plan = (|| {
            let (embedded, declaration) =
                crate::preview_source::embed_native_fragment(self.document(), source)?;
            let mut planning = self.document().clone();
            let mut edits = Vec::new();
            if let Some(declaration) = declaration {
                planning
                    .apply("Declare native asset namespace", vec![declaration.clone()])
                    .map_err(|error| error.to_string())?;
                edits.push(declaration);
            }
            let context = measured.context(&planning);
            if provider(self, target) {
                let position = measured
                    .to_parent
                    .transform(LayoutPoint::new(position.x, position.y))
                    .map_err(|error| error.to_string())?;
                edits.extend(
                    self.layout_tools()
                        .plan(
                            &context,
                            LayoutAction::Drop {
                                parent: target,
                                source: embedded,
                                position,
                            },
                        )
                        .map_err(|error| error.to_string())?,
                );
            } else {
                edits.push(Edit::InsertChild {
                    parent: placement.container,
                    index: placement.index,
                    source: embedded,
                });
            }
            validate_edits(self, &helpers, &intent, &edits, previous)?;
            Ok::<_, String>(edits)
        })();
        match plan {
            Ok(edits) => Ok(accepted(
                self,
                helpers,
                intent,
                edits,
                placement.description,
            )),
            Err(reason) => Ok(rejected(self, helpers, intent, reason)),
        }
    }

    /// Stage a handle displacement using the press-time snapshot. The returned preview can
    /// be rendered freely; only commit_layout_preview performs an authored transaction.
    pub fn preview_layout_handle(
        &self,
        snapshot: &LayoutHelperSnapshot,
        handle_index: usize,
        delta: Point,
    ) -> Result<LayoutEditPreview, String> {
        let guard = snapshot
            .guard
            .as_ref()
            .ok_or("This render-only helper snapshot cannot author layout")?;
        guard.validate(self)?;
        let handle = snapshot
            .scene
            .handles
            .get(handle_index)
            .ok_or("The layout handle is no longer available")?;
        if let Some(reason) = &handle.disabled_reason {
            return Err(reason.clone());
        }
        ensure_helper_layer_target(self, snapshot.scene.panel, &handle.action)?;
        let frame = snapshot.handle_frames.get(&handle.id);
        let inverse = frame
            .map_or(snapshot.to_artboard, |frame| frame.to_artboard)
            .inverse()
            .map_err(|error| error.to_string())?;
        let origin = inverse
            .transform(LayoutPoint::new(0.0, 0.0))
            .map_err(|error| error.to_string())?;
        let end = inverse
            .transform(point(delta)?)
            .map_err(|error| error.to_string())?;
        let local = LayoutPoint::new(end.x - origin.x, end.y - origin.y);
        let value = handle
            .value_after_delta(local)
            .map_err(|error| error.to_string())?;
        let mut helpers = snapshot.clone();
        let visual_handle = &mut helpers.scene.handles[handle_index];
        visual_handle.value = value;
        let distance = (value - handle.value) / handle.direction;
        let local_shift = match handle.axis {
            Axis::Horizontal => LayoutPoint::new(distance, 0.0),
            Axis::Vertical => LayoutPoint::new(0.0, distance),
        };
        let shift = frames::transform_vector(
            frame.map_or(Affine::IDENTITY, |frame| frame.to_panel),
            local_shift,
        )?;
        visual_handle.position.x += shift.x;
        visual_handle.position.y += shift.y;
        let position = visual_handle.position;
        helpers
            .scene
            .primitives
            .push(ferroui_designer_tools::LayoutGuide {
                role: LayoutGuideRole::Measurement,
                geometry: LayoutGuideGeometry::Label {
                    position,
                    text: format!("{}  {}", handle.label, compact(value)),
                },
            });
        let intent = PreviewIntent::Handle(handle.id.clone());
        let measured = measure(self, snapshot.scene.panel, None)?;
        let result = self
            .layout_tools()
            .plan_helper(
                &measured.context(self.document()),
                snapshot.scene.panel,
                &handle.action,
                value,
            )
            .map_err(|error| error.to_string());
        let edits = match result {
            Ok(edits) => edits,
            Err(reason) => return Ok(rejected(self, helpers, intent, reason)),
        };
        // Registry::plan_helper validates the exact candidate transaction for both built-in
        // and extension providers. Retain that accepted plan without a second AST pass.
        Ok(accepted(
            self,
            helpers,
            intent,
            edits,
            format!("Adjust {}", handle.label),
        ))
    }

    /// Apply the exact accepted edit list once after rechecking source and provider identity.
    /// Structural placement and layout helpers remain ordinary source operations even while
    /// recording animation, as their edits may include both properties and child ownership.
    pub fn commit_layout_preview(&mut self, preview: &LayoutEditPreview) -> Result<(), String> {
        preview.guard.validate(self)?;
        if let Some(reason) = &preview.rejection {
            return Err(reason.clone());
        }
        match &preview.intent {
            PreviewIntent::Move(nodes) => {
                for node in nodes {
                    self.ensure_layer_manipulable(*node)?;
                }
                self.ensure_layer_container_context(preview.target(), nodes)?;
            }
            PreviewIntent::Insert(_) => self.ensure_layer_manipulable(preview.target())?,
            PreviewIntent::Handle(id) => {
                let handle = preview
                    .helpers
                    .scene
                    .handles
                    .iter()
                    .find(|handle| &handle.id == id)
                    .ok_or("The layout handle is no longer available")?;
                ensure_helper_layer_target(self, preview.target(), &handle.action)?;
            }
        }
        if preview.edits.is_empty() {
            return Ok(());
        }
        let change = self
            .apply_at(
                preview.guard.revision,
                preview.label.clone(),
                preview.edits.clone(),
            )
            .map_err(|error| error.to_string())?;
        match &preview.intent {
            PreviewIntent::Move(nodes) => self.select(nodes.iter().copied()),
            PreviewIntent::Insert(_) => {
                let inserted = change.inserted.iter().copied().collect::<BTreeSet<_>>();
                let roots = self
                    .document()
                    .ast()
                    .nodes_in_source_order()
                    .filter(|node| {
                        inserted.contains(&node.id)
                            && node.kind == NodeKind::Element
                            && node.parent.is_none_or(|parent| !inserted.contains(&parent))
                    })
                    .map(|node| node.id)
                    .collect::<Vec<_>>();
                if !roots.is_empty() {
                    self.select(roots);
                }
            }
            PreviewIntent::Handle(_) => {}
        }
        Ok(())
    }
}

fn provider(session: &DesignerSession, panel: NodeId) -> bool {
    session
        .layout_tools()
        .adapter_for(session.document(), panel)
        .is_some()
        && !is_custom_panel(session, panel)
}

fn selected_container(session: &DesignerSession, node: NodeId) -> bool {
    if session.runtime_layout_capabilities(node).is_none() {
        return false;
    }
    if session
        .preview
        .visual(node)
        .is_some_and(|visual| visual.cast::<ferroui_controls::Panel>().is_some())
    {
        return true;
    }
    let Some(element) = session.document().ast().node(node) else {
        return false;
    };
    let Ok(container) = session.content_target_container(node) else {
        return false;
    };
    let Some(content) = session.document().ast().node(container) else {
        return false;
    };
    let children = content
        .children
        .iter()
        .filter_map(|id| session.document().ast().node(*id));
    if children
        .clone()
        .any(|child| child.kind == NodeKind::Element)
    {
        return true;
    }
    !["Content", "Child"].iter().any(|name| {
        element
            .attribute(name)
            .is_some_and(|value| !value.is_empty())
    }) && !children.into_iter().any(|child| {
        matches!(child.kind, NodeKind::Text | NodeKind::CData)
            && child
                .text
                .as_ref()
                .is_some_and(|text| !text.trim().is_empty())
    })
}

fn snapshot(
    session: &DesignerSession,
    panel: NodeId,
    pointer: Option<Point>,
    dragged: &[NodeId],
    measured: &Measurement,
) -> Result<LayoutHelperSnapshot, String> {
    let pointer = pointer
        .map(|position| {
            measured
                .to_parent
                .transform(point(position)?)
                .map_err(|error| error.to_string())
        })
        .transpose()?;
    let native_margins = session
        .layout_tools()
        .is_builtin_adapter_for(session.document(), panel)
        && !is_custom_panel(session, panel);
    let request = LayoutHelperRequest {
        panel,
        pointer,
        dragged_nodes: dragged.to_vec(),
        selected_nodes: if native_margins {
            Vec::new()
        } else {
            session.selection().to_vec()
        },
    };
    let context = measured.context(session.document());
    let mut scene = if provider(session, panel) {
        session
            .layout_tools()
            .design_helpers(&context, &request)
            .map_err(|error| error.to_string())?
    } else {
        LayoutHelperScene::generic(&context, &request).map_err(|error| error.to_string())?
    };
    if let Some(pointer) = pointer.filter(|_| scene.placement.is_none()) {
        let placement = if provider(session, panel) {
            session
                .layout_tools()
                .placement(&context, panel, pointer, dragged)
                .map_err(|error| error.to_string())?
        } else {
            None
        };
        scene.placement = Some(match placement {
            Some(placement) => placement,
            None => {
                let container = session.content_target_container(panel)?;
                LayoutPlacement {
                    container,
                    index: session.document().ast().element_children(container).len(),
                    attributes: Vec::new(),
                    bounds: context.bounds(panel).map_err(|error| error.to_string())?,
                    description: format!("Place in {}", scene.title),
                    rejection: None,
                }
            }
        });
    }
    let handle_frames = if native_margins {
        frames::append_native_margins(session, panel, measured, &mut scene)?
    } else {
        BTreeMap::new()
    };
    if session
        .runtime_layout_capabilities(panel)
        .is_none_or(|capabilities| !capabilities.resize_items || capabilities.generated_items)
    {
        for handle in &mut scene.handles {
            handle.disabled_reason =
                Some("This native panel does not permit layout editing".into());
        }
    }
    for handle in &mut scene.handles {
        if handle.disabled_reason.is_none() {
            handle.disabled_reason =
                ensure_helper_layer_target(session, panel, &handle.action).err();
        }
    }
    scene.validate().map_err(|error| error.to_string())?;
    Ok(LayoutHelperSnapshot {
        scene,
        to_artboard: measured.to_artboard,
        guard: Some(LayoutGuard::capture(session)),
        handle_frames,
    })
}

fn ensure_helper_layer_target(
    session: &DesignerSession,
    panel: NodeId,
    action: &ferroui_designer_tools::LayoutHandleAction,
) -> Result<(), String> {
    use ferroui_designer_tools::LayoutHandleAction;
    let target = match action {
        LayoutHandleAction::PanelProperty { panel, .. }
        | LayoutHandleAction::GridTrack { panel, .. } => *panel,
        LayoutHandleAction::ThicknessSide { node, .. } => *node,
        LayoutHandleAction::Custom { .. } => panel,
    };
    session.ensure_layer_manipulable(target)
}

fn writable(session: &DesignerSession) -> Result<(), String> {
    session
        .ensure_writable()
        .map_err(|error| error.to_string())?;
    if !session.can_manipulate() {
        return Err("Repair the native preview before editing layout".into());
    }
    Ok(())
}
fn point(value: Point) -> Result<LayoutPoint, String> {
    let result = LayoutPoint::new(value.x, value.y);
    result.validate().map_err(|error| error.to_string())?;
    Ok(result)
}
fn compact(value: f64) -> String {
    format!("{value:.2}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}
fn accepted(
    session: &DesignerSession,
    helpers: LayoutHelperSnapshot,
    intent: PreviewIntent,
    edits: Vec<Edit>,
    label: String,
) -> LayoutEditPreview {
    LayoutEditPreview {
        helpers,
        guard: LayoutGuard::capture(session),
        edits,
        intent,
        rejection: None,
        label,
    }
}
fn rejected(
    session: &DesignerSession,
    mut helpers: LayoutHelperSnapshot,
    intent: PreviewIntent,
    reason: String,
) -> LayoutEditPreview {
    if let Some(placement) = &mut helpers.scene.placement {
        placement.rejection = Some(reason.clone());
        placement.description = reason.clone();
    }
    helpers
        .scene
        .primitives
        .push(ferroui_designer_tools::LayoutGuide {
            role: LayoutGuideRole::Rejected,
            geometry: LayoutGuideGeometry::Label {
                position: helpers
                    .scene
                    .handles
                    .first()
                    .map_or(LayoutPoint::new(12.0, 18.0), |handle| handle.position),
                text: reason.clone(),
            },
        });
    LayoutEditPreview {
        helpers,
        guard: LayoutGuard::capture(session),
        edits: Vec::new(),
        intent,
        rejection: Some(reason.clone()),
        label: reason,
    }
}
fn validate_edits(
    session: &DesignerSession,
    helpers: &LayoutHelperSnapshot,
    intent: &PreviewIntent,
    edits: &[Edit],
    previous: Option<&LayoutEditPreview>,
) -> Result<(), String> {
    if previous.is_some_and(|previous| {
        previous.is_current(session)
            && previous.is_accepted()
            && &previous.intent == intent
            && previous.edits == edits
            && previous.target() == helpers.scene.panel
    }) {
        return Ok(());
    }
    let mut candidate = session.document().clone();
    candidate
        .apply("Validate layout preview", edits.to_vec())
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn reusable_placement(
    session: &DesignerSession,
    previous: &LayoutEditPreview,
    helpers: &LayoutHelperSnapshot,
    intent: &PreviewIntent,
) -> bool {
    previous.is_current(session)
        && previous.is_accepted()
        && &previous.intent == intent
        && previous.target() == helpers.scene.panel
        && previous
            .helpers
            .scene
            .placement
            .as_ref()
            .zip(helpers.scene.placement.as_ref())
            .is_some_and(|(before, after)| {
                before.container == after.container
                    && before.index == after.index
                    && before.attributes == after.attributes
                    && after.rejection.is_none()
            })
}

/// Hit test measured authored content rather than painted pixels. Empty/transparent panels
/// are valid destinations. A pruned dragged subtree cannot intercept its future parent.
pub(crate) fn layout_target(
    session: &DesignerSession,
    position: Point,
    excluded: &[NodeId],
) -> Option<NodeId> {
    let root = session.preview.root()?.upcast::<Visual>();
    let artboard = session.preview.host.clone().upcast::<Visual>();
    let moving = excluded;
    let excluded = excluded.iter().copied().collect::<BTreeSet<_>>();
    fn visit(
        session: &DesignerSession,
        visual: &Ref<Visual>,
        artboard: &Ref<Visual>,
        position: Point,
        excluded: &BTreeSet<NodeId>,
        moving: &[NodeId],
    ) -> Option<NodeId> {
        if !visual.is_visible() {
            return None;
        }
        let source = session.preview.source_id(visual);
        if source.is_some_and(|id| excluded.contains(&id)) {
            return None;
        }
        if source.is_some_and(|node| {
            let state = session.layer_state(node);
            state.is_locked() || state.is_hidden() || state.outside_isolation
        }) {
            return None;
        }
        let local = artboard.translate_point(position, visual)?;
        let inside = ferroui_base::Rect::from_size(visual.bounds().size()).contains(local);
        if !inside && visual.clip_to_bounds() {
            return None;
        }
        if inside {
            if let Some(flow) = source
                .and_then(|source| flow_edge_parent(session, source, visual, artboard, position))
                .filter(|owner| {
                    session
                        .ensure_layer_container_context(*owner, moving)
                        .is_ok()
                })
            {
                return Some(flow);
            }
        }
        let mut children = visual
            .visual_children()
            .snapshot()
            .iter()
            .cloned()
            .enumerate()
            .collect::<Vec<_>>();
        children.sort_by_key(|(index, child)| (child.z_index(), *index));
        for (_, child) in children.iter().rev() {
            if let Some(target) = visit(session, child, artboard, position, excluded, moving) {
                return Some(target);
            }
        }
        if inside {
            if let Some(source) = source.filter(|id| {
                session.ensure_layer_container_context(*id, moving).is_ok()
                    && session
                        .runtime_layout_capabilities(*id)
                        .is_some_and(|capabilities| {
                            capabilities.drop_items && !capabilities.generated_items
                        })
            }) {
                return Some(source);
            }
        }
        None
    }
    visit(session, &root, &artboard, position, &excluded, moving)
}

/// A six-screen-DIP edge band makes zero-gap flow layouts insertable between children.
/// The central area still targets the content holder itself, including occupied-host errors.
fn flow_edge_parent(
    session: &DesignerSession,
    source: NodeId,
    visual: &Ref<Visual>,
    artboard: &Ref<Visual>,
    position: Point,
) -> Option<NodeId> {
    if !session
        .preview
        .visual(source)
        .is_some_and(|authored| authored.ptr_eq(visual))
    {
        return None;
    }
    let owner = parent(session, source).ok()?;
    if !session
        .layout_tools()
        .is_builtin_adapter_for(session.document(), owner)
    {
        return None;
    }
    let element = session.document().ast().node(owner)?;
    if !matches!(
        element.local_name(),
        "StackPanel" | "ReversibleStackPanel" | "WrapPanel" | "FlexPanel"
    ) {
        return None;
    }
    let owner_visual = session.preview.visual(owner)?;
    let properties = crate::workspace_tools::effective_properties(&owner_visual);
    let horizontal = if element.local_name() == "FlexPanel" {
        properties
            .get("Direction")
            .is_none_or(|direction| direction.starts_with("Row"))
    } else {
        properties
            .get("Orientation")
            .is_some_and(|orientation| orientation == "Horizontal")
    };
    let root = visual.visual_root()?;
    let transform = visual.transform_to_visual(&root)?;
    let position = artboard.translate_point(position, &root)?;
    let bounds = visual.bounds();
    let corners = [
        Point::new(0.0, 0.0),
        Point::new(bounds.width, 0.0),
        Point::new(bounds.width, bounds.height),
        Point::new(0.0, bounds.height),
    ]
    .map(|point| point.transform(transform));
    let edges: &[(usize, usize)] = if matches!(element.local_name(), "WrapPanel" | "FlexPanel") {
        &[(0, 1), (1, 2), (2, 3), (3, 0)]
    } else if horizontal {
        &[(1, 2), (3, 0)]
    } else {
        &[(0, 1), (2, 3)]
    };
    edges
        .iter()
        .any(|(a, b)| segment_distance(position, corners[*a], corners[*b]) <= 6.0)
        .then_some(owner)
}

fn segment_distance(point: Point, start: Point, end: Point) -> f64 {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let length = dx * dx + dy * dy;
    let fraction = if length > 0.0 {
        (((point.x - start.x) * dx + (point.y - start.y) * dy) / length).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (point.x - start.x - fraction * dx).hypot(point.y - start.y - fraction * dy)
}

#[cfg(test)]
#[path = "workspace_layout_helpers_tests.rs"]
mod tests;
