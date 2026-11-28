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

fn add_inline_object(
    area: &TextArea,
    offset: usize,
    length: usize,
    child: Ref<Control>,
) -> VisualLineElementGeneratorRef {
    let mut generator = ProbeGenerator::new("inline child", Rc::new(RefCell::new(Vec::new())));
    generator.offset = offset;
    generator.factory = Some(Rc::new(move |context| {
        Ok(Box::new(crate::InlineObjectElement::new(
            length, child.clone(), &**context.global_text_run_properties(),
        )?))
    }));
    let generator: VisualLineElementGeneratorRef = Rc::new(generator).into();
    area.element_generators().add(generator.clone());
    generator
}

fn layout_inline_test(area: &TextArea) {
    area.measure(Size::new(360.0, 180.0));
    area.arrange(Rect::new(0.0, 0.0, 360.0, 180.0));
}

#[test]
fn line_number_width_uses_the_shaped_digit_and_tracks_document_growth() {
    let _scope = test_scope();
    let editor = laid_out_editor("one");
    editor.set_show_line_numbers(true);
    let area = editor.text_area();
    let margin = area.line_number_margin().unwrap();
    let advance = margin.number_layout("9", None).width();

    assert!((area.gutter_width() - (18.0 + 2.0 * advance)).abs() < 0.01);
    editor.set_line_numbers_min_width_in_digits(4);
    assert!((area.gutter_width() - (18.0 + 4.0 * advance)).abs() < 0.01);
    editor.set_line_numbers_min_width_in_digits(0);
    assert!((area.gutter_width() - (18.0 + advance)).abs() < 0.01);
    editor.document().insert(0, &"\n".repeat(99)).unwrap();
    assert!((area.gutter_width() - (18.0 + 3.0 * advance)).abs() < 0.01);
    editor.set_show_line_numbers(false);
    assert_eq!(area.gutter_width(), 6.0);
}

#[test]
fn editor_typeface_changes_rebuild_visual_lines_and_reach_line_numbers() {
    use ferroui_base::media::text_formatting::TextRunProperties;
    use ferroui_base::media::{FontStretch, FontStyle, FontWeight};
    use ferroui_controls::TextBlock;

    let _scope = test_scope();
    let editor = laid_out_editor("plain text\nsecond line");
    editor.set_show_line_numbers(true);
    let area = editor.text_area();
    let first = area.get_or_construct_visual_line(1).unwrap();

    editor.set_value(TextBlock::font_style_property(), FontStyle::Italic);
    editor.set_value(TextBlock::font_weight_property(), FontWeight::Bold);
    let styled = area.get_or_construct_visual_line(1).unwrap();
    assert!(!Rc::ptr_eq(&first, &styled));
    let face = styled.elements()[0].data().text_run_properties().typeface();
    assert_eq!(face.style(), FontStyle::Italic);
    assert_eq!(face.weight(), FontWeight::Bold);

    editor.set_value(TextBlock::font_stretch_property(), FontStretch::Condensed);
    let condensed = area.get_or_construct_visual_line(1).unwrap();
    assert!(!Rc::ptr_eq(&styled, &condensed));
    let face = condensed.elements()[0].data().text_run_properties().typeface();
    assert_eq!(face.stretch(), FontStretch::Condensed);

    let digit = area.line_number_margin().unwrap().number_layout("1", None);
    let digit_runs = digit.text_lines()[0].text_runs();
    let digit_face = digit_runs[0].properties().unwrap().typeface();
    assert_eq!(digit_face, face);
}

#[test]
fn raw_tabs_from_extensions_use_editor_indentation_metrics() {
    let _scope = test_scope();
    let editor = laid_out_editor("a\tb\na\tb");
    let area = editor.text_area();
    let mut generator = ProbeGenerator::new("raw tab", Rc::new(RefCell::new(Vec::new())));
    generator.offset = 1;
    generator.factory = Some(Rc::new(|context| {
        Ok(Box::new(crate::VisualLineText::new(
            1,
            &**context.global_text_run_properties(),
        )?))
    }));
    area.element_generators().add(Rc::new(generator).into());

    for indentation_size in [2, 7] {
        let mut options = editor.options();
        options.indentation_size = indentation_size;
        editor.set_options(options);
        let generated = area.try_line_layout(0).unwrap();
        assert_eq!(generated.display.visual_length(), 3);
        let raw_tab = generated.layout.hit_test_text_position(2).x
            - generated.layout.hit_test_text_position(1).x;
        assert!((raw_tab - indentation_size as f64 * area.wide_space_width()).abs() < 0.01);

        let ordinary = area.try_line_layout(1).unwrap();
        assert_eq!(ordinary.display.visual_length(), indentation_size + 1);
        let expanded_tab = ordinary.layout.hit_test_text_position(indentation_size as i32).x
            - ordinary.layout.hit_test_text_position(1).x;
        let expected = (indentation_size - 1) as f64 * area.wide_space_width();
        assert!((expanded_tab - expected).abs() < 0.01);
    }
}

#[test]
fn wide_space_uses_the_shaped_x_advance() {
    use ferroui_base::media::text_formatting::testing::{TestFont, TextTestScope};
    use ferroui_base::media::text_formatting::TextLayoutOptions;
    use ferroui_base::media::FontFamily;

    let _scope = TextTestScope::with_fonts(vec![
        TestFont::new("Narrow Space").with_ranges(&[(0x20, 0x20)]).with_advance(220),
        TestFont::new("Wider Letters").with_ranges(&[(0x61, 0x7a)]).with_advance(600),
    ]);
    let editor = TextEditor::new();
    editor.set_font_family(FontFamily::new("Narrow Space,Wider Letters"));
    let area = editor.text_area();
    let measure = |text| {
        TextLayout::new(
            text,
            editor.typeface(),
            TextLayoutOptions { font_size: editor.font_size(), ..Default::default() },
        )
        .width_including_trailing_whitespace()
    };
    let wide = measure("x");
    assert!(wide > measure(" "));
    assert!((area.wide_space_width() - wide).abs() < 0.01);
}

#[test]
fn new_visual_lines_snapshot_the_active_text_locale() {
    use crate::{FormattedTextContent, FormattedTextRun};
    use ferroui_base::media::text_formatting::ITextSource;
    use ferroui_base::utilities::{locale, set_current_locale, CultureInfo};

    struct RestoreCulture(CultureInfo);
    impl Drop for RestoreCulture {
        fn drop(&mut self) {
            CultureInfo::set_current_formatting(self.0.clone());
        }
    }

    let _restore = RestoreCulture(CultureInfo::current_formatting());
    let _scope = test_scope();
    let turkish = locale!("tr-TR");
    let azeri = locale!("az-Latn-AZ");
    set_current_locale(turkish.clone());
    let editor = laid_out_editor("one\ntwo\nthree");
    let area = editor.text_area();
    let mut options = editor.options();
    options.show_end_of_line = true;
    editor.set_options(options);

    let marker = |line: &Rc<crate::VisualLine>| {
        let source = line.text_source();
        let run = source.get_text_run(line.visual_length() as i32).unwrap();
        let FormattedTextContent::Layout(layout) =
            run.downcast_ref::<FormattedTextRun>().unwrap().content()
        else {
            panic!("expected a formatted line-ending marker");
        };
        layout.clone()
    };
    let first = area.get_or_construct_visual_line(1).unwrap();
    assert_eq!(first.context().global_text_run_properties().culture_info(), Some(&turkish));
    let old_marker = marker(&first);

    set_current_locale(azeri.clone());
    area.redraw_range(5..6).unwrap();
    let second = area.get_or_construct_visual_line(2).unwrap();
    let run = second.text_source().get_text_run(0).unwrap();
    assert_eq!(run.properties().unwrap().culture_info(), Some(&azeri));
    let next_marker = marker(&second);
    assert!(!Rc::ptr_eq(&old_marker, &next_marker));
    assert!(Rc::ptr_eq(&old_marker, &marker(&first)));
    let marker_runs = next_marker.text_lines()[0].text_runs();
    assert_eq!(marker_runs[0].properties().unwrap().culture_info(), Some(&azeri));
}

#[test]
fn empty_line_geometry_uses_the_default_selection_marker_width() {
    let _scope = test_scope();
    let editor = laid_out_editor("one\n\nthree");
    let segment = ferroui_edit_core::document::SimpleSegment::new(4, 0);
    let rectangles = editor.text_area().segment_rectangles(&segment, false).unwrap();
    assert_eq!(rectangles.len(), 1);
    assert_eq!(rectangles[0].width, 1.0);
}

#[test]
fn end_of_line_markers_reuse_native_layouts_and_follow_font_changes() {
    use crate::{FormattedTextContent, FormattedTextRun};
    use ferroui_base::media::text_formatting::ITextSource;

    let _scope = test_scope();
    let editor = laid_out_editor("one\ntwo\r\nthree\nfour");
    let area = editor.text_area();
    let mut options = editor.options();
    options.show_end_of_line = true;
    editor.set_options(options);
    let end_marker = |number| {
        let line = area.get_or_construct_visual_line(number).unwrap();
        let run = line
            .text_source()
            .get_text_run(line.visual_length() as i32)
            .unwrap();
        let FormattedTextContent::Layout(layout) =
            run.downcast_ref::<FormattedTextRun>().unwrap().content()
        else {
            panic!("expected a native end-of-line layout");
        };
        layout.clone()
    };
    let lf = end_marker(1);
    assert!(Rc::ptr_eq(&lf, &end_marker(3)));
    assert!(!Rc::ptr_eq(&lf, &end_marker(2)));
    editor.set_font_size(editor.font_size() + 6.0);
    let resized = end_marker(1);
    assert!(!Rc::ptr_eq(&lf, &resized));
    assert!(resized.width() > lf.width());
    assert!(Rc::ptr_eq(&resized, &end_marker(3)));
    area.set_non_printable_character_brush(Some(Rc::new(
        ImmutableSolidColorBrush::from_uint32(0xff123456),
    )));
    assert!(!Rc::ptr_eq(&resized, &end_marker(1)));
}

#[test]
fn live_whitespace_glyphs_and_brush_follow_editor_options() {
    use crate::{FormattedTextContent, FormattedTextRun};
    use ferroui_base::media::text_formatting::ITextSource;

    let _scope = test_scope();
    let text = " \t\n \t\r\n \t\r";
    let editor = laid_out_editor(text);
    let area = editor.text_area();
    let mut options = editor.options();
    options.show_spaces = true;
    options.space_glyph = "s".into();
    options.show_tabs = true;
    options.tab_glyph = "t".into();
    options.show_end_of_line = true;
    options.end_of_line_lf_glyph = "LF".into();
    options.end_of_line_crlf_glyph = "CRLF".into();
    options.end_of_line_cr_glyph = "CR".into();
    editor.set_options(options.clone());

    let gutter: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::from_uint32(0xff675432));
    let marker: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::from_uint32(0xff123456));
    editor.set_line_numbers_foreground(Some(gutter.clone()));
    area.set_non_printable_character_brush(Some(marker.clone()));
    let glyph = |line_number, column: Option<usize>| {
        let line = area.get_or_construct_visual_line(line_number).unwrap();
        let source = line.text_source();
        let run = source
            .get_text_run(column.unwrap_or(line.visual_length()) as i32)
            .unwrap();
        let FormattedTextContent::Layout(layout) =
            run.downcast_ref::<FormattedTextRun>().unwrap().content()
        else {
            panic!("expected a native marker layout");
        };
        let mut text = Vec::new();
        for line in layout.text_lines() {
            for run in line.text_runs().iter() {
                let glyph = run.text();
                if !glyph.is_empty() {
                    let fill = run.properties().unwrap().foreground_brush().unwrap();
                    assert!(Rc::ptr_eq(fill, &marker));
                    text.extend_from_slice(glyph.span());
                }
            }
        }
        String::from_utf16(&text).unwrap()
    };
    assert_eq!(glyph(1, Some(0)), "s");
    assert_eq!(glyph(1, Some(1)), "t");
    assert_eq!(glyph(1, None), "LF");
    assert_eq!(glyph(2, None), "CRLF");
    assert_eq!(glyph(3, None), "CR");
    assert!(glyph(4, None).chars().all(char::is_whitespace));

    options.end_of_line_lf_glyph = "new".into();
    editor.set_options(options);
    assert_eq!(glyph(1, None), "new");
    assert_eq!(editor.text(), text);
    assert!(Rc::ptr_eq(&editor.line_numbers_foreground().unwrap(), &gutter));
}

#[test]
fn mouse_hover_uses_text_and_link_cursors_and_respects_explicit_cursor() {
    use ferroui_base::input::{MouseButton, Pointer, PointerPointProperties, RawInputModifiers};
    use ferroui_controls::Window;

    let _scope = test_scope();
    let editor = laid_out_editor("https://example.org\n");
    editor.set_show_line_numbers(true);
    let area = editor.text_area();
    let window = Window::new();
    window.set_content(Some(Control::boxed(editor)));
    window.show();
    area.measure(Size::new(320.0, 200.0));
    area.arrange(Rect::new(0.0, 0.0, 320.0, 200.0));
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let raise_pointer = |event, point, modifiers| {
        let point = area.translate_point(point, &window).unwrap();
        area.raise_event(&PointerEventArgs::new(
            Some(event),
            area.clone(),
            pointer.clone(),
            Some(&window),
            point,
            1,
            PointerPointProperties::default(),
            modifiers,
        ));
    };
    let cursor = || area.cursor().map(|cursor| cursor.to_string());
    let link = Point::new(area.gutter_width() + 3.0, area.line_height() / 2.0);
    let whitespace = Point::new(230.0, 120.0);
    let enter = InputElement::pointer_entered_event();
    let moved = InputElement::pointer_moved_event();
    let exit = InputElement::pointer_exited_event();

    raise_pointer(enter, whitespace, KeyModifiers::NONE);
    assert_eq!(cursor().as_deref(), Some("Ibeam"));
    raise_pointer(moved, link, KeyModifiers::CONTROL);
    assert_eq!(cursor().as_deref(), Some("Hand"));
    raise_pointer(moved, link, KeyModifiers::NONE);
    assert_eq!(cursor().as_deref(), Some("Ibeam"));
    raise_pointer(moved, Point::new(3.0, link.y), KeyModifiers::NONE);
    assert_eq!(cursor(), None);

    let start = area.translate_point(link, &window).unwrap();
    let press = PointerPressedEventArgs::new(
        area.clone(), pointer.clone(), &window, start, 2,
        PointerPointProperties::new(RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::LeftButtonPressed),
        KeyModifiers::NONE, 1,
    );
    area.raise_event(&press);
    assert!(press.handled());
    raise_pointer(exit, Point::new(link.x, -5.0), KeyModifiers::NONE);
    assert_eq!(cursor(), None);
    raise_pointer(enter, link, KeyModifiers::CONTROL);
    assert_eq!(cursor().as_deref(), Some("Ibeam"));
    let release = PointerReleasedEventArgs::new(
        area.clone(), pointer.clone(), &window, start, 3,
        PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::LeftButtonReleased),
        KeyModifiers::NONE, MouseButton::Left,
    );
    area.raise_event(&release);
    assert!(release.handled());

    let custom = Cursor::new(StandardCursorType::Cross);
    area.set_cursor(Some(custom.clone()));
    raise_pointer(moved, whitespace, KeyModifiers::NONE);
    assert_eq!(area.cursor().as_ref(), Some(&custom));
    raise_pointer(exit, whitespace, KeyModifiers::NONE);
    assert_eq!(area.cursor().as_ref(), Some(&custom));
    area.clear_value(InputElement::cursor_property());
    assert_eq!(cursor(), None);
    window.close();
}

#[test]
fn stationary_mouse_cursor_follows_modifiers_and_scrolling() {
    use ferroui_base::input::raw::{RawPointerEventArgs, RawPointerEventType};
    use ferroui_base::input::{InputManager, MouseDevice, RawInputModifiers};
    use ferroui_controls::Window;
    use ferroui_controls::testing::TestServices;

    let _scope = crate::tests::test_scope_with_services(
        TestServices::styled_window().with_input_manager(Rc::new(InputManager::new())),
    );
    let editor = laid_out_editor(&format!("https://example.org\n{}", "plain text\n".repeat(12)));
    let area = editor.text_area();
    let window = Window::new();
    window.set_width(240.0);
    window.set_height(96.0);
    window.set_content(Some(Control::boxed(editor)));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);

    let link = Point::new(area.gutter_width() + 3.0, area.line_height() / 2.0);
    let position = area.translate_point(link, &window).unwrap();
    let emit = window.platform_impl().unwrap().input().unwrap();
    let mouse = MouseDevice::new();
    let move_to = |point| {
        emit(Rc::new(RawPointerEventArgs::new(
            mouse.clone(),
            1,
            window.input_root(),
            RawPointerEventType::Move,
            point,
            RawInputModifiers::NONE,
        )));
    };
    let set_control = |down| {
        let mut event = KeyEventArgs::new();
        event.key = Key::LeftCtrl;
        event.key_modifiers = if down { KeyModifiers::CONTROL } else { KeyModifiers::NONE };
        event.set_routed_event(Some(if down {
            InputElement::key_down_event()
        } else {
            InputElement::key_up_event()
        }));
        area.raise_event(&event);
        Dispatcher::ui_thread().run_jobs(None);
    };
    let cursor = || area.cursor().map(|cursor| cursor.to_string());

    move_to(position);
    assert_eq!(cursor().as_deref(), Some("Ibeam"));
    set_control(true);
    assert_eq!(cursor().as_deref(), Some("Hand"));
    set_control(false);
    assert_eq!(cursor().as_deref(), Some("Ibeam"));

    set_control(true);
    let second_line = area.heights.borrow().top(1);
    area.set_offset(Vector::new(0.0, second_line));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(cursor().as_deref(), Some("Ibeam"));
    area.set_offset(Vector::default());
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(cursor().as_deref(), Some("Hand"));

    move_to(Point::new(position.x, -1.0));
    set_control(false);
    assert_eq!(cursor(), None);
    window.close();
}

#[test]
fn line_number_drag_uses_wrapped_rows_including_the_final_crlf() {
    use ferroui_base::input::{
        IPointer, MouseButton, Pointer, PointerPointProperties, PointerType, RawInputModifiers,
    };
    use ferroui_controls::Window;

    let _scope = test_scope();
    let text = "alpha beta gamma delta epsilon zeta eta theta iota kappa\r\nnext";
    let editor = laid_out_editor(text);
    editor.set_show_line_numbers(true);
    editor.set_word_wrap(true);
    let area = editor.text_area();
    let window = Window::new();
    window.set_content(Some(Control::boxed(editor.clone())));
    window.show();
    area.measure(Size::new(135.0, 400.0));
    area.arrange(Rect::new(0.0, 0.0, 135.0, 400.0));
    area.ensure_visual_lines().unwrap();
    let line = area.try_line_layout(0).unwrap();
    let rows = line.layout.text_lines();
    assert!(rows.len() >= 3);
    let first_end = rows[1].first_text_source_index() as usize;
    let final_start = rows.last().unwrap().first_text_source_index() as usize;
    let final_end = text.find('\n').unwrap() + 1;
    let row_point = |index: usize| {
        let y = line.layout.line_origin(index).y + rows[index].height() / 2.0;
        area.translate_point(Point::new(3.0, y), &window).unwrap()
    };
    let first = row_point(0);
    let final_row = row_point(rows.len() - 1);
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let press = |point, modifiers| {
        let event = PointerPressedEventArgs::new(
            area.clone(), pointer.clone(), &window, point, 1,
            PointerPointProperties::new(RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::LeftButtonPressed),
            modifiers, 1,
        );
        area.raise_event(&event);
        assert!(event.handled());
    };
    let release = |point| {
        let event = PointerReleasedEventArgs::new(
            area.clone(), pointer.clone(), &window, point, 3,
            PointerPointProperties::new(RawInputModifiers::NONE, PointerUpdateKind::LeftButtonReleased),
            KeyModifiers::NONE, MouseButton::Left,
        );
        area.raise_event(&event);
        assert!(event.handled());
        assert!(pointer.captured().is_none());
    };

    area.invalidate_layout_cache();
    press(final_row, KeyModifiers::NONE);
    assert_eq!(editor.selected_text().unwrap(), text[final_start..final_end]);
    release(final_row);
    press(first, KeyModifiers::NONE);
    assert_eq!(editor.selected_text().unwrap(), text[..first_end]);
    assert!(editor.caret_position().is_at_end_of_line);
    release(first);

    press(final_row, KeyModifiers::SHIFT);
    assert_eq!(editor.selected_text().unwrap(), text[..final_end]);
    let move_back = PointerEventArgs::new(
        Some(InputElement::pointer_moved_event()), area.clone(), pointer.clone(),
        Some(&window), first, 2,
        PointerPointProperties::new(RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::Other),
        KeyModifiers::SHIFT,
    );
    area.raise_event(&move_back);
    assert!(move_back.handled());
    assert_eq!(editor.selected_text().unwrap(), text[..first_end]);
    release(first);

    let separator = area.visual_children().snapshot().iter()
        .find_map(|child| child.cast::<crate::DottedLineMargin>())
        .unwrap();
    let click = area.translate_point(
        Point::new(separator.bounds().center().x, line.layout.height() * 0.5),
        &window,
    ).unwrap();
    let event = PointerPressedEventArgs::new(
        area.clone(), pointer.clone(), &window, click, 4,
        PointerPointProperties::new(RawInputModifiers::LEFT_MOUSE_BUTTON, PointerUpdateKind::LeftButtonPressed),
        KeyModifiers::NONE, 1,
    );
    area.raise_event(&event);
    assert_eq!(editor.selected_text().unwrap(), text[..first_end]);
    assert!(pointer.captured().is_none());
    window.close();
}

#[test]
fn generated_formatted_text_keeps_native_metrics_and_drawing() {
    use crate::{FormattedTextContent, FormattedTextRun};
    use ferroui_base::media::text_formatting::testing::{DrawCall, RecordingDrawingSink};
    use ferroui_base::media::{Brushes, FlowDirection, FormattedText, Typeface};
    use ferroui_base::utilities::Locale;

    let _scope = test_scope();
    let editor = laid_out_editor("a!b");
    let area = editor.text_area();
    let text = Rc::new(FormattedText::new(
        "WIDE  \nnext",
        Locale::UNKNOWN,
        FlowDirection::LeftToRight,
        Typeface::default_typeface(),
        24.0,
        Some(Brushes::teal()),
    ));
    text.set_foreground_brush_range(Some(Brushes::magenta()), 7, 4);
    let prepared = text.clone();
    let mut generator = ProbeGenerator::new("prepared text", Rc::new(RefCell::new(Vec::new())));
    generator.offset = 1;
    generator.factory = Some(Rc::new(move |context| {
        Ok(Box::new(FormattedTextElement::from_formatted_text(
            prepared.clone(),
            1,
            &**context.global_text_run_properties(),
        )?))
    }));
    let generator: VisualLineElementGeneratorRef = Rc::new(generator).into();
    area.element_generators().add(generator.clone());

    let line = area.get_or_construct_visual_line(1).unwrap();
    assert_eq!(line.visual_length(), 3);
    let element = line.element_at(1).unwrap();
    let run = element.create_text_run(1, line.context()).unwrap();
    let run = run.downcast_ref::<FormattedTextRun>().unwrap();
    assert!(
        matches!(run.content(), FormattedTextContent::FormattedText(content) if Rc::ptr_eq(content, &text))
    );
    let drawable = run.as_drawable().unwrap();
    let width = text.width_including_trailing_whitespace();
    assert!(width > text.width());
    assert_eq!(drawable.size(), Size::new(width, text.height()));
    assert_eq!(drawable.baseline(), text.baseline());
    let span_width =
        || area.rectangle_for_offset(2).unwrap().x - area.rectangle_for_offset(1).unwrap().x;
    assert!((span_width() - width).abs() < 0.01);

    let mut sink = RecordingDrawingSink::new();
    area.cache.borrow()[&0]
        .layout
        .draw(&mut sink, Point::default());
    let glyphs: Vec<_> = sink
        .calls()
        .into_iter()
        .filter_map(|call| match call {
            DrawCall::GlyphRun { text, .. } => Some(text),
            _ => None,
        })
        .collect();
    assert!(glyphs.iter().any(|text| text.contains("WIDE")));
    assert!(glyphs.iter().any(|text| text.contains("next")));
    assert!(glyphs.iter().any(|text| text == "b"));
    assert_eq!(sink.transform_depth(), 0);

    text.set_font_size(30.0);
    area.redraw();
    let updated_width = text.width_including_trailing_whitespace();
    assert!(updated_width > width);
    assert!((span_width() - updated_width).abs() < 0.01);
    area.element_generators().remove(&generator);
    assert!(span_width() < width);
    assert_eq!(editor.text(), "a!b");
}

#[test]
fn rich_text_colorizer_overlays_real_runs_and_recolors_after_host_redraw() {
    use crate::RichTextColorizer;
    use ferroui_base::media::text_formatting::TextRunProperties;
    use ferroui_base::media::{Colors, FontStyle, FontWeight, Typeface};
    use ferroui_edit_core::highlighting::{HighlightingColor, RichTextModel};

    let _scope = test_scope();
    let editor = laid_out_editor("a😀bc\nhidden\ntail");
    let area = editor.text_area();
    area.resources().add_value("HostAccent", Colors::BLUE);
    let folding = editor.install_folding();
    folding.create_folding(5, 13).unwrap().set_is_folded(true);
    area.line_transformers().add(
        Rc::new(ColorizingTransformer::new(|context, line| {
            let source = context.first_document_line();
            line.change_document_part(source.offset(), source.end_offset(), |element| {
                let properties = element.data_mut().text_run_properties_mut();
                let font = properties.typeface();
                properties.set_typeface(Typeface::with_style(
                    font.font_family().clone(),
                    FontStyle::Italic,
                    font.weight(),
                    font.stretch(),
                ));
            })
        }))
        .into(),
    );

    let model = Rc::new(RefCell::new(RichTextModel::new()));
    {
        let mut model = model.borrow_mut();
        model
            .set_highlighting(
                1,
                2,
                Some(&HighlightingColor {
                    foreground: Some("resource:HostAccent".into()),
                    background: Some("Yellow".into()),
                    bold: Some(true),
                    font_size: Some(22.0),
                    ..HighlightingColor::default()
                }),
            )
            .unwrap();
        model
            .set_highlighting(
                3,
                1,
                Some(&HighlightingColor {
                    name: Some("metadata".into()),
                    ..HighlightingColor::default()
                }),
            )
            .unwrap();
        model.set_foreground(13, 4, Some("Red")).unwrap();
    }
    let colorizer: crate::VisualLineTransformerRef =
        Rc::new(RichTextColorizer::new(model.clone())).into();
    area.line_transformers().add(colorizer.clone());

    let line = area.get_or_construct_visual_line(1).unwrap();
    assert_eq!(line.last_document_line().line_number(), 3);
    let emoji = line.element_at(line.visual_offset(1)).unwrap().data();
    assert_eq!(emoji.document_length(), 2);
    let props = emoji.text_run_properties();
    assert_eq!(props.font_rendering_em_size(), 22.0);
    assert_eq!(props.typeface().style(), FontStyle::Italic);
    assert_eq!(props.typeface().weight(), FontWeight::Bold);
    assert_eq!(
        props
            .foreground_brush()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color(),
        Colors::BLUE
    );
    assert!(props.background_brush().is_none());
    assert_eq!(
        emoji
            .background_brush()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color(),
        Colors::YELLOW
    );
    assert_eq!(
        line.element_at(line.visual_offset(3))
            .unwrap()
            .data()
            .document_length(),
        2
    );
    let tail_foreground = |line: &crate::VisualLine| {
        line.element_at(line.visual_offset(13))
            .unwrap()
            .data()
            .text_run_properties()
            .foreground_brush()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color()
    };
    assert_eq!(tail_foreground(&line), Colors::RED);

    {
        let mut model = model.borrow_mut();
        model.set_foreground(13, 4, Some("Green")).unwrap();
        model
            .set_highlighting(
                1,
                2,
                Some(&HighlightingColor {
                    font_size: Some(18.0),
                    ..HighlightingColor::default()
                }),
            )
            .unwrap();
    }
    area.redraw();
    let refreshed = area.get_or_construct_visual_line(1).unwrap();
    assert_eq!(tail_foreground(&refreshed), Colors::GREEN);
    assert_eq!(tail_foreground(&line), Colors::RED);
    let emoji = refreshed.element_at(refreshed.visual_offset(1)).unwrap().data();
    assert_eq!(emoji.text_run_properties().font_rendering_em_size(), 18.0);
    assert!(emoji.background_brush().is_none());
    area.line_transformers().remove(&colorizer);
    let unstyled = area.get_or_construct_visual_line(1).unwrap();
    assert_ne!(tail_foreground(&unstyled), Colors::GREEN);
    assert_eq!(editor.text(), "a😀bc\nhidden\ntail");
}

#[test]
fn inline_controls_are_hosted_in_layout_and_reformat_when_metrics_change() {
    let _scope = test_scope();
    let editor = laid_out_editor("a!bc");
    editor.set_show_line_numbers(true);
    let area = editor.text_area();
    let child = ferroui_controls::Button::new();
    child.set_width(90.0);
    child.set_height(36.0);
    let generator = add_inline_object(&area, 1, 1, child.clone().upcast());
    render_editor(&editor);
    assert!(child.visual_parent().is_none());
    assert!(!child.is_measure_valid());

    layout_inline_test(&area);
    let layer = child.visual_parent().expect("inline visual parent");
    assert!(layer.clip_to_bounds());
    assert_eq!(layer.bounds(), area.content_viewport());
    assert!(child.parent().is_some_and(|parent| parent.ptr_eq(&layer)));
    assert_eq!(child.bounds().size(), Size::new(90.0, 36.0));
    let next_column = area.rectangle_for_offset(2).unwrap().x;

    child.set_width(112.0);
    ferroui_controls::TextBlock::set_baseline_offset_on(&child, 10.0);
    area.invalidate_measure();
    layout_inline_test(&area);
    assert!(child.visual_parent().is_some_and(|parent| parent.ptr_eq(&layer)));
    assert_eq!(child.bounds().width, 112.0);
    assert_eq!(area.rectangle_for_offset(2).unwrap().x - next_column, 22.0);
    assert!(editor.last_error().is_none());

    area.element_generators().remove(&generator);
    layout_inline_test(&area);
    assert!(child.visual_parent().is_none());
    assert!(child.parent().is_none());
}

#[test]
fn multi_column_inline_control_remains_one_measured_object_across_wrap() {
    use ferroui_controls::documents::EmbeddedControlRun;
    use ferroui_edit_core::document::{CaretPositioningMode, LogicalDirection};

    let _scope = test_scope();
    let editor = laid_out_editor("abcde");
    let area = editor.text_area();
    let button = ferroui_controls::Button::new();
    button.set_width(86.0);
    button.set_height(48.0);
    ferroui_controls::TextBlock::set_baseline_offset_on(&button, 31.0);
    let child = button.clone().upcast::<Control>();
    let mut generator =
        ProbeGenerator::new("wide inline child", Rc::new(RefCell::new(Vec::new())));
    generator.offset = 1;
    generator.factory = Some(Rc::new(move |context| {
        Ok(Box::new(crate::InlineObjectElement::with_visual_length(
            3,
            1,
            child.clone(),
            &**context.global_text_run_properties(),
        )?))
    }));
    let generator: VisualLineElementGeneratorRef = Rc::new(generator).into();
    area.element_generators().add(generator.clone());

    layout_inline_test(&area);
    let layer = button.visual_parent().unwrap();
    assert!(button.parent().is_some_and(|parent| parent.ptr_eq(&layer)));
    let verify_run = |wrapped| {
        let cached = area.cache.borrow()[&0].clone();
        let rows = cached.layout.text_lines();
        assert_eq!(rows.len() > 1, wrapped);
        let runs: Vec<_> = rows
            .iter()
            .flat_map(|row| row.text_runs().to_vec())
            .filter(|run| run.is::<EmbeddedControlRun>())
            .collect();
        assert_eq!(runs.len(), 1);
        let run = &runs[0];
        assert_eq!(run.length(), 3);
        let drawable = run.as_drawable().unwrap();
        assert_eq!(drawable.size(), Size::new(86.0, 48.0));
        assert_eq!(drawable.baseline(), 31.0);
        assert_eq!(button.bounds().size(), drawable.size());
        assert_eq!(cached.display.visual_length(), 7);
        assert_eq!(cached.display.visual_offset(2), 4);
        assert_eq!(
            cached
                .display
                .get_next_caret_position(
                    1,
                    LogicalDirection::Forward,
                    CaretPositioningMode::Normal,
                    false,
                )
                .unwrap(),
            Some(4),
        );
        assert!(editor.last_error().is_none());
    };
    verify_run(false);

    editor.set_word_wrap(true);
    area.measure(Size::new(60.0, 180.0));
    area.arrange(Rect::new(0.0, 0.0, 60.0, 180.0));
    verify_run(true);
    assert!(button.visual_parent().is_some_and(|parent| parent.ptr_eq(&layer)));

    area.element_generators().remove(&generator);
    layout_inline_test(&area);
    assert!(button.visual_parent().is_none());
    assert!(button.parent().is_none());
}

#[test]
fn inline_control_input_stays_with_the_child_and_scrolling_releases_both_parents() {
    use ferroui_base::input::{IKeyboardDevice, KeyboardDevice};
    use ferroui_controls::testing::TestServices;
    let _scope = crate::tests::test_scope_with_services(
        TestServices::styled_window()
            .with_keyboard_device(|| Some(KeyboardDevice::new() as Rc<dyn IKeyboardDevice>)),
    );
    let editor = laid_out_editor(&format!("a!b\n{}", "long enough to scroll\n".repeat(100)));
    let window = ferroui_controls::Window::new();
    window.set_content(Some(Control::boxed(editor.clone())));
    window.show();
    let area = editor.text_area();
    let child = ferroui_controls::Button::new();
    child.set_width(32.0);
    child.set_height(24.0);
    add_inline_object(&area, 1, 1, child.clone().upcast());
    layout_inline_test(&area);
    assert!(child.visual_parent().is_some());
    assert!(child.focus());

    use ferroui_controls::automation::peers::ControlAutomationPeer;
    use ferroui_controls::automation::provider::{IRootProvider, ITextProvider, ITextProvider2, TextUnit};
    let editor_peer = ControlAutomationPeer::create_peer_for_element(&editor);
    let child_peer = ControlAutomationPeer::create_peer_for_element(&child);
    let root = ControlAutomationPeer::create_peer_for_element(&window)
        .get_provider::<dyn IRootProvider>()
        .unwrap();
    let text = editor_peer.get_provider::<dyn ITextProvider>().unwrap();
    let caret = editor_peer.get_provider::<dyn ITextProvider2>().unwrap();
    let inline_range = text.range_from_child(&child_peer).unwrap();
    assert_eq!(inline_range.get_enclosing_element().unwrap(), child_peer);
    assert!(inline_range.get_children().unwrap().is_empty());
    inline_range.expand_to_enclosing_unit(TextUnit::Format).unwrap();
    assert_eq!(inline_range.get_text(None).unwrap(), "!");
    assert_eq!(inline_range.get_enclosing_element().unwrap(), child_peer);
    assert_eq!(inline_range.move_by(TextUnit::Format, -1).unwrap(), -1);
    assert_eq!(inline_range.get_text(None).unwrap(), "a");
    assert_eq!(inline_range.get_enclosing_element().unwrap(), editor_peer);
    assert_eq!(text.document_range().unwrap().get_children().unwrap(), [child_peer.clone()]);
    assert!(!caret.get_caret_range().unwrap().is_active);
    assert_eq!(root.get_focus(), Some(child_peer.clone()));
    assert!(!editor_peer.has_keyboard_focus());

    let mut input = TextInputEventArgs::new();
    input.text = Some("x".into());
    input.set_routed_event(Some(InputElement::text_input_event()));
    child.raise_event(&input);
    assert!(editor.text().starts_with("a!b"));
    assert!(!input.handled());

    editor.set_caret_offset(3).unwrap();
    let snippet = ferroui_edit_core::snippets::Snippet::parse("${1:first} ${2:second}").unwrap();
    editor.insert_snippet(&snippet).unwrap();
    layout_inline_test(&area);
    assert!(child.focus());
    let selected = editor.selected_text().unwrap();
    let mut tab = KeyEventArgs::new();
    tab.key = Key::Tab;
    tab.set_routed_event(Some(InputElement::key_down_event()));
    child.raise_event(&tab);
    assert!(!tab.handled());
    assert_eq!(editor.selected_text().unwrap(), selected);

    area.set_offset(Vector::new(0.0, 1000.0));
    layout_inline_test(&area);
    assert!(child.visual_parent().is_none());
    assert!(child.parent().is_none());
    assert!(area.is_focused());
    assert!(text.range_from_child(&child_peer).is_err());
    assert!(text.document_range().unwrap().get_children().unwrap().is_empty());
    assert!(caret.get_caret_range().unwrap().is_active);
    assert_eq!(root.get_focus(), Some(editor_peer.clone()));
    assert!(editor_peer.has_keyboard_focus());
    area.set_offset(Vector::default());
    layout_inline_test(&area);
    assert!(child.visual_parent().is_some());
    assert_eq!(child.bounds().size(), Size::new(32.0, 24.0));
    window.close();
}

#[test]
fn inserted_inline_control_ranges_are_distinct_from_ordinary_carets() {
    use ferroui_controls::automation::peers::ControlAutomationPeer;
    use ferroui_controls::automation::provider::{ITextProvider, TextUnit};

    let _scope = test_scope();
    let editor = laid_out_editor("abc");
    let window = ferroui_controls::Window::new();
    window.set_content(Some(Control::boxed(editor.clone())));
    window.show();
    let area = editor.text_area();
    let child = ferroui_controls::Button::new();
    child.set_width(32.0);
    child.set_height(24.0);
    let generator = add_inline_object(&area, 1, 0, child.clone().upcast());
    layout_inline_test(&area);
    let owner = ControlAutomationPeer::create_peer_for_element(&editor);
    let child_peer = ControlAutomationPeer::create_peer_for_element(&child);
    let text = owner.get_provider::<dyn ITextProvider>().unwrap();
    let range = text.range_from_child(&child_peer).unwrap();
    assert_eq!(range.text_offsets().unwrap(), Some(1..1));
    assert_eq!(range.get_text(None).unwrap(), "");
    assert_eq!(range.get_enclosing_element().unwrap(), child_peer);
    assert!(range.get_children().unwrap().is_empty());
    let duplicate = range.clone_range().unwrap();
    assert_eq!(duplicate.get_enclosing_element().unwrap(), child_peer);
    assert_eq!(duplicate.move_by(TextUnit::Character, 1).unwrap(), 1);
    assert_eq!(duplicate.get_enclosing_element().unwrap(), owner);

    let caret = text.range_from_offsets(1..1).unwrap().unwrap();
    assert_eq!(caret.get_enclosing_element().unwrap(), owner);
    assert!(caret.get_children().unwrap().is_empty());
    let preceding = text.range_from_offsets(0..1).unwrap().unwrap();
    assert!(preceding.get_children().unwrap().is_empty());
    assert_eq!(text.document_range().unwrap().get_children().unwrap(), [child_peer.clone()]);

    let root = child.visual_root().unwrap();
    let point = child.translate_point(Point::new(16.0, 12.0), &root).unwrap();
    let hit = text.embedded_range_from_point(point).unwrap().unwrap();
    assert_eq!(hit.get_enclosing_element().unwrap(), child_peer);
    assert!(hit.compare(&*range).unwrap());

    area.element_generators().remove(&generator);
    layout_inline_test(&area);
    assert_eq!(range.get_enclosing_element().unwrap(), owner);
    assert!(text.range_from_child(&child_peer).is_err());
    window.close();
}

#[test]
fn inline_controls_keep_their_existing_external_parent() {
    let _scope = test_scope();
    let editor = laid_out_editor("a!");
    let area = editor.text_area();
    let panel = ferroui_controls::Canvas::new();
    let child = ferroui_controls::Button::new();
    panel.children().add(child.clone().upcast::<Control>());
    add_inline_object(&area, 1, 1, child.clone().upcast());
    layout_inline_test(&area);
    assert!(child.visual_parent().is_some_and(|parent| parent.ptr_eq(&panel)));
    assert!(editor.last_error().unwrap().contains("must be unparented"));
}

#[test]
fn inline_control_position_matches_the_wrapped_bidirectional_formatter() {
    let _scope = test_scope();
    let prefix = "one two three four five six seven eight אבג ";
    let source = format!("{prefix}! דהו");
    let editor = laid_out_editor(&source);
    editor.set_word_wrap(true);
    let area = editor.text_area();
    let child = ferroui_controls::Button::new();
    child.set_width(28.0);
    child.set_height(23.0);
    let offset = prefix.encode_utf16().count();
    add_inline_object(&area, offset, 1, child.clone().upcast());
    area.measure(Size::new(135.0, 180.0));
    area.arrange(Rect::new(0.0, 0.0, 135.0, 180.0));
    let line = area.cache.borrow()[&0].clone();
    let column = line.display.visual_offset(offset);
    let rects = line.layout.hit_test_text_range(column as i32, 1);
    let formatted = rects[0];
    let position = child.translate_point(Point::default(), &area).unwrap();
    assert!(formatted.y > 0.0);
    assert!((position.x - (formatted.x + area.gutter_width())).abs() <= 1.0);
    assert!(position.y >= formatted.y && position.y < formatted.bottom());
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
fn offscreen_multiline_elements_are_discovered_at_their_owner() {
    let _scope = test_scope();
    let editor = laid_out_editor(&"plain text\n".repeat(160));
    let area = editor.text_area();
    let document = editor.document();
    let start = document.get_line_by_number(100).unwrap().offset();
    let end = document.get_line_by_number(107).unwrap().offset() + 1;
    let generator = ProbeGenerator::new("offscreen span", Rc::new(RefCell::new(Vec::new())))
        .element_at(start, end - start);
    let generator: VisualLineElementGeneratorRef = Rc::new(generator).into();
    area.element_generators().add(generator);

    let top = area.get_visual_top_by_document_line(104).unwrap();
    area.set_offset(Vector::new(0.0, top));
    area.ensure_visual_lines().unwrap();
    let viewport = area.visible_visual_lines().unwrap();
    assert_eq!(viewport[0].line.first_document_line().line_number(), 104);
    let interior = area.get_or_construct_visual_line(104).unwrap();
    assert_eq!(interior.first_document_line().line_number(), 104);
    let owner = area.get_or_construct_visual_line(100).unwrap();
    assert_eq!(owner.first_document_line().line_number(), 100);
    assert_eq!(owner.last_document_line().line_number(), 107);
    assert!(Rc::ptr_eq(&owner, &area.get_or_construct_visual_line(104).unwrap()));
}

#[test]
fn targeted_redraw_keeps_unaffected_lines_and_rebuilds_the_fold_owner() {
    let _scope = test_scope();
    let editor = laid_out_editor("zero\none\ntwo\nthree\nfour");
    let area = editor.text_area();
    let rows = || {
        area.ensure_visual_lines().unwrap();
        area.visible_visual_lines()
            .unwrap()
            .into_iter()
            .map(|row| row.line)
            .collect::<Vec<_>>()
    };
    let changed = |previous: &[Rc<crate::VisualLine>], current: &[Rc<crate::VisualLine>]| {
        current
            .iter()
            .filter(|line| !previous.iter().any(|old| Rc::ptr_eq(line, old)))
            .map(|line| line.first_document_line().line_number())
            .collect::<Vec<_>>()
    };

    let initial = rows();
    assert!(matches!(
        area.redraw_range(0..usize::MAX),
        Err(VisualLineError::InvalidRange { .. })
    ));
    assert!(area.visual_lines_valid());
    area.redraw_range(6..7).unwrap();
    assert!(!area.visual_lines_valid());
    let redrawn_range = rows();
    assert_eq!(changed(&initial, &redrawn_range), [2]);
    assert!(!area.redraw_visual_line(&initial[1]));
    assert!(area.redraw_visual_line(&redrawn_range[2]));
    let redrawn_line = rows();
    assert_eq!(changed(&redrawn_range, &redrawn_line), [3]);
    area.redraw_range(5..5).unwrap();
    let boundary = rows();
    assert_eq!(changed(&redrawn_line, &boundary), [1, 2]);

    let manager = editor.install_folding();
    manager.create_folding(2, 13).unwrap().set_is_folded(true);
    let folded = rows();
    assert_eq!(folded[0].last_document_line().line_number(), 4);
    area.redraw_range(6..7).unwrap();
    let refreshed = rows();
    assert_eq!(changed(&folded, &refreshed), [1]);
    assert!(Rc::ptr_eq(&area.get_visual_line(2).unwrap(), &refreshed[0]));
}

#[test]
fn redraw_range_forgets_evicted_generated_spans_and_remeasures_width() {
    let _scope = test_scope();
    let editor = laid_out_editor(&format!("a{{\n hidden\n}}z\n{}", "row\n".repeat(100)));
    let area = editor.text_area();
    let mut generator = ProbeGenerator::new("span", Rc::new(RefCell::new(Vec::new())));
    generator.offset = 1;
    generator.factory = Some(Rc::new(move |context| {
        Ok(Box::new(FormattedTextElement::new(
            "a wide label ".repeat(20),
            11,
            &**context.global_text_run_properties(),
        )?))
    }));
    let generator = Rc::new(generator);
    area.element_generators().add(generator.clone().into());
    area.ensure_visual_lines().unwrap();
    let wide = area.extent().width;
    assert!(wide > area.viewport().width);
    assert_eq!(area.heights.borrow().visible_owner(1), 0);
    area.set_offset(Vector::new(0.0, 1000.0));
    area.ensure_visual_lines().unwrap();
    assert!(area.get_visual_line(1).is_none());
    assert_eq!(area.heights.borrow().visible_owner(1), 0);

    let retained = area.visible_visual_lines().unwrap()[0].line.clone();
    generator.enabled.set(false);
    area.redraw_range(5..6).unwrap();
    area.ensure_visual_lines().unwrap();
    assert_eq!(area.heights.borrow().visible_owner(1), 1);
    let number = retained.first_document_line().line_number();
    assert!(Rc::ptr_eq(
        &area.get_visual_line(number).unwrap(),
        &retained
    ));
    assert!(area.extent().width < wide);
    area.set_offset(Vector::default());
    area.ensure_visual_lines().unwrap();
    assert_eq!(
        area.get_visual_line(2)
            .unwrap()
            .first_document_line()
            .line_number(),
        2
    );
}

#[test]
fn redraw_range_during_construction_retries_without_publishing_the_stale_line() {
    let _scope = test_scope();
    let editor = laid_out_editor("one\ntwo");
    let area = editor.text_area();
    let generator =
        ProbeGenerator::new("redraw", Rc::new(RefCell::new(Vec::new()))).element_at(1, 1);
    generator.on_start.replace(Some(Box::new(|context| {
        context.text_view().unwrap().redraw_range(1..2).unwrap();
    })));
    area.element_generators().add(Rc::new(generator).into());
    assert!(matches!(
        area.get_or_construct_visual_line(1),
        Err(VisualLineError::ConstructionInvalidated)
    ));
    assert!(area.get_visual_line(1).is_none());
    assert!(!area.visual_lines_valid());
    area.ensure_visual_lines().unwrap();
    assert!(area.get_visual_line(1).is_some());
    assert!(area.visual_lines_valid());
}

#[test]
fn viewport_observers_receive_only_current_visible_lines_without_recursive_layout() {
    let _scope = test_scope();
    let editor = laid_out_editor(&"row content\n".repeat(80));
    let area = editor.text_area();
    area.redraw();
    assert!(!area.visual_lines_valid());
    assert!(area.visible_visual_lines().is_none());

    let seen = Rc::new(RefCell::new(Vec::new()));
    let seen_by_handler = seen.clone();
    let subscription = area.visual_lines_changed(move |view, rows| {
        assert!(view.visual_lines_valid());
        view.ensure_visual_lines().unwrap();
        let visible = view.visible_visual_lines().unwrap();
        assert_eq!(visible.len(), rows.len());
        assert!(visible.iter().zip(rows).all(|(a, b)| Rc::ptr_eq(&a.line, &b.line)));
        assert!(rows.iter().all(|row| row.bounds.bottom() > 0.0 && row.bounds.y < view.viewport.get().height));
        seen_by_handler.borrow_mut().push(rows[0].line.first_document_line().line_number());
    });
    let constructing = area.visual_line_construction_starting({
        let weak = area.downgrade();
        move |_| assert!(matches!(weak.upgrade().unwrap().ensure_visual_lines(), Err(VisualLineError::RecursiveConstruction)))
    });

    area.ensure_visual_lines().unwrap();
    area.ensure_visual_lines().unwrap();
    assert_eq!(*seen.borrow(), [1]);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(*seen.borrow(), [1]);

    render_editor(&editor);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(*seen.borrow(), [1]);
    area.set_offset(Vector::new(0.0, area.line_height() * 8.25));
    assert!(!area.visual_lines_valid());
    area.ensure_visual_lines().unwrap();
    let rows = area.visible_visual_lines().unwrap();
    assert!(rows[0].bounds.y < 0.0);
    assert_eq!(rows[0].line.first_document_line().line_number(), 9);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(*seen.borrow(), [1, 9]);

    area.redraw();
    area.ensure_viewport_lines().unwrap();
    editor.document().insert(0, "edit ").unwrap();
    assert!(!area.visual_lines_valid());
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(*seen.borrow(), [1, 9]);

    area.ensure_viewport_lines().unwrap();
    drop(subscription);
    drop(constructing);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(*seen.borrow(), [1, 9]);
}

#[test]
fn viewport_binding_and_scroll_events_report_effective_changes() {
    use crate::TextEditorComponent;

    let _scope = test_scope();
    let editor = laid_out_editor(&"row\n".repeat(30));
    let area = editor.text_area();
    area.ensure_visual_lines().unwrap();
    let scrolls = Rc::new(RefCell::new(Vec::new()));
    let _scroll = area.scroll_offset_changed({
        let scrolls = scrolls.clone();
        move |view, old, new| {
            assert_eq!(view.scroll_offset(), new);
            assert!(!view.visual_lines_valid());
            scrolls.borrow_mut().push((old, new));
        }
    });
    let next = Vector::new(0.0, area.line_height() * 3.0);
    area.set_offset(next);
    area.set_offset(next);
    assert_eq!(*scrolls.borrow(), [(Vector::default(), next)]);

    let bindings = Rc::new(RefCell::new(Vec::new()));
    let _binding = area.subscribe_document_changed(Rc::new({
        let bindings = bindings.clone();
        let weak = area.downgrade();
        move |old, new| {
            let view = weak.upgrade().unwrap();
            assert_eq!(&view.editor().unwrap().document(), new);
            assert_eq!(view.scroll_offset(), Vector::default());
            assert!(!view.visual_lines_valid());
            bindings.borrow_mut().push((old.clone(), new.clone()));
        }
    })).unwrap();
    let old = editor.document();
    old.insert(0, "edit ").unwrap();
    assert!(bindings.borrow().is_empty());
    let new = TextDocument::from_text("replacement");
    editor.set_document(new.clone());
    assert_eq!(*bindings.borrow(), [(old, new)]);
}

#[test]
fn viewport_observers_can_follow_arrangement_and_empty_unbound_views() {
    let _scope = test_scope();
    let unbound = TextArea::new();
    assert!(!unbound.visual_lines_valid());
    unbound.ensure_visual_lines().unwrap();
    assert!(unbound.visible_visual_lines().unwrap().is_empty());

    let editor = laid_out_editor("one\ntwo\nthree");
    let area = editor.text_area();
    let count = Rc::new(Cell::new(0));
    let _subscription = area.visual_lines_changed({
        let count = count.clone();
        move |_, rows| count.set(rows.len())
    });
    area.redraw();
    area.measure(Size::new(240.0, 180.0));
    area.arrange(Rect::new(0.0, 0.0, 240.0, 180.0));
    assert!(area.visual_lines_valid());
    assert_eq!(count.get(), 0);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(count.get(), 3);
}

#[test]
fn viewport_observer_invalidation_cannot_return_an_old_snapshot_from_ensure() {
    let _scope = test_scope();
    let editor = laid_out_editor("one\ntwo");
    let area = editor.text_area();
    let calls = Rc::new(Cell::new(0));
    let _subscription = area.visual_lines_changed({
        let calls = calls.clone();
        move |view, _| {
            calls.set(calls.get() + 1);
            if calls.get() == 1 {
                view.redraw();
            }
        }
    });
    area.redraw();
    assert!(matches!(area.ensure_visual_lines(), Err(VisualLineError::ConstructionInvalidated)));
    assert!(area.visible_visual_lines().is_none());
    area.ensure_visual_lines().unwrap();
    assert_eq!(calls.get(), 2);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(calls.get(), 2);
}

#[test]
fn viewport_observers_keep_a_nested_update_pending_when_the_dispatcher_is_pumped() {
    let _scope = test_scope();
    let editor = laid_out_editor("one\ntwo");
    let area = editor.text_area();
    let calls = Rc::new(Cell::new(0));
    let _subscription = area.visual_lines_changed({
        let calls = calls.clone();
        move |view, _| {
            calls.set(calls.get() + 1);
            if calls.get() == 1 {
                view.redraw();
                view.ensure_visual_lines().unwrap();
                Dispatcher::ui_thread().run_jobs(None);
                assert_eq!(calls.get(), 1);
            }
        }
    });
    area.redraw();
    area.ensure_visual_lines().unwrap();
    assert_eq!(calls.get(), 1);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(calls.get(), 2);
}

#[test]
fn document_space_position_hits_choose_nearest_or_floor_and_preserve_virtual_cells() {
    use crate::{TextViewPosition, VisualYPosition};

    let _scope = test_scope();
    let editor = laid_out_editor("ab\nnext");
    let area = editor.text_area();
    let mut options = editor.options();
    options.enable_virtual_space = true;
    editor.set_options(options);
    let project = |column| {
        area.get_visual_position(TextViewPosition::new(1, column), VisualYPosition::LineMiddle)
            .unwrap()
    };
    let start = project(1);
    let after = project(2);
    let near_trailing = Point::new(start.x + (after.x - start.x) * 0.8, start.y);
    assert_eq!(area.get_position(near_trailing).unwrap().unwrap().column, 2);
    assert_eq!(area.get_position_floor(near_trailing).unwrap().unwrap().column, 1);
    let end = project(3);
    let just_beyond = Point::new(end.x + 0.2 * area.wide_space_width(), end.y);
    let nearest = area.get_position(just_beyond).unwrap().unwrap();
    let floor = area.get_position_floor(just_beyond).unwrap().unwrap();
    assert_eq!((nearest.visual_column, nearest.is_at_end_of_line), (Some(2), false));
    assert_eq!((floor.visual_column, floor.is_at_end_of_line), (Some(2), true));
    let virtual_point = Point::new(end.x + 2.8 * area.wide_space_width(), end.y);
    let nearest = area.get_position(virtual_point).unwrap().unwrap();
    let floor = area.get_position_floor(virtual_point).unwrap().unwrap();
    assert_eq!((nearest.column, nearest.visual_column, nearest.is_at_end_of_line), (3, Some(5), true));
    assert_eq!((floor.column, floor.visual_column, floor.is_at_end_of_line), (3, Some(4), true));
    let projected = area.get_visual_position(nearest, VisualYPosition::LineMiddle).unwrap();
    assert!((projected.x - (end.x + 3.0 * area.wide_space_width())).abs() < 0.001);
    assert_eq!(area.get_position(projected).unwrap(), Some(nearest));
    let mut options = editor.options();
    options.enable_virtual_space = false;
    editor.set_options(options);
    let bounded = area.get_position_floor(virtual_point).unwrap().unwrap();
    assert_eq!((bounded.column, bounded.visual_column, bounded.is_at_end_of_line), (3, Some(3), true));
    let nearest = area.get_position(virtual_point).unwrap().unwrap();
    assert_eq!((nearest.column, nearest.visual_column, nearest.is_at_end_of_line), (3, Some(2), false));
}

#[test]
fn document_space_position_preserves_wrapped_row_affinity() {
    use crate::{TextViewPosition, VisualYPosition};
    use ferroui_edit_core::document::TextLocation;

    let _scope = test_scope();
    let editor = laid_out_editor("abcdefghijklmnopqrstuv\nsecond");
    editor.set_word_wrap(true);
    let area = editor.text_area();
    area.measure(Size::new(64.0, 180.0));
    area.arrange(Rect::new(0.0, 0.0, 64.0, 180.0));
    area.ensure_visual_lines().unwrap();
    let line = area.cache.borrow()[&0].clone();
    let rows = line.layout.text_lines();
    assert!(rows.len() >= 2);
    let boundary = rows[1].first_text_source_index() as usize;
    let next = TextViewPosition::with_visual_column(TextLocation::new(1, boundary + 1), boundary);
    let previous = TextViewPosition { is_at_end_of_line: true, ..next };
    let next_top = area.get_visual_position(next, VisualYPosition::LineTop).unwrap();
    let previous_top = area.get_visual_position(previous, VisualYPosition::LineTop).unwrap();
    assert_eq!(previous_top.y, 0.0);
    assert_eq!(next_top.y, line.layout.line_origin(1).y);
    assert!(previous_top.x > next_top.x);
    let beyond = Point::new(rows[0].width_including_trailing_whitespace() + 20.0, next_top.y / 2.0);
    assert_eq!(area.get_position_floor(beyond).unwrap(), Some(previous));
    assert_eq!(area.get_position_floor(next_top).unwrap(), Some(next));
    assert_eq!(area.get_visual_top_by_document_line(2).unwrap(), line.layout.height());
}

#[test]
fn vertical_selection_stays_on_the_wrapped_bidirectional_row() {
    use crate::{EditorCommandKind, EditorNavigation};

    let _scope = test_scope();
    const PREFIX: &str = "abcdefghijklmnop";
    for (target, expected) in [
        ("ab אבג ", "ab אבג"),
        ("אבג 😀 e\u{0301}   ", "אבג 😀 e\u{0301}  "),
    ] {
        let editor = laid_out_editor(&format!(
            "{PREFIX}\n{target}12345678901234567890 xxxxxx yyyyyyyyy zzzzzzzz\nlast"
        ));
        let area = editor.text_area();
        let source_end = PREFIX.len();
        let width = area.rectangle_for_offset(source_end).unwrap().x + 10.0;
        editor.set_word_wrap(true);
        area.measure(Size::new(width, 360.0));
        area.arrange(Rect::new(0.0, 0.0, width, 360.0));
        editor.set_caret_offset(source_end).unwrap();

        let line = area.try_line_layout(1).unwrap();
        let rows = line.layout.text_lines();
        assert!(rows.len() > 1);
        assert_eq!(
            rows[1].first_text_source_index() as usize,
            target.encode_utf16().count()
        );
        let target_top = area.get_visual_top_by_document_line(2).unwrap();
        editor
            .command(EditorCommandKind::Navigate(EditorNavigation::Caret {
                movement: CaretMovement::LineDown,
                extend_selection: true,
            }))
            .execute(None);
        assert_eq!(
            editor.caret_offset(),
            source_end + 1 + expected.encode_utf16().count()
        );
        assert_eq!(editor.selected_text().unwrap(), format!("\n{expected}"));
        assert!(!editor.caret_position().is_at_end_of_line);
        let position = area
            .get_visual_position(editor.caret_position(), crate::VisualYPosition::LineTop)
            .unwrap();
        assert_eq!(position.y, target_top);

        let mut up = KeyEventArgs::new();
        up.key = Key::Up;
        up.set_routed_event(Some(InputElement::key_down_event()));
        area.raise_event(&up);
        assert!(up.handled());
        assert_eq!(editor.caret_offset(), source_end);
        assert!(editor.selection().is_empty());
    }
}

#[test]
fn vertical_movement_clamps_to_a_complete_grapheme_at_a_soft_break() {
    use crate::TextViewPosition;
    use ferroui_edit_core::document::TextLocation;

    let _scope = test_scope();
    let editor = laid_out_editor(&format!("x\n{}", "😀".repeat(24)));
    let mut options = editor.options();
    options.enable_virtual_space = true;
    editor.set_options(options);
    editor.set_word_wrap(true);
    let area = editor.text_area();
    area.measure(Size::new(90.0, 360.0));
    area.arrange(Rect::new(0.0, 0.0, 90.0, 360.0));
    editor
        .set_caret_position(TextViewPosition::with_visual_column(
            TextLocation::new(1, 2),
            24,
        ))
        .unwrap();
    assert_eq!(editor.session().caret_virtual_space(), 23);

    let line = area.try_line_layout(1).unwrap();
    let boundary = line.layout.text_lines()[1].first_text_source_index() as usize;
    assert!(boundary >= 2 && boundary % 2 == 0);
    let mut down = KeyEventArgs::new();
    down.key = Key::Down;
    down.set_routed_event(Some(InputElement::key_down_event()));
    area.raise_event(&down);
    assert!(down.handled());
    assert_eq!(editor.caret_offset(), line.offset + boundary - 2);
    assert_eq!(editor.session().caret_virtual_space(), 0);
}

#[test]
fn horizontal_bidi_movement_uses_logical_graphemes_and_line_down_preserves_the_final_caret() {
    use crate::{EditorCommandKind, EditorNavigation};

    let _scope = test_scope();
    let editor = laid_out_editor("ab אבג cd\nאבג");
    let area = editor.text_area();
    let key = |key, modifiers| {
        let mut event = KeyEventArgs::new();
        event.key = key;
        event.key_modifiers = modifiers;
        event.set_routed_event(Some(InputElement::key_down_event()));
        area.raise_event(&event);
        assert!(event.handled());
    };
    editor.set_caret_offset(3).unwrap();
    let initial_x = area.cursor_rectangle().x;
    key(Key::Right, KeyModifiers::NONE);
    assert_eq!(editor.caret_offset(), 4);
    assert!(area.cursor_rectangle().x < initial_x);
    key(Key::Right, KeyModifiers::SHIFT);
    assert_eq!(editor.caret_offset(), 5);
    assert_eq!(editor.selected_text().unwrap(), "ב");

    editor.set_caret_offset(10).unwrap();
    editor
        .command(EditorCommandKind::Navigate(EditorNavigation::Caret {
            movement: CaretMovement::LineEnd,
            extend_selection: false,
        }))
        .execute(None);
    let end = editor.caret_position();
    assert!(end.is_at_end_of_line);
    key(Key::Down, KeyModifiers::NONE);
    assert_eq!(editor.caret_position(), end);

    editor.set_caret_offset(4).unwrap();
    key(Key::PageDown, KeyModifiers::NONE);
    assert_eq!(editor.caret_offset(), 10);
}

#[test]
fn rectangular_pointer_drag_snaps_bidi_edges_and_uses_the_first_wrapped_row() {
    use ferroui_base::input::{
        MouseButton, Pointer, PointerPointProperties, PointerType, RawInputModifiers,
    };
    use ferroui_controls::Window;

    let _scope = test_scope();
    let text = "abcdefghijklmnop\nab אבג 12345678901234567890 xxxxxx yyyyyyyyy zzzzzzzz\nlast";
    for (reverse, end, ranges, selected) in [
        (false, 23, [(2, 6), (19, 23)], "cdef\n אבג"),
        (true, 2, [(2, 3), (19, 20)], "c\n "),
    ] {
        let editor = laid_out_editor(text);
        editor.set_show_line_numbers(false);
        let window = Window::new();
        window.set_content(Some(Control::boxed(editor.clone())));
        window.show();
        let area = editor.text_area();
        let width = area.rectangle_for_offset(16).unwrap().x + 10.0;
        editor.set_word_wrap(true);
        area.measure(Size::new(width, 300.0));
        area.arrange(Rect::new(0.0, 0.0, width, 300.0));
        area.ensure_visual_lines().unwrap();
        let first = area.rectangle_for_offset(2).unwrap();
        let advance = area.rectangle_for_offset(3).unwrap().x - first.x;
        let last = area.rectangle_for_offset(27).unwrap();
        let anchor = Point::new(first.x + 0.2 * advance, first.y + first.height * 0.5);
        let continuation = Point::new(first.x + 1.15 * advance, last.y + last.height * 0.5);
        assert_eq!(area.offset_at_point(continuation), 27);
        let (start, target) = if reverse {
            (continuation, anchor)
        } else {
            (anchor, continuation)
        };
        let start = area.translate_point(start, &window).unwrap();
        let target = area.translate_point(target, &window).unwrap();
        let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
        let held = PointerPointProperties::new(
            RawInputModifiers::LEFT_MOUSE_BUTTON,
            PointerUpdateKind::LeftButtonPressed,
        );
        let modifier = super::box_selection_modifier();
        let press = PointerPressedEventArgs::new(
            area.clone(),
            pointer.clone(),
            &window,
            start,
            1,
            held,
            modifier,
            1,
        );
        area.raise_event(&press);
        assert!(press.handled());
        assert_eq!(editor.caret_offset(), if reverse { 27 } else { 2 });
        area.raise_event(&PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            area.clone(),
            pointer.clone(),
            Some(&window),
            target,
            2,
            held,
            modifier,
        ));
        area.raise_event(&PointerReleasedEventArgs::new(
            area.clone(),
            pointer,
            &window,
            target,
            3,
            PointerPointProperties::new(
                RawInputModifiers::NONE,
                PointerUpdateKind::LeftButtonReleased,
            ),
            modifier,
            MouseButton::Left,
        ));
        let selection = editor.selection();
        assert!(selection.is_rectangular());
        let offsets: Vec<_> = selection
            .segments()
            .iter()
            .map(|row| (row.start_offset(), row.end_offset()))
            .collect();
        assert_eq!(offsets, ranges);
        assert_eq!(editor.caret_offset(), end);
        assert!(editor.caret_position().is_at_end_of_line);
        assert_eq!(editor.selected_text().unwrap(), selected);
        window.close();
    }
}

#[test]
fn pointer_exit_preserves_wrapped_lines_and_scroll_offset() {
    use ferroui_base::input::raw::{RawPointerEventArgs, RawPointerEventType};
    use ferroui_base::input::{InputManager, MouseDevice, RawInputModifiers};
    use ferroui_controls::Window;
    use ferroui_controls::testing::TestServices;

    let _scope = crate::tests::test_scope_with_services(
        TestServices::styled_window().with_input_manager(Rc::new(InputManager::new())),
    );
    let editor = laid_out_editor(&"ab אבג 😀 e\u{0301} 12345678901234567890 end\n".repeat(24));
    editor.set_word_wrap(true);
    let window = Window::new();
    window.set_width(160.0);
    window.set_height(120.0);
    window.set_content(Some(Control::boxed(editor.clone())));
    window.show();
    Dispatcher::ui_thread().run_jobs(None);

    let area = editor.text_area();
    let inside = area.translate_point(Point::new(40.0, 30.0), &window).unwrap();
    let emit = window.platform_impl().unwrap().input().unwrap();
    let mouse = MouseDevice::new();
    let move_to = |point| {
        emit(Rc::new(RawPointerEventArgs::new(
            mouse.clone(), 0, window.input_root(), RawPointerEventType::Move, point, RawInputModifiers::NONE,
        )));
    };
    move_to(inside);
    assert!(editor.is_pointer_over());
    editor.set_caret_offset(editor.document().text_length()).unwrap();
    render_editor(&editor);
    Dispatcher::ui_thread().run_jobs(None);

    let (offset, extent, lines) = (area.offset(), area.extent(), area.visual_lines());
    assert!(offset.y > 0.0 && !lines.is_empty());
    move_to(Point::new(inside.x, -1.0));
    assert!(!editor.is_pointer_over());
    assert_eq!(area.offset(), offset);
    assert_eq!(area.extent(), extent);
    let retained = area.visual_lines();
    assert_eq!(retained.len(), lines.len());
    assert!(retained.iter().zip(&lines).all(|(new, old)| Rc::ptr_eq(new, old)));
    window.close();
}

#[test]
fn captured_drags_use_the_viewport_edge_in_wrapped_bidi_text() {
    use ferroui_base::input::{
        MouseButton, Pointer, PointerPointProperties, PointerType, RawInputModifiers,
    };
    use ferroui_controls::Window;

    let _scope = test_scope();
    let text = "ab אבג 😀 e\u{0301} 12345678901234567890 end\r\n".repeat(30);
    let drag = |rectangular: bool, upward: bool, outside: bool| {
        let editor = laid_out_editor(&text);
        editor.set_show_line_numbers(false);
        editor.set_word_wrap(true);
        let window = Window::new();
        window.set_width(134.0);
        window.set_height(118.0);
        window.set_content(Some(Control::boxed(editor.clone())));
        window.show();
        Dispatcher::ui_thread().run_jobs(None);
        let area = editor.text_area();
        let height = area.viewport().height;
        assert!(height > 60.0 && height < 140.0, "{height}");
        area.ensure_visual_lines().unwrap();
        assert!(area.try_line_layout(0).unwrap().layout.text_lines().len() > 1);
        if upward {
            area.set_offset(Vector::new(0.0, 220.0));
            area.ensure_visual_lines().unwrap();
        }

        let point_at = |x, y| {
            area.translate_point(Point::new(x + area.gutter_width(), y), &window)
                .unwrap()
        };
        let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
        let held = PointerPointProperties::new(
            RawInputModifiers::LEFT_MOUSE_BUTTON,
            PointerUpdateKind::LeftButtonPressed,
        );
        let modifiers = if rectangular {
            super::box_selection_modifier()
        } else {
            KeyModifiers::NONE
        };
        let press = PointerPressedEventArgs::new(
            area.clone(),
            pointer.clone(),
            &window,
            point_at(19.0, height / 2.0),
            1,
            held,
            modifiers,
            1,
        );
        area.raise_event(&press);
        assert!(press.handled());
        let y = match (upward, outside) {
            (true, false) => 0.0,
            (true, true) => -5000.0,
            (false, false) => height,
            (false, true) => height + 5000.0,
        };
        let target = point_at(31.0, y);
        area.raise_event(&PointerEventArgs::new(
            Some(InputElement::pointer_moved_event()),
            area.clone(),
            pointer.clone(),
            Some(&window),
            target,
            2,
            held,
            modifiers,
        ));
        let snapshot = || (editor.selection(), editor.caret_position(), area.offset());
        let moved = snapshot();
        assert_eq!(moved.0.is_rectangular(), rectangular);
        assert!(editor.caret_offset() > 0 && editor.caret_offset() < editor.document().text_length());

        let direction = if upward { -1.0 } else { 1.0 };
        area.set_offset(Vector::new(0.0, moved.2.y + 60.0 * direction));
        Dispatcher::ui_thread().run_jobs(None);
        let scrolled = snapshot();
        assert_eq!(area.viewport().height, height);
        assert_ne!(scrolled.0, moved.0);
        area.raise_event(&PointerReleasedEventArgs::new(
            area.clone(),
            pointer,
            &window,
            target,
            3,
            PointerPointProperties::new(
                RawInputModifiers::NONE,
                PointerUpdateKind::LeftButtonReleased,
            ),
            modifiers,
            MouseButton::Left,
        ));
        let released = snapshot();
        window.close();
        (moved, scrolled, released)
    };
    for rectangular in [false, true] {
        for upward in [false, true] {
            assert_eq!(
                drag(rectangular, upward, false),
                drag(rectangular, upward, true),
                "rectangular={rectangular}, upward={upward}"
            );
        }
    }
}

#[test]
fn rectangular_paste_from_a_continuation_projects_the_first_insertion_too() {
    let _scope = test_scope();
    let source = "ab אבג 12345678901234567890 xxxxxx yyyyyyyyy zzzzzzzz\nabcdefghijklmnopqrstuv";
    let editor = laid_out_editor(source);
    let area = editor.text_area();
    let line2 = editor.document().get_line_by_number(2).unwrap().offset();
    let width = area.rectangle_for_offset(line2 + 16).unwrap().x + 10.0;
    editor.set_word_wrap(true);
    area.measure(Size::new(width, 300.0));
    area.arrange(Rect::new(0.0, 0.0, width, 300.0));
    let line = area.try_line_layout(0).unwrap();
    let start = line.layout.text_lines()[1].first_text_source_index() as usize + 2;
    assert_eq!(start, 9);
    editor.set_caret_offset(start).unwrap();
    editor.document().clear_undo_history().unwrap();
    assert!(area.paste_rectangle("X\nY", false).unwrap());
    assert_eq!(
        editor.text(),
        "abX אבג 12345678901234567890 xxxxxx yyyyyyyyy zzzzzzzz\nabYcdefghijklmnopqrstuv"
    );
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), source);
    assert!(!editor.undo().unwrap());
}

#[test]
fn document_space_floor_follows_logical_clusters_in_supplementary_and_rtl_text() {
    use crate::{TextViewPosition, VisualYPosition};

    let _scope = test_scope();
    for (text, start, end, right_to_left) in [("a😀b", 2, 4, false), ("ab אבג", 5, 6, true)] {
        let editor = laid_out_editor(text);
        let area = editor.text_area();
        let project = |column| {
            area.get_visual_position(TextViewPosition::new(1, column), VisualYPosition::LineMiddle)
                .unwrap()
        };
        let a = project(start);
        let b = project(end);
        assert_eq!(a.x > b.x, right_to_left);
        let point = Point::new(a.x + (b.x - a.x) * 0.8, a.y);
        assert_eq!(area.get_position_floor(point).unwrap().unwrap().column, start);
        assert_eq!(area.get_position(point).unwrap().unwrap().column, end);
    }
}

#[test]
fn document_space_floor_includes_the_row_terminator_after_markers_and_empty_rows() {
    use crate::{TextViewPosition, VisualYPosition};

    let _scope = test_scope();
    let editor = laid_out_editor("ab\nnext");
    let area = editor.text_area();
    let mut options = editor.options();
    options.show_end_of_line = true;
    editor.set_options(options);
    let hit = |line| {
        let point = area.get_visual_position(TextViewPosition::new(line, 1), VisualYPosition::LineMiddle).unwrap();
        let beyond = Point::new(point.x + 200.0, point.y);
        (area.get_position(beyond).unwrap().unwrap(), area.get_position_floor(beyond).unwrap().unwrap())
    };
    let (nearest, floor) = hit(1);
    assert_eq!((nearest.column, nearest.visual_column, nearest.is_at_end_of_line), (3, Some(3), false));
    assert_eq!((floor.column, floor.visual_column, floor.is_at_end_of_line), (3, Some(4), true));

    let mut options = editor.options();
    options.show_end_of_line = false;
    editor.set_options(options);
    editor.set_text("\n").unwrap();
    for number in [1, 2] {
        let (nearest, floor) = hit(number);
        assert_eq!((nearest.line, nearest.column, nearest.visual_column, nearest.is_at_end_of_line), (number, 1, Some(0), false));
        assert_eq!((floor.line, floor.column, floor.visual_column, floor.is_at_end_of_line), (number, 1, Some(1), true));
    }
}

#[test]
fn document_space_y_modes_distinguish_native_inline_box_and_text_metrics() {
    use crate::{TextViewPosition, VisualYPosition::*};
    use ferroui_controls::Button;

    let _scope = test_scope();
    let editor = laid_out_editor("inline\nnext");
    let area = editor.text_area();
    let button = Button::new();
    button.set_height(60.0);
    button.set_width(22.0);
    add_inline_object(&area, 0, 1, button.upcast());
    area.measure(Size::new(360.0, 180.0));
    area.arrange(Rect::new(0.0, 0.0, 360.0, 180.0));
    let position = TextViewPosition::new(1, 3);
    let y = |mode| area.get_visual_position(position, mode).unwrap().y;
    let metrics = area.default_text_metrics();
    assert_eq!(y(LineTop), 0.0);
    assert!(y(LineBottom) >= 60.0);
    assert!(y(TextTop) > y(LineTop));
    assert!((y(LineMiddle) - (y(LineTop) + y(LineBottom)) / 2.0).abs() < 0.001);
    assert!((y(TextMiddle) - (y(TextTop) + y(TextBottom)) / 2.0).abs() < 0.001);
    assert!((y(TextBottom) - y(TextTop) - metrics.height).abs() < 0.001);
    assert!((y(Baseline) - y(TextTop) - metrics.baseline).abs() < 0.001);
}

#[test]
fn editor_line_height_factor_centers_each_wrapped_row_and_updates_live_geometry() {
    use crate::{LineHeightFactor, TextViewPosition, VisualYPosition::*};

    let _scope = test_scope();
    let editor = laid_out_editor("abcdefghijklmnopqrstuv\nnext");
    editor.set_word_wrap(true);
    let area = editor.text_area();
    area.measure(Size::new(70.0, 400.0));
    area.arrange(Rect::new(0.0, 0.0, 70.0, 400.0));
    assert_eq!(editor.options().line_height_factor.get(), 1.16);
    for invalid in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert!(LineHeightFactor::new(invalid).is_err());
    }
    let snapshot = |factor| {
        let mut options = editor.options();
        options.line_height_factor = LineHeightFactor::new(factor).unwrap();
        editor.set_options(options);
        area.ensure_visual_lines().unwrap();
        area.cache.borrow()[&0].clone()
    };
    let natural = snapshot(0.5);
    let normal = snapshot(1.16);
    let spaced = snapshot(1.8);
    let natural_rows = natural.layout.text_lines();
    assert!(natural_rows.len() > 1);
    let close = |a: f64, b: f64| assert!((a - b).abs() < 0.001, "{a} != {b}");
    for (line, factor) in [(&normal, 1.16), (&spaced, 1.8)] {
        let metrics = area.default_text_metrics();
        let rows = line.layout.text_lines();
        assert_eq!(rows.len(), natural_rows.len());
        let mut total = 0.0;
        for (index, (natural, padded)) in natural_rows.iter().zip(rows).enumerate() {
            let expected = natural.height().max(metrics.height * factor);
            close(line.layout.line_origin(index).y, total);
            close(padded.height(), expected);
            close(padded.baseline(), natural.baseline() + (expected - natural.height()) / 2.0);
            close(padded.width_including_trailing_whitespace(), natural.width_including_trailing_whitespace());
            assert_eq!(padded.length(), natural.length());
            total += expected;
        }
        close(line.layout.height(), total);
    }
    let boundary = spaced.layout.text_lines()[1].first_text_source_index() as usize;
    let position = TextViewPosition::new(1, boundary + 1);
    let top = area.get_visual_position(position, LineTop).unwrap();
    let text = area.get_visual_position(position, TextTop).unwrap();
    assert!(text.y > top.y);
    assert_eq!(area.get_position_floor(top).unwrap().unwrap().column, boundary + 1);
    editor.set_caret_offset(boundary).unwrap();
    let caret = area.cursor_rectangle();
    close(caret.y, top.y);
    close(caret.height, spaced.layout.text_lines()[1].height());
    let rectangles = area.selection_rectangles(SelectionSegment::new(boundary, boundary + 1), false).unwrap();
    close(rectangles[0].y, top.y);
    close(rectangles[0].height, caret.height);
    close(area.get_visual_top_by_document_line(2).unwrap(), spaced.layout.height());
}

#[test]
fn editor_line_height_factor_preserves_tall_inline_controls_and_moves_short_ones_with_text() {
    use crate::LineHeightFactor;
    use ferroui_controls::Button;

    let _scope = test_scope();
    let editor = laid_out_editor("short\ntall\nlast");
    let area = editor.text_area();
    let short = Button::new();
    short.set_width(12.0);
    short.set_height(8.0);
    let tall = Button::new();
    tall.set_width(15.0);
    tall.set_height(60.0);
    add_inline_object(&area, 0, 1, short.clone().upcast());
    add_inline_object(&area, 6, 1, tall.clone().upcast());
    let measure = |factor| {
        let mut options = editor.options();
        options.line_height_factor = LineHeightFactor::new(factor).unwrap();
        editor.set_options(options);
        area.measure(Size::new(360.0, 400.0));
        area.arrange(Rect::new(0.0, 0.0, 360.0, 400.0));
        let cache = area.cache.borrow();
        let top = area.get_visual_top_by_document_line(2).unwrap();
        let child_y = |button: &Button| button.translate_point(Point::default(), &area).unwrap().y;
        (cache[&0].layout.height(), cache[&1].layout.height(), child_y(&short), child_y(&tall) - top)
    };
    let (natural_short, natural_tall, before_short, before_tall) = measure(0.5);
    let (padded_short, padded_tall, after_short, after_tall) = measure(1.8);
    assert!(natural_tall >= 60.0);
    assert_eq!(natural_tall, padded_tall);
    assert!((after_tall - before_tall).abs() < 0.001);
    assert!((after_short - before_short - (padded_short - natural_short) / 2.0).abs() < 0.001);
}

#[test]
fn document_space_queries_respect_scroll_visibility_and_collapsed_rows() {
    use crate::{TextViewPosition, VisualYPosition};

    let _scope = test_scope();
    let editor = laid_out_editor(&"short\n".repeat(80));
    let area = editor.text_area();
    let document = editor.document();
    let first = document.get_live_line_by_number(2).unwrap();
    let last = document.get_live_line_by_number(4).unwrap();
    let section = area.collapse_lines(&first, &last).unwrap();
    let y = area.get_visual_top_by_document_line(2).unwrap();
    for number in 3..=5 {
        assert_eq!(area.get_visual_top_by_document_line(number).unwrap(), y);
    }
    assert_eq!(area.get_visual_line_from_visual_top(y).unwrap().unwrap().first_document_line().line_number(), 5);
    assert!(area.get_position(Point::new(0.0, -1.0)).unwrap().is_none());
    assert!(area.get_visual_line_from_visual_top(f64::NAN).unwrap().is_none());
    let far = area.get_visual_position(TextViewPosition::new(35, 1), VisualYPosition::LineMiddle).unwrap();
    assert!(area.get_position(far).unwrap().is_none());
    area.set_offset(Vector::new(0.0, far.y - 20.0));
    assert_eq!(area.get_position(far).unwrap().unwrap().line, 35);
    assert!(area.get_position(Point::new(0.0, 0.0)).unwrap().is_none());
    section.uncollapse();
}

#[test]
fn collapse_each_inclusive_range_and_restore_measured_heights() {
    let _scope = test_scope();
    let editor = laid_out_editor("1\n2\n3\n4\n5\n6\n7\n8\n9\n10");
    let area = editor.text_area();
    let document = editor.document();
    let total: f64 = (1..=10).map(f64::from).sum();
    for number in 1..=10 {
        area.heights.borrow_mut().set_height(number - 1, number as f64);
    }
    for first in 1..=10 {
        for last in first..=10 {
            let start = document.get_live_line_by_number(first).unwrap();
            let end = document.get_live_line_by_number(last).unwrap();
            let section = area.collapse_lines(&start, &end).unwrap();
            assert_eq!(section.start(), Some(start));
            assert_eq!(section.end(), Some(end));
            let hidden: f64 = (first..=last).map(|number| number as f64).sum();
            assert!((area.document_height() - (total - hidden)).abs() < 0.001);
            let mut top = 0.0;
            for number in (1..=10).filter(|number| !(first..=last).contains(number)) {
                let line = area.get_document_line_by_visual_top(top + 0.5).unwrap();
                assert_eq!(line.line_number().unwrap(), number);
                top += number as f64;
            }
            if first == 1 && last == 10 {
                assert!(area.get_document_line_by_visual_top(0.0).is_none());
                assert!(matches!(
                    area.get_or_construct_visual_line(1),
                    Err(VisualLineError::NoVisibleLine { .. })
                ));
                render_editor(&editor);
                assert!(area.visual_lines().is_empty());
            }
            section.uncollapse();
            section.uncollapse();
            assert!(!section.is_collapsed());
            assert!(section.start().is_none() && section.end().is_none());
            assert!((area.document_height() - total).abs() < 0.001);
        }
    }
}

#[test]
fn collapsed_sections_track_insertions_and_boundary_removals() {
    let _scope = test_scope();
    let editor = laid_out_editor("1\n2\n3\n4\n5\n6\n7\n8\n9\n10");
    let area = editor.text_area();
    let document = editor.document();
    let lines = document.live_lines();
    let collapse = area.collapse_lines(&lines[3], &lines[5]).unwrap();
    document.insert(lines[4].offset().unwrap(), "a\nb\nc").unwrap();
    assert_eq!(collapse.start(), Some(lines[3].clone()));
    assert_eq!(collapse.end(), Some(lines[5].clone()));
    let hidden = || {
        (0..document.line_count()).filter(|&index| area.heights.borrow().is_hidden(index))
            .map(|index| index + 1).collect::<Vec<_>>()
    };
    assert_eq!(hidden(), [4, 5, 6, 7, 8]);
    let after_first = lines[3].next_line().unwrap();
    document.remove(lines[3].offset().unwrap() - 1, 1).unwrap();
    assert_eq!(collapse.start(), Some(after_first));
    assert_eq!(hidden(), [4, 5, 6, 7]);
    let before_last = lines[5].previous_line().unwrap();
    document.remove(lines[5].offset().unwrap() - 1, 1).unwrap();
    assert_eq!(collapse.end(), Some(before_last));
    assert_eq!(hidden(), [4, 5, 6]);
    let single = area.collapse_lines(&lines[9], &lines[9]).unwrap();
    document.remove(lines[9].offset().unwrap() - 1, 1).unwrap();
    assert!(!single.is_collapsed());
    let text = document.text();
    document.replace(0, document.text_length(), &format!("new\n{text}")).unwrap();
    assert!(!collapse.is_collapsed());
}

#[test]
fn collapsed_sections_keep_surviving_rows_after_multiline_deletion() {
    let _scope = test_scope();
    for (last, remove_first, remove_last) in [(7, 4, 6), (6, 5, 8)] {
        let editor = laid_out_editor("1\n2\n3\n4\n5\n6\n7\n8\n9\n10");
        let area = editor.text_area();
        let document = editor.document();
        let line = |number| document.get_live_line_by_number(number).unwrap();
        let section = area.collapse_lines(&line(3), &line(last)).unwrap();
        let start = line(remove_first).offset().unwrap();
        let end = line(remove_last).offset().unwrap();
        document.remove(start, end - start).unwrap();
        assert_eq!(section.start().unwrap().line_number(), Ok(3));
        assert_eq!(section.end().unwrap().line_number(), Ok(5));
        let hidden: Vec<_> = (0..document.line_count())
            .filter(|&index| area.heights.borrow().is_hidden(index))
            .map(|index| index + 1)
            .collect();
        assert_eq!(hidden, [3, 4, 5]);
    }
}

#[test]
fn overlapping_collapses_are_independent_and_belong_to_one_view() {
    let _scope = test_scope();
    let editor = laid_out_editor("1\n2\n3\n4\n5\n6\n7");
    let other_editor = laid_out_editor("other");
    let area = editor.text_area();
    let document = editor.document();
    let lines = document.live_lines();
    let other_line = other_editor.document().get_live_line_by_number(1).unwrap();
    assert!(matches!(area.collapse_lines(&lines[0], &other_line), Err(VisualLineError::Document(DocumentError::DifferentDocument))));
    assert!(matches!(area.collapse_lines(&lines[3], &lines[2]), Err(VisualLineError::InvalidCollapseRange { .. })));
    other_editor.set_document(document.clone());
    let a = area.collapse_lines(&lines[1], &lines[3]).unwrap();
    let b = area.collapse_lines(&lines[2], &lines[5]).unwrap();
    assert_eq!(area.heights.borrow().next_visible(1), 6);
    assert_eq!(other_editor.text_area().heights.borrow().next_visible(1), 1);
    a.clone().uncollapse();
    assert!(!a.is_collapsed());
    assert!(b.is_collapsed());
    assert_eq!(area.heights.borrow().next_visible(1), 1);
    assert_eq!(area.heights.borrow().next_visible(2), 6);
    editor.set_document(TextDocument::from_text("replacement"));
    assert!(!b.is_collapsed());
    assert_eq!(area.get_document_line_by_visual_top(0.0).unwrap().line_number(), Ok(1));
}

#[test]
fn generator_can_collapse_live_rows_during_construction() {
    let _scope = test_scope();
    let editor = laid_out_editor("a{\n hidden\n}z\nlast\nrest\nend");
    let area = editor.text_area();
    let document = editor.document();
    let lines = document.live_lines();
    let separate = area.collapse_lines(&lines[4], &lines[4]).unwrap();
    let handle = Rc::new(RefCell::new(None));
    let stored = handle.clone();
    let weak = area.downgrade();
    let generator = Rc::new(ProbeGenerator::new("span", Rc::new(RefCell::new(Vec::new()))).element_at(1, 11));
    generator.on_start.replace(Some(Box::new(move |_| {
        let area = weak.upgrade().unwrap();
        let section = area.collapse_lines(&lines[1], &lines[2]).unwrap();
        stored.replace(Some(section));
    })));
    area.element_generators().add(generator.into());
    let line = area.get_or_construct_visual_line(1).unwrap();
    render_editor(&editor);
    assert_eq!(line.last_document_line().line_number(), 3);
    assert_eq!(area.heights.borrow().next_visible(1), 3);
    assert_eq!(area.heights.borrow().next_visible(4), 5);
    handle.borrow().as_ref().unwrap().uncollapse();
    assert_eq!(area.heights.borrow().next_visible(1), 3);
    area.element_generators().clear();
    assert_eq!(area.heights.borrow().next_visible(1), 1);
    assert!(separate.is_collapsed());
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
fn installed_folds_take_precedence_over_extensions_at_the_same_offset() {
    let _scope = test_scope();
    let editor = laid_out_editor("a{\n hidden\n}z\nlast");
    let area = editor.text_area();
    let probe = Rc::new(ProbeGenerator::new("custom", Rc::default()).element_at(1, 1));
    area.element_generators().add(probe.into());
    let fold = editor.install_folding().create_folding(1, 12).unwrap();
    fold.set_is_folded(true);
    let line = area.get_or_construct_visual_line(1).unwrap();
    assert!(line.elements().iter().any(|element| element.as_any().is::<crate::FoldingLineElement>()));
    assert!(!line.elements().iter().any(|element| element.as_any().is::<FormattedTextElement>()));
    fold.set_is_folded(false);
    let line = area.get_or_construct_visual_line(1).unwrap();
    assert!(line.elements().iter().any(|element| element.as_any().is::<FormattedTextElement>()));
}

#[test]
fn installed_fold_resumes_after_a_custom_element_crosses_its_start() {
    let _scope = test_scope();
    let text = "a01234\r\nmid\r\nendZ\r\nlast";
    for resume_at in [5, 6, 8] {
        let editor = laid_out_editor(text);
        let area = editor.text_area();
        let probe = Rc::new(
            ProbeGenerator::new("custom", Rc::default()).element_at(1, resume_at - 1),
        );
        area.element_generators().add(probe.into());
        let fold = editor.install_folding().create_folding(3, 16).unwrap();
        fold.set_is_folded(true);

        let line = area.get_or_construct_visual_line(1).unwrap();
        assert_eq!(line.last_document_line().line_number(), 3, "{resume_at}");
        let marker = line.elements()[2]
            .as_any()
            .downcast_ref::<crate::FoldingLineElement>()
            .unwrap();
        assert_eq!(marker.data().relative_text_offset(), resume_at);
        assert_eq!(marker.data().end_relative_text_offset(), 16);
        assert_eq!(line.elements()[3].data().document_length(), 1);
        assert!(Rc::ptr_eq(&line, &area.get_or_construct_visual_line(3).unwrap()));
        let document = editor.document();
        assert_eq!(area.automation_rows(&document, 9).unwrap(), vec![0..19]);
        assert_eq!(area.automation_visibility(&document).unwrap().hidden, vec![3..16]);

        assert!(area.rectangle_for_offset(3).is_none());
        assert!(area.rectangle_for_offset(9).is_none());
        let marker_start = area.rectangle_for_offset(resume_at).unwrap();
        let suffix = area.rectangle_for_offset(16).unwrap();
        assert_eq!(marker_start.y, suffix.y);
        assert!(marker_start.x < suffix.x);
        assert_eq!(
            area.offset_at_point(Point::new(suffix.x, suffix.y + suffix.height / 2.0)),
            16
        );

        editor.set_caret_offset(1).unwrap();
        for (key, expected) in [
            (Key::Right, resume_at),
            (Key::Right, 16),
            (Key::Right, 17),
            (Key::Left, 16),
            (Key::Left, resume_at),
            (Key::Left, 1),
        ] {
            let mut event = KeyEventArgs::new();
            event.key = key;
            event.set_routed_event(Some(InputElement::key_down_event()));
            area.raise_event(&event);
            assert!(event.handled());
            assert_eq!(editor.caret_offset(), expected, "{resume_at}");
            assert!(fold.is_folded(), "{resume_at} {expected}");
        }

        editor.set_caret_offset(9).unwrap();
        assert!(!fold.is_folded());
        assert_eq!(editor.text(), text);
    }
}

#[test]
fn single_line_fold_hides_only_the_tail_left_by_a_custom_element() {
    let _scope = test_scope();
    for (custom_end, expected) in [(5, Some(5..7)), (7, None)] {
        let editor = laid_out_editor("a0123456z");
        let area = editor.text_area();
        let probe = Rc::new(
            ProbeGenerator::new("custom", Rc::default()).element_at(1, custom_end - 1),
        );
        area.element_generators().add(probe.into());
        let fold = editor.install_folding().create_folding(3, 7).unwrap();
        fold.set_is_folded(true);

        let line = area.get_or_construct_visual_line(1).unwrap();
        let marker = line.elements().iter().find(|element| {
            element.as_any().is::<crate::FoldingLineElement>()
        });
        assert_eq!(marker.map(|element| {
            element.data().relative_text_offset()..element.data().end_relative_text_offset()
        }), expected);
        let suffix = line.elements().last().unwrap().data();
        assert_eq!(suffix.relative_text_offset(), 7);
        assert_eq!(suffix.document_length(), 2);
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
                .push(event.uri.as_str().to_owned());
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
    assert_eq!(*received.borrow(), ["https://example.org/"]);
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
fn live_folding_range_changes_refresh_the_installed_placeholder() {
    let _scope = test_scope();
    let editor = laid_out_editor("zero\none line\ntwo line\nthree line\nfour");
    let area = editor.text_area();
    let manager = editor.install_folding();
    let fold = manager.create_folding(1, 9).unwrap();
    fold.set_is_folded(true);
    let marker_length = |line_number| {
        let line = area.get_or_construct_visual_line(line_number).unwrap();
        let marker = line
            .elements()
            .iter()
            .find(|element| element.as_any().is::<crate::FoldingLineElement>())
            .map(|element| element.data().document_length());
        (line.last_document_line().line_number(), marker)
    };
    assert_eq!(marker_length(1), (2, Some(8)));
    fold.set_start_offset(15).unwrap();
    fold.set_end_offset(32).unwrap();
    assert_eq!(marker_length(1), (1, None));
    assert_eq!(marker_length(3), (4, Some(17)));
    editor.document().replace(15, 17, "😀\nQ").unwrap();
    assert_eq!(marker_length(3), (4, Some(4)));
    editor.document().remove(15, 4).unwrap();
    assert!(!fold.is_folded());
    assert_eq!(marker_length(3), (3, None));
}

#[test]
fn crossing_fold_placeholders_keep_the_first_title_and_activate_its_section() {
    use ferroui_base::input::{
        Pointer, PointerPointProperties, PointerType, PointerUpdateKind, RawInputModifiers,
    };
    use ferroui_controls::Window;
    let _scope = test_scope();
    let editor = laid_out_editor("abcd\nEFGH\nijkl\nMNOP\nqrst\nUVWX");
    let manager = editor.install_folding();
    let closed = |start, end, title: &str| {
        let fold = manager.create_folding(start, end).unwrap();
        fold.set_title(Some(title.into()));
        fold.set_is_folded(true);
        fold
    };
    let first = closed(1, 13, "First 😀");
    let second = closed(1, 13, "Second and longer");
    let short = closed(1, 7, "short");
    let crossing = closed(9, 20, "crossing");
    let touching = closed(20, 22, "touching");
    let disjoint = closed(24, 27, "disjoint");
    let window = Window::new();
    window.set_content(Some(Control::boxed(editor.clone())));
    window.show();
    let area = editor.text_area();
    let markers = || {
        let line = area.get_or_construct_visual_line(1).unwrap();
        assert_eq!(line.last_document_line().line_number(), 6);
        line.elements()
            .iter()
            .filter(|element| element.as_any().is::<crate::FoldingLineElement>())
            .map(|element| {
                (
                    element.data().relative_text_offset(),
                    element.data().document_length(),
                    element.display_column_length(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(markers(), [(1, 21, 8), (24, 3, 8)]);
    let start = area.rectangle_for_offset(1).unwrap();
    let end = area.rectangle_for_offset(22).unwrap();
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
    assert!(!first.is_folded());
    assert!(
        [second, short, crossing, touching, disjoint]
            .iter()
            .all(|fold| fold.is_folded())
    );
    assert_eq!(markers(), [(1, 21, 17), (24, 3, 8)]);
    assert_eq!(editor.selection(), selection);
    window.close();
}

#[test]
fn an_edit_keeps_the_measured_height_of_wrapped_lines_it_did_not_touch() {
    let _scope = test_scope();
    let wrapped = "alpha beta gamma delta epsilon zeta eta theta iota kappa";
    let editor = laid_out_editor(&format!("{wrapped}\nsecond\nthird\nfourth"));
    editor.set_word_wrap(true);
    let area = editor.text_area();
    area.measure(Size::new(135.0, 400.0));
    area.arrange(Rect::new(0.0, 0.0, 135.0, 400.0));
    area.ensure_visual_lines().unwrap();
    let measured = area.heights.borrow().top(1);
    assert!(measured > area.line_height() * 1.5);

    let end = editor.document().text_length();
    editor.document().insert(end, "x").unwrap();
    assert_eq!(area.heights.borrow().top(1), measured);

    editor.document().insert(end, "\nfifth").unwrap();
    assert_eq!(area.heights.borrow().line_count(), 5);
    assert_eq!(area.heights.borrow().top(1), measured);
}
