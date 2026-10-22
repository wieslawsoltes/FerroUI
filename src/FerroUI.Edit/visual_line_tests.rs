use crate::formatted_text_element::{FormattedTextContent, FormattedTextElement};
use crate::i_text_run_construction_context::{
    EndOfLineMarkerOptions, ITextRunConstructionContext, VisualLineBuildOptions,
};
use crate::i_text_view_connect::ITextViewConnect;
use crate::visual_line::{VisualLine, VisualLineBuilder, VISUAL_LINE_LENGTH_LIMIT};
use crate::visual_line_element::{VisualLineElement, VisualLineElementData};
use crate::visual_line_element_generator::{
    VisualLineElementGenerator, VisualLineElementGeneratorRef,
};
use crate::visual_line_error::VisualLineError;
use crate::visual_line_text::VisualLineText;
use crate::visual_line_transformer::{
    ColorizingTransformer, DocumentColorizingTransformer, IVisualLineTransformer,
};
use ferroui_base::media::text_formatting::{
    DrawableTextRun, GenericTextParagraphProperties, GenericTextRunProperties, ITextDrawingSink,
    ITextSource, TextLayout, TextRun, TextRunProperties,
};
use ferroui_base::media::{FontFamily, TextAlignment, TextWrapping, Typeface};
use ferroui_base::utilities::ReadOnlyMemory;
use ferroui_base::{Point, Rect, Size};
use ferroui_edit_core::document::{
    CaretPositioningMode, DocumentError, LogicalDirection, TextDocument, TextSnapshot,
};
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn properties() -> Rc<dyn TextRunProperties> {
    Rc::new(GenericTextRunProperties::with_font_size(
        Typeface::new(FontFamily::new("Test Sans")),
        12.0,
    ))
}
fn builder(text: &str, generators: &[VisualLineElementGeneratorRef]) -> VisualLineBuilder {
    VisualLineBuilder::generate(
        TextSnapshot::from_text(text),
        1,
        properties(),
        generators,
        VisualLineBuildOptions::default(),
    )
    .unwrap()
}
fn layout(
    line: &Rc<VisualLine>,
    width: f64,
    wrap: TextWrapping,
) -> Result<TextLayout, VisualLineError> {
    line.create_text_layout(
        Rc::new(GenericTextParagraphProperties::with_options(
            properties(),
            TextAlignment::Left,
            wrap,
            0.0,
            0.0,
        )),
        width,
        f64::INFINITY,
        0,
        None,
        None,
    )
}

type ElementFactory =
    dyn Fn(&dyn ITextRunConstructionContext) -> Result<Box<dyn VisualLineElement>, VisualLineError>;
struct TestGenerator {
    name: &'static str,
    offset: usize,
    factory: Rc<ElementFactory>,
    events: Rc<RefCell<Vec<String>>>,
    long_lines: bool,
    fail_start: bool,
    invalid_interest: bool,
}
impl TestGenerator {
    fn at(
        offset: usize,
        document_length: usize,
        name: &'static str,
        events: Rc<RefCell<Vec<String>>>,
    ) -> Self {
        Self {
            name,
            offset,
            events,
            long_lines: false,
            fail_start: false,
            invalid_interest: false,
            factory: Rc::new(move |context| {
                Ok(Box::new(FormattedTextElement::new(
                    name,
                    document_length,
                    &**context.global_text_run_properties(),
                )?))
            }),
        }
    }
}
impl ITextViewConnect for TestGenerator {}
impl VisualLineElementGenerator for TestGenerator {
    fn start_generation(
        &self,
        _context: &dyn ITextRunConstructionContext,
    ) -> Result<(), VisualLineError> {
        self.events
            .borrow_mut()
            .push(format!("start {}", self.name));
        if self.fail_start {
            Err(VisualLineError::Extension("start failed".into()))
        } else {
            Ok(())
        }
    }
    fn finish_generation(&self) {
        self.events
            .borrow_mut()
            .push(format!("finish {}", self.name));
    }
    fn runs_on_long_lines(&self) -> bool {
        self.long_lines
    }
    fn get_first_interested_offset(
        &self,
        start: usize,
        _context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<usize>, VisualLineError> {
        if self.invalid_interest && start > 0 {
            return Ok(Some(start - 1));
        }
        Ok((self.offset >= start).then_some(self.offset))
    }
    fn construct_element(
        &self,
        _offset: usize,
        context: &dyn ITextRunConstructionContext,
    ) -> Result<Option<Box<dyn VisualLineElement>>, VisualLineError> {
        self.events
            .borrow_mut()
            .push(format!("construct {}", self.name));
        (self.factory)(context).map(Some)
    }
}
fn one_generator(offset: usize, length: usize) -> VisualLineElementGeneratorRef {
    Rc::new(TestGenerator::at(offset, length, "label", Rc::default())).into()
}

#[test]
fn generators_keep_order_and_zero_document_elements_cannot_loop() {
    let _scope = crate::tests::test_scope();
    let events = Rc::new(RefCell::new(Vec::new()));
    let generators: Vec<VisualLineElementGeneratorRef> = [
        TestGenerator::at(0, 0, "before", events.clone()),
        TestGenerator::at(0, 1, "replace", events.clone()),
        TestGenerator::at(0, 1, "later", events.clone()),
    ]
    .into_iter()
    .map(|generator| Rc::new(generator).into())
    .collect();
    let line = builder("ab", &generators).finish().unwrap();
    assert_eq!(line.elements().len(), 3);
    assert_eq!(line.document_length(), 2);
    assert_eq!(line.visual_length(), 3);
    assert_eq!(
        *events.borrow(),
        [
            "start before",
            "start replace",
            "start later",
            "construct before",
            "construct replace",
            "finish before",
            "finish replace",
            "finish later",
        ]
    );
    let line = builder("", &[one_generator(0, 0)]).finish().unwrap();
    assert_eq!((line.document_length(), line.visual_length()), (0, 1));
}

#[test]
fn generator_lifetimes_finish_when_start_or_construction_fails() {
    let _scope = crate::tests::test_scope();
    let events = Rc::new(RefCell::new(Vec::new()));
    let first = TestGenerator::at(0, 1, "first", events.clone());
    let mut second = TestGenerator::at(0, 1, "second", events.clone());
    second.fail_start = true;
    let result = VisualLineBuilder::generate(
        TextSnapshot::from_text("a"),
        1,
        properties(),
        &[Rc::new(first).into(), Rc::new(second).into()],
        VisualLineBuildOptions::default(),
    );
    assert!(matches!(result, Err(VisualLineError::Extension(_))));
    assert_eq!(
        *events.borrow(),
        [
            "start first",
            "start second",
            "finish first",
            "finish second"
        ]
    );
    events.borrow_mut().clear();
    let invalid = TestGenerator::at(0, 2, "too long", events.clone());
    let result = VisualLineBuilder::generate(
        TextSnapshot::from_text("a"),
        1,
        properties(),
        &[Rc::new(invalid).into()],
        VisualLineBuildOptions::default(),
    );
    assert!(matches!(
        result,
        Err(VisualLineError::Document(
            DocumentError::InvalidRange { .. }
        ))
    ));
    assert_eq!(
        events.borrow().last().map(String::as_str),
        Some("finish too long")
    );
}

#[test]
fn invalid_generator_interest_is_reported_instead_of_repeating_an_offset() {
    let _scope = crate::tests::test_scope();
    let mut generator = TestGenerator::at(0, 0, "insert", Rc::default());
    generator.invalid_interest = true;
    let result = VisualLineBuilder::generate(
        TextSnapshot::from_text("a"),
        1,
        properties(),
        &[Rc::new(generator).into()],
        VisualLineBuildOptions::default(),
    );
    assert!(matches!(
        result,
        Err(VisualLineError::GeneratorInterestBeforeStart {
            requested: 1,
            returned: 0
        })
    ));
}

#[test]
fn required_generators_still_run_on_long_lines() {
    let _scope = crate::tests::test_scope();
    let text = format!("{}\t", "a".repeat(VISUAL_LINE_LENGTH_LIMIT + 1));
    let events = Rc::new(RefCell::new(Vec::new()));
    let optional = Rc::new(TestGenerator::at(0, 1, "optional", events.clone()));
    let tabs = Rc::new(crate::tab_element_generator::TabElementGenerator::new(
        4, false,
    ));
    let line = builder(&text, &[optional.into(), tabs.into()])
        .finish()
        .unwrap();
    assert_eq!(line.document_length(), VISUAL_LINE_LENGTH_LIMIT + 2);
    assert_eq!(line.visual_length(), VISUAL_LINE_LENGTH_LIMIT + 4);
    assert_eq!(*events.borrow(), ["start optional", "finish optional"]);
    assert!(line
        .elements()
        .last()
        .unwrap()
        .as_any()
        .is::<crate::tab_element_generator::TabTextElement>());
}

#[test]
fn generated_elements_can_span_lines_but_cannot_end_inside_crlf() {
    let _scope = crate::tests::test_scope();
    let line = builder("a\r\nb", &[one_generator(0, 3)]).finish().unwrap();
    assert_eq!(line.first_document_line().line_number, 1);
    assert_eq!(line.last_document_line().line_number, 2);
    assert_eq!((line.document_length(), line.visual_length()), (4, 2));
    assert_eq!(line.visual_offset(2), 0);
    assert!(!line.is_document_offset_visible(1));
    assert!(line.is_document_offset_visible(3));
    let result = VisualLineBuilder::generate(
        TextSnapshot::from_text("a\r\nb"),
        1,
        properties(),
        &[one_generator(0, 2)],
        VisualLineBuildOptions::default(),
    );
    assert!(matches!(
        result,
        Err(VisualLineError::ElementEndsInDelimiter { offset: 2 })
    ));
}

#[test]
fn text_transformations_split_styles_and_replacements_preserve_document_length() {
    let _scope = crate::tests::test_scope();
    let mut line = builder("abcde", &[]);
    line.change_document_part(1, 4, |element| {
        element
            .data_mut()
            .text_run_properties_mut()
            .set_font_rendering_em_size(24.0);
    })
    .unwrap();
    let sizes: Vec<_> = line
        .elements()
        .iter()
        .map(|element| {
            (
                element.data().relative_text_offset(),
                element.data().document_length(),
                element
                    .data()
                    .text_run_properties()
                    .font_rendering_em_size(),
            )
        })
        .collect();
    assert_eq!(sizes, [(0, 1, 12.0), (1, 3, 24.0), (4, 1, 12.0)]);
    let result = line.replace_elements(
        1..2,
        vec![Box::new(VisualLineText::new(2, &*properties()).unwrap())],
    );
    assert!(matches!(
        result,
        Err(VisualLineError::DocumentLengthChanged {
            expected: 3,
            actual: 2
        })
    ));
    assert_eq!(line.document_length(), 5);
    assert_eq!(line.elements().len(), 3);
    let line = line.finish().unwrap();
    let layout = layout(&line, f64::INFINITY, TextWrapping::NoWrap).unwrap();
    assert!((layout.width() - 48.0).abs() < 0.01);
}

#[test]
fn partial_atomic_visual_ranges_are_not_split_or_restyled() {
    let _scope = crate::tests::test_scope();
    let tabs = Rc::new(crate::tab_element_generator::TabElementGenerator::new(
        4, false,
    ));
    let mut line = builder("\tx", &[tabs.into()]);
    line.change_visual_elements(1, 3, |element| {
        element
            .data_mut()
            .text_run_properties_mut()
            .set_font_rendering_em_size(30.0);
    })
    .unwrap();
    assert_eq!(line.elements()[0].data().visual_length(), 4);
    assert_eq!(
        line.elements()[0]
            .data()
            .text_run_properties()
            .font_rendering_em_size(),
        12.0
    );
    let line = line.finish().unwrap();
    assert_eq!(line.get_relative_offset(3), 0);
    assert_eq!(line.document_offset(1), 0);
    assert_eq!(line.document_offset(2), 1);
}

#[test]
fn document_colorizers_skip_fully_hidden_lines_and_visit_visible_continuations() {
    let _scope = crate::tests::test_scope();
    let mut line = builder("a\nhidden\nz", &[one_generator(1, 8)]);
    let visited = Rc::new(RefCell::new(Vec::new()));
    let recorded = visited.clone();
    let colorizer = Rc::new(DocumentColorizingTransformer::new(move |_, line| {
        let document = line.document_line();
        recorded.borrow_mut().push(document.line_number);
        line.change_line_part(document.offset, document.end_offset(), |element| {
            element
                .data_mut()
                .text_run_properties_mut()
                .set_font_rendering_em_size(15.0);
        })
    }));
    line.run_transformers(&[colorizer.into()]).unwrap();
    assert_eq!(*visited.borrow(), [1, 3]);
    assert_eq!(
        line.source_lines()
            .unwrap()
            .iter()
            .map(|line| line.line_number)
            .collect::<Vec<_>>(),
        [1, 3]
    );
}

#[test]
fn recursive_colorizers_fail_and_reset_their_lifetime_guard() {
    let _scope = crate::tests::test_scope();
    let holder: Rc<RefCell<Option<Rc<ColorizingTransformer>>>> = Rc::new(RefCell::new(None));
    let nested = holder.clone();
    let recurse = Rc::new(Cell::new(true));
    let condition = recurse.clone();
    let transformer = Rc::new(ColorizingTransformer::new(move |context, line| {
        if condition.replace(false) {
            nested.borrow().as_ref().unwrap().transform(context, line)?;
        }
        Ok(())
    }));
    *holder.borrow_mut() = Some(transformer.clone());
    let mut line = builder("abc", &[]);
    assert_eq!(
        line.run_transformers(&[transformer.clone().into()]),
        Err(VisualLineError::RecursiveTransformation)
    );
    line.run_transformers(&[transformer.into()]).unwrap();
    holder.borrow_mut().take();
}

struct CheckedRunElement {
    data: VisualLineElementData,
    length: i32,
    size: Size,
    preceding: usize,
    bad_caret: bool,
}
impl VisualLineElement for CheckedRunElement {
    fn data(&self) -> &VisualLineElementData {
        &self.data
    }
    fn data_mut(&mut self) -> &mut VisualLineElementData {
        &mut self.data
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn create_text_run(
        &self,
        _column: usize,
        _context: &dyn ITextRunConstructionContext,
    ) -> Result<Rc<dyn TextRun>, VisualLineError> {
        Ok(Rc::new(
            ferroui_base::media::text_formatting::testing::CustomDrawableRun::with_size(
                self.size,
                10.0,
                self.data.text_run_properties().snapshot(),
            )
            .with_length(self.length),
        ))
    }
    fn get_preceding_text(
        &self,
        _: usize,
        _: &dyn ITextRunConstructionContext,
    ) -> Result<ReadOnlyMemory<u16>, VisualLineError> {
        Ok(ReadOnlyMemory::from_vec(vec![97; self.preceding]))
    }
    fn get_next_caret_position(
        &self,
        column: isize,
        direction: LogicalDirection,
        mode: CaretPositioningMode,
        _: &dyn ITextRunConstructionContext,
    ) -> Result<Option<usize>, VisualLineError> {
        Ok(if self.bad_caret {
            Some(column.max(0) as usize)
        } else {
            crate::visual_line_element::atomic_caret_stop(&self.data, column, direction, mode)
        })
    }
}
fn checked_line(length: i32, size: Size, preceding: usize, bad_caret: bool) -> Rc<VisualLine> {
    let mut generator = TestGenerator::at(0, 1, "checked", Rc::default());
    generator.factory = Rc::new(move |context| {
        Ok(Box::new(CheckedRunElement {
            data: VisualLineElementData::new(1, 1, &**context.global_text_run_properties())?,
            length,
            size,
            preceding,
            bad_caret,
        }))
    });
    builder("a", &[Rc::new(generator).into()]).finish().unwrap()
}

#[test]
fn invalid_extension_runs_return_diagnostics_before_shaping() {
    let _scope = crate::tests::test_scope();
    for length in [0, -1, 2] {
        let line = checked_line(length, Size::new(10.0, 10.0), 0, false);
        assert!(matches!(
            layout(&line, 100.0, TextWrapping::NoWrap),
            Err(VisualLineError::InvalidRunLength { .. })
        ));
    }
    let line = checked_line(1, Size::new(f64::NAN, 10.0), 0, false);
    assert!(matches!(
        layout(&line, 100.0, TextWrapping::NoWrap),
        Err(VisualLineError::InvalidRunMetrics { visual_column: 0 })
    ));
}

#[test]
fn invalid_preceding_text_and_caret_stops_are_rejected() {
    let _scope = crate::tests::test_scope();
    let line = checked_line(1, Size::new(10.0, 10.0), 2, true);
    assert!(matches!(
        line.text_source().get_preceding_text(1),
        Err(VisualLineError::InvalidPrecedingText {
            visual_column: 1,
            length: 2,
            remaining: 1
        })
    ));
    assert!(matches!(
        line.get_next_caret_position(
            0,
            LogicalDirection::Forward,
            CaretPositioningMode::Normal,
            false
        ),
        Err(VisualLineError::InvalidCaretPosition {
            requested: 0,
            returned: 0
        })
    ));
}

#[test]
fn maximum_formatter_columns_are_checked_without_allocating_text() {
    let _scope = crate::tests::test_scope();
    let mut generator = TestGenerator::at(0, 0, "large", Rc::default());
    generator.factory = Rc::new(|context| {
        Ok(Box::new(CheckedRunElement {
            data: VisualLineElementData::new(
                i32::MAX as usize,
                0,
                &**context.global_text_run_properties(),
            )?,
            length: 1,
            size: Size::new(1.0, 1.0),
            preceding: 0,
            bad_caret: false,
        }))
    });
    let generator = Rc::new(generator);
    let line = builder("", &[generator.clone().into()]).finish().unwrap();
    assert_eq!(line.visual_length(), i32::MAX as usize);
    assert_eq!(
        line.get_next_caret_position(
            i32::MAX as isize,
            LogicalDirection::Backward,
            CaretPositioningMode::Normal,
            false,
        )
        .unwrap(),
        Some(0)
    );
    let result = VisualLineBuilder::generate(
        TextSnapshot::empty(),
        1,
        properties(),
        &[generator.into()],
        VisualLineBuildOptions {
            end_of_line: Some(EndOfLineMarkerOptions::default()),
            ..Default::default()
        },
    );
    assert!(matches!(result, Err(VisualLineError::OffsetOverflow)));
}

struct PaintedRun {
    properties: Rc<dyn TextRunProperties>,
    drawn: Rc<Cell<usize>>,
}
impl TextRun for PaintedRun {
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
impl DrawableTextRun for PaintedRun {
    fn size(&self) -> Size {
        Size::new(27.0, 30.0)
    }
    fn baseline(&self) -> f64 {
        22.0
    }
    fn draw(&self, sink: &mut dyn ITextDrawingSink, origin: Point) {
        self.drawn.set(self.drawn.get() + 1);
        sink.draw_rectangle(None, None, Rect::new(origin.x, origin.y, 27.0, 30.0));
    }
}

#[test]
fn custom_inline_drawables_participate_in_native_metrics_drawing_and_hit_testing() {
    let _scope = crate::tests::test_scope();
    let drawn = Rc::new(Cell::new(0));
    let marker = drawn.clone();
    let mut generator = TestGenerator::at(1, 1, "drawable", Rc::default());
    generator.factory = Rc::new(move |context| {
        let run = Rc::new(PaintedRun {
            properties: context.global_text_run_properties().clone(),
            drawn: marker.clone(),
        });
        Ok(Box::new(FormattedTextElement::from_content(
            FormattedTextContent::Drawable(run),
            1,
            &**context.global_text_run_properties(),
        )?))
    });
    let line = builder("axb", &[Rc::new(generator).into()])
        .finish()
        .unwrap();
    let formatted = layout(&line, f64::INFINITY, TextWrapping::NoWrap).unwrap();
    assert!((formatted.width() - 39.0).abs() < 0.01);
    assert!(formatted.height() >= 30.0);
    assert!((formatted.hit_test_text_position(2).x - 33.0).abs() < 0.01);
    let mut sink = ferroui_base::media::text_formatting::testing::RecordingDrawingSink::new();
    formatted.draw(&mut sink, Point::new(0.0, 0.0));
    assert_eq!(drawn.get(), 1);
    assert!(sink.calls().iter().any(|call| matches!(call,
        ferroui_base::media::text_formatting::testing::DrawCall::Rectangle(rect) if rect.width == 27.0 && rect.height == 30.0)));
    assert_eq!(sink.transform_depth(), 0);
    let wrapped = layout(&line, 33.0, TextWrapping::Wrap).unwrap();
    assert!(wrapped.text_lines().len() >= 2);
}

#[test]
fn tab_markers_do_not_change_tab_width_even_with_a_wide_custom_glyph() {
    let _scope = crate::tests::test_scope();
    let mut generator = crate::tab_element_generator::TabElementGenerator::new(4, true);
    generator.glyph = "a much wider tab marker".into();
    let line = builder("a\tb", &[Rc::new(generator).into()])
        .finish()
        .unwrap();
    assert_eq!((line.visual_length(), line.document_length()), (5, 3));
    let formatted = layout(&line, f64::INFINITY, TextWrapping::NoWrap).unwrap();
    assert!((formatted.width() - 30.0).abs() < 0.01);
    assert_eq!(
        line.get_next_caret_position(
            1,
            LogicalDirection::Forward,
            CaretPositioningMode::Normal,
            false
        )
        .unwrap(),
        Some(4)
    );
    assert_eq!(
        line.get_next_caret_position(
            4,
            LogicalDirection::Backward,
            CaretPositioningMode::Normal,
            false
        )
        .unwrap(),
        Some(1)
    );
}

#[test]
fn control_character_display_columns_affect_following_tab_stops() {
    let _scope = crate::tests::test_scope();
    let characters =
        Rc::new(crate::single_character_element_generator::SingleCharacterElementGenerator::new());
    let tabs = Rc::new(crate::tab_element_generator::TabElementGenerator::new(
        4, false,
    ));
    let line = builder("\0\tx", &[characters.into(), tabs.into()])
        .finish()
        .unwrap();
    assert_eq!(line.elements()[0].display_column_length(), 3); // NUL
    assert_eq!(line.elements()[1].data().visual_length(), 1);
    assert_eq!(line.visual_length(), 3);
    let formatted = layout(&line, f64::INFINITY, TextWrapping::NoWrap).unwrap();
    assert!((formatted.width() - 33.0).abs() < 0.01); // NUL 18 + box padding 3 + tab 6 + x 6
}

#[test]
fn word_and_grapheme_navigation_use_document_text_across_style_boundaries() {
    let _scope = crate::tests::test_scope();
    let mut line = builder("a😀e\u{301} word", &[]);
    line.change_document_part(1, 3, |element| {
        element
            .data_mut()
            .text_run_properties_mut()
            .set_font_rendering_em_size(15.0);
    })
    .unwrap();
    let line = line.finish().unwrap();
    assert_eq!(
        line.get_next_caret_position(
            1,
            LogicalDirection::Forward,
            CaretPositioningMode::Normal,
            false
        )
        .unwrap(),
        Some(3)
    );
    assert_eq!(
        line.get_next_caret_position(
            3,
            LogicalDirection::Forward,
            CaretPositioningMode::Normal,
            false
        )
        .unwrap(),
        Some(5)
    );
    assert_eq!(
        line.get_next_caret_position(
            5,
            LogicalDirection::Forward,
            CaretPositioningMode::WordStart,
            false
        )
        .unwrap(),
        Some(6)
    );
    assert_eq!(
        line.get_next_caret_position(
            10,
            LogicalDirection::Forward,
            CaretPositioningMode::Normal,
            true
        )
        .unwrap(),
        Some(11)
    );
}

#[test]
fn folding_generator_merges_overlapping_regions_and_uses_a_stable_document_version() {
    let _scope = crate::tests::test_scope();
    let document = TextDocument::from_text("a\nhidden\nz\tq");
    let manager = Rc::new(ferroui_edit_core::folding::FoldingManager::new(
        document.clone(),
    ));
    let first = manager.create_folding(1, 8).unwrap();
    first.set_title(Some("...".into()));
    first.set_is_folded(true);
    let second = manager.create_folding(5, 9).unwrap();
    second.set_is_folded(true);
    let fold = Rc::new(crate::folding_element_generator::FoldingElementGenerator::new(manager));
    let tabs = Rc::new(crate::tab_element_generator::TabElementGenerator::new(
        4, false,
    ));
    let snapshot = document.create_snapshot();
    let line = VisualLineBuilder::generate(
        snapshot.clone(),
        1,
        properties(),
        &[fold.clone().into(), tabs.into()],
        VisualLineBuildOptions::default(),
    )
    .unwrap()
    .finish()
    .unwrap();
    assert_eq!(line.last_document_line().line_number, 3);
    assert_eq!(line.elements()[1].data().document_length(), 8);
    assert_eq!(line.elements()[3].data().visual_length(), 3); // a + ... + z is display column 5
    assert_eq!(line.visual_offset(9), 2);
    document.insert(0, "x").unwrap();
    let result = VisualLineBuilder::generate(
        snapshot,
        1,
        properties(),
        &[fold.into()],
        VisualLineBuildOptions::default(),
    );
    assert!(matches!(
        result,
        Err(VisualLineError::Document(DocumentError::InvalidVersion))
    ));
}

#[test]
fn preedit_projection_keeps_virtual_padding_unadorned_and_document_unchanged() {
    let _scope = crate::tests::test_scope();
    let document = TextDocument::from_text("abc");
    let generator = Rc::new(
        crate::preedit_element_generator::PreeditElementGenerator::new(3, 3, "😀x", 2).unwrap(),
    );
    let line = VisualLineBuilder::generate(
        document.create_snapshot(),
        1,
        properties(),
        &[generator.into()],
        VisualLineBuildOptions::default(),
    )
    .unwrap()
    .finish()
    .unwrap();
    assert_eq!(document.text(), "abc");
    assert_eq!(line.document_length(), 3);
    assert_eq!(line.visual_length(), 8);
    assert_eq!(line.composition_range(), Some(5..8));
    let source = line.text_source();
    let padding = source.get_text_run(3).unwrap();
    assert_eq!(padding.text().to_string_lossy(), "  ");
    assert!(padding.properties().unwrap().text_decorations().is_none());
    let text = source.get_text_run(5).unwrap();
    assert_eq!(text.text().to_string_lossy(), "😀x");
    assert_eq!(
        text.properties().unwrap().text_decorations().unwrap().len(),
        1
    );
    let formatted = layout(&line, f64::INFINITY, TextWrapping::NoWrap).unwrap();
    assert!(formatted.hit_test_text_position(5).x > formatted.hit_test_text_position(3).x);
    assert!(matches!(
        crate::preedit_element_generator::PreeditElementGenerator::new(0, 0, "", 0),
        Err(VisualLineError::InvalidVisualLength)
    ));
}

#[test]
fn line_end_markers_have_one_visual_position_without_consuming_document_text() {
    let _scope = crate::tests::test_scope();
    for (text, expected) in [("a\r\n", "¶"), ("a\r", "\\r"), ("a\n", "\\n"), ("a", "")] {
        let line = VisualLineBuilder::generate(
            TextSnapshot::from_text(text),
            1,
            properties(),
            &[],
            VisualLineBuildOptions {
                end_of_line: Some(EndOfLineMarkerOptions::default()),
                ..Default::default()
            },
        )
        .unwrap()
        .finish()
        .unwrap();
        assert_eq!(line.visual_length(), 1);
        assert_eq!(line.document_length(), 1);
        assert_eq!(
            line.visual_length_with_end_of_line_marker(),
            if expected.is_empty() { 1 } else { 2 }
        );
        let source = line.text_source();
        let marker = source.get_text_run(1).unwrap();
        assert_eq!(marker.length(), 1);
        let mut sink = ferroui_base::media::text_formatting::testing::RecordingDrawingSink::new();
        marker
            .as_drawable()
            .unwrap()
            .draw(&mut sink, Point::new(0.0, 0.0));
        let actual: String = sink
            .calls()
            .iter()
            .filter_map(|call| match call {
                ferroui_base::media::text_formatting::testing::DrawCall::GlyphRun {
                    text, ..
                } => Some(text.clone()),
                _ => None,
            })
            .collect();
        if expected.is_empty() {
            // The native formatter retains font metrics with a zero-advance
            // space glyph for an empty line.
            assert_eq!(marker.as_drawable().unwrap().size().width, 0.0);
            assert!(actual.chars().all(char::is_whitespace));
        } else {
            assert_eq!(actual, expected);
        }
        assert!(source
            .get_text_run(2)
            .unwrap()
            .as_text_end_of_line()
            .is_some());
        assert!(layout(&line, f64::INFINITY, TextWrapping::NoWrap).is_ok());
    }
}

#[test]
fn links_keep_unicode_offsets_custom_patterns_and_uri_identity_after_styling() {
    let _scope = crate::tests::test_scope();
    let web = Rc::new(crate::link_element_generator::LinkElementGenerator::new());
    let mail = Rc::new(crate::link_element_generator::MailLinkElementGenerator::new());
    let mut line = builder(
        "😀 https://example.org/path. a+b@example.org",
        &[web.into(), mail.into()],
    );
    let links: Vec<_> = line
        .elements()
        .iter()
        .filter_map(|element| {
            element
                .as_any()
                .downcast_ref::<crate::visual_line_link_text::VisualLineLinkText>()
        })
        .map(|link| {
            (
                link.data().relative_text_offset(),
                link.navigate_uri
                    .as_ref()
                    .unwrap()
                    .original_string()
                    .to_owned(),
            )
        })
        .collect();
    assert_eq!(
        links,
        [
            (3, "https://example.org/path".into()),
            (29, "mailto:a+b@example.org".into())
        ]
    );
    line.change_document_part(5, 9, |element| {
        element
            .data_mut()
            .text_run_properties_mut()
            .set_font_rendering_em_size(18.0);
    })
    .unwrap();
    let matching_links: Vec<_> = line
        .elements()
        .iter()
        .filter_map(|element| {
            element
                .as_any()
                .downcast_ref::<crate::visual_line_link_text::VisualLineLinkText>()
        })
        .filter(|link| {
            link.navigate_uri.as_ref().unwrap().original_string() == "https://example.org/path"
        })
        .collect();
    assert_eq!(matching_links.len(), 3);
    assert!(matching_links
        .iter()
        .all(|link| link.require_control_modifier_for_click));
    let custom = crate::link_element_generator::LinkElementGenerator::with_pattern(
        r"(?<=issue:)[0-9]+",
        |matched| {
            ferroui_base::utilities::Uri::try_create(
                &format!("https://example.org/{}", matched.matched_text()),
                ferroui_base::utilities::UriKind::Absolute,
            )
        },
    )
    .unwrap();
    let line = builder("issue:42", &[Rc::new(custom).into()])
        .finish()
        .unwrap();
    assert_eq!(
        line.elements()
            .last()
            .unwrap()
            .data()
            .relative_text_offset(),
        6
    );
}

#[test]
fn hyperlink_click_routes_the_native_uri_event_and_respects_modifiers() {
    use crate::visual_line_link_text::{OpenUriRoutedEventArgs, VisualLineLinkText};
    use ferroui_base::input::{
        KeyModifiers, Pointer, PointerPointProperties, PointerPressedEventArgs, PointerType,
        PointerUpdateKind, RawInputModifiers,
    };
    use ferroui_base::utilities::{Uri, UriKind};
    let _scope = crate::tests::test_scope();
    let editor = crate::TextEditor::new();
    let area = editor.text_area();
    let received = Rc::new(RefCell::new(Vec::new()));
    let recorded = received.clone();
    area.add_handler(
        &VisualLineLinkText::open_uri_event(),
        move |_, event: &OpenUriRoutedEventArgs| {
            recorded.borrow_mut().push((
                event.uri.original_string().to_owned(),
                event.target_name.clone(),
            ));
            event.set_handled(true);
        },
    );
    let mut link = VisualLineLinkText::new(1, &*properties()).unwrap();
    link.navigate_uri = Uri::try_create("https://example.org", UriKind::Absolute);
    link.target_name = Some("details".into());
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let press = |modifiers| {
        PointerPressedEventArgs::new(
            area.clone(),
            pointer.clone(),
            &area,
            Point::new(0.0, 0.0),
            1,
            PointerPointProperties::new(
                RawInputModifiers::LEFT_MOUSE_BUTTON,
                PointerUpdateKind::LeftButtonPressed,
            ),
            modifiers,
            1,
        )
    };
    let unmodified = press(KeyModifiers::NONE);
    link.on_pointer_pressed(&area, &unmodified);
    assert!(!unmodified.handled());
    assert!(received.borrow().is_empty());
    let modified = press(KeyModifiers::CONTROL);
    link.on_pointer_pressed(&area, &modified);
    assert!(modified.handled());
    assert_eq!(
        *received.borrow(),
        [("https://example.org".into(), Some("details".into()))]
    );
    link.on_pointer_pressed(&area, &modified);
    assert_eq!(received.borrow().len(), 1);
}
