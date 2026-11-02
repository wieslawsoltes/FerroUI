//! The selection of a rich text view drawn over its lines (#439): the highlight behind the text,
//! the selected glyphs and decorations with the selection foreground, the run backgrounds left
//! out under the highlight, and the runs formatted without the selection.

use crate::documents::{Paragraph, Run};
use crate::test_support::test_scope;
use crate::RichTextBlock;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{
    BoxShadows, Brushes, Color, Colors, DrawingContext, FlowDirection, Geometry, GlyphRun, IBrush,
    IDrawingContextCore, IEffect, IPen, RenderOptions, TextDecorations, TextOptions,
};
use ferroui_base::platform::IGeometryImpl;
use ferroui_base::rendering::scene_graph::ICustomDrawOperation;
use ferroui_base::{Matrix, Point, Rect, Ref, RoundedRect, Size, Vector};
use std::rc::Rc;
use std::sync::Arc;

/// A glyph run or a line a view drew, in the coordinates of the view.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Painted {
    pub color: Option<Color>,
    /// The bounds of the glyph run, or the extent of the line.
    pub extent: Rect,
    /// The clip in effect, if any.
    pub clip: Option<Rect>,
    /// The characters of a glyph run; empty for a line.
    pub text: String,
}

impl Painted {
    /// The horizontal extent that shows through the clip.
    pub fn visible(&self) -> (f64, f64) {
        let mut start = self.extent.x;
        let mut end = self.extent.right();
        if let Some(clip) = self.clip {
            start = start.max(clip.x);
            end = end.min(clip.right());
        }
        (start, end.max(start))
    }
}

/// What a view drew, in order.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Drawn {
    Fill { color: Option<Color>, rect: Rect },
    Glyphs(Painted),
    Line(Painted),
}

pub(crate) fn color(brush: Option<&Rc<dyn IBrush>>) -> Option<Color> {
    brush
        .and_then(|brush| brush.as_solid_color_brush())
        .map(|brush| brush.color())
}

/// A drawing context core that records fills, glyph runs and lines with their brushes, the
/// translations and the rectangle clips in effect.
#[derive(Default)]
struct Recorder {
    drawn: Vec<Drawn>,
    offsets: Vec<Vector>,
    clips: Vec<Option<Rect>>,
}

impl Recorder {
    fn offset(&self) -> Vector {
        self.offsets.last().copied().unwrap_or_default()
    }

    fn clip(&self) -> Option<Rect> {
        self.clips.last().copied().flatten()
    }

    fn push_clip(&mut self, rect: Rect) {
        let rect = rect.translate(self.offset());
        let clip = match self.clip() {
            Some(current) => current.intersect(rect),
            None => rect,
        };
        self.clips.push(Some(clip));
    }
}

impl IDrawingContextCore for Recorder {
    fn draw_line_core(&mut self, pen: &Rc<dyn IPen>, p1: Point, p2: Point) {
        let offset = self.offset();
        let extent = Rect::new(
            p1.x.min(p2.x) + offset.x,
            p1.y.min(p2.y) + offset.y,
            (p1.x - p2.x).abs(),
            (p1.y - p2.y).abs(),
        );
        let clip = self.clip();
        self.drawn.push(Drawn::Line(Painted {
            color: color(pen.brush().as_ref()),
            extent,
            clip,
            text: String::new(),
        }));
    }

    fn draw_geometry_impl_core(
        &mut self,
        _brush: Option<&Rc<dyn IBrush>>,
        _pen: Option<&Rc<dyn IPen>>,
        _geometry: &Arc<dyn IGeometryImpl>,
    ) {
    }

    fn draw_rectangle_core(
        &mut self,
        brush: Option<&Rc<dyn IBrush>>,
        _pen: Option<&Rc<dyn IPen>>,
        rrect: RoundedRect,
        _box_shadows: &BoxShadows,
    ) {
        if brush.is_some() {
            let rect = rrect.rect.translate(self.offset());
            self.drawn.push(Drawn::Fill {
                color: color(brush),
                rect,
            });
        }
    }

    fn draw_ellipse_core(&mut self, _brush: Option<&Rc<dyn IBrush>>, _pen: Option<&Rc<dyn IPen>>, _rect: Rect) {}
    fn draw_bitmap(&mut self, _source: &ferroui_base::utilities::RefCounted<ferroui_base::platform::SharedBitmapImpl>, _opacity: f64, _source_rect: Rect, _dest_rect: Rect) {}
    fn custom(&mut self, _custom: &std::sync::Arc<dyn ICustomDrawOperation>) {}

    fn draw_glyph_run(&mut self, foreground: Option<&Rc<dyn IBrush>>, glyph_run: &Rc<GlyphRun>) {
        let extent = glyph_run.bounds().translate(self.offset());
        let clip = self.clip();
        self.drawn.push(Drawn::Glyphs(Painted {
            color: color(foreground),
            extent,
            clip,
            text: glyph_run.characters().to_string_lossy(),
        }));
    }

    fn push_clip_core(&mut self, rect: Rect) {
        self.push_clip(rect);
    }
    fn push_rounded_clip_core(&mut self, rect: RoundedRect) {
        self.push_clip(rect.rect);
    }
    fn push_geometry_clip_core(&mut self, _clip: &Ref<Geometry>) {
        let clip = self.clip();
        self.clips.push(clip);
    }
    fn push_opacity_core(&mut self, _opacity: f64) {}
    fn push_opacity_mask_core(&mut self, _mask: &Rc<dyn IBrush>, _bounds: Rect) {}
    fn push_transform_core(&mut self, matrix: Matrix) {
        let offset = self.offset();
        self.offsets
            .push(Vector::new(offset.x + matrix.m31, offset.y + matrix.m32));
    }
    fn push_render_options_core(&mut self, _render_options: RenderOptions) {}
    fn push_text_options_core(&mut self, _text_options: TextOptions) {}
    fn push_effect_core(&mut self, _effect: &Rc<dyn IEffect>, _bounds: Rect) {}
    fn pop_clip_core(&mut self) {
        self.clips.pop();
    }
    fn pop_geometry_clip_core(&mut self) {
        self.clips.pop();
    }
    fn pop_opacity_core(&mut self) {}
    fn pop_opacity_mask_core(&mut self) {}
    fn pop_transform_core(&mut self) {
        self.offsets.pop();
    }
    fn pop_render_options_core(&mut self) {}
    fn pop_text_options_core(&mut self) {}
    fn pop_effect_core(&mut self) {}
    fn dispose_core(&mut self) {}
}

/// Renders a view (not its children) and returns what it drew.
pub(crate) fn render(view: &RichTextBlock) -> Vec<Drawn> {
    let mut core = Recorder::default();
    {
        let mut context = DrawingContext::new(&mut core);
        view.render(&mut context);
    }
    assert!(core.offsets.is_empty() && core.clips.is_empty(), "balanced pushes");
    core.drawn
}

pub(crate) fn fills(drawn: &[Drawn], expected: Color) -> Vec<Rect> {
    drawn
        .iter()
        .filter_map(|drawn| match drawn {
            Drawn::Fill { color, rect } if *color == Some(expected) => Some(*rect),
            _ => None,
        })
        .collect()
}

pub(crate) fn glyphs(drawn: &[Drawn], expected: Color) -> Vec<Painted> {
    drawn
        .iter()
        .filter_map(|drawn| match drawn {
            Drawn::Glyphs(painted) if painted.color == Some(expected) => Some(painted.clone()),
            _ => None,
        })
        .collect()
}

pub(crate) fn lines(drawn: &[Drawn], expected: Color) -> Vec<Painted> {
    drawn
        .iter()
        .filter_map(|drawn| match drawn {
            Drawn::Line(painted) if painted.color == Some(expected) => Some(painted.clone()),
            _ => None,
        })
        .collect()
}

/// The glyph runs and lines drawn, without the fills.
pub(crate) fn text_drawing(drawn: &[Drawn]) -> Vec<Drawn> {
    drawn
        .iter()
        .filter(|drawn| !matches!(drawn, Drawn::Fill { .. }))
        .cloned()
        .collect()
}

const EPSILON: f64 = 1e-6;

pub(crate) fn close(a: f64, b: f64) -> bool {
    (a - b).abs() < EPSILON
}

/// Whether `a` and `b` share more than a point.
pub(crate) fn overlaps(a: (f64, f64), b: (f64, f64)) -> bool {
    a.0 < b.1 - EPSILON && b.0 < a.1 - EPSILON
}

/// Whether `inner` lies within `outer`.
pub(crate) fn within(inner: (f64, f64), outer: (f64, f64)) -> bool {
    inner.0 >= outer.0 - EPSILON && inner.1 <= outer.1 + EPSILON
}

fn measure(view: &RichTextBlock, width: f64, height: f64) {
    view.measure(Size::new(width, height));
    view.arrange(Rect::new(0.0, 0.0, width, height));
}

/// A view of one paragraph with one green, underlined run on an aqua background.
fn decorated(text: &str) -> Ref<RichTextBlock> {
    let view = RichTextBlock::new();
    let paragraph = Paragraph::new();
    let run = Run::with_text(Some(text));
    run.set_foreground(Some(Brushes::green()));
    run.set_background(Some(Brushes::aqua()));
    run.set_text_decorations(Some(TextDecorations::underline()));
    paragraph.add_child(run);
    view.blocks().add(paragraph);
    view.set_selection_highlight_color(Some(Brushes::red()));
    view.set_selection_foreground_brush(Some(Brushes::white()));
    measure(&view, 400.0, 100.0);
    view
}

/// The selected glyphs show the selection foreground exactly over `spans`, and the glyph runs
/// in `own` show everywhere else; so do the decorations.
fn assert_selected_text(drawn: &[Drawn], spans: &[(f64, f64)], own: Color) {
    let selected = glyphs(drawn, Colors::WHITE);
    assert!(!selected.is_empty(), "{drawn:#?}");
    let mut visible: Vec<_> = selected.iter().map(Painted::visible).collect();
    visible.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut expected = spans.to_vec();
    expected.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert_eq!(visible.len(), expected.len(), "{drawn:#?}");
    for (visible, expected) in visible.iter().zip(&expected) {
        assert!(
            close(visible.0, expected.0) && close(visible.1, expected.1),
            "selected glyphs {visible:?}, selection {expected:?}"
        );
    }
    for painted in glyphs(drawn, own) {
        let visible = painted.visible();
        assert!(
            spans.iter().all(|span| !overlaps(visible, *span)),
            "unselected glyphs {visible:?} under the selection {spans:?}"
        );
    }
    for painted in lines(drawn, Colors::WHITE) {
        let visible = painted.visible();
        assert!(
            spans.iter().any(|span| within(visible, *span)),
            "a selected decoration {visible:?} outside the selection {spans:?}"
        );
    }
    for painted in lines(drawn, own) {
        let visible = painted.visible();
        assert!(
            spans.iter().all(|span| !overlaps(visible, *span)),
            "an unselected decoration {visible:?} under the selection {spans:?}"
        );
    }
}

#[test]
fn the_selected_text_is_drawn_over_the_highlight_with_the_selection_foreground() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = decorated("abcdef");
    let source = view.page_layout().source.clone();
    view.select(1, 4);
    assert!(Rc::ptr_eq(&source, &view.page_layout().source));
    let selected = view.get_rects_for_range(1..4);
    assert_eq!(selected.len(), 1);
    let span = (selected[0].x, selected[0].right());
    let drawn = render(&view);

    // The highlight: the hit-test rectangles of the selected range, behind the text.
    assert_eq!(fills(&drawn, Colors::RED), selected);
    let last_highlight = drawn
        .iter()
        .rposition(|drawn| matches!(drawn, Drawn::Fill { color: Some(color), .. } if *color == Colors::RED))
        .unwrap();
    let first_text = drawn
        .iter()
        .position(|drawn| matches!(drawn, Drawn::Glyphs(_)))
        .unwrap();
    assert!(last_highlight < first_text);

    // The run's background leaves out the selection and covers the rest of the run.
    let unselected = view.get_rects_for_range(0..6);
    let run = (unselected[0].x, unselected[0].right());
    let backgrounds = fills(&drawn, Colors::AQUA);
    assert_eq!(backgrounds.len(), 2, "{drawn:#?}");
    assert!(close(backgrounds[0].x, run.0) && close(backgrounds[0].right(), span.0));
    assert!(close(backgrounds[1].x, span.1) && close(backgrounds[1].right(), run.1));

    // The selected glyphs and the selected part of the underline in the selection foreground.
    assert_selected_text(&drawn, &[span], Colors::GREEN);
    assert!(!lines(&drawn, Colors::WHITE).is_empty());
    assert!(!lines(&drawn, Colors::GREEN).is_empty());
    let white = glyphs(&drawn, Colors::WHITE);
    assert!(white[0].text.contains("abcdef"), "one glyph run, not split by the selection");

    // The runs keep the document's format: the selection is not a formatting override.
    use ferroui_base::media::text_formatting::ITextSource;
    let properties = view.page_layout().source.paragraphs[0]
        .source
        .get_text_run(1)
        .unwrap()
        .properties()
        .unwrap()
        .clone();
    assert_eq!(color(properties.foreground_brush()), Some(Colors::GREEN));
    assert_eq!(color(properties.background_brush()), Some(Colors::AQUA));
}

#[test]
fn a_selection_without_a_selection_foreground_draws_the_text_as_it_is() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = decorated("abcdef");
    view.set_selection_foreground_brush(None);
    let unselected = render(&view);
    view.select(1, 4);
    let selected = render(&view);
    let span = {
        let rects = view.get_rects_for_range(1..4);
        (rects[0].x, rects[0].right())
    };
    // The glyphs and decorations are drawn as without a selection.
    assert_eq!(text_drawing(&selected), text_drawing(&unselected));
    // The highlight shows through the run's background.
    assert_eq!(fills(&selected, Colors::RED).len(), 1);
    for rect in fills(&selected, Colors::AQUA) {
        assert!(!overlaps((rect.x, rect.right()), span));
    }

    // Without a highlight brush the run's background stays, and nothing else changes.
    view.set_selection_highlight_color(None);
    assert_eq!(render(&view), unselected);
    // Nor when the selection is not drawn.
    view.set_selection_highlight_color(Some(Brushes::red()));
    view.set_selection_foreground_brush(Some(Brushes::white()));
    view.set_is_text_selection_enabled(false);
    assert_eq!(render(&view), unselected);
}

#[test]
fn a_selection_across_paragraphs_highlights_the_paragraph_marks_and_the_empty_paragraph() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    for text in ["first", "", "third"] {
        let paragraph = Paragraph::new();
        if !text.is_empty() {
            let run = Run::with_text(Some(text));
            run.set_foreground(Some(Brushes::green()));
            paragraph.add_child(run);
        }
        view.blocks().add(paragraph);
    }
    view.set_selection_highlight_color(Some(Brushes::red()));
    view.set_selection_foreground_brush(Some(Brushes::white()));
    measure(&view, 400.0, 200.0);
    // "fi[rst" + mark, the empty paragraph's mark, "th]ird".
    view.select(2, 9);
    let drawn = render(&view);
    let highlight = fills(&drawn, Colors::RED);
    assert_eq!(highlight, view.get_rects_for_range(2..9));
    let metrics = view.line_metrics();
    assert_eq!(metrics.len(), 3);
    let on_line = |index: usize| -> Vec<Rect> {
        highlight
            .iter()
            .copied()
            .filter(|rect| rect.y >= metrics[index].bounds.y - EPSILON && rect.y < metrics[index].bounds.bottom() - EPSILON)
            .collect()
    };
    let marks = |rects: &[Rect]| rects.iter().filter(|rect| close(rect.width, 4.0)).count();
    let first = on_line(0);
    assert_eq!(marks(first.as_slice()), 1, "the mark of the first paragraph");
    let empty = on_line(1);
    assert_eq!(empty.len(), 1);
    assert_eq!(marks(empty.as_slice()), 1, "the mark of the empty paragraph");
    let last = on_line(2);
    assert_eq!(marks(last.as_slice()), 0, "the selection ends inside the last paragraph");

    // The selected glyphs: those of the text rectangles; the marks have no glyphs.
    let spans: Vec<(f64, f64)> = highlight
        .iter()
        .filter(|rect| !close(rect.width, 4.0))
        .map(|rect| (rect.x, rect.right()))
        .collect();
    assert_eq!(spans.len(), 2);
    assert_selected_text(&drawn, &spans, Colors::GREEN);
}

#[test]
fn a_selection_in_a_right_to_left_paragraph_is_drawn_where_its_text_is() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let view = RichTextBlock::new();
    let paragraph = Paragraph::new();
    paragraph.set_flow_direction(FlowDirection::RightToLeft);
    let run = Run::with_text(Some("\u{5D0}\u{5D1}\u{5D2}\u{5D3}\u{5D4}"));
    run.set_foreground(Some(Brushes::green()));
    paragraph.add_child(run);
    view.blocks().add(paragraph);
    view.set_selection_highlight_color(Some(Brushes::red()));
    view.set_selection_foreground_brush(Some(Brushes::white()));
    measure(&view, 400.0, 100.0);
    view.select(1, 3);
    let selected = view.get_rects_for_range(1..3);
    assert!(!selected.is_empty());
    let drawn = render(&view);
    assert_eq!(fills(&drawn, Colors::RED), selected);
    let mut spans: Vec<(f64, f64)> = selected.iter().map(|rect| (rect.x, rect.right())).collect();
    spans.sort_by(|a, b| a.0.total_cmp(&b.0));
    spans.dedup_by(|b, a| {
        if b.0 <= a.1 + EPSILON {
            a.1 = a.1.max(b.1);
            true
        } else {
            false
        }
    });
    assert_selected_text(&drawn, &spans, Colors::GREEN);
    // The first character is the rightmost: the selection is to its left.
    let first = view.get_rect_from_character_index(0);
    assert!(spans.iter().all(|span| span.1 <= first.x + EPSILON), "{spans:?} {first:?}");
}

#[test]
fn an_overflow_view_draws_its_selected_text_with_the_brushes_of_its_source_view() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    crate::register_types();
    let view = RichTextBlock::new();
    for text in ["first", "second"] {
        let paragraph = Paragraph::new();
        let run = Run::with_text(Some(text));
        run.set_foreground(Some(Brushes::green()));
        paragraph.add_child(run);
        view.blocks().add(paragraph);
    }
    let overflow = crate::RichTextBlockOverflow::new();
    view.set_overflow_content_target(Some(overflow.clone()));
    view.set_max_lines(1);
    view.set_selection_highlight_color(Some(Brushes::red()));
    view.set_selection_foreground_brush(Some(Brushes::white()));
    overflow.set_selection_highlight_color(Some(Brushes::red()));
    measure(&view, 400.0, 100.0);
    measure(&overflow, 400.0, 100.0);
    let source = view.page_layout().source.clone();
    // "fir[st", mark, "sec]ond".
    view.select(3, 9);
    assert!(Rc::ptr_eq(&source, &overflow.page_layout().source));
    let drawn = render(&overflow);
    let highlight = fills(&drawn, Colors::RED);
    assert_eq!(highlight, overflow.get_rects_for_range(3..9));
    let spans: Vec<(f64, f64)> = highlight.iter().map(|rect| (rect.x, rect.right())).collect();
    assert_selected_text(&drawn, &spans, Colors::GREEN);
    // The source view's selection foreground decides; without it the text is drawn as it is.
    let selected = drawn;
    view.set_selection_foreground_brush(None);
    let plain = render(&overflow);
    assert!(glyphs(&plain, Colors::WHITE).is_empty());
    assert_ne!(selected, plain);
}
