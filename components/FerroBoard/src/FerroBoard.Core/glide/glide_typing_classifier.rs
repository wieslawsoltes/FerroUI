//! Port of Glide/GlideTypingClassifier.cs, including its source pruning/cache quirks.
use crate::{
    layouts::{ComputedKey, KeyDataError, KeyRect},
    text::{
        DictionaryError, NormalizationError, TextCulture, TextUtils, Utf16Text, WordDictionary,
    },
};
use parking_lot::ReentrantMutex;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    sync::{Arc, LazyLock},
};
#[derive(Clone, Copy, Debug, Default)]
pub struct GesturePoint {
    pub x: f64,
    pub y: f64,
}
impl PartialEq for GesturePoint {
    fn eq(&self, o: &Self) -> bool {
        eq(self.x, o.x) && eq(self.y, o.y)
    }
}
impl Eq for GesturePoint {}
fn eq(a: f64, b: f64) -> bool {
    a == b || a.is_nan() && b.is_nan()
}
fn min(a: f64, b: f64) -> f64 {
    if a != b {
        if a.is_nan() || a < b { a } else { b }
    } else if a.is_sign_negative() {
        a
    } else {
        b
    }
}
fn max(a: f64, b: f64) -> f64 {
    -min(-a, -b)
}
fn compare(a: f64, b: f64) -> std::cmp::Ordering {
    a.partial_cmp(&b)
        .unwrap_or_else(|| a.is_nan().cmp(&b.is_nan()).reverse())
}
fn lower(c: u16) -> u16 {
    TextCulture::lower_unit_invariant(c)
}
#[derive(Debug)]
pub enum GlideError {
    Normalization(NormalizationError),
    KeyData(KeyDataError),
    Dictionary(DictionaryError),
    DuplicateLowercaseKey(u16),
}
impl std::fmt::Display for GlideError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Normalization(error) => write!(f, "glide normalization failed: {error}"),
            Self::KeyData(error) => write!(f, "glide key data failed: {error}"),
            Self::Dictionary(error) => write!(f, "glide dictionary failed: {error}"),
            Self::DuplicateLowercaseKey(unit) => {
                write!(f, "duplicate lowercase glide key: U+{unit:04X}")
            }
        }
    }
}
impl std::error::Error for GlideError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Normalization(error) => Some(error),
            Self::KeyData(error) => Some(error),
            Self::Dictionary(error) => Some(error),
            Self::DuplicateLowercaseKey(_) => None,
        }
    }
}
impl From<NormalizationError> for GlideError {
    fn from(e: NormalizationError) -> Self {
        Self::Normalization(e)
    }
}
pub type GlideResult<T> = Result<T, GlideError>;
#[derive(Clone)]
struct Gesture {
    points: Vec<GesturePoint>,
}
impl Default for Gesture {
    fn default() -> Self {
        Self {
            points: Vec::with_capacity(500),
        }
    }
}
impl Gesture {
    fn add(&mut self, x: f64, y: f64) {
        if self.points.len() < 500 {
            self.points.push(GesturePoint { x, y });
        }
    }
    fn first(&self) -> GesturePoint {
        self.points.first().copied().unwrap_or_default()
    }
    fn last(&self) -> GesturePoint {
        self.points.last().copied().unwrap_or_default()
    }
    fn get(&self, i: usize) -> GesturePoint {
        self.points.get(i).copied().unwrap_or_default()
    }
    fn length(&self) -> f64 {
        let mut length = 0.0;
        for p in self.points.windows(2) {
            length += distance(p[0], p[1]);
        }
        length
    }
    fn same(&self, o: &Self) -> bool {
        self.points.len() == o.points.len()
            && self
                .points
                .iter()
                .zip(&o.points)
                .all(|(a, b)| a.x == b.x && a.y == b.y)
    }
    fn resample(&self, n: i32) -> Self {
        let mut result = Self::default();
        self.resample_into(n, &mut result);
        result
    }
    fn resample_into(&self, n: i32, result: &mut Self) {
        result.points.clear();
        if self.points.is_empty() {
            return;
        }
        let step = self.length() / n as f64;
        let first = self.first();
        result.add(first.x, first.y);
        let (mut x, mut y) = (first.x, first.y);
        let mut error = 0.0;
        if self.points.len() == 1 || step <= 0.0 {
            for _ in 0..200 {
                result.add(first.x, first.y);
            }
            return;
        }
        for p in self.points.windows(2) {
            let (mut dx, mut dy) = (p[1].x - p[0].x, p[1].y - p[0].y);
            let norm = (dx * dx + dy * dy).sqrt();
            if norm <= 0.0 {
                continue;
            }
            dx /= norm;
            dy /= norm;
            let mut count = norm / step;
            error += count - count.floor();
            if error > 1.0 {
                count = count.floor() + error.floor();
                error %= 1.0;
            }
            for _ in 0..count as i32 {
                x += dx * step;
                y += dy * step;
                result.add(x, y);
            }
        }
    }
    fn normalized(&self) -> Self {
        let mut result = Self::default();
        self.normalized_into(&mut result);
        result
    }
    fn normalized_into(&self, result: &mut Self) {
        result.points.clear();
        let (mut max_x, mut max_y, mut min_x, mut min_y) =
            (-f64::MAX, -f64::MAX, f64::MAX, f64::MAX);
        for p in &self.points {
            max_x = max(p.x, max_x);
            max_y = max(p.y, max_y);
            min_x = min(p.x, min_x);
            min_y = min(p.y, min_y);
        }
        let (width, height) = (max_x - min_x, max_y - min_y);
        let longest = max(max(width, height), 0.00001);
        let (cx, cy) = (
            (width / 2.0 + min_x) / longest,
            (height / 2.0 + min_y) / longest,
        );
        for p in &self.points {
            result.add(p.x / longest - cx, p.y / longest - cy);
        }
    }
    fn ideal_into(
        word: &Utf16Text,
        keys: &HashMap<u16, KeyRect>,
        ideal: &mut Self,
        loops: &mut Self,
    ) -> GlideResult<bool> {
        ideal.points.clear();
        loops.points.clear();
        let mut previous = 0;
        let mut has_loops = false;
        for &ch in word.units() {
            let c = lower(ch);
            let key = if let Some(key) = keys.get(&c) {
                Some(key)
            } else {
                keys.get(&TextUtils::base_char(c)?)
            };
            let Some(key) = key else { continue };
            let (cx, cy) = (key.center_x(), key.center_y());
            if previous == c {
                loops.add(cx + key.width / 4.0, cy + key.height / 4.0);
                loops.add(cx + key.width / 4.0, cy - key.height / 4.0);
                loops.add(cx - key.width / 4.0, cy - key.height / 4.0);
                loops.add(cx - key.width / 4.0, cy + key.height / 4.0);
                has_loops = true;
                ideal.add(cx, cy);
            } else {
                ideal.add(cx, cy);
                loops.add(cx, cy);
            }
            previous = c;
        }
        Ok(has_loops)
    }
}
fn distance(a: GesturePoint, b: GesturePoint) -> f64 {
    let dx = a.x - b.x;
    let dy = a.y - b.y;
    (dx * dx + dy * dy).sqrt()
}
struct Pruner {
    tree: HashMap<(u16, u16), Vec<Utf16Text>>,
    lengths: HashMap<Utf16Text, f64>,
}
impl Pruner {
    fn new(words: &[(Utf16Text, i32)], keys: &HashMap<u16, KeyRect>) -> GlideResult<Self> {
        let mut tree = HashMap::new();
        for (word, _) in words {
            let a = TextUtils::base_char(word.units()[0])?;
            let b = TextUtils::base_char(*word.units().last().unwrap())?;
            if !keys.contains_key(&a) || !keys.contains_key(&b) {
                continue;
            }
            tree.entry((a, b))
                .or_insert_with(Vec::new)
                .push(word.clone());
        }
        Ok(Self {
            tree,
            lengths: HashMap::new(),
        })
    }
    fn extremities(&self, gesture: &Gesture, keys: &[(u16, KeyRect)]) -> Vec<Utf16Text> {
        let closest = |point| {
            let mut v: Vec<_> = keys
                .iter()
                .map(|(c, r)| {
                    (
                        *c,
                        distance(
                            GesturePoint {
                                x: r.center_x(),
                                y: r.center_y(),
                            },
                            point,
                        ),
                    )
                })
                .collect();
            v.sort_by(|a, b| compare(a.1, b.1));
            v.truncate(2);
            v
        };
        let mut result = Vec::new();
        for (s, _) in closest(gesture.first()) {
            for (e, _) in closest(gesture.last()) {
                if let Some(words) = self.tree.get(&(s, e)) {
                    result.extend(words.iter().cloned());
                }
            }
        }
        result
    }
    fn lengths(
        &mut self,
        gesture: &Gesture,
        words: Vec<Utf16Text>,
        keys: &HashMap<u16, KeyRect>,
        radius: f64,
    ) -> GlideResult<Vec<Utf16Text>> {
        let mut result = Vec::new();
        let user_length = gesture.length();
        if words.is_empty() {
            return Ok(result);
        }
        let (mut ideal, mut loops) = (Gesture::default(), Gesture::default());
        for word in words {
            let has_loops = Gesture::ideal_into(&word, keys, &mut ideal, &mut loops)?;
            for ideal in std::iter::once(&ideal).chain(has_loops.then_some(&loops)) {
                let length = if let Some(length) = self.lengths.get(&word) {
                    *length
                } else {
                    let length = ideal.length();
                    self.lengths.insert(word.clone(), length);
                    length
                };
                if (user_length - length).abs() < 8.42 * radius {
                    result.push(word.clone());
                }
            }
        }
        Ok(result)
    }
}
#[derive(Default)]
struct State {
    gesture: Gesture,
    keys: HashMap<u16, KeyRect>,
    // Geometry encounter order defines sampling radius and stable nearest-key ties.
    ordered_keys: Vec<(u16, KeyRect)>,
    words: Vec<(Utf16Text, i32)>,
    frequencies: HashMap<Utf16Text, i32>,
    pruner: Option<Pruner>,
    threshold: f64,
    cache: Option<(Gesture, i32, Arc<Vec<Utf16Text>>)>,
}
impl State {
    fn rebuild(&mut self) -> GlideResult<()> {
        self.cache = None;
        let pruner = if !self.keys.is_empty() && !self.words.is_empty() {
            Some(Pruner::new(&self.words, &self.keys)?)
        } else {
            None
        };
        self.pruner = pruner;
        Ok(())
    }
    fn add(&mut self, x: f64, y: f64) {
        if !self.gesture.points.is_empty() {
            let last = self.gesture.last();
            let (dx, dy) = (last.x - x, last.y - y);
            if dx * dx + dy * dy <= self.threshold {
                return;
            }
        }
        self.gesture.add(x, y);
    }
    fn suggestions(&mut self, count: i32) -> GlideResult<Arc<Vec<Utf16Text>>> {
        if self.pruner.is_none() || self.gesture.points.len() < 2 {
            static EMPTY: LazyLock<Arc<Vec<Utf16Text>>> = LazyLock::new(|| Arc::new(Vec::new()));
            return Ok(EMPTY.clone());
        }
        if let Some((g, n, result)) = &self.cache
            && *n == count
            && g.same(&self.gesture)
        {
            return Ok(result.clone());
        }
        let result = Arc::new(self.classify(count)?);
        self.cache = Some((self.gesture.clone(), count, result.clone()));
        Ok(result)
    }
    fn classify(&mut self, count: i32) -> GlideResult<Vec<Utf16Text>> {
        let mut candidates = Vec::new();
        let mut weights = Vec::new();
        let Some(pruner) = self.pruner.as_mut() else {
            return Ok(candidates);
        };
        let Some((_, first)) = self.ordered_keys.first() else {
            return Ok(candidates);
        };
        let radius = min(first.height, first.width);
        let remaining = pruner.extremities(&self.gesture, &self.ordered_keys);
        let user = self.gesture.resample(200);
        let normalized_user = user.normalized();
        let remaining = pruner.lengths(&self.gesture, remaining, &self.keys, radius)?;
        if remaining.is_empty() {
            return Ok(candidates);
        }
        let (mut ideal, mut loops) = (Gesture::default(), Gesture::default());
        let (mut gesture, mut normalized) = (Gesture::default(), Gesture::default());
        for word in remaining {
            let has_loops = Gesture::ideal_into(&word, &self.keys, &mut ideal, &mut loops)?;
            for ideal in std::iter::once(&ideal).chain(has_loops.then_some(&loops)) {
                ideal.resample_into(200, &mut gesture);
                gesture.normalized_into(&mut normalized);
                let (mut shape, mut location) = (0.0, 0.0);
                for i in 0..200 {
                    shape += distance(normalized.get(i), normalized_user.get(i));
                    let (a, b) = (gesture.get(i), user.get(i));
                    location += (a.x - b.x).abs() + (a.y - b.y).abs();
                }
                location = location / 200.0 / 2.0;
                let gaussian = |v: f64, std: f64| {
                    let factor = 1.0 / (std * (2.0 * std::f64::consts::PI).sqrt());
                    let exponent = (v / std).powf(2.0);
                    max(f64::from_bits(1), factor * (-0.5 * exponent).exp())
                };
                let frequency = 1.max(*self.frequencies.get(&word).unwrap_or(&1));
                let confidence = 1.0
                    / (gaussian(shape, 22.08)
                        * gaussian(location, 0.5109 * radius)
                        * frequency as f64);
                let mut index = 0;
                let mut duplicate = usize::MAX;
                while index < weights.len() && weights[index] <= confidence {
                    if candidates[index] == word {
                        duplicate = index;
                    }
                    index += 1;
                }
                if (index as i64) < count as i64 && index <= duplicate {
                    if duplicate < usize::MAX {
                        weights.remove(duplicate);
                        candidates.remove(duplicate);
                    }
                    weights.insert(index, confidence);
                    candidates.insert(index, word.clone());
                    if weights.len() as i64 > count as i64 {
                        weights.remove(count as usize);
                        candidates.remove(count as usize);
                    }
                }
            }
        }
        Ok(candidates)
    }
}
/// Shared synchronous classifier; computed-key enumeration and callbacks never retain a state borrow.
#[derive(Default)]
pub struct GlideTypingClassifier {
    gate: ReentrantMutex<RefCell<State>>,
}
impl GlideTypingClassifier {
    pub const PRUNING_LENGTH_THRESHOLD: f64 = 8.42;
    pub const SAMPLING_POINTS: i32 = 200;
    pub const SHAPE_STD: f64 = 22.08;
    pub const LOCATION_STD: f64 = 0.5109;
    pub fn is_ready(&self) -> bool {
        self.gate.lock().borrow().pruner.is_some()
    }
    pub fn is_empty(&self) -> bool {
        self.point_count() == 0
    }
    pub fn point_count(&self) -> usize {
        self.gate.lock().borrow().gesture.points.len()
    }
    pub fn clear(&self) {
        self.gate.lock().borrow_mut().gesture.points.clear();
    }
    pub fn add_point(&self, x: f64, y: f64) {
        self.gate.lock().borrow_mut().add(x, y);
    }
    pub fn set_layout(&self, keys: impl IntoIterator<Item = (u16, KeyRect)>) -> GlideResult<()> {
        let guard = self.gate.lock();
        let input: Vec<_> = keys.into_iter().collect();
        {
            let state = guard.borrow();
            if state.keys.len() == input.len()
                && input.iter().all(|(c, r)| state.keys.get(c) == Some(r))
            {
                return Ok(());
            }
        }
        let mut map = HashMap::new();
        let mut ordered_keys = Vec::with_capacity(input.len());
        for (c, r) in input {
            let c = lower(c);
            if map.insert(c, r).is_some() {
                return Err(GlideError::DuplicateLowercaseKey(c));
            }
            ordered_keys.push((c, r));
        }
        let mut state = guard.borrow_mut();
        state.keys = map;
        state.ordered_keys = ordered_keys;
        let first = state.ordered_keys.first().map(|(_, bounds)| *bounds);
        if let Some(first) = first {
            let width = first.width / 4.0;
            state.threshold = width * width;
        }
        state.rebuild()
    }
    pub fn set_computed_layout<'a>(
        &self,
        keys: impl IntoIterator<Item = &'a ComputedKey>,
    ) -> GlideResult<()> {
        let mut seen = HashSet::new();
        let mut ordered_keys = Vec::new();
        for key in keys {
            let output = key
                .data()
                .as_string(false)
                .map_err(GlideError::KeyData)?
                .ok_or(GlideError::KeyData(KeyDataError::NullReference))?;
            if output.len() != 1 {
                continue;
            }
            let c = output.units()[0];
            if !crate::text::is_letter_unit(c) && c != 39 {
                continue;
            }
            let c = TextUtils::base_char(c)?;
            let bounds = if key.visible_bounds().is_empty() {
                key.touch_bounds()
            } else {
                key.visible_bounds()
            };
            if seen.insert(c) {
                ordered_keys.push((c, bounds));
            }
        }
        self.set_layout(ordered_keys)
    }
    pub fn set_words(&self, words: impl IntoIterator<Item = (Utf16Text, i32)>) -> GlideResult<()> {
        let guard = self.gate.lock();
        let words = words.into_iter().filter(|(w, _)| w.len() > 1).collect();
        let mut state = guard.borrow_mut();
        state.words = words;
        state.frequencies = HashMap::new();
        for i in 0..state.words.len() {
            let (w, f) = state.words[i].clone();
            let frequency = 0.max(*state.frequencies.get(&w).unwrap_or(&0)).max(f);
            state.frequencies.insert(w.clone(), frequency);
        }
        state.rebuild()
    }
    pub fn set_dictionary(&self, dictionary: &WordDictionary) -> GlideResult<()> {
        let source_words = dictionary.words();
        let _guard = self.gate.lock();
        let words = source_words
            .into_iter()
            .map(|w| dictionary.get_frequency(&w).map(|f| (w, f)))
            .collect::<Result<Vec<_>, _>>()
            .map_err(GlideError::Dictionary)?;
        self.set_words(words)
    }
    pub fn get_suggestions(&self, count: i32) -> GlideResult<Arc<Vec<Utf16Text>>> {
        self.gate.lock().borrow_mut().suggestions(count)
    }
    pub fn classify(
        &self,
        points: impl IntoIterator<Item = GesturePoint>,
        count: i32,
    ) -> GlideResult<Arc<Vec<Utf16Text>>> {
        let guard = self.gate.lock();
        guard.borrow_mut().gesture.points.clear();
        for p in points {
            guard.borrow_mut().add(p.x, p.y);
        }
        guard.borrow_mut().suggestions(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    fn number(v: &Value) -> f64 {
        f64::from_bits(u64::from_str_radix(v.as_str().unwrap(), 16).unwrap())
    }
    fn dump(g: &Gesture) -> Value {
        json!({"count":g.points.len(),"length":format!("{:016x}",g.length().to_bits()),"points":g.points.iter().map(|p|[format!("{:016x}",p.x.to_bits()),format!("{:016x}",p.y.to_bits())]).collect::<Vec<_>>()})
    }
    // NaN payload/sign is not stable under optimized floating-point code. Source Gesture
    // comparisons reject every NaN regardless of sign; retain exact bits for every finite/zero value.
    fn canonical_nan(value: Value) -> Value {
        match value {
            Value::String(s)
                if s.len() == 16
                    && u64::from_str_radix(&s, 16)
                        .is_ok_and(|bits| f64::from_bits(bits).is_nan()) =>
            {
                Value::String("NaN".into())
            }
            Value::Array(v) => Value::Array(v.into_iter().map(canonical_nan).collect()),
            Value::Object(v) => {
                Value::Object(v.into_iter().map(|(k, v)| (k, canonical_nan(v))).collect())
            }
            other => other,
        }
    }
    #[test]
    fn source_resampling_and_box_normalization_match_exact_bits() {
        for line in include_str!("../../../tests/fixtures/glide-geometry.jsonl").lines() {
            let row: Value = serde_json::from_str(line).unwrap();
            let mut g = Gesture::default();
            for p in row["input"].as_array().unwrap() {
                g.add(number(&p[0]), number(&p[1]));
            }
            assert_eq!(dump(&g), row["original"]);
            let r = g.resample(200);
            assert_eq!(dump(&r), row["resampled"]);
            assert_eq!(
                canonical_nan(dump(&r.normalized())),
                canonical_nan(row["normalized"].clone())
            );
        }
    }
    #[test]
    fn reused_geometry_buffers_match_source_and_clear_previous_tails() {
        let rows = include_str!("../../../tests/fixtures/glide-geometry.jsonl")
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .collect::<Vec<_>>();
        let mut resampled = Gesture::default();
        let mut normalized = Gesture::default();
        for row in rows.iter().chain(rows.iter().rev()) {
            let mut gesture = Gesture::default();
            for point in row["input"].as_array().unwrap() {
                gesture.add(number(&point[0]), number(&point[1]));
            }
            gesture.resample_into(200, &mut resampled);
            resampled.normalized_into(&mut normalized);
            assert_eq!(dump(&resampled), row["resampled"]);
            assert_eq!(
                canonical_nan(dump(&normalized)),
                canonical_nan(row["normalized"].clone())
            );
            assert_eq!(
                resampled.get(resampled.points.len()),
                GesturePoint::default()
            );
            assert_eq!(
                normalized.get(normalized.points.len()),
                GesturePoint::default()
            );
        }
    }
}
