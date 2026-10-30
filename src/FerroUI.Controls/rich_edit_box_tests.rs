use crate::documents::{
    CharacterFormat, DocumentSnapshot, InlineSnapshot, InlineSnapshotKind, ParagraphFormat,
    ParagraphSnapshot, RichTextDocument,
};
use crate::rich_text::{
    EditError, EditKind, EditorSelection, RichEditClipboardFormat, RichEditTextDocument,
    RichTextEditor, RichTextTransfer, RtfError, RtfLimits, RtfReader, RtfWriter, TextFindOptions,
    TextFormatValue, TextRangeUnit,
};
use crate::{RichEditBox, RichEditCharacterCasing};
use ferroui_base::input::text_input::{TextInputOptions, TextSelection};
use ferroui_base::media::{FontFamily, FontStyle, FontWeight, TextAlignment};
use ferroui_base::threading::Dispatcher;
use ferroui_base::{Rect, Size};
use std::future::Future;
use std::task::{Context, Poll, Waker};

#[path = "rich_edit_clipboard_tests.rs"]
mod clipboard_tests;

fn run(text: &str, format: CharacterFormat) -> InlineSnapshot {
    InlineSnapshot {
        format,
        kind: InlineSnapshotKind::Run(text.to_owned()),
    }
}

fn rich_document() -> RichTextDocument {
    let document = RichTextDocument::new();
    document
        .restore(&DocumentSnapshot {
            paragraphs: vec![ParagraphSnapshot {
                format: ParagraphFormat {
                    text_alignment: Some(TextAlignment::Center),
                    ..ParagraphFormat::default()
                },
                inlines: vec![
                    InlineSnapshot {
                        format: CharacterFormat::default(),
                        kind: InlineSnapshotKind::Bold(vec![run(
                            "bold",
                            CharacterFormat::default(),
                        )]),
                    },
                    run(
                        " tail",
                        CharacterFormat {
                            font_style: Some(FontStyle::Italic),
                            ..CharacterFormat::default()
                        },
                    ),
                ],
            }],
            ..Default::default()
        })
        .unwrap();
    document
}

fn ready<T>(future: impl Future<Output = T>) -> T {
    let mut future = std::pin::pin!(future);
    match future
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("The test operation is synchronous."),
    }
}

/// A concrete test template using the same named parts and content-host contract
/// as the production themes. Tests retain a TestRoot so template bindings and
/// normal content-presenter attachment run through the framework lifecycle.
pub(crate) fn rich_edit_box_template() -> std::rc::Rc<dyn crate::templates::IControlTemplate> {
    use crate::presenters::{ContentPresenter, ScrollContentPresenter};
    use crate::primitives::TemplatedControl;
    use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
    use crate::{ContentControl, Grid, RowDefinitions, ScrollViewer, TextBlock};
    use ferroui_base::data::TemplateBinding;
    use ferroui_base::media::TextWrapping;
    use ferroui_base::{FerroProperty, Thickness};

    FuncControlTemplate::for_type::<RichEditBox>(|_, scope| {
        let grid = Grid::new();
        grid.set_row_definitions(RowDefinitions::parse("Auto,*,Auto").unwrap());
        let header = ContentPresenter::new();
        header.set_name(Some("HeaderContentPresenter".to_owned()));
        header.set_margin(Thickness::new(0.0, 0.0, 0.0, 8.0));
        header.set_text_wrapping(TextWrapping::Wrap);
        header.set_font_weight(FontWeight::Normal);
        header.bind_binding(
            ContentPresenter::content_property().as_property(),
            &TemplateBinding::new(RichEditBox::header_property().as_property()),
        );
        header.bind_binding(
            ContentPresenter::content_template_property().as_property(),
            &TemplateBinding::new(RichEditBox::header_template_property().as_property()),
        );
        grid.children().add(header.register_in_name_scope(&**scope));

        let content = ScrollViewer::new();
        content.set_name(Some("ContentElement".to_owned()));
        content.set_focusable(false);
        content.set_is_tab_stop(false);
        content.bind_binding(
            TemplatedControl::padding_property().as_property(),
            &TemplateBinding::new(
                crate::primitives::TemplatedControl::padding_property().as_property(),
            ),
        );
        content.set_template(Some(FuncControlTemplate::for_type::<ScrollViewer>(
            |_, scope| {
                let presenter = ScrollContentPresenter::new();
                presenter.set_name(Some("PART_ContentPresenter".to_owned()));
                presenter.set_can_horizontally_scroll(true);
                presenter.set_can_vertically_scroll(true);
                for property in [
                    ContentControl::content_property().as_property(),
                    TemplatedControl::padding_property().as_property(),
                ] {
                    presenter.bind_binding(property, &TemplateBinding::new(property));
                }
                presenter.register_in_name_scope(&**scope).upcast()
            },
        )));
        Grid::set_row(&content, 1);
        grid.children()
            .add(content.register_in_name_scope(&**scope));

        let placeholder = TextBlock::new();
        placeholder.set_name(Some("PlaceholderTextContentPresenter".to_owned()));
        placeholder.set_is_hit_test_visible(false);
        let bind = |target: &'static FerroProperty, source: &'static FerroProperty| {
            placeholder.bind_binding(target, &TemplateBinding::new(source));
        };
        bind(
            TextBlock::text_property().as_property(),
            RichEditBox::placeholder_text_property().as_property(),
        );
        bind(
            TextBlock::foreground_property().as_property(),
            RichEditBox::placeholder_foreground_property().as_property(),
        );
        bind(
            TextBlock::padding_property().as_property(),
            crate::primitives::TemplatedControl::padding_property().as_property(),
        );
        bind(
            TextBlock::text_alignment_property().as_property(),
            RichEditBox::text_alignment_property().as_property(),
        );
        bind(
            TextBlock::text_wrapping_property().as_property(),
            RichEditBox::text_wrapping_property().as_property(),
        );
        Grid::set_row(&placeholder, 1);
        grid.children()
            .add(placeholder.register_in_name_scope(&**scope));

        let description = ContentPresenter::new();
        description.set_name(Some("DescriptionPresenter".to_owned()));
        description.bind_binding(
            ContentPresenter::content_property().as_property(),
            &TemplateBinding::new(RichEditBox::description_property().as_property()),
        );
        Grid::set_row(&description, 2);
        grid.children()
            .add(description.register_in_name_scope(&**scope));
        grid.upcast()
    })
}

fn rich_edit_layout(root: &crate::test_support::TestRoot, size: Size) {
    root.set_client_size(size);
    root.measure(size);
    root.arrange(Rect::from_size(size));
    // Child invalidations are queued on the normal layout manager. They need
    // not synchronously invalidate a still-valid owning templated control.
    root.layout_manager().execute_layout_pass();
}

#[test]
fn rich_edit_undo_restores_nested_inline_structure_and_paragraph_format() {
    let _scope = Dispatcher::unit_test_scope();
    let document = rich_document();
    let original = document.snapshot();
    let editor = RichTextEditor::new(document.clone());
    editor.set_selection(EditorSelection::new(1, 3)).unwrap();
    editor.insert_text("XYZ", EditKind::Typing).unwrap();
    assert_eq!(document.text(), "bXYZd tail");
    assert_eq!(
        document.character_format_at(7).unwrap().font_style,
        Some(FontStyle::Italic)
    );
    assert!(editor.undo().unwrap());
    assert_eq!(document.snapshot(), original);
    assert_eq!(editor.selection(), EditorSelection::new(1, 3));
    assert!(editor.redo().unwrap());
    assert_eq!(document.text(), "bXYZd tail");
}

#[test]
fn rich_edit_equal_text_property_updates_preserve_format_selection_and_history() {
    let _scope = Dispatcher::unit_test_scope();
    let control = RichEditBox::new();
    control.set_rich_document(rich_document());
    let before = control.rich_document().snapshot();
    control.select(1, 3);
    assert!(control.insert_text("XYZ").unwrap());
    let after = control.rich_document().snapshot();
    let revision = control.rich_document().revision();
    let selection = (control.selection_start(), control.selection_end());

    control.set_direct_value(RichEditBox::text_property(), control.text());
    assert_eq!(control.rich_document().snapshot(), after);
    assert_eq!(control.rich_document().revision(), revision);
    assert_eq!((control.selection_start(), control.selection_end()), selection);
    assert!(control.can_undo());
    assert!(control.undo().unwrap());
    assert_eq!(control.rich_document().snapshot(), before);
    assert_eq!((control.selection_start(), control.selection_end()), (1, 3));

    let revision = control.rich_document().revision();
    control.set_direct_value(RichEditBox::text_property(), control.text());
    assert_eq!(control.rich_document().snapshot(), before);
    assert_eq!(control.rich_document().revision(), revision);
    assert!(control.can_redo());
    assert!(control.redo().unwrap());
    assert_eq!(control.rich_document().snapshot(), after);
    assert_eq!((control.selection_start(), control.selection_end()), selection);
}

#[test]
fn rich_edit_typing_coalesces_until_an_explicit_caret_change() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("");
    let editor = RichTextEditor::new(document.clone());
    editor.insert_text("a", EditKind::Typing).unwrap();
    editor.insert_text("b", EditKind::Typing).unwrap();
    editor.insert_text("c", EditKind::Typing).unwrap();
    assert!(editor.undo().unwrap());
    assert_eq!(document.text(), "");
    editor.redo().unwrap();
    editor.set_selection(EditorSelection::caret(1)).unwrap();
    editor.insert_text("!", EditKind::Typing).unwrap();
    editor.undo().unwrap();
    assert_eq!(document.text(), "abc");
    editor.undo().unwrap();
    assert_eq!(document.text(), "");
}

#[test]
fn rich_edit_deletion_uses_graphemes_for_combining_sequences_and_emoji() {
    let _scope = Dispatcher::unit_test_scope();
    for text in ["a\u{301}", "👨‍👩‍👧‍👦", "🇵🇱", "👍🏽"] {
        let document = RichTextDocument::from_text(&format!("x{text}y"));
        let editor = RichTextEditor::new(document.clone());
        editor
            .set_selection(EditorSelection::caret(1 + text.encode_utf16().count()))
            .unwrap();
        editor.delete_backward(false).unwrap();
        assert_eq!(document.text(), "xy", "{text}");
        editor.undo().unwrap();
        editor.set_selection(EditorSelection::caret(1)).unwrap();
        editor.delete_forward(false).unwrap();
        assert_eq!(document.text(), "xy", "{text}");
    }
}

#[test]
fn rich_edit_rejects_a_split_surrogate_selection_without_mutating_content() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("a😀b");
    let editor = RichTextEditor::new(document.clone());
    assert!(editor.set_selection(EditorSelection::new(0, 2)).is_err());
    assert_eq!(editor.selection(), EditorSelection::default());
    assert_eq!(document.text(), "a😀b");
}

#[test]
fn rich_edit_read_only_rejects_all_mutations_and_preserves_selection() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("read me");
    let editor = RichTextEditor::new(document.clone());
    editor.set_selection(EditorSelection::caret(3)).unwrap();
    editor.set_is_read_only(true);
    assert!(matches!(
        editor.insert_text("x", EditKind::Typing),
        Err(EditError::ReadOnly)
    ));
    assert!(matches!(
        editor.delete_backward(false),
        Err(EditError::ReadOnly)
    ));
    assert!(matches!(
        editor.apply_format(&CharacterFormat::default()),
        Err(EditError::ReadOnly)
    ));
    assert!(matches!(
        editor.set_preedit("x", None),
        Err(EditError::ReadOnly)
    ));
    assert_eq!(editor.selection(), EditorSelection::caret(3));
    assert_eq!(document.text(), "read me");
}

#[test]
fn rich_edit_max_length_counts_utf16_and_never_splits_a_scalar() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("ab");
    let editor = RichTextEditor::new(document.clone());
    editor.set_selection(EditorSelection::caret(2)).unwrap();
    editor.set_max_length(5);
    editor.insert_text("😀😀q", EditKind::Typing).unwrap();
    assert_eq!(document.text(), "ab😀");
    assert_eq!(editor.selection(), EditorSelection::caret(4));
    editor.set_selection(EditorSelection::new(0, 2)).unwrap();
    editor.insert_text("XYZ", EditKind::Typing).unwrap();
    assert_eq!(document.text(), "XYZ😀");
}

#[test]
fn rich_edit_rejected_length_limited_input_does_not_remove_a_selection() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("abcd");
    let editor = RichTextEditor::new(document.clone());
    editor.set_max_length(2);
    editor.set_selection(EditorSelection::new(0, 1)).unwrap();
    assert!(!editor.insert_text("😀", EditKind::Typing).unwrap());
    assert_eq!(document.text(), "abcd");
    assert_eq!(editor.selection(), EditorSelection::new(0, 1));
}

#[test]
fn rich_edit_compound_transaction_rolls_back_on_error_and_is_one_undo_unit() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("abc");
    let editor = RichTextEditor::new(document.clone());
    let failure = editor.edit_group(|editor| {
        editor.insert_text("!", EditKind::Typing)?;
        editor.set_selection(EditorSelection::caret(99))?;
        Ok(())
    });
    assert!(failure.is_err());
    assert_eq!(document.text(), "abc");
    assert!(!editor.can_undo());
    editor
        .edit_group(|editor| {
            editor.insert_text("!", EditKind::Typing)?;
            editor.set_selection(EditorSelection::new(1, 4))?;
            editor.apply_format(&CharacterFormat {
                font_weight: Some(FontWeight::Bold),
                ..CharacterFormat::default()
            })
        })
        .unwrap();
    assert_eq!(document.text(), "!abc");
    assert!(editor.undo().unwrap());
    assert_eq!(document.text(), "abc");
    assert!(!editor.can_undo());
}

#[test]
fn rich_edit_external_document_changes_invalidate_stale_undo_units() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("abc");
    let editor = RichTextEditor::new(document.clone());
    editor.insert_text("!", EditKind::Typing).unwrap();
    document.replace_text(0..1, "external").unwrap();
    assert!(!editor.can_undo());
    assert!(!editor.undo().unwrap());
    assert_eq!(document.text(), "externalabc");
}

#[test]
fn rich_edit_ime_repeated_preedit_cancel_restores_rich_content_exactly() {
    let _scope = Dispatcher::unit_test_scope();
    let document = rich_document();
    let original = document.snapshot();
    let editor = RichTextEditor::new(document.clone());
    editor.set_selection(EditorSelection::new(1, 3)).unwrap();
    editor.set_preedit("に", Some(1)).unwrap();
    assert_eq!(document.text(), "bにd tail");
    assert!(!editor.can_undo());
    editor.set_preedit("日本", Some(2)).unwrap();
    assert_eq!(document.text(), "b日本d tail");
    editor.cancel_composition().unwrap();
    assert_eq!(document.snapshot(), original);
    assert_eq!(editor.selection(), EditorSelection::new(1, 3));
    assert!(!editor.can_undo());
}

#[test]
fn rich_edit_ime_commit_creates_one_undo_unit_and_restores_original_selection() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("before");
    let editor = RichTextEditor::new(document.clone());
    editor.set_selection(EditorSelection::new(0, 6)).unwrap();
    editor.set_preedit("に", None).unwrap();
    editor.set_preedit("日本", None).unwrap();
    editor.commit_composition("日本語").unwrap();
    assert_eq!(document.text(), "日本語");
    assert!(!editor.is_composing());
    assert!(editor.undo().unwrap());
    assert_eq!(document.text(), "before");
    assert_eq!(editor.selection(), EditorSelection::new(0, 6));
    assert!(!editor.can_undo());
}

#[test]
fn rich_edit_ime_cancel_does_not_overwrite_an_external_edit() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("abc");
    let editor = RichTextEditor::new(document.clone());
    editor.set_preedit("日", None).unwrap();
    document.replace_text(0..1, "external").unwrap();
    editor.cancel_composition().unwrap();
    assert_eq!(document.text(), "externalabc");
}

#[test]
fn rich_edit_undo_limit_and_redo_branching_are_bounded() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("");
    let editor = RichTextEditor::new(document.clone());
    editor.set_undo_limit(2);
    for text in ["a", "b", "c"] {
        editor.checkpoint();
        editor.insert_text(text, EditKind::Typing).unwrap();
    }
    assert!(editor.undo().unwrap());
    assert!(editor.undo().unwrap());
    assert!(!editor.undo().unwrap());
    assert_eq!(document.text(), "a");
    editor.insert_text("X", EditKind::Typing).unwrap();
    assert!(!editor.can_redo());
    editor.set_undo_limit(0);
    assert!(!editor.can_undo());
}

#[test]
fn rich_edit_rtf_unicode_groups_fallback_and_cp1252_are_decoded() {
    let _scope = Dispatcher::unit_test_scope();
    let rtf = br"{\rtf1\ansi\ansicpg1252\uc1 A {\b bold} \u55357?\u56832? \'80{\*\unknown should not appear}}";
    let document = RtfReader::read(rtf).unwrap();
    assert_eq!(document.text(), "A bold 😀 €");
    let model = RichTextDocument::new();
    model.restore(&document).unwrap();
    assert_eq!(
        model.character_format_at(3).unwrap().font_weight,
        Some(FontWeight::Bold)
    );
    assert_ne!(
        model.character_format_at(7).unwrap().font_weight,
        Some(FontWeight::Bold)
    );
}

#[test]
fn rich_edit_rtf_fonts_colors_paragraph_metrics_and_hyperlinks_are_retained() {
    let _scope = Dispatcher::unit_test_scope();
    let rtf = br#"{\rtf1\ansi{\fonttbl{\f0\fnil Example Sans;}}{\colortbl;\red240\green20\blue30;}\f0\fs30\cf1\qc\fi300 hello {\field{\*\fldinst HYPERLINK "https://example.org"}{\fldrslt site}}\par\ql next}"#;
    let document = RtfReader::read(rtf).unwrap();
    assert_eq!(document.text(), "hello site\nnext");
    assert_eq!(
        document.paragraphs[0].format.text_alignment,
        Some(TextAlignment::Center)
    );
    assert_eq!(document.paragraphs[0].format.text_indent, Some(20.0));
    assert!(document.paragraphs[0].inlines.iter().any(|inline| matches!(&inline.kind, InlineSnapshotKind::Hyperlink { navigate_uri: Some(uri), .. } if uri == "https://example.org")));
    let model = RichTextDocument::new();
    model.restore(&document).unwrap();
    let format = model.character_format_at(0).unwrap();
    assert_eq!(format.font_family, Some(FontFamily::new("Example Sans")));
    assert_eq!(format.font_size, Some(20.0));
    assert_eq!(
        format
            .foreground
            .unwrap()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color()
            .r,
        240
    );
}

#[test]
fn rich_edit_rtf_writer_roundtrip_preserves_text_and_formatting_without_extra_spaces() {
    let _scope = Dispatcher::unit_test_scope();
    let document = rich_document();
    document.replace_text(9..9, " 😀 {\\}").unwrap();
    let encoded = RtfWriter::write(&document.snapshot()).unwrap();
    assert!(encoded.is_ascii());
    let decoded = RtfReader::read_str(&encoded).unwrap();
    assert_eq!(decoded.text(), document.text());
    let model = RichTextDocument::new();
    model.restore(&decoded).unwrap();
    assert_eq!(
        model.character_format_at(1).unwrap().font_weight,
        Some(FontWeight::Bold)
    );
    assert_eq!(
        model.character_format_at(6).unwrap().font_style,
        Some(FontStyle::Italic)
    );
    assert_eq!(
        decoded.paragraphs[0].format.text_alignment,
        Some(TextAlignment::Center)
    );
}

#[test]
fn rich_edit_rtf_rejects_malformed_and_unsupported_content_without_panicking() {
    let _scope = Dispatcher::unit_test_scope();
    for bytes in [
        b"{\\rtf1".as_slice(),
        b"{\\rtf1 \\'xz}",
        b"{\\rtf1 \\u-55357?}",
        b"{\\rtf1 \\fs-2 x}",
        b"{\\rtf1 \\b9999999999999999999999 x}",
        b"{\\rtf1}garbage",
    ] {
        assert!(
            RtfReader::read(bytes).is_err(),
            "{}",
            String::from_utf8_lossy(bytes)
        );
    }
    assert!(matches!(
        RtfReader::read(br"{\rtf1\ansicpg1361 x}"),
        Err(RtfError::UnsupportedCodePage(1361))
    ));
    assert!(matches!(
        RtfReader::read(br"{\rtf1{\object\bin2 {}}}"),
        Err(RtfError::UnsupportedFeature(_))
    ));
    assert!(matches!(
        RtfReader::read(br"{\rtf1\trowd a\cell\row}"),
        Err(RtfError::UnsupportedFeature(_))
    ));
}

#[test]
fn rich_edit_rtf_enforces_input_nesting_tokens_and_output_limits() {
    let _scope = Dispatcher::unit_test_scope();
    for (bytes, limits) in [
        (
            br"{\rtf1 abc}".as_slice(),
            RtfLimits {
                max_bytes: 4,
                ..RtfLimits::default()
            },
        ),
        (
            br"{\rtf1{{{x}}}}",
            RtfLimits {
                max_depth: 2,
                ..RtfLimits::default()
            },
        ),
        (
            br"{\rtf1\b a\i b}",
            RtfLimits {
                max_tokens: 2,
                ..RtfLimits::default()
            },
        ),
        (
            br"{\rtf1 abcd}",
            RtfLimits {
                max_text_length: 3,
                ..RtfLimits::default()
            },
        ),
        (
            br"{\rtf1\line\line}",
            RtfLimits {
                max_text_length: 1,
                ..RtfLimits::default()
            },
        ),
    ] {
        assert!(matches!(
            RtfReader::read_with_limits(bytes, limits),
            Err(RtfError::LimitExceeded(_))
        ));
    }
}

#[test]
fn rich_edit_framework_clipboard_preserves_exact_in_process_snapshot() {
    let _scope = Dispatcher::unit_test_scope();
    let original = rich_document().snapshot();
    let transfer = RichTextTransfer::create(&original, RichEditClipboardFormat::AllFormats);
    let rich = ready(RichTextTransfer::read(transfer.as_ref(), false))
        .unwrap()
        .unwrap();
    assert_eq!(rich, original);
    let plain = ready(RichTextTransfer::read(transfer.as_ref(), true))
        .unwrap()
        .unwrap();
    assert_eq!(plain.text(), original.text());
    assert_eq!(plain.paragraphs[0].format, ParagraphFormat::default());
}

#[test]
fn rich_edit_control_uses_document_layout_and_native_input_method_projection() {
    let _scope = crate::test_support::test_scope();
    let editor = RichEditBox::new();
    editor.set_template(Some(rich_edit_box_template()));
    let _root = crate::test_support::TestRoot::with_child(editor.clone());
    editor.set_text("hello\nworld").unwrap();
    rich_edit_layout(&_root, Size::new(200.0, 100.0));
    editor.select(1, 4);
    let input = editor.text_input_method_client();
    assert_eq!(input.surrounding_text(), "hello\nworld");
    assert_eq!(input.selection(), TextSelection::new(1, 4));
    assert!(input.cursor_rectangle().height > 0.0);
    assert!(input.text_view_visual().is_some());
    input.set_preedit_text_with_cursor(Some("日"), Some(1));
    assert_eq!(editor.text(), "h日o\nworld");
    input.set_preedit_text(None);
    assert_eq!(editor.text(), "hello\nworld");
    assert_eq!(input.selection(), TextSelection::new(1, 4));
}

#[test]
fn rich_edit_control_input_policy_maps_to_platform_options_and_enforces_casing() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichEditBox::new();
    editor.set_accepts_return(false);
    editor.set_character_casing(RichEditCharacterCasing::Upper);
    editor.set_max_length(6);
    editor.insert_text("ab\nßc").unwrap();
    assert_eq!(editor.text(), "ABSSC");
    let options = TextInputOptions::from_styled_element(&editor);
    assert!(!options.multiline);
    assert!(options.uppercase);
    assert!(!options.lowercase);
}

#[test]
fn rich_edit_control_clipboard_copy_cut_and_paste_use_the_same_rich_editor() {
    let _scope = Dispatcher::unit_test_scope();
    let clipboard = crate::testing::TestClipboardImpl::new();
    let source = RichEditBox::new();
    source.set_rich_document(rich_document());
    source.select_all();
    ready(source.copy_to_clipboard_async(clipboard.clipboard(), false)).unwrap();
    let target = RichEditBox::new();
    ready(target.paste_from_clipboard_async(clipboard.clipboard(), false)).unwrap();
    assert_eq!(target.text(), "bold tail");
    assert_eq!(
        target
            .rich_document()
            .character_format_at(1)
            .unwrap()
            .font_weight,
        Some(FontWeight::Bold)
    );
    assert!(target.undo().unwrap());
    assert_eq!(target.text(), "");
    ready(source.copy_to_clipboard_async(clipboard.clipboard(), true)).unwrap();
    assert_eq!(source.text(), "");
    source.undo().unwrap();
    assert_eq!(source.text(), "bold tail");
}

#[test]
fn rich_edit_automation_enforces_enabled_read_only_and_undoable_value_changes() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichEditBox::new();
    editor.set_text("initial").unwrap();
    let peer = crate::automation::peers::RichEditBoxAutomationPeer::new(&editor);
    editor.set_is_read_only(true);
    assert!(peer.set_value("forbidden").is_err());
    assert_eq!(editor.text(), "initial");
    editor.set_is_read_only(false);
    editor.set_is_enabled(false);
    assert!(peer.set_value("forbidden").is_err());
    editor.set_is_enabled(true);
    peer.set_value("allowed").unwrap();
    assert_eq!(editor.text(), "allowed");
    editor.undo().unwrap();
    assert_eq!(editor.text(), "initial");
}

#[test]
fn rich_edit_undo_deletion_restores_the_original_collapsed_caret() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichTextEditor::new(RichTextDocument::from_text("ab😀cd"));
    editor.set_selection(EditorSelection::caret(4)).unwrap();
    editor.delete_backward(false).unwrap();
    assert_eq!(editor.rich_document().text(), "abcd");
    editor.undo().unwrap();
    assert_eq!(editor.selection(), EditorSelection::caret(4));
    editor.delete_forward(false).unwrap();
    assert_eq!(editor.rich_document().text(), "ab😀d");
    editor.undo().unwrap();
    assert_eq!(editor.selection(), EditorSelection::caret(4));
}

#[test]
fn rich_edit_live_ranges_follow_edits_and_support_independent_clones() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("a😀bc");
    let services = RichEditTextDocument::new(document.clone());
    let range = services.get_range(1, 3).unwrap();
    let shared = range.clone();
    let independent = range.get_clone().unwrap();
    document.replace_text(0..0, "XY").unwrap();
    assert_eq!(range.range(), 3..5);
    assert_eq!(range.text().unwrap(), "😀");
    range.collapse(false).unwrap();
    assert_eq!(shared.range(), 5..5);
    assert_eq!(independent.range(), 3..5);
    assert!(range.in_range(&independent));
    assert!(!range.is_equal(&independent));
    assert!(services.get_range(3, 4).is_err());
}

#[test]
fn rich_edit_range_replacement_rebases_independent_selection_and_undoes_once() {
    let _scope = Dispatcher::unit_test_scope();
    let services = RichEditTextDocument::new(RichTextDocument::from_text("one two three"));
    services.selection().set_range(13, 8).unwrap();
    let range = services.get_range(4, 7).unwrap();
    range.set_text("2").unwrap();
    assert_eq!(services.text(), "one 2 three");
    assert_eq!(
        services.selection().selection(),
        EditorSelection::new(11, 6)
    );
    assert_eq!(range.range(), 4..5);
    services.undo().unwrap();
    assert_eq!(services.text(), "one two three");
    assert_eq!(
        services.selection().selection(),
        EditorSelection::new(13, 8)
    );
    assert!(!services.can_undo());
    services
        .get_range(4, 7)
        .unwrap()
        .set_snapshot(&DocumentSnapshot::default())
        .unwrap();
    assert_eq!(services.text(), "one  three");
}

#[test]
fn rich_edit_text_unit_movement_preserves_scalars_and_directed_selection() {
    let _scope = Dispatcher::unit_test_scope();
    let services = RichEditTextDocument::new(RichTextDocument::from_text("A😀e\u{301} Z"));
    let range = services.get_range(1, 3).unwrap();
    assert_eq!(range.move_by(TextRangeUnit::Character, 0).unwrap(), 0);
    assert_eq!(range.range(), 1..3);
    assert_eq!(range.move_by(TextRangeUnit::Character, 1).unwrap(), 1);
    assert_eq!(range.range(), 3..3);
    range.move_start(TextRangeUnit::Character, -1).unwrap();
    assert_eq!(range.text().unwrap(), "😀");
    services.selection().set_range(5, 5).unwrap();
    services
        .selection()
        .move_by(TextRangeUnit::Character, -2, true)
        .unwrap();
    assert_eq!(services.selection().selection(), EditorSelection::new(5, 3));
    assert_eq!(services.selection().text().unwrap(), "e\u{301}");
}

#[test]
fn rich_edit_range_search_keeps_utf16_offsets_and_reverse_overlapping_matches() {
    let _scope = Dispatcher::unit_test_scope();
    let services = RichEditTextDocument::new(RichTextDocument::from_text(
        "İstanbul 😀 aaa alpha alphabet",
    ));
    let all = services.document_range().unwrap();
    let unicode = all
        .find("i\u{307}stanbul", TextFindOptions::empty())
        .unwrap()
        .unwrap();
    assert_eq!(unicode.range(), 0..8);
    let overlap = all.find("aa", TextFindOptions::BACKWARDS).unwrap().unwrap();
    assert_eq!(overlap.range(), 13..15);
    let word = all
        .find(
            "alpha",
            TextFindOptions::WHOLE_WORD | TextFindOptions::BACKWARDS,
        )
        .unwrap()
        .unwrap();
    assert_eq!(word.range(), 16..21);
    assert!(all
        .find("istanbul", TextFindOptions::MATCH_CASE)
        .unwrap()
        .is_none());
    assert!(!all.find_text("missing", TextFindOptions::empty()).unwrap());
    assert_eq!(all.range(), 0..services.text_length());
}

#[test]
fn rich_edit_format_queries_distinguish_run_boundaries_and_mixed_paragraphs() {
    let _scope = Dispatcher::unit_test_scope();
    let services = RichEditTextDocument::new(rich_document());
    assert!(matches!(
        services
            .document_range()
            .unwrap()
            .character_format()
            .unwrap(),
        TextFormatValue::Mixed
    ));
    let TextFormatValue::Uniform(format) = services
        .get_range(4, 9)
        .unwrap()
        .character_format()
        .unwrap()
    else {
        panic!("The tail has one effective format.");
    };
    assert_eq!(format.font_style, Some(FontStyle::Italic));
    let other = RichEditTextDocument::new(RichTextDocument::from_text("one\ntwo"));
    other
        .get_range(4, 7)
        .unwrap()
        .set_paragraph_format(&ParagraphFormat {
            text_alignment: Some(TextAlignment::Right),
            ..ParagraphFormat::default()
        })
        .unwrap();
    assert!(matches!(
        other.document_range().unwrap().paragraph_format().unwrap(),
        TextFormatValue::Mixed
    ));
    other.undo().unwrap();
    assert!(matches!(
        other.document_range().unwrap().paragraph_format().unwrap(),
        TextFormatValue::Uniform(_)
    ));
}

#[test]
fn rich_edit_collapsed_typing_format_is_used_until_the_caret_moves() {
    let _scope = Dispatcher::unit_test_scope();
    let services = RichEditTextDocument::new(RichTextDocument::from_text("ab"));
    services.selection().set_range(2, 2).unwrap();
    services
        .selection()
        .set_character_format(&CharacterFormat {
            font_weight: Some(FontWeight::Bold),
            ..CharacterFormat::default()
        })
        .unwrap();
    assert_eq!(
        services.selection().character_format().unwrap().font_weight,
        Some(FontWeight::Bold)
    );
    services.selection().type_text("C").unwrap();
    assert_eq!(
        services
            .rich_document()
            .character_format_at(3)
            .unwrap()
            .font_weight,
        Some(FontWeight::Bold)
    );
    services.selection().set_range(0, 0).unwrap();
    assert_ne!(
        services.selection().character_format().unwrap().font_weight,
        Some(FontWeight::Bold)
    );
    services.selection().type_text("x").unwrap();
    assert_eq!(services.text(), "xabC");
}

#[test]
fn rich_edit_control_text_services_share_selection_history_and_visual_geometry() {
    let _scope = crate::test_support::test_scope();
    let control = RichEditBox::new();
    control.set_template(Some(rich_edit_box_template()));
    let _root = crate::test_support::TestRoot::with_child(control.clone());
    control.set_text("hello world").unwrap();
    let services = control.text_document();
    services.selection().set_range(1, 4).unwrap();
    assert_eq!((control.selection_start(), control.selection_end()), (1, 4));
    services.selection().type_text("i").unwrap();
    assert_eq!(control.text(), "hio world");
    assert_eq!(control.caret_index(), 2);
    rich_edit_layout(&_root, Size::new(200.0, 100.0));
    let range = services.get_range(0, 2).unwrap();
    assert!(range
        .get_rectangles(&control.text_presenter())
        .unwrap()
        .iter()
        .any(|rect| rect.width > 0.0 && rect.height > 0.0));
    let position = services
        .get_range_from_point(
            &control.text_presenter(),
            control.text_presenter().cursor_rectangle().top_left(),
        )
        .unwrap();
    assert!(position.start_position() <= services.text_length());
    let unrelated = crate::RichTextBlock::new();
    assert!(range.get_rectangles(&unrelated).is_err());
    control.undo().unwrap();
    assert_eq!(control.text(), "hello world");
}

#[test]
fn rich_edit_native_surrounding_text_excludes_preview_and_selection_cancels_it() {
    let _scope = Dispatcher::unit_test_scope();
    let control = RichEditBox::new();
    control.set_text("a😀bc").unwrap();
    control.select(1, 3);
    let input = control.text_input_method_client();
    input.set_preedit_text_with_cursor(Some("候補"), Some(1));
    assert_eq!(control.text(), "a候補bc");
    assert_eq!(input.surrounding_text(), "a😀bc");
    assert_eq!(input.selection(), TextSelection::new(1, 3));
    input.set_selection(TextSelection::new(4, 5));
    assert_eq!(control.text(), "a😀bc");
    assert!(!control.editor().is_composing());
    assert_eq!(input.selection(), TextSelection::new(4, 5));
}

#[test]
fn rich_edit_clipboard_captures_copy_before_poll_and_cancels_stale_paste() {
    let _scope = Dispatcher::unit_test_scope();
    let clipboard = crate::testing::TestClipboardImpl::new();
    let control = RichEditBox::new();
    control.set_text("original").unwrap();
    control.select_all();
    let cut = control.copy_to_clipboard_async(clipboard.clipboard(), true);
    control.set_text("replacement").unwrap();
    ready(cut).unwrap();
    assert_eq!(clipboard.text().as_deref(), Some("original"));
    assert_eq!(control.text(), "replacement");
    control.select_all();
    let paste = control.paste_from_clipboard_async(clipboard.clipboard(), false);
    control.set_text("later").unwrap();
    assert!(ready(paste).is_err());
    assert_eq!(control.text(), "later");
}

#[test]
fn rich_edit_clipboard_casing_and_single_line_policies_preserve_rich_runs() {
    let _scope = Dispatcher::unit_test_scope();
    let clipboard = crate::testing::TestClipboardImpl::new();
    let source = RichEditBox::new();
    source.set_rich_document(rich_document());
    source.select_all();
    ready(source.copy_to_clipboard_async(clipboard.clipboard(), false)).unwrap();
    let target = RichEditBox::new();
    target.set_accepts_return(false);
    target.set_character_casing(RichEditCharacterCasing::Upper);
    ready(target.paste_from_clipboard_async(clipboard.clipboard(), false)).unwrap();
    assert_eq!(target.text(), "BOLD TAIL");
    assert_eq!(
        target
            .rich_document()
            .character_format_at(1)
            .unwrap()
            .font_weight,
        Some(FontWeight::Bold)
    );
    assert_eq!(
        target.rich_document().character_format_at(7).unwrap().font_style,
        Some(FontStyle::Italic)
    );
}

#[test]
fn rich_edit_rtf_unicode_font_names_hyperlink_uris_and_targets_roundtrip() {
    let _scope = Dispatcher::unit_test_scope();
    let mut document = RtfReader::read(
        br#"{\rtf1{\field{\*\fldinst HYPERLINK "https://example.com"}{\fldrslt link}}}"#,
    )
    .unwrap();
    let hyperlink = &mut document.paragraphs[0].inlines[0];
    hyperlink.format.font_family = Some(FontFamily::new("日本語 Font"));
    let InlineSnapshotKind::Hyperlink {
        navigate_uri,
        target_name,
        ..
    } = &mut hyperlink.kind
    else {
        panic!("Expected a hyperlink field.");
    };
    *navigate_uri = Some("https://example.com/日本/😀".to_owned());
    *target_name = Some("詳細".to_owned());
    document.link_fields[0].target = "https://example.com/日本/😀".to_owned();
    document.link_fields[0].target_name = Some("詳細".to_owned());
    let rtf = RtfWriter::write(&document).unwrap();
    let decoded = RtfReader::read(rtf.as_bytes()).unwrap();
    let link = decoded.paragraphs[0]
        .inlines
        .iter()
        .find(|inline| matches!(inline.kind, InlineSnapshotKind::Hyperlink { .. }))
        .unwrap();
    let InlineSnapshotKind::Hyperlink {
        navigate_uri,
        target_name,
        ..
    } = &link.kind
    else {
        unreachable!();
    };
    assert_eq!(navigate_uri.as_deref(), Some("https://example.com/日本/😀"));
    assert_eq!(target_name.as_deref(), Some("詳細"));
    let model = RichTextDocument::new();
    model.restore(&decoded).unwrap();
    assert_eq!(
        model
            .character_format_at(1)
            .unwrap()
            .font_family
            .unwrap()
            .name(),
        "日本語 Font"
    );
    assert_eq!(model.text(), "link");
}

#[test]
fn rich_edit_rtf_hyperlink_bookmark_and_exact_line_spacing_are_preserved() {
    let _scope = Dispatcher::unit_test_scope();
    let snapshot = RtfReader::read(br#"{\rtf1\sl-300\slmult0{\field{\*\fldinst HYPERLINK \\l "section" \\t "frame"}{\fldrslt local}}}"#).unwrap();
    assert_eq!(snapshot.paragraphs[0].format.line_height, Some(20.0));
    assert_eq!(
        snapshot.paragraphs[0].format.line_stacking_strategy,
        Some(crate::documents::LineStackingStrategy::BlockLineHeight)
    );
    let InlineSnapshotKind::Hyperlink {
        navigate_uri,
        target_name,
        ..
    } = &snapshot.paragraphs[0].inlines[0].kind
    else {
        panic!("Expected a bookmark hyperlink.");
    };
    assert_eq!(navigate_uri.as_deref(), Some("#section"));
    assert_eq!(target_name.as_deref(), Some("frame"));
    let encoded = RtfWriter::write(&snapshot).unwrap();
    let roundtrip = RtfReader::read(encoded.as_bytes()).unwrap();
    assert_eq!(
        roundtrip.paragraphs[0].format.line_stacking_strategy,
        Some(crate::documents::LineStackingStrategy::BlockLineHeight)
    );
}

#[test]
fn rich_edit_ime_notifications_expose_committed_text_during_preview_publication() {
    let _scope = Dispatcher::unit_test_scope();
    let control = RichEditBox::new();
    control.set_text("original").unwrap();
    control.select(1, 4);
    let input = control.text_input_method_client();
    let observations = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let values = observations.clone();
    let weak = std::rc::Rc::downgrade(&input);
    let _subscription = input.surrounding_text_changed(std::rc::Rc::new(move || {
        if let Some(input) = weak.upgrade() {
            values
                .borrow_mut()
                .push((input.surrounding_text(), input.selection()));
        }
    }));
    input.set_preedit_text(Some("日"));
    input.set_preedit_text(Some("日本"));
    assert!(!observations.borrow().is_empty());
    assert!(observations
        .borrow()
        .iter()
        .all(|value| value == &("original".to_owned(), TextSelection::new(1, 4))));
    control.commit_composition("日本").unwrap();
    assert_eq!(input.surrounding_text(), "o日本inal");
}

#[test]
fn rich_edit_whole_word_search_accepts_phrases_and_rechecks_overlapping_matches() {
    let _scope = Dispatcher::unit_test_scope();
    let service = RichEditTextDocument::new(RichTextDocument::from_text(
        "new york, new yorker; new york.",
    ));
    let all = service.document_range().unwrap();
    let phrase = all
        .find("new york", TextFindOptions::WHOLE_WORD)
        .unwrap()
        .unwrap();
    assert_eq!(phrase.range(), 0..8);
    let last = all
        .find(
            "new york",
            TextFindOptions::WHOLE_WORD | TextFindOptions::BACKWARDS,
        )
        .unwrap()
        .unwrap();
    assert_eq!(last.text().unwrap(), "new york");
    assert!(last.start_position() > 8);
    assert!(service
        .get_range(1, 8)
        .unwrap()
        .find("ew", TextFindOptions::WHOLE_WORD)
        .unwrap()
        .is_none());
    let overlap = RichEditTextDocument::new(RichTextDocument::from_text("xa a a"));
    let matched = overlap
        .document_range()
        .unwrap()
        .find("a a", TextFindOptions::WHOLE_WORD)
        .unwrap()
        .unwrap();
    assert_eq!(matched.range(), 3..6);
}

#[test]
fn rich_edit_service_typing_resolves_ime_as_one_independent_undo_unit() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("base");
    let editor = RichTextEditor::new(document.clone());
    editor.set_selection(EditorSelection::caret(4)).unwrap();
    editor.insert_text("a", EditKind::Typing).unwrap();
    editor.set_preedit("preview", Some(7)).unwrap();
    assert_eq!(document.text(), "baseapreview");
    editor.insert_text("x", EditKind::Typing).unwrap();
    assert_eq!(document.text(), "baseax");
    assert!(!editor.is_composing());
    assert!(editor.undo().unwrap());
    assert_eq!(document.text(), "basea");
    assert!(editor.undo().unwrap());
    assert_eq!(document.text(), "base");
    assert!(editor.redo().unwrap());
    assert_eq!(document.text(), "basea");
}

#[test]
fn rich_edit_live_range_mutation_rebases_after_canceling_ime_preview() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("left right");
    let service = RichEditTextDocument::new(document.clone());
    let editor = service.editor();
    editor.set_selection(EditorSelection::new(0, 4)).unwrap();
    editor.set_preedit("long preview", Some(12)).unwrap();
    let range = service
        .document_range()
        .unwrap()
        .find("right", TextFindOptions::MATCH_CASE)
        .unwrap()
        .unwrap();
    range.set_text("R").unwrap();
    assert_eq!(document.text(), "left R");
    assert!(!editor.is_composing());
    assert_eq!(range.text().unwrap(), "R");
    assert!(editor.undo().unwrap());
    assert_eq!(document.text(), "left right");
}

#[test]
fn rich_edit_service_format_delete_and_undo_never_store_ime_preview_snapshots() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("abc");
    let editor = RichTextEditor::new(document.clone());
    editor.set_selection(EditorSelection::new(1, 2)).unwrap();
    editor.set_preedit("preview", Some(7)).unwrap();
    editor
        .apply_format(&CharacterFormat {
            font_weight: Some(FontWeight::Bold),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(document.text(), "abc");
    assert!(!editor.is_composing());
    assert!(editor.undo().unwrap());
    assert_eq!(document.text(), "abc");

    editor.set_selection(EditorSelection::new(1, 2)).unwrap();
    editor.set_preedit("preview", Some(7)).unwrap();
    editor.delete_backward(false).unwrap();
    assert_eq!(document.text(), "ac");
    assert!(editor.undo().unwrap());
    assert_eq!(document.text(), "abc");

    editor.set_selection(EditorSelection::caret(3)).unwrap();
    editor.insert_text("z", EditKind::Typing).unwrap();
    editor.set_preedit("preview", Some(7)).unwrap();
    assert!(editor.undo().unwrap());
    assert_eq!(document.text(), "abc");
    assert!(!editor.is_composing());
    assert!(editor.redo().unwrap());
    assert_eq!(document.text(), "abcz");
}

#[test]
fn rich_edit_read_only_service_rejects_edit_without_canceling_the_active_preview() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("text");
    let editor = RichTextEditor::new(document.clone());
    editor.set_selection(EditorSelection::caret(4)).unwrap();
    editor.set_preedit("preview", Some(7)).unwrap();
    let preview = document.snapshot();
    editor.set_is_read_only(true);
    assert!(matches!(
        editor.insert_text("replacement", EditKind::Typing),
        Err(EditError::ReadOnly)
    ));
    assert!(matches!(
        editor.delete_selection(),
        Err(EditError::ReadOnly)
    ));
    assert_eq!(document.snapshot(), preview);
    assert!(editor.is_composing());
    editor.cancel_composition().unwrap();
    assert_eq!(document.text(), "text");
}

#[test]
fn rich_edit_markup_integer_properties_saturate_public_usize_setters() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichEditBox::new();
    editor.set_max_length(usize::MAX);
    editor.set_undo_limit(usize::MAX);
    assert_eq!(editor.max_length(), i32::MAX as usize);
    assert_eq!(editor.undo_limit(), i32::MAX as usize);
    assert!(!RichEditBox::max_length_property().is_valid_value(&-1));
    assert!(!RichEditBox::undo_limit_property().is_valid_value(&-1));
}

#[test]
fn rich_edit_caret_timer_blinks_resets_after_input_and_stops_for_selection_or_blur() {
    use ferroui_base::input::{FocusChangedEventArgs, InputElement};
    let _scope = crate::test_support::test_scope();
    let editor = RichEditBox::new();
    editor.set_template(Some(rich_edit_box_template()));
    editor.set_text("text").unwrap();
    let root = crate::test_support::TestRoot::with_child(editor.clone());
    rich_edit_layout(&root, Size::new(200.0, 80.0));
    editor.raise_event(&FocusChangedEventArgs::new(InputElement::got_focus_event()));
    assert!(editor.is_focused());
    assert!(editor.caret_visible());
    let timer = Dispatcher::timers_for_unit_tests()
        .into_iter()
        .find(|timer| timer.interval() == std::time::Duration::from_millis(500))
        .expect("Focused editable text schedules the framework caret timer");
    Dispatcher::force_fire_timer_for_unit_tests(&timer);
    assert!(!editor.caret_visible());
    editor.insert_text("x").unwrap();
    assert!(editor.caret_visible());
    assert!(timer.is_enabled());
    editor.select(0, 2);
    assert!(!editor.caret_visible());
    assert!(!timer.is_enabled());
    editor.select(2, 2);
    assert!(editor.caret_visible());
    assert!(timer.is_enabled());
    editor.raise_event(&FocusChangedEventArgs::new(InputElement::lost_focus_event()));
    assert!(!editor.caret_visible());
    assert!(!timer.is_enabled());
}

#[test]
fn rich_edit_zero_caret_interval_is_steady_and_detaching_stops_the_timer() {
    use ferroui_base::animation::TimeSpan;
    use ferroui_base::input::{FocusChangedEventArgs, InputElement};
    let _scope = crate::test_support::test_scope();
    let editor = RichEditBox::new();
    editor.set_template(Some(rich_edit_box_template()));
    editor.set_caret_blink_interval(TimeSpan::ZERO);
    let root = crate::test_support::TestRoot::with_child(editor.clone());
    rich_edit_layout(&root, Size::new(200.0, 80.0));
    let before = Dispatcher::timers_for_unit_tests().len();
    editor.raise_event(&FocusChangedEventArgs::new(InputElement::got_focus_event()));
    assert!(editor.caret_visible());
    assert_eq!(Dispatcher::timers_for_unit_tests().len(), before);
    editor.set_caret_blink_interval(TimeSpan::from_milliseconds(750.0));
    let timer = Dispatcher::timers_for_unit_tests()
        .into_iter()
        .find(|timer| timer.interval() == std::time::Duration::from_millis(750))
        .unwrap();
    assert!(timer.is_enabled());
    root.set_child(None::<ferroui_base::Ref<crate::Control>>);
    assert!(!timer.is_enabled());
    assert!(!editor.caret_visible());
}

#[test]
fn rich_edit_rtf_unicode_fallback_consumes_control_words_symbols_and_binary_tokens() {
    let _scope = Dispatcher::unit_test_scope();
    let snapshot =
        RtfReader::read(br"{\rtf1\uc1 \u945\par x\u946\b y\u947\bin3 abcz\u948\%q}").unwrap();
    assert_eq!(snapshot.text(), "αxβyγzδq");
    assert_eq!(snapshot.paragraphs.len(), 1);
    let document = RichTextDocument::new();
    document.restore(&snapshot).unwrap();
    assert_ne!(
        document.character_format_at(3).unwrap().font_weight,
        Some(FontWeight::Bold)
    );
}

#[test]
fn rich_edit_rtf_unicode_fallback_ends_at_every_group_boundary() {
    let _scope = Dispatcher::unit_test_scope();
    let snapshot =
        RtfReader::read(br"{\rtf1\uc3 \u945{\*\unknown ignored}x{\uc2\u946?}y\u947{\b z}q}")
            .unwrap();
    assert_eq!(snapshot.text(), "αxβyγzq");
    let document = RichTextDocument::new();
    document.restore(&snapshot).unwrap();
    assert_eq!(
        document.character_format_at(6).unwrap().font_weight,
        Some(FontWeight::Bold)
    );
    assert_ne!(
        document.character_format_at(7).unwrap().font_weight,
        Some(FontWeight::Bold)
    );
}

#[test]
fn rich_edit_rtf_prefers_unicode_alternatives_over_ansi_destinations() {
    let _scope = Dispatcher::unit_test_scope();
    let snapshot = RtfReader::read(br"{\rtf1 A{\upr{wrong}{\*\ud \uc0\u945}}Z}").unwrap();
    assert_eq!(snapshot.text(), "AαZ");
    assert!(matches!(
        RtfReader::read(br"{\rtf1{\upr{missing Unicode alternative}}}"),
        Err(RtfError::InvalidParameter(
            "Unicode alternative destination"
        )),
    ));
}

#[test]
fn rich_edit_rtf_unicode_rules_apply_to_font_names_and_hyperlink_instructions() {
    let _scope = Dispatcher::unit_test_scope();
    let snapshot = RtfReader::read(
        br#"{\rtf1{\fonttbl{\f0 {\upr{wrong}{\*\ud \u915\unknown amma}};}}\f0 x{\field{\*\fldinst HYPERLINK "https://example/\u945\unknown x"}{\fldrslt link}}}"#,
    ).unwrap();
    assert_eq!(snapshot.text(), "xlink");
    let document = RichTextDocument::new();
    document.restore(&snapshot).unwrap();
    assert_eq!(
        document.character_format_at(1).unwrap().font_family,
        Some(FontFamily::new("Γamma"))
    );
    assert!(snapshot.paragraphs[0].inlines.iter().any(|inline| {
        matches!(&inline.kind, InlineSnapshotKind::Hyperlink { navigate_uri: Some(uri), .. } if uri == "https://example/αx")
    }));
}

#[test]
fn rich_edit_rtf_character_spacing_units_and_plain_resets_roundtrip() {
    use ferroui_base::media::FontStretch;
    let _scope = Dispatcher::unit_test_scope();
    let snapshot = RtfReader::read(br"{\rtf1\expnd6 a{\expndtw-15 b}\plain c}").unwrap();
    let document = RichTextDocument::new();
    document.restore(&snapshot).unwrap();
    assert_eq!(
        document.character_format_at(1).unwrap().letter_spacing,
        Some(2.0)
    );
    assert_eq!(
        document.character_format_at(2).unwrap().letter_spacing,
        Some(-1.0)
    );
    assert_eq!(
        document.character_format_at(3).unwrap().letter_spacing,
        Some(0.0)
    );
    assert_eq!(
        document.character_format_at(3).unwrap().font_stretch,
        Some(FontStretch::Normal)
    );

    document
        .apply_format(
            0..1,
            &CharacterFormat {
                letter_spacing: Some(1.4),
                font_stretch: Some(FontStretch::Normal),
                ..Default::default()
            },
        )
        .unwrap();
    let rtf = RtfWriter::write(&document.snapshot()).unwrap();
    assert!(rtf.contains("\\expnd4\\expndtw21"));
    assert!(rtf.contains("\\charscalex100"));
    let copy = RichTextDocument::new();
    copy.restore(&RtfReader::read(rtf.as_bytes()).unwrap())
        .unwrap();
    assert_eq!(
        copy.character_format_at(1).unwrap().letter_spacing,
        Some(1.4)
    );
    assert_eq!(
        copy.character_format_at(1).unwrap().font_stretch,
        Some(FontStretch::Normal)
    );
}

#[test]
fn rich_edit_rtf_rejects_unrepresented_scaling_and_font_encodings() {
    let _scope = Dispatcher::unit_test_scope();
    for input in [
        br"{\rtf1\charscalex80 x}".as_slice(),
        br"{\rtf1\charscalex120 x}".as_slice(),
    ] {
        assert!(matches!(
            RtfReader::read(input),
            Err(RtfError::UnsupportedFeature("character scaling"))
        ));
    }
    for input in [
        br"{\rtf1\charscalex0 x}".as_slice(),
        br"{\rtf1\charscalex x}".as_slice(),
    ] {
        assert!(matches!(
            RtfReader::read(input),
            Err(RtfError::InvalidParameter("character scaling"))
        ));
    }
    assert!(matches!(
        RtfReader::read(br"{\rtf1{\fonttbl{\f0\fcharset2 Symbol;}}\f0 A}"),
        Err(RtfError::UnsupportedFeature("font character set")),
    ));
    assert!(matches!(
        RtfReader::read(br"{\rtf1{\fonttbl{\f0\cpg1361 Example;}}\f0 A}"),
        Err(RtfError::UnsupportedCodePage(1361)),
    ));
}

#[test]
fn rich_edit_rtf_font_names_with_semicolons_are_escaped_losslessly() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("text");
    document
        .apply_format(
            0..4,
            &CharacterFormat {
                font_family: Some(FontFamily::new("Example; Family")),
                ..Default::default()
            },
        )
        .unwrap();
    let rtf = RtfWriter::write(&document.snapshot()).unwrap();
    assert!(rtf.contains("Example\\u59? Family"));
    let copy = RichTextDocument::new();
    copy.restore(&RtfReader::read(rtf.as_bytes()).unwrap())
        .unwrap();
    assert_eq!(
        copy.character_format_at(1).unwrap().font_family,
        Some(FontFamily::new("Example; Family"))
    );
}

#[test]
fn rich_edit_rtf_export_rejects_metrics_that_cannot_fit_native_parameters() {
    let _scope = Dispatcher::unit_test_scope();
    for character in [
        CharacterFormat {
            font_size: Some(f64::MAX),
            ..Default::default()
        },
        CharacterFormat {
            font_size: Some(0.1),
            ..Default::default()
        },
        CharacterFormat {
            letter_spacing: Some(f64::MAX),
            ..Default::default()
        },
    ] {
        let snapshot = DocumentSnapshot {
            paragraphs: vec![ParagraphSnapshot {
                inlines: vec![run("text", character)],
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(matches!(
            RtfWriter::write(&snapshot),
            Err(RtfError::InvalidParameter(_))
        ));
    }
    for format in [
        ParagraphFormat {
            text_indent: Some(f64::MAX),
            ..Default::default()
        },
        ParagraphFormat {
            line_height: Some(f64::MAX),
            ..Default::default()
        },
    ] {
        let snapshot = DocumentSnapshot {
            paragraphs: vec![ParagraphSnapshot {
                format,
                inlines: vec![run("text", CharacterFormat::default())],
            }],
            ..Default::default()
        };
        assert!(matches!(
            RtfWriter::write(&snapshot),
            Err(RtfError::InvalidParameter(_))
        ));
    }
}

#[test]
fn rich_edit_ime_service_maps_preedit_cursor_through_crlf_normalization() {
    let _scope = Dispatcher::unit_test_scope();
    let document = RichTextDocument::from_text("x");
    let editor = RichTextEditor::new(document.clone());
    editor.set_selection(EditorSelection::caret(1)).unwrap();
    for (native_cursor, expected_caret) in [(3, 3), (4, 3), (5, 5)] {
        editor.set_preedit("a\r\n😀b", Some(native_cursor)).unwrap();
        assert_eq!(document.text(), "xa\n😀b");
        assert_eq!(editor.selection(), EditorSelection::caret(expected_caret));
        assert_eq!(editor.surrounding_text(), "x");
    }
    editor.cancel_composition().unwrap();
    assert_eq!(document.text(), "x");
    assert_eq!(editor.selection(), EditorSelection::caret(1));
}

#[test]
fn rich_edit_native_ime_cursor_follows_filtered_and_case_expanded_text() {
    let _scope = Dispatcher::unit_test_scope();
    let control = RichEditBox::new();
    control.set_text("x").unwrap();
    control.select(1, 1);
    control.set_accepts_tab(false);
    control.set_character_casing(RichEditCharacterCasing::Upper);
    let input = control.text_input_method_client();
    input.set_preedit_text_with_cursor(Some("ß\tz"), Some(2));
    assert_eq!(control.text(), "xSSZ");
    assert_eq!(control.caret_index(), 3);
    assert_eq!(input.surrounding_text(), "x");
    input.set_preedit_text(None);

    control.set_character_casing(RichEditCharacterCasing::Lower);
    input.set_preedit_text_with_cursor(Some("İZ"), Some(1));
    assert_eq!(control.text(), "xi\u{307}z");
    assert_eq!(control.caret_index(), 3);
    input.set_preedit_text(None);
    assert_eq!(control.text(), "x");
}

// Port-specific contract tests derived from CRichEditBox::OnSelectionChanged,
// CTextBoxBase::ProcessSelectionChangingEvent and the RichEditBox API contract.
#[test]
fn rich_edit_selection_changing_cancellation_preserves_selection_and_properties() {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichEditBox::new();
    editor.set_text("a🦀bc").unwrap();
    editor.select(1, 1);
    let proposals = Rc::new(RefCell::new(Vec::new()));
    let seen = proposals.clone();
    let weak = editor.downgrade();
    editor.selection_changing(move |_, args| {
        let owner = weak.upgrade().unwrap();
        seen.borrow_mut()
            .push((args.selection_start(), args.selection_length()));
        assert_eq!((owner.selection_start(), owner.selection_end()), (4, 1));
        assert_eq!(
            owner.text_document().selection().selection(),
            EditorSelection::new(4, 1)
        );
        args.clone().set_cancel(true);
        assert!(args.cancel(), "clones share the cancellation flag");
    });
    let changed = Rc::new(Cell::new(0));
    let count = changed.clone();
    editor.selection_changed(move |_| count.set(count.get() + 1));
    let property_changes = Rc::new(Cell::new(0));
    let count = property_changes.clone();
    editor.property_changed(move |change| {
        if matches!(
            change.property().name(),
            "SelectionStart" | "SelectionEnd" | "SelectedText"
        ) {
            count.set(count.get() + 1);
        }
    });
    editor.select(4, 2);
    assert_eq!(&*proposals.borrow(), &[(1, 3)]);
    assert_eq!(editor.editor().selection(), EditorSelection::caret(1));
    assert_eq!((editor.selection_start(), editor.selection_end()), (1, 1));
    assert_eq!(changed.get(), 0);
    assert_eq!(property_changes.get(), 0);
}

#[test]
fn rich_edit_selection_changing_reentrant_view_and_tom_changes_publish_once() {
    use std::cell::RefCell;
    use std::rc::Rc;
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichEditBox::new();
    editor.set_text("abcdef").unwrap();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let seen = changes.clone();
    let weak = editor.downgrade();
    editor.selection_changing(move |_, args| {
        seen.borrow_mut().push("changing");
        let owner = weak.upgrade().unwrap();
        assert_eq!(owner.editor().selection(), EditorSelection::new(1, 4));
        owner.text_document().selection().set_range(5, 2).unwrap();
        assert_eq!((owner.selection_start(), owner.selection_end()), (5, 2));
        owner.select(6, 3);
        args.set_cancel(true);
    });
    let seen = changes.clone();
    editor.selection_changed(move |args| {
        assert_eq!((args.old_start, args.old_end), (0, 0));
        assert_eq!((args.start, args.end), (6, 3));
        seen.borrow_mut().push("changed");
    });
    editor.text_document().selection().set_range(1, 4).unwrap();
    assert_eq!(editor.editor().selection(), EditorSelection::new(6, 3));
    assert_eq!((editor.selection_start(), editor.selection_end()), (6, 3));
    assert_eq!(&*changes.borrow(), &["changing", "changed"]);
}

#[test]
fn rich_edit_canceled_post_edit_selection_is_retained_in_undo_and_redo() {
    use std::cell::Cell;
    use std::rc::Rc;
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichEditBox::new();
    editor.set_text("abcd").unwrap();
    editor.select(2, 2);
    let canceled = Rc::new(Cell::new(false));
    let state = canceled.clone();
    editor.selection_changing(move |_, args| {
        if !state.replace(true) {
            assert_eq!((args.selection_start(), args.selection_length()), (3, 0));
            args.set_cancel(true);
        }
    });
    editor.insert_text("X").unwrap();
    assert_eq!(editor.text(), "abXcd");
    assert_eq!(editor.editor().selection(), EditorSelection::caret(2));
    assert_eq!((editor.selection_start(), editor.selection_end()), (2, 2));
    assert!(editor.undo().unwrap());
    assert_eq!(editor.text(), "abcd");
    assert_eq!(editor.editor().selection(), EditorSelection::caret(2));
    assert!(editor.redo().unwrap());
    assert_eq!(editor.text(), "abXcd");
    assert_eq!(editor.editor().selection(), EditorSelection::caret(2));
}

#[test]
fn rich_edit_grouped_range_edits_expose_only_the_final_user_selection() {
    use std::cell::RefCell;
    use std::rc::Rc;
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichEditBox::new();
    editor.set_text("abcdef").unwrap();
    editor.select(6, 6);
    let proposals = Rc::new(RefCell::new(Vec::new()));
    let seen = proposals.clone();
    editor.selection_changing(move |_, args| {
        seen.borrow_mut()
            .push((args.selection_start(), args.selection_length()));
    });
    editor
        .text_document()
        .get_range(1, 3)
        .unwrap()
        .set_text("X")
        .unwrap();
    assert_eq!(editor.text(), "aXdef");
    assert_eq!(editor.editor().selection(), EditorSelection::caret(5));
    assert_eq!(&*proposals.borrow(), &[(5, 0)]);
}

#[test]
fn rich_edit_selection_property_keyboard_and_ime_requests_share_cancellation() {
    use ferroui_base::input::{InputElement, Key, KeyEventArgs, KeyModifiers};
    use std::cell::Cell;
    use std::rc::Rc;
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichEditBox::new();
    editor.set_text("abcd").unwrap();
    let count = Rc::new(Cell::new(0));
    let seen = count.clone();
    editor.selection_changing(move |_, args| {
        seen.set(seen.get() + 1);
        args.set_cancel(true);
    });
    editor.set_direct_value(RichEditBox::selection_end_property(), 3);
    let mut key = KeyEventArgs::new();
    key.key = Key::Right;
    key.key_modifiers = KeyModifiers::NONE;
    key.set_routed_event(Some(InputElement::key_down_event()));
    editor.raise_event(&key);
    assert!(key.handled());
    editor
        .text_input_method_client()
        .set_selection(TextSelection::new(1, 2));
    assert_eq!(count.get(), 3);
    assert_eq!(editor.editor().selection(), EditorSelection::caret(0));
    assert_eq!(
        editor.text_input_method_client().selection(),
        TextSelection::new(0, 0)
    );
}

#[test]
fn rich_edit_text_changing_reports_content_and_formatting_before_deferred_text_changed() {
    use std::cell::RefCell;
    use std::rc::Rc;
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichEditBox::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let seen = events.clone();
    let weak = editor.downgrade();
    editor.text_changing(move |_, args| {
        assert_eq!(weak.upgrade().unwrap().text(), "abcd");
        seen.borrow_mut().push(if args.is_content_changing() {
            "content"
        } else {
            "format"
        });
    });
    let seen = events.clone();
    editor.text_changed(move |_, _| seen.borrow_mut().push("changed"));
    editor.set_text("abcd").unwrap();
    editor.select(0, 4);
    editor
        .apply_format(&CharacterFormat {
            font_weight: Some(FontWeight::Bold),
            ..CharacterFormat::default()
        })
        .unwrap();
    assert_eq!(&*events.borrow(), &["content", "format"]);
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(
        &*events.borrow(),
        &["content", "format", "changed", "changed"]
    );
}

#[test]
fn rich_edit_deferred_text_changed_drops_replaced_documents_and_allows_further_edits() {
    use std::cell::Cell;
    use std::rc::Rc;
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichEditBox::new();
    let count = Rc::new(Cell::new(0));
    let seen = count.clone();
    let weak = editor.downgrade();
    editor.text_changed(move |_, _| {
        let owner = weak.upgrade().unwrap();
        seen.set(seen.get() + 1);
        if seen.get() == 1 {
            assert_eq!(owner.text(), "new");
            owner.insert_text("!").unwrap();
        }
    });
    editor.set_text("obsolete").unwrap();
    editor.select(8, 8);
    let replacement = RichTextDocument::new();
    replacement.replace_text(0..0, "new").unwrap();
    assert!(editor.try_set_rich_document(replacement));
    assert_eq!((editor.selection_start(), editor.selection_end()), (0, 0));
    Dispatcher::ui_thread().run_jobs(None);
    assert_eq!(editor.text(), "!new");
    assert_eq!(count.get(), 2);
}

#[test]
fn rich_edit_deferred_notifications_do_not_retain_the_control() {
    let _scope = Dispatcher::unit_test_scope();
    let weak = {
        let editor = RichEditBox::new();
        editor.set_text("queued").unwrap();
        editor.downgrade()
    };
    assert!(weak.upgrade().is_none());
    Dispatcher::ui_thread().run_jobs(None);
}

#[test]
fn rich_edit_header_description_and_placeholder_use_framework_visuals_and_shared_geometry() {
    use crate::{Border, Control};
    use ferroui_base::{Point, Thickness};
    let _scope = crate::test_support::test_scope();
    let editor = RichEditBox::new();
    editor.set_template(Some(rich_edit_box_template()));
    let _root = crate::test_support::TestRoot::with_child(editor.clone());
    editor.set_padding(Thickness::new(3.0, 5.0, 7.0, 11.0));
    editor.set_text("abc").unwrap();
    rich_edit_layout(&_root, Size::new(300.0, 200.0));
    let original = editor.get_rect_from_character_index(1);
    let header = Border::new();
    header.set_width(90.0);
    header.set_height(20.0);
    let description = Border::new();
    description.set_width(90.0);
    description.set_height(14.0);
    editor.set_header(Some(Control::boxed(header.clone())));
    editor.set_description(Some(Control::boxed(description.clone())));
    editor.set_placeholder_text("Empty notes");
    rich_edit_layout(&_root, Size::new(300.0, 200.0));
    let header_presenter = editor.header_presenter().unwrap();
    assert_eq!(header_presenter.child(), Some(header.upcast()));
    assert_eq!(header_presenter.desired_size().height, 28.0);
    assert_eq!(
        editor.description_presenter().unwrap().child(),
        Some(description.upcast())
    );
    let character = editor.get_rect_from_character_index(1);
    assert_eq!(character.y, original.y + 28.0);
    assert_eq!(
        editor.hit_test_text_position(Point::new(
            character.x,
            character.y + character.height / 2.0
        )),
        1
    );
    editor.select(1, 1);
    assert_eq!(
        editor.text_input_method_client().cursor_rectangle(),
        editor.cursor_rectangle()
    );
    assert_eq!(editor.cursor_rectangle().y, character.y);
    let placeholder = editor.placeholder_presenter().unwrap();
    assert!(!placeholder.is_visible());
    assert!(!placeholder.is_hit_test_visible());
    assert_eq!(placeholder.text(), Some("Empty notes".to_owned()));
    assert_eq!(editor.visual_children().count(), 1);
    editor.set_text("").unwrap();
    assert!(placeholder.is_visible());
    rich_edit_layout(&_root, Size::new(300.0, 200.0));
    assert_eq!(placeholder.bounds().y + placeholder.padding().top, 33.0);
    editor.set_header(None);
    editor.set_description(None);
    rich_edit_layout(&_root, Size::new(300.0, 200.0));
    assert_eq!(placeholder.bounds().y + placeholder.padding().top, 5.0);
    assert!(!header_presenter.is_visible());
}

#[test]
fn rich_edit_header_template_renders_without_header_data_and_is_not_rebuilt_by_measure() {
    use crate::templates::FuncDataTemplate;
    use crate::Border;
    use std::cell::Cell;
    use std::rc::Rc;
    let _scope = crate::test_support::test_scope();
    let editor = RichEditBox::new();
    editor.set_template(Some(rich_edit_box_template()));
    let _root = crate::test_support::TestRoot::with_child(editor.clone());
    let builds = Rc::new(Cell::new(0));
    let count = builds.clone();
    editor.set_header_template(Some(FuncDataTemplate::new(
        |_| true,
        move |data, _| {
            assert!(data.is_none());
            count.set(count.get() + 1);
            let child = Border::new();
            child.set_width(80.0);
            child.set_height(24.0);
            Some(child.upcast())
        },
        false,
    )));
    assert_eq!(builds.get(), 0);
    editor.apply_template();
    let presenter = editor.header_presenter().unwrap();
    assert!(presenter.is_visible());
    rich_edit_layout(&_root, Size::new(300.0, 200.0));
    editor.invalidate_measure();
    rich_edit_layout(&_root, Size::new(250.0, 200.0));
    assert_eq!(builds.get(), 1);
    assert_eq!(presenter.desired_size().height, 32.0);
    editor.set_header_template(None);
    assert!(!presenter.is_visible());
    assert!(presenter.child().is_none());
}

#[test]
fn rich_edit_placeholder_participates_in_measure_alignment_and_template_binding() {
    use ferroui_base::media::{Brushes, Colors, TextWrapping};
    let _scope = crate::test_support::test_scope();
    let editor = RichEditBox::new();
    editor.set_template(Some(rich_edit_box_template()));
    let _root = crate::test_support::TestRoot::with_child(editor.clone());
    editor.set_foreground(Some(Brushes::blue()));
    editor.set_placeholder_foreground(Some(Brushes::blue()));
    editor.set_placeholder_text("A lengthy placeholder that is laid out by a real text block");
    editor.set_text_wrapping(TextWrapping::Wrap);
    editor.set_text_alignment(TextAlignment::Right);
    rich_edit_layout(&_root, Size::new(120.0, 400.0));
    let placeholder = editor.placeholder_presenter().unwrap();
    assert_eq!(placeholder.text_alignment(), TextAlignment::Right);
    assert_eq!(placeholder.text_wrapping(), TextWrapping::Wrap);
    assert!(placeholder.desired_size().height > editor.get_rect_from_character_index(0).height);
    assert!(editor.desired_size().height >= placeholder.desired_size().height);
    editor.set_placeholder_foreground(Some(Brushes::red()));
    assert_eq!(
        placeholder
            .foreground()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color(),
        Colors::RED
    );
    editor.set_placeholder_foreground(None);
    assert!(placeholder.foreground().is_none());
    assert_eq!(
        editor
            .text_presenter()
            .foreground()
            .unwrap()
            .as_solid_color_brush()
            .unwrap()
            .color(),
        Colors::BLUE
    );
}

#[test]
fn rich_edit_ime_rejection_and_read_only_transition_have_balanced_composition_events() {
    use std::cell::RefCell;
    use std::rc::Rc;
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichEditBox::new();
    let events = Rc::new(RefCell::new(Vec::new()));
    let seen = events.clone();
    editor.add_handler(
        RichEditBox::text_composition_started_event(),
        move |_, _| seen.borrow_mut().push("start"),
    );
    let seen = events.clone();
    editor.add_handler(
        RichEditBox::text_composition_ended_event(),
        move |_, args| {
            assert!(!args.committed);
            seen.borrow_mut().push("cancel");
        },
    );
    editor.set_is_read_only(true);
    assert!(matches!(
        editor.set_preedit_text(Some("x"), None),
        Err(EditError::ReadOnly)
    ));
    assert!(events.borrow().is_empty());
    editor.set_is_read_only(false);
    editor.set_preedit_text(Some("x"), None).unwrap();
    editor.set_is_read_only(true);
    assert_eq!(&*events.borrow(), &["start", "cancel"]);
    assert_eq!(editor.text(), "");
    assert!(!editor.editor().is_composing());
}

#[test]
fn rich_edit_can_paste_observes_effective_enabled_state() {
    let _scope = crate::test_support::test_scope();
    let editor = RichEditBox::new();
    let root = crate::test_support::TestRoot::with_child(editor.clone());
    assert!(editor.can_paste());
    root.set_is_enabled(false);
    assert!(editor.is_enabled());
    assert!(!editor.is_effectively_enabled());
    assert!(!editor.can_paste());
    assert!(!editor.get_direct_value(RichEditBox::can_paste_property()));
    root.set_is_enabled(true);
    assert!(editor.get_direct_value(RichEditBox::can_paste_property()));
}

#[test]
fn rich_edit_native_selection_notifications_publish_once_per_committed_selection() {
    use std::cell::Cell;
    use std::rc::Rc;
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichEditBox::new();
    editor.set_text("abcd").unwrap();
    let count = Rc::new(Cell::new(0));
    let seen = count.clone();
    let input = editor.text_input_method_client();
    let _subscription = input.selection_changed(Rc::new(move || seen.set(seen.get() + 1)));
    editor.select(1, 1);
    assert_eq!(count.get(), 1);
    editor.insert_text("X").unwrap();
    assert_eq!(count.get(), 2);
    editor
        .apply_format(&CharacterFormat {
            font_weight: Some(FontWeight::Bold),
            ..CharacterFormat::default()
        })
        .unwrap();
    assert_eq!(count.get(), 2);
    editor.set_preedit_text(Some("字"), Some(1)).unwrap();
    assert_eq!(
        count.get(),
        2,
        "preview positions do not change the committed IME selection"
    );
    editor.cancel_composition().unwrap();
    assert_eq!(count.get(), 2);
}

#[test]
fn rich_edit_header_and_embedded_control_remain_in_the_same_visual_tree_after_relayout() {
    use crate::documents::{InlineUIContainer, Paragraph};
    use crate::Border;
    use std::rc::Rc;
    let _scope = crate::test_support::test_scope();
    let editor = RichEditBox::new();
    editor.set_template(Some(rich_edit_box_template()));
    let _root = crate::test_support::TestRoot::with_child(editor.clone());
    let child = Border::new();
    child.set_width(30.0);
    child.set_height(20.0);
    let inline = InlineUIContainer::new();
    inline.set_child(child.clone());
    let paragraph = Paragraph::with_text("a");
    paragraph.add_child(inline);
    editor.blocks().add(paragraph);
    editor.set_header(Some(Rc::new("Notes".to_owned())));
    for width in [300.0, 280.0, 300.0] {
        rich_edit_layout(&_root, Size::new(width, 200.0));
        assert_eq!(
            child.visual_parent(),
            Some(editor.text_presenter().upcast())
        );
        assert_eq!(
            editor
                .header_presenter()
                .unwrap()
                .find_ancestor_of_type::<RichEditBox>(false),
            Some(editor.clone())
        );
        assert_eq!(editor.visual_children().count(), 1);
        let bounds = Rect::from_size(child.bounds().size())
            .transform_to_aabb(child.transform_to_visual(&editor).unwrap());
        assert!(bounds.y >= editor.header_presenter().unwrap().desired_size().height);
    }
}

#[test]
fn rich_edit_detached_text_document_services_do_not_select_the_replacement_control_document() {
    let _scope = Dispatcher::unit_test_scope();
    let editor = RichEditBox::new();
    editor.set_text("previous").unwrap();
    let detached = editor.text_document();
    let replacement = RichTextDocument::new();
    replacement.replace_text(0..0, "new").unwrap();
    assert!(editor.try_set_rich_document(replacement));
    editor.select(1, 1);
    detached.selection().set_range(3, 6).unwrap();
    assert_eq!(detached.selection().selection(), EditorSelection::new(3, 6));
    detached.selection().type_text("X").unwrap();
    assert_eq!(editor.text(), "new");
    assert_eq!((editor.selection_start(), editor.selection_end()), (1, 1));
    assert_eq!(editor.editor().selection(), EditorSelection::caret(1));
}

fn rich_edit_press_key(editor: &RichEditBox, key: ferroui_base::input::Key) {
    use ferroui_base::input::{InputElement, KeyEventArgs};
    let mut event = KeyEventArgs::new();
    event.key = key;
    event.set_routed_event(Some(InputElement::key_down_event()));
    editor.raise_event(&event);
    assert!(event.handled());
}

#[test]
fn rich_edit_home_and_end_keep_paragraph_end_and_empty_paragraph_positions_on_their_line() {
    use ferroui_base::input::Key;
    let _scope = crate::test_support::test_scope();
    let editor = RichEditBox::new();
    editor.set_template(Some(rich_edit_box_template()));
    let _root = crate::test_support::TestRoot::with_child(editor.clone());
    editor.set_text("abc\n\nend").unwrap();
    rich_edit_layout(&_root, Size::new(300.0, 200.0));
    editor.select(3, 3);
    rich_edit_press_key(&editor, Key::Home);
    assert_eq!(editor.caret_index(), 0);
    rich_edit_press_key(&editor, Key::End);
    assert_eq!(editor.caret_index(), 3);
    editor.select(4, 4);
    rich_edit_press_key(&editor, Key::Home);
    assert_eq!(editor.caret_index(), 4);
    rich_edit_press_key(&editor, Key::End);
    assert_eq!(editor.caret_index(), 4);
}

#[test]
fn rich_edit_end_stops_before_an_inline_line_break() {
    use ferroui_base::input::Key;
    let _scope = crate::test_support::test_scope();
    let editor = RichEditBox::new();
    editor.set_template(Some(rich_edit_box_template()));
    let _root = crate::test_support::TestRoot::with_child(editor.clone());
    let line_break = InlineSnapshot {
        format: CharacterFormat::default(),
        kind: InlineSnapshotKind::LineBreak,
    };
    // Inline LineBreak uses the platform newline: CRLF occupies two UTF-16
    // positions on Windows, so the following line does not always start at 4.
    let following_line_start = 3 + line_break.text_length();
    editor
        .rich_document()
        .restore(&DocumentSnapshot {
            paragraphs: vec![ParagraphSnapshot {
                format: ParagraphFormat::default(),
                inlines: vec![
                    run("abc", CharacterFormat::default()),
                    line_break,
                    run("def", CharacterFormat::default()),
                ],
            }],
            ..Default::default()
        })
        .unwrap();
    rich_edit_layout(&_root, Size::new(300.0, 200.0));
    editor.select(0, 0);
    rich_edit_press_key(&editor, Key::End);
    assert_eq!(editor.caret_index(), 3);
    editor.select(following_line_start, following_line_start);
    rich_edit_press_key(&editor, Key::Home);
    assert_eq!(editor.caret_index(), following_line_start);
}

#[test]
fn rich_edit_vertical_navigation_preserves_desired_column_across_short_lines() {
    use ferroui_base::input::Key;
    let _scope = crate::test_support::test_scope();
    let editor = RichEditBox::new();
    editor.set_template(Some(rich_edit_box_template()));
    let _root = crate::test_support::TestRoot::with_child(editor.clone());
    editor.set_text("abcdef\nx\nabcdef").unwrap();
    rich_edit_layout(&_root, Size::new(300.0, 200.0));
    editor.select(5, 5);
    rich_edit_press_key(&editor, Key::Down);
    assert_eq!(editor.caret_index(), 8);
    rich_edit_press_key(&editor, Key::Down);
    assert_eq!(editor.caret_index(), 14);
    editor.select(0, 0);
    rich_edit_press_key(&editor, Key::Down);
    assert_eq!(editor.caret_index(), 7);
    rich_edit_press_key(&editor, Key::Down);
    assert_eq!(editor.caret_index(), 9);
}
#[test]
fn rich_edit_retemplate_preserves_composition_history_and_live_document_services() {
    use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
    use crate::{Border, ContentControl};
    use std::rc::Rc;

    let _scope = crate::test_support::test_scope();
    let editor = RichEditBox::new();
    editor.set_template(Some(rich_edit_box_template()));
    let _root = crate::test_support::TestRoot::with_child(editor.clone());
    editor.set_text("original").unwrap();
    rich_edit_layout(&_root, Size::new(220.0, 90.0));
    let presenter = editor.text_presenter();
    let document = editor.rich_document();
    let service = editor.editor();
    let range = editor.text_document().get_range(0, 3).unwrap();
    let old_host = editor
        .content_element()
        .unwrap()
        .cast::<ContentControl>()
        .unwrap();
    editor.select(8, 8);
    editor.insert_text("!").unwrap();
    editor.set_preedit_text(Some("preview"), Some(2)).unwrap();
    let preview = editor.rich_document().snapshot();
    let selection = service.selection();

    editor.set_template(Some(FuncControlTemplate::for_type::<RichEditBox>(
        |_, scope| {
            let host = Border::new();
            host.set_name(Some("ContentElement".to_owned()));
            host.set_padding(ferroui_base::Thickness::uniform(9.0));
            host.register_in_name_scope(&**scope).upcast()
        },
    )));
    editor.apply_template();
    rich_edit_layout(&_root, Size::new(180.0, 80.0));

    assert_eq!(editor.text_presenter(), presenter);
    assert_eq!(editor.rich_document(), document);
    assert!(Rc::ptr_eq(&editor.editor(), &service));
    assert!(service.is_composing());
    assert_eq!(editor.rich_document().snapshot(), preview);
    assert_eq!(service.selection(), selection);
    assert_eq!(range.text().unwrap(), "ori");
    assert!(old_host.content().is_none());
    assert_eq!(presenter.templated_parent(), Some(editor.clone().upcast()));
    assert_eq!(
        presenter.parent(),
        editor.content_element().map(|host| host.upcast())
    );

    editor.cancel_composition().unwrap();
    editor.undo().unwrap();
    assert_eq!(editor.text(), "original");
    editor.redo().unwrap();
    assert_eq!(editor.text(), "original!");
    editor.set_template(None);
    editor.apply_template();
    assert!(presenter.visual_parent().is_none());
    assert!(editor.content_element().is_none());
    editor.select(9, 9);
    editor.insert_text("?").unwrap();
    editor.set_template(Some(rich_edit_box_template()));
    editor.apply_template();
    rich_edit_layout(&_root, Size::new(180.0, 80.0));
    assert_eq!(editor.text_presenter(), presenter);
    assert_eq!(editor.text(), "original!?");
    assert_eq!(range.text().unwrap(), "ori");
}

#[test]
fn rich_edit_content_element_accepts_framework_hosts_and_missing_parts_are_optional() {
    use crate::presenters::ContentPresenter;
    use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
    use crate::{Border, ContentControl, Control, Panel, TextBlock};
    use ferroui_base::Ref;

    let _scope = crate::test_support::test_scope();
    let editor = RichEditBox::new();
    let _root = crate::test_support::TestRoot::with_child(editor.clone());
    editor.set_text("persistent document").unwrap();
    let presenter = editor.text_presenter();
    for kind in 0..6 {
        editor.set_template(Some(FuncControlTemplate::for_type::<RichEditBox>(
            move |_, scope| {
                let host: Ref<Control> = match kind {
                    0 => Border::new().upcast(),
                    1 => ContentPresenter::new().upcast(),
                    2 => Panel::new().upcast(),
                    3 => ContentControl::new().upcast(),
                    _ => TextBlock::new().upcast(),
                };
                if kind != 5 {
                    host.set_name(Some("ContentElement".to_owned()));
                    host.register_in_name_scope(&**scope)
                } else {
                    host
                }
            },
        )));
        editor.apply_template();
        rich_edit_layout(&_root, Size::new(180.0, 80.0));
        assert_eq!(editor.text(), "persistent document");
        assert_eq!(editor.text_presenter(), presenter);
        assert_eq!(editor.content_element().is_some(), kind < 4);
        assert_eq!(presenter.visual_parent().is_some(), kind < 4);
        assert!(editor.header_presenter().is_none());
        assert!(editor.description_presenter().is_none());
        assert!(editor.placeholder_presenter().is_none());
    }
}

#[test]
fn rich_edit_generic_content_hosts_reveal_the_caret_without_an_ancestor_scroller() {
    use crate::presenters::ContentPresenter;
    use crate::templates::{FuncControlTemplate, FuncTemplateNameScopeExtensions};
    use crate::{Border, ContentControl, Control, Panel};
    use ferroui_base::input::IScrollable;
    use ferroui_base::utilities::MathUtilities;
    use ferroui_base::Ref;

    let _scope = crate::test_support::test_scope();
    let editor = RichEditBox::new();
    let root = crate::test_support::TestRoot::with_child(editor.clone());
    editor
        .set_text(
            &(0..20)
                .map(|line| format!("line {line}"))
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap();
    let presenter = editor.text_presenter();
    for kind in 0..4 {
        editor.set_template(Some(FuncControlTemplate::for_type::<RichEditBox>(
            move |_, scope| {
                let host: Ref<Control> = match kind {
                    0 => Border::new().upcast(),
                    1 => ContentPresenter::new().upcast(),
                    2 => Panel::new().upcast(),
                    _ => ContentControl::new().upcast(),
                };
                host.set_name(Some("ContentElement".to_owned()));
                host.register_in_name_scope(&**scope)
            },
        )));
        rich_edit_layout(&root, Size::new(180.0, 60.0));
        editor.select(0, 0);
        Dispatcher::ui_thread().run_jobs(None);
        rich_edit_layout(&root, Size::new(180.0, 60.0));
        assert_eq!(presenter.offset().y, 0.0);
        assert!(presenter.extent().height > presenter.viewport().height);

        let end = editor.rich_document().text_length();
        editor.select(end, end);
        rich_edit_layout(&root, Size::new(180.0, 60.0));
        Dispatcher::ui_thread().run_jobs(None);
        assert!(presenter.offset().y > 0.0);
        let caret = presenter.cursor_rectangle();
        assert!(
            MathUtilities::greater_than_or_close(caret.y, 0.0),
            "host {kind}: caret {caret:?}, extent {:?}, viewport {:?}, offset {:?}",
            presenter.extent(),
            presenter.viewport(),
            presenter.offset(),
        );
        assert!(
            MathUtilities::less_than_or_close(caret.bottom(), presenter.viewport().height),
            "host {kind}: caret {caret:?}, extent {:?}, viewport {:?}, offset {:?}",
            presenter.extent(),
            presenter.viewport(),
            presenter.offset(),
        );
        assert_eq!(editor.editor().selection(), EditorSelection::new(end, end));
        assert_eq!(editor.text_presenter(), presenter);
    }
}

#[test]
fn rich_edit_scroll_view_formats_the_full_document_and_clamps_offsets_after_shrink() {
    use crate::ScrollViewer;
    use ferroui_base::input::IScrollable;
    use ferroui_base::Vector;

    let _scope = crate::test_support::test_scope();
    let editor = RichEditBox::new();
    editor.set_template(Some(rich_edit_box_template()));
    let _root = crate::test_support::TestRoot::with_child(editor.clone());
    editor
        .set_text(
            &(0..40)
                .map(|line| format!("line {line}"))
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap();
    rich_edit_layout(&_root, Size::new(160.0, 70.0));
    let presenter = editor.text_presenter();
    let scroll = editor
        .content_element()
        .unwrap()
        .cast::<ScrollViewer>()
        .unwrap();
    assert_eq!(presenter.line_metrics().len(), 40);
    assert_eq!(presenter.content_end(), editor.rich_document().text_length());
    assert!(!presenter.has_overflow_content());
    assert!(presenter.extent().height > presenter.viewport().height * 4.0);
    assert_eq!(scroll.extent(), presenter.extent());
    assert_eq!(scroll.viewport(), presenter.viewport());

    presenter.set_offset(Vector::new(f64::INFINITY, f64::INFINITY));
    assert_eq!(presenter.offset().x, 0.0);
    assert_eq!(
        presenter.offset().y,
        presenter.extent().height - presenter.viewport().height
    );
    assert_eq!(scroll.offset(), presenter.offset());
    rich_edit_layout(&_root, Size::new(160.0, 70.0));
    assert!(presenter.get_rect_from_character_index(0).y < 0.0);

    editor.set_text("tiny").unwrap();
    rich_edit_layout(&_root, Size::new(160.0, 70.0));
    assert_eq!(presenter.offset(), Vector::default());
    assert_eq!(scroll.offset(), Vector::default());
    assert_eq!(presenter.extent(), presenter.viewport());
}

#[test]
fn rich_edit_wrapping_uses_viewport_width_and_no_wrap_preserves_horizontal_extent() {
    use ferroui_base::input::IScrollable;
    use ferroui_base::media::TextWrapping;
    use ferroui_base::Vector;

    let _scope = crate::test_support::test_scope();
    let editor = RichEditBox::new();
    editor.set_template(Some(rich_edit_box_template()));
    let _root = crate::test_support::TestRoot::with_child(editor.clone());
    editor.set_text(&"wide words ".repeat(30)).unwrap();
    rich_edit_layout(&_root, Size::new(140.0, 70.0));
    let presenter = editor.text_presenter();
    assert!(presenter.line_metrics().len() > 1);
    assert_eq!(presenter.extent().width, presenter.viewport().width);
    presenter.set_offset(Vector::new(100.0, 0.0));
    assert_eq!(presenter.offset().x, 0.0);

    editor.set_text_wrapping(TextWrapping::NoWrap);
    rich_edit_layout(&_root, Size::new(140.0, 70.0));
    assert_eq!(presenter.line_metrics().len(), 1);
    assert!(presenter.extent().width > presenter.viewport().width * 2.0);
    presenter.set_offset(Vector::new(100.0, 0.0));
    assert_eq!(presenter.offset().x, 100.0);
    assert!(presenter.get_rect_from_character_index(0).x < 0.0);

    editor.set_text_wrapping(TextWrapping::Wrap);
    rich_edit_layout(&_root, Size::new(140.0, 70.0));
    assert_eq!(presenter.offset().x, 0.0);
    assert_eq!(presenter.extent().width, presenter.viewport().width);
    assert!(presenter.line_metrics().len() > 1);
}

#[test]
fn rich_edit_caret_scroll_keeps_template_chrome_fixed_and_ime_geometry_in_owner_coordinates() {
    use crate::{Border, Control};
    use ferroui_base::input::IScrollable;
    use ferroui_base::{Point, Thickness};

    let _scope = crate::test_support::test_scope();
    let editor = RichEditBox::new();
    editor.set_template(Some(rich_edit_box_template()));
    let _root = crate::test_support::TestRoot::with_child(editor.clone());
    let header = Border::new();
    header.set_height(20.0);
    let description = Border::new();
    description.set_height(14.0);
    editor.set_header(Some(Control::boxed(header)));
    editor.set_description(Some(Control::boxed(description)));
    editor.set_padding(Thickness::new(3.0, 5.0, 7.0, 11.0));
    editor
        .set_text(
            &(0..40)
                .map(|line| format!("line {line}"))
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap();
    rich_edit_layout(&_root, Size::new(220.0, 150.0));
    let header_bounds = editor.header_presenter().unwrap().bounds();
    let description_bounds = editor.description_presenter().unwrap().bounds();
    let length = editor.rich_document().text_length();
    editor.select(length, length);
    rich_edit_layout(&_root, Size::new(220.0, 150.0));
    let presenter = editor.text_presenter();
    assert!(presenter.offset().y > 0.0);
    assert_eq!(editor.header_presenter().unwrap().bounds(), header_bounds);
    assert_eq!(
        editor.description_presenter().unwrap().bounds(),
        description_bounds
    );
    let owner_rect = presenter
        .cursor_rectangle()
        .transform_to_aabb(presenter.transform_to_visual(&editor).unwrap());
    assert_eq!(editor.cursor_rectangle(), owner_rect);
    assert_eq!(
        editor.text_input_method_client().cursor_rectangle(),
        owner_rect
    );
    assert_eq!(
        editor.text_input_method_client().text_view_visual(),
        Some(presenter.clone().upcast())
    );
    let viewport_rect = Rect::from_size(presenter.viewport())
        .transform_to_aabb(presenter.transform_to_visual(&editor).unwrap());
    assert!(owner_rect.y >= viewport_rect.y);
    assert!(owner_rect.bottom() <= viewport_rect.bottom() + 1e-6);
    let character = editor.get_rect_from_character_index(length - 1);
    assert_eq!(
        editor.hit_test_text_position(Point::new(
            character.x,
            character.y + character.height / 2.0
        )),
        length - 1
    );
}

#[test]
fn rich_edit_selection_focus_brushes_restore_inline_format_and_observe_mutable_transparency() {
    use crate::documents::{Paragraph, Run};
    use crate::RichTextBlockImplExt;
    use ferroui_base::input::{FocusChangedEventArgs, InputElement};
    use ferroui_base::media::text_formatting::{ITextSource, TextRunProperties};
    use ferroui_base::media::{Brushes, Colors, IBrush, SolidColorBrush};
    use std::rc::Rc;

    let _scope = crate::test_support::test_scope();
    let editor = RichEditBox::new();
    editor.set_template(Some(rich_edit_box_template()));
    let _root = crate::test_support::TestRoot::with_child(editor.clone());
    let run = Run::with_text(Some("color"));
    run.set_foreground(Some(Brushes::blue()));
    run.set_background(Some(Brushes::green()));
    let paragraph = Paragraph::new();
    paragraph.add_child(run);
    editor.blocks().add(paragraph);
    editor.set_selection_foreground_brush(Some(Brushes::white()));
    editor.set_selection_highlight_color(Some(Brushes::red()));
    rich_edit_layout(&_root, Size::new(220.0, 80.0));
    editor.select(0, 3);
    let presenter = editor.text_presenter();
    let revision = editor.rich_document().revision();
    let properties = || -> Rc<dyn TextRunProperties> {
        presenter.page_layout().source.paragraphs[0]
            .source
            .get_text_run(1)
            .unwrap()
            .properties()
            .unwrap()
            .clone()
    };
    let color = |brush: Option<&Rc<dyn IBrush>>| {
        brush
            .and_then(|brush| brush.as_solid_color_brush())
            .map(|brush| brush.color())
    };
    assert!(!presenter.selection_is_visible());
    assert_eq!(color(properties().foreground_brush()), Some(Colors::BLUE));
    assert_eq!(color(properties().background_brush()), Some(Colors::GREEN));

    editor.raise_event(&FocusChangedEventArgs::new(InputElement::got_focus_event()));
    assert!(presenter.selection_is_visible());
    assert_eq!(
        color(presenter.selection_render_brush().as_ref()),
        Some(Colors::RED)
    );
    assert_eq!(color(properties().foreground_brush()), Some(Colors::WHITE));
    assert!(properties().background_brush().is_none());
    editor.raise_event(&FocusChangedEventArgs::new(InputElement::lost_focus_event()));
    assert!(!presenter.selection_is_visible());
    assert_eq!(color(properties().background_brush()), Some(Colors::GREEN));

    let inactive = SolidColorBrush::with_color(Colors::TRANSPARENT);
    editor.set_selection_highlight_color_when_not_focused(Some(inactive.clone().into()));
    inactive.set_color(Colors::RED);
    assert!(presenter.selection_is_visible());
    assert_eq!(color(properties().foreground_brush()), Some(Colors::WHITE));
    inactive.set_opacity(0.0);
    assert!(!presenter.selection_is_visible());
    assert_eq!(color(properties().foreground_brush()), Some(Colors::BLUE));
    assert_eq!(color(properties().background_brush()), Some(Colors::GREEN));
    assert_eq!(editor.rich_document().revision(), revision);
}

#[test]
fn rich_edit_presenter_selection_events_and_ime_requests_keep_the_owner_identity() {
    use crate::documents::{InlineUIContainer, Paragraph};
    use crate::{Border, RichTextBlock};
    use ferroui_base::input::text_input::TextInputMethodClientRequestedEventArgs;
    use ferroui_base::input::InputElement;
    use std::cell::Cell;
    use std::rc::Rc;

    let _scope = crate::test_support::test_scope();
    let editor = RichEditBox::new();
    editor.set_template(Some(rich_edit_box_template()));
    let _root = crate::test_support::TestRoot::with_child(editor.clone());
    let child = Border::new();
    child.set_width(20.0);
    child.set_height(20.0);
    let paragraph = Paragraph::with_text("text");
    paragraph.add_child(InlineUIContainer::with_child(child.clone()));
    editor.blocks().add(paragraph);
    rich_edit_layout(&_root, Size::new(220.0, 80.0));
    let owner_events = Rc::new(Cell::new(0));
    let observed = owner_events.clone();
    let weak = editor.downgrade();
    editor.add_handler(RichEditBox::selection_changed_event(), move |_, event| {
        let owner = weak.upgrade().unwrap();
        assert_eq!(event.source(), Some(owner.upcast()));
        observed.set(observed.get() + 1);
    });
    let presenter_events = Rc::new(Cell::new(0));
    let observed = presenter_events.clone();
    editor
        .text_presenter()
        .add_handler(RichTextBlock::selection_changed_event(), move |_, _| {
            observed.set(observed.get() + 1);
        });
    editor.text_presenter().select(1, 3);
    assert_eq!(owner_events.get(), 1);
    assert_eq!(presenter_events.get(), 0);
    assert_eq!(editor.editor().selection(), EditorSelection::new(1, 3));

    let event = TextInputMethodClientRequestedEventArgs::new();
    event.set_routed_event(Some(
        InputElement::text_input_method_client_requested_event(),
    ));
    editor.raise_event(&event);
    assert!(Rc::ptr_eq(
        &event.client().unwrap(),
        &editor.text_input_method_client()
    ));
    let event = TextInputMethodClientRequestedEventArgs::new();
    event.set_routed_event(Some(
        InputElement::text_input_method_client_requested_event(),
    ));
    child.raise_event(&event);
    assert!(event.client().is_none());
}

#[test]
fn rich_edit_handled_embedded_control_pointer_input_does_not_move_selection() {
    use crate::documents::{InlineUIContainer, Paragraph};
    use crate::Border;
    use ferroui_base::input::{
        InputElementImplExt, KeyModifiers, Pointer, PointerPointProperties,
        PointerPressedEventArgs, PointerType, PointerUpdateKind,
    };

    let _scope = crate::test_support::test_scope();
    let editor = RichEditBox::new();
    editor.set_template(Some(rich_edit_box_template()));
    let root = crate::test_support::TestRoot::with_child(editor.clone());
    let child = Border::new();
    child.set_width(20.0);
    child.set_height(20.0);
    let paragraph = Paragraph::with_text("prefix");
    paragraph.add_child(InlineUIContainer::with_child(child.clone()));
    editor.blocks().add(paragraph);
    rich_edit_layout(&root, Size::new(220.0, 80.0));
    editor.select(0, 0);
    let point = child.bounds().center();
    let pointer = Pointer::new(Pointer::get_next_free_id(), PointerType::Mouse, true);
    let event = PointerPressedEventArgs::new(
        child,
        pointer,
        &root,
        point,
        0,
        PointerPointProperties::new(
            ferroui_base::input::RawInputModifiers::LEFT_MOUSE_BUTTON,
            PointerUpdateKind::LeftButtonPressed,
        ),
        KeyModifiers::NONE,
        1,
    );
    event.set_handled(true);
    editor.text_presenter().on_pointer_pressed(&event);
    assert_eq!(editor.editor().selection(), EditorSelection::default());
    assert_eq!((editor.selection_start(), editor.selection_end()), (0, 0));
}

#[test]
fn rich_edit_line_services_use_paragraph_fallback_until_the_presenter_is_measured() {
    use ferroui_base::media::TextWrapping;

    let _scope = crate::test_support::test_scope();
    let editor = RichEditBox::new();
    editor.set_text_wrapping(TextWrapping::Wrap);
    editor
        .set_text("one two three four five six seven eight")
        .unwrap();
    let services = editor.text_document();
    let range = services.get_range(0, 0).unwrap();
    assert!(!editor.text_presenter().has_measured_layout());
    range.expand(TextRangeUnit::Line).unwrap();
    assert_eq!(range.range(), 0..editor.rich_document().text_length());

    // Offscreen measurement is sufficient. A window is not required for a
    // caller to request real geometry from a FerroUI document presenter.
    editor.text_presenter().measure(Size::new(90.0, 80.0));
    assert!(editor.text_presenter().has_measured_layout());
    let lines = editor.line_metrics();
    assert!(lines.len() > 1);
    range.set_range(0, 0).unwrap();
    range.expand(TextRangeUnit::Line).unwrap();
    assert_eq!(range.range(), 0..lines[1].start);
    editor.rich_document().replace_text(0..0, "x").unwrap();
    range.set_range(0, 0).unwrap();
    range.expand(TextRangeUnit::Line).unwrap();
    let lines = editor.line_metrics();
    assert!(lines.len() > 1);
    assert_eq!(range.range(), 0..lines[1].start);
}

#[test]
fn rich_edit_range_edge_scroll_alignment_is_stable_before_rearrange() {
    use ferroui_base::input::IScrollable;
    let _scope = crate::test_support::test_scope();
    let editor = RichEditBox::new();
    editor.set_template(Some(rich_edit_box_template()));
    let root = crate::test_support::TestRoot::with_child(editor.clone());
    editor
        .set_text(
            &(0..20)
                .map(|line| format!("line {line}"))
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .unwrap();
    rich_edit_layout(&root, Size::new(160.0, 40.0));
    let presenter = editor.text_presenter();
    let index = presenter.line_metrics()[8].start;
    let rect = presenter.get_rect_from_character_index(index);
    assert!(presenter.bring_text_rect_into_view_aligned(rect, false));
    let bottom = rect.bottom() - presenter.viewport().height;
    assert_eq!(presenter.offset().y, bottom);
    assert!(!presenter
        .bring_text_rect_into_view_aligned(presenter.get_rect_from_character_index(index), false));
    presenter
        .bring_text_rect_into_view_aligned(presenter.get_rect_from_character_index(index), true);
    assert_eq!(presenter.offset().y, rect.y);
    assert_eq!(editor.editor().selection(), EditorSelection::default());
}

fn rich_edit_flyout_services(overlay: bool) -> crate::testing::UnitTestApplicationScope {
    use crate::platform::{IPopupImpl, ITopLevelImpl};
    use crate::testing::{MockWindowingPlatform, TestServices, UnitTestApplication};
    use std::rc::Rc;

    UnitTestApplication::start(TestServices::focusable_window().with_windowing_platform(
        MockWindowingPlatform::with_window_impl(move || {
            let window = MockWindowingPlatform::create_window_mock();
            let weak = Rc::downgrade(&window);
            window.setup_create_popup(move |_| {
                if overlay {
                    None
                } else {
                    let parent: Rc<dyn ITopLevelImpl> = weak.upgrade()?;
                    Some(MockWindowingPlatform::create_popup_mock(parent) as Rc<dyn IPopupImpl>)
                }
            });
            window
        }),
    ))
}

fn rich_edit_flyout_fixture() -> (
    ferroui_base::Ref<crate::Window>,
    ferroui_base::Ref<RichEditBox>,
    ferroui_base::Ref<crate::flyouts::Flyout>,
    ferroui_base::Ref<crate::Border>,
    ferroui_base::Ref<crate::Border>,
) {
    use crate::flyouts::{Flyout, FlyoutShowMode};
    use crate::{Border, Control, StackPanel, Window};

    let editor = RichEditBox::new();
    editor.set_template(Some(rich_edit_box_template()));
    editor.set_height(80.0);
    editor.set_text("editable text").unwrap();
    let outside = Border::new();
    outside.set_focusable(true);
    outside.set_height(30.0);
    let panel = StackPanel::new();
    panel.children().add(editor.clone());
    panel.children().add(outside.clone());
    let window = Window::new();
    window.set_width(260.0);
    window.set_height(140.0);
    window.set_content(Some(Control::boxed(panel)));

    let popup_child = Border::new();
    popup_child.set_focusable(true);
    popup_child.set_width(70.0);
    popup_child.set_height(30.0);
    let flyout = Flyout::new();
    flyout.set_content(Some(Control::boxed(popup_child.clone())));
    flyout.set_show_mode(FlyoutShowMode::Standard);
    editor.set_context_flyout(flyout.clone());
    window.show();
    editor.focus();
    assert!(editor.is_focused());
    (window, editor, flyout, popup_child, outside)
}

#[test]
fn rich_edit_owned_context_flyout_preserves_focused_selection_and_owner_identity() {
    use crate::flyouts::FlyoutBaseImplExt;
    use ferroui_base::input::InputElement;
    use ferroui_base::media::text_formatting::testing::TextTestScope;
    use ferroui_base::media::{Brushes, Colors};
    use std::cell::Cell;
    use std::rc::Rc;

    for overlay in [false, true] {
        let _app = rich_edit_flyout_services(overlay);
        let _text = TextTestScope::new();
        let (window, editor, flyout, popup_child, _) = rich_edit_flyout_fixture();
        editor.set_selection_highlight_color(Some(Brushes::red()));
        editor.set_selection_highlight_color_when_not_focused(Some(Brushes::blue()));
        editor.select(1, 5);
        let observed = Rc::new(Cell::new(false));
        let checking = Rc::new(Cell::new(true));
        let count = observed.clone();
        let check = checking.clone();
        let weak = editor.downgrade();
        let subscription = editor.property_changed(move |change| {
            if check.get()
                && change.property() == InputElement::is_focused_property().as_property()
                && !change.get_new_value::<bool>()
            {
                let editor = weak.upgrade().unwrap();
                let brush = editor.active_selection_brush().unwrap();
                count.set(brush.as_solid_color_brush().unwrap().color() == Colors::RED);
            }
        });
        flyout.show_at(&editor);
        checking.set(false);
        assert!(
            observed.get(),
            "focus observers must see the retained brush"
        );
        assert!(!editor.is_focused());
        assert!(popup_child.is_focused());
        assert_eq!(
            window.focus_manager().get_focused_element(),
            Some(popup_child.upcast::<InputElement>())
        );
        assert!(editor.classes().contains(":flyout-focus"));
        assert_eq!((editor.selection_start(), editor.selection_end()), (1, 5));
        assert_eq!(editor.selected_text(), "dita");
        assert_eq!(
            editor
                .active_selection_brush()
                .unwrap()
                .as_solid_color_brush()
                .unwrap()
                .color(),
            Colors::RED
        );

        editor.focus();
        flyout.hide();
        assert!(editor.is_focused());
        assert!(!editor.classes().contains(":flyout-focus"));
        assert_eq!((editor.selection_start(), editor.selection_end()), (1, 5));
        subscription.dispose();
        window.close();
    }
}

#[test]
fn rich_edit_context_flyout_defers_composition_blur_until_focus_leaves_its_content() {
    use crate::flyouts::FlyoutBaseImplExt;
    use ferroui_base::media::text_formatting::testing::TextTestScope;
    use std::cell::Cell;
    use std::rc::Rc;

    for overlay in [false, true] {
        let _app = rich_edit_flyout_services(overlay);
        let _text = TextTestScope::new();
        let (window, editor, flyout, popup_child, outside) = rich_edit_flyout_fixture();
        editor.select(8, 8);
        editor.set_preedit_text(Some("preview"), Some(3)).unwrap();
        let preview = editor.rich_document().snapshot();
        let resets = Rc::new(Cell::new(0));
        let count = resets.clone();
        let subscription = editor
            .text_input_method_client()
            .reset_requested(Rc::new(move || {
                count.set(count.get() + 1);
            }));
        flyout.show_at(&editor);
        assert!(popup_child.is_focused());
        assert!(editor.editor().is_composing());
        assert_eq!(editor.rich_document().snapshot(), preview);
        assert_eq!(resets.get(), 0);

        outside.focus();
        assert!(outside.is_focused());
        assert!(!editor.classes().contains(":flyout-focus"));
        assert!(!editor.editor().is_composing());
        assert_eq!(editor.text(), "editable text");
        assert_eq!(resets.get(), 1);
        flyout.hide();
        assert_eq!(
            resets.get(),
            1,
            "closing must not repeat completed blur cleanup"
        );
        subscription.dispose();
        window.close();
    }
}

#[test]
fn rich_edit_context_flyout_focus_checks_actual_target_and_redirected_destination() {
    use crate::flyouts::FlyoutBaseImplExt;
    use ferroui_base::input::{FocusChangingEventArgs, InputElement};
    use ferroui_base::media::text_formatting::testing::TextTestScope;

    for overlay in [false, true] {
        let _app = rich_edit_flyout_services(overlay);
        let _text = TextTestScope::new();
        let (window, editor, flyout, popup_child, outside) = rich_edit_flyout_fixture();
        editor.set_preedit_text(Some("preview"), None).unwrap();
        // A shared flyout can be open for a different target. Its existence
        // alone must not preserve this editor's focus state.
        flyout.show_at(&outside);
        assert!(popup_child.is_focused());
        assert!(!editor.classes().contains(":flyout-focus"));
        assert!(!editor.editor().is_composing());
        flyout.hide();
        editor.focus();
        editor.set_preedit_text(Some("preview"), None).unwrap();
        let target = outside.clone().upcast::<InputElement>();
        let token = editor.add_handler(
            InputElement::losing_focus_event(),
            move |_, event: &FocusChangingEventArgs| {
                assert!(event.try_set_new_focused_element(Some(&target)));
            },
        );
        flyout.show_at(&editor);
        assert!(outside.is_focused());
        assert!(!editor.classes().contains(":flyout-focus"));
        assert!(!editor.editor().is_composing());
        editor.remove_handler(InputElement::losing_focus_event(), token);
        flyout.hide();
        window.close();
    }
}

#[test]
fn rich_edit_canceled_flyout_focus_keeps_editor_state_and_replacement_ends_retention() {
    use crate::flyouts::FlyoutBaseImplExt;
    use ferroui_base::input::{FocusChangingEventArgs, InputElement};
    use ferroui_base::media::text_formatting::testing::TextTestScope;

    for overlay in [false, true] {
        let _app = rich_edit_flyout_services(overlay);
        let _text = TextTestScope::new();
        let (window, editor, flyout, popup_child, _) = rich_edit_flyout_fixture();
        editor.set_preedit_text(Some("preview"), None).unwrap();
        let token = editor.add_handler(
            InputElement::losing_focus_event(),
            |_, event: &FocusChangingEventArgs| {
                assert!(event.try_cancel());
            },
        );
        flyout.show_at(&editor);
        assert!(editor.is_focused());
        assert!(!editor.classes().contains(":flyout-focus"));
        assert!(editor.editor().is_composing());
        editor.remove_handler(InputElement::losing_focus_event(), token);
        flyout.hide();

        flyout.show_at(&editor);
        assert!(popup_child.is_focused());
        assert!(editor.classes().contains(":flyout-focus"));
        editor.set_context_flyout(None);
        assert!(!editor.classes().contains(":flyout-focus"));
        assert!(!editor.editor().is_composing());
        assert_eq!(editor.text(), "editable text");
        flyout.hide();
        window.close();
    }
}
