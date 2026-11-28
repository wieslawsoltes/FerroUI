//! The lines of a data label of a largest width (`MaxWidth` of the source's `LabelGeometry`,
//! which the series set from `DataLabelsMaxWidth`): the text broken into lines as the pinned
//! `DrawingTextExtensions` (`Tokenize` and `BlobArray.Measure`) does, and drawn as one text of
//! the scene per line.

use crate::scene::{Geometry, PaintStyle, PointId, Primitive, Scene, TextAnchor};
use crate::series::DATA_LABELS_PADDING;
use ferroui_base::{Point, Size};

/// The text of a label broken into lines, with the size of the block of its lines (without the
/// label padding) and the height of a line.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LabelLines {
    pub(crate) lines: Vec<String>,
    pub(crate) size: Size,
    pub(crate) line_height: f64,
}

/// The words of a text (`Tokenize`): a word ends after a white space, which stays with it, and a
/// line feed is a word of its own; a carriage return is dropped.
fn tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut word = String::new();
    for c in text.chars() {
        if c == '\r' {
            continue;
        }
        if c == '\n' {
            if !word.is_empty() {
                tokens.push(std::mem::take(&mut word));
            }
            tokens.push("\n".to_owned());
            continue;
        }
        word.push(c);
        if c.is_whitespace() {
            tokens.push(std::mem::take(&mut word));
        }
    }
    if !word.is_empty() {
        tokens.push(word);
    }
    tokens
}

impl LabelLines {
    /// Breaks `text` into lines of at most `max_width` (`BlobArray.Measure`): a word that would
    /// pass the width, and a line feed, start a new line. As in the source, a first word wider
    /// than the width leaves an empty line before it, and a word wider than the width alone
    /// still takes its line.
    pub(crate) fn wrap(
        measure: &mut impl FnMut(&str, f64) -> Size,
        text: &str,
        size: f64,
        max_width: f64,
    ) -> Self {
        let mut lines = vec![String::new()];
        let mut x = 0.0_f64;
        let mut width = 0.0_f64;
        let mut line_height = 0.0_f64;
        for token in tokens(text) {
            let newline = token == "\n";
            let w = if newline {
                0.0
            } else {
                let measured = measure(&token, size);
                line_height = line_height.max(finite(measured.height));
                finite(measured.width)
            };
            if x + w > max_width || newline {
                lines.push(String::new());
                x = 0.0;
            }
            if !newline {
                lines.last_mut().expect("a line").push_str(&token);
                x += w;
                width = width.max(x);
            }
        }
        // A label of one line measures its whole text, as a label without a largest width.
        if lines.len() == 1 {
            let measured = measure(&lines[0], size);
            width = finite(measured.width);
            line_height = finite(measured.height);
        }
        if line_height == 0.0 {
            line_height = finite(measure(text, size).height);
        }
        let height = line_height * lines.len() as f64;
        Self {
            lines,
            size: Size::new(width, height),
            line_height,
        }
    }

    /// The texts of the lines. `origin` is where the text of a label of one line goes (its
    /// anchored side and its top), in a frame turned by `rotation` degrees about it. One line keeps
    /// the anchor; the lines of a longer label start at the left of their block
    /// (`LinesAlignment.Start`), which the anchor places.
    pub(crate) fn texts(
        &self,
        origin: Point,
        anchor: TextAnchor,
        rotation: f64,
        size: f64,
        style: &PaintStyle,
        id: PointId,
    ) -> Vec<Primitive> {
        let text = |origin: Point, text: &str, anchor: TextAnchor| {
            Primitive::new(
                Geometry::Text {
                    origin,
                    text: text.to_owned(),
                    size,
                    rotation,
                    anchor,
                },
                style.clone(),
            )
            .identified(id)
        };
        if self.lines.len() == 1 {
            return vec![text(origin, &self.lines[0], anchor)];
        }
        let left = match anchor {
            TextAnchor::Start => 0.0,
            TextAnchor::Center => -self.size.width * 0.5,
            TextAnchor::End => -self.size.width,
        };
        let (sin, cos) = rotation.to_radians().sin_cos();
        self.lines
            .iter()
            .enumerate()
            .filter(|(_, line)| !line.is_empty())
            .map(|(i, line)| {
                let (dx, dy) = (left, i as f64 * self.line_height);
                let at = Point::new(
                    origin.x + dx * cos - dy * sin,
                    origin.y + dx * sin + dy * cos,
                );
                text(at, line, TextAnchor::Start)
            })
            .collect()
    }
}

fn finite(value: f64) -> f64 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

/// `DataLabelsMaxWidth` of every series of a measured scene (`MaxWidth` of the data labels of
/// the source's series): a data label, a text of a point of series `i`, whose text is wider than
/// `max_widths[i]` less the label padding breaks into lines, one text of the scene each, from the
/// left of their block. A `centred` label keeps its middle where the one line had it; another
/// keeps its last line where the one line was (a label above its point). A series without a
/// width keeps its labels.
pub fn wrap_data_labels(
    scene: &mut Scene,
    max_widths: &[f64],
    mut measure_text: impl FnMut(&str, f64) -> Size,
    centred: impl Fn(PointId) -> bool,
) {
    let primitives = std::mem::take(&mut scene.primitives);
    for primitive in primitives {
        let Some(id) = primitive.id else {
            scene.primitives.push(primitive);
            continue;
        };
        let Geometry::Text {
            origin,
            ref text,
            size,
            rotation,
            anchor,
        } = primitive.geometry
        else {
            scene.primitives.push(primitive);
            continue;
        };
        let Some(&max_width) = max_widths.get(id.series) else {
            scene.primitives.push(primitive);
            continue;
        };
        let lines = LabelLines::wrap(
            &mut measure_text,
            text,
            size,
            max_width - DATA_LABELS_PADDING.0 * 2.0,
        );
        if lines.lines.len() <= 1 {
            scene.primitives.push(primitive);
            continue;
        }
        let extra = lines.line_height * (lines.lines.len() - 1) as f64;
        let shift = if centred(id) { extra * 0.5 } else { extra };
        let (sin, cos) = rotation.to_radians().sin_cos();
        let top = Point::new(origin.x + shift * sin, origin.y - shift * cos);
        let mut texts = lines.texts(top, anchor, rotation, size, &primitive.style, id);
        for text in &mut texts {
            text.clip = primitive.clip;
            text.entity_key = primitive.entity_key;
        }
        scene.primitives.extend(texts);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Text metrics of 0.45 of the text size per character, 1.2 of it per line.
    fn metrics(text: &str, size: f64) -> Size {
        Size::new(text.chars().count() as f64 * size * 0.45, size * 1.2)
    }

    fn wrap(text: &str, max_width: f64) -> LabelLines {
        LabelLines::wrap(&mut metrics, text, 15.0, max_width)
    }

    /// OtherTests/LabelsMeasureTest.MaxWidth: a long text of words in a label of 400 with a
    /// horizontal padding of 10 has lines no wider than the width less the padding; a long word
    /// takes a line wider than the width; a line feed starts a line.
    #[test]
    fn source_labels_measure_max_width() {
        let text =
            "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor \
                    incididunt ut labore et dolore magna aliqua.\r\n\r\nUt enim ad minim veniam, \
                    quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo \
                    consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse.";
        let label = wrap(text, 400.0 - 20.0);
        assert!(label.lines.len() > 3);
        assert!(label
            .lines
            .iter()
            .all(|line| metrics(line, 15.0).width <= 380.0));
        assert!(label.size.width + 20.0 <= 400.0);
        assert!(
            label.lines.iter().any(|line| line.is_empty()),
            "the blank line"
        );
        let long = format!("l{}ng ", "o".repeat(60));
        let label2 = wrap(&format!("short {long}{long}{long}short"), 100.0);
        assert_eq!(label2.lines.len(), 5);
        assert!(label2.size.width > 100.0);
        let label3 = wrap(
            "hello hello hello hello hello \r\n hello hello hello hello hello",
            100.0,
        );
        assert_eq!(label3.lines.len(), 6);
    }

    /// A label that fits keeps one line and its anchor; the lines of a longer one start at the
    /// left of their block, one line height apart, turned with the label.
    #[test]
    fn lines_are_drawn_from_the_left_of_their_block() {
        let style = PaintStyle::default();
        let id = PointId {
            series: 0,
            point: 0,
        };
        let one = wrap("short", 100.0);
        assert_eq!(one.lines, vec!["short".to_owned()]);
        let texts = one.texts(
            Point::new(50.0, 10.0),
            TextAnchor::Center,
            0.0,
            15.0,
            &style,
            id,
        );
        assert!(matches!(
            texts[0].geometry,
            Geometry::Text { origin, anchor: TextAnchor::Center, .. } if origin == Point::new(50.0, 10.0)
        ));
        let two = wrap("hello hello hello", 100.0);
        assert_eq!(two.lines.len(), 2);
        assert_eq!(two.size, Size::new(12.0 * 6.75, 2.0 * 18.0));
        let texts = two.texts(
            Point::new(100.0, 10.0),
            TextAnchor::End,
            90.0,
            15.0,
            &style,
            id,
        );
        let origins: Vec<_> = texts
            .iter()
            .map(|p| match p.geometry {
                Geometry::Text {
                    origin,
                    anchor,
                    rotation,
                    ..
                } => {
                    assert_eq!((anchor, rotation), (TextAnchor::Start, 90.0));
                    origin
                }
                _ => panic!("a text"),
            })
            .collect();
        // Turned a quarter: the block ends at the anchor along the turned text, and the second
        // line is a line height to the left of the first.
        assert!((origins[0].x - 100.0).abs() < 1e-9);
        assert!((origins[0].y - (10.0 - two.size.width)).abs() < 1e-9);
        assert!((origins[1].x - (100.0 - 18.0)).abs() < 1e-9);
    }

    /// `DataLabelsMaxWidth` of a scene: a label of a box series ("Min 1 Q1 2 ...") wider than
    /// 100 less the padding breaks into lines that end where the one line was; a centred label
    /// keeps its middle; a series without a width and a text without a point stay.
    #[test]
    fn data_labels_of_a_scene_break_at_the_width_of_their_series() {
        let label = |series: usize, origin: Point| {
            Primitive::new(
                Geometry::Text {
                    origin,
                    text: "Min 1 Q1 2 Median 3 Q3 4 Max 5".into(),
                    size: 15.0,
                    rotation: 0.0,
                    anchor: TextAnchor::Start,
                },
                PaintStyle::default(),
            )
            .identified(PointId { series, point: 0 })
        };
        let mut scene = Scene::new(
            Size::new(400.0, 400.0),
            ferroui_base::Rect::new(0.0, 0.0, 400.0, 400.0),
        );
        scene.primitives.push(label(0, Point::new(10.0, 200.0)));
        scene.primitives.push(label(1, Point::new(10.0, 300.0)));
        scene.primitives.push(label(2, Point::new(10.0, 100.0)));
        let mut axis = label(0, Point::new(0.0, 0.0));
        axis.id = None;
        scene.primitives.push(axis);
        wrap_data_labels(&mut scene, &[100.0, 100.0], metrics, |id| id.series == 1);
        let lines = |series: usize| -> Vec<Point> {
            scene
                .primitives
                .iter()
                .filter(|p| p.id.is_some_and(|id| id.series == series))
                .map(|p| match p.geometry {
                    Geometry::Text { origin, .. } => origin,
                    _ => panic!("a text"),
                })
                .collect()
        };
        let above = lines(0);
        assert!(above.len() > 1);
        let last = *above.last().unwrap();
        assert!((last.y - 200.0).abs() < 1e-9, "the last line is where the one line was");
        let centred = lines(1);
        let middle = (centred[0].y + centred.last().unwrap().y) * 0.5;
        assert!((middle - 300.0).abs() < 1e-9, "the middle stays");
        assert_eq!(lines(2), vec![Point::new(10.0, 100.0)], "no width, no break");
        assert!(scene.primitives.iter().any(|p| p.id.is_none()));
    }
}
