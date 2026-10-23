//! Incremental preparation is available only through Editor's complete change
//! feed. The public Document renderer keeps its conservative full refresh path.

use super::*;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Default)]
pub(super) struct PreparationStats {
    pub(super) full: usize,
    pub(super) incremental: usize,
    pub(super) nodes: usize,
    pub(super) visuals: usize,
}

/// Lifetime work counters, retained across clear/invalidate and read without
/// depending on which cached frame happened to be drawn most recently.
#[derive(Clone, Copy, Default)]
pub(super) struct PreparationTotals {
    pub(super) full: u64,
    pub(super) incremental: u64,
    pub(super) nodes: u64,
    pub(super) visuals: u64,
}

impl ScenePainter {
    pub(super) fn ensure_editor_prepared(&mut self, editor: &Editor, roots: &[NodeId]) -> bool {
        self.preparation = PreparationStats::default();
        let document = &editor.document;
        let key = self.scene_key_for(document, roots);
        let token = editor.render_change_token();
        // The roots, asset revision, document/map identity, and editor identity
        // must all remain continuous before consulting a scoped change list.
        let same_scene = self
            .scene_key
            .is_some_and(|old| old.0 == key.0 && old.2 == key.2);
        if same_scene {
            if let Some(previous) = self.editor_token.as_ref() {
                if previous == &token && self.scene_key == Some(key) {
                    return true;
                }
                if let EditorRenderChanges::Nodes { nodes, ancestors } =
                    editor.render_changes_since(previous)
                {
                    let unchanged = nodes.is_empty() && ancestors.is_empty();
                    if unchanged || self.prepare_incremental(document, &nodes, &ancestors) {
                        self.scene_key = Some(key);
                        self.editor_token = Some(token);
                        return unchanged;
                    }
                }
            }
        }
        self.prepare(document, roots);
        self.scene_key = Some(key);
        self.editor_token = Some(token);
        false
    }

    fn prepare_incremental(
        &mut self,
        document: &Document,
        nodes: &[NodeId],
        ancestors: &[NodeId],
    ) -> bool {
        if !self.incremental_ready || self.prepare_invalid_nodes != 0 {
            return false;
        }
        let mut dirty = BTreeSet::new();
        for (id, is_ancestor) in nodes
            .iter()
            .map(|id| (id, false))
            .chain(ancestors.iter().map(|id| (id, true)))
        {
            let Some(node) = document.nodes.get(id) else {
                return false;
            };
            // Hidden nodes, external ancestors of exported roots, and unusual
            // overlapping root lists deliberately retain full preparation.
            let Some(&index) = self.prepared_indices.get(id) else {
                // Entering/leaving inline editing invalidates the scene. With
                // that state and the editor ledger still continuous, this
                // exact leaf remains absent from preparation because the
                // native text editor paints it. Its ancestors are NOT skipped:
                // the remaining ledger entries still refresh their transforms,
                // derived geometry and bounds through the normal path below.
                if !is_ancestor
                    && self.editing_text == Some(*id)
                    && node.kind == NodeKind::Text
                    && node.children.is_empty()
                    && node.visible
                    && node.opacity > 0.0
                    && !node.is_mask
                    && valid_node_bounds(node)
                {
                    continue;
                }
                return false;
            };
            if !node.visible
                || (node.opacity <= 0.0 && !node.is_mask)
                || !valid_node_bounds(node)
                || self.editing_text == Some(*id)
                // A mask role changes the following siblings' linked scopes.
                // Rebuild that structural dependency once, never per pointer.
                || self.prepared[index].mask.is_some() != node.is_mask
            {
                return false;
            }
            dirty.insert(index);
            // The cached isolation count belongs to the previous scene until
            // postorder preparation. Role/blend transitions need its old value.
        }

        let previous_isolations: BTreeSet<_> = dirty
            .iter()
            .copied()
            .filter(|index| self.prepared[*index].needs_document_isolation())
            .collect();
        // Preorder refreshes world transforms and inherited outline requests.
        // The ledger includes descendants of edited containers, so no child
        // retains the previous parent's transform, blur scale, or text outline.
        for &index in &dirty {
            let node = &document.nodes[&self.prepared[index].id];
            let parent = self.prepared[index].parent;
            let (parent_world, inherited_outline) = if let Some(parent) = parent {
                (self.prepared[parent].world, self.prepared[parent].outline)
            } else {
                (
                    node.parent
                        .and_then(|id| document.world_transform(id))
                        .unwrap_or(Transform::IDENTITY),
                    false,
                )
            };
            let local = node.local_transform();
            let world = parent_world * local;
            let prepared = &mut self.prepared[index];
            if parent.is_none() {
                match (prepared.mask.is_some(), node.is_mask) {
                    (false, true) => self.root_mask_nodes += 1,
                    (true, false) => self.root_mask_nodes -= 1,
                    _ => {}
                }
            }
            prepared.local = if parent.is_none() { world } else { local };
            prepared.world = world;
            prepared.opacity = unit(node.opacity);
            prepared.blend = node.blend_mode;
            prepared.mask = node.is_mask.then_some(node.mask_type);
            prepared.clip =
                (node.clip_content && node.corner_smoothing == 0.0).then(|| rounded_rect(node));
            prepared.content_clip =
                (node.clip_content && node.corner_smoothing > 0.0).then(|| corner_geometry(node));
            prepared.child_blur_outset = blur_outset_world(&node.effects, world);
            prepared.outline = inherited_outline
                || node.kind == NodeKind::Boolean
                || (node.is_mask && node.mask_type == MaskType::Vector)
                || (node.kind == NodeKind::Group
                    && (!node.effects.is_empty()
                        || !node.strokes.is_empty()
                        || !node.fills.is_empty()));
        }

        self.preparation.incremental = 1;
        self.preparation_totals.incremental = self.preparation_totals.incremental.saturating_add(1);
        // Postorder shares the exact full-prepare path for visual fingerprints,
        // group/Boolean derived geometry, child bounds, and effect outsets.
        for &index in dirty.iter().rev() {
            let previous_isolation = previous_isolations.contains(&index);
            self.refresh_prepared_visual(document, index);
            if self.prepared[index].parent.is_none() {
                if let Ok(slot) = self.prepared_roots.binary_search(&index) {
                    let prepared = &self.prepared[index];
                    self.root_index.update(
                        slot,
                        prepared.world_subtree_bounds,
                        prepared.subtree_nodes,
                    );
                }
            }
            let current_isolation = self.prepared[index].needs_document_isolation();
            match (previous_isolation, current_isolation) {
                (false, true) => self.isolation_nodes += 1,
                (true, false) => self.isolation_nodes -= 1,
                _ => {}
            }
        }
        self.isolates_document = self.isolation_nodes != 0;
        // A changed earlier sibling may affect any later backdrop in the same
        // isolation scope. Keep this bounded cache conservatively invalidated.
        // A suppressed root text leaf alone cannot change that painted prefix.
        if !dirty.is_empty() {
            self.backdrops.clear();
        }
        true
    }
}

#[cfg(all(test, not(target_os = "emscripten")))]
mod tests {
    use super::*;
    use ferrodesign_core::Fill;
    use ferroui_base::media::imaging::{
        BitmapEncoderOptions, PngBitmapEncoderOptions, RenderTargetBitmap,
    };
    use ferroui_base::platform::IPlatformRenderInterface;
    use ferroui_base::{FerroLocator, PixelRect, PixelSize};

    const SIZE: i32 = 128;

    fn backend() -> Rc<dyn ferroui_base::reactive::IDisposable> {
        let scope = FerroLocator::enter_scope();
        FerroLocator::current_mutable()
            .bind::<dyn IPlatformRenderInterface>()
            .to_constant(Rc::new(ferroui_vello::PlatformRenderInterface::default()));
        scope
    }

    pub(super) fn render(
        painter: &mut ScenePainter,
        editor: &Editor,
        pan: f64,
        incremental: bool,
    ) -> (Vec<u8>, RenderStats) {
        render_roots(
            painter,
            editor,
            &editor.document.pages[0].children,
            pan,
            incremental,
        )
    }

    pub(super) fn render_roots(
        painter: &mut ScenePainter,
        editor: &Editor,
        roots: &[NodeId],
        pan: f64,
        incremental: bool,
    ) -> (Vec<u8>, RenderStats) {
        let target = RenderTargetBitmap::new(PixelSize::new(SIZE, SIZE));
        let stats;
        {
            let mut context = target.create_drawing_context();
            let origin = context.push_transform(Matrix::create_translation(-pan, 0.0));
            let viewport = Rect::new(pan, 0.0, SIZE as f64, SIZE as f64);
            painter.set_raster_scale(1.0);
            stats = if incremental {
                painter.render_editor(&mut context, editor, roots, viewport)
            } else {
                painter.render(&mut context, &editor.document, roots, viewport)
            };
            context.pop(origin);
        }
        let mut pixels = vec![0; (SIZE * SIZE * 4) as usize];
        target.copy_pixels(PixelRect::new(0, 0, SIZE, SIZE), &mut pixels, SIZE * 4);
        (pixels, stats)
    }

    pub(super) fn equal_pixels(actual: &[u8], expected: &[u8]) {
        assert_eq!(actual.len(), expected.len());
        let difference = actual.iter().zip(expected).position(|(a, b)| a != b);
        assert!(
            difference.is_none(),
            "first differing byte: {:?}",
            difference.map(|i| (i, actual[i], expected[i]))
        );
    }

    pub(super) fn equals_fresh(
        painter: &mut ScenePainter,
        editor: &Editor,
    ) -> (Vec<u8>, RenderStats) {
        let result = render(painter, editor, 0.0, true);
        let reference = render(&mut ScenePainter::default(), editor, 0.0, false);
        equal_pixels(&result.0, &reference.0);
        result
    }

    pub(super) fn rectangles(count: usize) -> (Editor, NodeId) {
        let mut document = Document::new("Incremental pixels");
        let mut first = 0;
        for i in 0..count {
            let id = document
                .insert_node(
                    1,
                    Node::new(
                        NodeKind::Rectangle,
                        "Rectangle",
                        (i % 100) as f64 * 20.0,
                        (i / 100) as f64 * 20.0,
                        12.0,
                        12.0,
                    )
                    .with_fill(Color::rgb(90, 110, 220)),
                )
                .unwrap();
            if i == 0 {
                first = id;
            }
        }
        (Editor::new(document), first)
    }

    #[test]
    fn incremental_renderer_prepares_one_leaf_in_ten_thousand_and_reuses_pan_frames() {
        let scope = backend();
        let (mut editor, leaf) = rectangles(10_000);
        let mut painter = ScenePainter::default();
        let (original, initial) = render(&mut painter, &editor, 0.0, true);
        assert_eq!((initial.full_prepares, initial.prepared_nodes), (1, 10_000));
        assert_eq!(
            (initial.total_full_prepares, initial.total_prepared_nodes),
            (1, 10_000)
        );
        assert!(
            initial.root_bounds_tests < 500,
            "{} broad-phase bounds tests",
            initial.root_bounds_tests
        );
        assert_eq!(initial.root_candidates, 49);
        assert_eq!(initial.visible_nodes + initial.culled_nodes, 10_000);
        editor.update_node(leaf, "Change one paint", |node| {
            node.fills = vec![Fill::solid(Color::rgb(240, 30, 40))]
        });
        let (changed, stats) = equals_fresh(&mut painter, &editor);
        assert_ne!(&original[..48], &changed[..48]);
        assert_eq!(
            (
                stats.full_prepares,
                stats.incremental_prepares,
                stats.prepared_nodes,
                stats.rebuilt_visuals
            ),
            (0, 1, 1, 1)
        );
        assert_eq!(stats.cached_nodes + 1, stats.visible_nodes);
        assert_eq!(
            (
                stats.total_full_prepares,
                stats.total_incremental_prepares,
                stats.total_prepared_nodes,
                stats.total_rebuilt_visuals
            ),
            (1, 1, 10_001, 10_001)
        );
        // A paint-only edit retains both the spatial bounds and viewport query.
        assert!(stats.root_query_reused);
        assert_eq!(stats.root_bounds_tests, 0);
        let (panned, stats) = render(&mut painter, &editor, 5.0, true);
        let (reference, _) = render(&mut ScenePainter::default(), &editor, 5.0, false);
        equal_pixels(&panned, &reference);
        assert_eq!(
            (
                stats.full_prepares,
                stats.incremental_prepares,
                stats.prepared_nodes,
                stats.rebuilt_visuals
            ),
            (0, 0, 0, 0)
        );
        assert_eq!(
            (
                stats.total_full_prepares,
                stats.total_incremental_prepares,
                stats.total_prepared_nodes,
                stats.total_rebuilt_visuals
            ),
            (1, 1, 10_001, 10_001)
        );
        let bounds = painter.rendered_editor_bounds(&editor, &editor.document.pages[0].children);
        let reference = ScenePainter::default()
            .rendered_bounds(&editor.document, &editor.document.pages[0].children);
        assert_eq!(bounds, reference);
        scope.dispose();
    }

    fn scan_reference(editor: &Editor, pan: f64) -> (Vec<u8>, RenderStats) {
        let mut reference = ScenePainter::default();
        reference.ensure_editor_prepared(editor, &editor.document.pages[0].children);
        // Exercise the original exhaustive root traversal with identical
        // prepared FerroUI geometry, effects, clipping and blend operations.
        reference.root_index.clear();
        render(&mut reference, editor, pan, true)
    }

    #[test]
    fn spatial_root_culling_matches_exhaustive_pixels_through_moves_history_and_stacking() {
        let scope = backend();
        let (mut editor, first) = rectangles(256);
        editor.transact("Build spatial regression", |document| {
            // Scramble stacking independently of spatial order. Visible alpha
            // and Multiply layers overlap so an ordering error changes pixels.
            document.pages[0].children.reverse();
            let ids = document.pages[0].children.clone();
            for (i, id) in ids.into_iter().enumerate() {
                let node = document.nodes.get_mut(&id).unwrap();
                node.x = ((i * 67) % 256) as f64 * 35.0;
                if i % 61 == 0 {
                    node.x = 20.0 + (i % 4) as f64 * 8.0;
                    node.y = 30.0;
                    node.width = 60.0;
                    node.height = 45.0;
                    node.opacity = 0.55;
                    node.blend_mode = BlendMode::Multiply;
                    node.fills = vec![Fill::solid(Color::rgb((i % 255) as u8, 180, 70))];
                }
            }
            // A root outside the viewport must survive via overflowing child
            // bounds. A blurred edge outside it must also remain drawable.
            let mut group = Node::new(NodeKind::Group, "Overflow group", 300.0, 10.0, 20.0, 20.0);
            group.opacity = 0.65;
            group.effects.push(Effect {
                kind: EffectKind::LayerBlur,
                color: Color::BLACK,
                radius: 3.0,
                offset: Point::default(),
                spread: 0.0,
                visible: true,
            });
            let group = document.insert_node(1, group).unwrap();
            document
                .insert_node(
                    group,
                    Node::new(NodeKind::Ellipse, "Overflow", -310.0, 0.0, 80.0, 80.0)
                        .with_fill(Color::rgb(230, 40, 70)),
                )
                .unwrap();
        });
        let mut painter = ScenePainter::default();
        for pan in [0.0, 45.0, 500.0, 8500.0, 0.0] {
            let (pixels, stats) = render(&mut painter, &editor, pan, true);
            let (reference, reference_stats) = scan_reference(&editor, pan);
            equal_pixels(&pixels, &reference);
            assert_eq!(stats.visible_nodes, reference_stats.visible_nodes);
            assert_eq!(stats.culled_nodes, reference_stats.culled_nodes);
        }
        let before = render(&mut painter, &editor, 0.0, true).0;
        editor.update_node(first, "Move far root into view", |node| {
            node.x = 60.0;
            node.y = 20.0;
            node.width = 45.0;
            node.height = 60.0;
        });
        let (moved, stats) = render(&mut painter, &editor, 0.0, true);
        equal_pixels(&moved, &scan_reference(&editor, 0.0).0);
        assert_ne!(before, moved);
        assert_eq!(stats.incremental_prepares, 1);
        assert!(!stats.root_query_reused);
        editor.undo();
        equal_pixels(&render(&mut painter, &editor, 0.0, true).0, &before);
        editor.redo();
        equal_pixels(&render(&mut painter, &editor, 0.0, true).0, &moved);
        editor.transact("Reverse stacking", |document| {
            document.pages[0].children.reverse()
        });
        let (restacked, stats) = render(&mut painter, &editor, 0.0, true);
        equal_pixels(&restacked, &scan_reference(&editor, 0.0).0);
        assert_eq!(stats.full_prepares, 1);
        scope.dispose();
    }

    #[test]
    fn spatial_root_culling_retains_offscreen_backdrop_sampling_sources() {
        let scope = backend();
        let (mut editor, _) = rectangles(64);
        editor.transact("Backdrop halo fixture", |document| {
            for node in document.nodes.values_mut() {
                node.x += 500.0;
            }
            let source = Node::new(
                NodeKind::Rectangle,
                "Offscreen source",
                -8.0,
                10.0,
                6.0,
                100.0,
            )
            .with_fill(Color::rgb(245, 30, 40));
            document.insert_node(1, source).unwrap();
            let mut glass = Node::new(NodeKind::Rectangle, "Visible blur", 0.0, 10.0, 80.0, 100.0)
                .with_fill(Color::rgba(255, 255, 255, 15));
            glass.effects.push(Effect {
                kind: EffectKind::BackgroundBlur,
                color: Color::BLACK,
                radius: 6.0,
                offset: Point::default(),
                spread: 0.0,
                visible: true,
            });
            document.insert_node(1, glass).unwrap();
        });
        let (actual, stats) = render(&mut ScenePainter::default(), &editor, 0.0, true);
        let (reference, reference_stats) = scan_reference(&editor, 0.0);
        equal_pixels(&actual, &reference);
        assert_eq!(stats.root_candidates, 1);
        assert_eq!(stats.visible_nodes, reference_stats.visible_nodes);
        assert_eq!(stats.backdrop_rasterizations, 1);
        let source_pixel = (50 * SIZE as usize + 1) * 4;
        assert!(
            actual[source_pixel + 3] > 15,
            "offscreen source must contribute alpha through the visible blur halo"
        );
        scope.dispose();
    }

    #[test]
    fn incremental_renderer_cumulative_work_survives_cached_frames_bounds_queries_and_clear() {
        let scope = backend();
        let (mut editor, leaf) = rectangles(2);
        let mut painter = ScenePainter::default();
        render(&mut painter, &editor, 0.0, true);
        editor.update_node(leaf, "Move", |node| node.x += 1.0);
        painter.rendered_editor_bounds(&editor, &editor.document.pages[0].children);
        let totals = painter.stats();
        assert_eq!(
            (
                totals.total_full_prepares,
                totals.total_incremental_prepares,
                totals.total_prepared_nodes
            ),
            (1, 1, 3)
        );
        let (_, cached) = render(&mut painter, &editor, 0.0, true);
        assert_eq!(cached.prepared_nodes, 0);
        assert_eq!(cached.total_prepared_nodes, totals.total_prepared_nodes);
        painter.clear();
        assert_eq!(
            painter.stats().total_prepared_nodes,
            totals.total_prepared_nodes
        );
        let (_, rebuilt) = render(&mut painter, &editor, 0.0, true);
        assert_eq!(
            (
                rebuilt.total_full_prepares,
                rebuilt.total_incremental_prepares,
                rebuilt.total_prepared_nodes
            ),
            (2, 1, 5)
        );
        scope.dispose();
    }

    #[test]
    fn incremental_renderer_preview_cancel_and_undo_redo_match_complete_scene_pixels() {
        let scope = backend();
        let (mut editor, leaf) = rectangles(3);
        let mut painter = ScenePainter::default();
        let original = render(&mut painter, &editor, 0.0, true).0;
        editor.begin_node_transaction(&[leaf], "Drag");
        editor.document.nodes.get_mut(&leaf).unwrap().y = 30.0;
        editor.refresh_preview_nodes(&[leaf]);
        let (preview, stats) = equals_fresh(&mut painter, &editor);
        assert_eq!(
            (
                stats.incremental_prepares,
                stats.prepared_nodes,
                stats.rebuilt_visuals
            ),
            (1, 1, 0)
        );
        assert!(preview != original);
        editor.cancel_transaction();
        let (cancelled, stats) = equals_fresh(&mut painter, &editor);
        assert_eq!(stats.prepared_nodes, 1);
        equal_pixels(&cancelled, &original);

        editor.begin_node_transaction(&[leaf], "Drag");
        editor.document.nodes.get_mut(&leaf).unwrap().y = 40.0;
        editor.refresh_preview_nodes(&[leaf]);
        editor.commit_transaction();
        let committed = equals_fresh(&mut painter, &editor).0;
        editor.undo();
        let (undone, stats) = equals_fresh(&mut painter, &editor);
        assert_eq!(stats.prepared_nodes, 1);
        equal_pixels(&undone, &original);
        editor.redo();
        let (redone, stats) = equals_fresh(&mut painter, &editor);
        assert_eq!(stats.prepared_nodes, 1);
        equal_pixels(&redone, &committed);
        scope.dispose();
    }

    #[test]
    fn incremental_renderer_refreshes_ancestor_transforms_rounded_clips_and_child_bounds() {
        let scope = backend();
        let mut document = Document::new("Ancestor pixels");
        let frame = document
            .insert_node(1, Node::new(NodeKind::Frame, "Frame", 8.0, 8.0, 72.0, 70.0))
            .unwrap();
        let child = document
            .insert_node(
                frame,
                Node::new(NodeKind::Rectangle, "Overflow", 45.0, 12.0, 70.0, 70.0)
                    .with_fill(Color::rgb(210, 30, 80)),
            )
            .unwrap();
        document
            .insert_node(
                frame,
                Node::new(NodeKind::Rectangle, "Other", 0.0, 0.0, 22.0, 22.0)
                    .with_fill(Color::BLACK),
            )
            .unwrap();
        let mut editor = Editor::new(document);
        let mut painter = ScenePainter::default();
        render(&mut painter, &editor, 0.0, true);
        editor.update_node(frame, "Transform", |node| {
            node.set_local_transform(
                Transform::translation(15.0, 12.0)
                    * Transform::rotation(12.0)
                    * Transform::scale(0.85, 0.8),
            );
            node.corner_radius = 14.0;
            node.opacity = 0.7;
        });
        let (_, stats) = equals_fresh(&mut painter, &editor);
        assert_eq!((stats.incremental_prepares, stats.prepared_nodes), (1, 3));
        editor.update_node(child, "Move child", |node| {
            node.x = -18.0;
            node.height = 85.0;
        });
        let (_, stats) = equals_fresh(&mut painter, &editor);
        assert_eq!((stats.incremental_prepares, stats.prepared_nodes), (1, 2));
        let roots = &editor.document.pages[0].children;
        assert_eq!(
            painter.rendered_editor_bounds(&editor, roots),
            ScenePainter::default().rendered_bounds(&editor.document, roots)
        );
        editor.update_node(frame, "Remove clip", |node| node.clip_content = false);
        let (_, stats) = equals_fresh(&mut painter, &editor);
        assert_eq!(stats.full_prepares, 1);
        scope.dispose();
    }

    #[test]
    fn incremental_renderer_rebuilds_derived_boolean_and_group_geometry_bottom_up() {
        let scope = backend();
        for kind in [NodeKind::Boolean, NodeKind::Group] {
            let mut document = Document::new("Derived geometry");
            let mut parent = Node::new(kind, "Derived", 10.0, 10.0, 80.0, 80.0)
                .with_fill(Color::rgb(40, 160, 230));
            parent.boolean_operation = BooleanOperation::Subtract;
            parent.strokes = vec![Stroke::new(Color::BLACK, 2.0)];
            let parent = document.insert_node(1, parent).unwrap();
            document
                .insert_node(
                    parent,
                    Node::new(NodeKind::Rectangle, "Outer", 0.0, 0.0, 80.0, 80.0)
                        .with_fill(Color::rgb(40, 160, 230)),
                )
                .unwrap();
            let hole = document
                .insert_node(
                    parent,
                    Node::new(NodeKind::Rectangle, "Moving child", 10.0, 10.0, 30.0, 30.0)
                        .with_fill(Color::BLACK),
                )
                .unwrap();
            let mut editor = Editor::new(document);
            let mut painter = ScenePainter::default();
            let before = render(&mut painter, &editor, 0.0, true).0;
            editor.update_node(hole, "Move operand", |node| node.x += 22.0);
            let (after, stats) = equals_fresh(&mut painter, &editor);
            assert!(before != after);
            assert_eq!((stats.incremental_prepares, stats.prepared_nodes), (1, 2));
            assert!(stats.rebuilt_visuals >= 1);
        }
        scope.dispose();
    }

    #[test]
    fn incremental_renderer_topology_editor_replacement_and_skipped_ledger_use_full_preparation() {
        let scope = backend();
        let (mut editor, leaf) = rectangles(3);
        let mut painter = ScenePainter::default();
        render(&mut painter, &editor, 0.0, true);
        editor.transact("Insert", |document| {
            document
                .insert_node(1, Node::new(NodeKind::Ellipse, "New", 5.0, 5.0, 30.0, 30.0))
                .unwrap();
        });
        assert_eq!(equals_fresh(&mut painter, &editor).1.full_prepares, 1);
        let mut replacement = Editor::new(editor.document.clone());
        assert_eq!(equals_fresh(&mut painter, &replacement).1.full_prepares, 1);
        for _ in 0..130 {
            replacement.update_node(leaf, "Move", |node| node.x += 0.1);
        }
        assert_eq!(equals_fresh(&mut painter, &replacement).1.full_prepares, 1);
        let (export, stats) = render_roots(&mut painter, &replacement, &[leaf], 0.0, true);
        let (reference, _) = render_roots(
            &mut ScenePainter::default(),
            &replacement,
            &[leaf],
            0.0,
            false,
        );
        equal_pixels(&export, &reference);
        assert_eq!(stats.full_prepares, 1);
        scope.dispose();
    }

    fn image_png(color: Color) -> Vec<u8> {
        let bitmap = RenderTargetBitmap::new(PixelSize::new(2, 2));
        {
            let mut context = bitmap.create_drawing_context();
            context.fill_rectangle(
                &solid_brush(color, 1.0),
                UiRect::new(0.0, 0.0, 2.0, 2.0),
                0.0,
            );
        }
        let mut bytes = Vec::new();
        bitmap
            .save(
                &mut bytes,
                &BitmapEncoderOptions::Png(PngBitmapEncoderOptions::default()),
            )
            .unwrap();
        bytes
    }

    #[test]
    fn incremental_renderer_image_registration_and_replacement_invalidate_visuals() {
        let scope = backend();
        let (mut editor, leaf) = rectangles(1);
        editor.update_node(leaf, "Image", |node| {
            node.fills = vec![Fill {
                paint: Paint::Image {
                    href: "asset://registered".into(),
                    scale: ImageScale::Stretch,
                    crop: None,
                },
                opacity: 1.0,
                visible: true,
            }]
        });
        let mut painter = ScenePainter::default();
        assert_eq!(render(&mut painter, &editor, 0.0, true).1.missing_images, 1);
        let mut previous = None;
        for color in [Color::rgb(240, 20, 40), Color::rgb(20, 220, 60)] {
            let bytes = image_png(color);
            painter
                .register_image("asset://registered", &bytes)
                .unwrap();
            let (actual, stats) = render(&mut painter, &editor, 0.0, true);
            let mut reference = ScenePainter::default();
            reference
                .register_image("asset://registered", &bytes)
                .unwrap();
            equal_pixels(&actual, &render(&mut reference, &editor, 0.0, false).0);
            assert_eq!(
                (
                    stats.full_prepares,
                    stats.rebuilt_visuals,
                    stats.missing_images
                ),
                (1, 1, 0)
            );
            if let Some(previous) = previous {
                assert!(previous != actual);
            }
            previous = Some(actual);
        }
        scope.dispose();
    }

    #[test]
    fn incremental_renderer_generic_document_and_explicit_invalidation_keep_full_fallback() {
        let scope = backend();
        let (mut editor, leaf) = rectangles(2);
        let mut painter = ScenePainter::default();
        render(&mut painter, &editor, 0.0, true);
        editor.document.nodes.get_mut(&leaf).unwrap().x = 30.0;
        editor.document.revision += 1;
        let (actual, stats) = render(&mut painter, &editor, 0.0, false);
        equal_pixels(
            &actual,
            &render(&mut ScenePainter::default(), &editor, 0.0, false).0,
        );
        assert_eq!(stats.full_prepares, 1);
        editor.refresh();
        assert_eq!(equals_fresh(&mut painter, &editor).1.full_prepares, 1);
        painter.invalidate();
        assert_eq!(equals_fresh(&mut painter, &editor).1.full_prepares, 1);
        painter.set_editing_text(Some(leaf));
        let (actual, stats) = render(&mut painter, &editor, 0.0, true);
        let mut reference = ScenePainter::default();
        reference.set_editing_text(Some(leaf));
        equal_pixels(&actual, &render(&mut reference, &editor, 0.0, false).0);
        assert_eq!(stats.full_prepares, 1);
        painter.set_editing_text(None);
        assert_eq!(equals_fresh(&mut painter, &editor).1.full_prepares, 1);
        scope.dispose();
    }

    #[test]
    fn incremental_renderer_invalidates_filtered_prefix_cache_without_repreparing_unrelated_nodes()
    {
        let scope = backend();
        let mut document = Document::new("Filtered prefix");
        let left = document
            .insert_node(
                1,
                Node::new(NodeKind::Rectangle, "White", 0.0, 0.0, 64.0, 128.0)
                    .with_fill(Color::WHITE),
            )
            .unwrap();
        document
            .insert_node(
                1,
                Node::new(NodeKind::Rectangle, "Black", 64.0, 0.0, 64.0, 128.0)
                    .with_fill(Color::BLACK),
            )
            .unwrap();
        let mut glass = Node::new(NodeKind::Rectangle, "Glass", 32.0, 32.0, 64.0, 64.0);
        glass.fills.clear();
        glass.effects = vec![Effect {
            kind: EffectKind::BackgroundBlur,
            radius: 6.0,
            color: Color::TRANSPARENT,
            offset: Point::new(0.0, 0.0),
            spread: 0.0,
            visible: true,
        }];
        document.insert_node(1, glass).unwrap();
        let mut editor = Editor::new(document);
        let mut painter = ScenePainter::default();
        assert_eq!(
            render(&mut painter, &editor, 0.0, true)
                .1
                .backdrop_rasterizations,
            1
        );
        editor.update_node(left, "Change prefix", |node| {
            node.fills = vec![Fill::solid(Color::rgb(230, 20, 60))]
        });
        let (_, stats) = equals_fresh(&mut painter, &editor);
        assert_eq!(
            (
                stats.incremental_prepares,
                stats.prepared_nodes,
                stats.backdrop_rasterizations
            ),
            (1, 1, 1)
        );
        let (_, stats) = render(&mut painter, &editor, 4.0, true);
        assert_eq!(
            (
                stats.prepared_nodes,
                stats.backdrop_rasterizations,
                stats.backdrop_cache_hits
            ),
            (0, 0, 1)
        );
        scope.dispose();
    }
}

#[cfg(all(test, not(target_os = "emscripten")))]
#[path = "incremental_text_tests.rs"]
mod incremental_text_tests;
