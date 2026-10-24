use super::*;
use crate::tests::{laid_out_editor, render_editor, test_scope};
use crate::{
    BackgroundRendererRef, FormattedTextContent, FormattedTextElement, IBackgroundRenderer,
    ITextRunConstructionContext, ITextViewConnect, KnownLayer, TextDocument, VisualLineElement,
    VisualLineElementGenerator, VisualLineError,
};
use ferroui_base::media::text_formatting::{
    DrawableTextRun, ITextDrawingSink, TextRun, TextRunProperties,
};
use ferroui_edit_core::document::{ISegment, SimpleSegment};
use std::any::Any;

type DrawAction = dyn Fn(&TextArea);

#[test]
fn builtin_renderers_follow_options_and_native_style_changes_without_reshaping() {
    let _scope = test_scope();
    let editor = laid_out_editor("first\nsecond");
    let area = editor.text_area();
    let mut options = editor.options();
    assert!(!options.show_column_rulers);
    assert_eq!(options.column_ruler_positions, [80]);
    options.highlight_current_line = true;
    options.show_column_rulers = true;
    options.column_ruler_positions = vec![4, 8];
    editor.set_options(options);
    editor.select(6, 0).unwrap();
    assert!(area.current_line_background().is_none());
    assert!(area.current_line_border().is_none());
    let log = render_editor(&editor);
    assert_eq!(area.current_line_highlight_renderer().line(), 2);
    assert_eq!(area.column_ruler_renderer().columns(), [4, 8]);
    assert!(log.iter().any(|line| line.contains("#1614dce0")));
    assert_eq!(
        log.iter()
            .filter(|line| line.starts_with("DrawLine #5a808080@1"))
            .count(),
        2,
    );
    let retained = area.get_visual_line(1).unwrap();
    area.set_current_line_background(Some(Rc::new(ImmutableSolidColorBrush::from_uint32(
        0xffaabb55,
    ))));
    area.set_current_line_border(Some(Rc::new(
        ferroui_base::media::immutable::ImmutablePen::from_uint32(0xffaabb66, 1.0),
    )));
    area.set_column_ruler_pen(Some(Rc::new(
        ferroui_base::media::immutable::ImmutablePen::from_uint32(0xffaabb77, 2.0),
    )));
    let log = render_editor(&editor);
    assert!(log.iter().any(|line| line.contains("#ffaabb55")));
    assert!(log.iter().any(|line| line.contains("#ffaabb66@1")));
    assert_eq!(
        log.iter()
            .filter(|line| line.starts_with("DrawLine #ffaabb77@2"))
            .count(),
        2
    );
    assert!(Rc::ptr_eq(&retained, &area.get_visual_line(1).unwrap()));
    area.set_current_line_background(None);
    area.set_current_line_border(None);
    area.set_column_ruler_pen(None);
    let log = render_editor(&editor);
    assert!(
        !log.iter()
            .any(|line| line.starts_with("DrawGeometry") || line.starts_with("DrawLine"))
    );
    area.set_default_highlight_line_colors();
    assert!(area.current_line_background().is_none());
    assert!(
        render_editor(&editor)
            .iter()
            .any(|line| line.contains("#1614dce0"))
    );
    assert!(Rc::ptr_eq(&retained, &area.get_visual_line(1).unwrap()));
}

#[test]
fn builtin_options_and_caret_are_captured_after_construction_callbacks() {
    let _scope = test_scope();
    let editor = laid_out_editor("first\nsecond");
    let area = editor.text_area();
    let weak = editor.downgrade();
    let once = Cell::new(false);
    let subscription = area.visual_line_construction_starting(move |_| {
        if once.replace(true) {
            return;
        }
        let editor = weak.upgrade().unwrap();
        let mut options = editor.options();
        options.highlight_current_line = true;
        options.show_column_rulers = true;
        options.column_ruler_positions = vec![12];
        editor.set_options(options);
        editor.select(6, 0).unwrap();
    });
    let log = render_editor(&editor);
    assert_eq!(area.current_line_highlight_renderer().line(), 2);
    assert!(area.current_line_highlight_renderer().enabled());
    assert_eq!(area.column_ruler_renderer().columns(), [12]);
    assert!(log.iter().any(|line| line.contains("#1614dce0")));
    assert!(editor.last_error().is_none(), "{:?}", editor.last_error());
    Dispatcher::ui_thread().run_jobs(None);
    assert!(area.cursor_rectangle().height > 0.0);
    assert!(editor.last_error().is_none(), "{:?}", editor.last_error());
    subscription.dispose();
}

struct Renderer {
    name: &'static str,
    layer: KnownLayer,
    color: u32,
    events: Rc<RefCell<Vec<String>>>,
    connected: Cell<bool>,
    on_add: RefCell<Option<Box<DrawAction>>>,
    on_remove: RefCell<Option<Box<DrawAction>>>,
    action: RefCell<Option<Box<DrawAction>>>,
    fails: bool,
}
impl Renderer {
    fn new(
        name: &'static str,
        layer: KnownLayer,
        color: u32,
        events: Rc<RefCell<Vec<String>>>,
    ) -> Self {
        Self {
            name,
            layer,
            color,
            events,
            connected: Cell::new(false),
            on_add: RefCell::new(None),
            on_remove: RefCell::new(None),
            action: RefCell::new(None),
            fails: false,
        }
    }
    fn record(&self, action: &str) {
        self.events
            .borrow_mut()
            .push(format!("{} {action}", self.name));
    }
}
impl ITextViewConnect for Renderer {
    fn add_to_text_view(&self, area: &TextArea) {
        assert!(!self.connected.replace(true));
        self.record("add");
        let action = self.on_add.borrow_mut().take();
        if let Some(action) = action {
            action(area);
        }
    }
    fn remove_from_text_view(&self, area: &TextArea) {
        assert!(self.connected.replace(false));
        self.record("remove");
        let action = self.on_remove.borrow_mut().take();
        if let Some(action) = action {
            action(area);
        }
    }
}
impl IBackgroundRenderer for Renderer {
    fn layer(&self) -> KnownLayer {
        self.layer
    }
    fn draw(&self, area: &TextArea, context: &mut DrawingContext) -> Result<(), VisualLineError> {
        assert!(self.connected.get());
        assert!(!area.visual_lines().is_empty());
        self.record("draw");
        let brush = Rc::new(ImmutableSolidColorBrush::from_uint32(self.color)) as Rc<dyn IBrush>;
        context.fill_rectangle(&brush, Rect::new(30.0, 0.0, 1.0, 1.0), 0.0);
        let action = self.action.borrow_mut().take();
        if let Some(action) = action {
            action(area);
        }
        if self.fails {
            Err(VisualLineError::Extension("renderer rejected frame".into()))
        } else {
            Ok(())
        }
    }
}

#[test]
fn renderers_draw_once_in_global_layer_order_around_native_content() {
    let _scope = test_scope();
    let editor = laid_out_editor("first\nsecond");
    editor.select(0, 3).unwrap();
    let area = editor.text_area();
    let events = Rc::new(RefCell::new(Vec::new()));
    for (name, layer, color) in [
        ("caret", KnownLayer::Caret, 0xffaabb01),
        ("text", KnownLayer::Text, 0xffaabb02),
        ("selection", KnownLayer::Selection, 0xffaabb03),
        ("background", KnownLayer::Background, 0xffaabb04),
    ] {
        area.background_renderers()
            .add(Rc::new(Renderer::new(name, layer, color, events.clone())).into());
    }
    events.borrow_mut().clear();
    let log = render_editor(&editor);
    assert_eq!(
        *events.borrow(),
        [
            "background draw",
            "selection draw",
            "text draw",
            "caret draw"
        ]
    );
    let position = |needle: &str| log.iter().position(|entry| entry.contains(needle)).unwrap();
    assert!(position("#ffaabb04") < position("#ffaabb03"));
    assert!(position("#ffaabb03") < position("DrawGeometry"));
    assert!(position("DrawGeometry") < position("#ffaabb02"));
    assert!(position("#ffaabb02") < position("DrawGlyphRun"));
    assert!(position("DrawGlyphRun") < position("#ffaabb01"));
    assert_eq!(
        log.iter()
            .filter(|entry| entry.starts_with("PushClip"))
            .count(),
        log.iter()
            .filter(|entry| entry.starts_with("PopClip"))
            .count()
    );
}

#[test]
fn renderer_changes_and_layer_invalidation_preserve_shaped_lines() {
    let _scope = test_scope();
    let editor = laid_out_editor("first\nsecond");
    let area = editor.text_area();
    area.ensure_visual_lines().unwrap();
    let before = area.get_visual_line(1).unwrap();
    let events = Rc::new(RefCell::new(Vec::new()));
    let renderer: BackgroundRendererRef = Rc::new(Renderer::new(
        "a",
        KnownLayer::Text,
        0xffffffff,
        events.clone(),
    ))
    .into();
    let collection = area.background_renderers();
    collection.add(renderer.clone());
    collection.add(renderer.clone());
    collection.move_item(0, 1);
    collection.remove(&renderer);
    assert_eq!(*events.borrow(), ["a add"]);
    area.invalidate_layer(KnownLayer::Selection);
    render_editor(&editor);
    assert!(Rc::ptr_eq(&before, &area.get_visual_line(1).unwrap()));
    collection.clear();
    assert_eq!(*events.borrow(), ["a add", "a draw", "a remove"]);
    assert!(Rc::ptr_eq(&before, &area.get_visual_line(1).unwrap()));
}

#[test]
fn renderer_removal_is_deferred_until_its_retained_frame_finishes() {
    let _scope = test_scope();
    let editor = laid_out_editor("first");
    let area = editor.text_area();
    let events = Rc::new(RefCell::new(Vec::new()));
    let a = Rc::new(Renderer::new(
        "a",
        KnownLayer::Background,
        0xffffffff,
        events.clone(),
    ));
    let b: BackgroundRendererRef = Rc::new(Renderer::new(
        "b",
        KnownLayer::Caret,
        0xffffffff,
        events.clone(),
    ))
    .into();
    let removed = b.clone();
    *a.action.borrow_mut() = Some(Box::new(move |area| {
        assert!(area.background_renderers().remove(&removed));
    }));
    area.background_renderers().add(a.into());
    area.background_renderers().add(b);
    events.borrow_mut().clear();
    render_editor(&editor);
    assert_eq!(*events.borrow(), ["a draw", "b draw", "b remove"]);
    assert!(editor.last_error().is_none());
    events.borrow_mut().clear();
    render_editor(&editor);
    assert_eq!(*events.borrow(), ["a draw"]);
}

#[test]
fn reentrant_rendering_waits_for_all_renderer_connection_hooks_to_finish() {
    let _scope = test_scope();
    let editor = laid_out_editor("first");
    let area = editor.text_area();
    let collection = area.background_renderers();
    let events = Rc::new(RefCell::new(Vec::new()));
    let nested_logs = Rc::new(RefCell::new(Vec::new()));
    let a = Rc::new(Renderer::new(
        "a",
        KnownLayer::Background,
        0xffaabb01,
        events.clone(),
    ));
    let b = Rc::new(Renderer::new(
        "b",
        KnownLayer::Caret,
        0xffaabb02,
        events.clone(),
    ));
    let logs = nested_logs.clone();
    *a.on_add.borrow_mut() = Some(Box::new(move |area| {
        let log = render_editor(&area.editor().unwrap());
        logs.borrow_mut().push(log);
    }));
    let a_identity: BackgroundRendererRef = a.clone().into();
    collection.add_range([a_identity.clone(), b.clone().into()]);
    assert_eq!(*events.borrow(), ["a add", "b add"]);
    assert_eq!(nested_logs.borrow().len(), 1);
    assert!(nested_logs.borrow()[0].is_empty());
    assert!(editor.last_error().is_none());

    events.borrow_mut().clear();
    assert!(
        render_editor(&editor)
            .iter()
            .any(|entry| entry.starts_with("DrawGlyphRun"))
    );
    assert_eq!(*events.borrow(), ["a draw", "b draw"]);

    let c = Rc::new(Renderer::new(
        "c",
        KnownLayer::Selection,
        0xffaabb03,
        events.clone(),
    ));
    let logs = nested_logs.clone();
    *a.on_remove.borrow_mut() = Some(Box::new(move |area| {
        let log = render_editor(&area.editor().unwrap());
        logs.borrow_mut().push(log);
    }));
    events.borrow_mut().clear();
    collection.set(collection.index_of(&a_identity).unwrap(), c.into());
    assert_eq!(*events.borrow(), ["a remove", "c add"]);
    assert_eq!(nested_logs.borrow().len(), 2);
    assert!(nested_logs.borrow()[1].is_empty());
    assert!(editor.last_error().is_none());
    events.borrow_mut().clear();
    render_editor(&editor);
    assert_eq!(*events.borrow(), ["c draw", "b draw"]);
}

struct MutatingDrawable {
    document: TextDocument,
    properties: Rc<dyn TextRunProperties>,
    draws: Rc<Cell<usize>>,
    mutated: Rc<Cell<bool>>,
}
impl TextRun for MutatingDrawable {
    fn properties(&self) -> Option<&Rc<dyn TextRunProperties>> {
        Some(&self.properties)
    }
    fn as_drawable(&self) -> Option<&dyn DrawableTextRun> {
        Some(self)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}
impl DrawableTextRun for MutatingDrawable {
    fn size(&self) -> Size {
        Size::new(27.0, 30.0)
    }
    fn baseline(&self) -> f64 {
        22.0
    }
    fn draw(&self, sink: &mut dyn ITextDrawingSink, origin: Point) {
        self.draws.set(self.draws.get() + 1);
        let brush = Rc::new(ImmutableSolidColorBrush::from_uint32(0xff01cc02)) as Rc<dyn IBrush>;
        sink.draw_rectangle(
            Some(&brush),
            None,
            Rect::new(origin.x, origin.y, 27.0, 30.0),
        );
        if !self.mutated.replace(true) {
            self.document.insert(0, "new ").unwrap();
        }
    }
}
struct MutatingDrawableGenerator {
    document: TextDocument,
    draws: Rc<Cell<usize>>,
    mutated: Rc<Cell<bool>>,
}
impl ITextViewConnect for MutatingDrawableGenerator {}
impl VisualLineElementGenerator for MutatingDrawableGenerator {
    fn get_first_interested_offset(
        &self,
        start: usize,
        _: &dyn ITextRunConstructionContext,
    ) -> Result<Option<usize>, VisualLineError> {
        Ok((start == 0).then_some(0))
    }
    fn construct_element(
        &self,
        _: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<Box<dyn VisualLineElement>>, VisualLineError> {
        let run = Rc::new(MutatingDrawable {
            document: self.document.clone(),
            properties: context.global_text_run_properties().clone(),
            draws: self.draws.clone(),
            mutated: self.mutated.clone(),
        });
        Ok(Some(Box::new(FormattedTextElement::from_content(
            FormattedTextContent::Drawable(run),
            1,
            &**context.global_text_run_properties(),
        )?)))
    }
}

#[test]
fn inline_drawable_mutation_stops_later_rows_carets_and_margin_glyphs() {
    let _scope = test_scope();
    let editor = laid_out_editor("X\nlater");
    editor.set_show_line_numbers(true);
    editor.set_foreground(Some(Rc::new(ImmutableSolidColorBrush::from_uint32(
        0xff3344cc,
    ))));
    editor.set_value(
        TextEditor::line_numbers_foreground_property(),
        Some(Rc::new(ImmutableSolidColorBrush::from_uint32(0xff5577ee)) as Rc<dyn IBrush>),
    );
    editor.set_caret_offset(2).unwrap();
    let area = editor.text_area();
    area.raise_event(&FocusChangedEventArgs::new(InputElement::got_focus_event()));
    assert!(area.is_focused());
    assert!(area.caret_visible.get());
    let draws = Rc::new(Cell::new(0));
    area.element_generators().add(
        Rc::new(MutatingDrawableGenerator {
            document: editor.document(),
            draws: draws.clone(),
            mutated: Rc::new(Cell::new(false)),
        })
        .into(),
    );
    let log = render_editor(&editor);
    assert_eq!(draws.get(), 1);
    assert_eq!(editor.text(), "new X\nlater");
    assert!(log.iter().any(|entry| entry.contains("#ff01cc02")));
    // The first line is entirely the drawable. All glyphs would therefore
    // belong to the later logical row or to the line-number margin.
    assert!(!log.iter().any(|entry| entry.starts_with("DrawGlyphRun")));
    assert!(
        !log.iter()
            .any(|entry| entry.starts_with("DrawRectangle #ff3344cc"))
    );
    assert_eq!(
        editor.last_error().as_deref(),
        Some(
            VisualLineError::ConstructionInvalidated
                .to_string()
                .as_str()
        )
    );
    assert_eq!(
        log.iter()
            .filter(|entry| entry.starts_with("PushClip"))
            .count(),
        log.iter()
            .filter(|entry| entry.starts_with("PopClip"))
            .count()
    );
    let recovered = render_editor(&editor);
    assert_eq!(draws.get(), 2);
    assert!(
        recovered
            .iter()
            .any(|entry| entry.starts_with("DrawGlyphRun #ff3344cc"))
    );
    assert!(
        recovered
            .iter()
            .any(|entry| entry.starts_with("DrawGlyphRun #ff5577ee"))
    );
    assert!(
        recovered
            .iter()
            .any(|entry| entry.starts_with("DrawRectangle #ff3344cc"))
    );
    area.raise_event(&FocusChangedEventArgs::new(InputElement::lost_focus_event()));
}

#[test]
fn renderer_document_mutation_aborts_stale_paint_without_retaining_borrows() {
    let _scope = test_scope();
    let editor = laid_out_editor("first");
    let events = Rc::new(RefCell::new(Vec::new()));
    let a = Rc::new(Renderer::new(
        "a",
        KnownLayer::Background,
        0xffffffff,
        events.clone(),
    ));
    *a.action.borrow_mut() = Some(Box::new(|area| {
        area.editor().unwrap().document().insert(0, "new ").unwrap();
    }));
    editor.text_area().background_renderers().add(a.into());
    let log = render_editor(&editor);
    assert_eq!(editor.text(), "new first");
    assert!(!log.iter().any(|entry| entry.starts_with("DrawGlyphRun")));
    assert!(editor.last_error().is_some());
    let current = editor.document().version();
    assert!(
        editor
            .text_area()
            .visual_lines()
            .iter()
            .all(|line| { line.context().document().version().as_ref() == Some(&current) })
    );
    assert!(
        render_editor(&editor)
            .iter()
            .any(|entry| entry.starts_with("DrawGlyphRun"))
    );
}

#[test]
fn renderer_errors_restore_native_clip_stack_and_report_diagnostics() {
    let _scope = test_scope();
    let editor = laid_out_editor("first");
    let mut renderer = Renderer::new(
        "a",
        KnownLayer::Selection,
        0xffffffff,
        Rc::new(RefCell::new(Vec::new())),
    );
    renderer.fails = true;
    editor
        .text_area()
        .background_renderers()
        .add(Rc::new(renderer).into());
    let log = render_editor(&editor);
    assert!(
        editor
            .last_error()
            .unwrap()
            .contains("renderer rejected frame")
    );
    assert_eq!(
        log.iter()
            .filter(|entry| entry.starts_with("PushClip"))
            .count(),
        log.iter()
            .filter(|entry| entry.starts_with("PopClip"))
            .count()
    );
    assert!(!log.iter().any(|entry| entry.starts_with("DrawGlyphRun")));
}

#[test]
fn public_background_geometry_constructs_visible_rows_and_includes_empty_lines() {
    let _scope = test_scope();
    let editor = laid_out_editor(&format!("\nabc\n{}", "offscreen\n".repeat(1000)));
    let area = editor.text_area();
    assert_eq!(area.cached_visual_line_count(), 0);
    let empty = area
        .segment_rectangles(&SimpleSegment::new(0, 1), false)
        .unwrap();
    assert!(!empty.is_empty());
    assert!(empty[0].width > 0.0 && empty[0].height > 0.0);
    assert!(area.cached_visual_line_count() < 30);
    assert!(area.visual_line_bounds(900).is_none());
    let normal = area.visual_line_bounds(2).unwrap();
    assert_eq!(normal.x, area.content_viewport().x);
    assert_eq!(normal.width, area.content_viewport().width);
    let virtual_selection = SelectionSegment::with_virtual_space(4, 2, 4, 6);
    let virtual_rects = area.selection_rectangles(virtual_selection, false).unwrap();
    assert_eq!(virtual_rects.len(), 1);
    assert!((virtual_rects[0].width - 4.0 * area.wide_space_width()).abs() < 0.001);
}

#[test]
fn public_segment_geometry_rejects_document_changes_from_segment_accessors() {
    struct MutatingSegment {
        document: TextDocument,
        in_length: bool,
        mutated: Cell<bool>,
    }
    impl MutatingSegment {
        fn mutate(&self) {
            if !self.mutated.replace(true) {
                self.document.insert(0, "new ").unwrap();
            }
        }
    }
    impl ISegment for MutatingSegment {
        fn offset(&self) -> usize {
            if !self.in_length {
                self.mutate();
            }
            0
        }
        fn length(&self) -> usize {
            if self.in_length {
                self.mutate();
            }
            1
        }
    }
    let _scope = test_scope();
    for in_length in [false, true] {
        let editor = laid_out_editor("first\nsecond");
        let area = editor.text_area();
        let segment = MutatingSegment {
            document: editor.document(),
            in_length,
            mutated: Cell::new(false),
        };
        assert_eq!(
            area.segment_rectangles(&segment, false),
            Err(VisualLineError::ConstructionInvalidated)
        );
        assert!(segment.mutated.get());
        assert_eq!(editor.text(), "new first\nsecond");
        let recovered = area
            .segment_rectangles(&SimpleSegment::new(4, 1), false)
            .unwrap();
        assert!(!recovered.is_empty());
        assert!(
            recovered
                .iter()
                .all(|rect| rect.width > 0.0 && rect.height > 0.0)
        );
        assert_eq!(
            area.get_visual_line(1)
                .unwrap()
                .context()
                .document()
                .version(),
            Some(editor.document().version())
        );
    }
}
