use super::*;
use crate::{EditorNavigation, TextDocument, VisualLineError};
use ferroui_edit_core::document::{CaretPositioningMode, LogicalDirection, TextUtilities};
use ferroui_edit_core::editing::EditError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NavigationCaret {
    pub(crate) offset: usize,
    pub(super) virtual_space: usize,
    pub(super) at_end_of_row: bool,
}

impl NavigationCaret {
    pub(crate) fn at_offset(offset: usize) -> Self {
        Self {
            offset,
            virtual_space: 0,
            at_end_of_row: false,
        }
    }

    pub(super) fn from_segment(segment: SelectionSegment, at_end_of_row: bool) -> Self {
        Self {
            offset: segment.caret,
            virtual_space: segment.caret_virtual_space,
            at_end_of_row,
        }
    }
}

/// The active caret may be on a continuation row. The rectangle's editable
/// segments are still projected onto each logical line's first formatted row.
#[derive(Clone, Copy, PartialEq)]
pub(super) struct BoxCaret {
    pub(super) anchor: (usize, f64),
    pub(super) position: NavigationCaret,
    pub(super) projected_primary: SelectionSegment,
}

/// The caret placed by a context click inside the active selection. The
/// logical segment retains its original endpoints for selection editing.
#[derive(Clone, Copy)]
pub(super) struct ContextCaret {
    pub(super) position: NavigationCaret,
    pub(super) segment: SelectionSegment,
}

impl TextArea {
    pub(super) fn active_context_caret(&self, selection: &Selection) -> Option<ContextCaret> {
        self.context_caret
            .get()
            .filter(|caret| caret.segment == selection.primary())
    }

    pub(super) fn active_box_caret(&self, selection: &Selection) -> Option<BoxCaret> {
        self.box_caret.get().filter(|state| {
            selection.is_rectangular() && state.projected_primary == selection.primary()
        })
    }

    pub(crate) fn navigation_caret(&self, selection: &Selection) -> NavigationCaret {
        self.projected_caret(selection).unwrap_or_else(|| {
            NavigationCaret::from_segment(selection.primary(), self.caret_at_end_of_line.get())
        })
    }

    pub(super) fn projected_caret(&self, selection: &Selection) -> Option<NavigationCaret> {
        self.active_context_caret(selection)
            .map(|caret| caret.position)
            .or_else(|| self.active_box_caret(selection).map(|caret| caret.position))
    }

    pub(crate) fn projected_caret_offset(&self) -> Option<usize> {
        Some(self.projected_caret(&self.editor()?.selection())?.offset)
    }

    fn caret_row(&self, line: &CachedVisualLine, caret: NavigationCaret) -> usize {
        let visual = line.display.visual_offset(caret.offset - line.offset);
        if caret.virtual_space > 0 {
            line.layout.text_lines().len().saturating_sub(1)
        } else {
            line.layout
                .get_line_index_from_character_index(visual as i32, caret.at_end_of_row)
                .max(0) as usize
        }
    }

    pub(super) fn line_boundary(
        &self,
        document: &TextDocument,
        caret: NavigationCaret,
        movement: CaretMovement,
    ) -> Result<NavigationCaret, VisualLineError> {
        let logical = document.get_line_by_offset(caret.offset)?;
        let start = movement == CaretMovement::LineStart;
        if self.viewport.get().width <= 0.0 {
            let offset = if start {
                let line = document
                    .create_snapshot()
                    .slice(logical.offset(), logical.length())?;
                let text_start = TextUtilities::get_next_caret_position(
                    &line,
                    -1,
                    LogicalDirection::Forward,
                    CaretPositioningMode::WordStart,
                )?
                .unwrap_or(logical.length())
                    + logical.offset();
                if caret.offset == text_start && caret.virtual_space == 0 {
                    logical.offset()
                } else {
                    text_start
                }
            } else {
                logical.end_offset()
            };
            return Ok(NavigationCaret {
                offset,
                virtual_space: 0,
                at_end_of_row: !start,
            });
        }
        let index = self
            .heights
            .borrow()
            .visible_owner(logical.line_number() - 1);
        let line = self.try_line_layout(index)?;
        let row = line
            .layout
            .text_lines()
            .get(self.caret_row(&line, caret))
            .ok_or(VisualLineError::ConstructionInvalidated)?;
        let row_start = row.first_text_source_index().max(0) as usize;
        let display = &line.display;
        let column = if start {
            let first = if row_start == 0 {
                display
                    .get_next_caret_position(
                        -1,
                        LogicalDirection::Forward,
                        CaretPositioningMode::WordStart,
                        false,
                    )?
                    .unwrap_or(0)
            } else {
                row_start
            };
            let old = display.visual_offset(caret.offset - line.offset);
            if old == first && caret.virtual_space == 0 {
                0
            } else {
                first
            }
        } else {
            row_start + (row.length() - row.new_line_length()).max(0) as usize
        };
        let offset = line.offset + display.get_relative_offset(column.min(display.visual_length()));
        Ok(NavigationCaret {
            offset,
            virtual_space: 0,
            at_end_of_row: !start,
        })
    }

    pub(super) fn move_to_line_boundary(
        &self,
        movement: CaretMovement,
        extend: bool,
    ) -> Result<bool, EditError> {
        let editor = self.editor().ok_or(EditError::InvalidSelection)?;
        let document = editor.document();
        let version = document.version();
        let revision = self.visual_lines.revision.get();
        let selection = editor.selection();
        let single_caret = selection.is_rectangular() && !extend;
        let mut primary_affinity = false;
        let mut segments = Vec::with_capacity(selection.segments().len());
        for (index, segment) in selection.segments().iter().enumerate() {
            let primary = index == selection.primary_index();
            if single_caret && !primary {
                continue;
            }
            let caret = if primary {
                self.navigation_caret(&selection)
            } else {
                NavigationCaret::from_segment(*segment, false)
            };
            let target = match self.line_boundary(&document, caret, movement) {
                Ok(target) => target,
                Err(error) => {
                    editor.report_error(error.to_string());
                    return Ok(caret.at_end_of_row);
                }
            };
            if primary {
                primary_affinity = target.at_end_of_row;
            }
            let (anchor, virtual_anchor) = if extend {
                segment.anchor_tuple()
            } else {
                (target.offset, 0)
            };
            segments.push(SelectionSegment::with_virtual_space(
                anchor,
                virtual_anchor,
                target.offset,
                0,
            ));
        }
        if editor.document() != document
            || document.version() != version
            || editor.selection() != selection
            || self.visual_lines.revision.get() != revision
        {
            return Err(EditError::StaleEditPlan);
        }
        let primary = if single_caret {
            0
        } else {
            selection.primary_index()
        };
        editor
            .session()
            .set_selection(Selection::from_segments(segments, primary))?;
        Ok(primary_affinity)
    }

    fn vertical_caret(
        &self,
        document: &TextDocument,
        caret: NavigationCaret,
        x: f64,
        upward: bool,
        page: bool,
        allow_virtual: bool,
    ) -> Result<NavigationCaret, VisualLineError> {
        let index = self.line_owner(document, caret.offset)?;
        let line = self.try_line_layout(index)?;
        let row = self.caret_row(&line, caret);
        let (line, row) = if page {
            let source = line
                .layout
                .text_lines()
                .get(row)
                .ok_or(VisualLineError::ConstructionInvalidated)?;
            let y = self.heights.borrow().top(index)
                + line.layout.line_origin(row).y
                + source.height() * 0.5;
            let distance = self.viewport.get().height.max(self.line_height());
            let target_y = y + if upward { -distance } else { distance };
            let target = self.heights.borrow().line_at(target_y);
            let line = self.try_line_layout(target)?;
            let row = line
                .layout
                .line_index_at_y(target_y - self.heights.borrow().top(target))
                .ok_or(VisualLineError::ConstructionInvalidated)?;
            (line, row)
        } else if upward && row > 0 {
            (line, row - 1)
        } else if !upward && row + 1 < line.layout.text_lines().len() {
            (line, row + 1)
        } else {
            let next = if upward {
                let Some(previous) = index.checked_sub(1) else {
                    return Ok(caret);
                };
                self.heights.borrow().visible_owner(previous)
            } else {
                self.heights.borrow().next_visible(index + 1)
            };
            if next >= document.line_count() {
                return Ok(caret);
            }
            let line = self.try_line_layout(next)?;
            let row = if upward {
                line.layout.text_lines().len().saturating_sub(1)
            } else {
                0
            };
            (line, row)
        };

        self.caret_at_row_x(&line, row, x, allow_virtual, true)
    }

    pub(super) fn caret_at_row_x(
        &self,
        line: &CachedVisualLine,
        row: usize,
        x: f64,
        allow_virtual: bool,
        stop_before_wrap: bool,
    ) -> Result<NavigationCaret, VisualLineError> {
        let rows = line.layout.text_lines();
        let target = rows
            .get(row)
            .ok_or(VisualLineError::ConstructionInvalidated)?;
        let last = row + 1 == rows.len();
        let x = x - line.layout.line_origin(row).x;
        let right = target.start() + target.width_including_trailing_whitespace();
        if last && allow_virtual && x > right {
            return Ok(NavigationCaret {
                offset: line.offset + line.length,
                virtual_space: ((x - right) / self.wide_space_width()).round() as usize,
                at_end_of_row: false,
            });
        }
        let start = target.first_text_source_index().max(0) as usize;
        let end = start + (target.length() - target.new_line_length()).max(0) as usize;
        let hit = target.get_character_hit_from_distance(x);
        let mut visual = (hit.first_character_index() + hit.trailing_length()).max(0) as usize;
        visual = visual.clamp(start, end).min(line.display.visual_length());
        if stop_before_wrap && !last && visual == end {
            // Keep a vertical move on its destination row and respect the
            // caret stops of shaped graphemes and generated elements.
            visual = line
                .display
                .get_next_caret_position(
                    end as isize,
                    LogicalDirection::Backward,
                    CaretPositioningMode::Normal,
                    false,
                )?
                .filter(|&column| column >= start)
                .unwrap_or(end);
        }
        Ok(NavigationCaret {
            offset: line.offset + line.display.get_relative_offset(visual),
            virtual_space: 0,
            at_end_of_row: !last && visual == end,
        })
    }

    pub(super) fn line_owner(
        &self,
        document: &TextDocument,
        offset: usize,
    ) -> Result<usize, VisualLineError> {
        let line = document.get_line_by_offset(offset)?;
        Ok(self.heights.borrow().visible_owner(line.line_number() - 1))
    }

    pub(super) fn navigation_caret_at_point(
        &self,
        point: Point,
        allow_virtual: bool,
    ) -> NavigationCaret {
        let (offset, virtual_space) = self.selection_position_at_point(point, allow_virtual);
        NavigationCaret {
            offset,
            virtual_space,
            at_end_of_row: virtual_space == 0
                && self
                    .position_at_point(point, false)
                    .is_some_and(|position| position.is_at_end_of_line),
        }
    }

    pub(crate) fn navigate(
        &self,
        editor: &TextEditor,
        navigation: EditorNavigation,
    ) -> Result<(), EditError> {
        if self.preedit.borrow().is_some() {
            return Ok(());
        }
        let session = editor.session();
        let affinity = match navigation {
            EditorNavigation::Rectangle(_) => return self.move_box_caret(editor, navigation),
            EditorNavigation::Caret { extend_selection: true, .. }
            | EditorNavigation::Page { extend_selection: true, .. }
                if session.selection().is_rectangular() =>
            {
                return self.move_box_caret(editor, navigation);
            }
            EditorNavigation::Caret { movement, extend_selection } => match movement {
                CaretMovement::LineUp | CaretMovement::LineDown
                    if self.viewport.get().width > 0.0 =>
                {
                    return self.move_vertical_caret(editor, movement == CaretMovement::LineUp, false, extend_selection);
                }
                CaretMovement::LineStart | CaretMovement::LineEnd => {
                    self.move_to_line_boundary(movement, extend_selection)?
                }
                _ => {
                    if !self.move_caret_over_visual_elements(movement, extend_selection) {
                        session.move_caret(movement, extend_selection);
                    }
                    false
                }
            },
            EditorNavigation::Page { upward, extend_selection } => {
                if self.viewport.get().width > 0.0 {
                    return self.move_vertical_caret(editor, upward, true, extend_selection);
                }
                let direction = if upward { -1.0 } else { 1.0 };
                let rect = self.cursor_rectangle();
                let offset = self.offset_at_point(Point::new(
                    rect.x, rect.y + direction * self.viewport.get().height,
                ));
                session.set_selection(if extend_selection {
                    Selection::simple(session.selection().primary().anchor, offset)
                } else {
                    Selection::empty(offset)
                })?;
                false
            }
        };
        editor.selection_changed_with_affinity(affinity);
        Ok(())
    }

    fn move_vertical_caret(
        &self,
        editor: &TextEditor,
        upward: bool,
        page: bool,
        extend_selection: bool,
    ) -> Result<(), EditError> {
        let session = editor.session();
        let selection = session.selection();
        let document = editor.document();
        let version = document.version();
        let revision = self.visual_lines.revision.get();
        let caret = self.navigation_caret(&selection);
        let x = self.desired_x.get().unwrap_or_else(|| self.caret_x(caret));
        let current = selection.primary();
        let target = self.vertical_caret(
            &document,
            caret,
            x,
            upward,
            page,
            editor.options().enable_virtual_space
                || caret.virtual_space > 0
                || selection.is_rectangular(),
        );
        if editor.document() != document
            || document.version() != version
            || editor.selection() != selection
            || self.visual_lines.revision.get() != revision
        {
            return Err(EditError::StaleEditPlan);
        }
        let target = match target {
            Ok(target) => target,
            Err(error) => {
                editor.report_error(error.to_string());
                return Ok(());
            }
        };
        let (anchor, anchor_extra) = if extend_selection {
            (current.anchor, current.anchor_virtual_space)
        } else {
            (target.offset, target.virtual_space)
        };
        let segment = SelectionSegment::with_virtual_space(
            anchor,
            anchor_extra,
            target.offset,
            target.virtual_space,
        );
        session.set_selection(Selection::from_segments([segment], 0))?;
        self.desired_x.set(Some(x));
        let was_moving = self.moving_vertically.replace(true);
        editor.selection_changed_with_affinity(target.at_end_of_row);
        self.moving_vertically.set(was_moving);
        Ok(())
    }

    fn move_box_caret(
        &self,
        editor: &TextEditor,
        navigation: EditorNavigation,
    ) -> Result<(), EditError> {
        let document = editor.document();
        let version = document.version();
        let revision = self.visual_lines.revision.get();
        let selection = editor.selection();
        let saved = self.active_box_caret(&selection);
        let caret = self.navigation_caret(&selection);
        let anchor = saved.map_or_else(
            || self.rectangle_anchor_for_selection(&selection),
            |saved| saved.anchor,
        );
        let (movement, page) = match navigation {
            EditorNavigation::Caret { movement, .. } | EditorNavigation::Rectangle(movement) => {
                (movement, false)
            }
            EditorNavigation::Page { upward, .. } => (
                if upward {
                    CaretMovement::LineUp
                } else {
                    CaretMovement::LineDown
                },
                true,
            ),
        };
        let vertical = matches!(movement, CaretMovement::LineUp | CaretMovement::LineDown);
        let desired_x =
            vertical.then(|| self.desired_x.get().unwrap_or_else(|| self.caret_x(caret)));
        let target = if let Some(x) = desired_x {
            self.vertical_caret(
                &document,
                caret,
                x,
                movement == CaretMovement::LineUp,
                page,
                true,
            )
        } else {
            match movement {
                CaretMovement::DocumentStart | CaretMovement::DocumentEnd => Ok(NavigationCaret {
                    offset: if movement == CaretMovement::DocumentStart {
                        0
                    } else {
                        document.text_length()
                    },
                    virtual_space: 0,
                    at_end_of_row: false,
                }),
                CaretMovement::LineStart | CaretMovement::LineEnd => {
                    self.line_boundary(&document, caret, movement)
                }
                _ => self.horizontal_caret(&document, caret, movement, true),
            }
        };
        let target = match target {
            Ok(target) => target,
            Err(error) => {
                editor.report_error(error.to_string());
                return Ok(());
            }
        };
        let index = self
            .line_owner(&document, target.offset)
            .map_err(|_| EditError::InvalidSelection)?;
        let target_x = self.caret_x(target);
        if editor.document() != document
            || document.version() != version
            || editor.selection() != selection
            || self.visual_lines.revision.get() != revision
        {
            return Err(EditError::StaleEditPlan);
        }
        self.select_rectangle_to(anchor, (index, target_x), target, desired_x)
    }

    pub(super) fn caret_x(&self, caret: NavigationCaret) -> f64 {
        let Some(line) = self
            .editor()
            .and_then(|editor| self.line_owner(&editor.document(), caret.offset).ok())
            .and_then(|index| self.line_layout(index))
        else {
            return 0.0;
        };
        self.caret_x_in_line(&line, caret)
    }

    pub(super) fn caret_x_in_line(&self, line: &CachedVisualLine, caret: NavigationCaret) -> f64 {
        let visual = line.display.visual_offset(caret.offset - line.offset);
        if caret.virtual_space > 0 {
            Self::virtual_space_bounds(&line.layout).x
                + caret.virtual_space as f64 * self.wide_space_width()
        } else {
            Self::caret_bounds(&line.layout, visual, caret.at_end_of_row).x
        }
    }
}
