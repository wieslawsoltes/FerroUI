use super::*;
use ferroui_base::Ref;
use ferroui_base::input::platform::{Clipboard, ClipboardErrorKind, IClipboardImpl};
use ferroui_base::input::{IAsyncDataTransferItem, ICommand};
use ferroui_base::reactive::IDisposable;
use ferroui_controls::testing::TestClipboardImpl;
use ferroui_edit_core::document::TextDocument;
use ferroui_edit_core::editing::{SelectionSegment, TextSegmentReadOnlySectionProvider};
use ferroui_edit_core::highlighting::HighlightingDefinition;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::future::poll_fn;
use std::task::{Context, Poll, Waker};

fn ready<T>(mut future: LocalBoxFuture<T>) -> T {
    match future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("Expected a synchronously completed test operation"),
    }
}

fn editor_with_text(text: &str) -> Ref<TextEditor> {
    let editor = TextEditor::new();
    editor.set_document(TextDocument::from_text(text));
    editor
}

fn selected_editor(text: &str) -> Ref<TextEditor> {
    let editor = editor_with_text(text);
    editor.select_all();
    editor
}

fn current_html(item: &DataTransferItem) -> String {
    match HtmlPlatform::current() {
        HtmlPlatform::Mime => item
            .try_get_value(&DataFormat::create_string_platform_format("text/html"))
            .unwrap(),
        HtmlPlatform::Apple => item
            .try_get_value(&DataFormat::create_string_platform_format("public.html"))
            .unwrap(),
        HtmlPlatform::Windows => String::from_utf8(
            item.try_get_value(&DataFormat::create_bytes_platform_format("HTML Format"))
                .unwrap()
                .to_vec(),
        )
        .unwrap(),
    }
}

fn clipboard_with_payload(text: &str, kind: ClipboardSelectionKind) -> Rc<dyn IClipboard> {
    let platform = TestClipboardImpl::new();
    let clipboard = platform.clipboard();
    let item = DataTransferItem::create_text(Some(text));
    match kind {
        ClipboardSelectionKind::Rectangular => item.set(
            &EditorClipboard::rectangular_selection_format(),
            Some("1".to_owned()),
        ),
        ClipboardSelectionKind::WholeLine => item.set(
            &EditorClipboard::whole_line_selection_format(),
            Some("1".to_owned()),
        ),
        ClipboardSelectionKind::Plain => {}
    }
    let data = DataTransfer::new();
    data.add(item);
    ready(clipboard.set_data_async(Some(data))).unwrap();
    clipboard
}

fn editor_with_paste_rules(text: &str) -> Ref<TextEditor> {
    use ferroui_textmate::{LanguageConfiguration, Registry, Theme};

    let editor = editor_with_text(text);
    let mut registry = Registry::new();
    registry.add_grammar_json(r#"{"scopeName":"source.paste-test","patterns":[
        {"begin":"\"","end":"\"","name":"string.quoted.test"}
    ]}"#).unwrap();
    let configuration = LanguageConfiguration::from_json(r#"{
        "brackets":[["{","}"]],
        "indentationRules":{"increaseIndentPattern":"\\{$","decreaseIndentPattern":"^\\s*\\}"}
    }"#).unwrap();
    editor.install_textmate(registry.load_grammar("source.paste-test").unwrap(), Theme::from_json("{}").unwrap())
        .set_language_configuration(Some(&configuration)).unwrap();
    editor
}

#[test]
fn optional_paste_indentation_preserves_relative_offsets_delimiters_and_undo() {
    let _scope = crate::tests::test_scope();
    let original = "if (outer) {\r\n\t\t\r\n}\r\n";
    let prefix = "if (outer) {\r\n\t\t";
    let payload = "if (value) {\n\twork😀();\r\t  deeper();\r\n}";
    let editor = editor_with_paste_rules(original);
    editor.set_caret_offset(prefix.len()).unwrap();
    let clipboard = clipboard_with_payload(payload, ClipboardSelectionKind::Plain);
    assert!(!editor.options().auto_indent_on_paste);
    assert_eq!(ready(EditorClipboard::paste_from_clipboard(&editor, clipboard.clone())).unwrap(), ClipboardPasteOutcome::Pasted);
    assert_eq!(editor.text(), format!("{prefix}{payload}\r\n}}\r\n"));
    assert!(editor.undo().unwrap());

    let mut options = editor.options();
    options.auto_indent_on_paste = true;
    editor.set_options(options);
    let reported = Rc::new(RefCell::new(String::new()));
    let received = reported.clone();
    let _subscription = editor.add_disposable_handler(TextEditor::text_pasted_event(), move |_, args| {
        *received.borrow_mut() = args.text().to_owned();
    });
    assert_eq!(ready(EditorClipboard::paste_from_clipboard(&editor, clipboard)).unwrap(), ClipboardPasteOutcome::Pasted);
    let adjusted = "\tif (value) {\n\t\twork😀();\r\t\t  deeper();\r\n\t}";
    let expected = format!("if (outer) {{\r\n{adjusted}\r\n}}\r\n");
    assert_eq!(editor.text(), expected);
    assert_eq!(&*reported.borrow(), adjusted);
    assert_eq!(editor.caret_offset(), expected.encode_utf16().count() - "\r\n}\r\n".len());
    assert_eq!(editor.document().undo_count(), 1);
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), original);
    assert_eq!(editor.caret_offset(), prefix.len());
    assert!(editor.redo().unwrap());
    assert_eq!(editor.text(), expected);
}

#[test]
fn paste_indentation_falls_back_for_protected_prefixes_and_string_boundaries() {
    let _scope = crate::tests::test_scope();
    for (prefix, protected) in [("if (outer) {\n\t\t", true), ("if (outer) {\n\t\t\"inside", false)] {
        let original = format!("{prefix}\n}}");
        let editor = editor_with_paste_rules(&original);
        let mut options = editor.options();
        options.auto_indent_on_paste = true;
        options.auto_indent_on_paste_within_string = false;
        editor.set_options(options);
        editor.set_caret_offset(prefix.len()).unwrap();
        if protected {
            let provider = Rc::new(TextSegmentReadOnlySectionProvider::new(editor.document()));
            provider.add(prefix.len() - 2, 1).unwrap();
            editor.session().set_read_only_section_provider(provider);
        }
        let payload = "if (inner) {\n\twork();\n}";
        let clipboard = clipboard_with_payload(payload, ClipboardSelectionKind::Plain);
        assert_eq!(ready(EditorClipboard::paste_from_clipboard(&editor, clipboard)).unwrap(), ClipboardPasteOutcome::Pasted);
        assert_eq!(editor.text(), format!("{prefix}{payload}\n}}"));
        assert_eq!(editor.document().undo_count(), 1);
    }
}

#[test]
fn paste_indentation_keeps_the_original_payload_when_a_line_exceeds_the_textmate_limit() {
    let _scope = crate::tests::test_scope();
    let prefix = "if (outer) {\n\t\t";
    let original = format!("{prefix}\n}}");
    let editor = editor_with_paste_rules(&original);
    let mut options = editor.options();
    options.auto_indent_on_paste = true;
    editor.set_options(options);
    editor.set_caret_offset(prefix.len()).unwrap();

    let limit = editor.textmate_installation().unwrap().max_tokenized_line_length();
    let payload = format!("if (inner) {{\n\t{}\n}}", "😀".repeat(limit / 2));
    let clipboard = clipboard_with_payload(&payload, ClipboardSelectionKind::Plain);
    assert_eq!(
        ready(EditorClipboard::paste_from_clipboard(&editor, clipboard)).unwrap(),
        ClipboardPasteOutcome::Pasted
    );
    assert_eq!(editor.text(), format!("{prefix}{payload}\n}}"));
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), original);
}

#[test]
fn large_paste_indentation_yields_and_keeps_a_stale_selection_untouched() {
    let _scope = crate::tests::test_scope();
    let editor = editor_with_paste_rules("outer {\n\t\t\n}");
    let mut options = editor.options();
    options.auto_indent_on_paste = true;
    editor.set_options(options);
    let initial = editor.text();
    let destination = "outer {\n\t\t".len();
    editor.set_caret_offset(destination).unwrap();
    let clipboard = clipboard_with_payload(&format!("inner {{\n{}}}", "\twork();\n".repeat(75)), ClipboardSelectionKind::Plain);
    let mut paste = EditorClipboard::paste_from_clipboard(&editor, clipboard.clone());
    let mut context = Context::from_waker(Waker::noop());
    assert!(paste.as_mut().poll(&mut context).is_pending());
    assert_eq!(editor.text(), initial);
    editor.set_caret_offset(0).unwrap();
    Dispatcher::ui_thread().run_jobs(None);
    assert!(matches!(paste.as_mut().poll(&mut context), Poll::Ready(Err(EditorClipboardError::Edit(EditError::StaleEditPlan)))));
    assert_eq!(editor.text(), initial);
    assert_eq!(editor.document().undo_count(), 0);

    editor.set_caret_offset(destination).unwrap();
    let mut paste = EditorClipboard::paste_from_clipboard(&editor, clipboard);
    for _ in 0..3 {
        assert!(paste.as_mut().poll(&mut context).is_pending());
        assert_eq!(editor.text(), initial);
        Dispatcher::ui_thread().run_jobs(None);
        // There are two background boundaries in this 77-line clipboard block.
        if paste.as_mut().poll(&mut context).is_ready() { break; }
    }
    let expected = format!("outer {{\n\tinner {{\n{}\t}}\n}}", "\t\twork();\n".repeat(75));
    assert_eq!(editor.text(), expected);
    assert_eq!(editor.document().undo_count(), 1);
}

#[test]
fn prepared_copy_keeps_text_exact_and_publishes_html_on_the_same_native_item() {
    let _scope = crate::tests::test_scope();
    let text = "a😀\t<&\r\nβ";
    let editor = selected_editor(text);
    let payload = EditorClipboard::prepare_copy(&editor).unwrap().unwrap();
    assert_eq!(payload.text, text);
    assert_eq!(payload.kind, ClipboardSelectionKind::Plain);
    assert!(payload.formatting_error.is_none());
    assert_eq!(payload.data.items().len(), 1);
    let item = &payload.data.items()[0];
    assert_eq!(
        item.try_get_value(&DataFormat::text()).as_deref(),
        Some(text)
    );
    assert_eq!(payload.data.formats().len(), 2);
    assert!(
        payload
            .data
            .formats()
            .contains(&EditorClipboard::html_format())
    );
    assert!(
        !payload
            .data
            .formats()
            .contains(&EditorClipboard::rectangular_selection_format().into())
    );
    let html = current_html(item);
    assert!(html.contains("😀"));
    assert!(html.contains("&lt;&amp;"));
    assert!(html.contains("&nbsp;"));
    assert_eq!(editor.text(), text);
    assert!(!editor.document().can_undo());
}

#[test]
fn rich_and_plain_copy_share_the_sessions_validated_newline_and_tab_options() {
    let _scope = crate::tests::test_scope();
    let editor = editor_with_text("a\t-b");
    let mut options = editor.options();
    options.indentation_size = 0;
    options.newline.clear();
    editor.set_options(options);
    editor
        .set_selection(Selection::from_segments(
            [SelectionSegment::new(0, 2), SelectionSegment::new(3, 4)],
            0,
        ))
        .unwrap();
    let data = EditorClipboard::prepare_copy(&editor).unwrap().unwrap();
    assert_eq!(data.text, "a\t\nb");
    assert_eq!(
        EditorClipboard::selected_rich_text(&editor).unwrap().text(),
        data.text
    );
    assert!(data.formatting_error.is_none());
    assert!(current_html(&data.data.items()[0]).contains("&nbsp;<br/>b"));
}

#[test]
fn html_platform_formats_use_native_mime_apple_or_utf8_cf_html_bytes() {
    let fragment = "<span>é😀</span>";
    for platform in [
        HtmlPlatform::Mime,
        HtmlPlatform::Apple,
        HtmlPlatform::Windows,
    ] {
        let item = DataTransferItem::new();
        platform.set(&item, fragment).unwrap();
        assert_eq!(item.formats().as_ref(), &[platform.format()]);
        match platform {
            HtmlPlatform::Mime => assert_eq!(
                item.try_get_value(&DataFormat::create_string_platform_format("text/html"))
                    .as_deref(),
                Some(fragment),
            ),
            HtmlPlatform::Apple => assert_eq!(
                item.try_get_value(&DataFormat::create_string_platform_format("public.html"))
                    .as_deref(),
                Some(fragment),
            ),
            HtmlPlatform::Windows => {
                let bytes = item
                    .try_get_value(&DataFormat::create_bytes_platform_format("HTML Format"))
                    .unwrap();
                let text = std::str::from_utf8(&bytes).unwrap();
                let offset = |name: &str| -> usize {
                    text.lines()
                        .find_map(|line| line.strip_prefix(name))
                        .unwrap()
                        .trim()
                        .parse()
                        .unwrap()
                };
                assert_eq!(
                    &bytes[offset("StartFragment:")..offset("EndFragment:")],
                    fragment.as_bytes()
                );
                assert_eq!(offset("EndHTML:"), bytes.len());
            }
        }
    }
}

#[test]
fn xshd_copy_retains_full_runtime_style_and_resolves_selected_ranges() {
    let _scope = crate::tests::test_scope();
    let editor = selected_editor("before word after");
    editor.select(7, 4).unwrap();
    let definition = HighlightingDefinition::load(r##"<SyntaxDefinition xmlns="http://icsharpcode.net/sharpdevelop/syntaxdefinition/2008"><Color name="keyword" foreground="#80112233" background="#445566" fontFamily="Test Sans" fontWeight="SemiBold" fontStyle="Oblique" fontSize="19" underline="true" strikethrough="true"/><RuleSet><Rule color="keyword">word</Rule></RuleSet></SyntaxDefinition>"##).unwrap();
    editor.set_syntax_highlighting(Some(Rc::new(definition)));
    let rich = EditorClipboard::selected_rich_text(&editor).unwrap();
    assert_eq!(rich.text(), "word");
    let style = rich.get_highlighted_sections(0, 4).unwrap().remove(0).color;
    assert_eq!(style.font_weight, Some(600));
    assert_eq!(style.font_family.as_deref(), Some("Test Sans"));
    assert_eq!(
        style.font_style,
        Some(ferroui_edit_core::highlighting::HighlightingFontStyle::Oblique)
    );
    assert_eq!(style.strikethrough, Some(true));
    let html = current_html(
        &EditorClipboard::prepare_copy(&editor)
            .unwrap()
            .unwrap()
            .data
            .items()[0],
    );
    assert!(html.contains("color: #11223380;"));
    assert!(html.contains("font-weight: 600;"));
    assert!(html.contains("font-style: oblique;"));
    assert!(html.contains("text-decoration: underline line-through;"));
    assert!(html.contains("font-family: &quot;Test Sans&quot;;"));
}

#[test]
fn textmate_copy_uses_utf16_token_ranges_and_overrides_xshd() {
    let _scope = crate::tests::test_scope();
    let editor = editor_with_text("a😀b");
    editor.set_syntax_highlighting(Some(Rc::new(
        HighlightingDefinition::load(r#"<SyntaxDefinition><RuleSet><Rule foreground="Red">.</Rule></RuleSet></SyntaxDefinition>"#).unwrap(),
    )));
    let mut registry = ferroui_textmate::Registry::new();
    registry.add_grammar_json(r#"{"scopeName":"source.clipboard","patterns":[{"match":"😀","name":"constant.emoji"}]}"#).unwrap();
    let grammar = registry.load_grammar("source.clipboard").unwrap();
    let theme = ferroui_textmate::Theme::from_json(r##"{"tokenColors":[{"scope":"constant.emoji","settings":{"foreground":"#12345680","fontStyle":"bold italic underline strikethrough"}}]}"##).unwrap();
    editor.install_textmate(grammar, theme);
    editor.select(1, 2).unwrap();
    let rich = EditorClipboard::selected_rich_text(&editor).unwrap();
    assert_eq!(rich.text(), "😀");
    let style = rich.get_highlighted_sections(0, 2).unwrap().remove(0).color;
    assert_eq!(style.foreground.as_deref(), Some("#80123456"));
    assert_eq!(style.bold, Some(true));
    assert_eq!(style.italic, Some(true));
    assert_eq!(style.strikethrough, Some(true));
    let html = current_html(
        &EditorClipboard::prepare_copy(&editor)
            .unwrap()
            .unwrap()
            .data
            .items()[0],
    );
    assert!(html.contains("#12345680"));
    assert!(html.contains("😀"));
    assert!(style.background.is_none());
    editor.select(0, 4).unwrap();
    let rich = EditorClipboard::selected_rich_text(&editor).unwrap();
    let sections = rich.get_highlighted_sections(0, 4).unwrap();
    assert_eq!(sections.len(), 3);
    assert_eq!(sections[1].offset, 1);
    assert_eq!(sections[1].length, 2);
    assert_eq!(sections[1].color, style);
    for section in [&sections[0], &sections[2]] {
        assert_eq!(section.color, HighlightingColor::default());
    }
}

#[test]
fn failed_rich_format_retains_plain_text_and_reports_the_error_to_copying_handlers() {
    let _scope = crate::tests::test_scope();
    let editor = selected_editor("word");
    let definition = HighlightingDefinition::load(r#"<SyntaxDefinition><RuleSet><Rule foreground="resource:MissingClipboardBrush">word</Rule></RuleSet></SyntaxDefinition>"#).unwrap();
    editor.set_syntax_highlighting(Some(Rc::new(definition)));
    let recorded = Rc::new(RefCell::new(None));
    let result = recorded.clone();
    let _subscription = editor.add_disposable_handler(TextEditor::copying_to_clipboard_event(), move |_, args| {
        *result.borrow_mut() = args.formatting_error().map(str::to_owned);
        assert_eq!(args.data().formats().len(), 1);
    });
    let platform = TestClipboardImpl::new();
    assert_eq!(
        ready(EditorClipboard::copy_to_clipboard(
            &editor,
            platform.clipboard(),
            false
        ))
        .unwrap(),
        ClipboardCopyOutcome::Copied
    );
    assert_eq!(platform.text().as_deref(), Some("word"));
    assert!(
        recorded
            .borrow()
            .as_ref()
            .unwrap()
            .contains("MissingClipboardBrush")
    );
    assert_eq!(editor.last_error(), recorded.borrow().clone());
}

#[test]
fn copy_customization_replaces_all_formats_and_copied_event_uses_custom_plain_text() {
    let _scope = crate::tests::test_scope();
    let editor = selected_editor("selected");
    let events = Rc::new(RefCell::new(Vec::new()));
    let raised = events.clone();
    let _subscription = editor.add_disposable_handler(TextEditor::copying_to_clipboard_event(), move |_, args| {
        raised.borrow_mut().push("copying".to_owned());
        let replacement = DataTransfer::new();
        replacement.add(DataTransferItem::create_text(Some("custom")));
        let clone = args.clone();
        clone.set_data(replacement);
        assert_eq!(args.text().as_deref(), Some("custom"));
    });
    let raised = events.clone();
    let _subscription = editor.add_disposable_handler(TextEditor::text_copied_event(), move |_, args| {
        raised.borrow_mut().push(args.text().unwrap().to_owned());
        assert!(!args.is_cut());
    });
    let raised = events.clone();
    let _area_subscription = editor.text_area().add_disposable_handler(
        TextArea::text_copied_event(), move |_, args| {
            assert_eq!(args.text(), Some("custom"));
            raised.borrow_mut().push("area copied".to_owned());
        },
    );
    let platform = TestClipboardImpl::new();
    let operation = EditorClipboard::copy_to_clipboard(&editor, platform.clipboard(), false);
    assert_eq!(*events.borrow(), vec!["copying"]);
    assert_eq!(platform.set_data_count(), 0);
    assert_eq!(ready(operation).unwrap(), ClipboardCopyOutcome::Copied);
    assert_eq!(*events.borrow(), vec!["copying", "area copied", "custom"]);
    assert_eq!(platform.text().as_deref(), Some("custom"));
    assert_eq!(editor.text(), "selected");
}

#[test]
fn copying_handler_can_inspect_then_add_an_item_format_without_stale_platform_metadata() {
    let _scope = crate::tests::test_scope();
    let editor = selected_editor("selected");
    let format = DataFormat::create_string_application_format("FerroUI.Edit.Test.Metadata");
    let custom = format.clone();
    let _subscription = editor.add_disposable_handler(TextEditor::copying_to_clipboard_event(), move |_, args| {
        let data = args.data();
        assert!(!data.formats().contains(&custom.clone().into()));
        data.items()[0].set(&custom, Some("custom".to_owned()));
    });
    let platform = TestClipboardImpl::new();
    ready(EditorClipboard::copy_to_clipboard(
        &editor,
        platform.clipboard(),
        false,
    ))
    .unwrap();
    let transfer = platform.data().unwrap();
    assert!(transfer.formats().contains(&format.clone().into()));
    assert_eq!(
        ready(transfer.try_get_value_async(&format))
            .unwrap()
            .as_deref(),
        Some("custom")
    );
}

#[test]
fn cancelled_copy_or_cut_never_writes_or_raises_success_or_edits() {
    let _scope = crate::tests::test_scope();
    for cut in [false, true] {
        let editor = selected_editor("selected");
        let _subscription = editor.add_disposable_handler(TextEditor::copying_to_clipboard_event(), |_, args| {
            args.clone().set_cancel(true);
        });
        let count = Rc::new(Cell::new(0));
        let observed = count.clone();
        let _subscription = editor.add_disposable_handler(TextEditor::text_copied_event(), move |_, _| {
            observed.set(observed.get() + 1)
        });
        let platform = TestClipboardImpl::new();
        assert_eq!(
            ready(EditorClipboard::copy_to_clipboard(
                &editor,
                platform.clipboard(),
                cut
            ))
            .unwrap(),
            ClipboardCopyOutcome::Cancelled
        );
        assert_eq!(platform.set_data_count(), 0);
        assert_eq!(count.get(), 0);
        assert_eq!(editor.text(), "selected");
        assert!(!editor.document().can_undo());
    }
}

#[test]
fn successful_multiselection_cut_has_one_undo_unit_and_failure_preserves_document() {
    let _scope = crate::tests::test_scope();
    let editor = editor_with_text("abcd\nefgh");
    editor
        .set_selection(Selection::from_segments(
            [SelectionSegment::new(1, 3), SelectionSegment::new(6, 8)],
            0,
        ))
        .unwrap();
    let platform = TestClipboardImpl::new();
    assert_eq!(
        ready(EditorClipboard::copy_to_clipboard(
            &editor,
            platform.clipboard(),
            true
        ))
        .unwrap(),
        ClipboardCopyOutcome::Cut
    );
    assert_eq!(platform.text().as_deref(), Some("bc\nfg"));
    assert_eq!(editor.text(), "ad\neh");
    assert_eq!(editor.document().undo_count(), 1);
    editor.undo().unwrap();
    assert_eq!(editor.text(), "abcd\nefgh");
    editor.select_all();
    let version = editor.document().version();
    let platform = TestClipboardImpl::throwing(ClipboardErrorKind::AccessDenied);
    assert!(matches!(
        ready(EditorClipboard::copy_to_clipboard(
            &editor,
            platform.clipboard(),
            true
        )),
        Err(EditorClipboardError::Clipboard(_))
    ));
    assert_eq!(editor.document().version(), version);
    assert_eq!(editor.text(), "abcd\nefgh");
}

#[derive(Default)]
struct GateClipboard {
    data: RefCell<Option<Rc<dyn IAsyncDataTransfer>>>,
    write_ready: Rc<Cell<bool>>,
    read_ready: Rc<Cell<bool>>,
}

impl GateClipboard {
    fn new() -> Rc<Self> {
        Rc::new(Self {
            read_ready: Rc::new(Cell::new(true)),
            ..Self::default()
        })
    }
    fn clipboard(self: &Rc<Self>) -> Rc<dyn IClipboard> {
        Clipboard::new(self.clone())
    }
}

impl IClipboardImpl for GateClipboard {
    fn try_get_data_async(
        &self,
    ) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
        let data = self.data.borrow().clone();
        let ready = self.read_ready.clone();
        Box::pin(poll_fn(move |_| {
            if ready.get() {
                Poll::Ready(Ok(data.clone()))
            } else {
                Poll::Pending
            }
        }))
    }
    fn set_data_async(
        &self,
        data: Rc<dyn IAsyncDataTransfer>,
    ) -> LocalBoxFuture<Result<(), ClipboardError>> {
        *self.data.borrow_mut() = Some(data);
        let ready = self.write_ready.clone();
        Box::pin(poll_fn(move |_| {
            if ready.get() {
                Poll::Ready(Ok(()))
            } else {
                Poll::Pending
            }
        }))
    }
    fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        self.data.borrow_mut().take();
        Box::pin(std::future::ready(Ok(())))
    }
}

#[test]
fn pending_cut_rechecks_document_selection_and_current_readonly_policy() {
    let _scope = crate::tests::test_scope();
    for change in 0..4 {
        let editor = selected_editor("original");
        let old_document = editor.document();
        let platform = GateClipboard::new();
        let mut cut = EditorClipboard::copy_to_clipboard(&editor, platform.clipboard(), true);
        assert!(
            cut.as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
        assert_eq!(editor.text(), "original");
        match change {
            0 => editor.select(1, 2).unwrap(),
            1 => old_document.insert(0, "!").unwrap(),
            2 => editor.set_document(TextDocument::from_text("replacement")),
            3 => editor.set_is_read_only(true),
            _ => unreachable!(),
        }
        let expected = editor.text();
        let version = editor.document().version();
        platform.write_ready.set(true);
        let result = ready(cut);
        if change == 3 {
            assert!(matches!(
                result,
                Err(EditorClipboardError::Edit(EditError::ReadOnly))
            ));
        } else {
            assert_eq!(result.unwrap(), ClipboardCopyOutcome::CopiedWithoutCut);
        }
        assert_eq!(editor.text(), expected);
        assert_eq!(editor.document().version(), version);
        assert!(!old_document.text().is_empty());
    }
}

#[test]
fn explicit_clipboard_ranges_preserve_selection_and_paste_at_the_requested_offset() {
    let _scope = crate::tests::test_scope();
    let editor = editor_with_text("a😀b tail");
    editor.select(5, 4).unwrap();
    let selection = editor.selection();
    let platform = TestClipboardImpl::new();
    assert_eq!(
        ready(EditorClipboard::copy_range_to_clipboard(
            &editor, platform.clipboard(), 1..3, false,
        )).unwrap(),
        ClipboardCopyOutcome::Copied,
    );
    assert_eq!(platform.text().as_deref(), Some("😀"));
    assert_eq!(editor.selection(), selection);

    let cut_platform = GateClipboard::new();
    let mut cut = EditorClipboard::copy_range_to_clipboard(&editor, cut_platform.clipboard(), 1..3, true);
    assert!(cut.as_mut().poll(&mut Context::from_waker(Waker::noop())).is_pending());
    editor.select(3, 1).unwrap();
    cut_platform.write_ready.set(true);
    assert_eq!(ready(cut).unwrap(), ClipboardCopyOutcome::Cut);
    assert_eq!(editor.text(), "ab tail");
    assert_eq!(editor.selection().primary(), SelectionSegment::new(1, 2));

    let clipboard = clipboard_with_payload("🦀\r\nβ", ClipboardSelectionKind::Rectangular);
    assert_eq!(
        ready(EditorClipboard::paste_at_from_clipboard(&editor, clipboard, 3)).unwrap(),
        ClipboardPasteOutcome::Pasted,
    );
    assert_eq!(editor.text(), "ab 🦀\r\nβtail");
    assert_eq!(editor.selection().primary(), SelectionSegment::new(1, 2));
    assert_eq!(editor.document().undo_count(), 2);
    editor.undo().unwrap();
    editor.undo().unwrap();
    assert_eq!(editor.text(), "a😀b tail");
}

#[test]
fn pending_explicit_clipboard_edits_reject_changed_and_replaced_documents() {
    let _scope = crate::tests::test_scope();
    for replace in [false, true] {
        let editor = editor_with_text("a😀b");
        let original = editor.document();
        let cut_platform = GateClipboard::new();
        let paste_platform = GateClipboard::new();
        let data = DataTransfer::new();
        data.add(DataTransferItem::create_text(Some("🦀")));
        *paste_platform.data.borrow_mut() = Some(data);
        paste_platform.read_ready.set(false);
        let mut cut = EditorClipboard::copy_range_to_clipboard(&editor, cut_platform.clipboard(), 1..3, true);
        let mut paste = EditorClipboard::paste_at_from_clipboard(&editor, paste_platform.clipboard(), 1);
        let mut cx = Context::from_waker(Waker::noop());
        assert!(cut.as_mut().poll(&mut cx).is_pending());
        assert!(paste.as_mut().poll(&mut cx).is_pending());
        if replace {
            editor.set_document(TextDocument::from_text("temporary"));
            editor.set_document(original.clone());
        } else {
            original.insert(0, "!").unwrap();
        }
        let expected = editor.text();
        let version = original.version();
        cut_platform.write_ready.set(true);
        paste_platform.read_ready.set(true);
        assert_eq!(ready(cut).unwrap(), ClipboardCopyOutcome::CopiedWithoutCut);
        assert_eq!(ready(paste).unwrap(), ClipboardPasteOutcome::DocumentReplaced);
        assert_eq!(editor.text(), expected);
        assert_eq!(original.version(), version);
    }
}

#[test]
fn explicit_clipboard_edits_check_protection_before_cut_and_after_paste_waits() {
    let _scope = crate::tests::test_scope();
    let editor = editor_with_text("a😀bc");
    let platform = GateClipboard::new();
    let data = DataTransfer::new();
    data.add(DataTransferItem::create_text(Some("🦀")));
    *platform.data.borrow_mut() = Some(data);
    platform.read_ready.set(false);
    let mut paste = EditorClipboard::paste_at_from_clipboard(&editor, platform.clipboard(), 3);
    assert!(paste.as_mut().poll(&mut Context::from_waker(Waker::noop())).is_pending());
    let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(editor.document()));
    protected.add(1, 3).unwrap();
    editor.session().set_read_only_section_provider(protected);
    platform.read_ready.set(true);
    assert!(matches!(ready(paste), Err(EditorClipboardError::Edit(EditError::ReadOnly))));
    let platform = TestClipboardImpl::new();
    assert!(matches!(
        ready(EditorClipboard::copy_range_to_clipboard(&editor, platform.clipboard(), 1..3, true)),
        Err(EditorClipboardError::Edit(EditError::ReadOnly)),
    ));
    assert_eq!(platform.set_data_count(), 0);
    assert_eq!(editor.text(), "a😀bc");
    assert!(!editor.document().can_undo());
}

#[test]
fn text_copied_handler_cannot_redirect_pending_cut_to_a_new_selection() {
    let _scope = crate::tests::test_scope();
    let editor = selected_editor("selected");
    let weak = editor.downgrade();
    let _subscription = editor.add_disposable_handler(TextEditor::text_copied_event(), move |_, args| {
        assert!(args.is_cut());
        weak.upgrade().unwrap().select(0, 1).unwrap();
    });
    let platform = TestClipboardImpl::new();
    assert_eq!(
        ready(EditorClipboard::copy_to_clipboard(
            &editor,
            platform.clipboard(),
            true
        ))
        .unwrap(),
        ClipboardCopyOutcome::CopiedWithoutCut
    );
    assert_eq!(editor.text(), "selected");
    assert!(!editor.document().can_undo());
}

#[test]
fn globally_read_only_cut_is_rejected_before_platform_write_and_empty_copy_is_noop() {
    let _scope = crate::tests::test_scope();
    let editor = selected_editor("protected");
    editor.set_is_read_only(true);
    let platform = TestClipboardImpl::new();
    assert!(matches!(
        ready(EditorClipboard::copy_to_clipboard(
            &editor,
            platform.clipboard(),
            true
        )),
        Err(EditorClipboardError::Edit(EditError::ReadOnly))
    ));
    assert_eq!(platform.set_data_count(), 0);
    editor.select(0, 0).unwrap();
    let mut options = editor.options();
    options.cut_copy_whole_line = false;
    editor.set_options(options);
    assert_eq!(
        ready(EditorClipboard::copy_to_clipboard(
            &editor,
            platform.clipboard(),
            false
        ))
        .unwrap(),
        ClipboardCopyOutcome::Empty
    );
    assert_eq!(platform.set_data_count(), 0);
}

#[test]
fn ordinary_clipboard_cut_and_paste_preserve_protected_spans_and_selection_history() {
    let _scope = crate::tests::test_scope();
    let original = "abcdef\r\nsecond";
    for (cut, selection, protection, expected, caret) in [
        (true, Selection::simple(1, 5), (2, 2), "acdf\r\nsecond", 1),
        (true, Selection::simple(5, 1), (2, 2), "acdf\r\nsecond", 1),
        (true, Selection::simple(1, 5), (1, 4), original, 5),
        (false, Selection::simple(1, 5), (2, 2), "acdXf\r\nsecond", 4),
        (false, Selection::simple(1, 5), (1, 4), original, 5),
    ] {
        let editor = editor_with_text(original);
        editor.set_selection(selection.clone()).unwrap();
        let provider = Rc::new(TextSegmentReadOnlySectionProvider::new(editor.document()));
        provider.add(protection.0, protection.1).unwrap();
        editor.session().set_read_only_section_provider(provider);
        if cut {
            assert!(editor.command(crate::EditorCommandKind::Cut).can_execute(None));
            let platform = TestClipboardImpl::new();
            assert_eq!(
                ready(EditorClipboard::copy_to_clipboard(&editor, platform.clipboard(), true)).unwrap(),
                ClipboardCopyOutcome::Cut,
            );
            assert_eq!(platform.text().as_deref(), Some("bcde"));
        } else {
            assert!(editor.command(crate::EditorCommandKind::Paste).can_execute(None));
            let clipboard = clipboard_with_payload("X", ClipboardSelectionKind::Plain);
            assert_eq!(
                ready(EditorClipboard::paste_from_clipboard(&editor, clipboard)).unwrap(),
                ClipboardPasteOutcome::Pasted,
            );
        }
        assert_eq!(editor.text(), expected);
        assert_eq!(editor.caret_offset(), caret);
        assert_eq!(editor.document().undo_count(), usize::from(expected != original));
        if expected == original {
            assert_eq!(editor.selection(), selection);
        } else {
            assert_eq!(editor.selection(), Selection::empty(caret));
            assert!(editor.undo().unwrap());
            assert_eq!(editor.text(), original);
            assert_eq!(editor.selection(), selection);
            assert!(editor.redo().unwrap());
            assert_eq!(editor.text(), expected);
            let mapped_selection = match (cut, selection.primary().caret) {
                (true, 1) => Selection::simple(3, 1),
                (true, _) => Selection::simple(1, 3),
                (false, _) => Selection::simple(1, 4),
            };
            assert_eq!(editor.selection(), mapped_selection);
        }
    }
}

fn rectangle_clipboard_editor(protected: &[(usize, usize)]) -> Ref<TextEditor> {
    let editor = editor_with_text("abcdef\r\nghijkl\r\nmnopqr");
    let mut options = editor.options();
    options.newline = "\r\n".to_owned();
    editor.set_options(options);
    editor
        .set_selection(Selection::from_rectangle_segments(
            [(1, 5), (9, 13), (17, 21)].map(|(start, end)| SelectionSegment::new(start, end)),
            2,
        ))
        .unwrap();
    let provider = Rc::new(TextSegmentReadOnlySectionProvider::new(editor.document()));
    for &(start, length) in protected {
        provider.add(start, length).unwrap();
    }
    editor.session().set_read_only_section_provider(provider);
    editor
}

#[test]
fn rectangle_clipboard_cut_copies_the_full_box_and_deletes_writable_spans() {
    let _scope = crate::tests::test_scope();
    let original = "abcdef\r\nghijkl\r\nmnopqr";
    for (protected, expected, carets) in [
        (&[(2, 2), (9, 4)][..], "acdf\r\nghijkl\r\nmr", [1, 7, 15]),
        (&[(1, 4), (9, 4), (17, 4)][..], original, [1, 9, 17]),
    ] {
        let editor = rectangle_clipboard_editor(protected);
        let selection = editor.selection();
        let command = editor.command(crate::EditorCommandKind::Cut);
        assert!(command.can_execute(None));
        let clipboard = TestClipboardImpl::new().clipboard();
        assert_eq!(
            ready(EditorClipboard::copy_to_clipboard(&editor, clipboard.clone(), true)).unwrap(),
            ClipboardCopyOutcome::Cut,
        );
        let payload = ready(EditorClipboard::read_clipboard(clipboard)).unwrap().unwrap();
        assert_eq!(payload.text, "bcde\r\nhijk\r\nnopq");
        assert_eq!(payload.kind, ClipboardSelectionKind::Rectangular);
        assert_eq!(editor.text(), expected);
        assert_eq!(
            editor.selection(),
            Selection::from_rectangle_segments(carets.map(SelectionSegment::empty), 2),
        );
        assert_eq!(editor.document().undo_count(), usize::from(expected != original));
        if expected != original {
            assert!(editor.undo().unwrap());
            assert_eq!(editor.text(), original);
            assert_eq!(editor.selection(), selection);
        }
    }
}

#[test]
fn rectangle_clipboard_paste_replaces_only_writable_spans_and_moves_the_caret() {
    let _scope = crate::tests::test_scope();
    let original = "abcdef\r\nghijkl\r\nmnopqr";
    for (protected, expected, caret) in [
        (&[(2, 2), (9, 4)][..], "acdXf\r\nghijkl\r\nmnopqr", 17),
        (&[(1, 4), (9, 4), (17, 4)][..], original, 18),
    ] {
        let editor = rectangle_clipboard_editor(protected);
        let selection = editor.selection();
        let command = editor.command(crate::EditorCommandKind::Paste);
        assert!(command.can_execute(None));
        let clipboard = clipboard_with_payload("X\r\nY", ClipboardSelectionKind::Plain);
        assert_eq!(
            ready(EditorClipboard::paste_from_clipboard(&editor, clipboard)).unwrap(),
            ClipboardPasteOutcome::Pasted,
        );
        assert_eq!(editor.text(), expected);
        assert_eq!(editor.selection(), Selection::empty(caret));
        assert_eq!(editor.document().undo_count(), usize::from(expected != original));
        if expected != original {
            assert!(editor.undo().unwrap());
            assert_eq!(editor.text(), original);
            assert_eq!(editor.selection(), selection);
        }
    }
    let editor = rectangle_clipboard_editor(&[(20, 2)]);
    assert!(!editor.command(crate::EditorCommandKind::Paste).can_execute(None));
}

#[test]
fn rectangle_clipboard_cut_honors_write_failure_and_current_protection() {
    let _scope = crate::tests::test_scope();
    let editor = rectangle_clipboard_editor(&[(2, 2), (9, 4)]);
    let original = editor.text();
    let selection = editor.selection();
    let failed = TestClipboardImpl::throwing(ClipboardErrorKind::Other);
    assert!(matches!(
        ready(EditorClipboard::copy_to_clipboard(&editor, failed.clipboard(), true)),
        Err(EditorClipboardError::Clipboard(_))
    ));
    assert_eq!(editor.text(), original);
    assert_eq!(editor.selection(), selection);
    assert!(!editor.document().can_undo());

    let platform = GateClipboard::new();
    let mut cut = EditorClipboard::copy_to_clipboard(&editor, platform.clipboard(), true);
    assert!(cut.as_mut().poll(&mut Context::from_waker(Waker::noop())).is_pending());
    let provider = Rc::new(TextSegmentReadOnlySectionProvider::new(editor.document()));
    provider.add(2, 2).unwrap();
    provider.add(9, 4).unwrap();
    provider.add(17, 4).unwrap();
    editor.session().set_read_only_section_provider(provider);
    platform.write_ready.set(true);
    assert_eq!(ready(cut).unwrap(), ClipboardCopyOutcome::Cut);
    assert_eq!(editor.text(), "acdf\r\nghijkl\r\nmnopqr");
}

#[test]
fn rectangle_clipboard_cut_leaves_the_old_document_alone_if_the_view_switches() {
    let _scope = crate::tests::test_scope();
    for protected in [&[][..], &[(2, 2), (9, 4)][..]] {
        let editor = rectangle_clipboard_editor(protected);
        let document = editor.document();
        let weak = editor.downgrade();
        let _subscription = document.update_started(move || {
            weak.upgrade().unwrap().set_document(TextDocument::from_text("replacement"));
        });
        let clipboard = TestClipboardImpl::new().clipboard();
        assert!(matches!(
            ready(EditorClipboard::copy_to_clipboard(&editor, clipboard, true)),
            Err(EditorClipboardError::Edit(EditError::StaleEditPlan))
        ));
        assert_eq!(document.text(), "abcdef\r\nghijkl\r\nmnopqr");
        assert!(!document.can_undo());
        assert_eq!(editor.text(), "replacement");
    }
}

#[test]
fn rectangle_copy_marker_round_trips_through_padded_column_paste_and_one_undo() {
    let _scope = crate::tests::test_scope();
    let source = editor_with_text("abc\nxyz");
    source
        .set_selection(Selection::from_rectangle_segments(
            [SelectionSegment::new(1, 3), SelectionSegment::new(5, 7)],
            0,
        ))
        .unwrap();
    let prepared = EditorClipboard::prepare_copy(&source).unwrap().unwrap();
    let item = &prepared.data.items()[0];
    for name in RECTANGLE_PLATFORM_FORMATS {
        let format = DataFormat::create_bytes_platform_format(name);
        assert_eq!(item.try_get_value(&format).as_deref(), Some(&[1][..]));
    }
    let platform = TestClipboardImpl::new();
    ready(EditorClipboard::copy_to_clipboard(
        &source,
        platform.clipboard(),
        false,
    ))
    .unwrap();
    let payload = ready(EditorClipboard::read_clipboard(platform.clipboard()))
        .unwrap()
        .unwrap();
    assert_eq!(
        payload,
        ClipboardText {
            text: "bc\nyz".into(),
            kind: ClipboardSelectionKind::Rectangular
        }
    );
    let destination = editor_with_text("12\nx\nuvw");
    let mut options = destination.options();
    options.enable_virtual_space = true;
    destination.set_options(options);
    destination.set_caret_offset(2).unwrap();
    assert_eq!(
        ready(EditorClipboard::paste_from_clipboard(
            &destination,
            platform.clipboard()
        ))
        .unwrap(),
        ClipboardPasteOutcome::Pasted
    );
    assert_eq!(destination.text(), "12bc\nx yz\nuvw");
    assert!(!destination.selection().is_rectangular());
    assert!(destination.selection().is_empty());
    assert_eq!(destination.document().undo_count(), 1);
    destination.undo().unwrap();
    assert_eq!(destination.text(), "12\nx\nuvw");
}

#[test]
fn foreign_rectangle_marker_is_checked_before_whole_line_and_changes_the_paste_geometry() {
    let _scope = crate::tests::test_scope();
    let clipboard = TestClipboardImpl::new().clipboard();
    for name in RECTANGLE_PLATFORM_FORMATS {
        for (marker, expected) in [(&[1][..], "1X2\n3Y4"), (&[1, 2][..], "1X\nY2\n34")] {
            let item = DataTransferItem::create_text(Some("X\nY"));
            item.set(
                &DataFormat::create_bytes_platform_format(name),
                Some(Rc::from(marker)),
            );
            let data = DataTransfer::new();
            data.add(item.clone());
            ready(clipboard.set_data_async(Some(data.clone()))).unwrap();

            let editor = editor_with_text("12\n34");
            editor.set_caret_offset(1).unwrap();
            ready(EditorClipboard::paste_from_clipboard(
                &editor,
                clipboard.clone(),
            ))
            .unwrap();
            assert_eq!(editor.text(), expected);

            item.set(
                &EditorClipboard::whole_line_selection_format(),
                Some("1".into()),
            );
            ready(clipboard.set_data_async(Some(data))).unwrap();
            let payload = ready(EditorClipboard::read_clipboard(clipboard.clone()))
                .unwrap()
                .unwrap();
            assert_eq!(
                payload.kind,
                if marker == [1] {
                    ClipboardSelectionKind::Rectangular
                } else {
                    ClipboardSelectionKind::WholeLine
                }
            );
        }
    }
}

#[test]
fn removing_the_ferroui_marker_in_a_copy_handler_suppresses_automatic_native_aliases() {
    let _scope = crate::tests::test_scope();
    let source = editor_with_text("abc\nxyz");
    source
        .set_selection(Selection::from_rectangle_segments(
            [SelectionSegment::new(1, 3), SelectionSegment::new(5, 7)],
            0,
        ))
        .unwrap();
    let _subscription =
        source.add_disposable_handler(TextEditor::copying_to_clipboard_event(), |_, args| {
            args.data().items()[0].set(&EditorClipboard::rectangular_selection_format(), None);
        });
    let clipboard = TestClipboardImpl::new().clipboard();
    ready(EditorClipboard::copy_to_clipboard(&source, clipboard.clone(), false)).unwrap();
    let payload = ready(EditorClipboard::read_clipboard(clipboard))
        .unwrap()
        .unwrap();
    assert_eq!(payload.kind, ClipboardSelectionKind::Plain);
    assert_eq!(payload.text, "bc\nyz");
}

#[cfg(windows)]
#[test]
fn windows_legacy_line_marker_round_trips_and_accepts_ole_padding() {
    let _scope = crate::tests::test_scope();
    let format = DataFormat::create_bytes_platform_format("MSDEVLineSelect");
    let source = editor_with_text("one\r\ntwo");
    let payload = EditorClipboard::prepare_copy(&source).unwrap().unwrap();
    assert_eq!(
        payload.data.items()[0].try_get_value(&format).as_deref(),
        Some(&[1][..])
    );

    let clipboard = TestClipboardImpl::new().clipboard();
    let item = DataTransferItem::create_text(Some("one\r\n"));
    item.set(&format, Some(Rc::from([1, 0, 0, 0])));
    let data = DataTransfer::new();
    data.add(item);
    ready(clipboard.set_data_async(Some(data))).unwrap();

    let editor = editor_with_text("zero\ntwo");
    editor.set_caret_offset(6).unwrap();
    ready(EditorClipboard::paste_from_clipboard(&editor, clipboard)).unwrap();
    assert_eq!(editor.text(), "zero\none\ntwo");
}

#[test]
fn rectangle_eof_falls_back_once_and_plain_multiline_paste_does_not_create_columns() {
    let _scope = crate::tests::test_scope();
    let rectangle = clipboard_with_payload("a\r\nb", ClipboardSelectionKind::Rectangular);
    let editor = editor_with_text("12");
    editor.set_caret_offset(1).unwrap();
    ready(EditorClipboard::paste_from_clipboard(&editor, rectangle)).unwrap();
    assert_eq!(editor.text(), "1a\r\nb2");
    assert_eq!(editor.document().undo_count(), 1);
    let editor = editor_with_text("12\n34");
    editor.set_caret_offset(1).unwrap();
    ready(EditorClipboard::paste_from_clipboard(
        &editor,
        clipboard_with_payload("a\nb", ClipboardSelectionKind::Plain),
    ))
    .unwrap();
    assert_eq!(editor.text(), "1a\nb2\n34");
}

#[test]
fn protected_column_clipboard_paste_matches_the_reference_text_caret_and_undo() {
    let _scope = crate::tests::test_scope();
    let original = "abcdef\r\nghijkl\r\nmnopqr";
    let short_last_row = "abcdef\r\nghijkl\r\nx";
    for arranged in [false, true] {
        for (text, input, protection, expected, caret, redo) in [
            (original, "X\r\nY", &[(1, 3)][..], "abcdef\r\nghYijkl\r\nmnopqr", 11, Some(2)),
            (original, "X\r\nY", &[(9, 3)][..], "abXcdef\r\nghijkl\r\nmnopqr", 12, Some(3)),
            (original, "X\r\nY", &[(1, 3), (9, 3)][..], original, 11, None),
            (original, "X", &[(1, 3)][..], original, 3, None),
            (short_last_row, "X\r\nY\r\nZ", &[(9, 3)][..], "abXcdef\r\nghijkl\r\nx Z", 20, Some(3)),
        ] {
            let editor = if arranged {
                crate::tests::laid_out_editor(text)
            } else {
                editor_with_text(text)
            };
            let document = editor.document();
            document.clear_undo_history().unwrap();
            let mut options = editor.options();
            options.enable_virtual_space = text == short_last_row;
            options.convert_tabs_to_spaces = true;
            editor.set_options(options);
            editor.set_caret_offset(2).unwrap();
            let provider = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
            for &(offset, length) in protection {
                provider.add(offset, length).unwrap();
            }
            editor.session().set_read_only_section_provider(provider);
            assert_eq!(
                ready(EditorClipboard::paste_from_clipboard(
                    &editor,
                    clipboard_with_payload(input, ClipboardSelectionKind::Rectangular),
                )).unwrap(),
                ClipboardPasteOutcome::Pasted,
            );
            assert_eq!(editor.text(), expected, "arranged={arranged}, input={input:?}");
            assert_eq!(editor.selection(), Selection::empty(caret));
            assert_eq!(document.undo_count(), usize::from(redo.is_some()));
            if let Some(redo_caret) = redo {
                assert!(editor.undo().unwrap());
                assert_eq!(editor.text(), text);
                assert_eq!(editor.selection(), Selection::empty(2));
                assert!(editor.redo().unwrap());
                assert_eq!(editor.text(), expected);
                assert_eq!(editor.selection(), Selection::empty(redo_caret));
            }
        }
    }
}

#[test]
fn projected_column_paste_aborts_if_a_callback_replaces_the_document() {
    let _scope = crate::tests::test_scope();
    let editor = crate::tests::laid_out_editor("abcdef\r\nghijkl");
    let original = editor.document();
    original.clear_undo_history().unwrap();
    editor.set_caret_offset(2).unwrap();
    let weak = editor.downgrade();
    let _subscription = original.update_started(move || {
        weak.upgrade().unwrap().set_document(TextDocument::from_text("replacement"));
    });
    assert!(matches!(
        ready(EditorClipboard::paste_from_clipboard(
            &editor,
            clipboard_with_payload("X\r\nY", ClipboardSelectionKind::Rectangular),
        )),
        Err(EditorClipboardError::Edit(EditError::StaleEditPlan))
    ));
    assert_eq!(editor.text(), "replacement");
    assert_eq!(original.text(), "abcdef\r\nghijkl");
    assert!(!original.can_undo());
}

#[test]
fn paste_customization_can_rewrite_or_cancel_before_one_commit_and_success_notification() {
    let _scope = crate::tests::test_scope();
    let editor = editor_with_text("text");
    editor.set_caret_offset(2).unwrap();
    let cancel = Rc::new(Cell::new(false));
    let state = cancel.clone();
    let _subscription = editor.add_disposable_handler(
        TextEditor::pasting_from_clipboard_event(),
        move |_, args| {
            assert_eq!(args.text(), "source");
            args.clone().set_text("😀".into());
            args.set_kind(ClipboardSelectionKind::Plain);
            args.set_cancel(state.get());
        },
    );
    let copied = Rc::new(RefCell::new(Vec::new()));
    let recorded = copied.clone();
    let _subscription = editor.add_disposable_handler(TextEditor::text_pasted_event(), move |_, args| {
        recorded.borrow_mut().push(args.text().to_owned())
    });
    let area_pasted = Rc::new(RefCell::new(Vec::new()));
    let recorded = area_pasted.clone();
    let _area_subscription = editor.text_area().add_disposable_handler(
        TextArea::text_pasted_event(), move |_, args| {
            recorded.borrow_mut().push(args.text().to_owned());
        },
    );
    let clipboard = clipboard_with_payload("source", ClipboardSelectionKind::WholeLine);
    assert_eq!(
        ready(EditorClipboard::paste_from_clipboard(
            &editor,
            clipboard.clone()
        ))
        .unwrap(),
        ClipboardPasteOutcome::Pasted
    );
    assert_eq!(editor.text(), "te😀xt");
    assert_eq!(*copied.borrow(), ["😀"]);
    assert_eq!(editor.document().undo_count(), 1);
    cancel.set(true);
    let version = editor.document().version();
    assert_eq!(
        ready(EditorClipboard::paste_from_clipboard(&editor, clipboard)).unwrap(),
        ClipboardPasteOutcome::Cancelled
    );
    assert_eq!(editor.document().version(), version);
    assert_eq!(copied.borrow().len(), 1);
    assert_eq!(*area_pasted.borrow(), ["😀"]);
}

#[test]
fn pending_paste_uses_current_selection_but_declines_a_replaced_document() {
    let _scope = crate::tests::test_scope();
    for replace_document in [false, true] {
        let editor = editor_with_text("abcd");
        let platform = GateClipboard::new();
        let data = DataTransfer::new();
        data.add(DataTransferItem::create_text(Some("X")));
        *platform.data.borrow_mut() = Some(data);
        platform.read_ready.set(false);
        let mut paste = EditorClipboard::paste_from_clipboard(&editor, platform.clipboard());
        assert!(
            paste
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
        if replace_document {
            editor.set_document(TextDocument::from_text("new"));
        } else {
            editor.select(1, 2).unwrap();
        }
        platform.read_ready.set(true);
        if replace_document {
            assert_eq!(
                ready(paste).unwrap(),
                ClipboardPasteOutcome::DocumentReplaced
            );
            assert_eq!(editor.text(), "new");
        } else {
            assert_eq!(ready(paste).unwrap(), ClipboardPasteOutcome::Pasted);
            assert_eq!(editor.text(), "aXd");
        }
    }
}

struct TrackingTransfer {
    data: Rc<DataTransfer>,
    disposed: Rc<Cell<usize>>,
    failure: bool,
    pending: bool,
}

impl IDisposable for TrackingTransfer {
    fn dispose(&self) {
        self.disposed.set(self.disposed.get() + 1);
    }
}

impl IAsyncDataTransfer for TrackingTransfer {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.data.formats()
    }
    fn items(&self) -> Rc<[Rc<dyn IAsyncDataTransferItem>]> {
        if self.pending {
            Rc::from(vec![
                Rc::new(PendingTextItem) as Rc<dyn IAsyncDataTransferItem>
            ])
        } else {
            IAsyncDataTransfer::items(&*self.data)
        }
    }
    fn try_items(&self) -> Result<Rc<[Rc<dyn IAsyncDataTransferItem>]>, ClipboardError> {
        if self.failure {
            Err(ClipboardError::access_denied("Test read failure"))
        } else {
            Ok(self.items())
        }
    }
}

struct PendingTextItem;
impl IAsyncDataTransferItem for PendingTextItem {
    fn formats(&self) -> Rc<[DataFormat]> {
        Rc::from(vec![DataFormat::text().into()])
    }
    fn try_get_raw_async(
        &self,
        _: &DataFormat,
    ) -> LocalBoxFuture<Result<Option<Rc<dyn Any>>, ClipboardError>> {
        Box::pin(std::future::pending())
    }
}

#[test]
fn incoming_transfer_is_disposed_after_success_error_and_dropped_pending_read() {
    for mode in 0..3 {
        let data = DataTransfer::new();
        data.add(DataTransferItem::create_text(Some("text")));
        let disposed = Rc::new(Cell::new(0));
        let transfer = Rc::new(TrackingTransfer {
            data,
            disposed: disposed.clone(),
            failure: mode == 1,
            pending: mode == 2,
        });
        let platform = GateClipboard::new();
        *platform.data.borrow_mut() = Some(transfer);
        let mut read = EditorClipboard::read_clipboard(platform.clipboard());
        if mode == 2 {
            assert!(
                read.as_mut()
                    .poll(&mut Context::from_waker(Waker::noop()))
                    .is_pending()
            );
            assert_eq!(disposed.get(), 0);
            drop(read);
        } else {
            let result = ready(read);
            assert_eq!(result.is_err(), mode == 1);
            if mode == 0 {
                assert_eq!(result.unwrap().unwrap().text, "text");
            }
        }
        assert_eq!(disposed.get(), 1);
    }
}

#[test]
fn whole_line_marker_inserts_at_line_start_and_rechecks_protection_after_update_started() {
    let _scope = crate::tests::test_scope();
    let editor = editor_with_text("first\r\nsecond");
    editor.set_caret_offset(10).unwrap();
    let clipboard = clipboard_with_payload("copied\r\n", ClipboardSelectionKind::WholeLine);
    ready(EditorClipboard::paste_from_clipboard(
        &editor,
        clipboard.clone(),
    ))
    .unwrap();
    assert_eq!(editor.text(), "first\r\ncopied\r\nsecond");
    assert_eq!(editor.document().undo_count(), 1);
    editor.undo().unwrap();
    editor.set_caret_offset(10).unwrap();
    let weak = editor.downgrade();
    let subscription = editor
        .document()
        .update_started(move || weak.upgrade().unwrap().set_is_read_only(true));
    let version = editor.document().version();
    assert!(matches!(
        ready(EditorClipboard::paste_from_clipboard(&editor, clipboard)),
        Err(EditorClipboardError::Edit(EditError::ReadOnly))
    ));
    assert_eq!(editor.text(), "first\r\nsecond");
    assert_eq!(editor.document().version(), version);
    subscription.dispose();
}

struct CallbackPolicy {
    can_insert: Rc<dyn Fn()>,
}

impl ferroui_edit_core::editing::IReadOnlySectionProvider for CallbackPolicy {
    fn can_insert(&self, _: usize) -> bool {
        (self.can_insert)();
        true
    }
    fn get_deletable_segments(&self, segment: SimpleSegment) -> Vec<SimpleSegment> {
        vec![segment]
    }
}

#[test]
fn whole_line_policy_callbacks_cannot_commit_a_stale_plan_or_bypass_new_readonly_state() {
    let _scope = crate::tests::test_scope();
    for callback_number in [1, 2] {
        for change_document in [false, true] {
            let editor = editor_with_text("first\nsecond");
            editor.set_caret_offset(8).unwrap();
            let calls = Cell::new(0);
            let weak = editor.downgrade();
            editor
                .session()
                .set_read_only_section_provider(Rc::new(CallbackPolicy {
                    can_insert: Rc::new(move || {
                        calls.set(calls.get() + 1);
                        if calls.get() == callback_number {
                            let editor = weak.upgrade().unwrap();
                            if change_document {
                                editor.document().insert(0, "!").unwrap();
                            } else {
                                editor.set_is_read_only(true);
                            }
                        }
                    }),
                }));
            let result = ready(EditorClipboard::paste_from_clipboard(
                &editor,
                clipboard_with_payload("copied\n", ClipboardSelectionKind::WholeLine),
            ));
            if change_document {
                assert!(matches!(
                    result,
                    Err(EditorClipboardError::Edit(EditError::StaleEditPlan))
                ));
                assert_eq!(editor.text(), "!first\nsecond");
            } else {
                assert!(matches!(
                    result,
                    Err(EditorClipboardError::Edit(EditError::ReadOnly))
                ));
                assert_eq!(editor.text(), "first\nsecond");
                assert!(!editor.document().can_undo());
            }
        }
    }
}

#[test]
fn whole_line_copy_defaults_to_the_caret_line_with_native_marker_and_rich_formats() {
    let _scope = crate::tests::test_scope();
    let newline = if cfg!(windows) { "\r\n" } else { "\n" };
    for (text, caret, expected) in [
        ("one\r\na😀b\r\nlast", 8, "a😀b\n"),
        ("one\n\nlast", 4, "\n"),
        ("one\rsecond", 5, "second"),
        ("a\nb", 3, "b"),
        ("a", 1, "a"),
        ("\n", 0, "\n"),
        ("\r\n", 1, "\n"),
        ("x\r\ny", 2, "x\n"),
    ] {
        let editor = editor_with_text(text);
        editor.set_caret_offset(caret).unwrap();
        assert!(editor.options().cut_copy_whole_line);
        let selection = editor.selection();
        let prepared = EditorClipboard::prepare_copy(&editor).unwrap().unwrap();
        assert_eq!(prepared.text, expected.replace('\n', newline));
        assert_eq!(prepared.kind, ClipboardSelectionKind::WholeLine);
        assert!(prepared.formatting_error.is_none());
        assert_eq!(prepared.data.items().len(), 1);
        let item = &prepared.data.items()[0];
        assert_eq!(
            item.try_get_value(&EditorClipboard::whole_line_selection_format())
                .as_deref(),
            Some("1")
        );
        assert!(
            !prepared
                .data
                .formats()
                .contains(&EditorClipboard::rectangular_selection_format().into())
        );
        assert_eq!(prepared.data.formats().len(), 3 + WHOLE_LINE_PLATFORM_FORMATS.len());
        assert!(!current_html(item).is_empty());
        assert_eq!(editor.selection(), selection);
        assert_eq!(editor.text(), text);
        assert_eq!(
            EditorClipboard::selected_rich_text(&editor).unwrap().text(),
            ""
        );
        assert!(!editor.document().can_undo());
    }
}

#[test]
fn whole_line_copy_skips_empty_eof_and_empty_multiple_carets_without_writing() {
    let _scope = crate::tests::test_scope();
    for (text, caret) in [("", 0), ("\n", 1), ("a\n", 2), ("a\r\n", 3)] {
        let editor = editor_with_text(text);
        editor.set_caret_offset(caret).unwrap();
        // The reference enables the command with the option, even when its
        // execution will find no text on the final logical line.
        assert!(EditorClipboard::can_copy(&editor));
        assert!(EditorClipboard::can_cut(&editor));
        for cut in [false, true] {
            let platform = TestClipboardImpl::new();
            assert_eq!(
                ready(EditorClipboard::copy_to_clipboard(
                    &editor,
                    platform.clipboard(),
                    cut
                ))
                .unwrap(),
                ClipboardCopyOutcome::Empty
            );
            assert_eq!(platform.set_data_count(), 0);
            assert_eq!(editor.text(), text);
            assert!(!editor.document().can_undo());
        }
    }
    for rectangle in [false, true] {
        let editor = editor_with_text("first\nsecond");
        let carets = [SelectionSegment::empty(1), SelectionSegment::empty(8)];
        let selection = if rectangle {
            Selection::from_rectangle_segments(carets, 0)
        } else {
            Selection::from_segments(carets, 0)
        };
        editor.set_selection(selection.clone()).unwrap();
        assert!(EditorClipboard::prepare_copy(&editor).unwrap().is_none());
        assert!(!EditorClipboard::can_copy(&editor));
        assert!(!EditorClipboard::can_cut(&editor));
        assert_eq!(editor.selection(), selection);
    }
}

#[test]
fn whole_line_commands_requery_options_and_keep_protected_lines_copyable() {
    let _scope = crate::tests::test_scope();
    let editor = editor_with_text("first\nsecond");
    editor.set_caret_offset(3).unwrap();
    let copy = crate::EditorCommand::new(&editor, crate::EditorCommandKind::Copy);
    let cut = crate::EditorCommand::new(&editor, crate::EditorCommandKind::Cut);
    let changes = Rc::new(Cell::new(0));
    let observed = changes.clone();
    let subscription = copy.can_execute_changed(Rc::new(move || observed.set(observed.get() + 1)));
    assert!(copy.can_execute(None));
    assert!(cut.can_execute(None));
    let mut options = editor.options();
    options.cut_copy_whole_line = false;
    editor.set_options(options.clone());
    assert!(!copy.can_execute(None));
    assert!(!cut.can_execute(None));
    assert_eq!(changes.get(), 1);
    options.cut_copy_whole_line = true;
    editor.set_options(options);
    assert!(copy.can_execute(None));
    assert!(cut.can_execute(None));
    assert_eq!(changes.get(), 2);
    let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(editor.document()));
    protected.add(0, 1).unwrap();
    editor.session().set_read_only_section_provider(protected);
    assert!(copy.can_execute(None));
    assert!(cut.can_execute(None));
    let mut options = editor.options();
    options.cut_copy_whole_line = false;
    editor.set_options(options);
    editor.select(1, 2).unwrap();
    assert!(copy.can_execute(None));
    assert!(cut.can_execute(None));
    let prepared = EditorClipboard::prepare_copy(&editor).unwrap().unwrap();
    assert_eq!(prepared.text, "ir");
    assert_eq!(prepared.kind, ClipboardSelectionKind::Plain);
    editor.set_is_read_only(true);
    assert!(copy.can_execute(None));
    assert!(!cut.can_execute(None));
    subscription.dispose();
}

#[test]
fn whole_line_cut_deletes_the_delimiter_and_preserves_final_line_boundaries_in_one_undo() {
    let _scope = crate::tests::test_scope();
    for (text, caret, expected, expected_caret, copied) in [
        ("first\r\nsecond", 3, "second", 0, "first\n"),
        (
            "first\r\nsecond\r\nlast",
            10,
            "first\r\nlast",
            7,
            "second\n",
        ),
        ("first\nlast", 10, "first\n", 6, "last"),
        ("first\n\nlast", 6, "first\nlast", 6, "\n"),
        ("a😀b", 3, "", 0, "a😀b"),
    ] {
        let editor = editor_with_text(text);
        editor.set_caret_offset(caret).unwrap();
        let platform = TestClipboardImpl::new();
        assert_eq!(
            ready(EditorClipboard::copy_to_clipboard(
                &editor,
                platform.clipboard(),
                true
            ))
            .unwrap(),
            ClipboardCopyOutcome::Cut
        );
        assert_eq!(editor.text(), expected);
        assert_eq!(editor.caret_offset(), expected_caret);
        assert!(editor.selection().is_empty());
        assert_eq!(editor.document().undo_count(), 1);
        let payload = ready(EditorClipboard::read_clipboard(platform.clipboard()))
            .unwrap()
            .unwrap();
        assert_eq!(payload.kind, ClipboardSelectionKind::WholeLine);
        assert_eq!(
            payload.text,
            copied.replace('\n', if cfg!(windows) { "\r\n" } else { "\n" })
        );
        editor.undo().unwrap();
        assert_eq!(editor.text(), text);
        editor.redo().unwrap();
        assert_eq!(editor.text(), expected);
    }
}

#[test]
fn whole_line_cut_deletes_writable_spans_in_reference_undo_order() {
    let _scope = crate::tests::test_scope();
    let original = "abcdef\r\nsecond";
    for (protection, expected, undo_texts) in [
        ((2, 2), "cdsecond", &["abcdsecond", original][..]),
        ((6, 2), "\r\nsecond", &[original][..]),
        ((0, 8), original, &[][..]),
    ] {
        let editor = editor_with_text(original);
        editor.set_caret_offset(2).unwrap();
        let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(editor.document()));
        protected.add(protection.0, protection.1).unwrap();
        editor.session().set_read_only_section_provider(protected);
        assert!(editor.command(crate::EditorCommandKind::Cut).can_execute(None));
        let platform = TestClipboardImpl::new();
        assert_eq!(
            ready(EditorClipboard::copy_to_clipboard(&editor, platform.clipboard(), true)).unwrap(),
            ClipboardCopyOutcome::Cut,
        );
        let newline = if cfg!(windows) { "\r\n" } else { "\n" };
        assert_eq!(platform.text(), Some(format!("abcdef{newline}")));
        assert_eq!(editor.text(), expected);
        assert_eq!(editor.caret_offset(), if undo_texts.is_empty() { 2 } else { 0 });
        assert_eq!(editor.document().undo_count(), undo_texts.len());
        for text in undo_texts {
            assert!(editor.undo().unwrap());
            assert_eq!(editor.text(), *text);
            assert_eq!(editor.selection(), Selection::empty(2));
        }
        for _ in undo_texts {
            assert!(editor.redo().unwrap());
        }
        assert_eq!(editor.text(), expected);
        assert_eq!(editor.caret_offset(), if undo_texts.is_empty() { 2 } else { 0 });
    }
}

#[test]
fn whole_line_cut_stops_before_the_next_span_if_the_document_is_replaced() {
    let _scope = crate::tests::test_scope();
    let original = "abcdef\r\nsecond";
    let editor = editor_with_text(original);
    editor.set_caret_offset(2).unwrap();
    let document = editor.document();
    let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(document.clone()));
    protected.add(2, 2).unwrap();
    editor.session().set_read_only_section_provider(protected);
    let weak = editor.downgrade();
    let subscription = document.update_finished(move || {
        weak.upgrade().unwrap().set_document(TextDocument::from_text("replacement"));
    });
    let clipboard = TestClipboardImpl::new().clipboard();
    assert!(matches!(
        ready(EditorClipboard::copy_to_clipboard(&editor, clipboard, true)),
        Err(EditorClipboardError::Edit(EditError::StaleEditPlan))
    ));
    assert_eq!(editor.text(), "replacement");
    assert_eq!(document.text(), "abcdsecond");
    assert_eq!(document.undo_count(), 1);
    subscription.dispose();
    assert!(document.undo().unwrap());
    assert_eq!(document.text(), original);
}

#[test]
fn whole_line_cut_respects_cancellation_and_failed_platform_writes() {
    let _scope = crate::tests::test_scope();
    for cancel in [true, false] {
        let editor = editor_with_text("first\nsecond");
        editor.set_caret_offset(3).unwrap();
        let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(editor.document()));
        protected.add(0, 1).unwrap();
        editor.session().set_read_only_section_provider(protected);
        let _subscription = editor.add_disposable_handler(TextEditor::copying_to_clipboard_event(), move |_, args| {
            assert_eq!(args.kind(), ClipboardSelectionKind::WholeLine);
            assert!(args.is_cut());
            args.set_cancel(cancel);
        });
        let copied = Rc::new(Cell::new(0));
        let observed = copied.clone();
        let _subscription = editor.add_disposable_handler(TextEditor::text_copied_event(), move |_, _| {
            observed.set(observed.get() + 1);
        });
        let platform = if cancel {
            TestClipboardImpl::new()
        } else {
            TestClipboardImpl::throwing(ClipboardErrorKind::AccessDenied)
        };
        let result = ready(EditorClipboard::copy_to_clipboard(
            &editor,
            platform.clipboard(),
            true,
        ));
        if cancel {
            assert_eq!(result.unwrap(), ClipboardCopyOutcome::Cancelled);
            assert_eq!(platform.set_data_count(), 0);
        } else {
            assert!(matches!(result, Err(EditorClipboardError::Clipboard(_))));
        }
        assert_eq!(copied.get(), 0);
        assert_eq!(editor.text(), "first\nsecond");
        assert_eq!(editor.caret_offset(), 3);
        assert!(!editor.document().can_undo());
    }
}

#[test]
fn pending_whole_line_cut_keeps_its_original_line_and_rechecks_current_policy() {
    let _scope = crate::tests::test_scope();
    for change in 0..5 {
        let editor = editor_with_text("first\nsecond");
        editor.set_caret_offset(2).unwrap();
        let old_document = editor.document();
        let platform = GateClipboard::new();
        let mut cut = EditorClipboard::copy_to_clipboard(&editor, platform.clipboard(), true);
        assert!(
            cut.as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
        match change {
            0 => editor.set_caret_offset(9).unwrap(),
            1 => old_document.insert(0, "!").unwrap(),
            2 => editor.set_document(TextDocument::from_text("replacement")),
            3 => editor.set_is_read_only(true),
            4 => {
                let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(editor.document()));
                protected.add(0, 1).unwrap();
                editor.session().set_read_only_section_provider(protected);
            }
            _ => unreachable!(),
        }
        let expected = editor.text();
        let version = editor.document().version();
        platform.write_ready.set(true);
        let result = ready(cut);
        if change == 4 {
            assert_eq!(result.unwrap(), ClipboardCopyOutcome::Cut);
            assert_eq!(editor.text(), "fsecond");
            assert!(editor.undo().unwrap());
            assert_eq!(editor.text(), expected);
            continue;
        }
        if change == 3 {
            assert!(matches!(
                result,
                Err(EditorClipboardError::Edit(EditError::ReadOnly))
            ));
        } else {
            assert_eq!(result.unwrap(), ClipboardCopyOutcome::CopiedWithoutCut);
        }
        assert_eq!(editor.text(), expected);
        assert_eq!(editor.document().version(), version);
        assert!(old_document.text().contains("first\nsecond"));
    }
}

#[test]
fn whole_line_cut_customization_keeps_its_source_and_success_handlers_can_decline_deletion() {
    let _scope = crate::tests::test_scope();
    for move_caret in [false, true] {
        let editor = editor_with_text("first\nsecond");
        editor.set_caret_offset(2).unwrap();
        let _subscription = editor.add_disposable_handler(TextEditor::copying_to_clipboard_event(), |_, args| {
            assert_eq!(args.kind(), ClipboardSelectionKind::WholeLine);
            let data = DataTransfer::new();
            data.add(DataTransferItem::create_text(Some("custom")));
            args.set_data(data);
        });
        let weak = editor.downgrade();
        let _subscription = editor.add_disposable_handler(TextEditor::text_copied_event(), move |_, args| {
            assert_eq!(args.text(), Some("custom"));
            assert!(args.is_cut());
            let editor = weak.upgrade().unwrap();
            assert_eq!(editor.text(), "first\nsecond");
            assert_eq!(editor.caret_offset(), 2);
            if move_caret {
                editor.set_caret_offset(9).unwrap();
            }
        });
        let platform = TestClipboardImpl::new();
        let result = ready(EditorClipboard::copy_to_clipboard(
            &editor,
            platform.clipboard(),
            true,
        ))
        .unwrap();
        assert_eq!(platform.text().as_deref(), Some("custom"));
        if move_caret {
            assert_eq!(result, ClipboardCopyOutcome::CopiedWithoutCut);
            assert_eq!(editor.text(), "first\nsecond");
            assert!(!editor.document().can_undo());
        } else {
            assert_eq!(result, ClipboardCopyOutcome::Cut);
            assert_eq!(editor.text(), "second");
            assert_eq!(editor.document().undo_count(), 1);
        }
    }
}

struct DeleteCallbackPolicy(Rc<dyn Fn()>);

impl ferroui_edit_core::editing::IReadOnlySectionProvider for DeleteCallbackPolicy {
    fn can_insert(&self, _: usize) -> bool {
        true
    }
    fn get_deletable_segments(&self, segment: SimpleSegment) -> Vec<SimpleSegment> {
        (self.0)();
        vec![segment]
    }
}

#[test]
fn whole_line_cut_rejects_reentry_at_preflight_post_write_and_update_start() {
    let _scope = crate::tests::test_scope();
    for callback_number in 1..=4 {
        for change in 0..4 {
            let editor = editor_with_text("first\nsecond");
            editor.set_caret_offset(2).unwrap();
            let calls = Cell::new(0);
            let weak = editor.downgrade();
            editor
                .session()
                .set_read_only_section_provider(Rc::new(DeleteCallbackPolicy(Rc::new(
                    move || {
                        calls.set(calls.get() + 1);
                        if calls.get() == callback_number {
                            let editor = weak.upgrade().unwrap();
                            match change {
                                0 => editor.set_caret_offset(9).unwrap(),
                                1 => editor.document().insert(0, "!").unwrap(),
                                2 => editor.set_document(TextDocument::from_text("replacement")),
                                3 => editor.set_is_read_only(true),
                                _ => unreachable!(),
                            }
                        }
                    },
                ))));
            let platform = TestClipboardImpl::new();
            let result = ready(EditorClipboard::copy_to_clipboard(
                &editor,
                platform.clipboard(),
                true,
            ));
            if change == 3 {
                assert!(matches!(
                    result,
                    Err(EditorClipboardError::Edit(EditError::ReadOnly))
                ));
            } else {
                assert!(matches!(
                    result,
                    Err(EditorClipboardError::Edit(EditError::StaleEditPlan))
                ));
            }
            assert_eq!(platform.set_data_count(), usize::from(callback_number != 1));
            assert_eq!(
                editor.text(),
                match change {
                    1 => "!first\nsecond",
                    2 => "replacement",
                    _ => "first\nsecond",
                }
            );
        }
    }
    // The outer update-start event is another callback boundary, distinct from
    // the provider's last validation call.
    let editor = editor_with_text("first\nsecond");
    editor.set_caret_offset(2).unwrap();
    let weak = editor.downgrade();
    let subscription = editor.document().update_started(move || {
        weak.upgrade().unwrap().set_caret_offset(9).unwrap();
    });
    assert!(matches!(
        ready(EditorClipboard::copy_to_clipboard(
            &editor,
            TestClipboardImpl::new().clipboard(),
            true
        )),
        Err(EditorClipboardError::Edit(EditError::StaleEditPlan))
    ));
    assert_eq!(editor.text(), "first\nsecond");
    assert!(!editor.document().can_undo());
    subscription.dispose();
}

#[test]
fn whole_line_marker_round_trip_uses_destination_delimiters_and_one_undo() {
    let _scope = crate::tests::test_scope();
    let source = editor_with_text("first\r\na😀b\r\nlast");
    source.set_caret_offset(8).unwrap();
    let platform = TestClipboardImpl::new();
    ready(EditorClipboard::copy_to_clipboard(
        &source,
        platform.clipboard(),
        false,
    ))
    .unwrap();
    let destination = editor_with_text("one\rtwo");
    destination.set_caret_offset(6).unwrap();
    let mut options = destination.options();
    options.cut_copy_whole_line = false;
    destination.set_options(options);
    ready(EditorClipboard::paste_from_clipboard(
        &destination,
        platform.clipboard(),
    ))
    .unwrap();
    assert_eq!(destination.text(), "one\ra😀b\rtwo");
    assert!(destination.selection().is_empty());
    assert_eq!(destination.caret_offset(), 11);
    assert_eq!(destination.document().undo_count(), 1);
    destination.undo().unwrap();
    assert_eq!(destination.text(), "one\rtwo");
}

#[test]
fn whole_line_paste_normalizes_to_the_target_line_and_reports_the_inserted_text() {
    let _scope = crate::tests::test_scope();
    for (document, caret, newline, prefix, suffix) in [
        ("first\r\nsecond", 10, "\r\n", "first\r\n", "second"),
        ("first\rsecond\nlast", 8, "\n", "first\r", "second\nlast"),
        ("first\rsecond", 2, "\r", "", "first\rsecond"),
        ("", 0, if cfg!(windows) { "\r\n" } else { "\n" }, "", ""),
    ] {
        let editor = editor_with_text(document);
        editor.set_caret_offset(caret).unwrap();
        let mut options = editor.options();
        options.convert_tabs_to_spaces = true;
        options.indentation_size = 3;
        options.newline = "\r".into();
        editor.set_options(options);
        let expected = format!("one   two{newline}three{newline}four{newline}");
        let _subscription = editor.add_disposable_handler(TextEditor::pasting_from_clipboard_event(), |_, args| {
            assert_eq!(args.text(), "original");
            args.set_text("one\ttwo\nthree\rfour\r\n".into());
        });
        let pasted = Rc::new(RefCell::new(None));
        let observed = pasted.clone();
        let _subscription = editor.add_disposable_handler(TextEditor::text_pasted_event(), move |_, args| {
            *observed.borrow_mut() = Some((args.text().to_owned(), args.kind()));
        });
        ready(EditorClipboard::paste_from_clipboard(
            &editor,
            clipboard_with_payload("original", ClipboardSelectionKind::WholeLine),
        ))
        .unwrap();
        assert_eq!(editor.text(), format!("{prefix}{expected}{suffix}"));
        assert_eq!(
            *pasted.borrow(),
            Some((expected, ClipboardSelectionKind::WholeLine))
        );
        assert_eq!(editor.document().undo_count(), 1);
    }
}

#[test]
fn whole_line_marker_keeps_selected_text_and_multiple_caret_fallbacks_exact() {
    let _scope = crate::tests::test_scope();
    let editor = editor_with_text("first\nsecond");
    let mut options = editor.options();
    options.convert_tabs_to_spaces = true;
    editor.set_options(options);
    editor.select(2, 2).unwrap();
    ready(EditorClipboard::paste_from_clipboard(
        &editor,
        clipboard_with_payload("a\t\r\nb", ClipboardSelectionKind::WholeLine),
    ))
    .unwrap();
    assert_eq!(editor.text(), "fia\t\r\nbt\nsecond");
    let editor = editor_with_text("ab\ncd");
    editor
        .set_selection(Selection::from_segments(
            [SelectionSegment::empty(1), SelectionSegment::empty(4)],
            0,
        ))
        .unwrap();
    ready(EditorClipboard::paste_from_clipboard(
        &editor,
        clipboard_with_payload("X", ClipboardSelectionKind::WholeLine),
    ))
    .unwrap();
    assert_eq!(editor.text(), "aXb\ncXd");
    assert_eq!(editor.document().undo_count(), 1);
}

#[test]
fn whole_line_paste_rejects_caret_changes_inside_policy_callbacks() {
    let _scope = crate::tests::test_scope();
    for callback_number in [1, 2] {
        let editor = editor_with_text("first\nsecond");
        editor.set_caret_offset(8).unwrap();
        let calls = Cell::new(0);
        let weak = editor.downgrade();
        editor
            .session()
            .set_read_only_section_provider(Rc::new(CallbackPolicy {
                can_insert: Rc::new(move || {
                    calls.set(calls.get() + 1);
                    if calls.get() == callback_number {
                        weak.upgrade().unwrap().set_caret_offset(2).unwrap();
                    }
                }),
            }));
        assert!(matches!(
            ready(EditorClipboard::paste_from_clipboard(
                &editor,
                clipboard_with_payload("copied\n", ClipboardSelectionKind::WholeLine),
            )),
            Err(EditorClipboardError::Edit(EditError::StaleEditPlan))
        ));
        assert_eq!(editor.text(), "first\nsecond");
        assert!(!editor.document().can_undo());
    }
}
