//! Canvas feature orchestration. Native measurements are cached until layout, model,
//! selection or provider registration changes. Pointer drafts never mutate the document.
use super::*;
use crate::{LayoutEditPreview, LayoutHelperSnapshot};
use ferroui_designer_tools::{Axis, Point as ToolPoint};

pub(super) struct HelperCache {
    uri: String,
    revision: u64,
    generation: u64,
    registry: u64,
    epoch: u64,
    syntax: Arc<Ast>,
    selection: Vec<NodeId>,
    snapshot: Option<LayoutHelperSnapshot>,
}

impl DesignSurface {
    pub fn layout_helpers_visible(&self) -> bool {
        self.helpers_visible.get()
    }

    /// Changes only editor presentation. Hiding helpers cancels pending canvas drafts.
    pub fn set_layout_helpers_visible(&self, visible: bool) {
        if self.helpers_visible.replace(visible) != visible {
            self.clear_pending();
            self.helper_focus.set(None);
            self.helper_hover.set(None);
            self.invalidate_layout_helpers();
            self.invalidate_visual();
        }
    }

    /// Invalidates measurements without disturbing a captured gesture or its source guard.
    pub fn invalidate_layout_helpers(&self) {
        self.helper_cache.borrow_mut().take();
    }

    /// Current selection's panel features in panel coordinates with an artboard transform.
    pub fn layout_helper_snapshot(&self) -> Option<LayoutHelperSnapshot> {
        if !self.helpers_visible.get() || self.interactive.get() {
            return None;
        }
        let session = self.session()?;
        let session = session.try_borrow().ok()?;
        if !session.can_manipulate()
            || session
                .selection()
                .iter()
                .any(|node| session.ensure_layer_manipulable(*node).is_err())
        {
            return None;
        }
        let registry = session.layout_tools().revision();
        let epoch = self.helper_epoch.get();
        if let Some(cache) = self.helper_cache.borrow().as_ref() {
            if cache.uri == session.document_uri()
                && cache.revision == session.document().revision()
                && cache.generation == session.generation()
                && cache.registry == registry
                && cache.epoch == epoch
                && cache.selection == session.selection()
                && std::ptr::eq(cache.syntax.as_ref(), session.document().ast())
            {
                return cache.snapshot.clone();
            }
        }
        let snapshot = session.selection_layout_helpers();
        *self.helper_cache.borrow_mut() = Some(HelperCache {
            uri: session.document_uri().to_owned(),
            revision: session.document().revision(),
            generation: session.generation(),
            registry,
            epoch,
            syntax: session.document().ast_snapshot(),
            selection: session.selection().to_vec(),
            snapshot: snapshot.clone(),
        });
        snapshot
    }

    /// The currently displayed drop/handle proposal, useful to hosts and native UI tests.
    pub fn layout_edit_preview(&self) -> Option<Rc<LayoutEditPreview>> {
        self.helper_preview.borrow().clone()
    }

    pub(super) fn update_layout_preview(&self, position: Point, modifiers: KeyModifiers) {
        if !self.helpers_visible.get() || self.interactive.get() || !self.pending_is_current() {
            self.helper_preview.borrow_mut().take();
            self.helper_error.borrow_mut().take();
            return;
        }
        let Some(session) = self.session() else {
            return;
        };
        let session = session.borrow();
        let previous = self.helper_preview.borrow().clone();
        let result = if let Some(source) = self.insert.borrow().as_ref() {
            if modifiers.contains(KeyModifiers::ALT)
                || !Rect::from_size(self.bounds().size()).contains(position)
            {
                Ok(None)
            } else {
                session
                    .preview_layout_insert_cached(source, position, previous.as_deref())
                    .map(Some)
            }
        } else if let Some(gesture) = self.gesture.borrow().as_ref() {
            let raw_delta = position - gesture.start;
            if raw_delta.x.abs() + raw_delta.y.abs() <= 3.0 / self.zoom.get() {
                Ok(None)
            } else {
                match gesture.handle {
                    Handle::Layout(index) => gesture
                        .helper_origin
                        .as_ref()
                        .ok_or_else(|| "Layout handle snapshot is unavailable".to_owned())
                        .and_then(|snapshot| {
                            session.preview_layout_handle(
                                snapshot,
                                index,
                                Point::new(raw_delta.x, raw_delta.y),
                            )
                        })
                        .map(Some),
                    Handle::Move if !modifiers.contains(KeyModifiers::ALT) => {
                        let point = self.snap_gesture(gesture, position, modifiers);
                        let delta = point - gesture.start;
                        let nodes = gesture
                            .rectangles
                            .iter()
                            .map(|(id, _)| *id)
                            .collect::<Vec<_>>();
                        session.preview_layout_move_cached(
                            &nodes,
                            point,
                            Point::new(delta.x, delta.y),
                            previous.as_deref(),
                        )
                    }
                    _ => Ok(None),
                }
            }
        } else {
            Ok(None)
        };
        match result {
            Ok(preview) => {
                *self.helper_preview.borrow_mut() = preview.map(Rc::new);
                self.helper_error.borrow_mut().take();
            }
            Err(error) => {
                self.helper_preview.borrow_mut().take();
                *self.helper_error.borrow_mut() = Some(error);
            }
        }
    }

    pub(super) fn update_helper_hover(&self, position: Point) {
        if self.gesture.borrow().is_some() || self.insert.borrow().is_some() {
            return;
        }
        let hover = self
            .layout_helper_snapshot()
            .and_then(|snapshot| crate::hover_layout_handle(&snapshot, position, self.zoom.get()));
        if self.helper_hover.replace(hover) != hover {
            self.invalidate_visual();
        }
    }

    pub(super) fn render_layout_feedback(&self, context: &mut DrawingContext) {
        if !self.helpers_visible.get() {
            return;
        }
        let preview = self.helper_preview.borrow().clone();
        let snapshot = if self.pending_is_current() {
            preview
                .as_ref()
                .map(|preview| preview.helpers.clone())
                .or_else(|| self.layout_helper_snapshot())
        } else {
            self.layout_helper_snapshot()
        };
        if let Some(snapshot) = snapshot {
            let active = self
                .gesture
                .borrow()
                .as_ref()
                .and_then(|gesture| match gesture.handle {
                    Handle::Layout(index) => Some(index),
                    _ => None,
                })
                .or(self.helper_focus.get());
            crate::render_layout_helpers_with_state(
                context,
                &snapshot,
                self.zoom.get(),
                &crate::LayoutHelperRenderState {
                    viewport: Some(Rect::from_size(self.bounds().size())),
                    hovered_handle: self.helper_hover.get(),
                    active_handle: active,
                },
            );
        }
        if let Some(error) = self.helper_error.borrow().as_ref() {
            let zoom = self.zoom.get();
            let text = FormattedText::new(
                error,
                CultureInfo::current_culture(),
                FlowDirection::LeftToRight,
                Typeface::default(),
                11.0 / zoom,
                Some(SolidColorBrush::from_uint32(0xfff48b95).into()),
            );
            context.draw_text(&text, Point::new(12.0 / zoom, 12.0 / zoom));
        }
    }

    pub(super) fn capture_drag_visuals(&self) {
        let mut gesture = self.gesture.borrow_mut();
        let Some(gesture) = gesture.as_mut() else {
            return;
        };
        if gesture.ghost_capture_attempted
            || !matches!(gesture.handle, Handle::Move | Handle::Resize(_))
        {
            return;
        }
        let delta = gesture.current - gesture.start;
        if (delta.x.abs() + delta.y.abs()) * self.zoom.get() <= 3.0 {
            return;
        }
        gesture.ghost_capture_attempted = true;
        let Some(session) = self.session() else {
            return;
        };
        let session = session.borrow();
        let nodes = gesture
            .rectangles
            .iter()
            .map(|(node, _)| *node)
            .collect::<Vec<_>>();
        let Ok(nodes) = ferroui_designer_tools::outermost(session.document(), &nodes) else {
            return;
        };
        let mut pixels = 0;
        // At most 32 MiB of RGBA pixels and 8 controls are retained by a pointer gesture.
        for node in nodes.into_iter().take(8) {
            let Some(visual) = session.preview.visual(node) else {
                continue;
            };
            let Some(transform) = visual.transform_to_visual(&self.to_ref().upcast()) else {
                continue;
            };
            if let Some(ghost) =
                crate::DragVisualGhost::capture(&visual, transform, self.zoom.get())
            {
                pixels += ghost.pixel_count();
                if pixels > 8 * 1024 * 1024 {
                    break;
                }
                gesture.ghosts.push(ghost);
            }
        }
    }

    pub(super) fn render_drag_visuals(&self, context: &mut DrawingContext) {
        let gesture = self.gesture.borrow();
        let Some(gesture) = gesture.as_ref() else {
            return;
        };
        if !self.pending_is_current() {
            return;
        }
        let adjustment = match gesture.handle {
            Handle::Move => {
                let delta = gesture.current - gesture.start;
                ferroui_base::Matrix::create_translation(delta.x, delta.y)
            }
            Handle::Resize(handle) => {
                let Some(geometry) = gesture.geometry else {
                    return;
                };
                if geometry.size.width <= 0.0 || geometry.size.height <= 0.0 {
                    return;
                }
                let bounds = geometry.resize(gesture.start, gesture.current, handle);
                geometry.from_surface
                    * ferroui_base::Matrix::create_scale(
                        bounds.width / geometry.size.width,
                        bounds.height / geometry.size.height,
                    )
                    * ferroui_base::Matrix::create_translation(bounds.x, bounds.y)
                    * geometry.to_surface
            }
            _ => return,
        };
        for ghost in &gesture.ghosts {
            ghost.render(context, adjustment, 0.7);
        }
    }

    pub(super) fn handle_layout_key(&self, event: &KeyEventArgs) -> bool {
        if self.interactive.get()
            || !self.helpers_visible.get()
            || self.gesture.borrow().is_some()
            || self.insert.borrow().is_some()
        {
            return false;
        }
        if event.key == Key::Escape && self.helper_focus.get().is_some() {
            self.helper_focus.set(None);
            self.invalidate_visual();
            return true;
        }
        let Some(snapshot) = self.layout_helper_snapshot() else {
            return false;
        };
        if event.key == Key::F8 {
            let enabled = snapshot
                .scene
                .handles
                .iter()
                .enumerate()
                .filter_map(|(index, handle)| handle.disabled_reason.is_none().then_some(index))
                .collect::<Vec<_>>();
            if enabled.is_empty() {
                return false;
            }
            let current = self
                .helper_focus
                .get()
                .and_then(|index| enabled.iter().position(|x| *x == index));
            let next = if event.key_modifiers.contains(KeyModifiers::SHIFT) {
                current.map_or(enabled.len() - 1, |index| {
                    (index + enabled.len() - 1) % enabled.len()
                })
            } else {
                current.map_or(0, |index| (index + 1) % enabled.len())
            };
            self.helper_focus.set(Some(enabled[next]));
            self.invalidate_visual();
            return true;
        }
        let Some(index) = self.helper_focus.get() else {
            return false;
        };
        let Some(handle) = snapshot.scene.handles.get(index) else {
            self.helper_focus.set(None);
            return false;
        };
        let sign = match event.key {
            Key::Left | Key::Up => -1.0,
            Key::Right | Key::Down => 1.0,
            _ => return false,
        };
        let step = if event.key_modifiers.contains(KeyModifiers::SHIFT) {
            10.0
        } else if event.key_modifiers.contains(KeyModifiers::CONTROL) {
            0.1
        } else {
            1.0
        };
        let distance = sign * step / handle.direction;
        let local = match handle.axis {
            Axis::Horizontal => ToolPoint::new(distance, 0.0),
            Axis::Vertical => ToolPoint::new(0.0, distance),
        };
        let result = snapshot
            .handle_delta_to_artboard(index, local)
            .map(|delta| Point::new(delta.x, delta.y))
            .and_then(|delta| {
                let session = self.session().ok_or("No designer session")?;
                let preview = session
                    .borrow()
                    .preview_layout_handle(&snapshot, index, delta)?;
                Ok(preview)
            });
        match result {
            Ok(preview) if preview.is_accepted() => {
                if let Some(commit) = self.commit.borrow().as_ref().cloned() {
                    commit(DesignGesture::LayoutEdit {
                        preview: Rc::new(preview),
                    });
                }
                self.invalidate_layout_helpers();
                self.helper_error.borrow_mut().take();
            }
            Ok(preview) => {
                *self.helper_error.borrow_mut() = preview.rejection().map(str::to_owned);
            }
            Err(error) => {
                *self.helper_error.borrow_mut() = Some(error);
            }
        }
        self.notify();
        true
    }
}
