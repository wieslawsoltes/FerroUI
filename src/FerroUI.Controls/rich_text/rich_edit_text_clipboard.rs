//! Asynchronous FerroUI equivalents of the TOM clipboard range/document methods.
//!
//! Source contracts: pinned Microsoft.UI.Text.xml ITextRange.Copy/Cut/Paste/
//! CanPaste and RichEditTextDocument.CanCopy/CanPaste. Clipboard access is an
//! injected framework service; native registered format numbers are not portable.
use super::*;
use crate::rich_text::rich_text_transfer::RichTextTransferContent;
use crate::rich_text::{RichEditClipboardFormat, RichTextPasteFormat, RichTextTransfer};
use ferroui_base::input::platform::{ClipboardError, IClipboard};
use ferroui_base::input::{IAsyncDataTransfer, LocalBoxFuture};
use ferroui_base::reactive::IDisposable;
use std::cell::Cell;

/// Only data retrieved from the clipboard belongs to the caller. Keep it alive
/// across payload awaits and dispose it on success, error, or future cancellation.
struct RetrievedTransfer(Rc<dyn IAsyncDataTransfer>);

impl Drop for RetrievedTransfer {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

/// Document mutation identity sees writes inside held model batches. The range
/// generation sees explicit endpoint/default changes, including away-and-back
/// moves. Editor publication also retires requests after selection or an empty
/// IME session changes, even if the visible document has not changed.
struct PendingRange {
    range: RichEditTextRange,
    generation: u64,
    mutation_version: u64,
    editor_changed: Rc<Cell<bool>>,
    subscription: Rc<dyn IDisposable>,
}

impl PendingRange {
    fn capture(range: &RichEditTextRange) -> Self {
        let changed = Rc::new(Cell::new(false));
        let observed = changed.clone();
        let subscription = range.editor.changed(move || observed.set(true));
        Self {
            range: range.clone(),
            generation: range.data.borrow().generation,
            mutation_version: range.document().mutation_version(),
            editor_changed: changed,
            subscription,
        }
    }

    fn ensure_current(&self) -> Result<(), ClipboardError> {
        ensure_writable(&self.range.editor)?;
        if self.editor_changed.get()
            || self.range.data.borrow().generation != self.generation
            || self.range.document().mutation_version() != self.mutation_version
        {
            return Err(ClipboardError::canceled(
                "The clipboard target changed before the operation completed.",
            ));
        }
        Ok(())
    }
}

impl Drop for PendingRange {
    fn drop(&mut self) {
        self.subscription.dispose();
    }
}

impl RichEditTextDocument {
    /// Whether copying document content is permitted. This is independent of
    /// selection length and edit protection. FerroUI documents currently have
    /// no copy-protection policy, so copying is permitted even for an empty or
    /// read-only document.
    pub fn can_copy(&self) -> bool {
        true
    }

    /// Queries the clipboard's supported representations without reading their
    /// payloads or changing selection, composition, document state, or history.
    pub fn can_paste_from_clipboard_async(
        &self,
        clipboard: Rc<dyn IClipboard>,
    ) -> LocalBoxFuture<Result<bool, ClipboardError>> {
        can_paste(
            self.editor.clone(),
            clipboard,
            RichTextPasteFormat::BestAvailable,
        )
    }
}

impl RichEditTextRange {
    /// Queries the exact requested format policy. Read-only targets return false
    /// without accessing the clipboard. Availability does not validate a payload
    /// that may be supplied lazily, or guarantee a subsequent platform read.
    pub fn can_paste_from_clipboard_async(
        &self,
        clipboard: Rc<dyn IClipboard>,
        format: RichTextPasteFormat,
    ) -> LocalBoxFuture<Result<bool, ClipboardError>> {
        can_paste(self.editor.clone(), clipboard, format)
    }

    /// Copies the captured range as exact in-process content, representable RTF,
    /// and plain text. Copy never changes the editor's selection, composition or
    /// undo history; a degenerate range leaves the clipboard unchanged.
    ///
    /// The immutable fragment is captured when this method is called. The
    /// clipboard owns the outgoing transfer once `set_data_async` is invoked.
    pub fn copy_to_clipboard_async(
        &self,
        clipboard: Rc<dyn IClipboard>,
    ) -> LocalBoxFuture<Result<(), ClipboardError>> {
        let transfer = self.clipboard_transfer();
        Box::pin(async move {
            if let Some(transfer) = transfer? {
                clipboard.set_data_async(Some(transfer)).await?;
            }
            Ok(())
        })
    }

    /// Copies this independent range, then deletes precisely that captured
    /// target after the clipboard write succeeds. Later edits, range changes,
    /// selection requests, or IME sessions cancel the pending deletion.
    ///
    /// On its first poll, a nonempty cut follows the editor's ordinary edit
    /// policy and cancels an existing preview before capturing committed text.
    /// A failed clipboard write leaves that committed text and history intact;
    /// it does not restart the canceled preview. An empty cut has no side effects.
    pub fn cut_to_clipboard_async(
        &self,
        clipboard: Rc<dyn IClipboard>,
    ) -> LocalBoxFuture<Result<(), ClipboardError>> {
        let range = self.clone();
        let pending = PendingRange::capture(self);
        Box::pin(async move {
            pending.ensure_current()?;
            if range.is_degenerate() {
                return Ok(());
            }
            drop(pending);
            range.prepare_clipboard_edit()?;
            let pending = PendingRange::capture(&range);
            let Some(transfer) = range.clipboard_transfer()? else {
                return Ok(());
            };
            pending.ensure_current()?;
            clipboard.set_data_async(Some(transfer)).await?;
            pending.ensure_current()?;
            drop(pending);
            range
                .replace_from_clipboard(None)
                .map_err(clipboard_edit_error)
        })
    }

    /// Replaces this independent range using exactly the requested clipboard
    /// format policy. A successful edit preserves and rebases the editor's
    /// directed selection and creates one undo unit.
    ///
    /// Missing/empty data, invalid payloads, platform errors and stale requests
    /// do not cancel IME or change the target. After a valid payload arrives, an
    /// original preview is canceled and the live committed endpoints are read
    /// again. Plain text inherits insertion formatting; rich fragments keep
    /// their assigned formatting and structure.
    pub fn paste_from_clipboard_async(
        &self,
        clipboard: Rc<dyn IClipboard>,
        format: RichTextPasteFormat,
    ) -> LocalBoxFuture<Result<(), ClipboardError>> {
        let range = self.clone();
        let pending = PendingRange::capture(self);
        let default_tab_stop = self
            .editor
            .default_paragraph_format()
            .default_tab_stop
            .unwrap_or(48.0);
        Box::pin(async move {
            pending.ensure_current()?;
            let Some(data) = clipboard.try_get_data_async().await? else {
                return Ok(());
            };
            let data = RetrievedTransfer(data);
            let content =
                RichTextTransfer::read_content(data.0.as_ref(), format, Some(default_tab_stop))
                    .await;
            // Disposal is an application/platform callback boundary too. It
            // must run before the final target validation and the model edit.
            drop(data);
            let content = content?;
            pending.ensure_current()?;
            let Some(content) = content else {
                return Ok(());
            };
            if range.limited_clipboard_content(&content)?.is_none() {
                return Ok(());
            }
            pending.ensure_current()?;
            drop(pending);
            range.prepare_clipboard_edit()?;
            let Some(content) = range.limited_clipboard_content(&content)? else {
                return Ok(());
            };
            range
                .replace_from_clipboard(Some(&content))
                .map_err(clipboard_edit_error)
        })
    }

    fn clipboard_transfer(&self) -> Result<Option<Rc<dyn IAsyncDataTransfer>>, ClipboardError> {
        if self.is_degenerate() {
            return Ok(None);
        }
        let snapshot = self.snapshot().map_err(clipboard_edit_error)?;
        Ok(Some(RichTextTransfer::create_with_default_tab_stop(
            &snapshot,
            RichEditClipboardFormat::AllFormats,
            self.editor
                .default_paragraph_format()
                .default_tab_stop
                .unwrap_or(48.0),
        )))
    }

    fn prepare_clipboard_edit(&self) -> Result<(), ClipboardError> {
        ensure_writable(&self.editor)?;
        let generation = self.data.borrow().generation;
        self.editor.prepare_edit().map_err(clipboard_edit_error)?;
        if self.data.borrow().generation != generation || self.editor.is_composing() {
            return Err(ClipboardError::canceled(
                "The clipboard target changed while ending the original composition.",
            ));
        }
        ensure_writable(&self.editor)
    }

    fn limited_clipboard_content(
        &self,
        content: &RichTextTransferContent,
    ) -> Result<Option<RichTextTransferContent>, ClipboardError> {
        let text = match content {
            RichTextTransferContent::PlainText(text) => DocumentSnapshot::from_text(text).text(),
            RichTextTransferContent::RichText(snapshot) => {
                snapshot
                    .validate()
                    .map_err(|error| clipboard_edit_error(error.into()))?;
                snapshot.text()
            }
        };
        let limit = if self.editor.max_length() == 0 {
            usize::MAX
        } else {
            self.editor
                .max_length()
                .saturating_sub(self.document().text_length().saturating_sub(self.length()))
        };
        let accepted = crate::rich_text::utf16_prefix(&text, limit);
        if accepted.is_empty() {
            return Ok(None);
        }
        Ok(Some(match content {
            RichTextTransferContent::PlainText(_) => {
                RichTextTransferContent::PlainText(accepted.to_owned())
            }
            RichTextTransferContent::RichText(snapshot) => {
                let length = accepted.encode_utf16().count();
                RichTextTransferContent::RichText(if length == snapshot.text_length() {
                    snapshot.clone()
                } else {
                    snapshot.slice_unchecked(0..length)
                })
            }
        }))
    }

    fn replace_from_clipboard(
        &self,
        content: Option<&RichTextTransferContent>,
    ) -> Result<(), EditError> {
        let checkpoint = self.range_read_checkpoint();
        let original = self.range();
        let original_format = self.range_typing_format();
        let insertion_format = if original.is_empty()
            && matches!(content, Some(RichTextTransferContent::PlainText(_)))
        {
            Some(self.caret_character_format()?)
        } else {
            None
        };
        self.ensure_range_read(checkpoint)?;
        let generation = checkpoint.0;
        let mut owned_generation = generation;
        let external_generation = self.editor.external_change_generation();
        let document = self.document();
        let result = self.editor.edit_group(|editor| {
            self.ensure_range_read(checkpoint)?;
            let selection = editor.selection();
            let typing_format = editor.typing_format();
            let inserted = match content {
                None => {
                    super::range_operations::delete_span(editor, original.clone(), false)?;
                    0
                }
                Some(RichTextTransferContent::PlainText(text)) => {
                    let was_empty = document.text_length() == 0;
                    document.replace_text(original.clone(), text)?;
                    let inserted = text.encode_utf16().count();
                    if was_empty {
                        document.apply_paragraph_format(
                            0..document.text_length(),
                            &editor.insertion_defaults(),
                        )?;
                    }
                    if let Some(format) = &insertion_format {
                        document.apply_format(original.start..original.start + inserted, format)?;
                    }
                    inserted
                }
                Some(RichTextTransferContent::RichText(snapshot)) => {
                    document.replace_range(original.clone(), snapshot)?;
                    snapshot.text_length()
                }
            };
            if self.data.borrow().generation != generation {
                return Err(EditError::ReentrantEdit);
            }
            let selection = rebase_selection(selection, original.clone(), inserted);
            editor.set_selection(selection)?;
            if selection.is_empty() {
                editor.replace_range_typing_format(&typing_format)?;
            }
            self.ensure_range_generation(generation)?;
            self.set_range(original.start, original.start + inserted)?;
            owned_generation = self.data.borrow().generation;
            Ok(())
        });
        if result.is_err()
            && self.editor.external_change_generation() == external_generation
            && self.data.borrow().generation == owned_generation
        {
            // Model/history rollback belongs to the shared editor. Restore our
            // explicit endpoints only while this command still owns them;
            // reentrant range changes and external document writes survive.
            self.set_range(original.start, original.end)?;
            self.set_range_typing_format(original_format)?;
        }
        result
    }
}

fn can_paste(
    editor: Rc<RichTextEditor>,
    clipboard: Rc<dyn IClipboard>,
    format: RichTextPasteFormat,
) -> LocalBoxFuture<Result<bool, ClipboardError>> {
    let permitted = !editor.is_read_only();
    Box::pin(async move {
        if !permitted {
            return Ok(false);
        }
        let Some(data) = clipboard.try_get_data_async().await? else {
            return Ok(false);
        };
        let data = RetrievedTransfer(data);
        let available = RichTextTransfer::can_read(data.0.as_ref(), format);
        drop(data);
        Ok(available? && !editor.is_read_only())
    })
}

fn ensure_writable(editor: &RichTextEditor) -> Result<(), ClipboardError> {
    if editor.is_read_only() {
        Err(clipboard_edit_error(EditError::ReadOnly))
    } else if editor.is_in_operation() {
        Err(clipboard_edit_error(EditError::ReentrantEdit))
    } else {
        Ok(())
    }
}

fn clipboard_edit_error(error: EditError) -> ClipboardError {
    match error {
        EditError::ReadOnly => ClipboardError::access_denied(error.to_string()),
        EditError::ReentrantEdit => ClipboardError::canceled(error.to_string()),
        _ => ClipboardError::other(error.to_string()),
    }
}

#[cfg(test)]
#[path = "rich_edit_text_clipboard_tests.rs"]
mod tests;
