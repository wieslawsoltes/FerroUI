//! Common-model hyperlink fields cross actual clipboard and editor paths.
//! External RTF preserves grouping; exact in-process payloads also preserve
//! field identities, watermark, sentinel state and target-frame metadata.

use super::{RichEditClipboardFormat, RichTextPasteFormat, RichTextTransfer};
use crate::documents::{
    CharacterFormat, DocumentLinkField, DocumentSnapshot, InlineSnapshot, InlineSnapshotKind,
    RichTextDocument,
};
use crate::rich_text::{EditorSelection, RichEditTextDocument, RtfReader, RtfWriter};
use crate::{Button, RichEditBox, RichEditCharacterCasing};
use ferroui_base::input::{
    AsyncDataTransferExtensions, DataFormat, DataTransfer, DataTransferItem, IAsyncDataTransfer,
};
use ferroui_base::media::FontWeight;
use ferroui_base::threading::Dispatcher;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::future::Future;
use std::ops::Range;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

fn ready<T>(future: impl Future<Output = T>) -> T {
    match Box::pin(future)
        .as_mut()
        .poll(&mut Context::from_waker(Waker::noop()))
    {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("The in-memory clipboard must complete synchronously."),
    }
}

fn one_field(text: &str, range: Range<usize>, id: u64) -> DocumentSnapshot {
    let mut snapshot = DocumentSnapshot::from_text(text);
    snapshot
        .add_link_field(DocumentLinkField {
            id,
            range,
            target: "https://example.test".into(),
            target_name: Some("frame".into()),
            has_sentinel: false,
        })
        .unwrap();
    snapshot
}

fn services(snapshot: &DocumentSnapshot) -> RichEditTextDocument {
    let model = RichTextDocument::new();
    model.restore(snapshot).unwrap();
    RichEditTextDocument::new(model)
}

fn item_transfer(item: Rc<DataTransferItem>) -> Rc<dyn IAsyncDataTransfer> {
    let transfer = DataTransfer::new();
    transfer.add(item);
    transfer
}

fn clipboard(snapshot: &DocumentSnapshot) -> Rc<crate::testing::TestClipboardImpl> {
    let clipboard = crate::testing::TestClipboardImpl::new();
    ready(
        clipboard
            .clipboard()
            .set_data_async(Some(RichTextTransfer::create(
                snapshot,
                RichEditClipboardFormat::AllFormats,
            ))),
    )
    .unwrap();
    clipboard
}

fn projected_ids(snapshot: &DocumentSnapshot) -> Vec<u64> {
    fn visit(inlines: &[InlineSnapshot], ids: &mut Vec<u64>) {
        for inline in inlines {
            if let InlineSnapshotKind::Hyperlink {
                field_id: Some(id), ..
            } = &inline.kind
            {
                ids.push(*id);
            }
            if let Some(children) = inline.children() {
                visit(children, ids);
            }
        }
    }
    let mut ids = Vec::new();
    for paragraph in &snapshot.paragraphs {
        visit(&paragraph.inlines, &mut ids);
    }
    ids
}

#[test]
fn rich_edit_link_transfer_keeps_exact_identity_watermark_sentinel_and_target_frame() {
    let _scope = Dispatcher::unit_test_scope();
    let mut source = one_field("a\n\nb", 0..4, 41);
    source.link_fields[0].has_sentinel = true;
    source.next_link_field_id = 99;
    let transfer = RichTextTransfer::create(&source, RichEditClipboardFormat::AllFormats);
    assert!(
        RichTextTransfer::can_read(transfer.as_ref(), RichTextPasteFormat::BestAvailable).unwrap()
    );
    assert!(!RichTextTransfer::can_read(transfer.as_ref(), RichTextPasteFormat::Rtf).unwrap());
    assert_eq!(
        ready(transfer.try_get_text_async()).unwrap().as_deref(),
        Some("a\n\nb")
    );
    let restored = ready(RichTextTransfer::read(transfer.as_ref(), false))
        .unwrap()
        .unwrap();
    assert_eq!(restored, source);
    assert_eq!(projected_ids(&restored), [41, 41]);
    let plain = ready(RichTextTransfer::read(transfer.as_ref(), true))
        .unwrap()
        .unwrap();
    assert_eq!(plain.text(), source.text());
    assert!(plain.link_fields.is_empty());
    assert!(projected_ids(&plain).is_empty());
    assert_eq!(plain.next_link_field_id, 1);
}

#[test]
fn rich_edit_link_external_rtf_retains_one_cross_paragraph_field_with_fresh_identity() {
    let _scope = Dispatcher::unit_test_scope();
    let mut source = one_field("prefix a\n\nb suffix", 7..11, 57);
    source.next_link_field_id = 100;
    let encoded = RtfWriter::write(&source).unwrap();
    assert_eq!(encoded.matches("\\field").count(), 1);
    let item = DataTransferItem::new();
    item.set(
        &RichTextTransfer::rtf_formats()[0],
        Some(Rc::<[u8]>::from(encoded.as_bytes())),
    );
    item.set(&DataFormat::text(), Some("unused fallback".into()));
    let transfer = item_transfer(item);
    for format in [RichTextPasteFormat::Rtf, RichTextPasteFormat::BestAvailable] {
        let restored = ready(RichTextTransfer::read_with_format_for_insertion(
            transfer.as_ref(),
            format,
            72.0,
        ))
        .unwrap()
        .unwrap();
        assert_eq!(restored.text(), source.text());
        assert_eq!(restored.link_fields.len(), 1);
        let field = &restored.link_fields[0];
        assert_eq!(field.id, 1);
        assert_eq!(field.range, 7..11);
        assert_eq!(field.target, source.link_fields[0].target);
        assert_eq!(field.target_name, source.link_fields[0].target_name);
        assert_eq!(restored.next_link_field_id, 2);
        assert_eq!(projected_ids(&restored), [1, 1]);
        assert!(restored
            .paragraphs
            .iter()
            .all(|paragraph| paragraph.format.default_tab_stop == Some(72.0)));
    }
}

#[test]
fn rich_edit_link_malformed_exact_payload_is_rejected_before_rtf_or_plain_fallback() {
    let _scope = Dispatcher::unit_test_scope();
    let valid = one_field("linked", 0..6, 1);
    let encoded = RtfWriter::write(&valid).unwrap();
    for invalid in 0..3 {
        let mut source = valid.clone();
        match invalid {
            0 => source.next_link_field_id = 1,
            1 => source.link_fields[0].range = 0..7,
            _ => source.link_fields[0].target = "conflicting target".into(),
        }
        let item = DataTransferItem::new();
        item.set(&RichTextTransfer::document_format(), Some(source));
        item.set(
            &RichTextTransfer::rtf_formats()[0],
            Some(Rc::<[u8]>::from(encoded.as_bytes())),
        );
        item.set(&DataFormat::text(), Some("plain fallback".into()));
        let transfer = item_transfer(item);
        assert!(ready(RichTextTransfer::read(transfer.as_ref(), false)).is_err());
        let plain = ready(RichTextTransfer::read(transfer.as_ref(), true))
            .unwrap()
            .unwrap();
        assert_eq!(plain.text(), "plain fallback");
        assert!(plain.link_fields.is_empty());
        assert_eq!(
            ready(RichTextTransfer::read_with_format(
                transfer.as_ref(),
                RichTextPasteFormat::Rtf,
            ))
            .unwrap()
            .unwrap()
            .text(),
            "linked"
        );
    }
}

#[test]
fn rich_edit_link_tom_copy_of_only_paragraph_marks_keeps_shared_field_metadata() {
    let _scope = Dispatcher::unit_test_scope();
    let source = one_field("Pa\n\nbZ", 1..5, 31);
    let document = services(&source);
    document.selection().set_range(6, 0).unwrap();
    let before = document.document().snapshot();
    let selection = document.selection().selection();
    let clipboard = crate::testing::TestClipboardImpl::new();
    ready(
        document
            .get_range(2, 4)
            .unwrap()
            .copy_to_clipboard_async(clipboard.clipboard()),
    )
    .unwrap();
    assert_eq!(clipboard.text().as_deref(), Some("\n\n"));
    let transfer = clipboard.data().unwrap();
    let fragment = ready(RichTextTransfer::read(transfer.as_ref(), false))
        .unwrap()
        .unwrap();
    assert_eq!(fragment.text(), "\n\n");
    assert_eq!(fragment.link_fields.len(), 1);
    assert_eq!(fragment.link_fields[0].id, 31);
    assert_eq!(fragment.link_fields[0].range, 0..2);
    assert_eq!(
        fragment.link_fields[0].target_name.as_deref(),
        Some("frame")
    );
    assert_eq!(fragment.next_link_field_id, 32);
    assert!(projected_ids(&fragment).is_empty());
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(document.selection().selection(), selection);
    assert!(!document.can_undo());
}

#[test]
fn rich_edit_link_tom_paste_remaps_collisions_once_and_undo_restores_exact_snapshot() {
    let _scope = Dispatcher::unit_test_scope();
    let source = one_field("B\nC", 0..3, 1);
    let document = services(&one_field("A", 0..1, 1));
    document.selection().set_range(1, 0).unwrap();
    let before = document.document().snapshot();
    let selection = document.selection().selection();
    let clipboard = clipboard(&source);
    let range = document.get_range(1, 1).unwrap();
    ready(
        range.paste_from_clipboard_async(clipboard.clipboard(), RichTextPasteFormat::BestAvailable),
    )
    .unwrap();
    assert_eq!(document.text(), "AB\nC");
    assert_eq!(range.range(), 1..4);
    let after = document.document().snapshot();
    after.validate().unwrap();
    assert_eq!(after.link_fields.len(), 2);
    let first = &after.link_fields[0];
    let inserted = &after.link_fields[1];
    assert_eq!(first.id, 1);
    assert_eq!(first.range, 0..1);
    assert_ne!(inserted.id, first.id);
    assert_eq!(inserted.range, 1..4);
    assert_eq!(inserted.target, first.target);
    assert_eq!(inserted.target_name, first.target_name);
    assert_eq!(projected_ids(&after), [first.id, inserted.id, inserted.id]);
    assert!(after.next_link_field_id > inserted.id);
    assert!(document.undo().unwrap());
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(document.selection().selection(), selection);
    assert!(!document.can_undo());
    assert!(document.redo().unwrap());
    assert_eq!(document.document().snapshot(), after);
}

#[test]
fn rich_edit_link_tom_paste_max_length_clips_field_metadata_at_scalar_boundaries() {
    let _scope = Dispatcher::unit_test_scope();
    let source = one_field("a😀\nb", 0..5, 17);
    for (limit, expected, range) in [(3, "xa", 1..2), (4, "xa😀", 1..4)] {
        let document = services(&DocumentSnapshot::from_text("x"));
        document.editor().set_max_length(limit);
        let before = document.document().snapshot();
        let clipboard = clipboard(&source);
        ready(
            document
                .get_range(1, 1)
                .unwrap()
                .paste_from_clipboard_async(
                    clipboard.clipboard(),
                    RichTextPasteFormat::BestAvailable,
                ),
        )
        .unwrap();
        assert_eq!(document.text(), expected);
        let snapshot = document.document().snapshot();
        snapshot.validate().unwrap();
        assert_eq!(snapshot.link_fields.len(), 1);
        assert_eq!(snapshot.link_fields[0].range, range);
        assert_eq!(
            snapshot.link_fields[0].target_name.as_deref(),
            Some("frame")
        );
        assert!(document.undo().unwrap());
        assert_eq!(document.document().snapshot(), before);
        assert!(!document.can_undo());
    }
}

#[test]
fn rich_edit_link_invalid_field_payload_does_not_cancel_existing_composition() {
    let _scope = Dispatcher::unit_test_scope();
    let document = services(&DocumentSnapshot::from_text("abc"));
    document.selection().set_range(1, 2).unwrap();
    document.editor().set_preedit("😀", Some(2)).unwrap();
    let before = document.document().snapshot();
    let revision = document.document().revision();
    let selection = document.selection().selection();
    let mut source = one_field("linked", 0..6, 1);
    source.next_link_field_id = 0;
    let clipboard = clipboard(&source);
    let result = ready(
        document
            .get_range(0, 1)
            .unwrap()
            .paste_from_clipboard_async(clipboard.clipboard(), RichTextPasteFormat::BestAvailable),
    );
    assert!(result.is_err());
    assert_eq!(document.document().snapshot(), before);
    assert_eq!(document.document().revision(), revision);
    assert_eq!(document.selection().selection(), selection);
    assert!(document.editor().is_composing());
    assert!(!document.can_undo());
    assert!(document.editor().cancel_composition().unwrap());
    assert_eq!(document.text(), "abc");
    assert_eq!(document.selection().selection(), EditorSelection::new(1, 2));
}

struct ArmOnDispose {
    inner: Rc<dyn IAsyncDataTransfer>,
    armed: Rc<Cell<bool>>,
}

impl ferroui_base::reactive::IDisposable for ArmOnDispose {
    fn dispose(&self) {
        self.armed.set(true);
    }
}

impl IAsyncDataTransfer for ArmOnDispose {
    fn formats(&self) -> Rc<[DataFormat]> {
        self.inner.formats()
    }
    fn items(&self) -> Rc<[Rc<dyn ferroui_base::input::IAsyncDataTransferItem>]> {
        self.inner.items()
    }
}

struct FilteringCallbackBrush {
    armed: Rc<Cell<bool>>,
    callback: RefCell<Option<Box<dyn FnOnce()>>>,
}

impl ferroui_base::media::IBrush for FilteringCallbackBrush {
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
        if self.armed.get() {
            let callback = self.callback.borrow_mut().take();
            if let Some(callback) = callback {
                callback();
            }
        }
        None
    }
}
