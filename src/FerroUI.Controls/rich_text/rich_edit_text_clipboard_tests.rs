//! Public TOM clipboard regressions with genuinely pending platform/payload futures.
use crate::documents::{
    CharacterFormat, DocumentSnapshot, InlineSnapshot, InlineSnapshotKind, ParagraphFormat,
    ParagraphSnapshot, RichTextDocument,
};
use crate::rich_text::{
    EditorSelection, RichEditTextDocument, RichTextPasteFormat, RichTextTransfer,
};
use ferroui_base::input::platform::{ClipboardError, ClipboardErrorKind, IClipboard};
use ferroui_base::input::{
    DataFormat, DataTransfer, DataTransferItem, IAsyncDataTransfer, IAsyncDataTransferItem,
    LocalBoxFuture,
};
use ferroui_base::media::{FontStyle, FontWeight};
use ferroui_base::reactive::IDisposable;
use ferroui_base::threading::Dispatcher;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

#[derive(Default)]
struct Gate {
    released: Cell<bool>,
    waker: RefCell<Option<Waker>>,
}

impl Gate {
    fn wait(self: &Rc<Self>) -> LocalBoxFuture<()> {
        let gate = self.clone();
        Box::pin(std::future::poll_fn(move |context| {
            if gate.released.get() {
                Poll::Ready(())
            } else {
                *gate.waker.borrow_mut() = Some(context.waker().clone());
                Poll::Pending
            }
        }))
    }

    fn release(&self) {
        self.released.set(true);
        if let Some(waker) = self.waker.borrow_mut().take() {
            waker.wake();
        }
    }
}

fn ready<T>(future: impl Future<Output = T>) -> T {
    match Box::pin(future)
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("The released clipboard operation must complete."),
    }
}

fn pending<T>(future: &mut LocalBoxFuture<T>) {
    assert!(
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
}

struct ObservedItem {
    inner: Rc<dyn IAsyncDataTransferItem>,
    reads: Rc<Cell<usize>>,
    gate: Option<Rc<Gate>>,
    error: Option<ClipboardError>,
}

impl IAsyncDataTransferItem for ObservedItem {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.inner.formats()
    }

    fn try_get_raw_async(
        &self,
        format: &DataFormat,
    ) -> LocalBoxFuture<Result<Option<Rc<dyn Any>>, ClipboardError>> {
        self.reads.set(self.reads.get() + 1);
        let operation = self.inner.try_get_raw_async(format);
        let gate = self.gate.clone();
        let error = self.error.clone();
        Box::pin(async move {
            if let Some(gate) = gate {
                gate.wait().await;
            }
            if let Some(error) = error {
                return Err(error);
            }
            operation.await
        })
    }
}

struct ObservedTransfer {
    formats: Rc<[DataFormat]>,
    items: Rc<[Rc<dyn IAsyncDataTransferItem>]>,
    reads: Rc<Cell<usize>>,
    disposals: Cell<usize>,
    on_dispose: RefCell<Option<Box<dyn FnOnce()>>>,
    formats_error: RefCell<Option<ClipboardError>>,
}

impl ObservedTransfer {
    fn new(
        data: Rc<dyn IAsyncDataTransfer>,
        gate: Option<Rc<Gate>>,
        error: Option<ClipboardError>,
    ) -> Rc<Self> {
        let reads = Rc::new(Cell::new(0));
        let items = data
            .items()
            .iter()
            .map(|item| {
                Rc::new(ObservedItem {
                    inner: item.clone(),
                    reads: reads.clone(),
                    gate: gate.clone(),
                    error: error.clone(),
                }) as Rc<dyn IAsyncDataTransferItem>
            })
            .collect::<Vec<_>>();
        Rc::new(Self {
            formats: data.formats(),
            items: Rc::from(items),
            reads,
            disposals: Cell::new(0),
            on_dispose: RefCell::new(None),
            formats_error: RefCell::new(None),
        })
    }
}

impl IDisposable for ObservedTransfer {
    fn dispose(&self) {
        self.disposals.set(self.disposals.get() + 1);
        let callback = self.on_dispose.borrow_mut().take();
        if let Some(callback) = callback {
            callback();
        }
    }
}

impl IAsyncDataTransfer for ObservedTransfer {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.formats.clone()
    }
    fn items(&self) -> Rc<[Rc<dyn IAsyncDataTransferItem>]> {
        self.items.clone()
    }
    fn try_formats(&self) -> Result<Rc<[DataFormat]>, ClipboardError> {
        match self.formats_error.borrow().clone() {
            Some(error) => Err(error),
            None => Ok(self.formats()),
        }
    }
}

#[derive(Default)]
struct TestClipboard {
    incoming: RefCell<Option<Rc<dyn IAsyncDataTransfer>>>,
    outgoing: RefCell<Option<Rc<dyn IAsyncDataTransfer>>>,
    read_gate: RefCell<Option<Rc<Gate>>>,
    write_gate: RefCell<Option<Rc<Gate>>>,
    read_error: RefCell<Option<ClipboardError>>,
    write_error: RefCell<Option<ClipboardError>>,
    reads: Cell<usize>,
    writes: Cell<usize>,
}

impl TestClipboard {
    fn receiving(data: Rc<dyn IAsyncDataTransfer>) -> Rc<Self> {
        Rc::new(Self {
            incoming: RefCell::new(Some(data)),
            ..Default::default()
        })
    }

    fn sent(&self) -> DocumentSnapshot {
        ready(RichTextTransfer::read_with_format(
            self.outgoing.borrow().as_ref().unwrap().as_ref(),
            RichTextPasteFormat::BestAvailable,
        ))
        .unwrap()
        .unwrap()
    }
}

impl IClipboard for TestClipboard {
    fn clear_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        let old = self.outgoing.borrow_mut().take();
        if let Some(old) = old {
            old.dispose();
        }
        Box::pin(std::future::ready(Ok(())))
    }
    fn set_data_async(
        &self,
        data: Option<Rc<dyn IAsyncDataTransfer>>,
    ) -> LocalBoxFuture<Result<(), ClipboardError>> {
        self.writes.set(self.writes.get() + 1);
        *self.outgoing.borrow_mut() = data;
        let gate = self.write_gate.borrow().clone();
        let error = self.write_error.borrow().clone();
        Box::pin(async move {
            if let Some(gate) = gate {
                gate.wait().await;
            }
            error.map_or(Ok(()), Err)
        })
    }
    fn flush_async(&self) -> LocalBoxFuture<Result<(), ClipboardError>> {
        Box::pin(std::future::ready(Ok(())))
    }
    fn try_get_data_async(
        &self,
    ) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
        self.reads.set(self.reads.get() + 1);
        let data = self.incoming.borrow().clone();
        let gate = self.read_gate.borrow().clone();
        let error = self.read_error.borrow().clone();
        Box::pin(async move {
            if let Some(gate) = gate {
                gate.wait().await;
            }
            error.map_or(Ok(data), Err)
        })
    }
    fn try_get_in_process_data_async(
        &self,
    ) -> LocalBoxFuture<Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError>> {
        Box::pin(std::future::ready(Ok(None)))
    }
}

fn services(text: &str) -> RichEditTextDocument {
    RichEditTextDocument::new(RichTextDocument::from_text(text))
}

fn item_transfer(item: Rc<DataTransferItem>) -> Rc<dyn IAsyncDataTransfer> {
    let transfer = DataTransfer::new();
    transfer.add(item);
    transfer
}

fn text_transfer(text: &str) -> Rc<dyn IAsyncDataTransfer> {
    let item = DataTransferItem::new();
    item.set(&DataFormat::text(), Some(text.to_owned()));
    item_transfer(item)
}

fn text_clipboard(text: &str) -> (Rc<TestClipboard>, Rc<ObservedTransfer>) {
    let transfer = ObservedTransfer::new(text_transfer(text), None, None);
    (TestClipboard::receiving(transfer.clone()), transfer)
}

#[test]
fn rich_tom_document_copy_permission_is_independent_of_selection_and_readonly() {
    let _scope = Dispatcher::unit_test_scope();
    for text in ["", "content"] {
        let document = services(text);
        document.editor().set_is_read_only(true);
        assert!(document.can_copy());
        let clipboard = Rc::new(TestClipboard::default());
        ready(
            document
                .get_range(0, 0)
                .unwrap()
                .copy_to_clipboard_async(clipboard.clone()),
        )
        .unwrap();
        assert_eq!(clipboard.writes.get(), 0);
        if !text.is_empty() {
            ready(
                document
                    .document_range()
                    .unwrap()
                    .copy_to_clipboard_async(clipboard.clone()),
            )
            .unwrap();
            assert_eq!(clipboard.sent().text(), text);
        }
        assert!(!document.can_undo());
    }
}

#[test]
fn rich_tom_copy_captures_rich_range_without_selection_or_history_changes() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("a😀bc");
    document
        .get_range(1, 3)
        .unwrap()
        .set_character_format(&CharacterFormat {
            font_weight: Some(FontWeight::Bold),
            ..Default::default()
        })
        .unwrap();
    document.selection().set_range(5, 4).unwrap();
    document.clear_undo_redo_history();
    let range = document.get_range(1, 3).unwrap();
    let expected = range.snapshot().unwrap();
    let selection = document.selection().selection();
    let revision = document.document().revision();
    let clipboard = Rc::new(TestClipboard::default());
    let copy = range.copy_to_clipboard_async(clipboard.clone());
    range.set_range(0, 1).unwrap();
    ready(copy).unwrap();
    assert_eq!(clipboard.sent(), expected);
    assert_eq!(document.selection().selection(), selection);
    assert_eq!(document.document().revision(), revision);
    assert!(!document.can_undo());
    let outgoing = clipboard.outgoing.borrow();
    let data = outgoing.as_ref().unwrap();
    assert!(RichTextTransfer::can_read(data.as_ref(), RichTextPasteFormat::Rtf).unwrap());
    assert_eq!(
        ready(RichTextTransfer::read_with_format(
            data.as_ref(),
            RichTextPasteFormat::PlainText
        ))
        .unwrap()
        .unwrap()
        .text(),
        "😀"
    );
}

#[test]
fn rich_tom_cut_rebases_reversed_independent_selection_and_is_one_undo_unit() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("a😀bc");
    document.selection().set_range(5, 4).unwrap();
    let before = document.document().snapshot();
    let range = document.get_range(1, 3).unwrap();
    let clipboard = Rc::new(TestClipboard::default());
    ready(range.cut_to_clipboard_async(clipboard.clone())).unwrap();
    assert_eq!(clipboard.sent().text(), "😀");
    assert_eq!(document.text(), "abc");
    assert_eq!(range.range(), 1..1);
    assert_eq!(document.selection().selection(), EditorSelection::new(3, 2));
    assert!(document.undo().unwrap());
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(document.selection().selection(), EditorSelection::new(5, 4));
    assert!(!document.can_undo());
    assert!(document.redo().unwrap());
    assert_eq!(document.text(), "abc");
}

#[test]
fn rich_tom_cut_failure_does_not_delete_content_or_clear_redo() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("original");
    document.set_text("changed").unwrap();
    assert!(document.undo().unwrap());
    document.selection().set_range(8, 7).unwrap();
    let range = document.get_range(1, 4).unwrap();
    let before = document.document().snapshot();
    let revision = document.document().revision();
    let clipboard = Rc::new(TestClipboard::default());
    *clipboard.write_error.borrow_mut() = Some(ClipboardError::timeout("busy"));
    let error = ready(range.cut_to_clipboard_async(clipboard.clone())).unwrap_err();
    assert_eq!(error.kind(), ClipboardErrorKind::Timeout);
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(document.document().revision(), revision);
    assert_eq!(document.selection().selection(), EditorSelection::new(8, 7));
    assert_eq!(range.range(), 1..4);
    assert!(document.can_redo());
    assert_eq!(clipboard.sent().text(), "rig");
}

#[test]
fn rich_tom_can_paste_probes_formats_without_reading_payload_or_ending_preview() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc");
    document.selection().set_range(1, 1).unwrap();
    document.editor().set_preedit("preview", Some(0)).unwrap();
    let before = document.document().snapshot();
    let item = DataTransferItem::new();
    item.set(
        &RichTextTransfer::rtf_formats()[0],
        Some(Rc::<[u8]>::from(&b"invalid RTF"[..])),
    );
    let data = ObservedTransfer::new(
        item_transfer(item),
        None,
        Some(ClipboardError::other("must not read")),
    );
    let clipboard = TestClipboard::receiving(data.clone());
    assert!(ready(document.can_paste_from_clipboard_async(clipboard.clone())).unwrap());
    assert_eq!(data.reads.get(), 0);
    assert_eq!(data.disposals.get(), 1);
    assert!(
        ready(
            document
                .get_range(0, 1)
                .unwrap()
                .can_paste_from_clipboard_async(clipboard, RichTextPasteFormat::Rtf)
        )
        .unwrap()
    );
    assert_eq!(data.reads.get(), 0);
    assert_eq!(data.disposals.get(), 2);
    assert_eq!(document.document().snapshot(), before);
    assert!(document.editor().is_composing());
}

#[test]
fn rich_tom_can_paste_readonly_and_unknown_formats_are_false_without_mutation() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc");
    let (clipboard, data) = text_clipboard("text");
    document.editor().set_is_read_only(true);
    assert!(!ready(document.can_paste_from_clipboard_async(clipboard.clone())).unwrap());
    assert_eq!(clipboard.reads.get(), 0);
    document.editor().set_is_read_only(false);
    let range = document.get_range(0, 1).unwrap();
    assert!(
        !ready(range.can_paste_from_clipboard_async(clipboard.clone(), RichTextPasteFormat::Rtf))
            .unwrap()
    );
    assert!(
        ready(range.can_paste_from_clipboard_async(clipboard, RichTextPasteFormat::PlainText))
            .unwrap()
    );
    assert_eq!(data.reads.get(), 0);
    assert_eq!(data.disposals.get(), 2);
    let item = DataTransferItem::new();
    item.set(
        &DataFormat::create_bytes_platform_format("unrecognized-format"),
        Some(Rc::<[u8]>::from(&b"x"[..])),
    );
    assert!(
        !ready(
            document.can_paste_from_clipboard_async(TestClipboard::receiving(item_transfer(item)))
        )
        .unwrap()
    );
    assert_eq!(document.text(), "abc");
    assert!(!document.can_undo());
}

#[test]
fn rich_tom_paste_format_policy_chooses_exact_rtf_plain_or_best_content() {
    let _scope = Dispatcher::unit_test_scope();
    for (format, expected) in [
        (RichTextPasteFormat::BestAvailable, "snapshot"),
        (RichTextPasteFormat::PlainText, "plain"),
        (RichTextPasteFormat::Rtf, "rich"),
    ] {
        let item = DataTransferItem::new();
        item.set(
            &RichTextTransfer::document_format(),
            Some(DocumentSnapshot::from_text("snapshot")),
        );
        item.set(&DataFormat::text(), Some("plain".to_owned()));
        item.set(
            &RichTextTransfer::rtf_formats()[0],
            Some(Rc::<[u8]>::from(&b"{\\rtf1\\ansi rich}"[..])),
        );
        let data = ObservedTransfer::new(item_transfer(item), None, None);
        let document = services("ab");
        document.selection().set_range(2, 0).unwrap();
        let range = document.get_range(1, 1).unwrap();
        ready(range.paste_from_clipboard_async(TestClipboard::receiving(data.clone()), format))
            .unwrap();
        assert_eq!(document.text(), format!("a{expected}b"));
        assert_eq!(range.range(), 1..1 + expected.len());
        assert_eq!(
            document.selection().selection(),
            EditorSelection::new(2 + expected.len(), 0)
        );
        assert_eq!(data.disposals.get(), 1);
        assert!(document.undo().unwrap());
        assert_eq!(document.text(), "ab");
        assert!(!document.can_undo());
    }
}

#[test]
fn rich_tom_explicit_rtf_never_falls_back_and_invalid_rtf_keeps_preview() {
    let _scope = Dispatcher::unit_test_scope();
    for rtf in [None, Some("{\\rtf1\\ansi\\b broken")] {
        let document = services("abc");
        document.selection().set_range(1, 1).unwrap();
        document.editor().set_preedit("😀", Some(2)).unwrap();
        let range = document.get_range(0, 1).unwrap();
        let before = document.document().snapshot();
        let revision = document.document().revision();
        let item = DataTransferItem::new();
        item.set(&DataFormat::text(), Some("fallback".to_owned()));
        if let Some(rtf) = rtf {
            item.set(
                &RichTextTransfer::rtf_formats()[0],
                Some(Rc::<[u8]>::from(rtf.as_bytes())),
            );
        }
        let data = ObservedTransfer::new(item_transfer(item), None, None);
        let result = ready(range.paste_from_clipboard_async(
            TestClipboard::receiving(data.clone()),
            RichTextPasteFormat::Rtf,
        ));
        assert_eq!(result.is_err(), rtf.is_some());
        assert_eq!(document.document().snapshot(), before);
        assert_eq!(document.document().revision(), revision);
        assert!(document.editor().is_composing());
        assert_eq!(data.disposals.get(), 1);
    }
}

#[test]
fn rich_tom_rtf_aliases_use_destination_tabs_and_preserve_explicit_stops() {
    let _scope = Dispatcher::unit_test_scope();
    for format in RichTextTransfer::rtf_formats() {
        let document = services("ab");
        document.set_default_tab_stop(54.0).unwrap();
        document.clear_undo_redo_history();
        let item = DataTransferItem::new();
        item.set(
            &format,
            Some(Rc::<[u8]>::from(
                &b"{\\rtf1\\ansi\\deftab120 one\\par\\tx240 two}"[..],
            )),
        );
        ready(
            document
                .get_range(1, 1)
                .unwrap()
                .paste_from_clipboard_async(
                    TestClipboard::receiving(item_transfer(item)),
                    RichTextPasteFormat::Rtf,
                ),
        )
        .unwrap();
        assert_eq!(document.text(), "aone\ntwob");
        let snapshot = document.document().snapshot();
        let second = &snapshot.paragraphs[1].format;
        assert_eq!(second.default_tab_stop, Some(72.0));
        assert_eq!(
            second.tab_stops.as_ref().unwrap().as_slice()[0].position,
            16.0
        );
    }
}

#[test]
fn rich_tom_plain_paste_uses_range_insertion_format_and_preserves_editor_typing_format() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("ab");
    document.selection().set_range(2, 2).unwrap();
    document
        .selection()
        .set_character_format(&CharacterFormat {
            font_weight: Some(FontWeight::Bold),
            ..Default::default()
        })
        .unwrap();
    let range = document.get_range(1, 1).unwrap();
    range
        .set_character_format(&CharacterFormat {
            font_style: Some(FontStyle::Italic),
            ..Default::default()
        })
        .unwrap();
    let editor_format = document.editor().typing_format();
    document.clear_undo_redo_history();
    ready(range.paste_from_clipboard_async(text_clipboard("😀").0, RichTextPasteFormat::PlainText))
        .unwrap();
    assert_eq!(document.text(), "a😀b");
    assert_eq!(
        document
            .document()
            .character_format_at(3)
            .unwrap()
            .font_style,
        Some(FontStyle::Italic)
    );
    assert_ne!(
        document
            .document()
            .character_format_at(3)
            .unwrap()
            .font_weight,
        Some(FontWeight::Bold)
    );
    assert_eq!(document.editor().typing_format(), editor_format);
    assert_eq!(document.selection().selection(), EditorSelection::caret(4));
    assert_eq!(range.range(), 1..3);
    assert!(document.undo().unwrap());
    assert_eq!(document.text(), "ab");
    assert_eq!(document.editor().typing_format(), editor_format);
    assert!(!document.can_undo());
}

#[test]
fn rich_tom_rich_paste_retains_nested_formatting_and_snapshot_undo() {
    let _scope = Dispatcher::unit_test_scope();
    let snapshot = DocumentSnapshot {
        paragraphs: vec![
            ParagraphSnapshot {
                format: ParagraphFormat::default(),
                inlines: vec![InlineSnapshot {
                    format: CharacterFormat {
                        font_weight: Some(FontWeight::Bold),
                        ..Default::default()
                    },
                    kind: InlineSnapshotKind::Span(vec![InlineSnapshot {
                        format: CharacterFormat {
                            font_style: Some(FontStyle::Italic),
                            ..Default::default()
                        },
                        kind: InlineSnapshotKind::Run("😀".to_owned()),
                    }]),
                }],
            },
            ParagraphSnapshot {
                format: ParagraphFormat {
                    left_indent: Some(24.0),
                    ..Default::default()
                },
                inlines: vec![InlineSnapshot::run("tail")],
            },
        ],
    };
    let item = DataTransferItem::new();
    item.set(&RichTextTransfer::document_format(), Some(snapshot.clone()));
    let document = services("ab");
    document.selection().set_range(2, 2).unwrap();
    let range = document.get_range(1, 1).unwrap();
    ready(range.paste_from_clipboard_async(
        TestClipboard::receiving(item_transfer(item)),
        RichTextPasteFormat::BestAvailable,
    ))
    .unwrap();
    assert_eq!(document.text(), "a😀\ntailb");
    let inserted = document.document().character_format_at(3).unwrap();
    assert_eq!(inserted.font_weight, Some(FontWeight::Bold));
    assert_eq!(inserted.font_style, Some(FontStyle::Italic));
    assert_eq!(document.document().paragraphs()[1].left_indent(), 24.0);
    let after = document.document().snapshot();
    assert_eq!(document.selection().selection(), EditorSelection::caret(9));
    assert!(document.undo().unwrap());
    assert_eq!(document.text(), "ab");
    assert!(document.redo().unwrap());
    assert_eq!(document.document().snapshot(), after);
}

#[test]
fn rich_tom_paste_truncates_at_utf16_boundaries_and_rejection_keeps_selection() {
    let _scope = Dispatcher::unit_test_scope();
    for rich in [false, true] {
        let document = services("abc");
        let range = document.get_range(1, 2).unwrap();
        document.selection().set_range(3, 0).unwrap();
        document.editor().set_max_length(3);
        let item = DataTransferItem::new();
        if rich {
            item.set(
                &RichTextTransfer::document_format(),
                Some(DocumentSnapshot::from_text("😀x")),
            );
        } else {
            item.set(&DataFormat::text(), Some("😀x".to_owned()));
        }
        let clipboard = TestClipboard::receiving(item_transfer(item));
        let revision = document.document().revision();
        ready(
            range.paste_from_clipboard_async(clipboard.clone(), RichTextPasteFormat::BestAvailable),
        )
        .unwrap();
        assert_eq!(document.text(), "abc");
        assert_eq!(range.range(), 1..2);
        assert_eq!(document.selection().selection(), EditorSelection::new(3, 0));
        assert_eq!(document.document().revision(), revision);
        assert!(!document.can_undo());
        document.editor().set_max_length(4);
        ready(range.paste_from_clipboard_async(clipboard, RichTextPasteFormat::BestAvailable))
            .unwrap();
        assert_eq!(document.text(), "a😀c");
        assert_eq!(range.range(), 1..3);
        assert_eq!(document.selection().selection(), EditorSelection::new(4, 0));
        assert!(document.undo().unwrap());
        assert_eq!(document.text(), "abc");
        assert!(!document.can_undo());
    }
}

#[test]
fn rich_tom_empty_paste_and_degenerate_cut_preserve_preview_and_redo() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc");
    document.set_text("changed").unwrap();
    document.undo().unwrap();
    document.selection().set_range(1, 1).unwrap();
    document.editor().set_preedit("preview", Some(0)).unwrap();
    let before = document.document().snapshot();
    let range = document.get_range(0, 1).unwrap();
    let revision = document.document().revision();
    ready(range.paste_from_clipboard_async(text_clipboard("").0, RichTextPasteFormat::PlainText))
        .unwrap();
    let clipboard = Rc::new(TestClipboard::default());
    ready(
        document
            .get_range(0, 0)
            .unwrap()
            .cut_to_clipboard_async(clipboard.clone()),
    )
    .unwrap();
    assert_eq!(clipboard.writes.get(), 0);
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(document.document().revision(), revision);
    assert!(document.editor().is_composing());
    document.editor().cancel_composition().unwrap();
    assert!(document.can_redo());
}

#[test]
fn rich_tom_cut_ends_preview_before_capturing_committed_clipboard_content() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc");
    document.selection().set_range(1, 2).unwrap();
    let range = document.get_range(0, 3).unwrap();
    document.editor().set_preedit("preview", Some(0)).unwrap();
    let clipboard = Rc::new(TestClipboard::default());
    ready(range.cut_to_clipboard_async(clipboard.clone())).unwrap();
    assert!(!document.editor().is_composing());
    assert_eq!(clipboard.sent().text(), "abc");
    assert_eq!(document.text(), "");
    assert!(document.undo().unwrap());
    assert_eq!(document.text(), "abc");
    assert!(!document.can_undo());
}

#[test]
fn rich_tom_paste_cancels_original_preview_then_reads_live_committed_endpoints() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc");
    document.selection().set_range(1, 2).unwrap();
    let range = document.get_range(2, 3).unwrap();
    document
        .editor()
        .set_preedit("long preview", Some(0))
        .unwrap();
    assert!(range.start_position() > 2);
    ready(range.paste_from_clipboard_async(text_clipboard("😀").0, RichTextPasteFormat::PlainText))
        .unwrap();
    assert_eq!(document.text(), "ab😀");
    assert_eq!(range.range(), 2..4);
    assert!(!document.editor().is_composing());
    assert!(document.undo().unwrap());
    assert_eq!(document.text(), "abc");
    assert!(!document.can_undo());
}

#[test]
fn rich_tom_pending_cut_sees_model_edits_inside_a_held_document_batch() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("original");
    let range = document.get_range(0, 8).unwrap();
    let clipboard = Rc::new(TestClipboard::default());
    let gate = Rc::new(Gate::default());
    *clipboard.write_gate.borrow_mut() = Some(gate.clone());
    let mut cut = range.cut_to_clipboard_async(clipboard.clone());
    pending(&mut cut);
    let model = document.document();
    let revision = model.revision();
    let batch = model.begin_edit();
    model.replace_text(0..8, "replaced").unwrap();
    let expected = model.snapshot();
    assert_eq!(model.revision(), revision);
    gate.release();
    assert_eq!(ready(cut).unwrap_err().kind(), ClipboardErrorKind::Canceled);
    assert_eq!(clipboard.sent().text(), "original");
    assert_eq!(model.snapshot(), expected);
    assert!(!document.can_undo());
    drop(batch);
}

#[test]
fn rich_tom_pending_paste_sees_model_edits_inside_a_held_document_batch() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("original");
    let range = document.get_range(0, 8).unwrap();
    let (clipboard, data) = text_clipboard("clipboard");
    let gate = Rc::new(Gate::default());
    *clipboard.read_gate.borrow_mut() = Some(gate.clone());
    let mut paste = range.paste_from_clipboard_async(clipboard, RichTextPasteFormat::PlainText);
    pending(&mut paste);
    let model = document.document();
    let revision = model.revision();
    let batch = model.begin_edit();
    model.replace_text(0..8, "replaced").unwrap();
    let expected = model.snapshot();
    assert_eq!(model.revision(), revision);
    gate.release();
    assert_eq!(
        ready(paste).unwrap_err().kind(),
        ClipboardErrorKind::Canceled
    );
    assert_eq!(model.snapshot(), expected);
    assert_eq!(data.disposals.get(), 1);
    assert!(!document.can_undo());
    drop(batch);
}

#[test]
fn rich_tom_pending_clipboard_edit_rejects_shared_range_moved_away_and_back() {
    let _scope = Dispatcher::unit_test_scope();
    for cut in [false, true] {
        let document = services("abcd");
        let range = document.get_range(1, 3).unwrap();
        let shared = range.clone();
        let (clipboard, _) = text_clipboard("paste");
        let gate = Rc::new(Gate::default());
        *clipboard.read_gate.borrow_mut() = Some(gate.clone());
        *clipboard.write_gate.borrow_mut() = Some(gate.clone());
        let mut operation = if cut {
            range.cut_to_clipboard_async(clipboard)
        } else {
            range.paste_from_clipboard_async(clipboard, RichTextPasteFormat::PlainText)
        };
        pending(&mut operation);
        shared.set_range(0, 1).unwrap();
        shared.set_range(1, 3).unwrap();
        let revision = document.document().revision();
        gate.release();
        assert_eq!(
            ready(operation).unwrap_err().kind(),
            ClipboardErrorKind::Canceled
        );
        assert_eq!(document.text(), "abcd");
        assert_eq!(document.document().revision(), revision);
        assert_eq!(range.range(), 1..3);
        assert!(!document.can_undo());
    }
}

#[test]
fn rich_tom_pending_paste_rejects_insertion_format_changed_away_and_back() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("ab");
    let range = document.get_range(1, 1).unwrap();
    let set_weight = |weight| {
        range
            .set_character_format(&CharacterFormat {
                font_weight: Some(weight),
                ..Default::default()
            })
            .unwrap()
    };
    set_weight(FontWeight::Normal);
    let (clipboard, data) = text_clipboard("paste");
    let gate = Rc::new(Gate::default());
    *clipboard.read_gate.borrow_mut() = Some(gate.clone());
    let mut paste = range.paste_from_clipboard_async(clipboard, RichTextPasteFormat::PlainText);
    pending(&mut paste);
    let revision = document.document().revision();
    set_weight(FontWeight::Bold);
    set_weight(FontWeight::Normal);
    gate.release();
    assert_eq!(
        ready(paste).unwrap_err().kind(),
        ClipboardErrorKind::Canceled
    );
    assert_eq!(document.text(), "ab");
    assert_eq!(document.document().revision(), revision);
    assert_eq!(data.disposals.get(), 1);
    assert!(!document.can_undo());
}

#[test]
fn rich_tom_pending_paste_is_not_invalidated_by_an_independent_clone() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abcd");
    let range = document.get_range(1, 3).unwrap();
    let independent = range.get_clone().unwrap();
    let (clipboard, _) = text_clipboard("X");
    let gate = Rc::new(Gate::default());
    *clipboard.read_gate.borrow_mut() = Some(gate.clone());
    let mut paste = range.paste_from_clipboard_async(clipboard, RichTextPasteFormat::PlainText);
    pending(&mut paste);
    independent.set_range(0, 0).unwrap();
    gate.release();
    ready(paste).unwrap();
    assert_eq!(document.text(), "aXd");
    assert_eq!(range.range(), 1..2);
    assert_eq!(independent.range(), 0..0);
}

#[test]
fn rich_tom_pending_paste_rejects_selection_roundtrip_without_document_mutation() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abcd");
    let (clipboard, _) = text_clipboard("X");
    let gate = Rc::new(Gate::default());
    *clipboard.read_gate.borrow_mut() = Some(gate.clone());
    let mut paste = document
        .get_range(1, 3)
        .unwrap()
        .paste_from_clipboard_async(clipboard, RichTextPasteFormat::PlainText);
    pending(&mut paste);
    let revision = document.document().revision();
    document.selection().set_range(4, 4).unwrap();
    document.selection().set_range(0, 0).unwrap();
    gate.release();
    assert_eq!(
        ready(paste).unwrap_err().kind(),
        ClipboardErrorKind::Canceled
    );
    assert_eq!(document.text(), "abcd");
    assert_eq!(document.document().revision(), revision);
    assert_eq!(document.selection().selection(), EditorSelection::caret(0));
}

#[test]
fn rich_tom_pending_paste_rejects_an_empty_composition_roundtrip() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abcd");
    let (clipboard, data) = text_clipboard("X");
    let gate = Rc::new(Gate::default());
    *clipboard.read_gate.borrow_mut() = Some(gate.clone());
    let mut paste = document
        .get_range(1, 3)
        .unwrap()
        .paste_from_clipboard_async(clipboard, RichTextPasteFormat::PlainText);
    pending(&mut paste);
    let revision = document.document().revision();
    document.editor().set_preedit("", Some(0)).unwrap();
    assert!(document.editor().is_composing());
    document.editor().cancel_composition().unwrap();
    assert!(!document.editor().is_composing());
    assert_eq!(document.document().revision(), revision);
    gate.release();
    assert_eq!(
        ready(paste).unwrap_err().kind(),
        ClipboardErrorKind::Canceled
    );
    assert_eq!(document.text(), "abcd");
    assert_eq!(data.disposals.get(), 1);
    assert!(!document.can_undo());
}

#[test]
fn rich_tom_pending_paste_rechecks_readonly_before_applying_data() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abcd");
    let (clipboard, data) = text_clipboard("X");
    let gate = Rc::new(Gate::default());
    *clipboard.read_gate.borrow_mut() = Some(gate.clone());
    let range = document.get_range(1, 3).unwrap();
    let mut paste = range.paste_from_clipboard_async(clipboard, RichTextPasteFormat::PlainText);
    pending(&mut paste);
    document.editor().set_is_read_only(true);
    gate.release();
    assert_eq!(
        ready(paste).unwrap_err().kind(),
        ClipboardErrorKind::AccessDenied
    );
    assert_eq!(document.text(), "abcd");
    assert_eq!(range.range(), 1..3);
    assert_eq!(data.disposals.get(), 1);
    assert!(!document.can_undo());
}

#[test]
fn rich_tom_dropping_pending_payload_disposes_retrieved_transfer_once() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc");
    let gate = Rc::new(Gate::default());
    let data = ObservedTransfer::new(text_transfer("paste"), Some(gate), None);
    let clipboard = TestClipboard::receiving(data.clone());
    let mut paste = document
        .get_range(1, 2)
        .unwrap()
        .paste_from_clipboard_async(clipboard, RichTextPasteFormat::PlainText);
    pending(&mut paste);
    assert_eq!(data.reads.get(), 1);
    assert_eq!(data.disposals.get(), 0);
    drop(paste);
    assert_eq!(data.disposals.get(), 1);
    assert_eq!(document.text(), "abc");
    assert!(!document.can_undo());
}

#[test]
fn rich_tom_payload_and_format_query_errors_dispose_the_retrieved_transfer() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc");
    let data = ObservedTransfer::new(
        text_transfer("paste"),
        None,
        Some(ClipboardError::timeout("payload busy")),
    );
    let clipboard = TestClipboard::receiving(data.clone());
    let error = ready(
        document
            .get_range(0, 1)
            .unwrap()
            .paste_from_clipboard_async(clipboard, RichTextPasteFormat::PlainText),
    )
    .unwrap_err();
    assert_eq!(error.kind(), ClipboardErrorKind::Timeout);
    assert_eq!(data.disposals.get(), 1);
    let data = ObservedTransfer::new(text_transfer("paste"), None, None);
    *data.formats_error.borrow_mut() = Some(ClipboardError::access_denied("formats denied"));
    let error =
        ready(document.can_paste_from_clipboard_async(TestClipboard::receiving(data.clone())))
            .unwrap_err();
    assert_eq!(error.kind(), ClipboardErrorKind::AccessDenied);
    assert_eq!(data.disposals.get(), 1);
    assert_eq!(data.reads.get(), 0);
    assert_eq!(document.text(), "abc");
}

#[test]
fn rich_tom_disposal_callbacks_cannot_redirect_the_captured_paste() {
    let _scope = Dispatcher::unit_test_scope();
    for model_change in [false, true] {
        let document = services("abcd");
        let range = document.get_range(1, 3).unwrap();
        let (clipboard, data) = text_clipboard("paste");
        let changed_range = range.clone();
        let changed_model = document.document();
        *data.on_dispose.borrow_mut() = Some(Box::new(move || {
            if model_change {
                changed_model.replace_text(0..4, "external").unwrap();
            } else {
                changed_range.set_range(0, 1).unwrap();
                changed_range.set_range(1, 3).unwrap();
            }
        }));
        let error =
            ready(range.paste_from_clipboard_async(clipboard, RichTextPasteFormat::PlainText))
                .unwrap_err();
        assert_eq!(error.kind(), ClipboardErrorKind::Canceled);
        assert_eq!(
            document.text(),
            if model_change { "external" } else { "abcd" }
        );
        assert_eq!(data.disposals.get(), 1);
        assert!(!document.can_undo());
    }
}

#[test]
fn rich_tom_invalid_snapshot_preserves_preview_and_disposes_transfer() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abc");
    document.selection().set_range(1, 1).unwrap();
    document.editor().set_preedit("preview", Some(0)).unwrap();
    let before = document.document().snapshot();
    let mut snapshot = DocumentSnapshot::from_text("invalid");
    snapshot.paragraphs[0].format.character.font_size = Some(f64::NAN);
    let item = DataTransferItem::new();
    item.set(&RichTextTransfer::document_format(), Some(snapshot));
    let data = ObservedTransfer::new(item_transfer(item), None, None);
    assert!(
        ready(
            document
                .get_range(0, 1)
                .unwrap()
                .paste_from_clipboard_async(
                    TestClipboard::receiving(data.clone()),
                    RichTextPasteFormat::BestAvailable
                )
        )
        .is_err()
    );
    assert_eq!(document.document().snapshot(), before);
    assert!(document.editor().is_composing());
    assert_eq!(data.disposals.get(), 1);
}

#[test]
fn rich_tom_publication_keeps_handler_owned_range_and_external_document_changes() {
    let _scope = Dispatcher::unit_test_scope();
    for model_change in [false, true] {
        let document = services("abcd");
        let range = document.get_range(1, 3).unwrap();
        let changed_range = range.clone();
        let changed_model = document.document();
        let called = Rc::new(Cell::new(false));
        let callback_called = called.clone();
        let _subscription = document.editor().changed(move || {
            if !callback_called.replace(true) {
                if model_change {
                    changed_model
                        .replace_text(0..changed_model.text_length(), "external")
                        .unwrap();
                } else {
                    changed_range.set_range(0, 0).unwrap();
                }
            }
        });
        let result = ready(
            range.paste_from_clipboard_async(text_clipboard("X").0, RichTextPasteFormat::PlainText),
        );
        assert!(called.get());
        if model_change {
            assert_eq!(result.unwrap_err().kind(), ClipboardErrorKind::Canceled);
            assert_eq!(document.text(), "external");
            assert!(!document.can_undo());
        } else {
            result.unwrap();
            assert_eq!(document.text(), "aXd");
            assert_eq!(range.range(), 0..0);
            assert!(document.can_undo());
        }
    }
}

#[test]
fn rich_range_gravity_pending_clipboard_rejects_mode_changed_away_and_back() {
    use crate::rich_text::RangeGravity;
    let _scope = Dispatcher::unit_test_scope();
    for cut in [false, true] {
        let document = services("abcd");
        let range = document.get_range(1, 3).unwrap();
        let shared = range.clone();
        let (clipboard, _) = text_clipboard("paste");
        let gate = Rc::new(Gate::default());
        *clipboard.read_gate.borrow_mut() = Some(gate.clone());
        *clipboard.write_gate.borrow_mut() = Some(gate.clone());
        let mut operation = if cut {
            range.cut_to_clipboard_async(clipboard)
        } else {
            range.paste_from_clipboard_async(clipboard, RichTextPasteFormat::PlainText)
        };
        pending(&mut operation);
        shared.set_gravity(RangeGravity::Forward);
        shared.set_gravity(RangeGravity::UIBehavior);
        gate.release();
        assert_eq!(
            ready(operation).unwrap_err().kind(),
            ClipboardErrorKind::Canceled
        );
        assert_eq!(document.text(), "abcd");
        assert_eq!(range.range(), 1..3);
        assert_eq!(range.gravity(), RangeGravity::UIBehavior);
        assert!(!document.can_undo());
    }
}

#[test]
fn rich_range_gravity_pending_paste_keeps_independent_clone_and_canonical_noop_valid() {
    use crate::rich_text::RangeGravity;
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abcd");
    let range = document.get_range(1, 3).unwrap();
    let independent = range.get_clone().unwrap();
    let (clipboard, _) = text_clipboard("X");
    let gate = Rc::new(Gate::default());
    *clipboard.read_gate.borrow_mut() = Some(gate.clone());
    let mut operation = range.paste_from_clipboard_async(clipboard, RichTextPasteFormat::PlainText);
    pending(&mut operation);
    independent.set_gravity(RangeGravity::Forward);
    range.set_gravity(RangeGravity::Outward);
    assert!(range.set_gravity_value(5).is_err());
    gate.release();
    ready(operation).unwrap();
    assert_eq!(document.text(), "aXd");
    assert_eq!(range.gravity(), RangeGravity::UIBehavior);
    assert_eq!(independent.gravity(), RangeGravity::Forward);
}

#[test]
fn rich_range_gravity_plain_paste_uses_affinity_and_restores_exact_typing_patch() {
    use crate::rich_text::{RangeGravity, TextFormatValue};
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abCD");
    document
        .document()
        .apply_format(
            0..2,
            &CharacterFormat {
                font_weight: Some(FontWeight::Bold),
                ..Default::default()
            },
        )
        .unwrap();
    document
        .document()
        .apply_format(
            2..4,
            &CharacterFormat {
                font_style: Some(FontStyle::Italic),
                ..Default::default()
            },
        )
        .unwrap();
    document.selection().set_range(2, 2).unwrap();
    document
        .editor()
        .apply_format(&CharacterFormat {
            font_size: Some(30.0),
            ..Default::default()
        })
        .unwrap();
    let typing = document.editor().typing_format();
    document.clear_undo_redo_history();
    let before = document.document().snapshot();
    let range = document.get_range(2, 2).unwrap();
    range.set_gravity(RangeGravity::Forward);
    ready(range.paste_from_clipboard_async(text_clipboard("X").0, RichTextPasteFormat::PlainText))
        .unwrap();
    let TextFormatValue::Uniform(inserted) = range.character_format().unwrap() else {
        panic!()
    };
    assert_eq!(inserted.font_weight, Some(FontWeight::Normal));
    assert_eq!(inserted.font_style, Some(FontStyle::Italic));
    assert_ne!(inserted.font_size, Some(30.0));
    assert_eq!(document.editor().typing_format(), typing);
    assert_eq!(document.selection().selection(), EditorSelection::caret(3));
    assert!(document.undo().unwrap());
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(document.editor().typing_format(), typing);
    assert_eq!(range.gravity(), RangeGravity::Forward);
    assert!(!document.can_undo());
}

struct LateTypingCallbackBrush {
    ready: Box<dyn Fn() -> bool>,
    callback: RefCell<Option<Box<dyn FnOnce()>>>,
}
impl ferroui_base::media::IBrush for LateTypingCallbackBrush {
    fn opacity(&self) -> f64 {
        1.0
    }
    fn transform(&self) -> Option<Rc<dyn ferroui_base::media::ITransform>> {
        None
    }
    fn transform_origin(&self) -> ferroui_base::RelativePoint {
        ferroui_base::RelativePoint::CENTER
    }
    fn relative_transform(&self) -> Option<Rc<dyn ferroui_base::media::ITransform>> {
        None
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_solid_color_brush(&self) -> Option<&dyn ferroui_base::media::ISolidColorBrush> {
        if (self.ready)() {
            let callback = self.callback.borrow_mut().take();
            if let Some(callback) = callback {
                callback();
            }
        }
        None
    }
}

#[test]
fn rich_range_gravity_paste_late_typing_restore_rejects_newer_range_ownership() {
    use crate::rich_text::RangeGravity;
    let _scope = Dispatcher::unit_test_scope();
    let document = services("abCD");
    document.selection().set_range(4, 4).unwrap();
    let range = document.get_range(2, 2).unwrap();
    range.set_gravity(RangeGravity::Forward);
    let editor = Rc::downgrade(&document.editor());
    let brush = Rc::new(LateTypingCallbackBrush {
        ready: Box::new(move || {
            editor.upgrade().is_some_and(|editor| {
                editor.document().text() == "abXCD"
                    && editor.selection() == EditorSelection::caret(5)
            })
        }),
        callback: RefCell::new(None),
    });
    document
        .editor()
        .apply_format(&CharacterFormat {
            foreground: Some(Some(brush.clone())),
            ..Default::default()
        })
        .unwrap();
    document.clear_undo_redo_history();
    let before = document.document().snapshot();
    let typing = document.editor().typing_format();
    let changed_range = range.clone();
    let called = Rc::new(Cell::new(false));
    let observed = called.clone();
    *brush.callback.borrow_mut() = Some(Box::new(move || {
        observed.set(true);
        changed_range.set_range(0, 0).unwrap();
        changed_range.set_gravity(RangeGravity::Backward);
    }));
    let error = ready(
        range.paste_from_clipboard_async(text_clipboard("X").0, RichTextPasteFormat::PlainText),
    )
    .unwrap_err();
    assert_eq!(error.kind(), ClipboardErrorKind::Canceled);
    assert!(called.get());
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(document.editor().typing_format(), typing);
    assert_eq!(range.range(), 0..0);
    assert_eq!(range.gravity(), RangeGravity::Backward);
    assert!(!document.can_undo());
}
