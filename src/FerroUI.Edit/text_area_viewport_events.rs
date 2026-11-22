use super::{CachedVisualLine, TextArea};
use crate::VisibleEditorLine;
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{Rect, Size, Vector};
use ferroui_edit_core::document::DocumentVersion;
use ferroui_edit_core::editing::Selection;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

type VisualLinesHandler = dyn Fn(&TextArea, &[VisibleEditorLine]);
type ScrollOffsetHandler = dyn Fn(&TextArea, Vector, Vector);

#[derive(PartialEq)]
struct ViewportKey {
    version: Option<DocumentVersion>,
    revision: u64,
    explicit_collapse_revision: u64,
    offset: Vector,
    size: Size,
    foreground_selection: Option<Selection>,
}

struct VisibleViewport {
    key: ViewportKey,
    lines: Vec<(Weak<CachedVisualLine>, Rect)>,
}

#[derive(Default)]
pub(super) struct ViewportEvents {
    visible: RefCell<Option<VisibleViewport>>,
    visual_pending: Cell<bool>,
    visual_job_queued: Cell<bool>,
    publishing_visual_change: Cell<bool>,
    visual: Rc<HandlerList<VisualLinesHandler>>,
    scroll: Rc<HandlerList<ScrollOffsetHandler>>,
}

struct PublishingVisualChange<'a>(&'a Cell<bool>);

impl Drop for PublishingVisualChange<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

fn subscribe<F: ?Sized + 'static>(handlers: &Rc<HandlerList<F>>, handler: Rc<F>) -> Subscription {
    let token = handlers.add(handler);
    let owner = Rc::downgrade(handlers);
    Disposable::create(move || {
        if let Some(handlers) = owner.upgrade() {
            handlers.remove(token);
        }
    })
    .into()
}

impl TextArea {
    /// Observes a completed viewport update. Layout and rendering deliver
    /// notifications through the UI dispatcher. The slice belongs to that update.
    pub fn visual_lines_changed(
        &self,
        handler: impl Fn(&TextArea, &[VisibleEditorLine]) + 'static,
    ) -> Subscription {
        subscribe(&self.viewport_events.visual, Rc::new(handler))
    }

    /// Observes a change to the effective scroll offset in document coordinates.
    pub fn scroll_offset_changed(
        &self,
        handler: impl Fn(&TextArea, Vector, Vector) + 'static,
    ) -> Subscription {
        subscribe(&self.viewport_events.scroll, Rc::new(handler))
    }

    pub fn scroll_offset(&self) -> Vector {
        self.offset.get()
    }

    /// Whether the published visible rows still match the editor view.
    pub fn visual_lines_valid(&self) -> bool {
        self.current_visible_viewport().is_some()
    }

    /// Returns the visible rows from the current viewport without formatting.
    /// Use `ensure_visual_lines` to build a snapshot after invalidation.
    pub fn visible_visual_lines(&self) -> Option<Vec<VisibleEditorLine>> {
        self.current_visible_viewport()?
            .lines
            .iter()
            .map(|(line, bounds)| {
                Some(VisibleEditorLine {
                    line: line.upgrade()?.display.clone(),
                    bounds: *bounds,
                })
            })
            .collect()
    }

    fn viewport_key(&self) -> ViewportKey {
        let editor = self.editor();
        ViewportKey {
            version: editor.as_ref().map(|editor| editor.document().version()),
            revision: self.visual_lines.revision.get(),
            explicit_collapse_revision: self.explicit_collapse_revision.get(),
            offset: self.offset.get(),
            size: self.viewport.get(),
            foreground_selection: editor
                .and_then(|editor| self.selection_foreground().map(|_| editor.selection())),
        }
    }

    fn current_visible_viewport(&self) -> Option<std::cell::Ref<'_, VisibleViewport>> {
        let key = self.viewport_key();
        std::cell::Ref::filter_map(self.viewport_events.visible.borrow(), |published| {
            let current = published.as_ref().filter(|old| old.key == key)?;
            let cache = self.cache.borrow();
            let heights = self.heights.borrow();
            current
                .lines
                .iter()
                .all(|(line, bounds)| {
                    line.upgrade().is_some_and(|line| {
                        let index = line.display.first_document_line().line_number() - 1;
                        cache
                            .get(&index)
                            .is_some_and(|cached| Rc::ptr_eq(cached, &line))
                            && !heights.is_hidden(index)
                            && heights.top(index) - key.offset.y == bounds.y
                    })
                })
                .then_some(current)
        })
        .ok()
    }

    pub(super) fn publish_visible_lines<'a>(
        &self,
        lines: impl Iterator<Item = (&'a Rc<CachedVisualLine>, Rect)> + Clone,
    ) {
        let key = self.viewport_key();
        let slot = &self.viewport_events.visible;
        let unchanged = slot.borrow().as_ref().is_some_and(|old| {
            old.key == key
                && old
                    .lines
                    .iter()
                    .map(|(line, bounds)| (line.as_ptr(), *bounds))
                    .eq(lines
                        .clone()
                        .map(|(line, bounds)| (Rc::as_ptr(line), bounds)))
        });
        if unchanged {
            return;
        }
        slot.replace(Some(VisibleViewport {
            key,
            lines: lines
                .map(|(line, bounds)| (Rc::downgrade(line), bounds))
                .collect(),
        }));
        if !self.viewport_events.visual.is_empty() {
            self.viewport_events.visual_pending.set(true);
            self.schedule_visible_lines_changed();
        }
    }

    fn schedule_visible_lines_changed(&self) {
        let events = &self.viewport_events;
        if !events.visual_pending.get() || events.visual_job_queued.replace(true) {
            return;
        }
        let weak = self.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                if let Some(view) = weak.upgrade() {
                    view.viewport_events.visual_job_queued.set(false);
                    view.deliver_visible_lines_changed();
                }
            },
            DispatcherPriority::NORMAL,
        );
    }

    pub(super) fn discard_visible_viewport(&self) {
        self.viewport_events.visible.take();
    }

    pub(super) fn deliver_visible_lines_changed(&self) {
        let events = &self.viewport_events;
        if self.rendering.get()
            || events.publishing_visual_change.get()
            || !events.visual_pending.replace(false)
        {
            return;
        }
        if let Some(lines) = self.visible_visual_lines() {
            events.publishing_visual_change.set(true);
            let _publication = PublishingVisualChange(&events.publishing_visual_change);
            for (_, handler) in events.visual.snapshot().iter() {
                handler(self, &lines);
            }
        }
        // A handler can build a newer snapshot and pump its dispatcher job
        // before the current publication has finished.
        self.schedule_visible_lines_changed();
    }

    pub(super) fn refresh_arranged_viewport(&self) {
        if !self.viewport_events.visual.is_empty() {
            if let Err(error) = self.ensure_viewport_lines() {
                if let Some(editor) = self.editor() {
                    if !matches!(error, crate::VisualLineError::ConstructionInvalidated) {
                        editor.report_error(error.to_string());
                    }
                }
            }
        }
    }

    pub(super) fn raise_scroll_offset_changed(&self, previous: Vector, current: Vector) {
        for (_, handler) in self.viewport_events.scroll.snapshot().iter() {
            handler(self, previous, current);
        }
    }
}
