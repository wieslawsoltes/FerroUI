//! Navigation work and reentrancy measured through the actual formatter service.
use super::*;
use crate::test_support::test_scope;
use crate::RichEditBox;
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::text_formatting::{
    ITextDrawingSink, ITextSource, JustificationProperties, TextBounds, TextCollapsingProperties,
    TextFormatter, TextFormatterImpl, TextLine, TextLineBreak, TextParagraphProperties, TextRun,
};
use ferroui_base::{FerroLocator, Point, Rect, Ref, Size};
use std::any::Any;
use std::cell::{Cell, Ref as CellRef, RefCell};
use std::rc::Rc;

#[derive(Default)]
struct GeometryProbe {
    hits: Cell<usize>,
    distances: Cell<usize>,
    on_hit: RefCell<Option<Box<dyn FnOnce()>>>,
}
impl GeometryProbe {
    fn reset_counts(&self) {
        self.hits.set(0);
        self.distances.set(0);
    }
    fn counts(&self) -> (usize, usize) {
        (self.hits.get(), self.distances.get())
    }
}

struct ObservedFormatter {
    inner: Rc<dyn TextFormatter>,
    probe: Rc<GeometryProbe>,
}
impl TextFormatter for ObservedFormatter {
    fn format_line(
        &self,
        source: &dyn ITextSource,
        index: i32,
        width: f64,
        properties: &Rc<dyn TextParagraphProperties>,
        previous: Option<&Rc<TextLineBreak>>,
    ) -> Option<Rc<dyn TextLine>> {
        self.inner
            .format_line(source, index, width, properties, previous)
            .map(|inner| {
                Rc::new(ObservedLine {
                    inner,
                    probe: self.probe.clone(),
                }) as Rc<dyn TextLine>
            })
    }
}

struct ObservedLine {
    inner: Rc<dyn TextLine>,
    probe: Rc<GeometryProbe>,
}
macro_rules! forward_metric {
    ($name:ident -> $type:ty) => {
        fn $name(&self) -> $type {
            self.inner.$name()
        }
    };
}
impl TextLine for ObservedLine {
    fn text_runs(&self) -> CellRef<'_, [Rc<dyn TextRun>]> {
        self.inner.text_runs()
    }
    forward_metric!(first_text_source_index -> i32);
    forward_metric!(length -> i32);
    forward_metric!(text_line_break -> Option<Rc<TextLineBreak>>);
    forward_metric!(baseline -> f64);
    forward_metric!(extent -> f64);
    forward_metric!(has_collapsed -> bool);
    forward_metric!(has_overflowed -> bool);
    forward_metric!(height -> f64);
    forward_metric!(new_line_length -> i32);
    forward_metric!(overhang_after -> f64);
    forward_metric!(overhang_leading -> f64);
    forward_metric!(overhang_trailing -> f64);
    forward_metric!(start -> f64);
    forward_metric!(trailing_whitespace_length -> i32);
    forward_metric!(width -> f64);
    forward_metric!(width_including_trailing_whitespace -> f64);
    fn draw(&self, context: &mut dyn ITextDrawingSink, origin: Point) {
        self.inner.draw(context, origin);
    }
    fn collapse(
        self: Rc<Self>,
        properties: &[Option<Rc<dyn TextCollapsingProperties>>],
    ) -> Rc<dyn TextLine> {
        Rc::new(Self {
            inner: self.inner.clone().collapse(properties),
            probe: self.probe.clone(),
        })
    }
    fn justify(&self, properties: &dyn JustificationProperties) {
        self.inner.justify(properties);
    }
    fn get_character_hit_from_distance(&self, distance: f64) -> CharacterHit {
        self.probe.hits.set(self.probe.hits.get() + 1);
        let callback = self.probe.on_hit.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
        self.inner.get_character_hit_from_distance(distance)
    }
    fn get_distance_from_character_hit(&self, hit: CharacterHit) -> f64 {
        self.probe.distances.set(self.probe.distances.get() + 1);
        self.inner.get_distance_from_character_hit(hit)
    }
    fn get_next_caret_character_hit(&self, hit: CharacterHit) -> CharacterHit {
        self.inner.get_next_caret_character_hit(hit)
    }
    fn get_previous_caret_character_hit(&self, hit: CharacterHit) -> CharacterHit {
        self.inner.get_previous_caret_character_hit(hit)
    }
    fn get_backspace_caret_character_hit(&self, hit: CharacterHit) -> CharacterHit {
        self.inner.get_backspace_caret_character_hit(hit)
    }
    fn get_text_bounds(&self, index: i32, length: i32) -> Vec<TextBounds> {
        self.inner.get_text_bounds(index, length)
    }
    fn dispose(&self) {
        self.inner.dispose();
    }
    fn as_any(&self) -> &dyn Any {
        self.inner.as_any()
    }
}

fn install_observer() -> Rc<GeometryProbe> {
    let probe = Rc::new(GeometryProbe::default());
    FerroLocator::current_mutable()
        .bind::<dyn TextFormatter>()
        .to_constant(Rc::new(ObservedFormatter {
            inner: Rc::new(TextFormatterImpl::new()),
            probe: probe.clone(),
        }));
    probe
}

fn arrange(view: &RichTextBlock) {
    view.measure(Size::new(300.0, f64::INFINITY));
    view.arrange(Rect::new(0.0, 0.0, 300.0, 100_000.0));
}

#[test]
fn rich_selection_vertical_extension_geometry_work_is_independent_of_story_size() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    let mut work = Vec::new();
    for count in [8, 1024] {
        let probe = install_observer();
        let view = RichTextBlock::new();
        view.set_document(RichTextDocument::from_text(
            &vec!["aaaaaa"; count].join("\n"),
        ));
        let document = RichEditTextDocument::new(view.document());
        document.editor().set_text_view(Some(view.clone())).unwrap();
        arrange(&view);
        assert_eq!(view.line_metrics().len(), count);
        document.selection().set_range(1, 1).unwrap();
        probe.reset_counts();
        assert_eq!(
            document
                .selection()
                .move_down(TextRangeUnit::Line, 1, true)
                .unwrap(),
            1
        );
        assert_eq!(document.selection().selection(), EditorSelection::new(1, 8));
        work.push(probe.counts());
    }
    assert!(work[0].0 > 0 && work[0].1 > 0);
    assert_eq!(
        work[0], work[1],
        "one crossed line must perform the same caret geometry work"
    );
}

#[test]
fn rich_selection_vertical_redirected_edge_geometry_preserves_newer_callback_state() {
    let _scope = test_scope();
    let _text = TextTestScope::new();
    for change in 0..4 {
        let probe = install_observer();
        let owner = RichEditBox::new();
        owner.set_text("aaaaaa\na\naaaaaa\nbbbbbb\ncccccc").unwrap();
        let document = owner.text_document();
        let view: Ref<RichTextBlock> = owner.text_presenter().upcast();
        document.editor().set_text_view(Some(view.clone())).unwrap();
        arrange(&view);
        owner.select(5, 5);
        document.clear_undo_redo_history();
        let end = document.text_length();
        // A deferred model batch proves the native mutation identity is used,
        // independently of whether its public revision notification has run.
        let batch = (change == 0).then(|| document.document().begin_edit());
        let callback_document = document.clone();
        let invoked = Rc::new(Cell::new(false));
        let observed = invoked.clone();
        let callback: Box<dyn FnOnce()> = Box::new(move || {
            invoked.set(true);
            match change {
                0 => callback_document
                    .document()
                    .replace_text(0..end, "external")
                    .unwrap(),
                1 => callback_document.selection().set_range(3, 3).unwrap(),
                2 => callback_document.selection().set_range(5, end).unwrap(),
                _ => callback_document.editor().set_text_view(None).unwrap(),
            }
        });
        let pending = Rc::new(RefCell::new(Some(callback)));
        let weak = owner.downgrade();
        owner.selection_changing(move |_, args| {
            let callback = pending.borrow_mut().take();
            if let Some(callback) = callback {
                weak.upgrade().unwrap().select(5, end);
                args.set_cancel(true);
                // Hit testing on the path to the redirected edge happens after
                // selection publication. Ordinary caret publication asks only
                // for distances and cannot consume this one-shot callback.
                *probe.on_hit.borrow_mut() = Some(callback);
            }
        });
        assert!(
            matches!(
                document.selection().move_down(TextRangeUnit::Line, 1, true),
                Err(EditError::ReentrantEdit)
            ),
            "change={change}"
        );
        assert!(
            observed.get(),
            "the redirected geometry callback must execute"
        );
        match change {
            0 => assert_eq!(document.text(), "external"),
            1 => assert_eq!(document.selection().selection(), EditorSelection::caret(3)),
            2 => assert_eq!(
                document.selection().selection(),
                EditorSelection::new(5, end)
            ),
            _ => assert!(document
                .selection()
                .move_down(TextRangeUnit::Line, 1, false)
                .is_err()),
        }
        assert!(!document.can_undo());
        drop(batch);
    }
}
