//! The font matching algorithm of CSS Fonts Level 4, section 5.2, over declared faces.
//!
//! It runs on descriptors alone, so a face is chosen without its bytes: a font is fetched only
//! once it is known to be needed.

use crate::css::{FontFaceDescriptors, FontStyleDescriptor};

/// The style a text asks for.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum FontStyleRequest {
    #[default]
    Normal,
    Italic,
    /// An angle in degrees, from -90 to 90.
    Oblique(f32),
}

/// What a text asks of a family.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FontMatchRequest {
    /// From 1 to 1000.
    pub weight: f32,
    /// A percentage of the normal width.
    pub width: f32,
    pub style: FontStyleRequest,
}

impl Default for FontMatchRequest {
    fn default() -> Self {
        Self { weight: 400.0, width: 100.0, style: FontStyleRequest::Normal }
    }
}

/// The angle where the standard separates a slight oblique from one that can stand for italic.
const ITALIC_ANGLE: f32 = 11.0;
/// The angle of `left` and `right`, which the standard names without an angle: the angle of
/// plain `oblique`, to the left or to the right.
const DIRECTION_ANGLE: f32 = FontStyleDescriptor::DEFAULT_OBLIQUE_ANGLE;
/// Stops a search just before zero, which the standard searches in a later step.
const ABOVE_ZERO: f32 = f32::MIN_POSITIVE;

#[derive(Clone, Copy, PartialEq)]
enum Axis {
    Italic,
    Oblique,
}

#[derive(Clone, Copy)]
enum Search {
    /// The smallest value from `from` up to `to`.
    Up { from: f32, to: f32 },
    /// The largest value from `from` down to `to`.
    Down { from: f32, to: f32 },
}

impl Search {
    const fn up(from: f32) -> Self {
        Self::Up { from, to: f32::INFINITY }
    }

    const fn down(from: f32) -> Self {
        Self::Down { from, to: f32::NEG_INFINITY }
    }

    /// The same search on the other side of zero.
    fn mirrored(self) -> Self {
        match self {
            Self::Up { from, to } => Self::Down { from: -from, to: -to },
            Self::Down { from, to } => Self::Up { from: -from, to: -to },
        }
    }

    /// The value this search finds in one inclusive range.
    fn hit(self, start: f32, end: f32) -> Option<f32> {
        match self {
            Self::Up { from, to } => {
                let value = start.max(from);
                (value <= end && value <= to).then_some(value)
            }
            Self::Down { from, to } => {
                let value = end.min(from);
                (value >= start && value >= to).then_some(value)
            }
        }
    }

    /// Whether `a` is a better result than `b`.
    fn better(self, a: f32, b: f32) -> bool {
        match self {
            Self::Up { .. } => a < b,
            Self::Down { .. } => a > b,
        }
    }
}

/// The best value of a search over the ranges of the candidates.
fn best(candidates: &[usize], range: impl Fn(usize) -> Option<(f32, f32)>, search: Search) -> Option<f32> {
    let mut found: Option<f32> = None;
    for &candidate in candidates {
        if let Some((start, end)) = range(candidate) {
            if let Some(value) = search.hit(start, end) {
                if found.is_none_or(|current| search.better(value, current)) {
                    found = Some(value);
                }
            }
        }
    }
    found
}

fn weight_range(face: &FontFaceDescriptors) -> (f32, f32) {
    face.weight.map_or((400.0, 400.0), |range| (range.start, range.end))
}

fn width_range(face: &FontFaceDescriptors) -> (f32, f32) {
    face.width.map_or((100.0, 100.0), |range| (range.start, range.end))
}

/// The range of a face on one of the two axes of style. A normal face is the value 0 on both.
fn style_range(face: &FontFaceDescriptors, axis: Axis) -> Option<(f32, f32)> {
    match (face.style, axis) {
        (FontStyleDescriptor::Auto | FontStyleDescriptor::Normal, _) => Some((0.0, 0.0)),
        (FontStyleDescriptor::Italic, Axis::Italic) => Some((1.0, 1.0)),
        (FontStyleDescriptor::Oblique { start, end }, Axis::Oblique) => Some((start, end)),
        (FontStyleDescriptor::Right, Axis::Oblique) => Some((DIRECTION_ANGLE, DIRECTION_ANGLE)),
        (FontStyleDescriptor::Left, Axis::Oblique) => Some((-DIRECTION_ANGLE, -DIRECTION_ANGLE)),
        _ => None,
    }
}

/// The searches for a style, in the order of the standard.
fn style_searches(style: FontStyleRequest) -> Vec<(Axis, Search)> {
    use Axis::{Italic, Oblique};
    let until_zero = |from: f32| Search::Down { from, to: ABOVE_ZERO };
    match style {
        FontStyleRequest::Normal => vec![
            (Oblique, Search::up(0.0)),
            (Italic, Search::up(0.0)),
            (Oblique, Search::down(0.0)),
            (Italic, Search::down(0.0)),
        ],
        FontStyleRequest::Italic => vec![
            (Italic, Search::up(1.0)),
            (Italic, until_zero(1.0)),
            (Oblique, Search::up(ITALIC_ANGLE)),
            (Oblique, until_zero(ITALIC_ANGLE)),
            (Italic, Search::down(0.0)),
            (Oblique, Search::down(0.0)),
        ],
        FontStyleRequest::Oblique(angle) => {
            let positive = angle.abs();
            let searches = if positive >= ITALIC_ANGLE {
                vec![
                    (Oblique, Search::up(positive)),
                    (Oblique, until_zero(positive)),
                    (Italic, Search::up(1.0)),
                    (Italic, until_zero(1.0)),
                    (Oblique, Search::down(0.0)),
                    (Italic, Search::down(0.0)),
                ]
            } else {
                vec![
                    (Oblique, Search::Up { from: positive, to: positive }),
                    (Oblique, Search::Down { from: positive, to: if positive > 0.0 { ABOVE_ZERO } else { 0.0 } }),
                    (Oblique, Search::up(positive)),
                    (Italic, until_zero(1.0)),
                    (Italic, Search::up(1.0)),
                    (Oblique, Search::down(0.0)),
                    (Italic, Search::down(0.0)),
                ]
            };
            if angle < 0.0 {
                searches.into_iter().map(|(axis, search)| (axis, search.mirrored())).collect()
            } else {
                searches
            }
        }
    }
}

/// Keeps the faces that hold the value the first search to find one found.
fn narrow(candidates: &mut Vec<usize>, searches: &[Search], range: impl Fn(usize) -> Option<(f32, f32)>) {
    for search in searches {
        if let Some(value) = best(candidates, &range, *search) {
            candidates.retain(|candidate| range(*candidate).is_some_and(|(start, end)| start <= value && value <= end));
            return;
        }
    }
}

fn narrow_width(faces: &[&FontFaceDescriptors], candidates: &mut Vec<usize>, width: f32) {
    let range = |index: usize| Some(width_range(faces[index]));
    let exact = Search::Up { from: width, to: width };
    if width <= 100.0 {
        narrow(candidates, &[exact, Search::down(width), Search::up(width)], range);
    } else {
        narrow(candidates, &[exact, Search::up(width), Search::down(width)], range);
    }
}

fn narrow_style(faces: &[&FontFaceDescriptors], candidates: &mut Vec<usize>, style: FontStyleRequest) {
    for (axis, search) in style_searches(style) {
        let range = |index: usize| style_range(faces[index], axis);
        if let Some(value) = best(candidates, range, search) {
            candidates.retain(|candidate| range(*candidate).is_some_and(|(start, end)| start <= value && value <= end));
            return;
        }
    }
}

fn narrow_weight(faces: &[&FontFaceDescriptors], candidates: &mut Vec<usize>, weight: f32) {
    let range = |index: usize| Some(weight_range(faces[index]));
    if (400.0..=500.0).contains(&weight) {
        let searches = [Search::Up { from: weight, to: 500.0 }, Search::down(weight), Search::up(500.0)];
        narrow(candidates, &searches, range);
    } else if weight < 400.0 {
        narrow(candidates, &[Search::down(weight), Search::up(weight)], range);
    } else {
        narrow(candidates, &[Search::up(weight), Search::down(weight)], range);
    }
}

/// Whether two faces have the same width, style and weight descriptors: rules that differ only
/// by `unicode-range` make up one composite face.
fn same_selection(a: &FontFaceDescriptors, b: &FontFaceDescriptors) -> bool {
    a.style == b.style && a.weight == b.weight && a.width == b.width
}

/// The composite face a request selects among the faces of one family: indices into `faces`,
/// the last defined first, which is the order `unicode-range` is searched in. Empty only when
/// `faces` is.
pub fn match_font_faces(faces: &[&FontFaceDescriptors], request: &FontMatchRequest) -> Vec<usize> {
    let mut candidates: Vec<usize> = (0..faces.len()).collect();
    narrow_width(faces, &mut candidates, request.width);
    narrow_style(faces, &mut candidates, request.style);
    narrow_weight(faces, &mut candidates, request.weight);
    let Some(&last) = candidates.last() else {
        return Vec::new();
    };
    candidates.retain(|candidate| same_selection(faces[*candidate], faces[last]));
    candidates.reverse();
    candidates
}

/// The first face of a composite whose `unicode-range` includes a character. Whether the font
/// really has a glyph for it is known only after it is loaded.
pub fn face_for_character(faces: &[&FontFaceDescriptors], composite: &[usize], code_point: u32) -> Option<usize> {
    composite.iter().copied().find(|index| faces[*index].unicode_range.contains(code_point))
}

/// The faces of a composite a text needs, in the order they are searched: each face whose
/// `unicode-range` includes a character of the text that no face before it covers.
pub fn faces_for_text(faces: &[&FontFaceDescriptors], composite: &[usize], text: &str) -> Vec<usize> {
    let mut needed = Vec::new();
    for character in text.chars() {
        if let Some(face) = face_for_character(faces, composite, character as u32) {
            if !needed.contains(&face) {
                needed.push(face);
            }
        }
    }
    needed.sort_by_key(|face| composite.iter().position(|index| index == face));
    needed
}

/// Whether a family name matches: family names are compared ignoring ASCII case.
pub fn family_matches(face: &FontFaceDescriptors, family: &str) -> bool {
    face.family.eq_ignore_ascii_case(family)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::{parse_font_face_block, UnicodeRangeSet};

    fn face(descriptors: &str) -> FontFaceDescriptors {
        parse_font_face_block(&format!("font-family: F; src: url(a); {descriptors}")).unwrap()
    }

    fn pick(declared: &[&str], request: FontMatchRequest) -> Vec<usize> {
        let faces: Vec<FontFaceDescriptors> = declared.iter().map(|text| face(text)).collect();
        let references: Vec<&FontFaceDescriptors> = faces.iter().collect();
        match_font_faces(&references, &request)
    }

    fn weight(declared: &[&str], value: f32) -> usize {
        pick(declared, FontMatchRequest { weight: value, ..Default::default() })[0]
    }

    fn width(declared: &[&str], value: f32) -> usize {
        pick(declared, FontMatchRequest { width: value, ..Default::default() })[0]
    }

    fn style(declared: &[&str], value: FontStyleRequest) -> usize {
        pick(declared, FontMatchRequest { style: value, ..Default::default() })[0]
    }

    const WEIGHTS: [&str; 5] =
        ["font-weight: 100", "font-weight: 300", "font-weight: 500", "font-weight: 700", "font-weight: 900"];

    #[test]
    fn weight_below_400_looks_lighter_first() {
        assert_eq!(weight(&WEIGHTS, 300.0), 1);
        assert_eq!(weight(&WEIGHTS, 200.0), 0);
        assert_eq!(weight(&WEIGHTS[2..], 200.0), 0, "then heavier");
    }

    #[test]
    fn weight_from_400_to_500_looks_up_to_500_first() {
        assert_eq!(weight(&WEIGHTS, 400.0), 2, "500 before 300");
        assert_eq!(weight(&WEIGHTS, 450.0), 2);
        let without_500 = ["font-weight: 300", "font-weight: 700"];
        assert_eq!(weight(&without_500, 400.0), 0, "then lighter, before anything above 500");
        assert_eq!(weight(&["font-weight: 600", "font-weight: 900"], 450.0), 0, "then above 500, ascending");
    }

    #[test]
    fn weight_above_500_looks_heavier_first() {
        assert_eq!(weight(&WEIGHTS, 600.0), 3);
        assert_eq!(weight(&WEIGHTS, 950.0), 4, "then lighter");
    }

    #[test]
    fn a_range_holds_the_weight() {
        assert_eq!(weight(&["font-weight: 100 300", "font-weight: 400 800"], 650.0), 1);
        assert_eq!(weight(&["font-weight: 100 300", "font-weight: 500 800"], 400.0), 1, "500 is in the range");
    }

    #[test]
    fn width_prefers_its_own_side() {
        let widths = ["font-width: 50%", "font-width: 75%", "font-width: 100%", "font-width: 150%"];
        assert_eq!(width(&widths, 75.0), 1);
        assert_eq!(width(&widths, 90.0), 1, "narrower first at or below 100%");
        assert_eq!(width(&widths, 120.0), 3, "wider first above 100%");
        assert_eq!(width(&widths[1..], 60.0), 0, "then wider");
        assert_eq!(width(&widths[..3], 120.0), 2, "then narrower");
        assert_eq!(width(&["font-width: 75% 125%", "font-width: 150%"], 110.0), 0);
    }

    #[test]
    fn width_comes_before_weight() {
        let faces = ["font-width: 75%; font-weight: 400", "font-width: 100%; font-weight: 900"];
        let request = FontMatchRequest { weight: 400.0, width: 100.0, ..Default::default() };
        assert_eq!(pick(&faces, request), vec![1]);
    }

    #[test]
    fn normal_style() {
        use FontStyleRequest::Normal;
        assert_eq!(style(&["font-style: italic", "font-style: normal"], Normal), 1);
        assert_eq!(style(&["font-style: italic", "font-style: oblique 20deg"], Normal), 1, "oblique before italic");
        assert_eq!(style(&["font-style: oblique -20deg", "font-style: italic"], Normal), 1, "italic before negative");
        assert_eq!(style(&["font-style: oblique -30deg", "font-style: oblique -10deg"], Normal), 1);
    }

    #[test]
    fn italic_style() {
        use FontStyleRequest::Italic;
        assert_eq!(style(&["font-style: normal", "font-style: italic"], Italic), 1);
        assert_eq!(style(&["font-style: normal", "font-style: oblique 20deg"], Italic), 1, "oblique before normal");
        assert_eq!(
            style(&["font-style: oblique 5deg", "font-style: oblique 20deg"], Italic),
            1,
            "11deg and above before the slight obliques"
        );
        assert_eq!(style(&["font-style: oblique -20deg", "font-style: oblique 5deg"], Italic), 1);
        assert_eq!(style(&["font-style: oblique -20deg", "font-style: normal"], Italic), 1, "zero before negative");
    }

    #[test]
    fn oblique_from_11_degrees() {
        let request = FontStyleRequest::Oblique(20.0);
        assert_eq!(style(&["font-style: oblique 15deg", "font-style: oblique 25deg"], request), 1, "above first");
        assert_eq!(style(&["font-style: italic", "font-style: oblique 15deg"], request), 1, "then below");
        assert_eq!(style(&["font-style: normal", "font-style: italic"], request), 1, "then italic");
        assert_eq!(style(&["font-style: oblique -5deg", "font-style: normal"], request), 1, "then zero");
        assert_eq!(style(&["font-style: oblique 10deg 30deg", "font-style: oblique 40deg"], request), 0);
    }

    #[test]
    fn slight_oblique() {
        let request = FontStyleRequest::Oblique(8.0);
        assert_eq!(style(&["font-style: oblique 20deg", "font-style: oblique 8deg"], request), 1, "exact");
        assert_eq!(style(&["font-style: oblique 20deg", "font-style: oblique 3deg"], request), 1, "below first");
        assert_eq!(style(&["font-style: italic", "font-style: oblique 20deg"], request), 1, "then above");
        assert_eq!(style(&["font-style: normal", "font-style: italic"], request), 1, "then italic");
        let upright = FontStyleRequest::Oblique(0.0);
        assert_eq!(style(&["font-style: oblique 5deg", "font-style: normal"], upright), 1);
    }

    #[test]
    fn negative_oblique_is_the_mirror() {
        let strong = FontStyleRequest::Oblique(-20.0);
        assert_eq!(style(&["font-style: oblique -15deg", "font-style: oblique -25deg"], strong), 1);
        assert_eq!(style(&["font-style: oblique 20deg", "font-style: oblique -15deg"], strong), 1);
        let slight = FontStyleRequest::Oblique(-8.0);
        assert_eq!(style(&["font-style: oblique -20deg", "font-style: oblique -3deg"], slight), 1, "toward zero first");
    }

    #[test]
    fn the_keywords_of_direction() {
        assert_eq!(style(&["font-style: left", "font-style: right"], FontStyleRequest::Oblique(14.0)), 1);
        assert_eq!(style(&["font-style: right", "font-style: left"], FontStyleRequest::Oblique(-14.0)), 1);
    }

    #[test]
    fn auto_selects_as_normal() {
        assert_eq!(style(&["font-style: italic", "font-style: auto"], FontStyleRequest::Normal), 1);
        assert_eq!(weight(&["font-weight: 900", "font-weight: auto"], 400.0), 1);
        assert_eq!(width(&["font-width: 50%", "font-width: auto"], 100.0), 1);
    }

    fn composite() -> Vec<FontFaceDescriptors> {
        vec![
            face("unicode-range: U+0-FF"),
            face("unicode-range: U+100-1FF"),
            face("font-weight: 700; unicode-range: U+0-FF"),
            face("unicode-range: U+41"),
        ]
    }

    #[test]
    fn a_composite_is_searched_from_the_last_rule() {
        let faces = composite();
        let references: Vec<&FontFaceDescriptors> = faces.iter().collect();
        let matched = match_font_faces(&references, &FontMatchRequest::default());
        assert_eq!(matched, vec![3, 1, 0], "the bold face is not part of it");
        assert_eq!(face_for_character(&references, &matched, 0x41), Some(3));
        assert_eq!(face_for_character(&references, &matched, 0x42), Some(0));
        assert_eq!(face_for_character(&references, &matched, 0x100), Some(1));
        assert_eq!(face_for_character(&references, &matched, 0x3042), None);
    }

    #[test]
    fn a_text_needs_only_the_faces_that_cover_it() {
        let faces = composite();
        let references: Vec<&FontFaceDescriptors> = faces.iter().collect();
        let matched = match_font_faces(&references, &FontMatchRequest::default());
        assert_eq!(faces_for_text(&references, &matched, "A"), vec![3]);
        assert_eq!(faces_for_text(&references, &matched, "BAB"), vec![3, 0]);
        assert_eq!(faces_for_text(&references, &matched, "\u{100}B"), vec![1, 0]);
        assert!(faces_for_text(&references, &matched, "\u{3042}").is_empty());
    }

    #[test]
    fn no_faces_no_match() {
        assert!(match_font_faces(&[], &FontMatchRequest::default()).is_empty());
    }

    #[test]
    fn family_names_ignore_case() {
        assert!(family_matches(&face(""), "f"));
        assert!(!family_matches(&face(""), "G"));
        assert_eq!(face("").unicode_range, UnicodeRangeSet::default());
    }
}
