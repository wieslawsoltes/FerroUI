use super::*;
use crate::clipboard_event_args::CopyingToClipboardEventArgs;
use crate::{EditorClipboard, EditorClipboardError, TextDocument};
use ferroui_base::input::platform::IPlatformDragSource;
use ferroui_base::input::{
    DataFormat, DataFormatOf, DataTransfer, DataTransferExtensions, DataTransferItem,
    DragDropEffects, DragEventArgs, IDataTransfer,
};
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_edit_core::document::{DocumentEdit, DocumentVersion, TextUtilities};
use ferroui_edit_core::editing::EditError;

#[cfg(test)]
#[path = "text_area_text_drag_tests.rs"]
mod tests;

ferroui_base::ferro_thread_local! {
    static ACTIVE_TEXT_DRAG: RefCell<Option<Rc<TextDrag>>> = const { RefCell::new(None) };
    static NEXT_DRAG_ID: Cell<u64> = const { Cell::new(0) };
}

struct TextDrag {
    editor: WeakRef<TextEditor>,
    document: TextDocument,
    version: DocumentVersion,
    selection: Selection,
    deletions: Vec<DocumentEdit>,
    data: Rc<dyn IDataTransfer>,
    id: String,
    moved: Cell<bool>,
}

struct TextDropTarget {
    editor: Ref<TextEditor>,
    caret: NavigationCaret,
    effect: DragDropEffects,
    source: Option<Rc<TextDrag>>,
}

impl TextDrag {
    fn matches_source(&self, editor: &TextEditor) -> bool {
        editor.document() == self.document
            && self.document.version() == self.version
            && editor.selection() == self.selection
    }

    fn covers(&self, offset: usize) -> bool {
        self.selection
            .segments()
            .iter()
            .any(|segment| segment.contains(offset))
    }

    fn can_move(&self) -> bool {
        self.editor.upgrade().is_some_and(|editor| {
            editor.text_area().uses_default_editor_input()
                && editor.options().enable_text_drag_drop
                && self.matches_source(&editor)
                && editor
                    .session()
                    .validate_document_edits(&self.deletions)
                    .is_ok()
                && self.matches_source(&editor)
        })
    }
}

struct ActiveDrag;

impl Drop for ActiveDrag {
    fn drop(&mut self) {
        ACTIVE_TEXT_DRAG.with(|active| {
            active.take();
        });
    }
}

fn drag_id_format() -> DataFormatOf<String> {
    DataFormat::create_string_application_format("ferroui-editor-drag-session")
}

fn local_text_drag(data: &Rc<dyn IDataTransfer>) -> Option<Rc<TextDrag>> {
    let drag = ACTIVE_TEXT_DRAG.with(|active| active.borrow().clone())?;
    // Several native backends preserve the transfer. Match the application
    // format as well for backends that round-trip through platform wrappers.
    (Rc::ptr_eq(&drag.data, data)
        || data.try_get_value(&drag_id_format()).as_ref() == Some(&drag.id))
    .then_some(drag)
}

impl TextArea {
    pub(super) fn register_text_drag_events() {
        DragDrop::drag_enter_event()
            .add_class_handler::<Self>(Self::preview_text_drop)
            .detach();
        DragDrop::drag_over_event()
            .add_class_handler::<Self>(Self::preview_text_drop)
            .detach();
        DragDrop::drag_leave_event()
            .add_class_handler::<Self>(|area, _| area.set_drop_caret(None))
            .detach();
        DragDrop::drop_event()
            .add_class_handler::<Self>(Self::receive_text_drop)
            .detach();
    }

    pub(super) fn can_start_text_drag(&self) -> bool {
        ACTIVE_TEXT_DRAG.with(|active| active.borrow().is_none())
            && FerroLocator::current()
                .get_service::<dyn IPlatformDragSource>()
                .is_some()
    }

    pub(super) fn start_text_drag(&self, editor: &TextEditor, trigger: &PointerPressedEventArgs) {
        if !self.can_start_text_drag() {
            return;
        }
        match self.prepare_text_drag(editor) {
            Ok(Some((drag, allowed))) => {
                if !self.can_start_text_drag() {
                    return;
                }
                ACTIVE_TEXT_DRAG.with(|active| active.replace(Some(drag.clone())));
                let active = ActiveDrag;
                let operation = DragDrop::do_drag_drop_async(trigger, drag.data.clone(), allowed);
                let area = self.to_ref().downgrade();
                drop(Dispatcher::ui_thread().spawn_local(async move {
                    let effect = operation.await;
                    drop(active);
                    if let Some(area) = area.upgrade() {
                        area.set_drop_caret(None);
                    }
                    if effect != DragDropEffects::MOVE
                        || !allowed.contains(DragDropEffects::MOVE)
                        || drag.moved.get()
                        || !drag.can_move()
                    {
                        return;
                    }
                    if let Some(editor) = drag.editor.upgrade() {
                        if let Err(error) = EditorClipboard::apply_clipboard_edits(
                            &editor,
                            &drag.document,
                            &drag.version,
                            &drag.selection,
                            &drag.deletions,
                        ) {
                            editor.report_error(error.to_string());
                        }
                    }
                }));
            }
            Ok(None) => {}
            Err(error) => editor.report_error(error.to_string()),
        }
    }

    fn prepare_text_drag(
        &self,
        editor: &TextEditor,
    ) -> Result<Option<(Rc<TextDrag>, DragDropEffects)>, EditorClipboardError> {
        let selection = editor.selection();
        if selection.is_empty() {
            return Ok(None);
        }
        let document = editor.document();
        let version = document.version();
        let Some(prepared) = EditorClipboard::prepare_copy_for_event(editor)? else {
            return Ok(None);
        };
        let args = CopyingToClipboardEventArgs::new(
            TextEditor::copying_to_clipboard_event(),
            prepared.data,
            false,
            prepared.kind,
            prepared.formatting_error,
        )
        .for_drag_drop();
        editor.raise_event(&args);
        if args.cancel() || args.handled() || !editor.options().enable_text_drag_drop {
            return Ok(None);
        }
        let id = NEXT_DRAG_ID.with(|next| {
            let id = next.get().wrapping_add(1);
            next.set(id);
            #[cfg(not(target_family = "wasm"))]
            return format!("{}-{id}", std::process::id());
            #[cfg(target_family = "wasm")]
            return id.to_string();
        });
        let payload = args.data();
        let data = DataTransfer::new();
        let marker = DataTransferItem::new();
        marker.set(&drag_id_format(), Some(id.clone()));
        data.add(marker);
        for item in payload.items().iter() {
            EditorClipboard::add_native_selection_markers(item, args.kind());
            data.add(item.clone());
        }
        let deletions = selection
            .segments()
            .iter()
            .filter(|segment| segment.length() > 0)
            .map(|segment| DocumentEdit::new(segment.start_offset(), segment.length(), ""))
            .collect();
        let drag = Rc::new(TextDrag {
            editor: editor.to_ref().downgrade(),
            document,
            version,
            selection,
            deletions,
            data,
            id,
            moved: Cell::new(false),
        });
        let mut allowed = DragDropEffects::COPY | DragDropEffects::LINK;
        if drag.can_move() {
            allowed |= DragDropEffects::MOVE;
        }
        Ok(drag.matches_source(editor).then_some((drag, allowed)))
    }

    pub(super) fn set_drop_caret(&self, caret: Option<NavigationCaret>) {
        if self.drop_caret.replace(caret) != caret {
            self.request_redraw();
        }
    }

    fn preview_text_drop(&self, e: &DragEventArgs) {
        if !self.handles_text_drop(e) {
            self.set_drop_caret(None);
            return;
        }
        let target = self.text_drop_target(e);
        self.set_drop_caret(target.as_ref().map(|target| target.caret));
        e.set_drag_effects(target.map_or(DragDropEffects::NONE, |target| target.effect));
        e.set_handled(
            e.data_transfer()
                .contains(DataFormat::text().as_data_format()),
        );
    }

    fn receive_text_drop(&self, e: &DragEventArgs) {
        self.set_drop_caret(None);
        if !self.handles_text_drop(e) {
            return;
        }
        let target = self.text_drop_target(e);
        e.set_drag_effects(DragDropEffects::NONE);
        e.set_handled(
            e.data_transfer()
                .contains(DataFormat::text().as_data_format()),
        );
        let Some(TextDropTarget {
            editor,
            caret,
            effect,
            source,
        }) = target
        else {
            return;
        };
        match self.commit_text_drop(&editor, caret, effect, source.as_deref(), e.data_transfer()) {
            Ok(true) => {
                self.focus();
                editor.selection_changed();
                e.set_drag_effects(effect);
            }
            Ok(false)
            | Err(EditorClipboardError::Edit(EditError::ReadOnly | EditError::StaleEditPlan)) => {}
            Err(error) => editor.report_error(error.to_string()),
        }
    }

    fn handles_text_drop(&self, e: &DragEventArgs) -> bool {
        !e.handled()
            && !self.is_hosted_child_event(e)
            && self.uses_default_editor_input()
            && self
                .editor()
                .is_some_and(|editor| editor.options().enable_text_drag_drop)
    }

    fn text_drop_target(&self, e: &DragEventArgs) -> Option<TextDropTarget> {
        if !e
            .data_transfer()
            .contains(DataFormat::text().as_data_format())
        {
            return None;
        }
        let editor = self.editor()?;
        let point = e.get_position(self);
        let viewport = self.viewport.get();
        if point.x < self.gutter_width()
            || point.x > viewport.width
            || point.y < 0.0
            || point.y > viewport.height
        {
            return None;
        }
        let document = editor.document();
        let version = document.version();
        let caret = self.navigation_caret_at_point(point, editor.options().enable_virtual_space);
        let drag = local_text_drag(e.data_transfer());
        if drag
            .as_ref()
            .is_some_and(|drag| drag.document == document && drag.covers(caret.offset))
        {
            return None;
        }
        let allowed = e.drag_effects();
        let effect = if !e.key_modifiers().contains(text_drag_copy_modifier())
            && allowed.contains(DragDropEffects::MOVE)
            && drag.as_ref().is_none_or(|drag| drag.can_move())
        {
            DragDropEffects::MOVE
        } else if allowed.contains(DragDropEffects::COPY) {
            DragDropEffects::COPY
        } else {
            return None;
        };
        let insertion = DocumentEdit::new(caret.offset, 0, " ");
        editor
            .session()
            .validate_document_edits(&[insertion])
            .ok()?;
        if editor.document() != document || document.version() != version {
            return None;
        }
        Some(TextDropTarget {
            editor,
            caret,
            effect,
            source: drag,
        })
    }

    fn commit_text_drop(
        &self,
        editor: &TextEditor,
        caret: NavigationCaret,
        effect: DragDropEffects,
        drag: Option<&TextDrag>,
        data: &Rc<dyn IDataTransfer>,
    ) -> Result<bool, EditorClipboardError> {
        let document = editor.document();
        let version = document.version();
        let selection = editor.selection();
        let Some(text) = data.try_get_text().filter(|text| !text.is_empty()) else {
            return Ok(false);
        };
        let same_document = drag.filter(|drag| drag.document == document);
        let moving = same_document.filter(|_| effect == DragDropEffects::MOVE);
        if moving.is_some_and(|drag| !drag.can_move()) {
            return Ok(false);
        }
        let deletions = moving.map_or(&[][..], |drag| drag.deletions.as_slice());
        let line = document.get_line_by_offset(caret.offset)?;
        let newline = TextUtilities::get_new_line_from_document(&document, line.line_number())?;
        let mut text = TextUtilities::normalize_new_lines(&text, &newline)?;
        let options = editor.options();
        if options.convert_tabs_to_spaces {
            text = text.replace('\t', &" ".repeat(options.indentation_size));
        }
        let position = SelectionSegment::with_virtual_space(
            caret.offset,
            caret.virtual_space,
            caret.offset,
            caret.virtual_space,
        );
        let rectangle = if EditorClipboard::has_rectangular_drop_marker(data.as_ref()) {
            self.rectangle_paste_positions(editor, caret, &text, selection.is_rectangular())?
        } else {
            None
        };
        if editor.document() != document
            || document.version() != version
            || editor.selection() != selection
        {
            return Err(EditError::StaleEditPlan.into());
        }
        if let Some(positions) = rectangle {
            if same_document.is_some_and(|drag| {
                positions
                    .segments()
                    .iter()
                    .any(|position| drag.covers(position.caret))
            }) {
                return Ok(false);
            }
            editor
                .session()
                .drop_rectangle_at(&positions, &text, deletions)?;
        } else {
            editor.session().drop_text_at(position, &text, deletions)?;
        }
        if let Some(drag) = moving {
            drag.moved.set(true);
        }
        Ok(true)
    }
}
