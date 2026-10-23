//! Movable native authoring palettes. Pointer motion changes a retained
//! FerroUI transform, never document geometry or the undo history.
use super::*;
use ferroui_base::input::IPointer;
use ferroui_base::layout::HorizontalAlignment;
use ferroui_base::media::TranslateTransform;

#[derive(Default)]
pub(super) struct PanelState {
    pub floating: Cell<bool>,
    title: RefCell<String>,
    position: Cell<Point>,
    transform: RefCell<Option<Ref<TranslateTransform>>>,
    drag: RefCell<Option<PanelDrag>>,
    context: RefCell<(NodeId, Vec<NodeId>)>,
}

struct PanelDrag {
    pointer: Rc<dyn IPointer>,
    start: Point,
    original: Point,
}

impl EditorShell {
    pub(super) fn wire_panels(self: &Rc<Self>) {
        let transform = TranslateTransform::new();
        self.control::<Border>("ModalPanel")
            .set_render_transform(Some(transform.clone().into()));
        *self.panel_state.transform.borrow_mut() = Some(transform);
        let header = self.control::<Grid>("ModalHeader");
        let weak = Rc::downgrade(self);
        header.add_handler(InputElement::pointer_pressed_event(), move |_, event| {
            let Some(shell) = weak.upgrade() else {
                return;
            };
            if !shell.panel_state.floating.get()
                || !event.properties().is_left_button_pressed
                || shell.button("CloseModalButton").is_pointer_over()
            {
                return;
            }
            if !shell.flush_inspector_drafts() {
                return;
            }
            let start = event.get_position(Some(&shell.root));
            *shell.panel_state.drag.borrow_mut() = Some(PanelDrag {
                pointer: event.pointer().clone(),
                start: Point::new(start.x, start.y),
                original: shell.panel_state.position.get(),
            });
            event.pointer().capture(Some(
                &shell
                    .control::<Grid>("ModalHeader")
                    .upcast::<InputElement>(),
            ));
            event.set_handled(true);
        });
        let weak = Rc::downgrade(self);
        header.add_handler(InputElement::pointer_moved_event(), move |_, event| {
            let Some(shell) = weak.upgrade() else {
                return;
            };
            let held = shell.panel_state.drag.borrow();
            let Some(drag) = held
                .as_ref()
                .filter(|drag| drag.pointer.id() == event.pointer().id())
            else {
                return;
            };
            let point = event.get_position(Some(&shell.root));
            let position = Point::new(
                drag.original.x + point.x - drag.start.x,
                drag.original.y + point.y - drag.start.y,
            );
            drop(held);
            shell.position_panel(position);
            event.set_handled(true);
        });
        let weak = Rc::downgrade(self);
        header.add_handler(InputElement::pointer_released_event(), move |_, event| {
            let Some(shell) = weak.upgrade() else {
                return;
            };
            if shell
                .panel_state
                .drag
                .borrow()
                .as_ref()
                .is_some_and(|drag| drag.pointer.id() == event.pointer().id())
            {
                shell.release_panel_drag();
                event.set_handled(true);
            }
        });
        let weak = Rc::downgrade(self);
        header.add_handler(InputElement::pointer_capture_lost_event(), move |_, _| {
            if let Some(shell) = weak.upgrade() {
                shell.panel_state.drag.borrow_mut().take();
            }
        });
        let weak = Rc::downgrade(self);
        self.root.size_changed(move |_, _| {
            if let Some(shell) = weak.upgrade() {
                shell.resize_panel();
            }
        });
        let weak = Rc::downgrade(self);
        self.control::<Border>("ModalPanel")
            .size_changed(move |_, _| {
                if let Some(shell) = weak.upgrade() {
                    if shell.panel_state.floating.get() {
                        shell.position_panel(shell.panel_state.position.get());
                    }
                }
            });
    }

    pub(super) fn release_panel_drag(&self) {
        let drag = self.panel_state.drag.borrow_mut().take();
        if let Some(drag) = drag {
            drag.pointer.capture(None);
        }
    }

    pub(super) fn configure_panel(&self, title: &str) {
        self.release_panel_drag();
        let floating = matches!(
            title,
            "Effects"
                | "Fill paints"
                | "Masks and compositing"
                | "Components and variants"
                | "Layout placement and order"
                | "Local variables"
                | "Prototype interactions"
                | "Rich text"
                | "Affine transform"
                | "Frame presets"
        );
        let keep_position = floating
            && self.panel_state.floating.get()
            && self.control::<Border>("ModalBackdrop").is_visible()
            && self.panel_state.title.borrow().as_str() == title;
        self.panel_state.floating.set(floating);
        *self.panel_state.title.borrow_mut() = title.to_string();
        let editor = self.editor.borrow();
        *self.panel_state.context.borrow_mut() = (editor.active_page, editor.selection.clone());
        drop(editor);
        let panel = self.control::<Border>("ModalPanel");
        self.control::<Border>("ModalBackdrop")
            .set_background(if floating {
                None
            } else {
                Some(icons::brush("#55000000"))
            });
        panel.set_horizontal_alignment(if floating {
            HorizontalAlignment::Left
        } else {
            HorizontalAlignment::Center
        });
        panel.set_vertical_alignment(if floating {
            VerticalAlignment::Top
        } else {
            VerticalAlignment::Center
        });
        self.resize_panel();
        if floating {
            if !keep_position {
                self.position_panel(self.preferred_panel_position());
            }
        } else if let Some(transform) = self.panel_state.transform.borrow().as_ref() {
            transform.set_x(0.0);
            transform.set_y(0.0);
        }
    }

    fn resize_panel(&self) {
        let root = self.root.bounds();
        if root.width <= 0.0 || root.height <= 0.0 {
            return;
        }
        let panel = self.control::<Border>("ModalPanel");
        panel.set_width(620.0_f64.min((root.width - 32.0).max(1.0)));
        panel.set_max_height(660.0_f64.min((root.height - 32.0).max(1.0)));
        self.control::<ferroui_controls::ScrollViewer>("ModalViewport")
            .set_max_height(470.0_f64.min((root.height - 220.0).max(100.0)));
        if self.panel_state.floating.get() {
            self.position_panel(self.panel_state.position.get());
        }
    }

    fn position_panel(&self, requested: Point) {
        let root = self.root.bounds();
        let panel = self.control::<Border>("ModalPanel");
        let size = panel.bounds();
        let width = if size.width > 0.0 {
            size.width
        } else {
            panel.width()
        };
        let height = if size.height > 0.0 {
            size.height
        } else {
            panel.max_height()
        };
        let position = Point::new(
            requested
                .x
                .clamp(16.0, (root.width - width - 16.0).max(16.0)),
            requested
                .y
                .clamp(16.0, (root.height - height - 16.0).max(16.0)),
        );
        self.panel_state.position.set(position);
        if let Some(transform) = self.panel_state.transform.borrow().as_ref() {
            transform.set_x(position.x);
            transform.set_y(position.y);
        }
    }

    fn preferred_panel_position(&self) -> Point {
        let root = self.root.bounds();
        let width = 620.0_f64.min((root.width - 32.0).max(1.0));
        let height = 660.0_f64.min((root.height - 32.0).max(1.0));
        let right = (root.width - width - 16.0).max(16.0);
        let bottom = (root.height - height - 16.0).max(16.0);
        let selection = self.editor.borrow().selection_bounds().map(|bounds| {
            let viewport = self.canvas.viewport.get();
            let origin = self
                .canvas
                .translate_point(ferroui_base::Point::new(0.0, 0.0), &self.root)
                .unwrap_or_default();
            core::Rect::new(
                bounds.x * viewport.zoom + viewport.pan.x + origin.x,
                bounds.y * viewport.zoom + viewport.pan.y + origin.y,
                bounds.width * viewport.zoom,
                bounds.height * viewport.zoom,
            )
        });
        [
            Point::new(right, 16.0),
            Point::new(right, bottom),
            Point::new(16.0, 16.0),
            Point::new(16.0, bottom),
        ]
        .into_iter()
        .min_by(|a, b| {
            let overlap = |p: &Point| {
                selection
                    .map(|s| {
                        ((p.x + width).min(s.x + s.width) - p.x.max(s.x)).max(0.0)
                            * ((p.y + height).min(s.y + s.height) - p.y.max(s.y)).max(0.0)
                    })
                    .unwrap_or_default()
            };
            overlap(a).total_cmp(&overlap(b))
        })
        .unwrap_or(Point::new(right, 16.0))
    }

    pub(super) fn refresh_open_panel(self: &Rc<Self>) {
        if !self.panel_state.floating.get() || !self.control::<Border>("ModalBackdrop").is_visible()
        {
            return;
        }
        let editor = self.editor.borrow();
        if *self.panel_state.context.borrow() == (editor.active_page, editor.selection.clone()) {
            return;
        }
        drop(editor);
        let title = self.panel_state.title.borrow().clone();
        match title.as_str() {
            "Effects" => self.effects_dialog(),
            "Fill paints" => self.paints_dialog(),
            "Masks and compositing" => self.compositing_dialog(),
            "Components and variants" => self.components_dialog(),
            "Layout placement and order" => self.layout_dialog(),
            "Local variables" => self.visual_variables_dialog(),
            "Prototype interactions" => self.interactions_dialog(),
            "Rich text" => self.rich_text_dialog(),
            "Affine transform" => self.affine_dialog(),
            _ => {}
        }
        self.sync_document_draft_dirty();
    }
}
