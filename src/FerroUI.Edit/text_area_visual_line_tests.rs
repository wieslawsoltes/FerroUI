use super::*;
use crate::tests::{laid_out_editor, render_editor, test_scope};
use crate::{
    ColorizingTransformer, FormattedTextElement, ITextRunConstructionContext, ITextViewConnect,
    TextDocument, VisualLineElement, VisualLineElementData, VisualLineElementGenerator,
    VisualLineElementGeneratorRef, VisualLineError,
};
use ferroui_base::media::text_formatting::TextRun;
use std::any::Any;

type GenerationAction = dyn Fn(&dyn ITextRunConstructionContext);
type ElementFactory =
    dyn Fn(&dyn ITextRunConstructionContext) -> Result<Box<dyn VisualLineElement>, VisualLineError>;

struct ProbeGenerator {
    name: &'static str,
    events: Rc<RefCell<Vec<String>>>,
    connected: Cell<bool>,
    enabled: Cell<bool>,
    offset: usize,
    factory: Option<Rc<ElementFactory>>,
    on_start: RefCell<Option<Box<GenerationAction>>>,
    on_drop: Option<Box<dyn Fn()>>,
}
impl ProbeGenerator {
    fn new(name: &'static str, events: Rc<RefCell<Vec<String>>>) -> Self {
        Self {
            name,
            events,
            connected: Cell::new(false),
            enabled: Cell::new(true),
            offset: 0,
            factory: None,
            on_start: RefCell::new(None),
            on_drop: None,
        }
    }
    fn element_at(mut self, offset: usize, length: usize) -> Self {
        self.offset = offset;
        self.factory = Some(Rc::new(move |context| {
            Ok(Box::new(FormattedTextElement::new(
                "custom",
                length,
                &**context.global_text_run_properties(),
            )?))
        }));
        self
    }
    fn record(&self, action: &str) {
        self.events
            .borrow_mut()
            .push(format!("{} {action}", self.name));
    }
}
impl Drop for ProbeGenerator {
    fn drop(&mut self) {
        if let Some(callback) = self.on_drop.take() {
            callback();
        }
    }
}
impl ITextViewConnect for ProbeGenerator {
    fn add_to_text_view(&self, _: &TextArea) {
        assert!(!self.connected.replace(true));
        self.record("add");
    }
    fn remove_from_text_view(&self, _: &TextArea) {
        assert!(self.connected.replace(false));
        self.record("remove");
    }
}
impl VisualLineElementGenerator for ProbeGenerator {
    fn start_generation(
        &self,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<(), VisualLineError> {
        assert!(
            self.connected.get(),
            "a retained generator was disconnected early"
        );
        self.record("start");
        let callback = self.on_start.borrow_mut().take();
        if let Some(callback) = callback {
            callback(context);
        }
        Ok(())
    }
    fn finish_generation(&self) {
        assert!(self.connected.get());
        self.record("finish");
    }
    fn get_first_interested_offset(
        &self,
        start: usize,
        _: &dyn ITextRunConstructionContext,
    ) -> Result<Option<usize>, VisualLineError> {
        Ok(
            (self.enabled.get() && self.factory.is_some() && self.offset >= start)
                .then_some(self.offset),
        )
    }
    fn construct_element(
        &self,
        _: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<Box<dyn VisualLineElement>>, VisualLineError> {
        self.factory
            .as_ref()
            .map(|factory| factory(context))
            .transpose()
    }
}

#[test]
fn multiline_generated_elements_drive_live_cache_geometry_and_hidden_rows() {
    let _scope = test_scope();
    let editor = laid_out_editor("a{\n hidden\n}z\nlast");
    let area = editor.text_area();
    let generator =
        Rc::new(ProbeGenerator::new("span", Rc::new(RefCell::new(Vec::new()))).element_at(1, 11));
    area.element_generators().add(generator.into());
    render_editor(&editor);
    assert_eq!(area.cached_visual_line_count(), 2);
    let first = area.get_visual_line(1).unwrap();
    assert_eq!(first.last_document_line().line_number(), 3);
    assert!(Rc::ptr_eq(&first, &area.get_visual_line(2).unwrap()));
    assert!(Rc::ptr_eq(&first, &area.get_visual_line(3).unwrap()));
    assert_eq!(area.heights.borrow().visible_owner(2), 0);
    assert!(area.rectangle_for_offset(4).is_none());
    let suffix = area.rectangle_for_offset(12).unwrap();
    assert_eq!(suffix.y, 0.0);
    assert_eq!(
        area.offset_at_point(Point::new(suffix.x, suffix.y + suffix.height / 2.0)),
        12
    );
    let following = area.rectangle_for_offset(14).unwrap();
    assert_eq!(following.y, area.cache.borrow()[&0].layout.height());
}

#[test]
fn construction_event_updates_one_batch_and_transformers_change_native_metrics() {
    let _scope = test_scope();
    let editor = laid_out_editor("one\ntwo\nthree");
    let area = editor.text_area();
    let events = Rc::new(RefCell::new(Vec::new()));
    let probe: VisualLineElementGeneratorRef =
        Rc::new(ProbeGenerator::new("g", events.clone())).into();
    let calls = Rc::new(Cell::new(0));
    let recorded = calls.clone();
    let weak = area.downgrade();
    let subscription = area.visual_line_construction_starting(move |args| {
        assert_eq!(args.first_line_in_view.line_number(), 1);
        recorded.set(recorded.get() + 1);
        if recorded.get() == 1 {
            let area = weak.upgrade().unwrap();
            area.element_generators().add(probe.clone());
            area.line_transformers().add(
                Rc::new(ColorizingTransformer::new(|_, line| {
                    line.change_visual_elements(0, line.visual_length(), |element| {
                        element
                            .data_mut()
                            .text_run_properties_mut()
                            .set_font_rendering_em_size(40.0);
                    })
                }))
                .into(),
            );
        }
    });
    render_editor(&editor);
    assert_eq!(calls.get(), 1);
    assert_eq!(
        events.borrow().iter().filter(|e| *e == "g start").count(),
        3
    );
    assert!(area.cache.borrow()[&0].layout.height() > area.line_height() * 2.0);
    render_editor(&editor);
    assert_eq!(
        calls.get(),
        1,
        "cached paints must not start a construction batch"
    );
    subscription.dispose();
}

#[test]
fn collection_identity_reordering_and_duplicates_connect_once() {
    let _scope = test_scope();
    let editor = laid_out_editor("one");
    let area = editor.text_area();
    let events = Rc::new(RefCell::new(Vec::new()));
    let a: VisualLineElementGeneratorRef = Rc::new(ProbeGenerator::new("a", events.clone())).into();
    let b: VisualLineElementGeneratorRef = Rc::new(ProbeGenerator::new("b", events.clone())).into();
    let entries = area.element_generators();
    entries.add(a.clone());
    entries.add(b.clone());
    entries.add(a.clone());
    entries.move_item(0, 2);
    assert_eq!(*events.borrow(), ["a add", "b add"]);
    entries.remove(&a);
    assert_eq!(*events.borrow(), ["a add", "b add"]);
    entries.remove(&a);
    entries.clear();
    assert_eq!(*events.borrow(), ["a add", "b add", "a remove", "b remove"]);
}

#[test]
fn removal_during_construction_defers_disconnect_until_all_finish_hooks() {
    let _scope = test_scope();
    let editor = laid_out_editor("one");
    let area = editor.text_area();
    let events = Rc::new(RefCell::new(Vec::new()));
    let a = Rc::new(ProbeGenerator::new("a", events.clone()));
    let b: VisualLineElementGeneratorRef = Rc::new(ProbeGenerator::new("b", events.clone())).into();
    let entries = area.element_generators();
    let changed = entries.clone();
    let removed = b.clone();
    *a.on_start.borrow_mut() = Some(Box::new(move |_| {
        assert!(changed.remove(&removed));
    }));
    entries.add(a.into());
    entries.add(b);
    assert!(matches!(
        area.get_or_construct_visual_line(1),
        Err(VisualLineError::ConstructionInvalidated)
    ));
    assert_eq!(area.cached_visual_line_count(), 0);
    let events = events.borrow();
    let finish = events.iter().position(|e| e == "b finish").unwrap();
    let removed = events.iter().position(|e| e == "b remove").unwrap();
    assert!(finish < removed);
    drop(events);
    assert!(area.get_or_construct_visual_line(1).is_ok());
}

#[test]
fn document_mutation_and_recursive_requests_cannot_publish_stale_lines() {
    let _scope = test_scope();
    let editor = laid_out_editor("one");
    let area = editor.text_area();
    let document = editor.document();
    let weak = area.downgrade();
    let probe = Rc::new(ProbeGenerator::new("g", Rc::new(RefCell::new(Vec::new()))));
    *probe.on_start.borrow_mut() = Some(Box::new(move |_| {
        assert!(matches!(
            weak.upgrade().unwrap().get_or_construct_visual_line(1),
            Err(VisualLineError::RecursiveConstruction)
        ));
        document.insert(0, "new ").unwrap();
    }));
    area.element_generators().add(probe.into());
    assert!(matches!(
        area.get_or_construct_visual_line(1),
        Err(VisualLineError::ConstructionInvalidated)
    ));
    assert_eq!(area.cached_visual_line_count(), 0);
    assert_eq!(editor.text(), "new one");
    assert_eq!(
        area.get_or_construct_visual_line(1)
            .unwrap()
            .document_length(),
        7
    );
}

#[test]
fn a_retired_extension_can_register_a_new_connected_extension() {
    let _scope = test_scope();
    let editor = laid_out_editor("one");
    let area = editor.text_area();
    let events = Rc::new(RefCell::new(Vec::new()));
    let b: VisualLineElementGeneratorRef = Rc::new(ProbeGenerator::new("b", events.clone())).into();
    let weak = area.downgrade();
    let mut a = ProbeGenerator::new("a", events.clone());
    a.on_drop = Some(Box::new(move || {
        weak.upgrade().unwrap().element_generators().add(b.clone());
    }));
    area.element_generators().add(Rc::new(a).into());
    area.element_generators().clear();
    assert_eq!(*events.borrow(), ["a add", "a remove", "b add"]);
    assert!(area.get_or_construct_visual_line(1).is_ok());
}

struct RetiringElement {
    inner: FormattedTextElement,
    retired: Rc<Cell<bool>>,
    document: TextDocument,
}
impl Drop for RetiringElement {
    fn drop(&mut self) {
        if !self.retired.replace(true) {
            self.document.insert(0, "!").unwrap();
        }
    }
}
impl VisualLineElement for RetiringElement {
    fn data(&self) -> &VisualLineElementData {
        self.inner.data()
    }
    fn data_mut(&mut self) -> &mut VisualLineElementData {
        self.inner.data_mut()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn create_text_run(
        &self,
        column: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Rc<dyn TextRun>, VisualLineError> {
        self.inner.create_text_run(column, context)
    }
}

#[test]
fn hidden_line_eviction_revalidates_after_custom_element_destructors() {
    let _scope = test_scope();
    let editor = laid_out_editor("first\nsecond\nthird");
    let area = editor.text_area();
    let document = editor.document();
    let retired = Rc::new(Cell::new(false));
    let flag = retired.clone();
    let span =
        Rc::new(ProbeGenerator::new("span", Rc::new(RefCell::new(Vec::new()))).element_at(0, 13));
    span.enabled.set(false);
    area.element_generators().add(span.clone().into());
    let mut probe = ProbeGenerator::new("g", Rc::new(RefCell::new(Vec::new())));
    probe.offset = 6;
    probe.factory = Some(Rc::new(move |context| {
        Ok(Box::new(RetiringElement {
            inner: FormattedTextElement::new("child", 1, &**context.global_text_run_properties())?,
            retired: flag.clone(),
            document: document.clone(),
        }))
    }));
    area.element_generators().add(Rc::new(probe).into());
    area.get_or_construct_visual_line(2).unwrap();
    // The existing extension now projects a multiline span while an interior
    // row is retained. Releasing that row invokes its application destructor.
    span.enabled.set(true);
    assert!(matches!(
        area.get_or_construct_visual_line(1),
        Err(VisualLineError::ConstructionInvalidated)
    ));
    assert!(retired.get());
    assert_eq!(editor.text(), "!first\nsecond\nthird");
    assert_eq!(area.cached_visual_line_count(), 0);
}

#[test]
fn multiline_preedit_uses_start_owner_before_paint_and_restores_rows_on_cancel() {
    let _scope = test_scope();
    let editor = laid_out_editor("a{\n hidden\n}z\nlast");
    let area = editor.text_area();
    editor.select(1, 11).unwrap();
    let version = editor.document().version();
    area.set_preedit(Some("漢😀"), Some(3));
    let caret = area.cursor_rectangle();
    assert_eq!(caret.y, 0.0);
    assert!(caret.x > area.gutter_width());
    assert_eq!(
        area.get_visual_line(3)
            .unwrap()
            .first_document_line()
            .line_number(),
        1
    );
    assert_eq!(area.heights.borrow().visible_owner(2), 0);
    assert_eq!(editor.document().version(), version);
    editor.select(14, 4).unwrap();
    assert!(area.cursor_rectangle().y > 0.0);
    assert_eq!(
        area.get_visual_line(4)
            .unwrap()
            .composition_range()
            .unwrap()
            .len(),
        3
    );
    area.set_preedit(None, None);
    assert_eq!(area.heights.borrow().visible_owner(2), 2);
    assert_eq!(editor.document().version(), version);
}

#[test]
fn folded_keyboard_movement_keeps_atomic_spans_closed() {
    let _scope = test_scope();
    let editor = laid_out_editor("a{\n hidden\n}z\nlast");
    let fold = editor.install_folding().create_folding(1, 12).unwrap();
    fold.set_is_folded(true);
    editor.set_caret_offset(1).unwrap();
    let area = editor.text_area();
    for (key, expected) in [
        (Key::Right, 12),
        (Key::Left, 1),
        (Key::End, 13),
        (Key::Home, 0),
    ] {
        let mut event = KeyEventArgs::new();
        event.key = key;
        event.set_routed_event(Some(InputElement::key_down_event()));
        area.raise_event(&event);
        assert!(event.handled());
        assert_eq!(editor.caret_offset(), expected);
        assert!(fold.is_folded());
    }
}

#[test]
fn overlapping_offscreen_folds_seed_the_complete_collapsed_union() {
    let _scope = test_scope();
    let editor = laid_out_editor("a\nb\nc\nd\ne\nf\ng");
    let manager = editor.install_folding();
    let a = manager.create_folding(1, 5).unwrap();
    let b = manager.create_folding(3, 9).unwrap();
    a.set_is_folded(true);
    b.set_is_folded(true);
    let area = editor.text_area();
    assert_eq!(area.cached_visual_line_count(), 0);
    assert_eq!(area.heights.borrow().visible_owner(4), 0);
    assert_eq!(area.heights.borrow().next_visible(1), 5);
    assert!(area.rectangle_for_offset(8).is_none());
    let merged = area.get_or_construct_visual_line(1).unwrap();
    assert_eq!(merged.last_document_line().line_number(), 5);
}

#[test]
fn unarranged_view_uses_utf16_tab_columns_without_allocating_a_display_line() {
    let _scope = test_scope();
    let editor = TextEditor::new();
    editor.set_text("a\t😀\tz").unwrap();
    let area = editor.text_area();
    for (offset, column) in [(0, 0), (1, 1), (2, 4), (4, 6), (5, 8), (6, 9)] {
        assert_eq!(area.visual_column_for_offset(offset), Some(column));
    }
    assert_eq!(area.cached_visual_line_count(), 0);
}

#[test]
fn viewport_routes_generated_hyperlink_pointer_input_before_editing_selection() {
    use crate::{OpenUriRoutedEventArgs, VisualLineLinkText};
    use ferroui_base::input::{
        Pointer, PointerPointProperties, PointerType, PointerUpdateKind, RawInputModifiers,
    };
    use ferroui_controls::Window;
    let _scope = test_scope();
    let editor = laid_out_editor("visit https://example.org now");
    let window = Window::new();
    window.set_content(Some(Control::boxed(editor.clone())));
    window.show();
    let area = editor.text_area();
    let received = Rc::new(RefCell::new(Vec::new()));
    let observed = received.clone();
    area.add_handler(
        &VisualLineLinkText::open_uri_event(),
        move |_, event: &OpenUriRoutedEventArgs| {
            observed
                .borrow_mut()
                .push(event.uri.original_string().to_owned());
            event.set_handled(true);
        },
    );
    let rect = area.rectangle_for_offset(12).unwrap();
    let selection = editor.selection();
    let point = area
        .translate_point(
            Point::new(rect.x + 1.0, rect.y + rect.height / 2.0),
            &window,
        )
        .unwrap();
    let press = PointerPressedEventArgs::new(
        area.clone(),
        Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true),
        &window,
        point,
        1,
        PointerPointProperties::new(
            RawInputModifiers::LEFT_MOUSE_BUTTON,
            PointerUpdateKind::LeftButtonPressed,
        ),
        KeyModifiers::CONTROL,
        1,
    );
    area.raise_event(&press);
    assert!(press.handled());
    assert_eq!(*received.borrow(), ["https://example.org"]);
    assert_eq!(editor.selection(), selection);
    let mut options = editor.options();
    options.enable_hyperlinks = false;
    editor.set_options(options);
    assert!(
        !area
            .get_or_construct_visual_line(1)
            .unwrap()
            .elements()
            .iter()
            .any(|element| element.as_any().is::<VisualLineLinkText>())
    );
    window.close();
}

#[test]
fn native_fold_placeholder_handles_pointer_activation() {
    use ferroui_base::input::{
        Pointer, PointerPointProperties, PointerType, PointerUpdateKind, RawInputModifiers,
    };
    use ferroui_controls::Window;
    let _scope = test_scope();
    let editor = laid_out_editor("a{\n hidden\n}z\nlast");
    let fold = editor.install_folding().create_folding(1, 12).unwrap();
    fold.set_is_folded(true);
    let window = Window::new();
    window.set_content(Some(Control::boxed(editor.clone())));
    window.show();
    let area = editor.text_area();
    let start = area.rectangle_for_offset(1).unwrap();
    let end = area.rectangle_for_offset(12).unwrap();
    let selection = editor.selection();
    let point = area
        .translate_point(
            Point::new((start.x + end.x) / 2.0, start.y + start.height / 2.0),
            &window,
        )
        .unwrap();
    let press = PointerPressedEventArgs::new(
        area.clone(),
        Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true),
        &window,
        point,
        1,
        PointerPointProperties::new(
            RawInputModifiers::LEFT_MOUSE_BUTTON,
            PointerUpdateKind::LeftButtonPressed,
        ),
        KeyModifiers::NONE,
        1,
    );
    area.raise_event(&press);
    assert!(press.handled());
    assert!(!fold.is_folded());
    assert_eq!(editor.selection(), selection);
    window.close();
}
