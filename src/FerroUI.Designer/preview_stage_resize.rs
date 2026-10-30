//! A native corner grip retains a detached size draft. The accepted graph stays
//! untouched until the host receives one guarded resize command on release.
use ferroui_base::input::{
    Cursor, IPointer, InputElement, Key, KeyModifiers, MouseButton, PointerEventArgs,
    StandardCursorType,
};
use ferroui_base::media::SolidColorBrush;
use ferroui_base::{Matrix, Point, Ref, Size, Thickness, Visual, WeakRef};
use ferroui_controls::automation::AutomationProperties;
use ferroui_controls::{Border, Control, ToolTip};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type Commit = Rc<dyn Fn(Size)>;
type Draft = Rc<dyn Fn(Option<Size>)>;

#[derive(Clone)]
struct Drag {
    root: WeakRef<Visual>,
    to_root: Matrix,
    start: Point,
    start_root: Point,
    original: Size,
    current: Size,
    moved: bool,
}

pub(super) struct ResizeGrip {
    control: Ref<Border>,
    host: WeakRef<Control>,
    pointer: RefCell<Option<Rc<dyn IPointer>>>,
    drag: RefCell<Option<Drag>>,
    retired: Cell<bool>,
    commit: RefCell<Option<Commit>>,
    draft: RefCell<Option<Draft>>,
}

impl ResizeGrip {
    pub(super) fn new(host: &Ref<Control>, name: &str) -> Rc<Self> {
        let control = Border::new();
        control.set_name(Some(name.to_owned()));
        control.classes().add("designer-chrome");
        control.set_width(28.0);
        control.set_height(28.0);
        control.set_padding(Thickness::uniform(6.0));
        control.set_background(Some(SolidColorBrush::parse("Transparent").unwrap().into()));
        control.set_focusable(true);
        control.set_cursor(Some(Cursor::new(StandardCursorType::BottomRightCorner)));
        control.set_child(Some(
            crate::icons::vector_icon(
                "F1 M13,2 L15,2 15,15 2,15 2,13 13,13 Z M9,6 L11,6 11,11 6,11 6,9 9,9 Z",
                16.0,
            )
            .upcast::<Control>(),
        ));
        AutomationProperties::set_name(&control, Some("Resize preview layout"));
        ToolTip::set_tip(&control, Some(Rc::new(
            "Drag to resize the logical preview. Release applies a fixed size. Escape cancels. Arrow keys change one unit; Shift changes ten.".to_owned(),
        )));
        let grip = Rc::new(Self {
            control,
            host: host.downgrade(),
            pointer: RefCell::new(None),
            drag: RefCell::new(None),
            retired: Cell::new(false),
            commit: RefCell::new(None),
            draft: RefCell::new(None),
        });
        grip.connect();
        grip
    }

    pub(super) fn control(&self) -> Ref<Control> {
        self.control.clone().upcast()
    }

    pub(super) fn on_commit(&self, callback: impl Fn(Size) + 'static) {
        *self.commit.borrow_mut() = Some(Rc::new(callback));
    }

    pub(super) fn on_draft(&self, callback: impl Fn(Option<Size>) + 'static) {
        *self.draft.borrow_mut() = Some(Rc::new(callback));
    }

    pub(super) fn cancel(&self) {
        self.drag.borrow_mut().take();
        let pointer = self.pointer.borrow_mut().take();
        if let Some(pointer) = pointer {
            if pointer.captured().as_ref() == Some(&self.control.clone().upcast::<InputElement>()) {
                pointer.capture(None);
            }
        }
        self.notify_draft(None);
    }

    pub(super) fn retire(&self) {
        self.retired.set(true);
        self.cancel();
    }

    fn notify_draft(&self, size: Option<Size>) {
        let callback = self.draft.borrow().clone();
        if let Some(callback) = callback {
            callback(size);
        }
    }

    fn owns_pointer(&self, pointer: &Rc<dyn IPointer>) -> bool {
        self.pointer
            .borrow()
            .as_ref()
            .is_some_and(|active| active.id() == pointer.id())
    }

    fn host(&self) -> Option<Ref<Control>> {
        if self.retired.get() || !self.control.is_effectively_visible() {
            return None;
        }
        self.host
            .upgrade()
            .filter(|host| host.is_effectively_visible() && valid_size(host.bounds().size()))
    }

    fn update_drag(&self, event: &PointerEventArgs) {
        let Some(mut drag) = self.drag.borrow().clone() else {
            return;
        };
        let valid = self.host().and_then(|host| {
            let root = drag.root.upgrade()?;
            (host.bounds().size() == drag.original
                && host.transform_to_visual(&root) == Some(drag.to_root))
            .then_some((host, root))
        });
        let Some((host, root)) = valid else {
            self.cancel();
            return;
        };
        let point = event.get_position(Some(&host));
        let delta = point - drag.start;
        if !delta.x.is_finite() || !delta.y.is_finite() {
            self.cancel();
            return;
        }
        drag.current = Size::new(
            (drag.original.width + delta.x).round().clamp(1.0, 16384.0),
            (drag.original.height + delta.y).round().clamp(1.0, 16384.0),
        );
        let movement = event.get_position(Some(&root)) - drag.start_root;
        drag.moved |= movement.x.hypot(movement.y) >= 2.0;
        let size = drag.current;
        *self.drag.borrow_mut() = Some(drag);
        self.notify_draft(Some(size));
    }

    fn request(&self, size: Size) {
        if self.retired.get() || !valid_size(size) {
            return;
        }
        let callback = self.commit.borrow().clone();
        if let Some(callback) = callback {
            callback(size);
        }
    }

    fn connect(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        self.control
            .add_handler(InputElement::pointer_pressed_event(), move |_, event| {
                let Some(grip) = weak.upgrade() else {
                    return;
                };
                if !event.properties().is_left_button_pressed {
                    return;
                }
                grip.cancel();
                let Some(host) = grip.host() else {
                    return;
                };
                let Some(root) = host.visual_root() else {
                    return;
                };
                let Some(to_root) = host.transform_to_visual(&root) else {
                    return;
                };
                if crate::affine_manipulation::inverse(to_root).is_none() {
                    return;
                }
                grip.control.focus();
                let size = host.bounds().size();
                *grip.drag.borrow_mut() = Some(Drag {
                    root: root.downgrade(),
                    to_root,
                    start: event.get_position(Some(&host)),
                    start_root: event.get_position(Some(&root)),
                    original: size,
                    current: size,
                    moved: false,
                });
                *grip.pointer.borrow_mut() = Some(event.pointer().clone());
                event
                    .pointer()
                    .capture(Some(&grip.control.clone().upcast::<InputElement>()));
                grip.notify_draft(Some(size));
                event.set_handled(true);
            });
        let weak = Rc::downgrade(self);
        self.control
            .add_handler(InputElement::pointer_moved_event(), move |_, event| {
                let Some(grip) = weak.upgrade() else {
                    return;
                };
                if grip.owns_pointer(event.pointer()) {
                    grip.update_drag(event);
                    event.set_handled(true);
                }
            });
        let weak = Rc::downgrade(self);
        self.control
            .add_handler(InputElement::pointer_released_event(), move |_, event| {
                let Some(grip) = weak.upgrade() else {
                    return;
                };
                if event.initial_press_mouse_button() != MouseButton::Left
                    || !grip.owns_pointer(event.pointer())
                {
                    return;
                }
                // Include the final release position even if the platform coalesced the
                // last motion. Cancellation releases capture before a host can retire us.
                grip.update_drag(event);
                let drag = grip.drag.borrow().clone();
                grip.cancel();
                if let Some(drag) = drag.filter(|drag| drag.moved && drag.current != drag.original)
                {
                    grip.request(drag.current);
                }
                event.set_handled(true);
            });
        let weak = Rc::downgrade(self);
        self.control.add_handler(
            InputElement::pointer_capture_lost_event(),
            move |_, event| {
                if let Some(grip) = weak.upgrade() {
                    if grip.owns_pointer(event.pointer()) {
                        grip.cancel();
                    }
                }
            },
        );
        let weak = Rc::downgrade(self);
        self.control
            .add_handler(InputElement::lost_focus_event(), move |_, _| {
                if let Some(grip) = weak.upgrade() {
                    grip.cancel();
                }
            });
        let weak = Rc::downgrade(self);
        self.control
            .add_handler(InputElement::key_down_event(), move |_, event| {
                let Some(grip) = weak.upgrade() else {
                    return;
                };
                if event.key == Key::Tab {
                    return;
                }
                // Resize has no authored undo entry. Keep Delete/Undo and other
                // editor shortcuts from acting on an unrelated source selection.
                event.set_handled(true);
                if event.key == Key::Escape {
                    grip.cancel();
                    return;
                }
                if !matches!(event.key, Key::Left | Key::Right | Key::Up | Key::Down)
                    || event
                        .key_modifiers
                        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::META)
                {
                    return;
                }
                grip.cancel();
                let Some(host) = grip.host() else {
                    return;
                };
                let original = host.bounds().size();
                let mut size = original;
                let step = if event.key_modifiers.contains(KeyModifiers::SHIFT) {
                    10.0
                } else {
                    1.0
                };
                match event.key {
                    Key::Left => size.width -= step,
                    Key::Right => size.width += step,
                    Key::Up => size.height -= step,
                    Key::Down => size.height += step,
                    _ => {}
                }
                size.width = size.width.clamp(1.0, 16384.0);
                size.height = size.height.clamp(1.0, 16384.0);
                if size != original {
                    grip.request(size);
                }
                event.set_handled(true);
            });
    }
}

impl Drop for ResizeGrip {
    fn drop(&mut self) {
        let pointer = self.pointer.get_mut().take();
        if let Some(pointer) = pointer {
            if pointer.captured().as_ref() == Some(&self.control.clone().upcast::<InputElement>()) {
                pointer.capture(None);
            }
        }
    }
}

fn valid_size(size: Size) -> bool {
    size.width.is_finite()
        && size.height.is_finite()
        && (1.0..=16384.0).contains(&size.width)
        && (1.0..=16384.0).contains(&size.height)
}
