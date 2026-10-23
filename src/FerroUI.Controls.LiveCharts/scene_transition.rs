//! Deterministic scene transitions driven by the host animation clock.
//!
//! Point identities are supplied by the chart engine. A point can own several
//! primitives (a financial body and wick, for example); geometry kind and stable
//! occurrence order disambiguate those primitives. No platform timer is owned
//! here and no graphics-backend objects cross the calculation boundary.

use ferroui_base::{
    media::{Color, IBrush},
    Point, Rect, Size,
};
use ferroui_livecharts::{
    easing_functions::{EasingFunction, EasingFunctions},
    paint::{brush_or_color, fade_brush, interpolate_brush},
    Geometry, HitRegion, HitShape, PaintStyle, PathCommand, PointId, Primitive, Scene,
};
use std::{
    collections::{HashMap, VecDeque},
    rc::Rc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum GeometryKind {
    Line,
    Polyline,
    Polygon,
    Rectangle,
    Ellipse,
    Path,
    Text,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum EntityKey {
    Host(u64, u64),
    Index(PointId),
    Anonymous,
}

fn point_key(scene: &Scene, id: PointId) -> EntityKey {
    scene
        .entity_keys
        .get(&id)
        .map_or(EntityKey::Index(id), |&(series, point)| {
            EntityKey::Host(series, point)
        })
}

fn primitive_entity_key(scene: &Scene, primitive: &Primitive) -> Option<(u64, u64)> {
    primitive.entity_key.or_else(|| {
        primitive
            .id
            .and_then(|id| scene.entity_keys.get(&id).copied())
    })
}

fn primitive_key(scene: &Scene, primitive: &Primitive) -> EntityKey {
    if let Some((series, point)) = primitive_entity_key(scene, primitive) {
        EntityKey::Host(series, point)
    } else {
        primitive.id.map_or(EntityKey::Anonymous, EntityKey::Index)
    }
}

fn geometry_kind(geometry: &Geometry) -> GeometryKind {
    match geometry {
        Geometry::Line { .. } => GeometryKind::Line,
        Geometry::Polyline(_) => GeometryKind::Polyline,
        Geometry::Polygon(_) => GeometryKind::Polygon,
        Geometry::Rectangle { .. } => GeometryKind::Rectangle,
        Geometry::Ellipse(_) => GeometryKind::Ellipse,
        Geometry::Path(_) => GeometryKind::Path,
        Geometry::Text { .. } => GeometryKind::Text,
    }
}

/// A transition between two complete, immutable measured scenes.
///
/// Retarget by passing `Rc::new(previous.sample(now_ms))` as the next `from`
/// scene. The first sample of the new transition is exactly that scene, including
/// partially faded primitives; no geometry jumps back to its previous target.
pub struct SceneTransition {
    from: Rc<Scene>,
    to: Rc<Scene>,
    started_ms: f64,
    duration_ms: f64,
    easing: EasingFunction,
    pairs: Vec<(Option<usize>, usize)>,
    exits: Vec<usize>,
    hit_pairs: Vec<Option<usize>>,
}

impl SceneTransition {
    pub fn new(from: Rc<Scene>, to: Rc<Scene>, started_ms: f64, duration_ms: f64) -> Self {
        let mut old: HashMap<(EntityKey, GeometryKind), VecDeque<usize>> = HashMap::new();
        for (index, primitive) in from.primitives.iter().enumerate() {
            old.entry((
                primitive_key(&from, primitive),
                geometry_kind(&primitive.geometry),
            ))
            .or_default()
            .push_back(index);
        }
        let mut used = vec![false; from.primitives.len()];
        let pairs = to
            .primitives
            .iter()
            .enumerate()
            .map(|(to_index, primitive)| {
                let from_index = old
                    .get_mut(&(
                        primitive_key(&to, primitive),
                        geometry_kind(&primitive.geometry),
                    ))
                    .and_then(VecDeque::pop_front);
                if let Some(index) = from_index {
                    used[index] = true;
                }
                (from_index, to_index)
            })
            .collect();
        let exits = used
            .into_iter()
            .enumerate()
            .filter_map(|(index, used)| (!used).then_some(index))
            .collect();
        let mut old_hits: HashMap<EntityKey, VecDeque<usize>> = HashMap::new();
        for (index, hit) in from.hits.iter().enumerate() {
            old_hits
                .entry(point_key(&from, hit.id))
                .or_default()
                .push_back(index);
        }
        let hit_pairs = to
            .hits
            .iter()
            .map(|hit| {
                old_hits
                    .get_mut(&point_key(&to, hit.id))
                    .and_then(VecDeque::pop_front)
            })
            .collect();
        Self {
            from,
            to,
            started_ms,
            duration_ms,
            easing: Rc::new(EasingFunctions::cubic_out),
            pairs,
            exits,
            hit_pairs,
        }
    }

    pub fn with_easing(mut self, easing: EasingFunction) -> Self {
        self.easing = easing;
        self
    }

    pub fn is_completed(&self, now_ms: f64) -> bool {
        !self.duration_ms.is_finite()
            || self.duration_ms <= 0.0
            || !self.started_ms.is_finite()
            || !now_ms.is_finite()
            || now_ms - self.started_ms >= self.duration_ms
    }

    pub fn sample(&self, now_ms: f64) -> Scene {
        if self.is_completed(now_ms) {
            return (*self.to).clone();
        }
        let progress = ((now_ms - self.started_ms) / self.duration_ms).clamp(0.0, 1.0);
        if progress <= 0.0 {
            return (*self.from).clone();
        }
        let eased = (self.easing)(progress);
        // Custom easing cannot poison rendering with NaN. Finite overshoot is
        // allowed for geometry, while alpha and nonnegative dimensions clamp.
        let geometry_progress = if eased.is_finite() { eased } else { progress };
        let alpha_progress = geometry_progress.clamp(0.0, 1.0);
        let mut scene = Scene::new(
            interpolate_size(self.from.size, self.to.size, geometry_progress),
            interpolate_rect(
                self.from.plot_bounds,
                self.to.plot_bounds,
                geometry_progress,
            ),
        );
        scene
            .primitives
            .reserve(self.pairs.len() * 2 + self.exits.len());
        for &index in &self.exits {
            let mut old = self.from.primitives[index].clone();
            old.entity_key = primitive_entity_key(&self.from, &old);
            old.style.opacity *= 1.0 - alpha_progress;
            if old.style.opacity > 0.0 {
                scene.primitives.push(old);
            }
        }
        for &(old_index, new_index) in &self.pairs {
            let target = &self.to.primitives[new_index];
            if let Some(old) = old_index.map(|i| &self.from.primitives[i]) {
                if let Some(geometry) =
                    interpolate_geometry(&old.geometry, &target.geometry, geometry_progress)
                {
                    if old.style.dashes.len() == target.style.dashes.len() {
                        if let Some(style) =
                            interpolate_style(&old.style, &target.style, geometry_progress)
                        {
                            scene.primitives.push(Primitive {
                                geometry,
                                style,
                                clip: interpolate_clip(
                                    old.clip,
                                    target.clip,
                                    self.from.size,
                                    self.to.size,
                                    geometry_progress,
                                ),
                                id: target.id,
                                entity_key: primitive_entity_key(&self.to, target),
                            });
                            continue;
                        }
                    }
                }
                let mut fading = old.clone();
                fading.entity_key = primitive_entity_key(&self.from, old);
                fading.style.opacity *= 1.0 - alpha_progress;
                if fading.style.opacity > 0.0 {
                    scene.primitives.push(fading);
                }
            }
            let mut entering = target.clone();
            entering.entity_key = primitive_entity_key(&self.to, target);
            entering.style.opacity *= alpha_progress;
            if entering.style.opacity > 0.0 {
                scene.primitives.push(entering);
            }
        }
        scene.hits.reserve(self.to.hits.len());
        for (target, old_index) in self.to.hits.iter().zip(&self.hit_pairs) {
            let hit = if let Some(old) = old_index.map(|i| &self.from.hits[i]) {
                HitRegion {
                    id: target.id,
                    shape: interpolate_hit_shape(&old.shape, &target.shape, geometry_progress)
                        .unwrap_or_else(|| target.shape.clone()),
                    center: interpolate_point(old.center, target.center, geometry_progress),
                    value: target.value,
                    label: target.label.clone(),
                }
            } else {
                target.clone()
            };
            scene.hits.push(hit);
        }
        scene.legend = self.to.legend.clone();
        scene.automatic_finding_strategy = self.to.automatic_finding_strategy;
        scene.entity_keys = self.to.entity_keys.clone();
        scene
    }
}

fn interpolate(a: f64, b: f64, p: f64) -> f64 {
    let delta = b - a;
    if delta.is_finite() {
        a + delta * p
    } else {
        a * (1.0 - p) + b * p
    }
}
fn interpolate_angle(a: f64, b: f64, p: f64) -> f64 {
    a + ((b - a + 180.0).rem_euclid(360.0) - 180.0) * p
}
fn interpolate_point(a: Point, b: Point, p: f64) -> Point {
    Point::new(interpolate(a.x, b.x, p), interpolate(a.y, b.y, p))
}
fn interpolate_size(a: Size, b: Size, p: f64) -> Size {
    Size::new(
        interpolate(a.width, b.width, p).max(0.0),
        interpolate(a.height, b.height, p).max(0.0),
    )
}
fn interpolate_rect(a: Rect, b: Rect, p: f64) -> Rect {
    Rect::new(
        interpolate(a.x, b.x, p),
        interpolate(a.y, b.y, p),
        interpolate(a.width, b.width, p).max(0.0),
        interpolate(a.height, b.height, p).max(0.0),
    )
}
fn interpolate_points(a: &[Point], b: &[Point], p: f64) -> Option<Vec<Point>> {
    (a.len() == b.len()).then(|| {
        a.iter()
            .zip(b)
            .map(|(&a, &b)| interpolate_point(a, b, p))
            .collect()
    })
}
fn interpolate_clip(
    a: Option<Rect>,
    b: Option<Rect>,
    from_size: Size,
    to_size: Size,
    p: f64,
) -> Option<Rect> {
    match (a, b) {
        (None, None) => None,
        (a, b) => Some(interpolate_rect(
            a.unwrap_or(Rect::new(0.0, 0.0, from_size.width, from_size.height)),
            b.unwrap_or(Rect::new(0.0, 0.0, to_size.width, to_size.height)),
            p,
        )),
    }
}
fn interpolate_color(a: Option<Color>, b: Option<Color>, p: f64) -> Option<Color> {
    if a.is_none() && b.is_none() {
        return None;
    }
    let (a, b) = match (a, b) {
        (Some(a), Some(b)) => (a, b),
        (Some(a), None) => (a, Color { a: 0, ..a }),
        (None, Some(b)) => (Color { a: 0, ..b }, b),
        (None, None) => return None,
    };
    let channel = |a: u8, b: u8| interpolate(f64::from(a), f64::from(b), p) as u8;
    Some(Color::from_argb(
        channel(a.a, b.a),
        channel(a.r, b.r),
        channel(a.g, b.g),
        channel(a.b, b.b),
    ))
}
fn interpolate_brush_override(
    a: &Option<Rc<dyn IBrush>>,
    a_color: Option<Color>,
    b: &Option<Rc<dyn IBrush>>,
    b_color: Option<Color>,
    p: f64,
) -> Option<Option<Rc<dyn IBrush>>> {
    if a.is_none() && b.is_none() {
        return Some(None);
    }
    match (brush_or_color(a, a_color), brush_or_color(b, b_color)) {
        (Some(a), Some(b)) => Some(Some(interpolate_brush(&a, &b, p)?)),
        (Some(a), None) => Some(Some(fade_brush(&a, 1.0 - p)?)),
        (None, Some(b)) => Some(Some(fade_brush(&b, p)?)),
        (None, None) => Some(None),
    }
}

fn interpolate_style(a: &PaintStyle, b: &PaintStyle, p: f64) -> Option<PaintStyle> {
    if a.brush_space != b.brush_space {
        return None;
    }
    Some(PaintStyle {
        fill: interpolate_color(a.fill, b.fill, p),
        stroke: interpolate_color(a.stroke, b.stroke, p),
        fill_brush: interpolate_brush_override(&a.fill_brush, a.fill, &b.fill_brush, b.fill, p)?,
        stroke_brush: interpolate_brush_override(
            &a.stroke_brush,
            a.stroke,
            &b.stroke_brush,
            b.stroke,
            p,
        )?,
        brush_space: b.brush_space,
        stroke_width: interpolate(a.stroke_width, b.stroke_width, p).max(0.0),
        stroke_cap: b.stroke_cap,
        stroke_join: b.stroke_join,
        stroke_miter: interpolate(a.stroke_miter, b.stroke_miter, p).max(0.0),
        opacity: interpolate(a.opacity, b.opacity, p).clamp(0.0, 1.0),
        dashes: a
            .dashes
            .iter()
            .zip(&b.dashes)
            .map(|(&a, &b)| interpolate(a, b, p).max(0.0))
            .collect(),
        dash_offset: interpolate(a.dash_offset, b.dash_offset, p),
    })
}
fn interpolate_geometry(a: &Geometry, b: &Geometry, p: f64) -> Option<Geometry> {
    Some(match (a, b) {
        (Geometry::Line { start: a0, end: a1 }, Geometry::Line { start: b0, end: b1 }) => {
            Geometry::Line {
                start: interpolate_point(*a0, *b0, p),
                end: interpolate_point(*a1, *b1, p),
            }
        }
        (Geometry::Polyline(a), Geometry::Polyline(b)) => {
            Geometry::Polyline(interpolate_points(a, b, p)?)
        }
        (Geometry::Polygon(a), Geometry::Polygon(b)) => {
            Geometry::Polygon(interpolate_points(a, b, p)?)
        }
        (
            Geometry::Rectangle {
                rect: a,
                radius: ar,
            },
            Geometry::Rectangle {
                rect: b,
                radius: br,
            },
        ) => Geometry::Rectangle {
            rect: interpolate_rect(*a, *b, p),
            radius: interpolate(*ar, *br, p).max(0.0),
        },
        (Geometry::Ellipse(a), Geometry::Ellipse(b)) => {
            Geometry::Ellipse(interpolate_rect(*a, *b, p))
        }
        (Geometry::Path(a), Geometry::Path(b)) => {
            if a.len() != b.len() {
                return None;
            }
            let commands = a
                .iter()
                .zip(b)
                .map(|(a, b)| {
                    Some(match (a, b) {
                        (PathCommand::MoveTo(a), PathCommand::MoveTo(b)) => {
                            PathCommand::MoveTo(interpolate_point(*a, *b, p))
                        }
                        (PathCommand::LineTo(a), PathCommand::LineTo(b)) => {
                            PathCommand::LineTo(interpolate_point(*a, *b, p))
                        }
                        (
                            PathCommand::CubicTo {
                                control1: a0,
                                control2: a1,
                                end: a2,
                            },
                            PathCommand::CubicTo {
                                control1: b0,
                                control2: b1,
                                end: b2,
                            },
                        ) => PathCommand::CubicTo {
                            control1: interpolate_point(*a0, *b0, p),
                            control2: interpolate_point(*a1, *b1, p),
                            end: interpolate_point(*a2, *b2, p),
                        },
                        (PathCommand::Close, PathCommand::Close) => PathCommand::Close,
                        _ => return None,
                    })
                })
                .collect::<Option<Vec<_>>>()?;
            Geometry::Path(commands)
        }
        (
            Geometry::Text {
                origin: a,
                text: at,
                size: asz,
                rotation: ar,
                anchor: aa,
            },
            Geometry::Text {
                origin: b,
                text: bt,
                size: bsz,
                rotation: br,
                anchor: ba,
            },
        ) if at == bt && aa == ba => Geometry::Text {
            origin: interpolate_point(*a, *b, p),
            text: bt.clone(),
            size: interpolate(*asz, *bsz, p).max(0.0),
            rotation: interpolate_angle(*ar, *br, p),
            anchor: *ba,
        },
        _ => return None,
    })
}
fn interpolate_hit_shape(a: &HitShape, b: &HitShape, p: f64) -> Option<HitShape> {
    Some(match (a, b) {
        (HitShape::Rectangle(a), HitShape::Rectangle(b)) => {
            HitShape::Rectangle(interpolate_rect(*a, *b, p))
        }
        (
            HitShape::Circle {
                center: a,
                radius: ar,
            },
            HitShape::Circle {
                center: b,
                radius: br,
            },
        ) => HitShape::Circle {
            center: interpolate_point(*a, *b, p),
            radius: interpolate(*ar, *br, p).max(0.0),
        },
        (
            HitShape::AnnularSector {
                center: a,
                inner_radius: ai,
                outer_radius: ao,
                start_angle: aa,
                sweep_angle: aw,
            },
            HitShape::AnnularSector {
                center: b,
                inner_radius: bi,
                outer_radius: bo,
                start_angle: ba,
                sweep_angle: bw,
            },
        ) => HitShape::AnnularSector {
            center: interpolate_point(*a, *b, p),
            inner_radius: interpolate(*ai, *bi, p).max(0.0),
            outer_radius: interpolate(*ao, *bo, p).max(0.0),
            start_angle: interpolate_angle(*aa, *ba, p),
            sweep_angle: interpolate(*aw, *bw, p),
        },
        (HitShape::Polygon(a), HitShape::Polygon(b)) => {
            HitShape::Polygon(interpolate_points(a, b, p)?)
        }
        (HitShape::CompoundPolygon(a), HitShape::CompoundPolygon(b)) => {
            if a.len() != b.len() {
                return None;
            }
            HitShape::CompoundPolygon(
                a.iter()
                    .zip(b)
                    .map(|(a, b)| interpolate_points(a, b, p))
                    .collect::<Option<Vec<_>>>()?,
            )
        }
        (
            HitShape::WithHoverArea {
                shape: a,
                bounds: ar,
            },
            HitShape::WithHoverArea {
                shape: b,
                bounds: br,
            },
        ) => HitShape::WithHoverArea {
            shape: Box::new(interpolate_hit_shape(a, b, p)?),
            bounds: interpolate_rect(*ar, *br, p),
        },
        (HitShape::Compound(a), HitShape::Compound(b)) => {
            if a.len() != b.len() {
                return None;
            }
            HitShape::Compound(
                a.iter()
                    .zip(b)
                    .map(|(a, b)| interpolate_hit_shape(a, b, p))
                    .collect::<Option<Vec<_>>>()?,
            )
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scene(x: f64) -> Scene {
        let mut s = Scene::new(Size::new(200.0, 100.0), Rect::new(0.0, 0.0, 200.0, 100.0));
        let id = PointId {
            series: 0,
            point: 0,
        };
        let rect = Rect::new(x, 10.0, 20.0, 40.0);
        s.primitives.push(
            Primitive::new(
                Geometry::Rectangle { rect, radius: 2.0 },
                PaintStyle::fill(Color::from_rgb(255, 0, 0)),
            )
            .identified(id),
        );
        s.hits.push(HitRegion {
            id,
            shape: HitShape::Rectangle(rect),
            center: Point::new(x + 10.0, 30.0),
            value: 1.0,
            label: "A".into(),
        });
        s
    }
    fn linear(from: Scene, to: Scene) -> SceneTransition {
        SceneTransition::new(Rc::new(from), Rc::new(to), 0.0, 100.0)
            .with_easing(Rc::new(EasingFunctions::lineal))
    }
    #[test]
    fn endpoints_are_exact_and_hits_track_morphed_geometry() {
        let a = scene(0.0);
        let b = scene(100.0);
        let t = linear(a.clone(), b.clone());
        assert_eq!(t.sample(0.0), a);
        assert_eq!(t.sample(100.0), b);
        let mid = t.sample(50.0);
        assert_eq!(mid.hits[0].center.x, 60.0);
        assert!(mid.hits[0].contains(Point::new(55.0, 20.0)));
        assert!(!mid.hits[0].contains(Point::new(5.0, 20.0)));
    }
    #[test]
    fn compound_markers_and_category_hover_follow_transition() {
        let mut a = scene(0.0);
        let mut b = scene(100.0);
        for (scene, x) in [(&mut a, 0.0), (&mut b, 100.0)] {
            scene.hits[0].shape = HitShape::WithHoverArea {
                shape: Box::new(HitShape::Compound(vec![
                    HitShape::Circle {
                        center: Point::new(x + 5.0, 10.0),
                        radius: 2.0,
                    },
                    HitShape::Circle {
                        center: Point::new(x + 15.0, 30.0),
                        radius: 2.0,
                    },
                ])),
                bounds: Rect::new(x, 0.0, 20.0, 100.0),
            };
        }
        b.automatic_finding_strategy = ferroui_livecharts::FindingStrategy::CompareOnlyYTakeClosest;
        let mid = linear(a, b).sample(50.0);
        assert_eq!(
            mid.automatic_finding_strategy,
            ferroui_livecharts::FindingStrategy::CompareOnlyYTakeClosest
        );
        assert!(mid.hits[0].contains(Point::new(55.0, 10.0)));
        assert!(mid.hits[0].contains(Point::new(65.0, 30.0)));
        assert!(!mid.hits[0].contains(Point::new(55.0, 50.0)));
        let HitShape::WithHoverArea { bounds, .. } = mid.hits[0].shape else {
            panic!("hover region lost")
        };
        assert_eq!(bounds, Rect::new(50.0, 0.0, 20.0, 100.0));
    }
    #[test]
    fn identity_matches_reordered_points() {
        let mut a = scene(0.0);
        let mut second = a.primitives[0].clone();
        second.id = Some(PointId {
            series: 0,
            point: 1,
        });
        second.geometry = Geometry::Ellipse(Rect::new(50.0, 0.0, 10.0, 10.0));
        a.primitives.push(second);
        let mut b = a.clone();
        b.primitives.reverse();
        let mid = linear(a, b.clone()).sample(50.0);
        assert_eq!(mid.primitives, b.primitives);
    }
    fn two_points(first_x: f64, second_x: f64, keys: [(u64, u64); 2]) -> Scene {
        let mut result = scene(first_x);
        let second = scene(second_x);
        let first_id = PointId {
            series: 0,
            point: 0,
        };
        let second_id = PointId {
            series: 0,
            point: 1,
        };
        let mut primitive = second.primitives[0].clone();
        primitive.id = Some(second_id);
        result.primitives.push(primitive);
        let mut hit = second.hits[0].clone();
        hit.id = second_id;
        result.hits.push(hit);
        result.entity_keys.insert(first_id, keys[0]);
        result.entity_keys.insert(second_id, keys[1]);
        result
    }
    #[test]
    fn host_entity_identity_survives_reordered_event_indices() {
        let a = two_points(0.0, 100.0, [(1, 7), (1, 8)]);
        let b = two_points(200.0, 300.0, [(1, 8), (1, 7)]);
        let mid = linear(a, b).sample(50.0);
        for primitive in &mid.primitives {
            let Geometry::Rectangle { rect, .. } = primitive.geometry else {
                panic!("rectangle lost")
            };
            assert_eq!(rect.x, 150.0);
        }
        assert_eq!(mid.primitives[0].entity_key, Some((1, 8)));
        assert_eq!(mid.primitives[1].entity_key, Some((1, 7)));
        assert_eq!(
            mid.hits[0].id,
            PointId {
                series: 0,
                point: 0
            }
        );
        assert_eq!(
            mid.hits[1].id,
            PointId {
                series: 0,
                point: 1
            }
        );
        assert_eq!((mid.hits[0].center.x, mid.hits[1].center.x), (160.0, 160.0));
    }
    #[test]
    fn replacement_keeps_exiting_identity_when_retargeted_mid_fade() {
        let with_key = |x, key| {
            let mut result = scene(x);
            result.entity_keys.insert(
                PointId {
                    series: 0,
                    point: 0,
                },
                key,
            );
            result
        };
        let mid = linear(with_key(0.0, (1, 7)), with_key(100.0, (1, 8))).sample(50.0);
        assert_eq!(mid.primitives.len(), 2);
        assert_eq!(mid.primitives[0].entity_key, Some((1, 7)));
        assert_eq!(mid.primitives[1].entity_key, Some((1, 8)));
        let retargeted = linear(mid, with_key(200.0, (1, 8))).sample(50.0);
        let replacement = retargeted
            .primitives
            .iter()
            .find(|p| p.entity_key == Some((1, 8)))
            .unwrap();
        let Geometry::Rectangle { rect, .. } = replacement.geometry else {
            panic!("rectangle lost")
        };
        assert_eq!(rect.x, 150.0);
        assert_eq!(replacement.style.opacity, 0.75);
        assert_eq!(retargeted.hits.len(), 1);
        assert_eq!(retargeted.hits[0].center.x, 160.0);
    }
    #[test]
    fn duplicate_host_references_match_by_occurrence() {
        let a = two_points(0.0, 100.0, [(1, 7), (1, 7)]);
        let b = two_points(10.0, 110.0, [(1, 7), (1, 7)]);
        let mid = linear(a, b).sample(50.0);
        assert_eq!(mid.primitives.len(), 2);
        for (primitive, expected_x) in mid.primitives.iter().zip([5.0, 105.0]) {
            let Geometry::Rectangle { rect, .. } = primitive.geometry else {
                panic!("rectangle lost")
            };
            assert_eq!(rect.x, expected_x);
        }
        assert_eq!((mid.hits[0].center.x, mid.hits[1].center.x), (15.0, 115.0));
    }
    #[test]
    fn native_gradient_and_pen_retarget_from_sampled_paint_values() {
        use ferroui_base::media::immutable::{ImmutableGradientStop, ImmutableLinearGradientBrush};
        let painted = |x, channel| {
            let mut scene = scene(x);
            scene.primitives[0].style.fill_brush =
                Some(Rc::new(ImmutableLinearGradientBrush::from_stops(&[
                    ImmutableGradientStop::new(0.0, Color::from_rgb(channel, channel, channel)),
                    ImmutableGradientStop::new(1.0, Color::from_rgb(channel, channel, channel)),
                ])));
            scene
        };
        let a = painted(0.0, 0);
        let mut b = painted(100.0, 255);
        b.primitives[0].style.stroke_width = 5.0;
        b.primitives[0].style.stroke_miter = 8.0;
        b.primitives[0].style.dash_offset = 10.0;
        let mid = linear(a, b).sample(50.0);
        let style = &mid.primitives[0].style;
        assert_eq!(
            (style.stroke_width, style.stroke_miter, style.dash_offset),
            (3.0, 4.0, 5.0)
        );
        let color = |scene: &Scene| {
            scene.primitives[0]
                .style
                .fill_brush
                .as_ref()
                .unwrap()
                .as_gradient_brush()
                .unwrap()
                .gradient_stops()[0]
                .color()
        };
        assert_eq!(color(&mid), Color::from_rgb(127, 127, 127));
        let next = SceneTransition::new(
            Rc::new(mid.clone()),
            Rc::new(painted(200.0, 200)),
            50.0,
            100.0,
        )
        .with_easing(Rc::new(EasingFunctions::lineal));
        assert_eq!(next.sample(50.0), mid);
        assert_eq!(color(&next.sample(100.0)), Color::from_rgb(163, 163, 163));
    }
    #[test]
    fn incompatible_topology_and_changed_text_crossfade() {
        let mut a = scene(0.0);
        a.primitives[0].geometry =
            Geometry::Polyline(vec![Point::new(0.0, 0.0), Point::new(1.0, 1.0)]);
        let mut b = a.clone();
        b.primitives[0].geometry = Geometry::Polyline(vec![
            Point::new(0.0, 0.0),
            Point::new(1.0, 1.0),
            Point::new(2.0, 0.0),
        ]);
        let mid = linear(a, b).sample(50.0);
        assert_eq!(mid.primitives.len(), 2);
        assert!(mid.primitives.iter().all(|p| p.style.opacity == 0.5));
        let text = |value: &str| Geometry::Text {
            origin: Point::new(0.0, 0.0),
            text: value.into(),
            size: 12.0,
            rotation: 0.0,
            anchor: ferroui_livecharts::TextAnchor::Start,
        };
        assert!(interpolate_geometry(&text("1"), &text("2"), 0.5).is_none());
    }
    #[test]
    fn retarget_starts_from_visible_state() {
        let first = linear(scene(0.0), scene(100.0));
        let visible = first.sample(40.0);
        let next =
            SceneTransition::new(Rc::new(visible.clone()), Rc::new(scene(150.0)), 40.0, 100.0);
        assert_eq!(next.sample(40.0), visible);
    }
    #[test]
    fn additions_and_removals_fade_and_removed_hits_are_not_active() {
        let a = scene(0.0);
        let mut b = a.clone();
        b.primitives[0].id = Some(PointId {
            series: 1,
            point: 0,
        });
        b.hits[0].id = PointId {
            series: 1,
            point: 0,
        };
        let mid = linear(a, b).sample(50.0);
        assert_eq!(mid.primitives.len(), 2);
        assert_eq!(mid.hits.len(), 1);
        assert_eq!(mid.hits[0].id.series, 1);
        assert!(mid.primitives.iter().all(|p| p.style.opacity == 0.5));
    }
    #[test]
    fn invalid_timing_completes_without_nan() {
        let end = scene(10.0);
        for duration in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let t = SceneTransition::new(Rc::new(scene(0.0)), Rc::new(end.clone()), 0.0, duration);
            assert!(t.is_completed(0.0));
            assert_eq!(t.sample(0.0), end);
        }
    }
}
