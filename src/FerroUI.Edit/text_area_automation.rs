use super::*;
use crate::automation::merge_ranges;
use crate::{TextDocument, VisualLineError};
use ferroui_controls::automation::provider::{ProviderError, TextScrollPlacement};
use std::ops::Range;

pub(crate) struct AutomationVisibility {
    pub(crate) hidden: Vec<Range<usize>>,
    unprojected: Vec<Range<usize>>,
}

impl AutomationVisibility {
    pub(crate) fn unprojected_at(&self, offset: usize) -> Option<&Range<usize>> {
        self.unprojected.get(self.unprojected.partition_point(|span| span.end <= offset))
            .filter(|span| span.contains(&offset))
    }
}

#[derive(Default)]
pub(super) struct AutomationFormatChanges {
    document: RefCell<Option<TextDocument>>,
    queued: Cell<bool>,
    delivering: Cell<bool>,
}

struct FormatDelivery<'a>(&'a TextArea);

impl Drop for FormatDelivery<'_> {
    fn drop(&mut self) {
        self.0.automation_format_changes.delivering.set(false);
        self.0.schedule_automation_format_change();
    }
}

impl TextArea {
    pub(super) fn notify_automation_format_change(&self, document: TextDocument) {
        if self.editor().is_some_and(|editor| editor.document() == document) {
            self.automation_format_changes.document.replace(Some(document));
            self.schedule_automation_format_change();
        }
    }

    pub(super) fn schedule_automation_format_change(&self) {
        let changes = &self.automation_format_changes;
        if !self.can_deliver_format_change()
            || changes.document.borrow().is_none()
            || changes.queued.replace(true)
        {
            return;
        }
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(view) = weak.upgrade() {
                    view.deliver_automation_format_change();
                }
            },
            DispatcherPriority::NORMAL,
        );
    }

    fn can_deliver_format_change(&self) -> bool {
        !self.rendering.get()
            && !self.visual_line_construction_in_progress()
            && !self.automation_format_changes.delivering.get()
    }

    fn deliver_automation_format_change(&self) {
        let changes = &self.automation_format_changes;
        changes.queued.set(false);
        if !self.can_deliver_format_change() {
            return;
        }
        let Some(document) = changes.document.take() else { return };
        let Some(editor) = self.editor().filter(|editor| editor.document() == document) else {
            return;
        };
        changes.delivering.set(true);
        let _delivery = FormatDelivery(self);
        editor.raise_automation_format_changed();
    }

    pub(crate) fn automation_line(
        &self,
        document: &TextDocument,
        offset: usize,
    ) -> Result<Rc<crate::VisualLine>, VisualLineError> {
        Ok(self.automation_layout(document, offset)?.display.clone())
    }

    /// Gets the projection containing the offset and the next visible source line.
    /// An explicitly hidden line may have no projected content of its own.
    pub(crate) fn automation_scan_line(
        &self,
        document: &TextDocument,
        offset: usize,
    ) -> Result<(Option<Rc<crate::VisualLine>>, usize), VisualLineError> {
        let logical = document.get_line_by_offset(offset)?;
        let projected = self.automation_projected_layout(document, logical.line_number() - 1)?
            .map(|line| line.display.clone());
        let last = projected.as_ref().map_or(logical.line_number(), |line| {
            line.last_document_line().line_number()
        });
        let next = self.heights.borrow().next_visible(last);
        let next_offset = source_line_offset(document, next)?;
        Ok((projected, next_offset))
    }

    pub(crate) fn automation_visibility(
        &self,
        document: &TextDocument,
    ) -> Result<AutomationVisibility, VisualLineError> {
        let mut hidden: Vec<_> = self.folds
            .borrow()
            .iter()
            .map(|fold| fold.start_offset()..fold.end_offset())
            .collect();
        let mut sections = Vec::new();
        if let Some(active) = self.explicit_line_collapses.borrow().as_ref() {
            active.extend_intervals(&mut sections);
        }
        let sections = merge_ranges(sections.into_iter().map(|(first, end)| first..end).collect());
        let version = document.version();
        let revision = self.visual_lines.revision.get();
        let collapse_revision = self.explicit_collapse_revision.get();
        let mut unprojected = Vec::new();
        for section in sections {
            let mut first = section.start;
            if let Some(line) = self.automation_projected_layout(document, first)? {
                // A custom multiline element may project the collapsed source
                // through its owner, including a suffix of the last source row.
                first = first.max(line.display.last_document_line().line_number());
            }
            if first < section.end {
                let span = source_line_offset(document, first)?..source_line_offset(document, section.end)?;
                if !span.is_empty() {
                    unprojected.push(span);
                }
            }
        }
        if document.version() != version || self.visual_lines.revision.get() != revision
            || self.explicit_collapse_revision.get() != collapse_revision
            || self.editor().is_none_or(|editor| editor.document() != *document)
        {
            return Err(VisualLineError::ConstructionInvalidated);
        }
        hidden.extend(unprojected.iter().cloned());
        Ok(AutomationVisibility { hidden: merge_ranges(hidden), unprojected })
    }

    pub(crate) fn automation_rows(
        &self,
        document: &TextDocument,
        offset: usize,
    ) -> Result<Vec<Range<usize>>, VisualLineError> {
        let logical = document.get_line_by_offset(offset)?;
        let Some(line) = self.automation_projected_layout(document, logical.line_number() - 1)? else {
            let start = logical.offset();
            return Ok(vec![start..start + logical.total_length()]);
        };
        let mut rows: Vec<_> = (0..line.layout.text_lines().len())
            .map(|index| line.document_range_for_row(index))
            .filter(|range| !range.is_empty())
            .collect();
        if rows.is_empty() {
            rows.push(line.document_range_for_row(0));
        }
        Ok(rows)
    }

    pub(crate) fn automation_scroll(
        &self,
        document: &TextDocument,
        span: Range<usize>,
        align_to_top: bool,
    ) -> Result<(), VisualLineError> {
        let Some(span) = self.automation_scroll_span(document, span)? else { return Ok(()) };
        let offset = if align_to_top || span.is_empty() { span.start } else { span.end - 1 };
        let index = self.automation_line_index(document, offset)?;
        let line = self.try_line_layout(index)?;
        let column = line
            .display
            .visual_offset(offset.saturating_sub(line.offset)) as i32;
        let row = line
            .layout
            .get_line_index_from_character_index(column, false) as usize;
        let bounds = line.layout.hit_test_text_position(column);
        let origin = line.layout.line_origin(row);
        let row_height = line.layout.text_lines().get(row).map_or(0.0, |row| row.height());
        let viewport = self.content_viewport();
        let mut scroll = self.offset.get();
        let x = bounds.x;
        if x < scroll.x {
            scroll.x = x;
        } else if x + 1.0 > scroll.x + viewport.width {
            scroll.x = x + 1.0 - viewport.width;
        }
        scroll.y = self.heights.borrow().top(index) + origin.y;
        if !align_to_top {
            scroll.y += row_height - viewport.height;
        }
        self.update_extent();
        self.set_offset(scroll);
        Ok(())
    }

    pub(crate) fn automation_scroll_to(
        &self,
        document: &TextDocument,
        span: Range<usize>,
        placement: TextScrollPlacement,
    ) -> Result<bool, ProviderError> {
        use TextScrollPlacement::*;

        let viewport = self.content_viewport();
        if viewport.is_empty() || !self.is_effectively_visible() || self.visual_root().is_none() {
            return Ok(false);
        }
        let point = match placement {
            Point(point) => {
                let Some(local) = self.visual_root().and_then(|root| root.transform_to_visual(self)) else {
                    return Ok(false);
                };
                let point = point.transform(local);
                if !viewport.contains_exclusive(point) {
                    return Ok(false);
                }
                Some(point - viewport.top_left())
            }
            _ => None,
        };

        let version = document.version();
        let revision = self.visual_lines.revision.get();
        let collapse_revision = self.explicit_collapse_revision.get();
        let Some(span) = self.automation_scroll_span(document, span)? else { return Ok(false) };
        // Placements toward the bottom use the last displayed character when
        // the entire range cannot fit. Other placements begin at the first.
        let bounds = match self.automation_scroll_bounds(document, span.clone(), viewport.height)? {
            Some(bounds) => bounds,
            None => {
                let anchor = if matches!(placement, Bottom | BottomRight) && !span.is_empty() {
                    span.end - 1
                } else {
                    span.start
                };
                let index = self.automation_line_index(document, anchor)?;
                let line = self.try_line_layout(index)?;
                let column = line.display.visual_offset(anchor.saturating_sub(line.offset)) as i32;
                let hit = line.layout.hit_test_text_position(column);
                Rect::new(hit.x.min(hit.right()), self.heights.borrow().top(index) + hit.y,
                    hit.width.abs().max(1.0), hit.height)
            }
        };

        self.update_extent();
        // Layout extensions and scroll notifications can reenter the editor.
        if document.version() != version || self.visual_lines.revision.get() != revision
            || self.explicit_collapse_revision.get() != collapse_revision
            || self.editor().is_none_or(|editor| editor.document() != *document)
            || self.content_viewport() != viewport
        {
            return Err(VisualLineError::ConstructionInvalidated.into());
        }

        let old = self.offset.get();
        let offset = if let Some(point) = point {
            Vector::new(bounds.left() - point.x, bounds.top() - point.y)
        } else {
            let x = match placement {
                TopLeft | Left => bounds.left(),
                BottomRight | Right => bounds.right() - viewport.width,
                _ => reveal_axis(old.x, bounds.left(), bounds.right(), viewport.width),
            };
            let y = match placement {
                TopLeft | Top => bounds.top(),
                BottomRight | Bottom => bounds.bottom() - viewport.height,
                _ => reveal_axis(old.y, bounds.top(), bounds.bottom(), viewport.height),
            };
            Vector::new(x, y)
        };
        self.set_offset(offset);
        let actual = self.offset.get();
        let visible = bounds.translate(Vector::new(viewport.x - actual.x, viewport.y - actual.y));
        Ok(!visible.intersect(viewport).is_empty())
    }

    fn automation_scroll_bounds(
        &self,
        document: &TextDocument,
        span: Range<usize>,
        viewport_height: f64,
    ) -> Result<Option<Rect>, VisualLineError> {
        if span.is_empty() {
            return Ok(None);
        }
        let mut index = self.automation_line_index(document, span.start)?;
        let last = self.automation_line_index(document, span.end - 1)?;
        {
            let heights = self.heights.borrow();
            if heights.top(last) - heights.top(index) >= viewport_height {
                return Ok(None);
            }
        }

        let mut bounds: Option<Rect> = None;
        while index <= last {
            let line = self.try_line_layout(index)?;
            let top = self.heights.borrow().top(index);
            let column = |offset: usize| -> i32 {
                line.display.visual_offset(offset.saturating_sub(line.offset)) as i32
            };
            let start = column(span.start);
            let end = column(span.end);
            for rect in line.layout.hit_test_text_range(start, end - start) {
                let rect = rect.translate(Vector::new(0.0, top));
                let joined = bounds.map_or(rect, |bounds| bounds.union(rect));
                if joined.height > viewport_height {
                    return Ok(None);
                }
                bounds = Some(joined);
            }
            index = self.heights.borrow().next_visible(line.display.last_document_line().line_number());
        }
        Ok(bounds.filter(|bounds| !bounds.is_empty()))
    }

    fn automation_scroll_span(
        &self,
        document: &TextDocument,
        span: Range<usize>,
    ) -> Result<Option<Range<usize>>, VisualLineError> {
        if span.is_empty() {
            let line = document.get_line_by_offset(span.start)?;
            return Ok(self.automation_projected_layout(document, line.line_number() - 1)?
                .map(|_| span));
        }
        let visibility = self.automation_visibility(document)?;
        let start = visibility.unprojected_at(span.start).map_or(span.start, |hidden| hidden.end);
        let end = visibility.unprojected_at(span.end - 1).map_or(span.end, |hidden| hidden.start);
        Ok((start < end).then_some(start..end))
    }

    fn automation_projected_layout(
        &self,
        document: &TextDocument,
        index: usize,
    ) -> Result<Option<Rc<CachedVisualLine>>, VisualLineError> {
        let owner = self.heights.borrow().visible_owner(index);
        if owner >= document.line_count() {
            return Ok(None);
        }
        let line = self.automation_layout_at(owner)?;
        Ok((index < line.display.last_document_line().line_number()).then_some(line))
    }

    fn automation_line_index(
        &self,
        document: &TextDocument,
        offset: usize,
    ) -> Result<usize, VisualLineError> {
        let line = document.get_line_by_offset(offset)?;
        Ok(self.heights.borrow().visible_owner(line.line_number() - 1))
    }

    fn automation_layout(
        &self,
        document: &TextDocument,
        offset: usize,
    ) -> Result<Rc<CachedVisualLine>, VisualLineError> {
        let index = self.automation_line_index(document, offset)?;
        self.automation_layout_at(index)
    }

    fn automation_layout_at(&self, index: usize) -> Result<Rc<CachedVisualLine>, VisualLineError> {
        let had_layout = self.cache.borrow().contains_key(&index);
        let line = self.try_line_layout(index)?;
        if !had_layout {
            let top = self.heights.borrow().top(index) - self.offset.get().y;
            if top >= self.viewport.get().height || top + line.layout.height() <= 0.0 {
                // Format and Line requests can scan the document independently
                // of rendering. Keep only the retained viewport's layouts.
                self.cache.borrow_mut().remove(&index);
            }
        }
        Ok(line)
    }
}

fn source_line_offset(document: &TextDocument, index: usize) -> Result<usize, VisualLineError> {
    Ok(if index < document.line_count() {
        document.get_line_by_number(index + 1)?.offset()
    } else {
        document.text_length()
    })
}

fn reveal_axis(current: f64, start: f64, end: f64, viewport: f64) -> f64 {
    let trailing = end - viewport;
    current.clamp(start.min(trailing), start.max(trailing))
}
