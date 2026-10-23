//! Atomic conversion to editable vector geometry. Rendering hosts supply exact
//! outlines through their public geometry APIs; core owns hierarchy and history.
use crate::*;
use std::collections::BTreeSet;

fn invalid(message: &str) -> DocumentError {
    DocumentError::Invalid(message.into())
}

/// A validated request to extract geometry in the first selected root's local
/// coordinates. The renderer may need real shaped text or evaluated booleans.
#[derive(Clone, Debug)]
pub struct FlattenRequest {
    pub roots: Vec<NodeId>,
    pub destination_world: Transform,
}

impl Editor {
    /// Converts one ordinary shape without a platform renderer. Hosts should use
    /// `flatten_selection_with_geometry` to include text, booleans and groups.
    pub fn flatten_selection(&mut self) -> Option<NodeId> {
        self.flatten_selection_with_geometry(|document, request| {
            if request.roots.len() != 1 {
                return Err(invalid(
                    "Combining layers requires the rendering host's geometry service.",
                ));
            }
            let node = &document.nodes[&request.roots[0]];
            if matches!(
                node.kind,
                NodeKind::Text | NodeKind::Boolean | NodeKind::Group
            ) {
                return Err(invalid(
                    "This layer requires the rendering host's outline service.",
                ));
            }
            Ok(outline_path(node))
        })
        .ok()
    }

    /// Preflights every affected layer before requesting outlines or creating
    /// history. In-place conversion preserves the surviving root's identity,
    /// parent, stacking position, affine transform, paint/stroke/effect stacks,
    /// interactions and layout-child settings. A failed operation changes no
    /// document, revision, selection, ID allocator, history or redo state.
    ///
    /// Multiple roots (and group children) can share one vector only when they
    /// are opaque, uniformly painted shapes without independent compositing.
    /// Heterogeneous artwork is rejected explicitly until flattening preserves
    /// independent region paint ownership in its replacement network.
    pub fn flatten_selection_with_geometry(
        &mut self,
        mut extract: impl FnMut(&Document, &FlattenRequest) -> Result<VectorPath, DocumentError>,
    ) -> Result<NodeId, DocumentError> {
        let selected: BTreeSet<_> = self.selection.iter().copied().collect();
        if selected.is_empty() {
            return Err(invalid(
                "Select a shape, text layer, boolean or compatible shape group to flatten.",
            ));
        }
        for id in &selected {
            if !self.document.nodes.contains_key(id) {
                return Err(DocumentError::MissingNode(*id));
            }
            if self.document.effective_locked(*id) {
                return Err(invalid("Unlock every selected layer before flattening."));
            }
            if self.document.page_of(*id) != Some(self.active_page) {
                return Err(invalid("Flatten layers on the active page only."));
            }
        }
        let roots: BTreeSet<_> = selected
            .iter()
            .copied()
            .filter(|id| {
                !selected
                    .iter()
                    .any(|ancestor| ancestor != id && self.document.is_ancestor(*ancestor, *id))
            })
            .collect();
        let first_root = roots
            .first()
            .ok_or_else(|| invalid("The selected hierarchy is cyclic."))?;
        let parent = self.document.nodes[first_root]
            .parent
            .ok_or_else(|| invalid("Selected layers must belong to a parent."))?;
        if roots
            .iter()
            .any(|id| self.document.nodes[id].parent != Some(parent))
        {
            return Err(invalid(
                "Flatten layers with the same parent to preserve their stacking order.",
            ));
        }
        let siblings = self
            .document
            .children(parent)
            .ok_or(DocumentError::InvalidParent(parent))?;
        let ordered: Vec<_> = siblings
            .iter()
            .copied()
            .filter(|id| roots.contains(id))
            .collect();
        if ordered.len() != roots.len() {
            return Err(invalid("The selected hierarchy is inconsistent."));
        }
        if ordered.len() > 1 {
            let first = siblings.iter().position(|id| *id == ordered[0]).unwrap();
            if siblings[first..first + ordered.len()]
                .iter()
                .any(|id| !roots.contains(id))
            {
                return Err(invalid("Select adjacent layers to flatten without changing other layers' stacking order."));
            }
            if self
                .document
                .nodes
                .get(&parent)
                .is_some_and(|node| node.layout.is_some())
            {
                return Err(invalid(
                    "Flatten each automatic-layout child separately to retain layout placement.",
                ));
            }
        }
        let survivor = ordered[0];
        let original = &self.document.nodes[&survivor];
        if ordered.len() == 1 && original.kind == NodeKind::Vector {
            return Ok(survivor);
        }
        let destination_world = self
            .document
            .world_transform(survivor)
            .filter(|world| world.inverse().is_some())
            .ok_or_else(|| invalid("The selected transform is singular or unavailable."))?;
        let mut removed = BTreeSet::new();
        for root in &ordered {
            for id in self.document.descendants(*root) {
                let node = &self.document.nodes[&id];
                if node.vector_network.as_ref().is_some_and(|network|
                    network.regions.values().any(|region| region.fills.is_some())) {
                    return Err(invalid("This vector has independent region paints. Keep its network representation to preserve their paint stacks and transparent regions."));
                }
                if id != survivor {
                    removed.insert(id);
                }
                if self.document.effective_locked(id) {
                    return Err(invalid(
                        "Unlock all affected descendants before flattening.",
                    ));
                }
                if node.component_source.is_some()
                    || matches!(node.kind, NodeKind::Component | NodeKind::Instance)
                {
                    return Err(invalid(
                        "Detach component instances before flattening their structure.",
                    ));
                }
                if node.is_mask {
                    return Err(invalid("Flattening a mask scope would change sibling compositing; remove the mask role first."));
                }
            }
        }
        // References must not disappear incidentally when the structural edit
        // removes operands or child layers. Keeping the root ID preserves all
        // references to the root itself.
        if !removed.is_empty()
            && (self
                .document
                .comments
                .iter()
                .any(|comment| comment.node.is_some_and(|id| removed.contains(&id)))
                || self
                    .document
                    .pages
                    .iter()
                    .any(|page| page.prototype_start.is_some_and(|id| removed.contains(&id)))
                || self.document.nodes.values().any(|node| {
                    node.interactions
                        .iter()
                        .any(|interaction| match interaction.action {
                            PrototypeAction::NavigateTo { target }
                            | PrototypeAction::OpenOverlay { target, .. }
                            | PrototypeAction::ScrollTo { target } => removed.contains(&target),
                            _ => false,
                        })
                })
                || self.document.nodes.values().any(|node| {
                    node.component_id.is_some_and(|id| removed.contains(&id))
                        || node
                            .component_source
                            .is_some_and(|id| removed.contains(&id))
                })
                || removed
                    .iter()
                    .any(|id| !self.document.nodes[id].interactions.is_empty()))
        {
            return Err(invalid("A removed layer owns interactions or is referenced by comments, prototypes or components; flatten it separately to preserve those references."));
        }

        let compound = ordered.len() > 1 || original.kind == NodeKind::Group;
        let fills = if compound {
            let mut common: Option<Vec<Fill>> = None;
            for root in &ordered {
                uniform_fills(
                    &self.document,
                    *root,
                    *root == survivor && ordered.len() == 1,
                    &mut common,
                )?;
            }
            common
                .ok_or_else(|| invalid("The selected group contains no visible filled shapes."))?
        } else {
            validate_single(original)?;
            effective_fills(&self.document, survivor)
        };
        let request = FlattenRequest {
            roots: ordered.clone(),
            destination_world,
        };
        let path = extract(&self.document, &request)?;
        if path.segments.is_empty()
            || path.tight_bounds().is_none()
            || !path.validate_anchor_modes()
        {
            return Err(invalid(
                "The active renderer could not produce a finite, nonempty editable outline.",
            ));
        }
        let mut result = original.clone();
        result.kind = NodeKind::Vector;
        result.children.clear();
        result.path = path;
        result.vector_network = None;
        result.fills = fills;
        result.text_style = None;
        result.text.clear();
        result.text_runs.clear();
    result.text_paragraphs.clear();
        result.font_features.clear();
        result.layout = None;
        result.layout_grid = None;
        result.layout_grid_overlays.clear();
        result.grid_style = None;
        result.clip_content = false; // Any group/boolean clip is baked into the outline.
        result.corner_radius = 0.0;
        result.corner_radii = None;
        result.corner_smoothing = 0.0;
        if compound {
            result.fill_style = None;
            // One existing group retains its allocation and parent-layout
            // footprint. Only a multi-root union needs a new bounding box.
            if ordered.len() > 1 && !normalize_vector_bounds(&mut result) {
                return Err(invalid(
                    "The combined outline exceeds finite geometry bounds.",
                ));
            }
        }
        // Font/text bindings no longer describe vector artwork. Geometry and
        // paint/effect bindings retain their meaning for in-place conversion.
        crate::remove_layout_guide_bindings(&mut result);
        crate::layout_variables::remove_layout_variable_bindings(&mut result);
        result.variable_bindings.retain(|property, _| {
            !matches!(
                property.as_str(),
                "text"
                    | "font_size"
                    | "font_family"
                    | "font_weight"
                    | "letter_spacing"
                    | "line_height"
            )
        });
        result
            .overridden_properties
            .extend(["kind".into(), "path".into()]);
        if removed.is_empty() {
            // The common single-leaf operation changes no structure or global
            // metadata. Keep its history and renderer invalidation scoped.
            self.edit_nodes("Flatten to vector", &[survivor], |editor| {
                editor.document.nodes.insert(survivor, result);
                editor.selection = vec![survivor];
            });
            return Ok(survivor);
        }
        let mut candidate = self.document.clone();
        for id in &removed {
            candidate.nodes.remove(id);
            candidate.guides.remove(id);
        }
        candidate.nodes.insert(survivor, result);
        candidate
            .children_mut(parent)
            .ok_or(DocumentError::InvalidParent(parent))?
            .retain(|id| !removed.contains(id));
        candidate.validate()?;
        self.edit("Flatten to vector", |editor| {
            editor.document = candidate;
            editor.selection = vec![survivor];
        });
        Ok(survivor)
    }
}

fn effective_fills(document: &Document, id: NodeId) -> Vec<Fill> {
    let node = &document.nodes[&id];
    if node.kind == NodeKind::Boolean
        && !node
            .fills
            .iter()
            .any(|fill| fill.visible && fill.opacity > 0.0)
    {
        if let Some(child) = node.children.iter().find(|id| {
            document
                .nodes
                .get(id)
                .is_some_and(|node| node.visible && node.opacity > 0.0)
        }) {
            return effective_fills(document, *child);
        }
    }
    node.fills.clone()
}

fn validate_single(node: &Node) -> Result<(), DocumentError> {
    if node.kind.is_container() && node.kind != NodeKind::Boolean && !node.children.is_empty() {
        return Err(invalid("Flatten a frame's artwork separately to retain its independent paints, clips and layout."));
    }
    let box_shape = node.corner_smoothing == 0.0 && matches!(
        node.kind,
        NodeKind::Rectangle | NodeKind::Frame | NodeKind::Section
    );
    if box_shape
        && node.effects.iter().any(|effect| {
            effect.visible && effect.color.a > 0 && effect.kind == EffectKind::DropShadow
        })
        && !node.fills.iter().any(fully_opaque_fill)
    {
        // Box shadows exclude the original silhouette. A vector shadow paints
        // behind its silhouette instead, so an absent or translucent fill
        // exposes shadow pixels that the box never painted. Layer opacity is
        // applied after this composition and does not affect this condition.
        return Err(invalid("Box-shaped drop shadows exclude the layer interior; flatten only with a fully opaque fill to preserve their appearance."));
    }
    if box_shape
        && node
            .corner_radii
            .unwrap_or([node.corner_radius; 4])
            .iter()
            .any(|radius| *radius <= 0.0)
        && node.effects.iter().any(|effect| {
            effect.visible
                && effect.color.a > 0
                && match effect.kind {
                    EffectKind::DropShadow => effect.spread > 0.0,
                    EffectKind::InnerShadow => effect.spread < 0.0,
                    _ => false,
                }
        })
    {
        // Box-shadow expansion retains square corners. Expanding a general
        // vector uses a round-join stroke, which rounds those shadow corners.
        return Err(invalid("Outward box-shadow spread preserves square corners that a vector shadow rounds; remove that spread or round every corner before flattening."));
    }
    if node.kind.is_container()
        && node.kind != NodeKind::Boolean
        && node.effects.iter().any(|effect| {
            effect.visible && effect.color.a > 0 && effect.kind == EffectKind::InnerShadow
        })
        && node
            .strokes
            .iter()
            .any(|stroke| stroke.visible && stroke.opacity > 0.0 && stroke.width > 0.0)
    {
        // Empty frames/sections are still containers: their inner shadow is
        // above the stroke, while a vector's inner shadow is below its stroke.
        return Err(invalid("A container's inner shadows cover its strokes; flatten the stroke or shadow separately to preserve that ordering."));
    }
    if node.kind == NodeKind::Text {
        // Range colors are painted by individual glyph runs. One vector paint
        // stack cannot represent those independent regions yet.
        if node.text_runs.iter().any(|run| run.color.is_some()) {
            return Err(invalid("Text with individual run colors needs separate vector regions; flatten uniformly painted text."));
        }
        if node
            .fills
            .iter()
            .any(|fill| fill.visible && !matches!(fill.paint, Paint::Solid(_)))
        {
            return Err(invalid(
                "Text gradient/image coordinates differ per glyph run; flatten solid-painted text.",
            ));
        }
        if node.fills.iter().any(|fill| {
            fill.visible
                && fill.opacity > 0.0
                && matches!(fill.paint, Paint::Solid(color)
                    if color.a > 0 && (color.a < 255 || fill.opacity < 1.0))
        }) {
            // Text paints every glyph and decoration separately. A translucent
            // paint accumulates alpha where they overlap; one filled path
            // cannot retain that accumulation. Whole-layer opacity remains
            // supported because the renderer applies it to the isolated node.
            return Err(invalid("Translucent text fills composite overlapping glyphs and decorations separately; keep the text or use opaque fills before flattening."));
        }
    }
    Ok(())
}

fn fully_opaque_fill(fill: &Fill) -> bool {
    fill.visible
        && fill.opacity == 1.0
        && match &fill.paint {
            Paint::Solid(color) => color.a == 255,
            Paint::LinearGradient { stops, .. } | Paint::RadialGradient { stops, .. }
            | Paint::AngularGradient { stops, .. } => {
                !stops.is_empty() && stops.iter().all(|stop| stop.color.a == 255)
            }
            // Opacity of source pixels (or empty space from image fitting)
            // cannot be established from the image paint's metadata alone.
            Paint::Image { .. } => false,
        }
}

fn uniform_fills(
    document: &Document,
    id: NodeId,
    root_scope: bool,
    common: &mut Option<Vec<Fill>>,
) -> Result<(), DocumentError> {
    let node = &document.nodes[&id];
    if !node.visible || node.opacity != 1.0 && !root_scope {
        return Err(invalid(
            "Flatten a group only when all children are visible and fully opaque.",
        ));
    }
    if !root_scope
        && (!matches!(node.blend_mode, BlendMode::Normal | BlendMode::PassThrough)
            || node.effects.iter().any(|effect| effect.visible))
    {
        return Err(invalid(
            "Flatten each layer separately to preserve independent blending and effects.",
        ));
    }
    if node
        .strokes
        .iter()
        .any(|stroke| stroke.visible && stroke.opacity > 0.0 && stroke.width > 0.0)
    {
        return Err(invalid(
            "Flatten stroked layers separately to preserve their independent stroke paints.",
        ));
    }
    if node.kind == NodeKind::Group {
        if !node.fills.is_empty() || node.clip_content || node.layout.is_some() {
            return Err(invalid(
                "Flatten groups without their own paints, clipping or automatic layout.",
            ));
        }
        for child in &node.children {
            uniform_fills(document, *child, false, common)?;
        }
        return Ok(());
    }
    validate_single(node)?;
    let fills = effective_fills(document, id);
    if fills.len() != 1
        || !fills[0].visible
        || fills[0].opacity != 1.0
        || !matches!(fills[0].paint, Paint::Solid(color) if color.a == 255)
    {
        return Err(invalid("Combining layers requires one shared opaque solid fill; flatten other paint stacks separately."));
    }
    if let Some(common) = common {
        if *common != fills {
            return Err(invalid("Different layer colors need independently painted vector regions; flatten those layers separately."));
        }
    } else {
        *common = Some(fills);
    }
    Ok(())
}

#[cfg(test)]
#[path = "flatten_tests.rs"]
mod tests;
