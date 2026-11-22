use super::*;
use ferroui_base::input::LocalBoxFuture;
use ferroui_base::platform::storage::{IStorageFolder, IStorageItem, StorageItemProperties};
use ferroui_base::reactive::IDisposable;
use ferroui_base::utilities::UriRef;
use std::future::{Future, poll_fn, ready};
use std::task::{Context, Poll, Waker};

#[test]
fn load_preserves_document_identity_and_bom_and_clears_history() {
    let _scope = crate::tests::test_scope();
    let editor = TextEditor::new();
    editor.set_text("previous text").unwrap();
    editor.set_caret_offset(5).unwrap();
    let document = editor.document();
    let text = "a😀\r\nb";
    let bytes: Vec<_> = TextEncoding::Utf16Be
        .byte_order_mark()
        .iter()
        .copied()
        .chain(text.encode_utf16().flat_map(u16::to_be_bytes))
        .collect();

    editor.load(bytes.as_slice()).unwrap();
    assert_eq!(editor.document(), document);
    assert_eq!(editor.text(), text);
    assert_eq!(editor.caret_offset(), 0);
    assert_eq!(editor.encoding(), TextEncoding::Utf16Be);
    assert!(editor.write_bom());
    assert!(!editor.is_modified());
    assert!(!document.can_undo());
    let mut output = Vec::new();
    editor.save(&mut output).unwrap();
    assert_eq!(output, bytes);
}

#[test]
fn save_marks_the_written_revision_clean_only_after_flush() {
    struct FailFlush;
    impl Write for FailFlush {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            Ok(bytes.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("flush failed"))
        }
    }

    let _scope = crate::tests::test_scope();
    let editor = TextEditor::new();
    let mut output = Vec::new();
    editor.set_text("price €").unwrap();
    editor.set_encoding(TextEncoding::Ascii);
    assert_eq!(
        editor.save(&mut output).unwrap_err().kind(),
        io::ErrorKind::InvalidData
    );
    assert!(editor.is_modified());
    editor.set_encoding(TextEncoding::Windows1252);
    assert!(editor.save(FailFlush).is_err());
    assert!(editor.is_modified());

    output.clear();
    editor.save(&mut output).unwrap();
    assert_eq!(output, b"price \x80");
    assert!(!editor.is_modified());
    editor.document().insert(0, "new ").unwrap();
    assert!(editor.get_direct_value(TextEditor::is_modified_property()));
    editor.set_is_modified(true);
    editor.undo().unwrap();
    assert!(!editor.get_direct_value(TextEditor::is_modified_property()));
}

#[test]
fn save_uses_one_snapshot_when_the_writer_edits_the_document() {
    struct EditingWriter {
        document: TextDocument,
        output: Vec<u8>,
    }

    impl Write for EditingWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.output.write(bytes)
        }
        fn flush(&mut self) -> io::Result<()> {
            self.document.insert(0, "new ").map_err(io::Error::other)
        }
    }

    let _scope = crate::tests::test_scope();
    let editor = TextEditor::new();
    editor.set_text("text").unwrap();
    let mut target = EditingWriter {
        document: editor.document(),
        output: Vec::new(),
    };
    editor.save(&mut target).unwrap();
    assert_eq!(target.output, b"text");
    assert_eq!(editor.text(), "new text");
    assert!(editor.is_modified());
}

#[test]
fn a_failed_read_keeps_the_existing_document_and_encoding() {
    let _scope = crate::tests::test_scope();
    let editor = TextEditor::new();
    editor.set_text("unsaved content").unwrap();
    editor.set_encoding(TextEncoding::Windows1252);
    let version = editor.document().version();
    let source = b"partial".as_slice();
    struct FailRead;
    impl Read for FailRead {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("read failed"))
        }
    }
    assert!(editor.load(source.chain(FailRead)).is_err());
    assert_eq!(editor.text(), "unsaved content");
    assert_eq!(editor.document().version(), version);
    assert_eq!(editor.encoding(), TextEncoding::Windows1252);
    assert!(editor.is_modified());
}

#[derive(Default)]
struct PendingWrite {
    staged: Vec<Vec<u8>>,
    committed: Vec<u8>,
    outcome: Option<io::Result<()>>,
}

struct TestFile {
    name: &'static str,
    content: &'static [u8],
    write: Rc<RefCell<PendingWrite>>,
}

impl IDisposable for TestFile {
    fn dispose(&self) {}
}

impl IStorageItem for TestFile {
    fn name(&self) -> String { self.name.to_owned() }
    fn path(&self) -> UriRef { UriRef::parse(self.name).unwrap() }
    fn get_basic_properties_async(&self) -> LocalBoxFuture<StorageItemProperties> {
        Box::pin(ready(StorageItemProperties::default()))
    }
    fn can_bookmark(&self) -> bool { false }
    fn save_bookmark_async(&self) -> LocalBoxFuture<Option<String>> { Box::pin(ready(None)) }
    fn get_parent_async(&self) -> LocalBoxFuture<Option<Rc<dyn IStorageFolder>>> { Box::pin(ready(None)) }
    fn delete_async(&self) -> LocalBoxFuture<io::Result<()>> { unreachable!() }
    fn move_async(&self, _: Rc<dyn IStorageFolder>) -> LocalBoxFuture<io::Result<Option<Rc<dyn IStorageItem>>>> { unreachable!() }
}

impl IStorageFile for TestFile {
    fn open_read_async(&self) -> LocalBoxFuture<io::Result<Box<dyn Read>>> {
        Box::pin(ready(Ok(Box::new(self.content) as Box<dyn Read>)))
    }
    fn open_write_async(&self) -> LocalBoxFuture<io::Result<Box<dyn Write>>> {
        panic!("The editor must use the platform commit operation")
    }
    fn write_with_async(&self, write: Box<dyn FnOnce(&mut dyn Write) -> io::Result<()>>) -> LocalBoxFuture<io::Result<()>> {
        let file = self.write.clone();
        Box::pin(async move {
            let mut output = Vec::new();
            write(&mut output)?;
            file.borrow_mut().staged.push(output.clone());
            poll_fn(|_| {
                let mut file = file.borrow_mut();
                let Some(outcome) = file.outcome.take() else { return Poll::Pending };
                if outcome.is_ok() {
                    file.committed = std::mem::take(&mut output);
                }
                Poll::Ready(outcome)
            }).await
        })
    }
}

#[test]
fn platform_saves_finish_in_order_and_leave_failed_writes_dirty() {
    let _scope = crate::tests::test_scope();
    let editor = TextEditor::new();
    let file = TestFile { name: "file.txt", content: b"", write: Rc::default() };
    let mut context = Context::from_waker(Waker::noop());
    editor.set_text("first").unwrap();
    let mut first = std::pin::pin!(editor.save_file(&file));
    assert!(first.as_mut().poll(&mut context).is_pending());
    assert!(editor.is_modified());
    assert!(editor.document().file_name().is_none());

    editor.set_text("second").unwrap();
    let mut second = std::pin::pin!(editor.save_file(&file));
    assert!(second.as_mut().poll(&mut context).is_pending());
    assert_eq!(file.write.borrow().staged, [b"first".to_vec()]);
    file.write.borrow_mut().outcome = Some(Ok(()));
    assert!(matches!(first.as_mut().poll(&mut context), Poll::Ready(Ok(()))));
    assert!(editor.is_modified());
    assert!(second.as_mut().poll(&mut context).is_pending());
    assert_eq!(file.write.borrow().staged, [b"first".to_vec(), b"second".to_vec()]);

    file.write.borrow_mut().outcome = Some(Err(io::Error::other("commit failed")));
    assert!(matches!(second.as_mut().poll(&mut context), Poll::Ready(Err(_))));
    assert!(editor.is_modified());
    assert_eq!(file.write.borrow().committed, b"first");

    file.write.borrow_mut().outcome = Some(Ok(()));
    let mut retry = std::pin::pin!(editor.save_file(&file));
    assert!(matches!(retry.as_mut().poll(&mut context), Poll::Ready(Ok(()))));
    assert_eq!(file.write.borrow().committed, b"second");
    assert!(!editor.is_modified());
}

#[test]
fn a_pending_save_does_not_rename_a_document_loaded_in_place() {
    let _scope = crate::tests::test_scope();
    let editor = TextEditor::new();
    editor.set_text("old text").unwrap();
    let document = editor.document();
    let old = TestFile { name: "old.txt", content: b"", write: Rc::default() };
    let mut context = Context::from_waker(Waker::noop());
    let mut save = std::pin::pin!(editor.save_file(&old));
    assert!(save.as_mut().poll(&mut context).is_pending());
    editor.load(b"new text".as_slice()).unwrap();
    document.set_file_name(Some("new.txt".into()));
    old.write.borrow_mut().outcome = Some(Ok(()));
    assert!(matches!(save.as_mut().poll(&mut context), Poll::Ready(Ok(()))));

    assert_eq!(old.write.borrow().committed, b"old text");
    assert_eq!(editor.document(), document);
    assert_eq!(document.file_name().as_deref(), Some("new.txt"));
    assert_eq!(editor.text(), "new text");
    assert!(!editor.is_modified());
}

#[test]
fn a_platform_load_waits_for_the_save_and_invalidates_a_queued_save_of_the_old_document() {
    let _scope = crate::tests::test_scope();
    let editor = TextEditor::new();
    editor.set_text("old text").unwrap();
    let old = TestFile { name: "old.txt", content: b"", write: Rc::default() };
    let new = TestFile { name: "new.txt", content: b"new text", write: Rc::default() };
    let mut context = Context::from_waker(Waker::noop());
    let mut save = std::pin::pin!(editor.save_file(&old));
    let mut load = std::pin::pin!(editor.load_file(&new));
    let mut obsolete = std::pin::pin!(editor.save_file(&old));
    assert!(save.as_mut().poll(&mut context).is_pending());
    assert!(load.as_mut().poll(&mut context).is_pending());
    assert!(obsolete.as_mut().poll(&mut context).is_pending());
    old.write.borrow_mut().outcome = Some(Ok(()));
    assert!(matches!(save.as_mut().poll(&mut context), Poll::Ready(Ok(()))));
    assert!(matches!(load.as_mut().poll(&mut context), Poll::Ready(Ok(()))));
    assert!(matches!(obsolete.as_mut().poll(&mut context), Poll::Ready(Err(_))));
    assert_eq!(old.write.borrow().staged, [b"old text".to_vec()]);
    assert_eq!(editor.document().file_name().as_deref(), Some("new.txt"));
    assert_eq!(editor.text(), "new text");
}

#[test]
fn changing_the_encoding_during_a_save_keeps_the_document_dirty() {
    let _scope = crate::tests::test_scope();
    let editor = TextEditor::new();
    editor.set_text("value").unwrap();
    let file = TestFile { name: "file.txt", content: b"", write: Rc::default() };
    let mut context = Context::from_waker(Waker::noop());
    let mut save = std::pin::pin!(editor.save_file(&file));
    assert!(save.as_mut().poll(&mut context).is_pending());
    editor.set_encoding(TextEncoding::Utf16Be);
    editor.set_write_bom(true);
    file.write.borrow_mut().outcome = Some(Ok(()));
    assert!(matches!(save.as_mut().poll(&mut context), Poll::Ready(Ok(()))));
    assert_eq!(file.write.borrow().committed, b"value");
    assert!(editor.is_modified());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn platform_save_validates_text_before_opening_a_truncating_stream() {
    use ferroui_base::platform::storage::file_io::{BclStorageFile, FileSystemInfo};
    use std::future::Future;
    use std::task::{Context, Poll, Waker};

    fn ready<T>(future: impl Future<Output = T>) -> T {
        match std::pin::pin!(future).poll(&mut Context::from_waker(Waker::noop())) {
            Poll::Ready(result) => result,
            Poll::Pending => panic!("local filesystem operations are immediately ready"),
        }
    }

    struct TempPath(std::path::PathBuf);
    impl Drop for TempPath {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.0);
        }
    }

    let _scope = crate::tests::test_scope();
    let path = std::env::temp_dir().join(format!("ferro-editor-save-{}.txt", std::process::id()));
    let mut original = std::fs::File::create_new(&path).unwrap();
    let temporary = TempPath(path);
    original.write_all(b"existing file").unwrap();
    drop(original);
    let file = BclStorageFile::new(FileSystemInfo::file(&temporary.0));
    let editor = TextEditor::new();
    editor.set_text("😀").unwrap();
    editor.set_encoding(TextEncoding::Latin1);
    let error = ready(editor.save_file(file.as_ref())).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidData);
    assert_eq!(std::fs::read(&temporary.0).unwrap(), b"existing file");
    assert!(editor.is_modified());

    editor.set_encoding(TextEncoding::Utf32Le);
    editor.set_write_bom(true);
    ready(editor.save_file(file.as_ref())).unwrap();
    assert!(!editor.is_modified());
    let loaded = TextEditor::new();
    ready(loaded.load_file(file.as_ref())).unwrap();
    assert_eq!(loaded.text(), "😀");
    assert_eq!(loaded.encoding(), TextEncoding::Utf32Le);
    assert!(!loaded.is_modified());
}
