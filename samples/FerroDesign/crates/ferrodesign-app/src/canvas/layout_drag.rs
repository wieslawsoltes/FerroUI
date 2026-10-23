//! Native pointer ownership and public DrawingContext insertion adorners.
//! A retained core session keeps pointer updates independent of document size.
use super::*;

const DRAG_THRESHOLD: f64 = 4.0;

pub(super) struct LayoutDragGesture {
    session: core::LayoutTransferSession,
    start_world: Point,
    start_screen: Point,
    current_world: Point,
    active: bool,
    target: Option<core::LayoutTransferTarget>,
    bounds: Rect,
    exclude: Vec<NodeId>,
    render_token: core::EditorRenderToken,
}

impl LayoutDragGesture {
    fn new(
        session: core::LayoutTransferSession,
        world: Point,
        screen: Point,
        editor: &Editor,
    ) -> Self {
        let bounds = session
            .source_corners()
            .map(|corners| {
                corners
                    .into_iter()
                    .map(|point| Rect::new(point.x, point.y, 0.0, 0.0))
                    .reduce(Rect::union)
                    .unwrap_or_default()
            })
            .reduce(Rect::union)
            .unwrap_or_default();
        let exclude = session.source_ids().collect();
        Self {
            session,
            start_world: world,
            start_screen: screen,
            current_world: world,
            active: false,
            target: None,
            bounds,
            exclude,
            render_token: editor.render_change_token(),
        }
    }

    fn update(&mut self, world: Point, screen: Point) {
        self.current_world = world;
        self.active |= self.start_screen.distance(screen) >= DRAG_THRESHOLD;
        if self.active {
            self.target = self.session.target(world);
        }
    }

    fn owns(&self, editor: &Editor) -> bool {
        editor.document.revision == self.session.revision()
            && editor.revision == self.session.revision()
            && editor.active_page == self.session.page()
            && editor.selection.as_slice() == self.session.selection()
            && editor.render_change_token() == self.render_token
            && !editor.has_active_transaction()
    }
}

/// The ordinary move still owns its original snapped preview transaction.
/// This immutable geometry index only detects entry into another frame. The
/// revision is advanced exclusively after this canvas reports a move preview,
/// so an external transaction, selection or document change invalidates it.
pub(super) struct FreeLayoutProbe {
    session: core::LayoutTransferSession,
    start_world: Point,
    start_screen: Point,
    revision: u64,
    transaction: Option<core::EditorTransactionToken>,
}

impl FreeLayoutProbe {
    pub(super) fn record_transaction(&mut self, editor: &Editor) {
        self.transaction = editor.active_transaction_token();
    }
    pub(super) fn record_revision(&mut self, revision: u64) {
        self.revision = revision;
    }

    fn owns(&self, editor: &Editor) -> bool {
        self.owns_transaction(editor)
            && editor.active_page == self.session.page()
            && editor.selection.as_slice() == self.session.selection()
    }

    fn owns_transaction(&self, editor: &Editor) -> bool {
        editor.revision == self.revision
            && editor.document.revision == self.revision
            && self.transaction.is_some()
            && editor.active_transaction_token() == self.transaction
    }

    fn cancel_transaction(&self, editor: &mut Editor) -> bool {
        if !self.owns_transaction(editor) {
            return false;
        }
        let selection = editor.selection.clone();
        let page = editor.active_page;
        editor.cancel_transaction();
        editor.selection = selection
            .into_iter()
            .filter(|id| editor.document.nodes.contains_key(id))
            .collect();
        if page != editor.active_page && editor.document.page(page).is_some() {
            editor.active_page = page;
            editor.refresh();
        }
        true
    }
}

impl DesignCanvas {
    /// State of the actual retained native pointer gesture, for diagnostics and
    /// browser regression probes. Reading it never borrows or edits the model.
    pub fn layout_drag_state(&self) -> Option<serde_json::Value> {
        self.layout_drag.borrow().as_ref().map(|gesture| {
            serde_json::json!({
                "child": gesture.session.child(), "parent": gesture.session.parent(),
                "selection": gesture.session.selection(),
                "active": gesture.active, "target": gesture.target,
            })
        })
    }

    /// Any layout-owned root starts a deferred selection transfer. A free
    /// selection retains the ordinary move and its snapping until it enters
    /// another frame. Protected or stale selections never start a transaction.
    pub(super) fn layout_drag_press(
        &self,
        editor: &Editor,
        child: NodeId,
        world: Point,
        screen: Point,
    ) -> bool {
        if !editor.selection.contains(&child) || editor.has_active_transaction() {
            return true;
        }
        let Ok(session) =
            core::LayoutTransferSession::new(&editor.document, &editor.selection, child, world)
        else {
            return true;
        };
        if session.has_layout_source() {
            *self.layout_drag.borrow_mut() =
                Some(LayoutDragGesture::new(session, world, screen, editor));
            self.hover.set(None);
            self.snap_guides.set((None, None));
            true
        } else {
            *self.free_layout_probe.borrow_mut() = Some(FreeLayoutProbe {
                revision: session.revision(),
                transaction: None,
                session,
                start_world: world,
                start_screen: screen,
            });
            false
        }
    }

    pub fn cancel_layout_drag(&self) -> bool {
        let active = self.layout_drag.borrow_mut().take().is_some();
        let probe = self.free_layout_probe.borrow_mut().take();
        let had_probe = probe.is_some();
        let mut restored = false;
        if let Some(probe) = probe {
            let moved = matches!(*self.gesture.borrow(), Gesture::Move { .. });
            if moved {
                *self.gesture.borrow_mut() = Gesture::None;
                let mut editor = self.editor.borrow_mut();
                restored = probe.cancel_transaction(&mut editor);
            }
        }
        if active || had_probe {
            self.snap_guides.set((None, None));
            self.invalidate_visual();
            if restored {
                self.emit(CanvasEvent::Document);
            }
        }
        active || had_probe
    }

    pub(super) fn layout_drag_move(&self, world: Point, screen: Point) -> bool {
        self.layout_drag_move_constrained(world, screen, false)
    }

    /// Transfer a free move only after entering a different editable frame.
    /// Returning to the original source footprint is a true no-op; regular
    /// movement inside an ordinary group does not implicitly ungroup children.
    fn promote_free_layout_drag(&self, world: Point, screen: Point) -> bool {
        let mut pending = self.free_layout_probe.borrow_mut();
        let Some(probe) = pending.as_ref() else {
            return false;
        };
        let moving = matches!(*self.gesture.borrow(), Gesture::Move { .. });
        let editor = self.editor.borrow();
        let owned = moving && probe.owns(&editor);
        let valid = owned && self.tool.get() == Tool::Select && self.presentation.get().is_none();
        if !valid {
            drop(editor);
            drop(pending);
            self.cancel_layout_drag();
            return true;
        }
        if probe.start_screen.distance(screen) < DRAG_THRESHOLD {
            return false;
        }
        let Some(target) = probe
            .session
            .target(world)
            .filter(|target| probe.session.reparents(*target))
        else {
            return false;
        };
        if target.parent == probe.session.page()
            && probe.session.source_ids().all(|id| {
                editor
                    .document
                    .nodes
                    .get(&id)
                    .and_then(|node| node.parent)
                    .and_then(|parent| editor.document.nodes.get(&parent))
                    .is_none_or(|parent| parent.kind == NodeKind::Group && parent.layout.is_none())
            })
        {
            return false;
        }
        drop(editor);
        let probe = pending.take().unwrap();
        drop(pending);
        *self.gesture.borrow_mut() = Gesture::None;
        let result = {
            let mut editor = self.editor.borrow_mut();
            editor.cancel_transaction();
            core::LayoutTransferSession::new(
                &editor.document,
                &editor.selection,
                probe.session.child(),
                probe.start_world,
            )
            .map(|session| {
                LayoutDragGesture::new(session, probe.start_world, probe.start_screen, &editor)
            })
        };
        self.snap_guides.set((None, None));
        match result {
            Ok(mut gesture) => {
                gesture.update(world, screen);
                *self.layout_drag.borrow_mut() = Some(gesture);
            }
            Err(error) => self.emit(CanvasEvent::Notice(error.to_string())),
        }
        // Restore the old snapped preview before displaying the deferred one.
        self.emit(CanvasEvent::Document);
        self.invalidate_visual();
        true
    }

    pub(super) fn layout_drag_move_constrained(
        &self,
        world: Point,
        screen: Point,
        shift: bool,
    ) -> bool {
        let promoted = self.promote_free_layout_drag(world, screen);
        let mut active = self.layout_drag.borrow_mut();
        let Some(gesture) = active.as_mut() else {
            return promoted;
        };
        let valid = gesture.owns(&self.editor.borrow())
            && self.tool.get() == Tool::Select
            && self.presentation.get().is_none();
        if valid {
            gesture.update(world, screen);
            self.snap_guides.set((None, None));
            if gesture
                .target
                .is_some_and(|target| target.kind == core::LayoutTransferKind::Free)
            {
                let mut delta = world - gesture.start_world;
                if shift {
                    if delta.x.abs() > delta.y.abs() {
                        delta.y = 0.0;
                    } else {
                        delta.x = 0.0;
                    }
                }
                if self.snap.get() {
                    delta.x = delta.x.round();
                    delta.y = delta.y.round();
                    let proposed = Rect::new(
                        gesture.bounds.x + delta.x,
                        gesture.bounds.y + delta.y,
                        gesture.bounds.width,
                        gesture.bounds.height,
                    );
                    let snap = self.editor.borrow().smart_snap(
                        proposed,
                        5.0 / self.viewport.get().zoom,
                        &gesture.exclude,
                    );
                    let snap_x = !shift || delta.x != 0.0;
                    let snap_y = !shift || delta.y != 0.0;
                    if snap_x {
                        delta.x += snap.delta.x;
                    }
                    if snap_y {
                        delta.y += snap.delta.y;
                    }
                    self.snap_guides.set((
                        if snap_x { snap.vertical_guide } else { None },
                        if snap_y { snap.horizontal_guide } else { None },
                    ));
                }
                gesture.current_world = gesture.start_world + delta;
                if let Some(target) = &mut gesture.target {
                    target.position = gesture.current_world;
                }
            }
        } else {
            *active = None;
            self.snap_guides.set((None, None));
        }
        drop(active);
        self.invalidate_visual();
        true
    }

    /// Take ownership before releasing capture or raising callbacks. A late
    /// release after Escape, Undo, a camera/tool change, or capture loss cannot
    /// use an old session to write a new history entry.
    pub(super) fn layout_drag_release(&self, world: Point, screen: Point) -> bool {
        self.layout_drag_release_constrained(world, screen, false)
    }

    pub(super) fn layout_drag_release_constrained(
        &self,
        world: Point,
        screen: Point,
        shift: bool,
    ) -> bool {
        let consumed = self.layout_drag_move_constrained(world, screen, shift);
        let Some(gesture) = self.layout_drag.borrow_mut().take() else {
            // A normal free move still commits through its established path.
            // Clear the probe before releasing native capture synchronously.
            self.free_layout_probe.borrow_mut().take();
            return consumed;
        };
        let (result, changed) = {
            let mut editor = self.editor.borrow_mut();
            let revision = editor.revision;
            if gesture.owns(&editor) && gesture.active {
                let result = gesture
                    .target
                    .map(|target| editor.commit_layout_transfer(&gesture.session, target));
                (result, editor.revision != revision)
            } else {
                (None, false)
            }
        };
        self.snap_guides.set((None, None));
        self.invalidate_visual();
        if let Some(result) = result {
            match result {
                Ok(result) => {
                    if changed {
                        self.emit(CanvasEvent::Document);
                    }
                    if !result.warnings.is_empty() {
                        self.emit(CanvasEvent::Notice(result.warnings.join("\n")));
                    }
                }
                Err(error) => self.emit(CanvasEvent::Notice(error.to_string())),
            }
        }
        true
    }

    pub(super) fn render_layout_drag(&self, context: &mut DrawingContext, viewport: Viewport) {
        let active = self.layout_drag.borrow();
        let Some(gesture) = active.as_ref().filter(|gesture| gesture.active) else {
            return;
        };
        let delta = gesture.current_world - gesture.start_world;
        let outline = pen(0xA60D99FF, 1.0);
        for corners in gesture.session.source_corners() {
            let corners = corners.map(|point| viewport.world_to_screen(point + delta));
            for index in 0..4 {
                context.draw_line(&outline, ui(corners[index]), ui(corners[(index + 1) % 4]));
            }
        }
        let Some(target) = gesture.target else {
            return;
        };
        if target.kind != core::LayoutTransferKind::Flow {
            if target.kind == core::LayoutTransferKind::Free {
                if let Some(corners) = gesture.session.destination_corners(target) {
                    let corners = corners.map(|point| viewport.world_to_screen(point));
                    let guide = pen(0xFF0D99FF, 2.0);
                    for index in 0..4 {
                        context.draw_line(&guide, ui(corners[index]), ui(corners[(index + 1) % 4]));
                    }
                }
            }
            return;
        }
        let mut start = viewport.world_to_screen(target.marker.start);
        let mut end = viewport.world_to_screen(target.marker.end);
        let direction = end - start;
        let length = direction.distance(Point::default());
        let direction = if length > 1e-6 {
            direction * (1.0 / length)
        } else {
            Point::new(0.0, 1.0)
        };
        // Very small/zero-size children still have a discoverable insertion
        // edge. All adorner dimensions are screen pixels at any canvas zoom.
        if length < 12.0 {
            let center = (start + end) * 0.5;
            start = center - direction * 6.0;
            end = center + direction * 6.0;
        }
        let normal = Point::new(-direction.y, direction.x) * 4.0;
        let halo = pen(0xFFFFFFFF, 4.0);
        let guide = pen(0xFF0D99FF, 2.0);
        context.draw_line(&halo, ui(start), ui(end));
        context.draw_line(&guide, ui(start), ui(end));
        context.draw_line(&guide, ui(start - normal), ui(start + normal));
        context.draw_line(&guide, ui(end - normal), ui(end + normal));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gesture() -> (Editor, LayoutDragGesture, Point) {
        let mut document = core::Document::new("Pointer ownership");
        let page = document.pages[0].id;
        let mut frame = Node::new(NodeKind::Frame, "Row", 10.0, 20.0, 300.0, 120.0);
        frame.layout = Some(core::AutoLayout::default());
        let parent = document.insert_node(page, frame).unwrap();
        let children: Vec<_> = (0..3)
            .map(|index| {
                document
                    .insert_node(
                        parent,
                        Node::new(
                            NodeKind::Rectangle,
                            format!("Child {index}"),
                            0.0,
                            0.0,
                            50.0,
                            40.0,
                        ),
                    )
                    .unwrap()
            })
            .collect();
        document.layout_all();
        let mut editor = Editor::new(document);
        editor.selection = vec![children[0]];
        let start = editor.document.world_bounds(children[0]).unwrap().center();
        let last = editor.document.world_bounds(children[2]).unwrap();
        let end = Point::new(last.right() - 1.0, last.center().y);
        let session = core::LayoutTransferSession::new(
            &editor.document,
            &editor.selection,
            children[0],
            start,
        )
        .unwrap();
        let gesture = LayoutDragGesture::new(session, start, start, &editor);
        (editor, gesture, end)
    }

    #[test]
    fn native_drag_threshold_and_release_position_do_not_need_intermediate_pointer_events() {
        let (mut editor, mut gesture, end) = gesture();
        gesture.update(
            gesture.start_world + Point::new(3.0, 0.0),
            gesture.start_screen + Point::new(3.0, 0.0),
        );
        assert!(!gesture.active);
        assert!(gesture.target.is_none());
        gesture.update(end, end);
        assert!(gesture.active && gesture.owns(&editor));
        let target = gesture.target.unwrap();
        assert_eq!(target.index, 2);
        assert!(!editor.can_undo());
        editor
            .commit_layout_transfer(&gesture.session, target)
            .unwrap();
        assert_eq!(editor.history_len(), 1);
        assert!(!gesture.owns(&editor));
    }

    #[test]
    fn deferred_gesture_does_not_own_unrelated_pending_history_or_replaced_selection() {
        let (mut editor, gesture, _) = gesture();
        assert!(gesture.owns(&editor));
        editor.begin_transaction("Another editor");
        assert!(!gesture.owns(&editor));
        editor.cancel_transaction();
        assert!(
            !gesture.owns(&editor),
            "Restoration changed the document revision"
        );
        let session = core::LayoutTransferSession::new(
            &editor.document,
            &editor.selection,
            editor.selection[0],
            Point::default(),
        )
        .unwrap();
        let gesture = LayoutDragGesture::new(session, Point::default(), Point::default(), &editor);
        editor.selection.clear();
        assert!(!gesture.owns(&editor));
    }

    #[test]
    fn actual_native_capture_loss_discards_preview_and_late_release_without_history() {
        use ferroui_base::input::{IPointer, Pointer as NativePointer, PointerType};

        for transfer in [false, true] {
            let (editor, mut gesture, end) = gesture();
            let before = editor.document.to_json().unwrap();
            let revision = editor.revision;
            gesture.update(end, end);
            let editor = Rc::new(RefCell::new(editor));
            let canvas = DesignCanvas::new(editor.clone());
            *canvas.layout_drag.borrow_mut() = Some(gesture);
            let pointer =
                NativePointer::new(NativePointer::get_next_free_id(), PointerType::Mouse, true);
            pointer.capture(Some(&canvas.clone().upcast::<InputElement>()));
            if transfer {
                pointer.capture(Some(&InputElement::new()));
            } else {
                pointer.platform_capture_lost();
            }
            assert!(canvas.layout_drag_state().is_none());
            assert!(!canvas.layout_drag_move(end + Point::new(10.0, 0.0), end));
            assert!(!canvas.layout_drag_release(end, end));
            assert_eq!(editor.borrow().document.to_json().unwrap(), before);
            assert_eq!(editor.borrow().revision, revision);
            assert!(!editor.borrow().can_undo());
            assert!(!editor.borrow().has_active_transaction());
        }
    }

    #[test]
    fn successful_drop_clears_preview_before_synchronous_capture_loss_can_cancel_it() {
        use ferroui_base::input::{IPointer, Pointer as NativePointer, PointerType};

        let (editor, gesture, end) = gesture();
        let parent = gesture.session.parent();
        let child = gesture.session.child();
        let editor = Rc::new(RefCell::new(editor));
        let canvas = DesignCanvas::new(editor.clone());
        *canvas.layout_drag.borrow_mut() = Some(gesture);
        let pointer =
            NativePointer::new(NativePointer::get_next_free_id(), PointerType::Mouse, true);
        pointer.capture(Some(&canvas.clone().upcast::<InputElement>()));
        assert!(canvas.layout_drag_release(end, end));
        pointer.capture(None);
        assert!(canvas.layout_drag_state().is_none());
        assert_eq!(
            editor.borrow().document.nodes[&parent].children.last(),
            Some(&child)
        );
        assert_eq!(editor.borrow().history_len(), 1);
        assert!(!editor.borrow().has_active_transaction());
    }

    #[test]
    fn escape_cancellation_does_not_restore_or_commit_an_unrelated_pending_edit() {
        let (editor, gesture, _) = gesture();
        let child = gesture.session.child();
        let editor = Rc::new(RefCell::new(editor));
        let canvas = DesignCanvas::new(editor.clone());
        *canvas.layout_drag.borrow_mut() = Some(gesture);
        editor.borrow_mut().begin_transaction("External edit");
        editor
            .borrow_mut()
            .document
            .nodes
            .get_mut(&child)
            .unwrap()
            .name = "External draft".into();
        canvas.cancel();
        assert!(canvas.layout_drag_state().is_none());
        assert!(editor.borrow().has_active_transaction());
        assert_eq!(
            editor.borrow().document.nodes[&child].name,
            "External draft"
        );
        assert!(!editor.borrow().can_undo());
        editor.borrow_mut().cancel_transaction();
        assert_ne!(
            editor.borrow().document.nodes[&child].name,
            "External draft"
        );
    }

    fn free_move() -> (Ref<DesignCanvas>, NodeId, NodeId, Point, Point, String) {
        let mut document = core::Document::new("Free move handoff");
        let page = document.pages[0].id;
        let child = document
            .insert_node(
                page,
                Node::new(NodeKind::Rectangle, "Free child", 20.0, 40.0, 30.0, 24.0),
            )
            .unwrap();
        let mut frame = Node::new(NodeKind::Frame, "Target", 300.0, 20.0, 240.0, 120.0);
        frame.layout = Some(core::AutoLayout {
            height: core::SizingMode::Fixed,
            ..core::AutoLayout::default()
        });
        let parent = document.insert_node(page, frame).unwrap();
        document
            .insert_node(
                parent,
                Node::new(NodeKind::Rectangle, "Existing", 0.0, 0.0, 50.0, 40.0),
            )
            .unwrap();
        document.layout_all();
        let before = document.to_json().unwrap();
        let mut editor = Editor::new(document);
        editor.selection = vec![child];
        let start = editor.document.world_bounds(child).unwrap().center();
        let canvas = DesignCanvas::new(Rc::new(RefCell::new(editor)));
        canvas.snap.set(false);
        {
            let mut editor = canvas.editor.borrow_mut();
            assert!(!canvas.layout_drag_press(&editor, child, start, start));
            let nodes = vec![editor.document.nodes[&child].clone()];
            let bounds = editor.document.world_bounds(child);
            editor.begin_node_transaction(&[child], "Move layers");
            canvas
                .free_layout_probe
                .borrow_mut()
                .as_mut()
                .unwrap()
                .record_transaction(&editor);
            *canvas.gesture.borrow_mut() = Gesture::Move {
                start,
                nodes,
                bounds,
                exclude: vec![child],
                moved: true,
            };
            editor.document.nodes.get_mut(&child).unwrap().x += 40.0;
            editor.refresh_preview_nodes(&[child]);
            canvas
                .free_layout_probe
                .borrow_mut()
                .as_mut()
                .unwrap()
                .record_revision(editor.revision);
        }
        (
            canvas,
            child,
            parent,
            start,
            Point::new(317.0, 56.0),
            before,
        )
    }

    #[test]
    fn snapped_free_preview_hands_off_to_one_atomic_layout_transfer() {
        let (canvas, child, parent, _, end, before) = free_move();
        assert!(canvas.editor.borrow().has_active_transaction());
        assert!(canvas.layout_drag_move(end, end));
        assert!(canvas.free_layout_probe.borrow().is_none());
        assert!(matches!(*canvas.gesture.borrow(), Gesture::None));
        assert!(!canvas.editor.borrow().has_active_transaction());
        assert_eq!(canvas.editor.borrow().document.to_json().unwrap(), before);
        assert_eq!(
            canvas.layout_drag_state().unwrap()["target"]["parent"],
            parent
        );
        assert!(canvas.layout_drag_release(end, end));
        let mut editor = canvas.editor.borrow_mut();
        assert_eq!(editor.document.nodes[&child].parent, Some(parent));
        assert_eq!(editor.document.nodes[&parent].children[0], child);
        assert_eq!(editor.history_len(), 1);
        assert_eq!(editor.selection, vec![child]);
        editor.undo();
        assert_eq!(editor.document.to_json().unwrap(), before);
        editor.redo();
        assert_eq!(editor.document.nodes[&child].parent, Some(parent));
    }

    #[test]
    fn free_handoff_escape_and_native_capture_loss_preserve_document_and_late_release() {
        use ferroui_base::input::{IPointer, Pointer as NativePointer, PointerType};
        for handoff in [false, true] {
            let (canvas, _, _, _, end, before) = free_move();
            if handoff {
                assert!(canvas.layout_drag_move(end, end));
            }
            let pointer =
                NativePointer::new(NativePointer::get_next_free_id(), PointerType::Mouse, true);
            pointer.capture(Some(&canvas.clone().upcast::<InputElement>()));
            pointer.platform_capture_lost();
            assert!(canvas.layout_drag_state().is_none());
            assert!(canvas.free_layout_probe.borrow().is_none());
            assert!(matches!(*canvas.gesture.borrow(), Gesture::None));
            assert_eq!(canvas.editor.borrow().document.to_json().unwrap(), before);
            assert!(!canvas.editor.borrow().has_active_transaction());
            assert!(!canvas.layout_drag_release(end, end));
            assert!(!canvas.editor.borrow().can_undo());
        }
        let (canvas, _, _, start, end, before) = free_move();
        assert!(canvas.layout_drag_move(end, end));
        assert!(canvas.cancel_active_edit());
        assert!(!canvas.layout_drag_move(start, start));
        assert!(!canvas.layout_drag_release(end, end));
        assert_eq!(canvas.editor.borrow().document.to_json().unwrap(), before);
        assert!(!canvas.editor.borrow().can_undo());
    }

    #[test]
    fn late_free_move_does_not_cancel_or_commit_an_external_transaction() {
        let (canvas, child, _, _, end, _) = free_move();
        {
            let mut editor = canvas.editor.borrow_mut();
            editor.cancel_transaction();
            editor.begin_transaction("External authoring");
            editor.document.nodes.get_mut(&child).unwrap().name = "External draft".into();
        }
        assert!(canvas.layout_drag_move(end, end));
        assert!(canvas.free_layout_probe.borrow().is_none());
        assert!(matches!(*canvas.gesture.borrow(), Gesture::None));
        assert!(!canvas.layout_drag_release(end, end));
        assert_eq!(
            canvas.editor.borrow().document.nodes[&child].name,
            "External draft"
        );
        assert!(canvas.editor.borrow().has_active_transaction());
        assert!(!canvas.editor.borrow().can_undo());
    }

    #[test]
    fn selection_change_cancels_owned_free_preview_without_leaving_pending_history() {
        let (canvas, _, parent, _, end, before) = free_move();
        canvas.editor.borrow_mut().selection = vec![parent];
        assert!(canvas.layout_drag_move(end, end));
        assert_eq!(canvas.editor.borrow().selection, vec![parent]);
        assert_eq!(canvas.editor.borrow().document.to_json().unwrap(), before);
        assert!(!canvas.editor.borrow().has_active_transaction());
        assert!(!canvas.editor.borrow().can_undo());
        assert!(!canvas.layout_drag_release(end, end));
    }

    #[test]
    fn replacing_editor_at_the_same_revision_cannot_reuse_either_gesture_owner() {
        let (canvas, child, _, _, end, _) = free_move();
        let mut replacement = Editor::new(canvas.editor.borrow().document.clone());
        replacement.revision = canvas.editor.borrow().revision;
        replacement.document.revision = replacement.revision;
        replacement.selection = vec![child];
        replacement.begin_transaction("Move layers");
        replacement.document.nodes.get_mut(&child).unwrap().name = "Replacement draft".into();
        *canvas.editor.borrow_mut() = replacement;
        assert!(canvas.layout_drag_move(end, end));
        assert!(canvas.editor.borrow().has_active_transaction());
        assert_eq!(
            canvas.editor.borrow().document.nodes[&child].name,
            "Replacement draft"
        );
        assert!(!canvas.layout_drag_release(end, end));
        let (editor, gesture, _) = gesture();
        let mut replacement = Editor::new(editor.document.clone());
        replacement.revision = editor.revision;
        replacement.document.revision = editor.document.revision;
        replacement.selection = editor.selection.clone();
        assert!(!gesture.owns(&replacement));
    }

    #[test]
    fn normal_free_release_retains_the_existing_scoped_commit_path() {
        let (canvas, child, _, start, _, _) = free_move();
        let end = start + Point::new(40.0, 0.0);
        assert!(!canvas.layout_drag_release(end, end));
        assert!(canvas.free_layout_probe.borrow().is_none());
        assert!(canvas.editor.borrow().has_active_transaction());
        assert!(matches!(*canvas.gesture.borrow(), Gesture::Move { .. }));
        assert!(canvas.editor.borrow_mut().commit_transaction());
        assert_eq!(canvas.editor.borrow().document.nodes[&child].x, 60.0);
        assert_eq!(canvas.editor.borrow().history_len(), 1);
    }
}
