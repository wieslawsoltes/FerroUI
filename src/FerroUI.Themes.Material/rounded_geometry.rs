/*
 * Copyright 2023 The Android Open Source Project
 * Copyright 2024 The Android Open Source Project
 * Copyright 2026 FerroUI contributors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
//! Rounded polygon construction and feature-preserving cubic morphs, ported from
//! AndroidX graphics-shapes at e4d933c98df7baa7ea2b83eea6e086915961e0ff:
//! Cubic.kt, RoundedPolygon.kt, Shapes.kt, PolygonMeasure.kt, FeatureMapping.kt,
//! FloatMapping.kt and Morph.kt. The three-segment length measurer is deliberately
//! retained for feature matching, so corresponding corners move as on Android.

use std::f64::consts::{PI, TAU};
use std::ops::{Add, Div, Mul, Sub};

const DISTANCE_EPSILON: f64 = 1e-4;
const ANGLE_EPSILON: f64 = 1e-6;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct V {
    pub x: f64,
    pub y: f64,
}
impl V {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn dot(self, other: Self) -> f64 {
        self.x * other.x + self.y * other.y
    }
    pub fn length(self) -> f64 {
        self.dot(self).sqrt()
    }
    fn direction(self) -> Self {
        let length = self.length();
        if length > 0.0 {
            self / length
        } else {
            Self::default()
        }
    }
    fn rotate90(self) -> Self {
        Self::new(-self.y, self.x)
    }
    pub fn rotate(self, radians: f64) -> Self {
        Self::new(
            self.x * radians.cos() - self.y * radians.sin(),
            self.x * radians.sin() + self.y * radians.cos(),
        )
    }
    fn lerp(self, other: Self, t: f64) -> Self {
        self * (1.0 - t) + other * t
    }
}
impl Add for V {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y)
    }
}
impl Sub for V {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y)
    }
}
impl Mul<f64> for V {
    type Output = Self;
    fn mul(self, v: f64) -> Self {
        Self::new(self.x * v, self.y * v)
    }
}
impl Div<f64> for V {
    type Output = Self;
    fn div(self, v: f64) -> Self {
        Self::new(self.x / v, self.y / v)
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Cubic(pub [V; 4]);
impl Cubic {
    fn line(start: V, end: V) -> Self {
        Self([
            start,
            start.lerp(end, 1.0 / 3.0),
            start.lerp(end, 2.0 / 3.0),
            end,
        ])
    }
    fn arc(center: V, start: V, end: V) -> Self {
        let a = (start - center).direction();
        let b = (end - center).direction();
        let cosine = a.dot(b).clamp(-1.0, 1.0);
        if cosine > 0.999 {
            return Self::line(start, end);
        }
        let clockwise = a.rotate90().dot(end - center) >= 0.0;
        let k = (start - center).length() * 4.0 / 3.0
            * ((2.0 * (1.0 - cosine)).sqrt() - (1.0 - cosine * cosine).max(0.0).sqrt())
            / (1.0 - cosine)
            * if clockwise { 1.0 } else { -1.0 };
        Self([start, start + a.rotate90() * k, end - b.rotate90() * k, end])
    }
    fn reversed(self) -> Self {
        Self([self.0[3], self.0[2], self.0[1], self.0[0]])
    }
    pub fn point(self, t: f64) -> V {
        let u = 1.0 - t;
        self.0[0] * (u * u * u)
            + self.0[1] * (3.0 * u * u * t)
            + self.0[2] * (3.0 * u * t * t)
            + self.0[3] * (t * t * t)
    }
    pub fn split(self, t: f64) -> (Self, Self) {
        let a = self.0[0].lerp(self.0[1], t);
        let b = self.0[1].lerp(self.0[2], t);
        let c = self.0[2].lerp(self.0[3], t);
        let d = a.lerp(b, t);
        let e = b.lerp(c, t);
        let middle = d.lerp(e, t);
        (
            Self([self.0[0], a, d, middle]),
            Self([middle, e, c, self.0[3]]),
        )
    }
    pub fn subcurve(self, start: f64, end: f64) -> Self {
        let (left, _) = self.split(end);
        if start <= 0.0 {
            left
        } else {
            left.split((start / end).clamp(0.0, 1.0)).1
        }
    }
    pub fn transformed(self, f: impl Fn(V) -> V) -> Self {
        Self(self.0.map(f))
    }
    pub fn interpolate(self, other: Self, t: f64) -> Self {
        Self(std::array::from_fn(|i| self.0[i].lerp(other.0[i], t)))
    }
    fn zero_length(self) -> bool {
        (self.0[0].x - self.0[3].x).abs() < DISTANCE_EPSILON
            && (self.0[0].y - self.0[3].y).abs() < DISTANCE_EPSILON
    }
    fn measure(self, threshold: f64) -> (f64, f64) {
        let mut total = 0.0;
        let mut remainder = threshold;
        let mut previous = self.0[0];
        for i in 1..=3 {
            let progress = i as f64 / 3.0;
            let point = self.point(progress);
            let length = (point - previous).length();
            if length >= remainder {
                return (progress - (1.0 - remainder / length) / 3.0, threshold);
            }
            remainder -= length;
            total += length;
            previous = point;
        }
        (1.0, total)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Rounding {
    pub radius: f64,
    pub smoothing: f64,
}
impl Rounding {
    pub const fn new(radius: f64, smoothing: f64) -> Self {
        Self { radius, smoothing }
    }
}
#[derive(Clone, Debug)]
struct Feature {
    cubics: Vec<Cubic>,
    convex: Option<bool>,
}
#[derive(Clone, Debug)]
pub(crate) struct Polygon {
    features: Vec<Feature>,
    pub center: V,
}
impl Polygon {
    pub fn from_vertices(vertices: &[V], rounding: &[Rounding], center: V) -> Self {
        assert!(vertices.len() >= 3 && vertices.len() == rounding.len());
        let n = vertices.len();
        let corners: Vec<_> = (0..n)
            .map(|i| {
                RoundedCorner::new(
                    vertices[(i + n - 1) % n],
                    vertices[i],
                    vertices[(i + 1) % n],
                    rounding[i],
                )
            })
            .collect();
        let cut_adjustments: Vec<_> = (0..n)
            .map(|i| {
                let round = corners[i].round_cut + corners[(i + 1) % n].round_cut;
                let cut = corners[i].expected_cut() + corners[(i + 1) % n].expected_cut();
                let length = (vertices[i] - vertices[(i + 1) % n]).length();
                if round > length {
                    (length / round, 0.0)
                } else if cut > length {
                    (1.0, (length - round) / (cut - round))
                } else {
                    (1.0, 1.0)
                }
            })
            .collect();
        let curves: Vec<_> = (0..n)
            .map(|i| {
                let cuts = std::array::from_fn::<_, 2, _>(|delta| {
                    let (round, cut) = cut_adjustments[(i + n - 1 + delta) % n];
                    corners[i].round_cut * round
                        + (corners[i].expected_cut() - corners[i].round_cut) * cut
                });
                corners[i].cubics(cuts[0], cuts[1])
            })
            .collect();
        let mut features = Vec::with_capacity(n * 2);
        for i in 0..n {
            let a = vertices[i] - vertices[(i + n - 1) % n];
            let b = vertices[(i + 1) % n] - vertices[i];
            features.push(Feature {
                cubics: curves[i].clone(),
                convex: Some(a.x * b.y - a.y * b.x > 0.0),
            });
            features.push(Feature {
                cubics: vec![Cubic::line(
                    curves[i].last().unwrap().0[3],
                    curves[(i + 1) % n][0].0[0],
                )],
                convex: None,
            });
        }
        Self { features, center }
    }
    pub fn regular(count: usize, radius: f64, rounding: Rounding) -> Self {
        let vertices: Vec<_> = (0..count)
            .map(|i| {
                let angle = TAU * i as f64 / count as f64;
                V::new(angle.cos() * radius, angle.sin() * radius)
            })
            .collect();
        Self::from_vertices(&vertices, &vec![rounding; count], V::default())
    }
    pub fn circle(count: usize) -> Self {
        Self::regular(
            count,
            1.0 / (PI / count as f64).cos(),
            Rounding::new(1.0, 0.0),
        )
    }
    pub fn star(count: usize, inner_radius: f64, outer: Rounding, inner: Rounding) -> Self {
        let vertices: Vec<_> = (0..count * 2)
            .map(|i| {
                let angle = PI * i as f64 / count as f64;
                let radius = if i % 2 == 0 { 1.0 } else { inner_radius };
                V::new(angle.cos() * radius, angle.sin() * radius)
            })
            .collect();
        let rounding: Vec<_> = (0..count * 2)
            .map(|i| if i % 2 == 0 { outer } else { inner })
            .collect();
        Self::from_vertices(&vertices, &rounding, V::default())
    }
    pub fn transformed(&self, f: impl Fn(V) -> V) -> Self {
        Self {
            features: self
                .features
                .iter()
                .map(|feature| Feature {
                    cubics: feature.cubics.iter().map(|c| c.transformed(&f)).collect(),
                    convex: feature.convex,
                })
                .collect(),
            center: f(self.center),
        }
    }
    pub fn normalized(&self) -> Self {
        let bounds = path_bounds(&self.cubics());
        let width = bounds[2] - bounds[0];
        let height = bounds[3] - bounds[1];
        let side = width.max(height);
        let offset = V::new(
            (side - width) / 2.0 - bounds[0],
            (side - height) / 2.0 - bounds[1],
        );
        self.transformed(|p| (p + offset) / side)
    }
    pub fn cubics(&self) -> Vec<Cubic> {
        let mut flattened = Vec::new();
        let first = &self.features[0];
        let split = first.cubics.len() == 3;
        if split {
            let (_, end) = first.cubics[1].split(0.5);
            flattened.extend([end, first.cubics[2]]);
        } else {
            flattened.extend(&first.cubics);
        }
        for feature in self.features.iter().skip(1) {
            flattened.extend(&feature.cubics);
        }
        if split {
            let (start, _) = first.cubics[1].split(0.5);
            flattened.extend([first.cubics[0], start]);
        }
        let mut output: Vec<Cubic> = Vec::new();
        for cubic in flattened {
            if !cubic.zero_length() {
                output.push(cubic);
            } else if let Some(last) = output.last_mut() {
                last.0[3] = cubic.0[3];
            }
        }
        if let Some(first) = output.first().copied() {
            output.last_mut().unwrap().0[3] = first.0[0];
        }
        output
    }
    pub fn rotation_safe_scale(&self) -> f64 {
        let curves = self.cubics();
        let bounds = path_bounds(&curves);
        let radius = curves.iter().fold(0.0_f64, |radius, c| {
            radius
                .max((c.0[0] - self.center).length())
                .max((c.point(0.5) - self.center).length())
        });
        (bounds[2] - bounds[0]).max(bounds[3] - bounds[1]) / (2.0 * radius)
    }
}

struct RoundedCorner {
    p0: V,
    p1: V,
    p2: V,
    d1: V,
    d2: V,
    radius: f64,
    smoothing: f64,
    round_cut: f64,
}
impl RoundedCorner {
    fn new(p0: V, p1: V, p2: V, rounding: Rounding) -> Self {
        let v1 = p0 - p1;
        let v2 = p2 - p1;
        let l1 = v1.length();
        let l2 = v2.length();
        let (d1, d2, radius, smoothing) = if l1 > 0.0 && l2 > 0.0 {
            (v1 / l1, v2 / l2, rounding.radius, rounding.smoothing)
        } else {
            (V::default(), V::default(), 0.0, 0.0)
        };
        let cos = d1.dot(d2).clamp(-1.0, 1.0);
        let sin = (1.0 - cos * cos).max(0.0).sqrt();
        Self {
            p0,
            p1,
            p2,
            d1,
            d2,
            radius,
            smoothing,
            round_cut: if sin > 1e-3 {
                radius * (cos + 1.0) / sin
            } else {
                0.0
            },
        }
    }
    fn expected_cut(&self) -> f64 {
        (1.0 + self.smoothing) * self.round_cut
    }
    fn actual_smoothing(&self, allowed: f64) -> f64 {
        if allowed > self.expected_cut() {
            self.smoothing
        } else if allowed > self.round_cut {
            self.smoothing * (allowed - self.round_cut) / (self.expected_cut() - self.round_cut)
        } else {
            0.0
        }
    }
    fn cubics(&self, allowed0: f64, allowed1: f64) -> Vec<Cubic> {
        let allowed = allowed0.min(allowed1);
        if self.round_cut < DISTANCE_EPSILON
            || allowed < DISTANCE_EPSILON
            || self.radius < DISTANCE_EPSILON
        {
            return vec![Cubic::line(self.p1, self.p1)];
        }
        let cut = allowed.min(self.round_cut);
        let radius = self.radius * cut / self.round_cut;
        let center = self.p1
            + ((self.d1 + self.d2) / 2.0).direction() * (radius * radius + cut * cut).sqrt();
        let c0 = self.p1 + self.d1 * cut;
        let c2 = self.p1 + self.d2 * cut;
        let flank0 = flanking_curve(
            cut,
            self.actual_smoothing(allowed0),
            self.p1,
            self.p0,
            c0,
            c2,
            center,
            radius,
        );
        let flank2 = flanking_curve(
            cut,
            self.actual_smoothing(allowed1),
            self.p1,
            self.p2,
            c2,
            c0,
            center,
            radius,
        )
        .reversed();
        vec![flank0, Cubic::arc(center, flank0.0[3], flank2.0[0]), flank2]
    }
}
fn flanking_curve(
    cut: f64,
    smoothing: f64,
    corner: V,
    side: V,
    intersection: V,
    opposite: V,
    center: V,
    radius: f64,
) -> Cubic {
    let direction = (side - corner).direction();
    let start = corner + direction * cut * (1.0 + smoothing);
    let p = intersection.lerp((intersection + opposite) / 2.0, smoothing);
    let end = center + (p - center).direction() * radius;
    let tangent = (end - center).rotate90();
    let rotated = tangent.rotate90();
    let denominator = direction.dot(rotated);
    let numerator = (end - side).dot(rotated);
    let anchor_end = if denominator.abs() < DISTANCE_EPSILON
        || denominator.abs() < DISTANCE_EPSILON * numerator.abs()
    {
        intersection
    } else {
        side + direction * (numerator / denominator)
    };
    Cubic([start, (start + anchor_end * 2.0) / 3.0, anchor_end, end])
}

/// Matches concave to concave and convex to convex corners before splitting
/// curves, preserving the AndroidX shape sequence through a cubic interpolation.
pub(crate) struct Morph {
    pairs: Vec<(Cubic, Cubic)>,
}
impl Morph {
    pub fn new(start: &Polygon, end: &Polygon) -> Self {
        let first = MeasuredPolygon::new(start);
        let second = MeasuredPolygon::new(end);
        let mapping = feature_mapping(&first.features, &second.features);
        let cut = map_progress(&mapping, 0.0, false);
        let second = second.cut_and_shift(cut);
        let mut i1 = 1;
        let mut i2 = 1;
        let mut b1 = first.cubics.first().copied();
        let mut b2 = second.cubics.first().copied();
        let mut pairs = Vec::new();
        while let (Some(a), Some(b)) = (b1, b2) {
            let end1 = if i1 == first.cubics.len() { 1.0 } else { a.end };
            let end2 = if i2 == second.cubics.len() {
                1.0
            } else {
                map_progress(&mapping, (b.end + cut).rem_euclid(1.0), true)
            };
            let minimum = end1.min(end2);
            let seg1 = if end1 > minimum + ANGLE_EPSILON {
                let (left, right) = a.cut(minimum);
                b1 = Some(right);
                left
            } else {
                b1 = first.cubics.get(i1).copied();
                i1 += 1;
                a
            };
            let seg2 = if end2 > minimum + ANGLE_EPSILON {
                let (left, right) =
                    b.cut((map_progress(&mapping, minimum, false) - cut).rem_euclid(1.0));
                b2 = Some(right);
                left
            } else {
                b2 = second.cubics.get(i2).copied();
                i2 += 1;
                b
            };
            pairs.push((seg1.cubic, seg2.cubic));
        }
        debug_assert!(
            b1.is_none() && b2.is_none(),
            "Both shape outlines must be fully matched"
        );
        Self { pairs }
    }
    pub fn cubics(&self, progress: f64) -> Vec<Cubic> {
        let mut result: Vec<_> = self
            .pairs
            .iter()
            .map(|(a, b)| a.interpolate(*b, progress))
            .collect();
        if let Some(first) = result.first().copied() {
            result.last_mut().unwrap().0[3] = first.0[0];
        }
        result
    }
}
#[derive(Clone, Copy)]
struct MeasuredCubic {
    cubic: Cubic,
    start: f64,
    end: f64,
}
impl MeasuredCubic {
    fn cut(self, progress: f64) -> (Self, Self) {
        let progress = progress.clamp(self.start, self.end);
        let size = self.end - self.start;
        let relative = if size > 0.0 {
            (progress - self.start) / size
        } else {
            0.0
        };
        let t = self
            .cubic
            .measure(relative * self.cubic.measure(f64::INFINITY).1)
            .0;
        let (a, b) = self.cubic.split(t);
        (
            Self {
                cubic: a,
                start: self.start,
                end: progress,
            },
            Self {
                cubic: b,
                start: progress,
                end: self.end,
            },
        )
    }
}
#[derive(Clone, Copy)]
struct MeasuredFeature {
    progress: f64,
    point: V,
    convex: bool,
}
struct MeasuredPolygon {
    cubics: Vec<MeasuredCubic>,
    features: Vec<MeasuredFeature>,
}
impl MeasuredPolygon {
    fn new(polygon: &Polygon) -> Self {
        let mut cubics = Vec::new();
        let mut representatives = Vec::new();
        for feature in &polygon.features {
            for (index, cubic) in feature.cubics.iter().enumerate() {
                if let Some(convex) = feature.convex {
                    if index == feature.cubics.len() / 2 {
                        representatives.push((
                            cubics.len(),
                            (feature.cubics[0].0[0] + feature.cubics.last().unwrap().0[3]) / 2.0,
                            convex,
                        ));
                    }
                }
                cubics.push(*cubic);
            }
        }
        let mut lengths = vec![0.0];
        for cubic in &cubics {
            lengths.push(lengths.last().unwrap() + cubic.measure(f64::INFINITY).1);
        }
        let total = *lengths.last().unwrap();
        let progress: Vec<_> = lengths.iter().map(|v| v / total).collect();
        let features = representatives
            .iter()
            .map(|&(index, point, convex)| MeasuredFeature {
                progress: ((progress[index] + progress[index + 1]) / 2.0).rem_euclid(1.0),
                point,
                convex,
            })
            .collect();
        Self {
            cubics: filter_measured(cubics, &progress),
            features,
        }
    }
    fn cut_and_shift(self, cut: f64) -> Self {
        if cut < DISTANCE_EPSILON {
            return self;
        }
        let target = self
            .cubics
            .iter()
            .position(|c| cut >= c.start && cut <= c.end)
            .unwrap();
        let (first, last) = self.cubics[target].cut(cut);
        let n = self.cubics.len();
        let mut curves = vec![last.cubic];
        for offset in 1..n {
            curves.push(self.cubics[(target + offset) % n].cubic);
        }
        curves.push(first.cubic);
        let mut progress = vec![0.0];
        for index in 1..n + 1 {
            progress.push((self.cubics[(target + index - 1) % n].end - cut).rem_euclid(1.0));
        }
        progress.push(1.0);
        Self {
            cubics: filter_measured(curves, &progress),
            features: self.features,
        }
    }
}
fn filter_measured(cubics: Vec<Cubic>, progress: &[f64]) -> Vec<MeasuredCubic> {
    let mut result = Vec::new();
    let mut start = 0.0;
    for (i, cubic) in cubics.into_iter().enumerate() {
        if progress[i + 1] - progress[i] > DISTANCE_EPSILON {
            result.push(MeasuredCubic {
                cubic,
                start,
                end: progress[i + 1],
            });
            start = progress[i + 1];
        }
    }
    if let Some(last) = result.last_mut() {
        last.end = 1.0;
    }
    result
}
fn progress_distance(a: f64, b: f64) -> f64 {
    let difference = (a - b).abs();
    difference.min(1.0 - difference)
}
fn in_range(value: f64, start: f64, end: f64) -> bool {
    if end >= start {
        value >= start && value <= end
    } else {
        value >= start || value <= end
    }
}
fn feature_mapping(first: &[MeasuredFeature], second: &[MeasuredFeature]) -> Vec<(f64, f64)> {
    let mut candidates = Vec::new();
    for (i, a) in first.iter().enumerate() {
        for (j, b) in second.iter().enumerate() {
            if a.convex == b.convex {
                let d = a.point - b.point;
                candidates.push((d.dot(d), i, j));
            }
        }
    }
    candidates.sort_by(|a, b| a.0.total_cmp(&b.0));
    if candidates.is_empty() {
        return vec![(0.0, 0.0), (0.5, 0.5)];
    }
    if candidates.len() == 1 {
        let (_, i, j) = candidates[0];
        let (a, b) = (first[i].progress, second[j].progress);
        return vec![(a, b), ((a + 0.5) % 1.0, (b + 0.5) % 1.0)];
    }
    let mut used1 = vec![false; first.len()];
    let mut used2 = vec![false; second.len()];
    let mut mapping: Vec<(f64, f64)> = Vec::new();
    for (_, i, j) in candidates {
        if used1[i] || used2[j] {
            continue;
        }
        let a = first[i].progress;
        let b = second[j].progress;
        let index = mapping.partition_point(|v| v.0 < a);
        let n = mapping.len();
        if n >= 1 {
            let before = mapping[(index + n - 1) % n];
            let after = mapping[index % n];
            if progress_distance(a, before.0) < DISTANCE_EPSILON
                || progress_distance(a, after.0) < DISTANCE_EPSILON
                || progress_distance(b, before.1) < DISTANCE_EPSILON
                || progress_distance(b, after.1) < DISTANCE_EPSILON
            {
                continue;
            }
            if n > 1 && !in_range(b, before.1, after.1) {
                continue;
            }
        }
        mapping.insert(index, (a, b));
        used1[i] = true;
        used2[j] = true;
    }
    // Normal polygons have several corners; keep a stable mapping for degenerate
    // single-corner custom data as well.
    if mapping.len() == 1 {
        let (a, b) = mapping[0];
        mapping.push(((a + 0.5) % 1.0, (b + 0.5) % 1.0));
        mapping.sort_by(|a, b| a.0.total_cmp(&b.0));
    }
    mapping
}
fn map_progress(mapping: &[(f64, f64)], value: f64, reverse: bool) -> f64 {
    let values = |i: usize| {
        let (a, b) = mapping[i];
        if reverse {
            (b, a)
        } else {
            (a, b)
        }
    };
    let i = (0..mapping.len())
        .find(|&i| in_range(value, values(i).0, values((i + 1) % mapping.len()).0))
        .unwrap_or(0);
    let (x0, y0) = values(i);
    let (x1, y1) = values((i + 1) % mapping.len());
    let xsize = (x1 - x0).rem_euclid(1.0);
    let ysize = (y1 - y0).rem_euclid(1.0);
    let fraction = if xsize < 0.001 {
        0.5
    } else {
        (value - x0).rem_euclid(1.0) / xsize
    };
    (y0 + ysize * fraction).rem_euclid(1.0)
}

/// Android Path.getBounds uses the control-point box, as does normalization.
pub(crate) fn path_bounds(cubics: &[Cubic]) -> [f64; 4] {
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for cubic in cubics {
        for p in cubic.0 {
            bounds[0] = bounds[0].min(p.x);
            bounds[1] = bounds[1].min(p.y);
            bounds[2] = bounds[2].max(p.x);
            bounds[3] = bounds[3].max(p.y);
        }
    }
    bounds
}
/// Accurate arc-length indexing for trimming a rendered path. Cubics remain
/// cubics in the output; the table only determines subdivision parameters.
pub(crate) struct PathMeasure {
    cubics: Vec<Cubic>,
    tables: Vec<[f64; 65]>,
    ends: Vec<f64>,
    pub length: f64,
}
impl PathMeasure {
    pub fn new(cubics: Vec<Cubic>) -> Self {
        let mut tables = Vec::new();
        let mut ends = Vec::new();
        let mut length = 0.0;
        for cubic in &cubics {
            let mut table = [0.0; 65];
            let mut previous = cubic.0[0];
            for i in 1..=64 {
                let point = cubic.point(i as f64 / 64.0);
                table[i] = table[i - 1] + (point - previous).length();
                previous = point;
            }
            length += table[64];
            tables.push(table);
            ends.push(length);
        }
        Self {
            cubics,
            tables,
            ends,
            length,
        }
    }
    fn parameter(table: &[f64; 65], distance: f64) -> f64 {
        if distance <= 0.0 {
            return 0.0;
        }
        if distance >= table[64] {
            return 1.0;
        }
        let index = table.partition_point(|v| *v < distance).clamp(1, 64);
        let span = table[index] - table[index - 1];
        ((index - 1) as f64
            + if span > 0.0 {
                (distance - table[index - 1]) / span
            } else {
                0.0
            })
            / 64.0
    }
    pub fn segment(&self, start: f64, end: f64) -> Vec<Cubic> {
        let mut output = Vec::new();
        if end <= start || self.length <= 0.0 {
            return output;
        }
        // Allow a wrapped path, corresponding to Android's repeated toPath.
        let first = (start / self.length).floor() as i64;
        let last = (end / self.length).floor() as i64;
        for repetition in first..=last {
            let shift = repetition as f64 * self.length;
            for (i, cubic) in self.cubics.iter().enumerate() {
                let low = if i == 0 { 0.0 } else { self.ends[i - 1] };
                let high = self.ends[i];
                let a = (start - shift).max(low);
                let b = (end - shift).min(high);
                if b <= a {
                    continue;
                }
                output.push(cubic.subcurve(
                    Self::parameter(&self.tables[i], a - low),
                    Self::parameter(&self.tables[i], b - low),
                ));
            }
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn assert_contiguous(cubics: &[Cubic]) {
        assert!(!cubics.is_empty());
        for i in 0..cubics.len() {
            assert!((cubics[i].0[3] - cubics[(i + 1) % cubics.len()].0[0]).length() < 1e-8);
            for point in cubics[i].0 {
                assert!(point.x.is_finite() && point.y.is_finite());
            }
        }
    }
    #[test]
    fn rounded_circle_and_star_match_native_dimensions() {
        for count in 5..=24 {
            let circle = Polygon::circle(count).normalized();
            let star = Polygon::star(
                count,
                0.75,
                Rounding::new(0.35, 0.4),
                Rounding::new(0.5, 0.0),
            )
            .normalized();
            for shape in [&circle, &star] {
                let cubics = shape.cubics();
                assert_contiguous(&cubics);
                let bounds = path_bounds(&cubics);
                assert!(bounds.iter().all(|v| *v >= -1e-9 && *v <= 1.0 + 1e-9));
            }
            let morph = Morph::new(&circle, &star);
            for progress in [0.0, 0.25, 0.5, 0.75, 1.0] {
                assert_contiguous(&morph.cubics(progress));
            }
        }
    }
    #[test]
    fn path_subdivision_preserves_the_original_cubic_curve() {
        let c = Cubic([
            V::new(0.0, 0.0),
            V::new(0.0, 1.0),
            V::new(1.0, 1.0),
            V::new(1.0, 0.0),
        ]);
        let sub = c.subcurve(0.2, 0.7);
        for i in 0..=20 {
            let t = i as f64 / 20.0;
            assert!((sub.point(t) - c.point(0.2 + 0.5 * t)).length() < 1e-12);
        }
        let path = PathMeasure::new(vec![c]);
        let segments = path.segment(path.length * 0.25, path.length * 0.75);
        assert_eq!(1, segments.len());
        assert!((segments[0].0[0].y - segments[0].0[3].y).abs() < 1e-10);
    }
    #[test]
    fn differently_sided_shapes_morph_without_losing_outline() {
        let circle = Polygon::circle(10).normalized();
        for count in 3..=12 {
            let polygon = Polygon::regular(count, 1.0, Rounding::new(0.2, 0.0)).normalized();
            let morph = Morph::new(&circle, &polygon);
            for p in [0.0, 0.25, 0.5, 1.0, 1.05] {
                assert_contiguous(&morph.cubics(p));
            }
            let expected = PathMeasure::new(polygon.cubics()).length;
            let actual = PathMeasure::new(morph.cubics(1.0)).length;
            assert!((expected - actual).abs() < 0.0001);
        }
    }
}

/// The seven MaterialShapes used by LoadingIndicatorDefaults in source order:
/// SoftBurst, Cookie9Sided, Pentagon, Pill, Sunny, Cookie4Sided, and Oval.
/// The coordinates and per-vertex radii come from pinned MaterialShapes.kt.
pub(crate) fn loading_polygons() -> Vec<Polygon> {
    let custom = |points: &[(f64, f64, f64)], reps: usize, mirroring: bool| {
        let center = V::new(0.5, 0.5);
        let mut vertices = Vec::new();
        let mut rounding = Vec::new();
        if mirroring {
            let actual_reps = reps * 2;
            let section = TAU / actual_reps as f64;
            let first = V::new(points[0].0, points[0].1) - center;
            let first_angle = first.y.atan2(first.x);
            for repetition in 0..actual_reps {
                for index in 0..points.len() {
                    let i = if repetition % 2 == 0 {
                        index
                    } else {
                        points.len() - index - 1
                    };
                    if i > 0 || repetition % 2 == 0 {
                        let vector = V::new(points[i].0, points[i].1) - center;
                        let angle = vector.y.atan2(vector.x);
                        let a = section * repetition as f64
                            + if repetition % 2 == 0 {
                                angle
                            } else {
                                section - angle + 2.0 * first_angle
                            };
                        vertices.push(V::new(a.cos(), a.sin()) * vector.length() + center);
                        rounding.push(Rounding::new(points[i].2, 0.0));
                    }
                }
            }
        } else {
            for repetition in 0..reps {
                for &(x, y, radius) in points {
                    vertices.push(
                        (V::new(x, y) - center).rotate(TAU * repetition as f64 / reps as f64)
                            + center,
                    );
                    rounding.push(Rounding::new(radius, 0.0));
                }
            }
        }
        Polygon::from_vertices(&vertices, &rounding, center).normalized()
    };
    vec![
        custom(&[(0.193, 0.277, 0.053), (0.176, 0.055, 0.053)], 10, false),
        Polygon::star(9, 0.8, Rounding::new(0.5, 0.0), Rounding::new(0.5, 0.0))
            .transformed(|p| p.rotate(-PI / 2.0))
            .normalized(),
        custom(
            &[
                (0.5, -0.009, 0.172),
                (1.030, 0.365, 0.164),
                (0.828, 0.970, 0.169),
            ],
            1,
            true,
        ),
        custom(
            &[
                (0.961, 0.039, 0.426),
                (1.001, 0.428, 0.0),
                (1.0, 0.609, 1.0),
            ],
            2,
            true,
        ),
        Polygon::star(8, 0.8, Rounding::new(0.15, 0.0), Rounding::new(0.15, 0.0)).normalized(),
        custom(&[(1.237, 1.236, 0.258), (0.5, 0.918, 0.233)], 4, false),
        Polygon::circle(8)
            .transformed(|p| V::new(p.x, p.y * 0.64).rotate(-PI / 4.0))
            .normalized(),
    ]
}

#[cfg(test)]
mod loading_tests {
    use super::*;
    #[test]
    fn all_seven_native_loading_shapes_morph_contiguously() {
        let shapes = loading_polygons();
        assert_eq!(7, shapes.len());
        for i in 0..shapes.len() {
            let a = shapes[i].normalized();
            let b = shapes[(i + 1) % shapes.len()].normalized();
            let morph = Morph::new(&a, &b);
            for progress in [0.0, 0.3, 0.7, 1.0, 1.09] {
                let cubics = morph.cubics(progress);
                for j in 0..cubics.len() {
                    assert!((cubics[j].0[3] - cubics[(j + 1) % cubics.len()].0[0]).length() < 1e-8);
                }
            }
            let target = PathMeasure::new(b.cubics()).length;
            let output = PathMeasure::new(morph.cubics(1.0)).length;
            assert!(
                (target - output).abs() < 0.0002,
                "shape{i}: target{target}, output{output}"
            );
            assert!(a.rotation_safe_scale() > 0.75 && a.rotation_safe_scale() < 1.1);
        }
    }
}
