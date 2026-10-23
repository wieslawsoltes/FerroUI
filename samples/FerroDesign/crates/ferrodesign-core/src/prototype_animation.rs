//! Disposable presentation scenes. Geometry and paint interpolation are pure;
//! the native host supplies clock samples and paints these normal document
//! nodes through the same FerroUI scene painter used by the editor.

use crate::*;
use std::collections::{BTreeMap, VecDeque};

/// A compact copy of the visible presentation, including modal scrims. Only
/// active frame/overlay subtrees are copied, never unrelated document pages.
#[derive(Clone, Debug)]
pub struct PrototypeFrame {
    pub document: Document,
    pub root: NodeId,
    /// Translation between the normalized scene and the authored world.
    pub origin: Point,
    pub presentation: Option<NodeId>,
    pub overlays: Vec<NodeId>,
}

impl PrototypeFrame {
    pub fn capture(
        document: &Document,
        current: NodeId,
        overlays: &[(NodeId, Point)],
    ) -> Option<Self> {
        let bounds = document.world_bounds(current)?;
        if !bounds.is_finite() || bounds.width <= 0.0 || bounds.height <= 0.0 {
            return None;
        }
        let origin = Point::new(bounds.x, bounds.y);
        let mut scene = Document::new("Prototype presentation");
        let page = scene.pages[0].id;
        let mut stage = Node::new(
            NodeKind::Frame,
            "__prototype_viewport",
            0.0,
            0.0,
            bounds.width,
            bounds.height,
        );
        stage.fills.clear();
        let root = insert_scene_node(&mut scene, page, stage);
        let frame = copy_tree(document, current, &mut scene, root, 0)?;
        let transform = Transform::translation(-origin.x, -origin.y)
            * document.world_transform(current)?;
        let node = scene.nodes.get_mut(&frame)?;
        node.name = "__prototype_frame".into();
        node.set_local_transform(transform);
        for (index, &(overlay, position)) in overlays.iter().enumerate() {
            let Some(overlay_bounds) = document.world_bounds(overlay) else {
                continue;
            };
            let mut scrim = Node::new(
                NodeKind::Rectangle,
                format!("__prototype_scrim_{index}"),
                0.0,
                0.0,
                bounds.width,
                bounds.height,
            );
            scrim.fills = vec![Fill::solid(Color::rgba(0, 0, 0, 102))];
            insert_scene_node(&mut scene, root, scrim);
            let copied = copy_tree(document, overlay, &mut scene, root, 0)?;
            let transform = Transform::translation(
                position.x - overlay_bounds.x,
                position.y - overlay_bounds.y,
            ) * document.world_transform(overlay)?;
            let node = scene.nodes.get_mut(&copied)?;
            // Reopening the same overlay matches; different overlays dissolve.
            node.name = format!("__prototype_overlay_{overlay}");
            node.set_local_transform(transform);
        }
        scene.revision = 1;
        Some(Self {
            document: scene,
            root,
            origin,
            presentation: Some(current),
            overlays: overlays.iter().map(|entry| entry.0).collect(),
        })
    }

    pub fn bounds(&self) -> Rect {
        self.document.nodes[&self.root].local_bounds()
    }
}

#[derive(Clone, Debug)]
enum PrototypeTrack {
    Node { id: NodeId, from: Box<Node>, to: Box<Node> },
    Layer {
        id: NodeId,
        from_position: Point,
        to_position: Point,
        from_opacity: f64,
        to_opacity: f64,
    },
}

/// Prepared once per action. `sample` mutates only animated nodes and existing
/// paint/path storage; it does not clone a document, run auto layout, or rebuild
/// the editor's hit-test index. The host disables input until it installs the
/// final canonical preview, so temporary scene IDs cannot become interactions.
#[derive(Clone, Debug)]
pub struct PrototypeScene {
    pub document: Document,
    pub root: NodeId,
    pub from_bounds: Rect,
    pub to_bounds: Rect,
    pub from_origin: Point,
    pub to_origin: Point,
    pub transition: Transition,
    pub progress: f64,
    tracks: Vec<PrototypeTrack>,
}

impl PrototypeScene {
    pub fn new(from: PrototypeFrame, to: PrototypeFrame, transition: Transition) -> Self {
        let from_bounds = from.bounds();
        let to_bounds = to.bounds();
        let mut scene = Self {
            document: Document::new("Prototype transition"),
            root: 0,
            from_bounds,
            to_bounds,
            from_origin: from.origin,
            to_origin: to.origin,
            transition,
            progress: 0.0,
            tracks: Vec::new(),
        };
        let page = scene.document.pages[0].id;
        let overlay_change = from.presentation.is_some()
            && from.presentation == to.presentation
            && from.overlays != to.overlays;
        if scene.transition.kind == TransitionKind::SmartAnimate || overlay_change {
            scene.root = scene.merge_nodes(&from.document, from.root, &to.document, to.root, page, 0);
        } else {
            let mut stage = Node::new(
                NodeKind::Frame,
                "__prototype_transition",
                0.0,
                0.0,
                from_bounds.width.max(to_bounds.width),
                from_bounds.height.max(to_bounds.height),
            );
            stage.fills.clear();
            scene.root = insert_scene_node(&mut scene.document, page, stage);
            let outgoing = copy_tree(&from.document, from.root, &mut scene.document, scene.root, 0)
                .expect("captured presentation root exists");
            let incoming = copy_tree(&to.document, to.root, &mut scene.document, scene.root, 0)
                .expect("captured presentation root exists");
            let width = from_bounds.width.max(to_bounds.width);
            let height = from_bounds.height.max(to_bounds.height);
            let zero = Point::default();
            let mut outgoing_end = zero;
            let mut incoming_start = zero;
            // Keeping an opaque source beneath an incoming opaque frame gives
            // a true color dissolve. Fading both creates an unintended dark
            // pulse because ordinary alpha-over weights the source twice.
            let mut outgoing_alpha = if opaque_presentation(&to)
                && to_bounds.width >= from_bounds.width && to_bounds.height >= from_bounds.height
            { (1.0, 1.0) } else { (1.0, 0.0) };
            let mut incoming_alpha = (0.0, 1.0);
            match scene.transition.kind {
                TransitionKind::SlideLeft | TransitionKind::Push => {
                    outgoing_end.x = -width;
                    incoming_start.x = width;
                    outgoing_alpha = (1.0, 1.0);
                    incoming_alpha = (1.0, 1.0);
                }
                TransitionKind::SlideRight => {
                    outgoing_end.x = width;
                    incoming_start.x = -width;
                    outgoing_alpha = (1.0, 1.0);
                    incoming_alpha = (1.0, 1.0);
                }
                TransitionKind::SlideUp => {
                    outgoing_end.y = -height;
                    incoming_start.y = height;
                    outgoing_alpha = (1.0, 1.0);
                    incoming_alpha = (1.0, 1.0);
                }
                TransitionKind::SlideDown => {
                    outgoing_end.y = height;
                    incoming_start.y = -height;
                    outgoing_alpha = (1.0, 1.0);
                    incoming_alpha = (1.0, 1.0);
                }
                TransitionKind::MoveIn => {
                    incoming_start.x = width;
                    outgoing_alpha = (1.0, 1.0);
                    incoming_alpha = (1.0, 1.0);
                }
                TransitionKind::MoveOut => {
                    outgoing_end.x = width;
                    outgoing_alpha = (1.0, 1.0);
                    incoming_alpha = (1.0, 1.0);
                    scene.document.nodes.get_mut(&scene.root).unwrap().children.reverse();
                }
                TransitionKind::Instant => {
                    outgoing_alpha = (0.0, 0.0);
                    incoming_alpha = (1.0, 1.0);
                }
                TransitionKind::Dissolve | TransitionKind::SmartAnimate => {}
            }
            scene.layer_track(outgoing, zero, outgoing_end, outgoing_alpha);
            scene.layer_track(incoming, incoming_start, zero, incoming_alpha);
        }
        scene.sample(0.0);
        scene
    }

    pub fn animated_nodes(&self) -> usize {
        self.tracks.len()
    }

    pub fn eased_progress(&self) -> f64 {
        prototype_easing(self.transition.easing, self.progress)
    }

    /// Returns true at the final sample, including zero-duration transitions.
    pub fn sample(&mut self, elapsed_ms: f64) -> bool {
        self.progress = if self.transition.kind == TransitionKind::Instant
            || self.transition.duration_ms == 0
        {
            1.0
        } else if elapsed_ms.is_finite() {
            (elapsed_ms / self.transition.duration_ms as f64).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let t = self.eased_progress();
        self.document.revision = self.document.revision.wrapping_add(1);
        for track in &self.tracks {
            match track {
                PrototypeTrack::Node { id, from, to } => {
                    if let Some(node) = self.document.nodes.get_mut(id) {
                        interpolate_node(node, from, to, t);
                        node.revision = self.document.revision;
                    }
                }
                PrototypeTrack::Layer {
                    id, from_position, to_position, from_opacity, to_opacity,
                } => {
                    if let Some(node) = self.document.nodes.get_mut(id) {
                        let position = from_position.lerp(*to_position, t);
                        node.x = position.x;
                        node.y = position.y;
                        node.opacity = lerp(*from_opacity, *to_opacity, t).clamp(0.0, 1.0);
                        node.revision = self.document.revision;
                    }
                }
            }
        }
        self.progress >= 1.0
    }

    fn layer_track(&mut self, id: NodeId, from: Point, to: Point, opacity: (f64, f64)) {
        let node = &self.document.nodes[&id];
        let position = Point::new(node.x, node.y);
        self.tracks.push(PrototypeTrack::Layer {
            id,
            from_position: position + from,
            to_position: position + to,
            from_opacity: node.opacity * opacity.0,
            to_opacity: node.opacity * opacity.1,
        });
    }

    fn merge_nodes(
        &mut self,
        from: &Document,
        a: NodeId,
        to: &Document,
        b: NodeId,
        parent: NodeId,
        depth: usize,
    ) -> NodeId {
        let old = &from.nodes[&a];
        let new = &to.nodes[&b];
        let id = insert_scene_node(&mut self.document, parent, new.clone());
        if animated_properties_differ(old, new) {
            self.tracks.push(PrototypeTrack::Node {
                id,
                from: Box::new(old.clone()),
                to: Box::new(new.clone()),
            });
        }
        if depth >= MAX_DOCUMENT_DEPTH - 1 {
            return id;
        }
        // Duplicate sibling names are matched by their authored occurrence,
        // and names are scoped to their parent. Reparented/incompatible layers
        // crossfade instead of morphing an unrelated layer with the same name.
        let mut available: BTreeMap<(&str, u8), VecDeque<NodeId>> = BTreeMap::new();
        let mut old_rank = BTreeMap::new();
        for (index, &child) in old.children.iter().enumerate() {
            old_rank.insert(child, index);
            if let Some(node) = from.nodes.get(&child) {
                available.entry((&node.name, node.kind as u8)).or_default().push_back(child);
            }
        }
        let mut outgoing = Vec::new();
        for &child in &new.children {
            let Some(node) = to.nodes.get(&child) else { continue };
            let matched = available
                .get_mut(&(node.name.as_str(), node.kind as u8))
                .and_then(VecDeque::pop_front);
            if let Some(matched) = matched.filter(|other| compatible_nodes(&from.nodes[other], node)) {
                self.merge_nodes(from, matched, to, child, id, depth + 1);
            } else {
                if let Some(matched) = matched {
                    outgoing.push(matched);
                }
                if let Some(copied) = copy_tree(to, child, &mut self.document, id, depth + 1) {
                    self.appearance_track(copied, true);
                }
            }
        }
        outgoing.extend(available.into_values().flatten());
        outgoing.sort_by_key(|child| old_rank.get(child).copied().unwrap_or(usize::MAX));
        let incoming = std::mem::take(&mut self.document.nodes.get_mut(&id).unwrap().children);
        let mut outgoing_order = Vec::with_capacity(outgoing.len());
        for child in outgoing {
            if let Some(copied) = copy_tree(from, child, &mut self.document, id, depth + 1) {
                self.appearance_track(copied, false);
                outgoing_order.push((old_rank.get(&child).copied().unwrap_or(incoming.len()), copied));
            }
        }
        // Merge stacking order in one pass. Repeated Vec insertion and sibling
        // position scans would make a large unmatched frame quadratic.
        let mut children = Vec::with_capacity(incoming.len() + outgoing_order.len());
        let mut outgoing = outgoing_order.into_iter().peekable();
        for (index, child) in incoming.into_iter().enumerate() {
            while outgoing.peek().is_some_and(|entry| entry.0 <= index) {
                children.push(outgoing.next().unwrap().1);
            }
            children.push(child);
        }
        children.extend(outgoing.map(|entry| entry.1));
        self.document.nodes.get_mut(&id).unwrap().children = children;
        id
    }

    fn appearance_track(&mut self, id: NodeId, entering: bool) {
        let width = self.from_bounds.width.max(self.to_bounds.width);
        let height = self.from_bounds.height.max(self.to_bounds.height);
        let scrim = self.document.nodes[&id].name.starts_with("__prototype_scrim_");
        let offset = if scrim { Point::default() } else {
            match self.transition.kind {
                TransitionKind::SlideLeft | TransitionKind::Push => Point::new(if entering { width } else { -width }, 0.0),
                TransitionKind::SlideRight => Point::new(if entering { -width } else { width }, 0.0),
                TransitionKind::SlideUp => Point::new(0.0, if entering { height } else { -height }),
                TransitionKind::SlideDown => Point::new(0.0, if entering { -height } else { height }),
                TransitionKind::MoveIn | TransitionKind::MoveOut => Point::new(width, 0.0),
                _ => Point::default(),
            }
        };
        if offset != Point::default() {
            self.layer_track(id, if entering { offset } else { Point::default() }, if entering { Point::default() } else { offset }, (1.0, 1.0));
        } else {
            self.layer_track(id, Point::default(), Point::default(), if entering { (0.0, 1.0) } else { (1.0, 0.0) });
        }
    }
}

fn opaque_presentation(frame: &PrototypeFrame) -> bool {
    let Some(root) = frame.document.nodes[&frame.root].children.first().and_then(|id| frame.document.nodes.get(id)) else { return false };
    root.opacity >= 1.0 && root.visible && root.rotation.abs() < GEOMETRY_EPSILON
        && root.x.abs() < GEOMETRY_EPSILON && root.y.abs() < GEOMETRY_EPSILON
        && root.affine_transform.unwrap_or(Transform::IDENTITY) == Transform::IDENTITY
        && root.effects.is_empty()
        && root.corner_radius == 0.0 && root.corner_radii.is_none()
        && root.fills.iter().any(|fill| fill.visible && fill.opacity >= 1.0 && match &fill.paint {
            Paint::Solid(color) => color.a == 255,
            Paint::LinearGradient { stops, .. } | Paint::RadialGradient { stops, .. }
            | Paint::AngularGradient { stops, .. } => !stops.is_empty() && stops.iter().all(|stop| stop.color.a == 255),
            Paint::Image { .. } => false,
        })
}

/// CSS-compatible cubic curves and a damped spring with exact endpoints.
/// Spring overshoot is preserved for geometry; opacity/color channels clamp.
pub fn prototype_easing(easing: Easing, progress: f64) -> f64 {
    let t = if progress.is_finite() { progress.clamp(0.0, 1.0) } else { 0.0 };
    if t == 0.0 || t == 1.0 { return t; }
    let cubic = |x1: f64, y1: f64, x2: f64, y2: f64| {
        let coordinate = |s: f64, a: f64, b: f64| {
            3.0 * (1.0 - s).powi(2) * s * a + 3.0 * (1.0 - s) * s * s * b + s.powi(3)
        };
        let (mut low, mut high) = (0.0, 1.0);
        for _ in 0..24 {
            let middle = (low + high) * 0.5;
            if coordinate(middle, x1, x2) < t { low = middle; } else { high = middle; }
        }
        coordinate((low + high) * 0.5, y1, y2)
    };
    match easing {
        Easing::Linear => t,
        Easing::EaseIn => cubic(0.42, 0.0, 1.0, 1.0),
        Easing::EaseOut => cubic(0.0, 0.0, 0.58, 1.0),
        Easing::EaseInOut => cubic(0.42, 0.0, 0.58, 1.0),
        Easing::Spring => {
            let spring = |x: f64| 1.0 - (-6.0 * x).exp() * ((10.0 * x).cos() + 0.6 * (10.0 * x).sin());
            spring(t) / spring(1.0)
        }
    }
}

fn insert_scene_node(document: &mut Document, parent: NodeId, mut node: Node) -> NodeId {
    let id = document.allocate_id();
    node.id = id;
    node.parent = Some(parent);
    node.children.clear();
    node.interactions.clear();
    node.revision = 1;
    document.nodes.insert(id, node);
    if let Some(parent) = document.nodes.get_mut(&parent) {
        parent.children.push(id);
    } else if let Some(page) = document.page_mut(parent) {
        page.children.push(id);
    }
    id
}

fn copy_tree(source: &Document, root: NodeId, target: &mut Document, parent: NodeId, depth: usize) -> Option<NodeId> {
    if depth >= MAX_DOCUMENT_DEPTH { return None; }
    let node = source.nodes.get(&root)?;
    let id = insert_scene_node(target, parent, node.clone());
    for &child in &node.children {
        copy_tree(source, child, target, id, depth + 1);
    }
    Some(id)
}

fn compatible_paint(a: &Paint, b: &Paint) -> bool {
    match (a, b) {
        (Paint::Solid(_), Paint::Solid(_)) => true,
        (Paint::LinearGradient { stops: a, .. }, Paint::LinearGradient { stops: b, .. })
        | (Paint::RadialGradient { stops: a, .. }, Paint::RadialGradient { stops: b, .. })
        | (Paint::AngularGradient { stops: a, .. }, Paint::AngularGradient { stops: b, .. }) => a.len() == b.len(),
        (Paint::Image { href: a, scale: sa, crop: ca }, Paint::Image { href: b, scale: sb, crop: cb }) => a == b && sa == sb && ca == cb,
        _ => false,
    }
}

fn compatible_nodes(a: &Node, b: &Node) -> bool {
    a.kind == b.kind && a.visible == b.visible && a.flip_x == b.flip_x && a.flip_y == b.flip_y
        && a.blend_mode == b.blend_mode && a.clip_content == b.clip_content
        && a.text == b.text && a.font_family == b.font_family && a.text_runs == b.text_runs
        && a.text_paragraphs == b.text_paragraphs && a.text_direction == b.text_direction && a.font_features == b.font_features
        && a.text_align == b.text_align && a.text_vertical_align == b.text_vertical_align
        && a.italic == b.italic && a.text_decoration == b.text_decoration
        && a.polygon_sides == b.polygon_sides && a.boolean_operation == b.boolean_operation
        && a.vector_network == b.vector_network
        && (a.vector_network.is_none() || (a.width == b.width && a.height == b.height))
        && a.path.fill_rule == b.path.fill_rule && a.path.segments.len() == b.path.segments.len()
        && a.path.segments.iter().zip(&b.path.segments).all(|(a, b)| std::mem::discriminant(a) == std::mem::discriminant(b))
        && a.fills.len() == b.fills.len()
        && a.fills.iter().zip(&b.fills).all(|(a, b)| compatible_paint(&a.paint, &b.paint))
        && a.strokes.len() == b.strokes.len()
        && a.strokes.iter().zip(&b.strokes).all(|(a, b)| a.align == b.align && a.cap == b.cap && a.join == b.join && a.dash.len() == b.dash.len()
            && compatible_width_profile(&a.width_profile, &b.width_profile) && compatible_paint(&a.paint, &b.paint))
        && a.effects.len() == b.effects.len()
        && a.effects.iter().zip(&b.effects).all(|(a, b)| a.kind == b.kind)
}

fn compatible_width_profile(a: &crate::StrokeWidthProfile, b: &crate::StrokeWidthProfile) -> bool {
    match (a, b) {
        (crate::StrokeWidthProfile::Custom { points: a }, crate::StrokeWidthProfile::Custom { points: b }) =>
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| a.position == b.position),
        _ => a == b,
    }
}

fn animated_properties_differ(a: &Node, b: &Node) -> bool {
    a.x != b.x || a.y != b.y || a.width != b.width || a.height != b.height
        || a.rotation != b.rotation || a.affine_transform != b.affine_transform
        || a.opacity != b.opacity || a.corner_radius != b.corner_radius || a.corner_radii != b.corner_radii || a.corner_smoothing != b.corner_smoothing
        || a.font_size != b.font_size || a.font_weight != b.font_weight || a.line_height != b.line_height
        || a.letter_spacing != b.letter_spacing || a.star_inner_radius != b.star_inner_radius
        || a.fills != b.fills || a.strokes != b.strokes || a.effects != b.effects || a.path != b.path
}

fn lerp(a: f64, b: f64, t: f64) -> f64 { a + (b - a) * t }

fn interpolate_paint(out: &mut Paint, a: &Paint, b: &Paint, t: f64) {
    match (out, a, b) {
        (Paint::Solid(out), Paint::Solid(a), Paint::Solid(b)) => *out = a.lerp(*b, t),
        (Paint::LinearGradient { start, end, stops }, Paint::LinearGradient { start: sa, end: ea, stops: a }, Paint::LinearGradient { start: sb, end: eb, stops: b }) => {
            *start = sa.lerp(*sb, t); *end = ea.lerp(*eb, t);
            interpolate_stops(stops, a, b, t);
        }
        (Paint::RadialGradient { center, radius, stops }, Paint::RadialGradient { center: ca, radius: ra, stops: a }, Paint::RadialGradient { center: cb, radius: rb, stops: b }) => {
            *center = ca.lerp(*cb, t); *radius = lerp(*ra, *rb, t).max(0.0);
            interpolate_stops(stops, a, b, t);
        }
        (Paint::AngularGradient { center, angle, stops }, Paint::AngularGradient { center: ca, angle: aa, stops: a }, Paint::AngularGradient { center: cb, angle: ab, stops: b }) => {
            *center = ca.lerp(*cb, t);
            // Match rotation tracks: cross the seam by the shortest angular path.
            let start = aa.rem_euclid(360.0);
            *angle = start + ((ab.rem_euclid(360.0) - start + 180.0).rem_euclid(360.0) - 180.0) * t;
            interpolate_stops(stops, a, b, t);
        }
        _ => {}
    }
}

fn interpolate_stops(out: &mut [GradientStop], a: &[GradientStop], b: &[GradientStop], t: f64) {
    for ((out, a), b) in out.iter_mut().zip(a).zip(b) {
        out.offset = lerp(a.offset, b.offset, t).clamp(0.0, 1.0);
        out.color = a.color.lerp(b.color, t);
    }
}

fn interpolate_node(out: &mut Node, a: &Node, b: &Node, t: f64) {
    out.x = lerp(a.x, b.x, t); out.y = lerp(a.y, b.y, t);
    out.width = lerp(a.width, b.width, t).max(0.0); out.height = lerp(a.height, b.height, t).max(0.0);
    let angle = (b.rotation - a.rotation + 180.0).rem_euclid(360.0) - 180.0;
    out.rotation = a.rotation + angle * t;
    if a.affine_transform.is_some() || b.affine_transform.is_some() {
        let a = a.affine_transform.unwrap_or(Transform::IDENTITY);
        let b = b.affine_transform.unwrap_or(Transform::IDENTITY);
        out.affine_transform = Some(Transform {
            a: lerp(a.a, b.a, t), b: lerp(a.b, b.b, t), c: lerp(a.c, b.c, t),
            d: lerp(a.d, b.d, t), e: lerp(a.e, b.e, t), f: lerp(a.f, b.f, t),
        });
    }
    out.opacity = lerp(a.opacity, b.opacity, t).clamp(0.0, 1.0);
    out.corner_radius = lerp(a.corner_radius, b.corner_radius, t).max(0.0);
    out.corner_smoothing = lerp(a.corner_smoothing, b.corner_smoothing, t).clamp(0.0, 1.0);
    if a.corner_radii.is_some() || b.corner_radii.is_some() {
        let a = a.corner_radii.unwrap_or([a.corner_radius; 4]);
        let b = b.corner_radii.unwrap_or([b.corner_radius; 4]);
        out.corner_radii = Some(std::array::from_fn(|i| lerp(a[i], b[i], t).max(0.0)));
    }
    out.font_size = lerp(a.font_size, b.font_size, t).max(0.1);
    out.font_weight = lerp(a.font_weight as f64, b.font_weight as f64, t).round().clamp(1.0, 1000.0) as u16;
    out.line_height = lerp(a.line_height, b.line_height, t).max(0.0);
    out.letter_spacing = lerp(a.letter_spacing, b.letter_spacing, t);
    out.star_inner_radius = lerp(a.star_inner_radius, b.star_inner_radius, t).clamp(0.0, 1.0);
    for ((out, a), b) in out.fills.iter_mut().zip(&a.fills).zip(&b.fills) {
        interpolate_paint(&mut out.paint, &a.paint, &b.paint, t);
        out.visible = a.visible || b.visible;
        out.opacity = lerp(if a.visible { a.opacity } else { 0.0 }, if b.visible { b.opacity } else { 0.0 }, t).clamp(0.0, 1.0);
    }
    for ((out, a), b) in out.strokes.iter_mut().zip(&a.strokes).zip(&b.strokes) {
        interpolate_paint(&mut out.paint, &a.paint, &b.paint, t);
        out.visible = a.visible || b.visible;
        out.width = lerp(a.width, b.width, t).max(0.0);
        out.opacity = lerp(if a.visible { a.opacity } else { 0.0 }, if b.visible { b.opacity } else { 0.0 }, t).clamp(0.0, 1.0);
        for ((out, a), b) in out.dash.iter_mut().zip(&a.dash).zip(&b.dash) { *out = lerp(*a, *b, t).max(0.0); }
        if let (crate::StrokeWidthProfile::Custom { points: out }, crate::StrokeWidthProfile::Custom { points: a },
            crate::StrokeWidthProfile::Custom { points: b }) = (&mut out.width_profile, &a.width_profile, &b.width_profile)
        {
            for ((out, a), b) in out.iter_mut().zip(a).zip(b) { out.width = lerp(a.width, b.width, t).max(0.0); }
        }
    }
    for ((out, a), b) in out.effects.iter_mut().zip(&a.effects).zip(&b.effects) {
        out.visible = a.visible || b.visible;
        let mut ca = a.color; let mut cb = b.color;
        if !a.visible { ca.a = 0; } if !b.visible { cb.a = 0; }
        out.color = ca.lerp(cb, t); out.offset = a.offset.lerp(b.offset, t);
        let blur = matches!(a.kind, EffectKind::LayerBlur | EffectKind::BackgroundBlur);
        out.radius = lerp(if blur && !a.visible { 0.0 } else { a.radius }, if blur && !b.visible { 0.0 } else { b.radius }, t).max(0.0);
        out.spread = lerp(a.spread, b.spread, t);
    }
    for ((out, a), b) in out.path.segments.iter_mut().zip(&a.path.segments).zip(&b.path.segments) {
        match (out, a, b) {
            (PathSegment::MoveTo { point }, PathSegment::MoveTo { point: a }, PathSegment::MoveTo { point: b })
            | (PathSegment::LineTo { point }, PathSegment::LineTo { point: a }, PathSegment::LineTo { point: b }) => *point = a.lerp(*b, t),
            (PathSegment::QuadraticTo { control, point }, PathSegment::QuadraticTo { control: ca, point: a }, PathSegment::QuadraticTo { control: cb, point: b }) => { *control = ca.lerp(*cb, t); *point = a.lerp(*b, t); }
            (PathSegment::CubicTo { control1, control2, point }, PathSegment::CubicTo { control1: c1a, control2: c2a, point: a }, PathSegment::CubicTo { control1: c1b, control2: c2b, point: b }) => { *control1 = c1a.lerp(*c1b, t); *control2 = c2a.lerp(*c2b, t); *point = a.lerp(*b, t); }
            _ => {}
        }
    }
}
