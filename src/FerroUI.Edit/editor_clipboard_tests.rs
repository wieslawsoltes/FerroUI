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
}

#[test]
fn failed_rich_format_retains_plain_text_and_reports_the_error_to_copying_handlers() {
    let _scope = crate::tests::test_scope();
    let editor = selected_editor("word");
    let definition = HighlightingDefinition::load(r#"<SyntaxDefinition><RuleSet><Rule foreground="resource:MissingClipboardBrush">word</Rule></RuleSet></SyntaxDefinition>"#).unwrap();
    editor.set_syntax_highlighting(Some(Rc::new(definition)));
    let recorded = Rc::new(RefCell::new(None));
    let result = recorded.clone();
    editor.add_disposable_handler(TextEditor::copying_to_clipboard_event(), move |_, args| {
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
    editor.add_disposable_handler(TextEditor::copying_to_clipboard_event(), move |_, args| {
        raised.borrow_mut().push("copying".to_owned());
        let replacement = DataTransfer::new();
        replacement.add(DataTransferItem::create_text(Some("custom")));
        let clone = args.clone();
        clone.set_data(replacement);
        assert_eq!(args.text().as_deref(), Some("custom"));
    });
    let raised = events.clone();
    editor.add_disposable_handler(TextEditor::text_copied_event(), move |_, args| {
        raised.borrow_mut().push(args.text().unwrap().to_owned());
        assert!(!args.is_cut());
    });
    let platform = TestClipboardImpl::new();
    let operation = EditorClipboard::copy_to_clipboard(&editor, platform.clipboard(), false);
    assert_eq!(*events.borrow(), vec!["copying"]);
    assert_eq!(platform.set_data_count(), 0);
    assert_eq!(ready(operation).unwrap(), ClipboardCopyOutcome::Copied);
    assert_eq!(*events.borrow(), vec!["copying", "custom"]);
    assert_eq!(platform.text().as_deref(), Some("custom"));
    assert_eq!(editor.text(), "selected");
}

#[test]
fn copying_handler_can_inspect_then_add_an_item_format_without_stale_platform_metadata() {
    let _scope = crate::tests::test_scope();
    let editor = selected_editor("selected");
    let format = DataFormat::create_string_application_format("FerroUI.Edit.Test.Metadata");
    let custom = format.clone();
    editor.add_disposable_handler(TextEditor::copying_to_clipboard_event(), move |_, args| {
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
        editor.add_disposable_handler(TextEditor::copying_to_clipboard_event(), |_, args| {
            args.clone().set_cancel(true);
        });
        let count = Rc::new(Cell::new(0));
        let observed = count.clone();
        editor.add_disposable_handler(TextEditor::text_copied_event(), move |_, _| {
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
fn text_copied_handler_cannot_redirect_pending_cut_to_a_new_selection() {
    let _scope = crate::tests::test_scope();
    let editor = selected_editor("selected");
    let weak = editor.downgrade();
    editor.add_disposable_handler(TextEditor::text_copied_event(), move |_, args| {
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
fn protected_cut_is_rejected_before_platform_write_and_empty_copy_is_noop() {
    let _scope = crate::tests::test_scope();
    let editor = selected_editor("protected");
    let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(editor.document()));
    protected.add(2, 3).unwrap();
    editor.session().set_read_only_section_provider(protected);
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
fn rectangle_copy_marker_round_trips_through_padded_column_paste_and_one_undo() {
    let _scope = crate::tests::test_scope();
    let source = editor_with_text("abc\nxyz");
    source
        .set_selection(Selection::from_rectangle_segments(
            [SelectionSegment::new(1, 3), SelectionSegment::new(5, 7)],
            0,
        ))
        .unwrap();
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
fn protected_rectangle_paste_is_atomic_and_does_not_fall_back() {
    let _scope = crate::tests::test_scope();
    let editor = editor_with_text("abcd\nefgh");
    editor.set_caret_offset(2).unwrap();
    let provider = Rc::new(TextSegmentReadOnlySectionProvider::new(editor.document()));
    provider.add(6, 2).unwrap();
    editor.session().set_read_only_section_provider(provider);
    let version = editor.document().version();
    let result = ready(EditorClipboard::paste_from_clipboard(
        &editor,
        clipboard_with_payload("X\nY", ClipboardSelectionKind::Rectangular),
    ));
    assert!(matches!(
        result,
        Err(EditorClipboardError::Edit(EditError::ReadOnly))
    ));
    assert_eq!(editor.text(), "abcd\nefgh");
    assert_eq!(editor.document().version(), version);
    assert!(!editor.document().can_undo());
}

#[test]
fn paste_customization_can_rewrite_or_cancel_before_one_commit_and_success_notification() {
    let _scope = crate::tests::test_scope();
    let editor = editor_with_text("text");
    editor.set_caret_offset(2).unwrap();
    let cancel = Rc::new(Cell::new(false));
    let state = cancel.clone();
    editor.add_disposable_handler(
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
    editor.add_disposable_handler(TextEditor::text_pasted_event(), move |_, args| {
        recorded.borrow_mut().push(args.text().to_owned())
    });
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
        assert_eq!(prepared.data.formats().len(), 3);
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
fn whole_line_commands_requery_option_changes_and_full_line_protection() {
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
    assert!(!cut.can_execute(None));
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
fn whole_line_cut_respects_protection_cancellation_and_failed_platform_writes() {
    let _scope = crate::tests::test_scope();
    for mode in 0..3 {
        let editor = editor_with_text("first\nsecond");
        editor.set_caret_offset(3).unwrap();
        if mode == 0 {
            let protected = Rc::new(TextSegmentReadOnlySectionProvider::new(editor.document()));
            protected.add(0, 1).unwrap();
            editor.session().set_read_only_section_provider(protected);
        }
        editor.add_disposable_handler(TextEditor::copying_to_clipboard_event(), move |_, args| {
            assert_eq!(args.kind(), ClipboardSelectionKind::WholeLine);
            assert!(args.is_cut());
            args.set_cancel(mode == 1);
        });
        let copied = Rc::new(Cell::new(0));
        let observed = copied.clone();
        editor.add_disposable_handler(TextEditor::text_copied_event(), move |_, _| {
            observed.set(observed.get() + 1);
        });
        let platform = if mode == 2 {
            TestClipboardImpl::throwing(ClipboardErrorKind::AccessDenied)
        } else {
            TestClipboardImpl::new()
        };
        let result = ready(EditorClipboard::copy_to_clipboard(
            &editor,
            platform.clipboard(),
            true,
        ));
        match mode {
            0 => assert!(matches!(
                result,
                Err(EditorClipboardError::Edit(EditError::ReadOnly))
            )),
            1 => assert_eq!(result.unwrap(), ClipboardCopyOutcome::Cancelled),
            2 => assert!(matches!(result, Err(EditorClipboardError::Clipboard(_)))),
            _ => unreachable!(),
        }
        if mode != 2 {
            assert_eq!(platform.set_data_count(), 0);
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
        if change >= 3 {
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
        editor.add_disposable_handler(TextEditor::copying_to_clipboard_event(), |_, args| {
            assert_eq!(args.kind(), ClipboardSelectionKind::WholeLine);
            let data = DataTransfer::new();
            data.add(DataTransferItem::create_text(Some("custom")));
            args.set_data(data);
        });
        let weak = editor.downgrade();
        editor.add_disposable_handler(TextEditor::text_copied_event(), move |_, args| {
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
    for callback_number in 1..=3 {
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
        editor.add_disposable_handler(TextEditor::pasting_from_clipboard_event(), |_, args| {
            assert_eq!(args.text(), "original");
            args.set_text("one\ttwo\nthree\rfour\r\n".into());
        });
        let pasted = Rc::new(RefCell::new(None));
        let observed = pasted.clone();
        editor.add_disposable_handler(TextEditor::text_pasted_event(), move |_, args| {
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
