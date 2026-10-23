//! Backend-neutral document rendering through FerroUI's public media API.
//!
//! A prepared scene survives camera changes. Immutable brushes, pens, geometries,
//! and shaped text layouts survive unrelated document edits. The host applies
//! the camera transform; this module owns only document-space transforms.
//! No graphics-backend types occur in this module.

#[cfg(all(test, not(target_os = "emscripten")))]
mod angular_tests;
mod backdrop;
pub(crate) mod flatten;
pub(crate) mod typography;
pub(crate) mod text_export;
mod incremental;
#[cfg(all(test, not(target_os = "emscripten")))]
mod mask_tests;
mod masks;
mod picking;
mod root_index;
mod regions;
#[cfg(all(test, not(target_os = "emscripten")))]
mod region_tests;
#[cfg(all(test, not(target_os = "emscripten")))]
mod svg_regions_tests;
#[cfg(all(test, not(target_os = "emscripten")))]
mod shadow_tests;
#[cfg(all(test, not(target_os = "emscripten")))]
mod corner_tests;
#[cfg(all(test, not(target_os = "emscripten")))]
mod polygon_corner_tests;

use std::cell::Cell;
use std::collections::{hash_map::DefaultHasher, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::io::{self, Cursor};
use std::rc::Rc;

use ferrodesign_core::{
    BlendMode, BooleanOperation, Color, Document, Editor, EditorRenderChanges, EditorRenderToken,
    Effect, EffectKind, FillRule, ImageScale, MaskType, Node, NodeId, NodeKind, Paint, PathSegment,
    Point, Rect, Stroke, StrokeAlign, StrokeCap, StrokeJoin, TextAlign, TextAutoResize,
    TextDecoration, TextVerticalAlign, Transform,
};
use ferroui_base::media::imaging::{Bitmap, BitmapBlendingMode};
use ferroui_base::media::immutable::{
    ImmutableConicGradientBrush, ImmutableDashStyle, ImmutableGradientStop, ImmutableImageBrush, ImmutableLinearGradientBrush,
    ImmutablePen, ImmutableRadialGradientBrush, ImmutableSolidColorBrush,
};
use ferroui_base::media::text_formatting::{
    GenericTextRunProperties, TextLayout, TextLayoutOptions, TextRunCache,
    TextRunProperties,
};
use ferroui_base::media::{
    AlignmentX, AlignmentY, BaselineAlignment, BaselinePixelAlignment, BoxShadow, BoxShadows, CombinedGeometry,
    DrawingContext, EllipseGeometry, FlowDirection, FontFamily, FontStretch, FontStyle, FontWeight,
    Geometry, GeometryCombineMode, GeometryGroup, GradientSpreadMethod, IBrush, IEffect,
    IImmutableBrush, IPen, ImmutableBlurEffect, MatrixTransform, PenLineCap, PenLineJoin,
    PushedState, RectangleGeometry, RenderOptions, StreamGeometry, Stretch, TextAlignment,
    TextDecorationCollection, TextDecorations, TextHintingMode, TextOptions, TextWrapping, TileMode, Typeface,
};
use ferroui_base::platform::IGeometryContext;
use ferroui_base::utilities::{CultureInfo, ValueSpan};
use ferroui_base::{
    Matrix, Point as UiPoint, Rect as UiRect, Ref, RelativePoint, RelativeUnit, RoundedRect,
};

const MAX_RENDER_DEPTH: usize = 256;
const MAX_IMAGE_BYTES: usize = 48 * 1024 * 1024;

/// Counters for the last render, including limitations that must remain visible
/// to diagnostics instead of silently changing the document.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RenderStats {
    pub visible_nodes: usize,
    pub culled_nodes: usize,
    pub cached_nodes: usize,
    pub missing_images: usize,
    /// Effects declined by the active backend or bounded offscreen fallback.
    pub unsupported_effects: usize,
    pub invalid_nodes: usize,
    pub backdrop_cache_hits: usize,
    pub backdrop_rasterizations: usize,
    pub backdrop_skipped: usize,
    pub backdrop_too_large: usize,
    pub full_prepares: usize,
    pub incremental_prepares: usize,
    pub prepared_nodes: usize,
    pub rebuilt_visuals: usize,
    /// Spatial broad-phase work, separate from preparation and GPU drawing.
    pub root_bounds_tests: usize,
    pub root_candidates: usize,
    pub root_query_reused: bool,
    pub total_full_prepares: u64,
    pub total_incremental_prepares: u64,
    pub total_prepared_nodes: u64,
    pub total_rebuilt_visuals: u64,
}

enum ImageAsset {
    Ready(Rc<Bitmap>),
    Error(String),
}

struct CachedText {
    layout: TextLayout,
    origin: UiPoint,
}

struct CachedStroke {
    pen: Rc<dyn IPen>,
    brush: Rc<dyn IBrush>,
    outline: Option<Ref<Geometry>>,
    clip: Option<Ref<Geometry>>,
    arrow: Option<Ref<Geometry>>,
}

struct CachedShadow {
    geometry: Ref<Geometry>,
    brush: Rc<dyn IBrush>,
    blur: Option<Rc<dyn IEffect>>,
    bounds: UiRect,
    translation: Matrix,
    inset: bool,
}

struct CachedNode {
    fingerprint: u64,
    geometry: Ref<Geometry>,
    stroke_geometry: Ref<Geometry>,
    rounded_rect: Option<RoundedRect>,
    fills: Vec<Rc<dyn IBrush>>,
    network_fills: Option<regions::CachedNetworkFills>,
    strokes: Vec<CachedStroke>,
    texts: Vec<CachedText>,
    box_outer: BoxShadows,
    box_inner: BoxShadows,
    path_shadows: Vec<CachedShadow>,
    blurs: Vec<Rc<dyn IEffect>>,
    background_blurs: Vec<f64>,
    local_bounds: Rect,
    missing_images: usize,
    unsupported_effects: usize,
    missing_image_pen: Option<Rc<dyn IPen>>,
    suppress_children: bool,
    inner_shadows_below_strokes: bool,
    vector_mask: Ref<Geometry>,
}

impl CachedNode {
    fn empty() -> Self {
        Self {
            fingerprint: 0,
            geometry: StreamGeometry::new().upcast(),
            stroke_geometry: StreamGeometry::new().upcast(),
            rounded_rect: None,
            fills: Vec::new(),
            network_fills: None,
            strokes: Vec::new(),
            texts: Vec::new(),
            box_outer: BoxShadows::default(),
            box_inner: BoxShadows::default(),
            path_shadows: Vec::new(),
            blurs: Vec::new(),
            background_blurs: Vec::new(),
            local_bounds: Rect::new(0.0, 0.0, 0.0, 0.0),
            missing_images: 0,
            unsupported_effects: 0,
            missing_image_pen: None,
            suppress_children: false,
            inner_shadows_below_strokes: true,
            vector_mask: StreamGeometry::new().upcast(),
        }
    }
}

struct PreparedNode {
    id: NodeId,
    local: Transform,
    world: Transform,
    parent: Option<usize>,
    opacity: f64,
    blend: BlendMode,
    mask: Option<MaskType>,
    mask_scope: Option<usize>,
    vector_clip: Option<Ref<Geometry>>,
    clip: Option<RoundedRect>,
    content_clip: Option<Ref<Geometry>>,
    child_blur_outset: f64,
    local_subtree_bounds: Rect,
    world_subtree_bounds: Rect,
    end: usize,
    subtree_nodes: usize,
    visual: Rc<CachedNode>,
    reused: Cell<bool>,
    outline: bool,
}

impl PreparedNode {
    fn needs_document_isolation(&self) -> bool {
        !self.visual.background_blurs.is_empty()
            || (self.blend == BlendMode::PassThrough && self.opacity < 1.0)
            || self.mask.is_some_and(|kind| kind != MaskType::Vector)
            || !matches!(self.blend, BlendMode::Normal | BlendMode::PassThrough)
    }

    fn isolates(&self) -> bool {
        self.mask.is_some() || self.blend != BlendMode::PassThrough || !self.visual.blurs.is_empty()
    }
}

/// Main-thread scene renderer, shared by native and WebAssembly hosts.
///
/// Editor operations update Document::revision. Hosts that replace a document
/// or mutate public fields directly should call invalidate(). Assets registered
/// through register_image() automatically invalidate the relevant brushes.
#[derive(Default)]
pub struct ScenePainter {
    cache: HashMap<NodeId, Rc<CachedNode>>,
    images: HashMap<String, ImageAsset>,
    prepared: Vec<PreparedNode>,
    prepared_roots: Vec<usize>,
    root_mask_nodes: usize,
    mask_scopes: Vec<picking::MaskScope>,
    picking_key: Option<picking::PickKey>,
    root_index: root_index::RootPaintIndex,
    visiting: HashSet<NodeId>,
    empty: Option<Rc<CachedNode>>,
    scene_key: Option<(usize, u64, u64)>,
    asset_revision: u64,
    prepare_invalid_nodes: usize,
    stats: RenderStats,
    editing_text: Option<NodeId>,
    backdrops: backdrop::BackdropCache,
    isolates_document: bool,
    isolation_nodes: usize,
    prepared_indices: HashMap<NodeId, usize>,
    incremental_ready: bool,
    editor_token: Option<EditorRenderToken>,
    preparation: incremental::PreparationStats,
    preparation_totals: incremental::PreparationTotals,
}

impl ScenePainter {
    pub fn stats(&self) -> RenderStats {
        RenderStats {
            total_full_prepares: self.preparation_totals.full,
            total_incremental_prepares: self.preparation_totals.incremental,
            total_prepared_nodes: self.preparation_totals.nodes,
            total_rebuilt_visuals: self.preparation_totals.visuals,
            ..self.stats
        }
    }

    pub fn invalidate(&mut self) {
        self.scene_key = None;
        self.editor_token = None;
    }

    /// Physical pixels per document unit before a node's own world transform.
    /// Hosts pass camera zoom times render scaling; raster exports pass their
    /// requested output scale. Panning does not invalidate cached backdrops.
    pub fn set_raster_scale(&mut self, scale: f64) {
        self.backdrops.set_raster_scale(scale);
    }

    /// The active native text editor paints this layer while its caret is open.
    /// This is presentation state; the authored document remains unchanged.
    pub fn set_editing_text(&mut self, node: Option<NodeId>) {
        if self.editing_text != node {
            self.editing_text = node;
            self.invalidate();
        }
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.images.clear();
        self.prepared.clear();
        self.prepared_roots.clear();
        self.root_mask_nodes = 0;
        self.mask_scopes.clear();
        self.root_index.clear();
        self.visiting.clear();
        self.empty = None;
        self.backdrops.clear();
        self.isolates_document = false;
        self.isolation_nodes = 0;
        self.prepared_indices.clear();
        self.incremental_ready = false;
        self.stats = RenderStats::default();
        self.invalidate();
    }

    /// Decode bytes with FerroUI's platform bitmap loader. An application may
    /// resolve a file or URL asynchronously, then register it under its href.
    /// Network access never blocks the render loop.
    pub fn register_image(&mut self, href: impl Into<String>, bytes: &[u8]) -> io::Result<()> {
        if bytes.len() > MAX_IMAGE_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Image exceeds 48 MiB",
            ));
        }
        let bitmap = Bitmap::from_stream(&mut Cursor::new(bytes))?;
        self.images
            .insert(href.into(), ImageAsset::Ready(Rc::new(bitmap)));
        self.asset_revision = self.asset_revision.wrapping_add(1);
        self.invalidate();
        Ok(())
    }

    /// Decode an embedded image with FerroUI, retain the bitmap and return its
    /// logical size for an image layer. No browser canvas rasterizer is involved.
    pub fn register_image_data(&mut self, href: &str) -> io::Result<(f64, f64)> {
        if let Some(ImageAsset::Ready(bitmap)) = self.images.get(href) {
            let size = bitmap.size();
            return Ok((size.width, size.height));
        }
        let bytes = decode_image_data_url(href)?;
        let bitmap = Bitmap::from_stream(&mut Cursor::new(bytes))?;
        let size = bitmap.size();
        self.images
            .insert(href.to_owned(), ImageAsset::Ready(Rc::new(bitmap)));
        self.asset_revision = self.asset_revision.wrapping_add(1);
        self.invalidate();
        Ok((size.width, size.height))
    }

    pub fn image_errors(&self) -> impl Iterator<Item = (&str, &str)> {
        self.images.iter().filter_map(|(href, asset)| match asset {
            ImageAsset::Error(error) => Some((href.as_str(), error.as_str())),
            ImageAsset::Ready(_) => None,
        })
    }

    /// Paint roots into an already camera-transformed drawing context.
    /// viewport is an axis-aligned rectangle in document/world coordinates.
    pub fn render(
        &mut self,
        context: &mut DrawingContext<'_>,
        document: &Document,
        roots: &[NodeId],
        viewport: Rect,
    ) -> RenderStats {
        self.editor_token = None;
        let scene_reused = self.ensure_prepared(document, roots);
        self.paint_prepared(context, viewport, scene_reused)
    }

    /// Render an authored editor scene using its complete change history.
    /// Disposable prototype documents and arbitrary document users keep the
    /// conservative `render` path above. Arbitrary public field writes must be
    /// followed by `Editor::refresh`, as required by the editor's spatial index.
    pub fn render_editor(
        &mut self,
        context: &mut DrawingContext<'_>,
        editor: &Editor,
        roots: &[NodeId],
        viewport: Rect,
    ) -> RenderStats {
        let scene_reused = self.ensure_editor_prepared(editor, roots);
        self.paint_prepared(context, viewport, scene_reused)
    }

    fn paint_prepared(
        &mut self,
        context: &mut DrawingContext<'_>,
        viewport: Rect,
        scene_reused: bool,
    ) -> RenderStats {
        self.stats = RenderStats {
            invalid_nodes: self.prepare_invalid_nodes,
            full_prepares: self.preparation.full,
            incremental_prepares: self.preparation.incremental,
            prepared_nodes: self.preparation.nodes,
            rebuilt_visuals: self.preparation.visuals,
            ..RenderStats::default()
        };
        self.backdrops.begin_frame();
        if viewport.is_finite() && viewport.width >= 0.0 && viewport.height >= 0.0 {
            // The host canvas/page background is presentation, not an authored
            // blending input. Isolate every backdrop-sensitive scene so an
            // opaque vector/alpha mask cannot change that contract. The cached
            // count preserves the direct path for plain Normal-only scenes.
            let document_layer = if self.isolates_document {
                context.try_push_compositing_layer(
                    BitmapBlendingMode::SourceOver,
                    1.0,
                    ui_rect(viewport),
                )
            } else {
                None
            };
            self.backdrops.set_enabled(document_layer.is_some());
            // Root masks retain their ordered scope even when the source is
            // outside the viewport; culling their source cannot reveal content.
            let root_masks = self.root_mask_nodes != 0;
            let roots = if self.root_index.is_active() && !root_masks {
                let query = self.root_index.query(viewport);
                self.stats.root_bounds_tests = query.bounds_tests;
                self.stats.root_query_reused = query.reused;
                self.stats.culled_nodes += query.culled_nodes;
                self.root_index.candidates()
            } else {
                self.stats.root_bounds_tests = self.prepared_roots.len();
                &self.prepared_roots
            };
            self.stats.root_candidates = roots.len();
            if root_masks {
                masks::paint_siblings(
                    context,
                    &self.prepared,
                    0,
                    self.prepared.len(),
                    viewport,
                    scene_reused,
                    &mut self.stats,
                    &mut self.backdrops,
                    &self.prepared_roots,
                );
            } else {
                for &root in roots {
                    paint_subtree(
                        context,
                        &self.prepared,
                        root,
                        viewport,
                        scene_reused,
                        &mut self.stats,
                        &mut self.backdrops,
                        &self.prepared_roots,
                    );
                }
            }
            context.pop(document_layer.unwrap_or(PushedState::NONE));
        }
        self.stats.backdrop_cache_hits = self.backdrops.hits();
        self.stats.backdrop_rasterizations = self.backdrops.rasterizations();
        self.stats = self.stats();
        self.stats
    }

    /// Conservative world-space paint bounds, including strokes, effects and
    /// unclipped child overflow. Use these bounds for raster exports rather
    /// than the document's selection rectangles. Requires initialized FerroUI
    /// rendering/font services and reuses the same cache as render().
    pub fn rendered_bounds(&mut self, document: &Document, roots: &[NodeId]) -> Option<Rect> {
        self.editor_token = None;
        self.ensure_prepared(document, roots);
        self.prepared_bounds()
    }

    pub fn rendered_editor_bounds(&mut self, editor: &Editor, roots: &[NodeId]) -> Option<Rect> {
        self.ensure_editor_prepared(editor, roots);
        self.prepared_bounds()
    }

    fn prepared_bounds(&self) -> Option<Rect> {
        self.prepared_roots
            .iter()
            .map(|&index| self.prepared[index].world_subtree_bounds)
            .reduce(|a, b| a.union(b))
    }

    fn ensure_prepared(&mut self, document: &Document, roots: &[NodeId]) -> bool {
        self.preparation = incremental::PreparationStats::default();
        let key = self.scene_key_for(document, roots);
        let reused = self.scene_key == Some(key);
        if !reused {
            self.prepare(document, roots);
            self.scene_key = Some(key);
        }
        reused
    }

    fn scene_key_for(&self, document: &Document, roots: &[NodeId]) -> (usize, u64, u64) {
        let mut signature = DefaultHasher::new();
        roots.hash(&mut signature);
        self.asset_revision.hash(&mut signature);
        document.nodes.len().hash(&mut signature);
        // Detect a replaced map even if deserialization reset its revision.
        document
            .nodes
            .values()
            .next()
            .map(|node| node as *const Node as usize)
            .hash(&mut signature);
        (
            document as *const Document as usize,
            document.revision,
            signature.finish(),
        )
    }

    fn prepare(&mut self, document: &Document, roots: &[NodeId]) {
        self.preparation = incremental::PreparationStats {
            full: 1,
            ..Default::default()
        };
        self.preparation_totals.full = self.preparation_totals.full.saturating_add(1);
        self.prepared.clear();
        self.prepared_roots.clear();
        self.root_mask_nodes = 0;
        self.visiting.clear();
        self.prepare_invalid_nodes = 0;
        self.backdrops.clear();
        self.isolates_document = false;
        self.isolation_nodes = 0;
        self.prepared_indices.clear();
        self.incremental_ready = true;
        self.cache.retain(|id, _| document.nodes.contains_key(id));
        if self.empty.is_none() {
            self.empty = Some(Rc::new(CachedNode::empty()));
        }
        for &root in roots {
            let parent_world = document
                .nodes
                .get(&root)
                .and_then(|node| node.parent)
                .and_then(|parent| document.world_transform(parent))
                .unwrap_or(Transform::IDENTITY);
            if let Some(index) = self.prepare_node(document, root, parent_world, None, 0, false) {
                self.prepared_roots.push(index);
            }
        }
        picking::prepare_scopes(&mut self.prepared, &mut self.mask_scopes);
        self.root_index
            .rebuild(&self.prepared, &self.prepared_roots);
    }

    fn prepare_node(
        &mut self,
        document: &Document,
        id: NodeId,
        parent_world: Transform,
        parent: Option<usize>,
        depth: usize,
        needs_outline: bool,
    ) -> Option<usize> {
        let Some(node) = document.nodes.get(&id) else {
            self.prepare_invalid_nodes += 1;
            return None;
        };
        if !node.visible || (node.opacity <= 0.0 && !node.is_mask) || self.editing_text == Some(id)
        {
            return None;
        }
        if depth >= MAX_RENDER_DEPTH || !valid_node_bounds(node) || !self.visiting.insert(id) {
            self.prepare_invalid_nodes += 1;
            self.incremental_ready = false;
            return None;
        }
        let local = node.local_transform();
        let world = parent_world * local;
        let child_outline = needs_outline
            || (node.is_mask && node.mask_type == MaskType::Vector)
            || node.kind == NodeKind::Boolean
            || (node.kind == NodeKind::Group
                && (!node.effects.is_empty()
                    || !node.strokes.is_empty()
                    || !node.fills.is_empty()));
        let index = self.prepared.len();
        let placeholder = self
            .cache
            .get(&id)
            .cloned()
            .unwrap_or_else(|| self.empty.as_ref().unwrap().clone());
        self.prepared.push(PreparedNode {
            id,
            // An isolated nested root still paints at its document position.
            // Its ancestors are absent from the drawing-context state stack.
            local: if depth == 0 { world } else { local },
            world,
            parent,
            opacity: unit(node.opacity),
            blend: node.blend_mode,
            mask: node.is_mask.then_some(node.mask_type),
            mask_scope: None,
            vector_clip: None,
            clip: (node.clip_content && node.corner_smoothing == 0.0).then(|| rounded_rect(node)),
            content_clip: (node.clip_content && node.corner_smoothing > 0.0).then(|| corner_geometry(node)),
            child_blur_outset: blur_outset_world(&node.effects, world),
            local_subtree_bounds: node.local_bounds(),
            world_subtree_bounds: world.transform_rect(node.local_bounds()),
            end: index + 1,
            subtree_nodes: 1,
            visual: placeholder,
            reused: Cell::new(false),
            outline: child_outline,
        });
        if parent.is_none() && node.is_mask {
            self.root_mask_nodes += 1;
        }
        if self.prepared_indices.insert(id, index).is_some() {
            self.incremental_ready = false;
        }
        for &child in &node.children {
            self.prepare_node(
                document,
                child,
                world,
                Some(index),
                depth + 1,
                child_outline,
            );
        }
        let end = self.prepared.len();
        self.prepared[index].end = end;
        self.prepared[index].subtree_nodes = end - index;
        self.refresh_prepared_visual(document, index);
        self.isolation_nodes += usize::from(self.prepared[index].needs_document_isolation());
        self.isolates_document = self.isolation_nodes != 0;
        self.visiting.remove(&id);
        Some(index)
    }

    /// Shared by full postorder preparation and a verified dirty dependency
    /// set. The same paint fingerprint and bounds rules govern both paths.
    fn refresh_prepared_visual(&mut self, document: &Document, index: usize) {
        self.preparation.nodes += 1;
        self.preparation_totals.nodes = self.preparation_totals.nodes.saturating_add(1);
        let id = self.prepared[index].id;
        let node = &document.nodes[&id];
        let end = self.prepared[index].end;
        let outline = self.prepared[index].outline;
        let mut child_signature = DefaultHasher::new();
        let mut child_bounds = None;
        let mut cursor = index + 1;
        while cursor < end {
            let child = &self.prepared[cursor];
            child.visual.fingerprint.hash(&mut child_signature);
            hash_transform(child.local, &mut child_signature);
            hash_number(child.opacity, &mut child_signature);
            child.mask.is_some().hash(&mut child_signature);
            if let Some(kind) = child.mask {
                std::mem::discriminant(&kind).hash(&mut child_signature);
            }
            let bounds = child.local.transform_rect(child.local_subtree_bounds);
            child_bounds = union_optional(child_bounds, Some(bounds));
            cursor = child.end;
        }
        let fingerprint =
            visual_fingerprint(node, child_signature.finish(), outline, self.asset_revision);
        let existing = self
            .cache
            .get(&id)
            .filter(|entry| entry.fingerprint == fingerprint)
            .cloned();
        let reused = existing.is_some();
        let visual = existing.unwrap_or_else(|| {
            self.preparation.visuals += 1;
            self.preparation_totals.visuals = self.preparation_totals.visuals.saturating_add(1);
            Rc::new(build_visual(
                node,
                fingerprint,
                &self.prepared[index + 1..end],
                index + 1,
                outline,
                &mut self.images,
            ))
        });
        let subtree = subtree_bounds(
            node,
            visual.local_bounds,
            if visual.suppress_children {
                None
            } else {
                child_bounds
            },
        );
        let prepared = &mut self.prepared[index];
        prepared.local_subtree_bounds = subtree;
        prepared.world_subtree_bounds = prepared.world.transform_rect(subtree);
        prepared.visual = visual.clone();
        prepared.vector_clip = (prepared.mask == Some(MaskType::Vector)).then(|| {
            let geometry = if prepared.opacity <= 0.0 {
                StreamGeometry::new().upcast::<Geometry>()
            } else {
                visual.vector_mask.clone_geometry()
            };
            geometry.set_transform(MatrixTransform::with_matrix(ui_matrix(prepared.local)));
            geometry
        });
        prepared.reused.set(reused);
        self.cache.insert(id, visual);
    }
}

fn paint_subtree(
    context: &mut DrawingContext<'_>,
    scene: &[PreparedNode],
    index: usize,
    viewport: Rect,
    scene_reused: bool,
    stats: &mut RenderStats,
    backdrops: &mut backdrop::BackdropCache,
    roots: &[usize],
) {
    let node = &scene[index];
    if !node.world_subtree_bounds.intersects(viewport) {
        stats.culled_nodes += node.subtree_nodes;
        return;
    }
    stats.visible_nodes += 1;
    let reused = node.reused.replace(true);
    stats.cached_nodes += usize::from(scene_reused || reused);
    stats.missing_images += node.visual.missing_images;
    stats.unsupported_effects += node.visual.unsupported_effects;

    let transform = context.push_transform(ui_matrix(node.local));
    if (!node.visual.background_blurs.is_empty()
        || (node.blend == BlendMode::PassThrough && node.opacity < 1.0))
        && backdrops.try_paint(context, scene, roots, index, scene_reused, stats)
    {
        context.pop(transform);
        return;
    }
    // If the bounded pass-through fallback was declined, still consume group
    // opacity once. The diagnostic reports its missing external backdrop.
    let isolate = node.opacity < 1.0
        || (node.blend != BlendMode::PassThrough
            && (node.blend != BlendMode::Normal || node.end > index + 1));
    let composite = if isolate {
        context.try_push_compositing_layer(
            bitmap_blend(node.blend),
            node.opacity,
            ui_rect(node.local_subtree_bounds),
        )
    } else {
        None
    };
    let isolated = composite.is_some();
    if isolate && !isolated {
        stats.unsupported_effects += 1;
    }
    let opacity = if !isolated && node.opacity < 1.0 {
        context.push_opacity(node.opacity)
    } else {
        PushedState::NONE
    };
    let blend = if !isolated && !matches!(node.blend, BlendMode::Normal | BlendMode::PassThrough) {
        context.push_render_options(RenderOptions {
            bitmap_blending_mode: bitmap_blend(node.blend),
            ..RenderOptions::default()
        })
    } else {
        PushedState::NONE
    };
    paint_blurs(
        context,
        scene,
        index,
        viewport,
        scene_reused,
        stats,
        0,
        backdrops,
        roots,
    );
    context.pop(blend);
    context.pop(opacity);
    context.pop(composite.unwrap_or(PushedState::NONE));
    context.pop(transform);
}

// Recursion balances each effect state without allocating a temporary Vec per frame.
fn paint_blurs(
    context: &mut DrawingContext<'_>,
    scene: &[PreparedNode],
    index: usize,
    viewport: Rect,
    scene_reused: bool,
    stats: &mut RenderStats,
    blur_index: usize,
    backdrops: &mut backdrop::BackdropCache,
    roots: &[usize],
) {
    let node = &scene[index];
    if let Some(effect) = node.visual.blurs.get(blur_index) {
        let state = context.push_effect(effect, ui_rect(node.local_subtree_bounds));
        paint_blurs(
            context,
            scene,
            index,
            viewport,
            scene_reused,
            stats,
            blur_index + 1,
            backdrops,
            roots,
        );
        context.pop(state);
        return;
    }
    paint_shadows(context, &node.visual, false);
    if node.visual.inner_shadows_below_strokes {
        // Figma's layer order is fill -> inner shadow -> stroke; groups place
        // inner shadows above their compound contents instead. Boolean output
        // is one derived leaf shape, even though its source has child nodes.
        // https://help.figma.com/hc/en-us/articles/360041488473-Apply-effects-to-layers
        paint_fills(context, &node.visual);
        paint_shadows(context, &node.visual, true);
        paint_strokes(context, &node.visual);
        return;
    }
    paint_body(context, &node.visual);
    if !node.visual.suppress_children {
        let clip = push_content_clip(context, node);
        // A child outside the visible rectangle can still contribute pixels
        // through an ancestor's layer blur. Keep those source pixels alive.
        let child_viewport = viewport.inflate(node.child_blur_outset);
        masks::paint_siblings(
            context,
            scene,
            index + 1,
            node.end,
            child_viewport,
            scene_reused,
            stats,
            backdrops,
            roots,
        );
        context.pop(clip);
    }
    paint_shadows(context, &node.visual, true);
}

fn paint_body(context: &mut DrawingContext<'_>, visual: &CachedNode) {
    // Container/backdrop-prefix callers retain their established body order.
    paint_fills(context, visual);
    paint_strokes(context, visual);
}

fn paint_fills(context: &mut DrawingContext<'_>, visual: &CachedNode) {
    if let Some(network) = &visual.network_fills {
        network.paint(context, &visual.geometry, &visual.fills);
        return;
    }
    if visual.texts.is_empty() {
        // The first fill is the topmost item in the inspector.
        for fill in visual.fills.iter().rev() {
            context.draw_geometry(Some(fill), None, &visual.geometry);
        }
    } else {
        let text_options = context.push_text_options(typography::design_text_options());
        for text in visual.texts.iter().rev() {
            text.layout.draw(context, text.origin);
        }
        context.pop(text_options);
    }
}

fn paint_strokes(context: &mut DrawingContext<'_>, visual: &CachedNode) {
    for stroke in visual.strokes.iter().rev() {
        let clip = stroke.clip.as_ref().map(|clip| context.push_geometry_clip(clip));
        if let Some(outline) = &stroke.outline {
            context.draw_geometry(Some(&stroke.brush), None, outline);
        } else {
            context.draw_geometry(None, Some(&stroke.pen), &visual.stroke_geometry);
        }
        if let Some(arrow) = &stroke.arrow {
            context.draw_geometry(Some(&stroke.brush), None, arrow);
        }
        if let Some(clip) = clip { context.pop(clip); }
    }
    if let Some(pen) = &visual.missing_image_pen {
        let bounds = visual.geometry.bounds();
        let clip = context.push_geometry_clip(&visual.geometry);
        context.draw_line(
            pen,
            UiPoint::new(bounds.x, bounds.y),
            UiPoint::new(bounds.x + bounds.width, bounds.y + bounds.height),
        );
        context.draw_line(
            pen,
            UiPoint::new(bounds.x + bounds.width, bounds.y),
            UiPoint::new(bounds.x, bounds.y + bounds.height),
        );
        context.pop(clip);
    }
}

fn paint_shadows(context: &mut DrawingContext<'_>, visual: &CachedNode, inset: bool) {
    if let Some(rect) = visual.rounded_rect {
        let shadows = if inset {
            &visual.box_inner
        } else {
            &visual.box_outer
        };
        if shadows.count() > 0 {
            context.draw_rounded_rectangle(None, None, rect, shadows);
        }
    }
    for shadow in visual
        .path_shadows
        .iter()
        .rev()
        .filter(|shadow| shadow.inset == inset)
    {
        let clip = if inset {
            context.push_geometry_clip(&visual.geometry)
        } else {
            PushedState::NONE
        };
        let translation = context.push_transform(shadow.translation);
        let blur = shadow
            .blur
            .as_ref()
            .map(|effect| context.push_effect(effect, shadow.bounds))
            .unwrap_or(PushedState::NONE);
        context.draw_geometry(Some(&shadow.brush), None, &shadow.geometry);
        context.pop(blur);
        context.pop(translation);
        context.pop(clip);
    }
}

fn build_visual(
    node: &Node,
    fingerprint: u64,
    children: &[PreparedNode],
    child_base: usize,
    needs_outline: bool,
    images: &mut HashMap<String, ImageAsset>,
) -> CachedNode {
    let mut missing_images = 0;
    let mut fills = Vec::new();
    for fill in node
        .fills
        .iter()
        .filter(|fill| fill.visible && fill.opacity > 0.0)
    {
        let (brush, missing) = make_brush(&fill.paint, fill.opacity, images);
        fills.push(brush as Rc<dyn IBrush>);
        missing_images += usize::from(missing);
    }
    // A nondestructive Boolean inherits its first operand's fill when no fill
    // has been explicitly assigned to the result.
    if fills.is_empty() && node.kind == NodeKind::Boolean {
        if let Some(first) = children.first() {
            fills.extend(first.visual.fills.iter().cloned());
        }
    }

    let texts: Vec<CachedText> = if node.kind == NodeKind::Text {
        fills
            .iter()
            .map(|brush| make_text_layout(node, brush.clone()))
            .collect()
    } else {
        Vec::new()
    };
    let fallback_text = if node.kind == NodeKind::Text && texts.is_empty() {
        Some(make_text_layout(node, solid_brush(Color::BLACK, 1.0)))
    } else {
        None
    };
    let text_layout = texts.first().or(fallback_text.as_ref());
    let outline_required = needs_outline
        || node
            .strokes
            .iter()
            .any(|stroke| stroke.visible && stroke.width > 0.0)
        || node.effects.iter().any(|effect| {
            effect.visible
                && matches!(
                    effect.kind,
                    EffectKind::DropShadow | EffectKind::InnerShadow | EffectKind::BackgroundBlur
                )
        })
        || missing_images > 0;

    let geometry = match node.kind {
        NodeKind::Text => {
            if outline_required {
                text_layout
                    .map(|text| text_outline(&text.layout, text.origin))
                    .unwrap_or_else(|| StreamGeometry::new().upcast())
            } else {
                StreamGeometry::new().upcast()
            }
        }
        NodeKind::Group if !outline_required && fills.is_empty() => StreamGeometry::new().upcast(),
        NodeKind::Group | NodeKind::Boolean => combine_children(
            children,
            child_base,
            if node.kind == NodeKind::Boolean {
                boolean_mode(node.boolean_operation)
            } else {
                GeometryCombineMode::Union
            },
        )
        .unwrap_or_else(|| StreamGeometry::new().upcast()),
        _ => shape_geometry(node),
    };

    let geometry = if node.clip_content && matches!(node.kind, NodeKind::Group | NodeKind::Boolean)
    {
        combined(
            GeometryCombineMode::Intersect,
            geometry,
            corner_geometry(node),
        )
    } else {
        geometry
    };
    let stroke_geometry = node
        .vector_network
        .as_ref()
        .map(|network| path_geometry(&network.stroke_path()))
        .unwrap_or_else(|| geometry.clone());
    let network_fills = regions::prepare(node, &geometry, images, &mut missing_images);
    let mut local_bounds = node.local_bounds();
    if outline_required || node.kind != NodeKind::Text {
        let geometry_bounds = core_rect(geometry.bounds());
        if geometry_bounds.is_finite() {
            local_bounds = local_bounds.union(geometry_bounds);
        }
    }
    if let Some(text) = text_layout {
        let text_bounds = Rect::new(
            text.origin.x,
            text.origin.y,
            text.layout.width_including_trailing_whitespace(),
            text.layout.height(),
        )
        .inflate(font_size(node.font_size) * 0.4);
        if text_bounds.is_finite() {
            local_bounds = local_bounds.union(text_bounds);
        }
    }

    let mut strokes = Vec::new();
    let mut unsupported_effects = 0;
    let mut vector_mask = if let Some(network) = &network_fills {
        network.vector_mask(node, &geometry)
    } else if node
        .fills
        .iter()
        .any(|fill| fill.visible && fill.opacity > 0.0 && masks::paint_has_alpha(&fill.paint))
        || (node.kind == NodeKind::Boolean && !fills.is_empty())
    {
        geometry.clone()
    } else {
        StreamGeometry::new().upcast()
    };
    for stroke in node
        .strokes
        .iter()
        .filter(|stroke| stroke.visible && stroke.width > 0.0 && stroke.opacity > 0.0)
    {
        let (immutable_brush, missing) = make_brush(&stroke.paint, stroke.opacity, images);
        missing_images += usize::from(missing);
        let brush: Rc<dyn IBrush> = immutable_brush.clone();
        let aligned = stroke.align != StrokeAlign::Center && is_closed(node);
        let pen = make_pen(stroke, immutable_brush, if aligned { 2.0 } else { 1.0 });
        let mut stroke_clip = None;
        let outline = if !stroke.width_profile.is_uniform() {
            match ferrodesign_core::variable_width_outline(node, stroke) {
                Ok(path) => {
                    let band = path_geometry(&path);
                    if aligned {
                        stroke_clip = Some(if stroke.align == StrokeAlign::Inside { geometry.clone() }
                            else { combined(GeometryCombineMode::Exclude,
                                RectangleGeometry::with_rect(band.bounds().inflate(1.0)).upcast(), geometry.clone()) });
                    }
                    Some(band)
                }
                Err(_) => { unsupported_effects += 1; continue; }
            }
        } else if aligned {
            let widened = stroke_geometry.get_widened_geometry(pen.as_ref());
            Some(combined(
                if stroke.align == StrokeAlign::Inside {
                    GeometryCombineMode::Intersect
                } else {
                    GeometryCombineMode::Exclude
                },
                widened,
                geometry.clone(),
            ))
        } else {
            None
        };
        let arrow = if stroke.cap == StrokeCap::Arrow && !is_closed(node) {
            arrow_geometry(node, stroke.width)
        } else {
            None
        };
        let stroke_bounds = outline.as_ref().map(|outline| core_rect(outline.bounds()))
            .unwrap_or_else(|| core_rect(stroke_geometry.get_render_bounds(pen.as_ref())));
        if stroke_bounds.is_finite() {
            local_bounds = local_bounds.union(stroke_bounds);
        }
        if let Some(arrow) = &arrow {
            local_bounds = local_bounds.union(core_rect(arrow.bounds()));
        }
        if needs_outline && masks::paint_has_alpha(&stroke.paint) {
            let stroke_region = outline
                .clone()
                .unwrap_or_else(|| stroke_geometry.get_widened_geometry(pen.as_ref()));
            let stroke_region = if let Some(clip) = &stroke_clip {
                combined(GeometryCombineMode::Intersect, stroke_region, clip.clone())
            } else { stroke_region };
            vector_mask = combined(GeometryCombineMode::Union, vector_mask, stroke_region);
            if let Some(arrow) = &arrow {
                vector_mask = combined(GeometryCombineMode::Union, vector_mask, arrow.clone());
            }
        }
        strokes.push(CachedStroke {
            pen,
            brush,
            outline,
            clip: stroke_clip,
            arrow,
        });
    }

    if node.kind.is_container() && node.kind != NodeKind::Boolean && needs_outline {
        vector_mask = masks::add_vector_children(vector_mask, children, child_base, node);
    }

    let rect = match node.kind {
        NodeKind::Rectangle
        | NodeKind::Frame
        | NodeKind::Component
        | NodeKind::Instance
        | NodeKind::Section if node.corner_smoothing == 0.0 => Some(rounded_rect(node)),
        _ => None,
    };
    let mut outer = Vec::new();
    let mut inner = Vec::new();
    let mut path_shadows = Vec::new();
    let mut blurs: Vec<Rc<dyn IEffect>> = Vec::new();
    let mut background_blurs = Vec::new();
    for effect in node.effects.iter().filter(|effect| effect.visible) {
        match effect.kind {
            EffectKind::LayerBlur => {
                if effect.radius > 0.0 {
                    blurs.push(Rc::new(ImmutableBlurEffect::new(radius(effect.radius))));
                }
            }
            EffectKind::BackgroundBlur => {
                if effect.radius > 0.0 {
                    background_blurs.push(radius(effect.radius));
                }
            }
            EffectKind::DropShadow | EffectKind::InnerShadow => {
                if effect.color.a == 0 {
                    continue;
                }
                let inset = effect.kind == EffectKind::InnerShadow;
                if rect.is_some() {
                    let shadow = BoxShadow {
                        offset_x: finite(effect.offset.x),
                        offset_y: finite(effect.offset.y),
                        blur: radius(effect.radius),
                        spread: finite(effect.spread),
                        color: ui_color(effect.color),
                        is_inset: inset,
                    };
                    if inset {
                        inner.push(shadow);
                    } else {
                        outer.push(shadow);
                    }
                } else {
                    path_shadows.push(make_path_shadow(&geometry, effect));
                }
            }
        }
    }
    // FerroUI draws BoxShadows in supplied order. The model's first effect
    // must land on top, matching paint_shadows' reverse path-shadow traversal.
    // Prepare this order once rather than allocating or reversing per frame.
    outer.reverse();
    inner.reverse();
    let missing_image_pen = (missing_images > 0).then(|| {
        let brush: Rc<dyn IImmutableBrush> = Rc::new(ImmutableSolidColorBrush::new(ui_color(
            Color::rgb(164, 169, 181),
        )));
        Rc::new(ImmutablePen::new(
            Some(brush),
            1.0,
            None,
            PenLineCap::Flat,
            PenLineJoin::Miter,
            10.0,
        )) as Rc<dyn IPen>
    });
    CachedNode {
        fingerprint,
        geometry,
        stroke_geometry,
        rounded_rect: rect,
        fills,
        network_fills,
        strokes,
        texts,
        box_outer: box_shadows(&outer),
        box_inner: box_shadows(&inner),
        path_shadows,
        blurs,
        background_blurs,
        local_bounds,
        missing_images,
        unsupported_effects,
        missing_image_pen,
        suppress_children: node.kind == NodeKind::Boolean,
        inner_shadows_below_strokes: !node.kind.is_container() || node.kind == NodeKind::Boolean,
        vector_mask,
    }
}

fn shape_geometry(node: &Node) -> Ref<Geometry> {
    match node.kind {
        NodeKind::Ellipse => EllipseGeometry::with_rect(ui_rect(node.local_bounds())).upcast(),
        NodeKind::Line => polyline_geometry(
            &[Point::new(0.0, 0.0), Point::new(node.width, node.height)],
            false,
        ),
        NodeKind::Polygon | NodeKind::Star => path_geometry(&ferrodesign_core::polygon_corner_path(node)),
        NodeKind::Vector => {
            if let Some(network) = &node.vector_network {
                path_geometry(&network.fill_path())
            } else {
                path_geometry(&node.path)
            }
        }
        _ => corner_geometry(node),
    }
}

pub(crate) fn path_geometry(path: &ferrodesign_core::VectorPath) -> Ref<Geometry> {
    let geometry = StreamGeometry::new();
    {
        let mut context = geometry.open();
        context.set_fill_rule(match path.fill_rule {
            FillRule::NonZero => ferroui_base::media::FillRule::NonZero,
            FillRule::EvenOdd => ferroui_base::media::FillRule::EvenOdd,
        });
        let mut open = false;
        for segment in &path.segments {
            match segment {
                PathSegment::MoveTo { point } => {
                    if open {
                        context.end_figure(false);
                    }
                    context.begin_figure(ui_point(*point), true);
                    open = true;
                }
                PathSegment::LineTo { point } if open => {
                    context.line_to(ui_point(*point), true);
                }
                PathSegment::CubicTo {
                    control1,
                    control2,
                    point,
                } if open => {
                    context.cubic_bezier_to(
                        ui_point(*control1),
                        ui_point(*control2),
                        ui_point(*point),
                        true,
                    );
                }
                PathSegment::QuadraticTo { control, point } if open => {
                    context.quadratic_bezier_to(ui_point(*control), ui_point(*point), true);
                }
                PathSegment::Close if open => {
                    context.end_figure(true);
                    open = false;
                }
                _ => {}
            }
        }
        if open {
            context.end_figure(false);
        }
    }
    geometry.upcast()
}

fn polyline_geometry(points: &[Point], closed: bool) -> Ref<Geometry> {
    let geometry = StreamGeometry::new();
    if let Some(first) = points.first() {
        let mut context = geometry.open();
        context.begin_figure(ui_point(*first), closed);
        for point in &points[1..] {
            context.line_to(ui_point(*point), true);
        }
        context.end_figure(closed);
    }
    geometry.upcast()
}

pub(crate) fn rounded_rect(node: &Node) -> RoundedRect {
    let w = node.width.max(0.0);
    let h = node.height.max(0.0);
    let mut r = node
        .corner_radii
        .unwrap_or([node.corner_radius; 4])
        .map(|value| finite(value).max(0.0));
    // Preserve asymmetric corner proportions while ensuring adjacent arcs fit.
    let scale = [
        ratio(w, r[0] + r[1]),
        ratio(w, r[3] + r[2]),
        ratio(h, r[0] + r[3]),
        ratio(h, r[1] + r[2]),
    ]
    .into_iter()
    .fold(1.0_f64, f64::min);
    for radius in &mut r {
        *radius *= scale;
    }
    RoundedRect::from_radii(UiRect::new(0.0, 0.0, w, h), r[0], r[1], r[2], r[3])
}

pub(crate) fn corner_geometry(node: &Node) -> Ref<Geometry> {
    if node.corner_smoothing > 0.0 { path_geometry(&ferrodesign_core::corner_path(node)) }
    else { rounded_geometry(rounded_rect(node)) }
}

fn push_content_clip(context: &mut DrawingContext<'_>, node: &PreparedNode) -> PushedState {
    if let Some(geometry) = &node.content_clip { context.push_geometry_clip(geometry) }
    else { node.clip.map(|rect| context.push_rounded_clip(rect)).unwrap_or(PushedState::NONE) }
}

fn rounded_geometry(rect: RoundedRect) -> Ref<Geometry> {
    let geometry = StreamGeometry::new();
    let (x, y) = (rect.rect.x, rect.rect.y);
    let (right, bottom) = (x + rect.rect.width, y + rect.rect.height);
    let tl = rect.radii_top_left;
    let tr = rect.radii_top_right;
    let br = rect.radii_bottom_right;
    let bl = rect.radii_bottom_left;
    const K: f64 = 0.5522847498307936;
    {
        let mut path = geometry.open();
        path.begin_figure(UiPoint::new(x + tl.x, y), true);
        path.line_to(UiPoint::new(right - tr.x, y), true);
        path.cubic_bezier_to(
            UiPoint::new(right - tr.x * (1.0 - K), y),
            UiPoint::new(right, y + tr.y * (1.0 - K)),
            UiPoint::new(right, y + tr.y),
            true,
        );
        path.line_to(UiPoint::new(right, bottom - br.y), true);
        path.cubic_bezier_to(
            UiPoint::new(right, bottom - br.y * (1.0 - K)),
            UiPoint::new(right - br.x * (1.0 - K), bottom),
            UiPoint::new(right - br.x, bottom),
            true,
        );
        path.line_to(UiPoint::new(x + bl.x, bottom), true);
        path.cubic_bezier_to(
            UiPoint::new(x + bl.x * (1.0 - K), bottom),
            UiPoint::new(x, bottom - bl.y * (1.0 - K)),
            UiPoint::new(x, bottom - bl.y),
            true,
        );
        path.line_to(UiPoint::new(x, y + tl.y), true);
        path.cubic_bezier_to(
            UiPoint::new(x, y + tl.y * (1.0 - K)),
            UiPoint::new(x + tl.x * (1.0 - K), y),
            UiPoint::new(x + tl.x, y),
            true,
        );
        path.end_figure(true);
    }
    geometry.upcast()
}

fn combine_children(
    children: &[PreparedNode],
    child_base: usize,
    mode: GeometryCombineMode,
) -> Option<Ref<Geometry>> {
    let mut result = None;
    let mut cursor = 0;
    while cursor < children.len() {
        let child = &children[cursor];
        let geometry = child.visual.geometry.clone_geometry();
        geometry.set_transform(MatrixTransform::with_matrix(ui_matrix(child.local)));
        result = Some(match result {
            Some(previous) => combined(mode, previous, geometry),
            None => geometry,
        });
        cursor = child.end.saturating_sub(child_base);
    }
    result
}

fn combined(mode: GeometryCombineMode, a: Ref<Geometry>, b: Ref<Geometry>) -> Ref<Geometry> {
    CombinedGeometry::with_mode(mode, Some(a), Some(b)).upcast()
}

fn boolean_mode(operation: BooleanOperation) -> GeometryCombineMode {
    match operation {
        BooleanOperation::Union => GeometryCombineMode::Union,
        BooleanOperation::Subtract => GeometryCombineMode::Exclude,
        BooleanOperation::Intersect => GeometryCombineMode::Intersect,
        BooleanOperation::Exclude => GeometryCombineMode::Xor,
    }
}

fn is_closed(node: &Node) -> bool {
    node.has_closed_stroke_contours()
}

fn arrow_geometry(node: &Node, width: f64) -> Option<Ref<Geometry>> {
    let points = if node.kind == NodeKind::Line {
        vec![Point::new(0.0, 0.0), Point::new(node.width, node.height)]
    } else {
        node.path.flattened(0.5).into_iter().last()?
    };
    let tip = *points.last()?;
    let previous = points
        .iter()
        .rev()
        .skip(1)
        .find(|point| point.distance(tip) > 0.001)?;
    let delta = tip - *previous;
    let length = delta.distance(Point::new(0.0, 0.0));
    let unit = Point::new(delta.x / length, delta.y / length);
    let arrow_length = finite(width).max(1.0) * 3.5;
    let wing = arrow_length * 0.48;
    let base = tip - unit * arrow_length;
    Some(polyline_geometry(
        &[
            tip,
            Point::new(base.x - unit.y * wing, base.y + unit.x * wing),
            Point::new(base.x + unit.y * wing, base.y - unit.x * wing),
        ],
        true,
    ))
}

fn make_pen(stroke: &Stroke, brush: Rc<dyn IImmutableBrush>, thickness_scale: f64) -> Rc<dyn IPen> {
    let width = radius(stroke.width).max(0.001) * thickness_scale;
    let dash_values: Vec<f64> = stroke
        .dash
        .iter()
        .copied()
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| value / width)
        .collect();
    let dash = (!dash_values.is_empty())
        .then(|| Rc::new(ImmutableDashStyle::new(Some(&dash_values), 0.0)));
    Rc::new(ImmutablePen::new(
        Some(brush),
        width,
        dash,
        match stroke.cap {
            StrokeCap::Round => PenLineCap::Round,
            StrokeCap::Square => PenLineCap::Square,
            StrokeCap::Butt | StrokeCap::Arrow => PenLineCap::Flat,
        },
        match stroke.join {
            StrokeJoin::Miter => PenLineJoin::Miter,
            StrokeJoin::Round => PenLineJoin::Round,
            StrokeJoin::Bevel => PenLineJoin::Bevel,
        },
        10.0,
    ))
}

fn make_path_shadow(geometry: &Ref<Geometry>, effect: &Effect) -> CachedShadow {
    let inset = effect.kind == EffectKind::InnerShadow;
    let blur_radius = radius(effect.radius);
    let spread = finite(effect.spread);
    let offset = Point::new(finite(effect.offset.x), finite(effect.offset.y));
    let geometry = if inset {
        let hole = spread_geometry(geometry, -spread);
        let bounds = core_rect(geometry.bounds())
            .inflate(blur_radius * 3.0 + spread.abs() + offset.x.abs() + offset.y.abs() + 8.0);
        combined(
            GeometryCombineMode::Exclude,
            RectangleGeometry::with_rect(ui_rect(bounds)).upcast(),
            hole,
        )
    } else {
        spread_geometry(geometry, spread)
    };
    let bounds = core_rect(geometry.bounds()).inflate(blur_radius * 3.0 + 1.0);
    CachedShadow {
        geometry,
        brush: solid_brush(effect.color, 1.0),
        blur: (blur_radius > 0.0)
            .then(|| Rc::new(ImmutableBlurEffect::new(blur_radius)) as Rc<dyn IEffect>),
        bounds: ui_rect(bounds),
        translation: Matrix::create_translation(offset.x, offset.y),
        inset,
    }
}

fn spread_geometry(geometry: &Ref<Geometry>, spread: f64) -> Ref<Geometry> {
    if spread.abs() < 0.001 {
        return geometry.clone();
    }
    let brush: Rc<dyn IImmutableBrush> =
        Rc::new(ImmutableSolidColorBrush::new(ui_color(Color::WHITE)));
    let pen: Rc<dyn IPen> = Rc::new(ImmutablePen::new(
        Some(brush),
        spread.abs() * 2.0,
        None,
        PenLineCap::Round,
        PenLineJoin::Round,
        10.0,
    ));
    let widened = geometry.get_widened_geometry(pen.as_ref());
    combined(
        if spread > 0.0 {
            GeometryCombineMode::Union
        } else {
            GeometryCombineMode::Exclude
        },
        geometry.clone(),
        widened,
    )
}

fn box_shadows(shadows: &[BoxShadow]) -> BoxShadows {
    match shadows.split_first() {
        Some((first, rest)) => BoxShadows::with_rest(*first, rest),
        None => BoxShadows::default(),
    }
}

fn make_text_layout(node: &Node, brush: Rc<dyn IBrush>) -> CachedText {
    let default_typeface = typeface(&node.font_family, node.font_weight, node.italic);
    let overrides = typography::range_styles(node, brush.clone());
    let unbounded = node.text_auto_resize == TextAutoResize::WidthAndHeight;
    let layout = TextLayout::new_with_paragraph_styles(
        &node.text,
        default_typeface,
        TextLayoutOptions {
            font_size: font_size(node.font_size),
            foreground: Some(brush),
            text_alignment: match node.text_align {
                TextAlign::Left => TextAlignment::Left,
                TextAlign::Center => TextAlignment::Center,
                TextAlign::Right => TextAlignment::Right,
                TextAlign::Justify => TextAlignment::Justify,
            },
            text_wrapping: if unbounded {
                TextWrapping::NoWrap
            } else {
                TextWrapping::Wrap
            },
            text_trimming: None,
            text_decorations: decorations(node.text_decoration),
            flow_direction: typography::flow_direction(node),
            max_width: if unbounded {
                f64::INFINITY
            } else {
                node.width.max(0.001)
            },
            max_height: f64::INFINITY,
            line_height: font_size(node.font_size) * finite(node.line_height).clamp(0.01, 100.0),
            letter_spacing: finite(node.letter_spacing),
            font_features: typography::font_features(node),
            text_style_overrides: overrides,
            text_run_cache: Some(Rc::new(TextRunCache::new())),
            ..TextLayoutOptions::default()
        },
        typography::paragraph_styles(node),
    );
    let origin = text_origin(
        node,
        layout.width_including_trailing_whitespace(),
        layout.height(),
        layout.baseline(),
    );
    CachedText { layout, origin }
}

fn text_origin(node: &Node, width: f64, height: f64, baseline: f64) -> UiPoint {
    let extra = node.height - height;
    let mut origin = UiPoint::new(
        0.0,
        match node.text_vertical_align {
            TextVerticalAlign::Top => 0.0,
            TextVerticalAlign::Center => extra * 0.5,
            TextVerticalAlign::Bottom => extra,
        },
    );
    if node.metadata.get("svg_element").map(String::as_str) == Some("text") {
        let anchor = match node.metadata.get("svg_text_anchor").map(String::as_str) {
            Some("middle") => 0.5,
            Some("end") => 1.0,
            _ => 0.0,
        };
        // Import records estimated selection bounds. Preserve the original
        // SVG anchor and baseline using the actual shaped layout metrics.
        origin.x += anchor * (node.width - width);
        if let Some(offset) = node
            .metadata
            .get("svg_text_baseline_offset")
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| value.is_finite())
        {
            origin.y += offset - baseline;
        }
    }
    origin
}

fn typeface(family: &str, weight: u16, italic: bool) -> Typeface {
    Typeface::with_style(
        FontFamily::new(if family.trim().is_empty() {
            "Inter"
        } else {
            family
        }),
        if italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        },
        FontWeight(weight.clamp(1, 1000) as i32),
        FontStretch::Normal,
    )
}

fn decorations(decoration: TextDecoration) -> Option<TextDecorationCollection> {
    match decoration {
        TextDecoration::None => None,
        TextDecoration::Underline => Some(TextDecorations::underline()),
        TextDecoration::Strikethrough => Some(TextDecorations::strikethrough()),
    }
}

/// Obtain contours from the same cached FerroUI layout used by visible text.
fn text_outline(layout: &TextLayout, origin: UiPoint) -> Ref<Geometry> {
    layout.build_geometry(origin)
}

fn byte_to_utf16(text: &str, byte_offset: usize) -> i32 {
    let mut boundary = byte_offset.min(text.len());
    while !text.is_char_boundary(boundary) {
        boundary -= 1;
    }
    text[..boundary]
        .encode_utf16()
        .count()
        .min(i32::MAX as usize) as i32
}

fn make_brush(
    paint: &Paint,
    opacity: f64,
    images: &mut HashMap<String, ImageAsset>,
) -> (Rc<dyn IImmutableBrush>, bool) {
    let opacity = unit(opacity);
    match paint {
        Paint::Solid(color) => (
            Rc::new(ImmutableSolidColorBrush::with_opacity(
                ui_color(*color),
                opacity,
            )),
            false,
        ),
        Paint::LinearGradient { start, end, stops } => {
            let stops = gradient_stops(stops);
            (
                Rc::new(ImmutableLinearGradientBrush::new(
                    &stops,
                    opacity,
                    None,
                    None,
                    GradientSpreadMethod::Pad,
                    Some(relative_point(*start)),
                    Some(relative_point(*end)),
                    None,
                )),
                false,
            )
        }
        Paint::RadialGradient {
            center,
            radius: radial_radius,
            stops,
        } => {
            let stops = gradient_stops(stops);
            (
                Rc::new(ImmutableRadialGradientBrush::with_radius(
                    &stops,
                    opacity,
                    None,
                    None,
                    GradientSpreadMethod::Pad,
                    Some(relative_point(*center)),
                    Some(relative_point(*center)),
                    finite(*radial_radius).max(0.00001),
                )),
                false,
            )
        }
        Paint::AngularGradient { center, angle, stops } => (
            Rc::new(ImmutableConicGradientBrush::new(
                &gradient_stops(stops), opacity, None, None, GradientSpreadMethod::Pad,
                Some(relative_point(*center)), finite(*angle).rem_euclid(360.0), None,
            )),
            false,
        ),
        Paint::Image { href, scale, crop } => {
            if !images.contains_key(href) {
                let decoded = decode_image_data_url(href)
                    .and_then(|bytes| Bitmap::from_stream(&mut Cursor::new(bytes)));
                let asset = match decoded {
                    Ok(bitmap) => ImageAsset::Ready(Rc::new(bitmap)),
                    Err(error) => ImageAsset::Error(error.to_string()),
                };
                images.insert(href.clone(), asset);
            }
            if let Some(ImageAsset::Ready(bitmap)) = images.get(href) {
                let tiled = *scale == ImageScale::Tile;
                let destination = tiled.then(|| {
                    let size = bitmap.size();
                    ferroui_base::RelativeRect::new(
                        0.0,
                        0.0,
                        (size.width * crop.map_or(1.0, |crop| crop.width)).max(0.001),
                        (size.height * crop.map_or(1.0, |crop| crop.height)).max(0.001),
                        RelativeUnit::Absolute,
                    )
                });
                (
                    Rc::new(ImmutableImageBrush::new(
                        Some(bitmap.clone()),
                        AlignmentX::Center,
                        AlignmentY::Center,
                        destination,
                        opacity,
                        None,
                        RelativePoint::CENTER,
                        crop.map(|crop| {
                            ferroui_base::RelativeRect::new(
                                crop.x,
                                crop.y,
                                crop.width,
                                crop.height,
                                RelativeUnit::Relative,
                            )
                        }),
                        match scale {
                            ImageScale::Fill => Stretch::UniformToFill,
                            ImageScale::Fit => Stretch::Uniform,
                            ImageScale::Stretch => Stretch::Fill,
                            ImageScale::Tile => Stretch::None,
                        },
                        if tiled {
                            TileMode::Tile
                        } else {
                            TileMode::None
                        },
                        None,
                    )),
                    false,
                )
            } else {
                (
                    Rc::new(ImmutableSolidColorBrush::with_opacity(
                        ui_color(Color::rgb(232, 234, 239)),
                        opacity,
                    )),
                    true,
                )
            }
        }
    }
}

fn gradient_stops(stops: &[ferrodesign_core::GradientStop]) -> Vec<ImmutableGradientStop> {
    let mut stops: Vec<_> = stops
        .iter()
        .map(|stop| ImmutableGradientStop::new(unit(stop.offset), ui_color(stop.color)))
        .collect();
    stops.sort_by(|a, b| a.offset().total_cmp(&b.offset()));
    if stops.is_empty() {
        stops.push(ImmutableGradientStop::new(
            0.0,
            ui_color(Color::TRANSPARENT),
        ));
        stops.push(ImmutableGradientStop::new(
            1.0,
            ui_color(Color::TRANSPARENT),
        ));
    } else if stops.len() == 1 {
        stops.push(ImmutableGradientStop::new(1.0, stops[0].color()));
    }
    stops
}

fn solid_brush(color: Color, opacity: f64) -> Rc<dyn IBrush> {
    Rc::new(ImmutableSolidColorBrush::with_opacity(
        ui_color(color),
        unit(opacity),
    ))
}

fn decode_image_data_url(href: &str) -> io::Result<Vec<u8>> {
    if !href.starts_with("data:image/") {
        return Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            "Image href has not been registered; resolve it asynchronously and call register_image",
        ));
    }
    if href.len() > MAX_IMAGE_BYTES * 4 / 3 + 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Image exceeds 48 MiB",
        ));
    }
    let (header, payload) = href
        .split_once(',')
        .ok_or_else(|| invalid_image("Data URL has no payload separator"))?;
    let bytes = percent_decode(payload.as_bytes())?;
    let decoded = if header
        .split(';')
        .any(|part| part.eq_ignore_ascii_case("base64"))
    {
        decode_base64(&bytes)?
    } else {
        bytes
    };
    if decoded.len() > MAX_IMAGE_BYTES {
        return Err(invalid_image("Image exceeds 48 MiB"));
    }
    Ok(decoded)
}

fn percent_decode(bytes: &[u8]) -> io::Result<Vec<u8>> {
    let mut result = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = bytes.get(index + 1).and_then(|value| hex_digit(*value));
            let low = bytes.get(index + 2).and_then(|value| hex_digit(*value));
            let (Some(high), Some(low)) = (high, low) else {
                return Err(invalid_image("Malformed percent escape in image"));
            };
            result.push(high * 16 + low);
            index += 3;
        } else {
            result.push(bytes[index]);
            index += 1;
        }
    }
    Ok(result)
}

fn decode_base64(bytes: &[u8]) -> io::Result<Vec<u8>> {
    let mut output = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut accumulator = 0_u32;
    let mut bits = 0_u8;
    let mut symbols = 0_usize;
    let mut padding = 0_usize;
    for &byte in bytes {
        if byte.is_ascii_whitespace() {
            continue;
        }
        if byte == b'=' {
            padding += 1;
            if padding > 2 {
                return Err(invalid_image("Too much base64 padding"));
            }
            continue;
        }
        if padding != 0 {
            return Err(invalid_image("Base64 data after padding"));
        }
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return Err(invalid_image("Invalid base64 image byte")),
        };
        symbols += 1;
        accumulator = (accumulator << 6) | value as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push((accumulator >> bits) as u8);
            accumulator &= (1_u32 << bits) - 1;
        }
    }
    if symbols % 4 == 1 || accumulator != 0 {
        return Err(invalid_image("Incomplete base64 image"));
    }
    if padding > 0 && (symbols + padding) % 4 != 0 {
        return Err(invalid_image("Invalid base64 padding"));
    }
    Ok(output)
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn invalid_image(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn subtree_bounds(node: &Node, own: Rect, children: Option<Rect>) -> Rect {
    let children = if node.clip_content {
        children.and_then(|bounds| bounds.intersection(node.local_bounds()))
    } else {
        children
    };
    let mut bounds = children.map(|child| own.union(child)).unwrap_or(own);
    // Expand the complete subtree, not just the parent's rectangle: a group
    // may have children and shadows entirely outside that rectangle.
    for effect in node.effects.iter().filter(|effect| effect.visible) {
        match effect.kind {
            EffectKind::LayerBlur => {
                bounds = bounds.inflate(radius(effect.radius) * 3.0);
            }
            EffectKind::DropShadow => {
                let spread = radius(effect.radius) * 3.0 + finite(effect.spread).abs();
                let shadow = Rect::new(
                    bounds.x + finite(effect.offset.x),
                    bounds.y + finite(effect.offset.y),
                    bounds.width,
                    bounds.height,
                )
                .inflate(spread);
                bounds = bounds.union(shadow);
            }
            EffectKind::InnerShadow | EffectKind::BackgroundBlur => {}
        }
    }
    bounds
}

fn blur_outset_world(effects: &[Effect], world: Transform) -> f64 {
    let radius: f64 = effects
        .iter()
        .filter(|effect| effect.visible && effect.kind == EffectKind::LayerBlur)
        .map(|effect| radius(effect.radius) * 3.0)
        .sum();
    let scale = (world.a.abs() + world.c.abs()).max(world.b.abs() + world.d.abs());
    radius * scale
}

fn union_optional(a: Option<Rect>, b: Option<Rect>) -> Option<Rect> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.union(b)),
        (a, b) => a.or(b),
    }
}

fn valid_node_bounds(node: &Node) -> bool {
    [
        node.x,
        node.y,
        node.width,
        node.height,
        node.rotation,
        node.opacity,
    ]
    .iter()
    .all(|value| value.is_finite())
        && node.width >= 0.0
        && node.height >= 0.0
}

fn finite(value: f64) -> f64 {
    if value.is_finite() {
        value
    } else {
        0.0
    }
}

fn unit(value: f64) -> f64 {
    finite(value).clamp(0.0, 1.0)
}

fn radius(value: f64) -> f64 {
    finite(value).clamp(0.0, 1_000_000.0)
}

fn font_size(value: f64) -> f64 {
    finite(value).clamp(0.1, 100_000.0)
}

fn ratio(length: f64, radii: f64) -> f64 {
    if radii > 0.0 {
        (length / radii).min(1.0)
    } else {
        1.0
    }
}

fn ui_point(point: Point) -> UiPoint {
    UiPoint::new(point.x, point.y)
}

fn relative_point(point: Point) -> RelativePoint {
    RelativePoint::new(finite(point.x), finite(point.y), RelativeUnit::Relative)
}

fn ui_rect(rect: Rect) -> UiRect {
    UiRect::new(rect.x, rect.y, rect.width, rect.height)
}

fn core_rect(rect: UiRect) -> Rect {
    Rect::new(rect.x, rect.y, rect.width, rect.height)
}

fn ui_matrix(transform: Transform) -> Matrix {
    Matrix::new(
        transform.a,
        transform.b,
        transform.c,
        transform.d,
        transform.e,
        transform.f,
    )
}

fn ui_color(color: Color) -> ferroui_base::media::Color {
    ferroui_base::media::Color::from_argb(color.a, color.r, color.g, color.b)
}

fn bitmap_blend(mode: BlendMode) -> BitmapBlendingMode {
    match mode {
        BlendMode::Normal | BlendMode::PassThrough => BitmapBlendingMode::SourceOver,
        BlendMode::Multiply => BitmapBlendingMode::Multiply,
        BlendMode::Screen => BitmapBlendingMode::Screen,
        BlendMode::Overlay => BitmapBlendingMode::Overlay,
        BlendMode::Darken => BitmapBlendingMode::Darken,
        BlendMode::Lighten => BitmapBlendingMode::Lighten,
        BlendMode::ColorDodge => BitmapBlendingMode::ColorDodge,
        BlendMode::ColorBurn => BitmapBlendingMode::ColorBurn,
        BlendMode::HardLight => BitmapBlendingMode::HardLight,
        BlendMode::SoftLight => BitmapBlendingMode::SoftLight,
        BlendMode::Difference => BitmapBlendingMode::Difference,
        BlendMode::Exclusion => BitmapBlendingMode::Exclusion,
        BlendMode::Hue => BitmapBlendingMode::Hue,
        BlendMode::Saturation => BitmapBlendingMode::Saturation,
        BlendMode::Color => BitmapBlendingMode::Color,
        BlendMode::Luminosity => BitmapBlendingMode::Luminosity,
    }
}

fn visual_fingerprint(node: &Node, children: u64, needs_outline: bool, assets: u64) -> u64 {
    let mut hash = DefaultHasher::new();
    std::mem::discriminant(&node.kind).hash(&mut hash);
    needs_outline.hash(&mut hash);
    assets.hash(&mut hash);
    for number in [
        node.width,
        node.height,
        node.corner_radius,
        node.corner_smoothing,
        node.font_size,
        node.line_height,
        node.letter_spacing,
        node.star_inner_radius,
    ] {
        hash_number(number, &mut hash);
    }
    node.corner_radii.is_some().hash(&mut hash);
    if let Some(radii) = node.corner_radii {
        for radius in radii {
            hash_number(radius, &mut hash);
        }
    }
    node.clip_content.hash(&mut hash);
    node.polygon_sides.hash(&mut hash);
    node.font_family.hash(&mut hash);
    node.font_weight.hash(&mut hash);
    node.text_direction.hash(&mut hash);
    node.font_features.hash(&mut hash);
    node.text.hash(&mut hash);
    node.metadata.get("svg_element").hash(&mut hash);
    node.metadata.get("svg_text_anchor").hash(&mut hash);
    node.metadata
        .get("svg_text_baseline_offset")
        .hash(&mut hash);
    node.italic.hash(&mut hash);
    std::mem::discriminant(&node.text_align).hash(&mut hash);
    std::mem::discriminant(&node.text_vertical_align).hash(&mut hash);
    std::mem::discriminant(&node.text_decoration).hash(&mut hash);
    std::mem::discriminant(&node.text_auto_resize).hash(&mut hash);
    for run in &node.text_runs {
        run.start.hash(&mut hash);
        run.end.hash(&mut hash);
        run.font_family.hash(&mut hash);
        run.font_size.map(f64::to_bits).hash(&mut hash);
        run.font_weight.hash(&mut hash);
        run.italic.hash(&mut hash);
        run.color
            .map(|color| [color.r, color.g, color.b, color.a])
            .hash(&mut hash);
        run.decoration
            .map(|decoration| std::mem::discriminant(&decoration))
            .hash(&mut hash);
    }
    node.text_paragraphs.len().hash(&mut hash);
    for paragraph in &node.text_paragraphs {
        paragraph.start.hash(&mut hash);
        paragraph.end.hash(&mut hash);
        paragraph.alignment.map(|value| std::mem::discriminant(&value)).hash(&mut hash);
        paragraph.direction.hash(&mut hash);
        for value in [paragraph.line_height, paragraph.letter_spacing, paragraph.space_before,
            paragraph.space_after, paragraph.indent] {
            value.map(f64::to_bits).hash(&mut hash);
        }
    }
    for fill in &node.fills {
        fill.visible.hash(&mut hash);
        hash_number(fill.opacity, &mut hash);
        hash_paint(&fill.paint, &mut hash);
    }
    for stroke in &node.strokes {
        stroke.visible.hash(&mut hash);
        hash_number(stroke.width, &mut hash);
        hash_number(stroke.opacity, &mut hash);
        std::mem::discriminant(&stroke.align).hash(&mut hash);
        std::mem::discriminant(&stroke.cap).hash(&mut hash);
        std::mem::discriminant(&stroke.join).hash(&mut hash);
        for dash in &stroke.dash {
            hash_number(*dash, &mut hash);
        }
        std::mem::discriminant(&stroke.width_profile).hash(&mut hash);
        if let ferrodesign_core::StrokeWidthProfile::Custom { points } = &stroke.width_profile {
            points.len().hash(&mut hash);
            for point in points {
                hash_number(point.position, &mut hash);
                hash_number(point.width, &mut hash);
            }
        }
        hash_paint(&stroke.paint, &mut hash);
    }
    for effect in &node.effects {
        effect.visible.hash(&mut hash);
        std::mem::discriminant(&effect.kind).hash(&mut hash);
        hash_color(effect.color, &mut hash);
        hash_point(effect.offset, &mut hash);
        hash_number(effect.radius, &mut hash);
        hash_number(effect.spread, &mut hash);
    }
    std::mem::discriminant(&node.path.fill_rule).hash(&mut hash);
    for segment in &node.path.segments {
        std::mem::discriminant(segment).hash(&mut hash);
        match segment {
            PathSegment::MoveTo { point } | PathSegment::LineTo { point } => {
                hash_point(*point, &mut hash);
            }
            PathSegment::CubicTo {
                control1,
                control2,
                point,
            } => {
                hash_point(*control1, &mut hash);
                hash_point(*control2, &mut hash);
                hash_point(*point, &mut hash);
            }
            PathSegment::QuadraticTo { control, point } => {
                hash_point(*control, &mut hash);
                hash_point(*point, &mut hash);
            }
            PathSegment::Close => {}
        }
    }
    node.vector_network.is_some().hash(&mut hash);
    if let Some(network) = &node.vector_network {
        std::mem::discriminant(&network.fill_rule).hash(&mut hash);
        for (id, vertex) in &network.vertices {
            id.hash(&mut hash);
            hash_point(vertex.position, &mut hash);
        }
        for (id, edge) in &network.edges {
            id.hash(&mut hash);
            edge.start.hash(&mut hash);
            edge.end.hash(&mut hash);
            edge.stroke.hash(&mut hash);
            std::mem::discriminant(&edge.curve).hash(&mut hash);
            if let ferrodesign_core::NetworkCurve::Cubic { control1, control2 } = edge.curve {
                hash_point(control1, &mut hash);
                hash_point(control2, &mut hash);
            }
        }
        for (id, region) in &network.regions {
            id.hash(&mut hash);
            region.fills.is_some().hash(&mut hash);
            if let Some(fills) = &region.fills {
                fills.len().hash(&mut hash);
                for fill in fills {
                    fill.visible.hash(&mut hash);
                    hash_number(fill.opacity, &mut hash);
                    hash_paint(&fill.paint, &mut hash);
                }
            }
            for contour in &region.contours {
                contour.len().hash(&mut hash);
                for edge in contour {
                    edge.edge.hash(&mut hash);
                    edge.reversed.hash(&mut hash);
                }
            }
        }
    }
    if node.kind == NodeKind::Boolean
        || (node.kind == NodeKind::Group
            && (needs_outline
                || !node.fills.is_empty()
                || !node.strokes.is_empty()
                || !node.effects.is_empty()))
    {
        children.hash(&mut hash);
        std::mem::discriminant(&node.boolean_operation).hash(&mut hash);
    }
    hash.finish()
}

fn hash_number(number: f64, hash: &mut DefaultHasher) {
    number.to_bits().hash(hash);
}

fn hash_color(color: Color, hash: &mut DefaultHasher) {
    [color.r, color.g, color.b, color.a].hash(hash);
}

fn hash_point(point: Point, hash: &mut DefaultHasher) {
    hash_number(point.x, hash);
    hash_number(point.y, hash);
}

fn hash_transform(transform: Transform, hash: &mut DefaultHasher) {
    for number in [
        transform.a,
        transform.b,
        transform.c,
        transform.d,
        transform.e,
        transform.f,
    ] {
        hash_number(number, hash);
    }
}

fn hash_paint(paint: &Paint, hash: &mut DefaultHasher) {
    std::mem::discriminant(paint).hash(hash);
    match paint {
        Paint::Solid(color) => hash_color(*color, hash),
        Paint::LinearGradient { start, end, stops } => {
            hash_point(*start, hash);
            hash_point(*end, hash);
            for stop in stops {
                hash_number(stop.offset, hash);
                hash_color(stop.color, hash);
            }
        }
        Paint::RadialGradient {
            center,
            radius,
            stops,
        } => {
            hash_point(*center, hash);
            hash_number(*radius, hash);
            for stop in stops {
                hash_number(stop.offset, hash);
                hash_color(stop.color, hash);
            }
        }
        Paint::AngularGradient { center, angle, stops } => {
            hash_point(*center, hash);
            hash_number(*angle, hash);
            for stop in stops {
                hash_number(stop.offset, hash);
                hash_color(stop.color, hash);
            }
        }
        Paint::Image { href, scale, crop } => {
            href.hash(hash);
            std::mem::discriminant(scale).hash(hash);
            crop.is_some().hash(hash);
            if let Some(crop) = crop {
                for value in [crop.x, crop.y, crop.width, crop.height] {
                    hash_number(value, hash);
                }
            }
        }
    }
}

#[cfg(all(test, not(target_os = "emscripten")))]
mod stroke_profile_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use ferrodesign_core::Fill;

    #[test]
    fn ferro_ui_transform_preserves_core_rotation_reflection_and_parent_order() {
        let parent = Transform::translation(317.0, -28.0) * Transform::rotation(19.0);
        let local = Transform::translation(12.0, 8.0)
            * Transform::rotation(-35.0)
            * Transform::scale(-1.0, 0.75);
        let point = Point::new(23.0, 57.0);
        let expected = (parent * local).apply(point);
        let actual = (ui_matrix(local) * ui_matrix(parent)).transform(ui_point(point));
        assert!((actual.x - expected.x).abs() < 1e-9);
        assert!((actual.y - expected.y).abs() < 1e-9);
    }

    #[test]
    fn moving_a_node_reuses_visuals_but_fill_and_boolean_operands_invalidate_them() {
        let mut node = Node::default();
        let before = visual_fingerprint(&node, 10, false, 0);
        node.x += 800.0;
        node.rotation = 36.0;
        node.opacity = 0.25;
        node.flip_x = true;
        assert_eq!(before, visual_fingerprint(&node, 99, false, 0));
        node.fills = vec![Fill {
            paint: Paint::Solid(Color::rgb(255, 42, 16)),
            opacity: 1.0,
            visible: true,
        }];
        assert_ne!(before, visual_fingerprint(&node, 10, false, 0));
        node.kind = NodeKind::Boolean;
        assert_ne!(
            visual_fingerprint(&node, 10, true, 0),
            visual_fingerprint(&node, 11, true, 0),
        );
        assert_ne!(
            visual_fingerprint(&node, 10, true, 0),
            visual_fingerprint(&node, 10, true, 1),
        );
    }

    #[test]
    fn overflowing_children_are_visible_and_frame_clips_limit_their_bounds() {
        let mut node = Node::default();
        node.kind = NodeKind::Group;
        node.width = 20.0;
        node.height = 20.0;
        let child = Rect::new(500.0, 200.0, 40.0, 40.0);
        let viewport = Rect::new(510.0, 210.0, 5.0, 5.0);
        let bounds = subtree_bounds(&node, node.local_bounds(), Some(child));
        assert!(bounds.intersects(viewport));
        node.kind = NodeKind::Frame;
        node.clip_content = true;
        let clipped = subtree_bounds(&node, node.local_bounds(), Some(child));
        assert!(!clipped.intersects(viewport));
        assert_eq!(clipped, node.local_bounds());
    }

    #[test]
    fn parent_blur_expands_overflowing_child_bounds() {
        let mut node = Node::default();
        node.kind = NodeKind::Group;
        node.effects.push(Effect {
            kind: EffectKind::LayerBlur,
            color: Color::BLACK,
            offset: Point::new(0.0, 0.0),
            radius: 10.0,
            spread: 0.0,
            visible: true,
        });
        let child = Rect::new(400.0, 200.0, 40.0, 40.0);
        let bounds = subtree_bounds(&node, node.local_bounds(), Some(child));
        assert!(bounds.intersects(Rect::new(459.0, 210.0, 2.0, 2.0)));
        let viewport = Rect::new(459.0, 210.0, 2.0, 2.0);
        assert!(!child.intersects(viewport));
        assert!(child
            .intersects(viewport.inflate(blur_outset_world(&node.effects, Transform::IDENTITY,))));
    }

    #[test]
    fn imported_svg_text_preserves_anchor_and_baseline_with_real_font_metrics() {
        let mut node = Node::default();
        node.kind = NodeKind::Text;
        node.x = 150.0;
        node.y = 40.0;
        node.width = 100.0;
        node.metadata.insert("svg_element".into(), "text".into());
        node.metadata
            .insert("svg_text_anchor".into(), "middle".into());
        node.metadata
            .insert("svg_text_baseline_offset".into(), "16".into());
        let old_fingerprint = visual_fingerprint(&node, 0, false, 0);
        let origin = text_origin(&node, 80.0, 24.0, 19.0);
        assert_eq!(node.x + origin.x + 40.0, 200.0);
        assert_eq!(node.y + origin.y + 19.0, 56.0);
        node.metadata.insert("svg_text_anchor".into(), "end".into());
        assert_ne!(old_fingerprint, visual_fingerprint(&node, 0, false, 0));
        let origin = text_origin(&node, 80.0, 24.0, 19.0);
        assert_eq!(node.x + origin.x + 80.0, 250.0);
    }

    #[test]
    fn utf8_run_offsets_convert_to_utf16_without_splitting_codepoints() {
        let text = "A🦀ż文";
        assert_eq!(byte_to_utf16(text, 0), 0);
        assert_eq!(byte_to_utf16(text, 1), 1);
        assert_eq!(byte_to_utf16(text, 3), 1);
        assert_eq!(byte_to_utf16(text, 5), 3);
        assert_eq!(byte_to_utf16(text, 7), 4);
        assert_eq!(byte_to_utf16(text, usize::MAX), 5);
    }

    #[test]
    fn image_data_decoder_preserves_bytes_and_rejects_invalid_payloads() {
        assert_eq!(
            decode_image_data_url("data:image/png;base64,AAECA/8=").unwrap(),
            vec![0, 1, 2, 3, 255]
        );
        assert_eq!(
            decode_image_data_url("data:image/png,%00%01%FF+").unwrap(),
            vec![0, 1, 255, b'+']
        );
        assert!(decode_image_data_url("data:image/png;base64,A").is_err());
        assert!(decode_image_data_url("data:image/png;base64,AB==").is_err());
        assert!(decode_image_data_url("data:image/png;base64,AA=A").is_err());
        assert!(decode_image_data_url("data:image/png,%0X").is_err());
        assert_eq!(
            decode_image_data_url("https://example.com/image.png")
                .unwrap_err()
                .kind(),
            io::ErrorKind::WouldBlock
        );
    }
}
