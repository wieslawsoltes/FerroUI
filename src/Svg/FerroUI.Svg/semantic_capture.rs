//! Explicit UI-thread snapshots. Capture never divides a rendered compositing island.

use crate::semantics::SemanticTextBudget;
use crate::{
    SvgError, SvgSemanticAccessibility, SvgSemanticGridPlacement, SvgSemanticLayout,
    SvgSemanticNode, SvgSemanticState, SvgSemanticTree,
};
use ferroui_base::input::InputElement;
use ferroui_base::layout::Layoutable;
use ferroui_base::{FerroObject, Matrix, Rect, Ref, Visual};
use ferroui_controls::automation::peers::ControlAutomationPeer;
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::primitives::AccessText;
use ferroui_controls::{Control, Grid, TextBlock, TextBox};
use std::collections::HashMap;

/// An application callback that customizes a captured node without retaining its visual.
pub type SvgSemanticCustomizer<'a> = dyn FnMut(&Visual, &mut SvgSemanticNode) + 'a;

/// Which accessibility values are allowed into a semantic snapshot.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SvgSemanticAccessibilityMode {
    /// Do not read or create accessibility peers or copy authored accessibility text.
    #[default]
    None,
    /// Read explicit attached properties, without creating or consulting an automation peer.
    AttachedProperties,
    /// Resolve control peers, including their role and computed name/help text.
    ///
    /// This can create a peer and execute application overrides. Computed text can contain
    /// application/user content, including validation messages. No providers are invoked.
    AutomationPeer,
}

/// Policy for an explicit visual-tree snapshot. Ordinary drawing never performs this walk.
#[derive(Clone, Debug)]
pub struct SvgSemanticCaptureOptions {
    /// Maximum number of captured nodes, including the root. Must be positive.
    pub max_nodes: usize,
    /// Maximum descendant depth; the root is depth zero.
    pub max_depth: usize,
    /// Include branches that the immediate renderer skips for visibility, suppression or
    /// zero opacity. This does not attempt occlusion or clip visibility analysis.
    pub include_hidden: bool,
    pub include_names: bool,
    /// Includes authored style classes and `:`-prefixed pseudoclasses.
    pub include_classes: bool,
    pub include_layout: bool,
    pub include_input_state: bool,
    /// Copy TextBlock text, including AccessText with its access-key marker removed.
    /// TextBox subtrees/template content, passwords, data contexts and arbitrary properties
    /// are excluded from automatic text collection.
    pub include_text: bool,
    pub accessibility: SvgSemanticAccessibilityMode,
}

impl Default for SvgSemanticCaptureOptions {
    fn default() -> Self {
        Self {
            max_nodes: SvgSemanticTree::MAX_NODES,
            max_depth: 256,
            include_hidden: true,
            include_names: true,
            include_classes: true,
            include_layout: true,
            include_input_state: true,
            include_text: false,
            accessibility: SvgSemanticAccessibilityMode::None,
        }
    }
}

struct PendingNode {
    visual: Ref<Visual>,
    parent: Option<usize>,
    parent_transform: Matrix,
    path: String,
    depth: usize,
    suppress_text: bool,
}

struct NodeRelations {
    visual: Ref<Visual>,
    parent: Option<usize>,
    logical_parent: Option<Ref<FerroObject>>,
    templated_parent: Option<Ref<FerroObject>>,
}

/// Snapshot a laid-out visual tree without rendering it, performing layout, or retaining UI
/// objects in the result. Call on the root's UI thread after the desired layout pass.
///
/// Nodes follow visual-collection preorder, not Z-sorted paint order. The default `v0-0-1`
/// keys are deterministic for an unchanged hierarchy; applications requiring identity across
/// insertions/reordering should provide stable keys through `customize`.
///
/// `layout_bounds` is the arranged rectangle in the visual's parent space. `local_to_layer`
/// maps coordinates relative to the visual's own origin into the same layer coordinates used
/// by `ImmediateRenderer::render`: the root's layout translation is omitted, while its render,
/// mirror and manipulation transforms are included. Bounds are layout bounds, not visible ink
/// bounds; clipping and occlusion are not inferred.
///
/// The callback runs after the source snapshot is complete, once per included node. It may
/// replace IDs and add application metadata, but must not mutate the UI tree. Parent, logical
/// parent and template-parent links are derived from the captured tree and resolved against
/// final IDs after every callback. Relationships to objects outside the snapshot are omitted.
/// No text, accessibility content or source information is collected unless explicitly enabled
/// or supplied by that callback. Callback-supplied fields are validated by `SvgSemanticTree`.
pub fn capture_visual_semantics(
    root: &Ref<Visual>,
    options: &SvgSemanticCaptureOptions,
    mut customize: Option<&mut SvgSemanticCustomizer<'_>>,
) -> Result<SvgSemanticTree, SvgError> {
    root.verify_access();
    if options.max_nodes == 0 || options.max_nodes > SvgSemanticTree::MAX_NODES {
        return Err(SvgError::InvalidOptions(format!(
            "semantic capture max_nodes must be in 1..={}",
            SvgSemanticTree::MAX_NODES
        )));
    }

    let mut pending = vec![PendingNode {
        visual: root.clone(),
        parent: None,
        parent_transform: Matrix::IDENTITY,
        path: "v0".into(),
        depth: 0,
        suppress_text: options.include_text && has_text_box_ancestor(root),
    }];
    let mut nodes = Vec::new();
    let mut budget = SemanticTextBudget::default();
    let mut relations = Vec::new();
    let mut identities = HashMap::<Ref<FerroObject>, usize>::new();

    while let Some(entry) = pending.pop() {
        let visual = &entry.visual;
        if !options.include_hidden && skipped_by_immediate_renderer(visual) {
            continue;
        }
        if entry.depth > options.max_depth {
            return Err(SvgError::InvalidOptions(format!(
                "semantic capture exceeds maximum depth {}",
                options.max_depth
            )));
        }
        if nodes.len() == options.max_nodes {
            return Err(SvgError::InvalidOptions(format!(
                "semantic capture exceeds maximum node count {}",
                options.max_nodes
            )));
        }
        let index = nodes.len();
        if identities
            .insert(visual.clone().upcast::<FerroObject>(), index)
            .is_some()
        {
            return Err(SvgError::InvalidLayer(
                "semantic capture requires an acyclic visual tree without repeated nodes".into(),
            ));
        }
        let bounds = visual.bounds();
        validate_bounds(bounds)?;
        let local_to_layer =
            immediate_local_transform(visual, entry.parent.is_none()) * entry.parent_transform;
        validate_transform(local_to_layer)?;

        let suppress_text = options.include_text
            && (entry.suppress_text
                || visual.is::<TextBox>()
                || visual
                    .templated_parent()
                    .is_some_and(|parent| parent.is::<TextBox>()));
        let mut node = SvgSemanticNode::new(entry.path.clone(), visual.get_type().full_name());
        node.layout_bounds = bounds;
        node.local_to_layer = local_to_layer;
        node.name = options.include_names.then(|| visual.name()).flatten();
        if options.include_classes {
            node.classes = visual.classes().snapshot().as_ref().clone();
        }
        node.state = capture_state(visual, options.include_input_state);
        if options.include_layout {
            node.layout = visual.downcast_ref::<Layoutable>().map(capture_layout);
            if visual
                .visual_parent()
                .is_some_and(|parent| parent.is::<Grid>())
            {
                if let (Some(control), Some(layout)) =
                    (visual.downcast_ref::<Control>(), node.layout.as_mut())
                {
                    layout.grid = Some(SvgSemanticGridPlacement {
                        row: Grid::get_row(control),
                        column: Grid::get_column(control),
                        row_span: Grid::get_row_span(control),
                        column_span: Grid::get_column_span(control),
                    });
                }
            }
        }
        if options.include_text && !suppress_text {
            node.text = capture_text(visual);
        }
        node.accessibility = capture_accessibility(visual, options.accessibility);
        budget.observe(&node)?;
        nodes.push(node);
        relations.push(NodeRelations {
            visual: visual.clone(),
            parent: entry.parent,
            logical_parent: visual.parent().map(Ref::upcast::<FerroObject>),
            templated_parent: visual.templated_parent(),
        });

        if let Some(children) = visual.visual_children_snapshot() {
            for (child_index, child) in children.iter().enumerate().rev() {
                if !options.include_hidden && skipped_by_immediate_renderer(child) {
                    continue;
                }
                // Bound queued work as well as completed nodes. A very wide tree cannot
                // allocate an unbounded pending stack before the next iteration checks it.
                if pending.len() >= options.max_nodes - nodes.len() {
                    return Err(SvgError::InvalidOptions(format!(
                        "semantic capture exceeds maximum node count {}",
                        options.max_nodes
                    )));
                }
                if entry.depth >= options.max_depth {
                    return Err(SvgError::InvalidOptions(format!(
                        "semantic capture exceeds maximum depth {}",
                        options.max_depth
                    )));
                }
                let path = format!("{}-{child_index}", entry.path);
                if path.len() > SvgSemanticTree::MAX_KEY_BYTES {
                    return Err(SvgError::InvalidOptions(
                        "semantic capture structural key exceeds the maximum key length".into(),
                    ));
                }
                pending.push(PendingNode {
                    visual: child.clone(),
                    parent: Some(index),
                    parent_transform: local_to_layer,
                    path,
                    depth: entry.depth + 1,
                    suppress_text,
                });
            }
        }
    }

    if let Some(customize) = customize.as_mut() {
        for (node, relation) in nodes.iter_mut().zip(&relations) {
            customize(&relation.visual, node);
        }
    }
    let final_ids: Vec<String> = nodes.iter().map(|node| node.id.clone()).collect();
    for (node, relation) in nodes.iter_mut().zip(&relations) {
        node.parent = relation.parent.map(|index| final_ids[index].clone());
        node.logical_parent = relation
            .logical_parent
            .as_ref()
            .and_then(|parent| identities.get(parent))
            .map(|index| final_ids[*index].clone());
        node.templated_parent = relation
            .templated_parent
            .as_ref()
            .and_then(|parent| identities.get(parent))
            .map(|index| final_ids[*index].clone());
    }
    SvgSemanticTree::new(nodes)
}

fn skipped_by_immediate_renderer(visual: &Visual) -> bool {
    let opacity = visual.opacity();
    !visual.is_visible() || visual.is_render_suppressed() || opacity <= 0.0 || opacity.is_nan()
}

fn immediate_local_transform(visual: &Visual, root: bool) -> Matrix {
    let bounds = if root {
        Rect::from_size(visual.bounds().size())
    } else {
        visual.bounds()
    };
    let position = Matrix::create_translation(bounds.x, bounds.y);
    let mut transform = match visual.render_transform().map(|value| value.value()) {
        Some(render) => {
            let origin = visual
                .render_transform_origin()
                .to_pixels(visual.bounds().size());
            let offset = Matrix::create_translation(origin.x, origin.y);
            (-offset) * render * offset * position
        }
        None => position,
    };
    if let Some(manipulation) = visual.manipulation_transform() {
        transform = transform * (-position) * manipulation * position;
    }
    if visual.has_mirror_transform() {
        transform = Matrix::new(-1.0, 0.0, 0.0, 1.0, visual.bounds().width, 0.0) * transform;
    }
    transform
}

fn validate_bounds(bounds: Rect) -> Result<(), SvgError> {
    if bounds.width < 0.0
        || bounds.height < 0.0
        || [
            bounds.x,
            bounds.y,
            bounds.width,
            bounds.height,
            bounds.right(),
            bounds.bottom(),
        ]
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(SvgError::InvalidLayer(
            "semantic layout bounds must be finite and nonnegative in size".into(),
        ));
    }
    Ok(())
}

fn validate_transform(transform: Matrix) -> Result<(), SvgError> {
    if [
        transform.m11,
        transform.m12,
        transform.m13,
        transform.m21,
        transform.m22,
        transform.m23,
        transform.m31,
        transform.m32,
        transform.m33,
    ]
    .iter()
    .any(|value| !value.is_finite())
    {
        return Err(SvgError::InvalidLayer(
            "semantic visual transforms must remain finite after composition".into(),
        ));
    }
    Ok(())
}

fn capture_state(visual: &Visual, include_input: bool) -> SvgSemanticState {
    let input = include_input
        .then(|| visual.downcast_ref::<InputElement>())
        .flatten();
    SvgSemanticState {
        visible: visual.is_visible(),
        effectively_visible: visual.is_effectively_visible(),
        render_suppressed: visual.is_render_suppressed(),
        clip_to_bounds: visual.clip_to_bounds(),
        has_clip: visual.clip().is_some(),
        has_opacity_mask: visual.opacity_mask().is_some(),
        has_effect: visual.effect().is_some(),
        opacity: visual.opacity(),
        z_index: visual.z_index(),
        enabled: input.map(InputElement::is_enabled),
        effectively_enabled: input.map(InputElement::is_effectively_enabled),
        focusable: input.map(InputElement::focusable),
        focused: input.map(InputElement::is_focused),
        hit_test_visible: input.map(InputElement::is_hit_test_visible),
        is_tab_stop: input.map(InputElement::is_tab_stop),
        tab_index: input.map(InputElement::tab_index),
    }
}

fn capture_layout(layout: &Layoutable) -> SvgSemanticLayout {
    let optional = |value: f64| value.is_finite().then_some(value);
    SvgSemanticLayout {
        desired_size: layout.desired_size(),
        margin: layout.margin(),
        width: optional(layout.width()),
        height: optional(layout.height()),
        max_width: optional(layout.max_width()),
        max_height: optional(layout.max_height()),
        min_width: layout.min_width(),
        min_height: layout.min_height(),
        horizontal_alignment: format!("{:?}", layout.horizontal_alignment()),
        vertical_alignment: format!("{:?}", layout.vertical_alignment()),
        measure_valid: layout.is_measure_valid(),
        arrange_valid: layout.is_arrange_valid(),
        grid: None,
    }
}

fn has_text_box_ancestor(root: &Visual) -> bool {
    let mut current = root.visual_parent();
    while let Some(visual) = current {
        if visual.is::<TextBox>()
            || visual
                .templated_parent()
                .is_some_and(|parent| parent.is::<TextBox>())
        {
            return true;
        }
        current = visual.visual_parent();
    }
    false
}

fn capture_text(visual: &Visual) -> Option<String> {
    let text_block = visual.downcast_ref::<TextBlock>()?;
    if text_block.has_complex_content() {
        // Inlines() creates its empty collection on first access. The content check keeps
        // this read from allocating a collection or raising a property change for plain text.
        text_block.inlines().and_then(|inlines| inlines.text())
    } else if visual.is::<AccessText>() {
        AccessText::remove_access_key_marker(text_block.text().as_deref())
    } else {
        text_block.text()
    }
}

fn capture_accessibility(
    visual: &Visual,
    mode: SvgSemanticAccessibilityMode,
) -> Option<SvgSemanticAccessibility> {
    if mode == SvgSemanticAccessibilityMode::None {
        return None;
    }
    let mut result = SvgSemanticAccessibility {
        automation_id: AutomationProperties::get_automation_id(visual),
        name: AutomationProperties::get_name(visual),
        help_text: AutomationProperties::get_help_text(visual),
        role: AutomationProperties::get_control_type_override(visual)
            .map(|value| format!("{value:?}")),
        landmark_type: AutomationProperties::get_landmark_type(visual)
            .map(|value| format!("{value:?}")),
        live_setting: Some(format!(
            "{:?}",
            AutomationProperties::get_live_setting(visual)
        )),
        required: Some(AutomationProperties::get_is_required_for_form(visual)),
        heading_level: Some(AutomationProperties::get_heading_level(visual)),
        position_in_set: Some(AutomationProperties::get_position_in_set(visual)),
        size_of_set: Some(AutomationProperties::get_size_of_set(visual)),
    };
    if mode == SvgSemanticAccessibilityMode::AutomationPeer {
        if let Some(control) = visual.downcast_ref::<Control>() {
            let peer = ControlAutomationPeer::create_peer_for_element(control);
            let nonempty = |value: String| (!value.is_empty()).then_some(value);
            result.automation_id = peer.get_automation_id();
            result.name = nonempty(peer.get_name());
            result.help_text = nonempty(peer.get_help_text());
            result.role = Some(format!("{:?}", peer.get_automation_control_type()));
            result.landmark_type = peer.get_landmark_type().map(|value| format!("{value:?}"));
            result.heading_level = Some(peer.get_heading_level());
            result.live_setting = Some(format!("{:?}", peer.get_live_setting()));
        }
    }
    Some(result)
}

#[cfg(test)]
#[path = "semantic_capture_tests.rs"]
mod tests;
