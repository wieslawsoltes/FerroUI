use super::*;
use crate::{TextDocument, VisualLineError};
use ferroui_base::input::{IPointer, PointerPointProperties};
use ferroui_edit_core::document::{
    AnchorMovementType, AnchorSegment, CaretPositioningMode, DocumentChangeEventArgs,
    DocumentVersion, LogicalDirection, TextAnchor,
};
use ferroui_edit_core::editing::EditError;
use std::any::Any;
use std::ops::Range;

#[derive(Clone, Copy)]
enum SelectionUnit {
    Word,
    Line,
    GutterLine,
}

#[derive(Clone, Copy)]
pub(super) enum TouchSelection {
    Caret,
    Word,
    WordDrag,
}

#[derive(Clone)]
enum SelectionDrag {
    PendingText(PendingTextDrag),
    Characters {
        anchor: TextAnchor,
        virtual_space: usize,
    },
    Box {
        anchor: TextAnchor,
        x: f64,
    },
    Unit {
        anchor: AnchorSegment,
        unit: SelectionUnit,
    },
}

#[derive(Clone)]
struct PendingTextDrag {
    start: Point,
    root_position: Point,
    timestamp: u64,
    properties: PointerPointProperties,
    modifiers: KeyModifiers,
    cookie: Option<Rc<dyn Any>>,
    selection: Selection,
    version: DocumentVersion,
}

impl PendingTextDrag {
    fn trigger(
        self,
        area: &TextArea,
        pointer: Rc<dyn IPointer>,
    ) -> Option<PointerPressedEventArgs> {
        let root = area.visual_root()?;
        Some(
            PointerPressedEventArgs::new(
                area.to_ref(),
                pointer,
                &root,
                self.root_position,
                self.timestamp,
                self.properties,
                self.modifiers,
                1,
            )
            .with_platform_input_event_cookie(self.cookie),
        )
    }
}

#[derive(Clone)]
pub(super) struct PointerSelection {
    pub(super) id: i32,
    pub(super) point: Point,
    pointer: std::rc::Weak<dyn IPointer>,
    document: TextDocument,
    drag: SelectionDrag,
}

struct ExtensionScope<'a>(&'a Cell<bool>);

impl Drop for ExtensionScope<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

impl TextArea {
    fn clamp_selection_point(&self, point: Point) -> Point {
        Point::new(point.x, point.y.clamp(0.0, self.viewport.get().height))
    }

    pub(super) fn move_context_caret(
        &self,
        editor: &TextEditor,
        point: Point,
    ) -> Result<(), EditError> {
        let document = editor.document();
        let version = document.version();
        let layout = self.visual_lines.revision.get();
        let selection = editor.selection();
        let previous = self.navigation_caret(&selection);
        let allow_virtual = selection.is_rectangular() || editor.options().enable_virtual_space;
        let position = self.navigation_caret_at_point(point, allow_virtual);
        if editor.document() != document
            || document.version() != version
            || editor.selection() != selection
            || self.visual_lines.revision.get() != layout
        {
            return Err(EditError::StaleEditPlan);
        }
        let containing = selection
            .segments()
            .iter()
            .copied()
            .enumerate()
            .find(|(_, segment)| {
                !segment.is_empty()
                    && segment.contains_position(position.offset, position.virtual_space)
            });
        if let Some((index, segment)) = containing {
            if index != selection.primary_index() {
                let segments = selection.segments().iter().copied();
                let updated = if selection.is_rectangular() {
                    Selection::from_rectangle_segments(segments, index)
                } else {
                    Selection::from_segments(segments, index)
                };
                editor.session().set_selection(updated)?;
                self.box_caret
                    .set(self.active_box_caret(&selection).map(|mut box_caret| {
                        box_caret.projected_primary = segment;
                        box_caret
                    }));
            }
            self.context_caret
                .set(Some(ContextCaret { position, segment }));
            self.apply_caret_affinity(position.at_end_of_row);
        } else {
            editor
                .session()
                .set_caret_position(position.offset, position.virtual_space)?;
            self.selection_changed_with_affinity(position.at_end_of_row);
        }
        editor.notify_selection_changed(previous != position);
        Ok(())
    }

    pub(crate) fn rebase_context_caret(
        &self,
        editor: &TextEditor,
        change: &DocumentChangeEventArgs,
    ) -> bool {
        let Some(caret) = self.context_caret.take() else {
            return false;
        };
        let previous = caret.position;
        let movement = if (previous.offset, previous.virtual_space) == caret.segment.end_tuple() {
            AnchorMovementType::BeforeInsertion
        } else {
            AnchorMovementType::Default
        };
        let offset = change.get_new_offset(previous.offset, movement);
        let document = editor.document();
        let virtual_space = if document
            .get_line_by_offset(offset)
            .is_ok_and(|line| line.end_offset() == offset)
        {
            previous.virtual_space
        } else {
            0
        };
        let segment = editor.selection().primary();
        if segment.is_empty() || !segment.contains_position(offset, virtual_space) {
            return false;
        }
        let position = NavigationCaret {
            offset,
            virtual_space,
            at_end_of_row: false,
        };
        self.context_caret
            .set(Some(ContextCaret { position, segment }));
        self.box_caret.set(None);
        self.apply_caret_affinity(false);
        editor.notify_selection_changed(previous != position);
        true
    }

    pub(super) fn cancel_pointer_selection(&self) {
        self.touch_press.set(None);
        let state = self.pointer_selection.take();
        if let Some(pointer) = state.and_then(|state| state.pointer.upgrade()) {
            if pointer
                .captured()
                .is_some_and(|target| target.ptr_eq(&self.to_ref()))
            {
                pointer.capture(None);
            }
        }
        self.set_hover_cursor(None);
    }

    pub(super) fn select_touch(
        &self,
        pointer: &Rc<dyn IPointer>,
        point: Point,
        mode: TouchSelection,
    ) -> bool {
        if pointer.type_() != PointerType::Touch || self.touch_press.get() != Some(pointer.id()) {
            return false;
        }
        self.touch_press.set(None);
        if !self.text_viewport_contains(point) || !self.uses_default_editor_input() {
            return false;
        }
        self.focus();
        let Some(editor) = self.editor().filter(|_| self.uses_default_editor_input()) else {
            return false;
        };
        if let Err(error) = self.apply_touch_selection(&editor, pointer, point, mode) {
            editor.report_error(error.to_string());
        }
        true
    }

    fn apply_touch_selection(
        &self,
        editor: &TextEditor,
        pointer: &Rc<dyn IPointer>,
        point: Point,
        mode: TouchSelection,
    ) -> Result<(), EditError> {
        let document = editor.document();
        let version = document.version();
        let selection = editor.selection();
        let revision = self.visual_lines.revision.get();
        let (segment, affinity) = match mode {
            TouchSelection::Caret => {
                let position =
                    self.navigation_caret_at_point(point, editor.options().enable_virtual_space);
                let segment = SelectionSegment::with_virtual_space(
                    position.offset,
                    position.virtual_space,
                    position.offset,
                    position.virtual_space,
                );
                (segment, position.at_end_of_row)
            }
            TouchSelection::Word | TouchSelection::WordDrag => {
                let (range, affinity) = self
                    .pointer_unit(point, SelectionUnit::Word)
                    .map_err(|_| EditError::InvalidSelection)?;
                (SelectionSegment::new(range.start, range.end), affinity)
            }
        };
        if editor.document() != document
            || document.version() != version
            || editor.selection() != selection
            || self.visual_lines.revision.get() != revision
        {
            return Err(EditError::StaleEditPlan);
        }
        let drag = if matches!(mode, TouchSelection::WordDrag) {
            let anchor = AnchorSegment::new(&document, segment.start_offset(), segment.length())?;
            Some(SelectionDrag::Unit {
                anchor,
                unit: SelectionUnit::Word,
            })
        } else {
            None
        };
        editor.session().set_selection(Selection::from_segments([segment], 0))?;
        if let Some(drag) = drag {
            self.capture_selection(pointer, point, document, drag);
        }
        self.updating_pointer_selection.set(true);
        let _extension = ExtensionScope(&self.updating_pointer_selection);
        editor.selection_changed_with_affinity(affinity);
        Ok(())
    }

    pub(crate) fn press_line_number_margin(&self, event: &PointerPressedEventArgs) {
        if event.handled() || self.pointer_selection.borrow().is_some() || !self.uses_default_editor_input() {
            return;
        }
        let point = event.get_current_point(Some(self));
        if !point.properties.is_left_button_pressed {
            return;
        }
        let Some(editor) = self.editor() else { return };
        self.focus();
        if let Err(error) = self.begin_pointer_selection(&editor, event, point.position, true) {
            editor.report_error(error.to_string());
        }
        event.set_handled(true);
    }

    pub(super) fn begin_pointer_selection(
        &self,
        editor: &TextEditor,
        e: &PointerPressedEventArgs,
        point: Point,
        line_number: bool,
    ) -> Result<(), EditError> {
        let document = editor.document();
        let version = document.version();
        let selection = editor.selection();
        let options = editor.options();
        let shift = e.key_modifiers().contains(KeyModifiers::SHIFT);
        let gutter = line_number || point.x < self.gutter_width();
        let box_mode = options.enable_rectangular_selection
            && e.key_modifiers().contains(box_selection_modifier())
            && !gutter;
        let caret = self.navigation_caret_at_point(point, options.enable_virtual_space || box_mode);
        if !gutter
            && !shift
            && e.click_count() == 1
            && options.enable_text_drag_drop
            && self.can_start_text_drag()
            && selection.segments().iter().any(|segment| {
                segment.length() > 0 && segment.contains_position(caret.offset, caret.virtual_space)
            })
        {
            if editor.document() != document
                || document.version() != version
                || editor.selection() != selection
            {
                return Err(EditError::StaleEditPlan);
            }
            let root_point = e.get_current_point(None);
            self.capture_selection(
                e.pointer(),
                point,
                document,
                SelectionDrag::PendingText(PendingTextDrag {
                    start: point,
                    root_position: root_point.position,
                    timestamp: e.timestamp(),
                    properties: root_point.properties,
                    modifiers: e.key_modifiers(),
                    cookie: e.platform_input_event_cookie().cloned(),
                    selection,
                    version,
                }),
            );
            return Ok(());
        }
        let primary = selection.primary();
        let control_word = !gutter
            && !box_mode
            && e.click_count() == 1
            && e.key_modifiers().contains(KeyModifiers::CONTROL);
        let unit = if line_number {
            Some(SelectionUnit::GutterLine)
        } else if box_mode {
            None
        } else if e.click_count() == 2 || control_word {
            Some(SelectionUnit::Word)
        } else if e.click_count() >= 3 {
            Some(SelectionUnit::Line)
        } else {
            None
        };
        let (drag, segment, affinity) = if let Some(unit) = unit {
            let (target, soft_wrap_end) = self
                .pointer_unit(point, unit)
                .map_err(|_| EditError::InvalidSelection)?;
            let existing = primary.start_offset()..primary.end_offset();
            let gutter_unit = matches!(unit, SelectionUnit::GutterLine);
            let simple = !selection.is_rectangular() && selection.segments().len() == 1;
            let extend = shift && !selection.is_empty() && (!gutter_unit || simple);
            let segment = if control_word && shift && !selection.is_rectangular() {
                SelectionSegment::with_virtual_space(
                    primary.anchor_tuple().0,
                    primary.anchor_tuple().1,
                    caret.offset,
                    caret.virtual_space,
                )
            } else if extend {
                extend_unit(existing.clone(), target.clone())
            } else {
                SelectionSegment::new(target.start, target.end)
            };
            let anchor = if control_word {
                AnchorSegment::new(&document, target.start, target.len())?
            } else if gutter_unit && extend {
                AnchorSegment::new(&document, existing.start, existing.len())?
            } else {
                AnchorSegment::new(&document, segment.start_offset(), segment.length())?
            };
            let affinity = soft_wrap_end && segment.caret == target.end;
            (SelectionDrag::Unit { anchor, unit }, segment, affinity)
        } else {
            let (anchor, virtual_space) = if shift {
                primary.anchor_tuple()
            } else {
                (caret.offset, caret.virtual_space)
            };
            let tracked = document.create_anchor(anchor)?;
            tracked.set_survive_deletion(true);
            let drag = if box_mode {
                let x = if !shift {
                    self.caret_x(caret)
                } else if let Some(box_caret) = self.active_box_caret(&selection) {
                    box_caret.anchor.1
                } else {
                    self.x_for_position(anchor, virtual_space)
                };
                SelectionDrag::Box { anchor: tracked, x }
            } else {
                SelectionDrag::Characters {
                    anchor: tracked,
                    virtual_space,
                }
            };
            (
                drag,
                SelectionSegment::with_virtual_space(
                    anchor,
                    virtual_space,
                    caret.offset,
                    caret.virtual_space,
                ),
                caret.at_end_of_row,
            )
        };
        if editor.document() != document
            || document.version() != version
            || editor.selection() != selection
        {
            return Err(EditError::StaleEditPlan);
        }
        editor
            .session()
            .set_selection(Selection::from_segments([segment], 0))?;
        self.capture_selection(e.pointer(), point, document, drag);
        self.updating_pointer_selection.set(true);
        let _extension = ExtensionScope(&self.updating_pointer_selection);
        editor.selection_changed_with_affinity(affinity);
        Ok(())
    }

    fn capture_selection(
        &self,
        pointer: &Rc<dyn IPointer>,
        point: Point,
        document: TextDocument,
        drag: SelectionDrag,
    ) {
        pointer.capture(Some(&self.to_ref().upcast()));
        if !pointer
            .captured()
            .is_some_and(|target| target.ptr_eq(&self.to_ref()))
        {
            return;
        }
        self.pointer_selection.replace(Some(PointerSelection {
            id: pointer.id(),
            point,
            pointer: Rc::downgrade(pointer),
            document,
            drag,
        }));
    }

    pub(super) fn move_pointer_selection(&self, e: &PointerEventArgs, point: Point) {
        let pending =
            self.pointer_selection
                .borrow()
                .as_ref()
                .and_then(|state| match &state.drag {
                    SelectionDrag::PendingText(pending) => {
                        Some((pending.clone(), state.document.clone()))
                    }
                    _ => None,
                });
        let Some((pending, document)) = pending else {
            self.extend_pointer_selection(point);
            return;
        };
        if (point.x - pending.start.x).abs() <= 2.0 && (point.y - pending.start.y).abs() <= 2.0 {
            return;
        }
        self.pointer_selection.take();
        e.pointer().capture(None);
        let Some(editor) = self.editor().filter(|editor| {
            editor.document() == document
                && document.version() == pending.version
                && editor.selection() == pending.selection
                && editor.options().enable_text_drag_drop
        }) else {
            return;
        };
        if let Some(trigger) = pending.trigger(self, e.pointer().clone()) {
            self.start_text_drag(&editor, &trigger);
        }
    }

    pub(super) fn finish_pointer_selection(&self, point: Point) {
        let pending =
            self.pointer_selection
                .borrow()
                .as_ref()
                .and_then(|state| match &state.drag {
                    SelectionDrag::PendingText(pending) => {
                        Some((pending.clone(), state.document.clone()))
                    }
                    _ => None,
                });
        let Some(editor) = self.editor() else {
            return;
        };
        if let Some((pending, document)) = pending {
            if editor.document() == document
                && document.version() == pending.version
                && editor.selection() == pending.selection
            {
                let caret = self.navigation_caret_at_point(
                    self.clamp_selection_point(point),
                    editor.options().enable_virtual_space,
                );
                if editor.document() == document
                    && document.version() == pending.version
                    && editor.selection() == pending.selection
                {
                    let selection = Selection::from_segments(
                        [SelectionSegment::with_virtual_space(
                            caret.offset,
                            caret.virtual_space,
                            caret.offset,
                            caret.virtual_space,
                        )],
                        0,
                    );
                    if editor.session().set_selection(selection).is_ok() {
                        editor.selection_changed_with_affinity(caret.at_end_of_row);
                    }
                }
            }
        } else if editor.options().extend_selection_on_mouse_up {
            self.extend_pointer_selection(point);
        }
    }

    pub(super) fn extend_pointer_selection(&self, point: Point) {
        if self.updating_pointer_selection.replace(true) {
            return;
        }
        let _scope = ExtensionScope(&self.updating_pointer_selection);
        let drag = self.pointer_selection.borrow_mut().as_mut().map(|drag| {
            drag.point = point;
            drag.clone()
        });
        if let (Some(drag), Some(editor)) = (drag, self.editor()) {
            if let Err(error) = self.extend_pointer_drag(&editor, drag, point) {
                editor.report_error(error.to_string());
            }
        }
    }

    fn extend_pointer_drag(
        &self,
        editor: &TextEditor,
        state: PointerSelection,
        point: Point,
    ) -> Result<(), EditError> {
        let document = editor.document();
        if document != state.document {
            return Ok(());
        }
        let version = document.version();
        let selection = editor.selection();
        // Keep the raw pointer in the capture state so scrolling can
        // project it against the current viewport.
        let point = self.clamp_selection_point(point);
        let (segment, affinity) = match state.drag {
            SelectionDrag::PendingText(_) => return Ok(()),
            SelectionDrag::Characters {
                anchor,
                virtual_space,
            } => {
                let anchor = anchor.offset()?;
                let virtual_space = if virtual_space > 0
                    && document.get_line_by_offset(anchor)?.end_offset() == anchor
                {
                    virtual_space
                } else {
                    0
                };
                let caret = self.navigation_caret_at_point(
                    point,
                    editor.options().enable_virtual_space || virtual_space > 0,
                );
                (
                    SelectionSegment::with_virtual_space(
                        anchor,
                        virtual_space,
                        caret.offset,
                        caret.virtual_space,
                    ),
                    caret.at_end_of_row,
                )
            }
            SelectionDrag::Box { anchor, x } => {
                let index = self
                    .line_owner(&document, anchor.offset()?)
                    .map_err(|_| EditError::InvalidSelection)?;
                let revision = self.visual_lines.revision.get();
                let (target, caret) = self.rectangle_endpoint(point)?;
                if editor.document() != document
                    || document.version() != version
                    || editor.selection() != selection
                    || self.visual_lines.revision.get() != revision
                {
                    return Err(EditError::StaleEditPlan);
                }
                return self.select_rectangle_to((index, x), target, caret, None);
            }
            SelectionDrag::Unit { anchor, unit } => {
                let (target, soft_wrap_end) = self
                    .pointer_unit(point, unit)
                    .map_err(|_| EditError::InvalidSelection)?;
                let end = target.end;
                let segment = extend_unit(anchor.offset()?..anchor.end_offset()?, target);
                (segment, soft_wrap_end && segment.caret == end)
            }
        };
        if editor.document() != document
            || document.version() != version
            || editor.selection() != selection
        {
            return Err(EditError::StaleEditPlan);
        }
        editor
            .session()
            .set_selection(Selection::from_segments([segment], 0))?;
        editor.selection_changed_with_affinity(affinity);
        Ok(())
    }

    fn pointer_unit(
        &self,
        point: Point,
        unit: SelectionUnit,
    ) -> Result<(Range<usize>, bool), VisualLineError> {
        if matches!(unit, SelectionUnit::GutterLine) {
            // Focus and layout changes can restore estimated line heights.
            // Resolve the viewport before using its heights to pick a row.
            self.ensure_viewport_lines()?;
        }
        let scroll = self.offset.get();
        let y = self.clamp_selection_point(point).y + scroll.y;
        let index = self.heights.borrow().line_at(y);
        let line = self.try_line_layout(index)?;
        let display = &line.display;
        match unit {
            SelectionUnit::GutterLine => {
                let row = line
                    .layout
                    .line_index_at_y(y - self.heights.borrow().top(index))
                    .ok_or(VisualLineError::ConstructionInvalidated)?;
                let range = line.document_range_for_row(row);
                return Ok((range, row + 1 < line.layout.text_lines().len()));
            }
            SelectionUnit::Line => {
                return Ok((line.offset..display.last_document_line().end_offset(), false));
            }
            SelectionUnit::Word => {}
        }
        let local = Point::new(
            point.x + scroll.x - self.gutter_width(),
            y - self.heights.borrow().top(index),
        );
        let column = (line.layout.hit_test_point(local).text_position().max(0) as usize)
            .min(display.visual_length());
        let start = display
            .get_next_caret_position(
                column as isize + 1,
                LogicalDirection::Backward,
                CaretPositioningMode::WordStartOrSymbol,
                false,
            )?
            .unwrap_or(0);
        let end = display
            .get_next_caret_position(
                start as isize,
                LogicalDirection::Forward,
                CaretPositioningMode::WordBorderOrSymbol,
                false,
            )?
            .unwrap_or(display.visual_length());
        Ok((
            line.offset + display.get_relative_offset(start)
                ..line.offset + display.get_relative_offset(end),
            false,
        ))
    }

    pub(super) fn refresh_pointer_selection_after_scroll(&self) {
        if self.updating_pointer_selection.get()
            || self.pointer_selection.borrow().is_none()
            || self.pointer_selection_refresh_pending.replace(true)
        {
            return;
        }
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(area) = weak.upgrade() {
                    area.pointer_selection_refresh_pending.set(false);
                    let point = area
                        .pointer_selection
                        .borrow()
                        .as_ref()
                        .map(|drag| drag.point);
                    if let Some(point) = point {
                        area.extend_pointer_selection(point);
                    }
                }
            },
            DispatcherPriority::NORMAL,
        );
    }
}

fn extend_unit(anchor: Range<usize>, target: Range<usize>) -> SelectionSegment {
    if target.start < anchor.start {
        SelectionSegment::new(anchor.end.max(target.end), target.start)
    } else {
        SelectionSegment::new(anchor.start, anchor.end.max(target.end))
    }
}
