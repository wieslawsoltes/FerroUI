//! Property-based selection uses borrowed signatures and a single page walk.
//! No document/paint/image clones, transactions, scene invalidations or history
//! entries are needed. Hashes only narrow the candidates: full property equality
//! always verifies a match, including a hash collision.
use super::selectable;
use crate::{Color, Editor, Node, NodeId, NodeKind, Paint, TextRun, MAX_DOCUMENT_DEPTH};
use std::collections::{hash_map::RandomState, BTreeSet, HashMap, HashSet};
use std::hash::{BuildHasher, Hash, Hasher};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectionMatch {
    /// Kind, ordered layer paint/effect stacks, opacity, blend, corners, and
    /// resolved typography. Position, dimensions, paths and style IDs differ.
    Appearance,
    Fill,
    Stroke,
    Effect,
    /// Paragraph properties and the ordered resolved character-style sequence,
    /// independent of text content, byte offsets and adjacent equal spans.
    TextProperties,
    /// The same set of actually used font faces (family, weight and italic).
    Font,
    /// Instances referring to the exact same component, including its variant.
    Instance,
}

impl SelectionMatch {
    pub fn supports(self, node: &Node) -> bool {
        match self {
            Self::TextProperties | Self::Font => node.kind == NodeKind::Text,
            Self::Instance => node.kind == NodeKind::Instance && node.component_id.is_some(),
            _ => true,
        }
    }
}

impl Editor {
    /// Query all visible, unlocked matching layers on the active page, in the
    /// Layers panel's front-to-back depth-first order. A mixed selection forms
    /// the union of its distinct property signatures. Hidden/locked ancestors
    /// prune a subtree; stale and cross-page source IDs never broaden the query.
    pub fn matching_selection(&self, property: SelectionMatch) -> Vec<NodeId> {
        if self.has_active_transaction() {
            return Vec::new();
        }
        let hasher = RandomState::new();
        let mut signatures = HashMap::<u64, Vec<MatchKey<'_>>>::new();
        let mut selected = HashSet::new();
        for &id in &self.selection {
            if !selected.insert(id) || !selectable(self, id) {
                continue;
            }
            let node = &self.document.nodes[&id];
            if !property.supports(node) {
                continue;
            }
            let key = MatchKey::new(node, property);
            let bucket = signatures.entry(key.fingerprint(&hasher)).or_default();
            if !bucket.iter().any(|other| key.matches(other)) {
                bucket.push(key);
            }
        }
        if signatures.is_empty() {
            return Vec::new();
        }
        let mut pending: Vec<_> = self.document.page(self.active_page)
            .map(|page| page.children.iter().map(|&id| (id, 0)).collect())
            .unwrap_or_default();
        let mut visited = HashSet::new();
        let mut result = Vec::new();
        while let Some((id, depth)) = pending.pop() {
            if !visited.insert(id) || depth >= MAX_DOCUMENT_DEPTH {
                continue;
            }
            let Some(node) = self.document.nodes.get(&id) else { continue; };
            if !node.visible || node.locked {
                continue;
            }
            if property.supports(node) {
                let key = MatchKey::new(node, property);
                if signatures.get(&key.fingerprint(&hasher))
                    .is_some_and(|bucket| bucket.iter().any(|source| key.matches(source))) {
                    result.push(id);
                }
            }
            pending.extend(node.children.iter().map(|&child| (child, depth + 1)));
        }
        result
    }

    /// Selection-only command. Unsupported/empty seeds and active gestures are
    /// no-ops; a successful query preserves document identity, redo and caches.
    pub fn select_matching(&mut self, property: SelectionMatch) -> bool {
        let next = self.matching_selection(property);
        if next.is_empty() || next == self.selection {
            return false;
        }
        self.selection = next;
        true
    }

    /// Select the other eligible top-level objects. A root containing a selected
    /// descendant is excluded too, so inversion cannot reselect that descendant
    /// indirectly through its parent. An empty selection selects all roots.
    pub fn invert_selection(&mut self) -> bool {
        if self.has_active_transaction() { return false; }
        let mut excluded = HashSet::new();
        for &id in &self.selection {
            if !selectable(self, id) { continue; }
            let mut current = id;
            for _ in 0..MAX_DOCUMENT_DEPTH {
                let Some(node) = self.document.nodes.get(&current) else { break; };
                match node.parent {
                    Some(parent) if parent != self.active_page => current = parent,
                    _ => { excluded.insert(current); break; }
                }
            }
        }
        let next: Vec<_> = self.document.page(self.active_page).into_iter()
            .flat_map(|page| page.children.iter().rev().copied())
            .filter(|id| !excluded.contains(id) && selectable(self, *id)).collect();
        if next == self.selection { return false; }
        self.selection = next;
        true
    }
}

/// Numeric equality treats either zero representation identically. Document
/// validation excludes NaNs; using integer keys also keeps text-key Eq total.
fn number(value: f64) -> u64 { if value == 0.0 { 0 } else { value.to_bits() } }
fn color(value: Color) -> [u8; 4] { [value.r, value.g, value.b, value.a] }

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct CharacterStyle<'a> {
    family: &'a str,
    size: u64,
    weight: u16,
    italic: bool,
    decoration: u8,
    color: Option<[u8; 4]>,
}
impl<'a> CharacterStyle<'a> {
    fn new(node: &'a Node, run: Option<&'a TextRun>) -> Self {
        Self {
            family: run.and_then(|run| run.font_family.as_deref()).unwrap_or(&node.font_family),
            size: number(run.and_then(|run| run.font_size).unwrap_or(node.font_size)),
            weight: run.and_then(|run| run.font_weight).unwrap_or(node.font_weight),
            italic: run.and_then(|run| run.italic).unwrap_or(node.italic),
            decoration: run.and_then(|run| run.decoration).unwrap_or(node.text_decoration) as u8,
            color: run.and_then(|run| run.color).map(color),
        }
    }
}

fn character_styles(node: &Node) -> Vec<CharacterStyle<'_>> {
    let mut ranges: Vec<_> = node.text_runs.iter().filter(|run| run.start < run.end
        && run.end <= node.text.len() && node.text.is_char_boundary(run.start)
        && node.text.is_char_boundary(run.end)).collect();
    // Match the renderer's stable first-starting-run precedence for imports.
    ranges.sort_by_key(|run| run.start);
    let mut styles = Vec::with_capacity(ranges.len() * 2 + 1);
    let mut append = |style| {
        if styles.last() != Some(&style) { styles.push(style); }
    };
    let mut cursor = 0;
    for run in ranges {
        if run.end <= cursor { continue; }
        if run.start > cursor { append(CharacterStyle::new(node, None)); }
        append(CharacterStyle::new(node, Some(run)));
        cursor = run.end;
    }
    if cursor < node.text.len() || node.text.is_empty() {
        append(CharacterStyle::new(node, None));
    }
    styles
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ParagraphStyle {
    alignment: u8,
    direction: u8,
    line_height: u64,
    letter_spacing: u64,
    before: u64,
    after: u64,
    indent: u64,
}

fn paragraph_styles(node: &Node) -> Vec<ParagraphStyle> {
    let mut result = Vec::new();
    let mut explicit = node.text_paragraphs.iter().peekable();
    for range in crate::text_paragraph_ranges(&node.text) {
        while explicit.peek().is_some_and(|style| style.start < range.start) { explicit.next(); }
        let style = explicit.peek().copied().filter(|style| style.start == range.start && style.end == range.end);
        let resolved = ParagraphStyle {
            alignment: style.and_then(|style| style.alignment).unwrap_or(node.text_align) as u8,
            direction: style.and_then(|style| style.direction).or(node.text_direction).unwrap_or(crate::TextDirection::LeftToRight) as u8,
            line_height: number(style.and_then(|style| style.line_height).unwrap_or(node.line_height)),
            letter_spacing: number(style.and_then(|style| style.letter_spacing).unwrap_or(node.letter_spacing)),
            before: number(style.and_then(|style| style.space_before).unwrap_or(0.0)),
            after: number(style.and_then(|style| style.space_after).unwrap_or(0.0)),
            indent: number(style.and_then(|style| style.indent).unwrap_or(0.0)),
        };
        if result.last() != Some(&resolved) { result.push(resolved); }
    }
    result
}

struct MatchKey<'a> {
    node: &'a Node,
    property: SelectionMatch,
    characters: Vec<CharacterStyle<'a>>,
    paragraphs: Vec<ParagraphStyle>,
    fonts: BTreeSet<(&'a str, u16, bool)>,
    features: Vec<(&'a str, u32)>,
}
impl<'a> MatchKey<'a> {
    fn new(node: &'a Node, property: SelectionMatch) -> Self {
        let characters = if node.kind == NodeKind::Text && matches!(property,
            SelectionMatch::Appearance | SelectionMatch::TextProperties | SelectionMatch::Font) {
            character_styles(node)
        } else { Vec::new() };
        let fonts = if property == SelectionMatch::Font {
            characters.iter().map(|style| (style.family, style.weight, style.italic)).collect()
        } else { BTreeSet::new() };
        let mut features: Vec<_> = if node.kind == NodeKind::Text && matches!(property,
            SelectionMatch::Appearance | SelectionMatch::TextProperties) {
            node.font_features.iter().map(|feature| (feature.tag.as_str(), feature.value)).collect()
        } else { Vec::new() };
        features.sort_unstable();
        let paragraphs = if node.kind == NodeKind::Text && matches!(property,
            SelectionMatch::Appearance | SelectionMatch::TextProperties) {
            paragraph_styles(node)
        } else { Vec::new() };
        Self { node, property, characters, paragraphs, fonts, features }
    }

    fn text_box(&self) -> (u8, u8) {
        let n = self.node;
        (n.text_vertical_align as u8, n.text_auto_resize as u8)
    }

    fn same_text(&self, other: &Self) -> bool {
        self.text_box() == other.text_box() && self.paragraphs == other.paragraphs && self.characters == other.characters
            && self.features == other.features
    }

    fn matches(&self, other: &Self) -> bool {
        let (a, b) = (self.node, other.node);
        match self.property {
            SelectionMatch::Fill => a.fills == b.fills,
            SelectionMatch::Stroke => a.strokes == b.strokes,
            SelectionMatch::Effect => a.effects == b.effects,
            SelectionMatch::Font => self.fonts == other.fonts,
            SelectionMatch::TextProperties => self.same_text(other),
            SelectionMatch::Instance => a.component_id == b.component_id,
            SelectionMatch::Appearance => a.kind == b.kind && a.fills == b.fills
                && a.strokes == b.strokes && a.effects == b.effects
                && a.opacity == b.opacity && a.blend_mode == b.blend_mode
                && a.corner_smoothing == b.corner_smoothing
                && a.corner_radii.unwrap_or([a.corner_radius; 4]) == b.corner_radii.unwrap_or([b.corner_radius; 4])
                && (a.kind != NodeKind::Text || self.same_text(other)),
        }
    }

    fn fingerprint(&self, random: &RandomState) -> u64 {
        let mut hash = random.build_hasher();
        let n = self.node;
        let appearance = self.property == SelectionMatch::Appearance;
        if appearance || self.property == SelectionMatch::Fill {
            n.fills.len().hash(&mut hash);
            for fill in &n.fills {
                hash_paint(&fill.paint, &mut hash);
                number(fill.opacity).hash(&mut hash);
                fill.visible.hash(&mut hash);
            }
        }
        if appearance || self.property == SelectionMatch::Stroke {
            n.strokes.len().hash(&mut hash);
            for stroke in &n.strokes {
                hash_paint(&stroke.paint, &mut hash);
                (number(stroke.width), number(stroke.opacity), stroke.visible,
                    stroke.align as u8, stroke.cap as u8, stroke.join as u8).hash(&mut hash);
                stroke.dash.len().hash(&mut hash);
                for &dash in &stroke.dash { number(dash).hash(&mut hash); }
                std::mem::discriminant(&stroke.width_profile).hash(&mut hash);
                if let crate::StrokeWidthProfile::Custom { points } = &stroke.width_profile {
                    points.len().hash(&mut hash);
                    for point in points { (number(point.position), number(point.width)).hash(&mut hash); }
                }
            }
        }
        if appearance || self.property == SelectionMatch::Effect {
            n.effects.len().hash(&mut hash);
            for effect in &n.effects {
                (effect.kind as u8, color(effect.color), number(effect.offset.x), number(effect.offset.y),
                    number(effect.radius), number(effect.spread), effect.visible).hash(&mut hash);
            }
        }
        if (appearance && n.kind == NodeKind::Text) || self.property == SelectionMatch::TextProperties {
            self.text_box().hash(&mut hash);
            self.paragraphs.hash(&mut hash);
            self.characters.hash(&mut hash);
            self.features.hash(&mut hash);
        }
        if self.property == SelectionMatch::Font { self.fonts.hash(&mut hash); }
        if self.property == SelectionMatch::Instance { n.component_id.hash(&mut hash); }
        if appearance {
            (n.kind as u8, number(n.opacity), n.blend_mode as u8).hash(&mut hash);
            n.corner_radii.unwrap_or([n.corner_radius; 4]).map(number).hash(&mut hash);
            number(n.corner_smoothing).hash(&mut hash);
        }
        hash.finish()
    }
}

fn hash_paint(paint: &Paint, hash: &mut impl Hasher) {
    std::mem::discriminant(paint).hash(hash);
    let stops = match paint {
        Paint::Solid(value) => { color(*value).hash(hash); return; }
        Paint::LinearGradient { start, end, stops } => {
            (number(start.x), number(start.y), number(end.x), number(end.y)).hash(hash);
            stops
        }
        Paint::RadialGradient { center, radius, stops } => {
            (number(center.x), number(center.y), number(*radius)).hash(hash);
            stops
        }
        Paint::AngularGradient { center, angle, stops } => {
            (number(center.x), number(center.y), number(*angle)).hash(hash);
            stops
        }
        Paint::Image { href, scale, crop } => {
            href.hash(hash);
            (*scale as u8).hash(hash);
            crop.map(|r| [r.x, r.y, r.width, r.height].map(number)).hash(hash);
            return;
        }
    };
    stops.len().hash(hash);
    for stop in stops { (number(stop.offset), color(stop.color)).hash(hash); }
}

#[cfg(test)]
mod tests;
