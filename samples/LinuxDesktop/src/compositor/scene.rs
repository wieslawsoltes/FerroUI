//! A client surface is one retained FerroUI composition visual. Moving,
//! restacking or scaling it only changes visual properties. A new buffer
//! replaces its bitmap, leaving the visual and other clients intact.
use crate::compositor_server::{Input, Scene, Server};
use ferroui_base::input::{
    FocusChangedEventArgs, InputElementImpl, InputElementImplExt, KeyEventArgs, KeyModifiers,
    PhysicalKey, PointerCaptureLostEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs, PointerUpdateKind, PointerWheelEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::imaging::Bitmap;
use ferroui_base::platform::{AlphaFormat, PixelFormat};
use ferroui_base::rendering::composition::{
    CompositionDrawingSurface, CompositionSurfaceVisual, CompositionVisual, Compositor,
    ElementComposition,
};
use ferroui_base::{
    ferro_class, ferro_impl_classes, instantiate, FerroObjectImpl, FerroObjectImplExt,
    FerroPropertyChangedEventArgs, ObjectType, PixelSize, Point, Rect, Ref, StyledElementImpl,
    Vector, Vector3D, Visual, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::{Control, ControlImpl};
use ferroui_view::prelude::{Element, IntoElement};
use ferroui_wayland::xkb_key_transform::XkbKeyTransform;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::OnceLock;
use std::time::Instant;

pub struct ClientScene(Ref<ClientPlane>);

impl ClientScene {
    pub fn new(server: Rc<Server>) -> Rc<Self> {
        let plane = instantiate(ClientPlane {
            base: Control::construct(),
            server,
            visuals: RefCell::new(None),
            hit_regions: RefCell::new(Vec::new()),
            windows: RefCell::new(HashMap::new()),
            buttons: Cell::new(0),
            drag: Cell::new(None),
            focused: Cell::new(None),
            size: Cell::new((0, 0)),
            epoch: Instant::now(),
        });
        plane.set_focusable(true);
        Rc::new(Self(plane))
    }

    pub fn element(&self) -> Element {
        self.0.clone().into_element()
    }

    pub fn refresh(&self, scene: &Scene) {
        self.0.apply_scene(scene);
    }

    pub fn has_keyboard_focus(&self) -> bool {
        self.0.is_focused()
    }

    /// Restore keyboard delivery after the shell dismisses its own UI.
    pub fn focus(&self) {
        if !self.0.hit_regions.borrow().is_empty() {
            self.0.focus();
        }
    }
}

struct Layer {
    visual: CompositionSurfaceVisual,
    surface: CompositionDrawingSurface,
    revision: Option<u64>,
}

impl Drop for Layer {
    fn drop(&mut self) {
        self.visual.dispose();
        self.surface.dispose();
    }
}

struct VisualTree {
    compositor: Rc<Compositor>,
    root: Rc<CompositionVisual>,
    layers: HashMap<u64, Layer>,
    serial: Option<u64>,
}

#[derive(Copy, Clone)]
struct Drag {
    window: u64,
    start: Point,
    bounds: Rect,
    resize: bool,
}

#[repr(C)]
struct ClientPlane {
    base: Control,
    server: Rc<Server>,
    visuals: RefCell<Option<VisualTree>>,
    hit_regions: RefCell<Vec<(u64, Rect)>>,
    windows: RefCell<HashMap<u64, Rect>>,
    buttons: Cell<u8>,
    drag: Cell<Option<Drag>>,
    focused: Cell<Option<u64>>,
    size: Cell<(i32, i32)>,
    epoch: Instant,
}

ferro_class!(ClientPlane: Control);
ferro_impl_classes!(ClientPlane: ControlImpl, LayoutableImpl, InteractiveImpl, StyledElementImpl);

impl FerroObjectImpl for ClientPlane {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Visual::bounds_property().as_property() {
            this.resize();
        }
    }
}

impl VisualImpl for ClientPlane {
    fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_visual_tree(this, e);
        if let Some(host) = ElementComposition::get_element_visual(this) {
            let compositor = host.compositor().clone();
            let root = compositor.create_container_visual();
            root.set_clip_to_bounds(true);
            ElementComposition::set_element_child_visual(this, Some(root.clone()));
            *this.visuals.borrow_mut() = Some(VisualTree {
                compositor,
                root,
                layers: HashMap::new(),
                serial: None,
            });
            this.size.set((0, 0));
            this.resize();
            this.apply_scene(&this.server.snapshot());
        }
    }

    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        if let Some(tree) = this.visuals.borrow_mut().take() {
            ElementComposition::set_element_child_visual(this, None);
            tree.root.children().clear();
            tree.root.dispose();
        }
        Self::parent_on_detached_from_visual_tree(this, e);
    }

    fn custom_hit_test(this: &Self, pt: Point) -> Option<bool> {
        // Empty desktop pixels still belong to the wallpaper and shortcuts below.
        Some(this.window_at(pt).is_some())
    }
}

impl ClientPlane {
    fn time(&self) -> u32 {
        self.epoch.elapsed().as_millis() as u32
    }

    fn resize(&self) {
        let bounds = self.bounds();
        let size = (bounds.width.round() as i32, bounds.height.round() as i32);
        if size.0 <= 0 || size.1 <= 0 || size == self.size.get() {
            return;
        }
        let tree = self.visuals.borrow();
        if let Some(tree) = tree.as_ref() {
            self.size.set(size);
            tree.root.set_size(Vector::new(bounds.width, bounds.height));
            self.server.send(Input::Resize {
                width: size.0,
                height: size.1,
            });
        }
    }

    fn window_at(&self, pt: Point) -> Option<u64> {
        self.hit_regions
            .borrow()
            .iter()
            .rev()
            .find(|(_, rect)| rect.contains(pt))
            .map(|(id, _)| *id)
    }

    fn apply_scene(&self, scene: &Scene) {
        let mut tree = self.visuals.borrow_mut();
        let Some(tree) = tree.as_mut() else { return };
        if tree.serial == Some(scene.serial) {
            return;
        }
        let children = tree.root.children();
        tree.layers.retain(|id, layer| {
            let present = scene.surfaces.iter().any(|s| s.id == *id);
            if !present {
                children.remove(&layer.visual);
            }
            present
        });
        let mut regions = self.hit_regions.borrow_mut();
        regions.clear();
        for (index, s) in scene.surfaces.iter().enumerate() {
            let layer = tree.layers.entry(s.id).or_insert_with(|| {
                let surface = tree.compositor.create_drawing_surface();
                let visual = tree.compositor.create_surface_visual();
                visual.set_surface(Some((*surface).clone()));
                Layer {
                    visual,
                    surface,
                    revision: None,
                }
            });
            if layer.revision != Some(s.pixels.revision) {
                let p = &s.pixels;
                let bitmap = Bitmap::from_pixels(
                    PixelFormat::BGRA8888,
                    AlphaFormat::Premul,
                    &p.bytes,
                    PixelSize::new(p.width, p.height),
                    Vector::new(96., 96.),
                    p.width * 4,
                );
                drop(layer.surface.update_with_bitmap(&bitmap));
                layer.revision = Some(p.revision);
            }
            layer
                .visual
                .set_offset(Vector3D::new(s.x.into(), s.y.into(), 0.));
            layer
                .visual
                .set_size(Vector::new(s.width.into(), s.height.into()));
            if index >= children.count() || !Rc::ptr_eq(&children.get(index), &layer.visual) {
                children.remove(&layer.visual);
                children.insert(index, (*layer.visual).clone());
            }
            regions.push((
                s.window,
                Rect::new(s.x.into(), s.y.into(), s.width.into(), s.height.into()),
            ));
        }
        drop(regions);
        let mut windows = self.windows.borrow_mut();
        windows.clear();
        for w in &scene.windows {
            windows.insert(
                w.id,
                Rect::new(w.x.into(), w.y.into(), w.width.into(), w.height.into()),
            );
        }
        drop(windows);
        let focused = scene
            .windows
            .iter()
            .find(|w| w.focused && !w.minimized)
            .map(|w| w.id);
        if self.focused.replace(focused) != focused && focused.is_some() {
            self.focus();
        }
        tree.serial = Some(scene.serial);
        let serial = scene.serial;
        let presented = self.server.present_callback();
        tree.compositor
            .request_composition_batch_commit_async()
            .rendered()
            .on_completed(move || presented(serial));
    }

    fn motion(&self, pt: Point) {
        if let Some(drag) = self.drag.get() {
            let dx = pt.x - drag.start.x;
            let dy = pt.y - drag.start.y;
            self.server.send(if drag.resize {
                Input::Configure {
                    id: drag.window,
                    width: (drag.bounds.width + dx).round().max(1.) as i32,
                    height: (drag.bounds.height + dy).round().max(1.) as i32,
                }
            } else {
                Input::Move {
                    id: drag.window,
                    x: (drag.bounds.x + dx).round() as i32,
                    y: (drag.bounds.y + dy).round() as i32,
                }
            });
        } else {
            self.server.send(Input::Motion {
                x: pt.x,
                y: pt.y,
                time: self.time(),
            });
        }
    }

    fn key(&self, e: &KeyEventArgs, pressed: bool) {
        // Invert FerroUI's native scan-code table once. Layout-dependent text
        // is translated by the child's keymap from the physical key.
        static CODES: OnceLock<HashMap<PhysicalKey, u32>> = OnceLock::new();
        let codes = CODES.get_or_init(|| {
            (1..=0x2ff)
                .filter_map(|code| {
                    let key = XkbKeyTransform::physical_key_from_evdev(code);
                    (key != PhysicalKey::None).then_some((key, code))
                })
                .collect()
        });
        if let Some(&code) = codes.get(&e.physical_key) {
            self.server.send(Input::Key {
                code,
                pressed,
                time: self.time(),
            });
            e.set_handled(true);
        }
    }
}

// Linux input-event-codes.h; bit positions also track simultaneous held buttons.
fn button(kind: PointerUpdateKind) -> Option<(u8, u32)> {
    use PointerUpdateKind::*;
    Some(match kind {
        LeftButtonPressed | LeftButtonReleased => (1, 0x110),
        RightButtonPressed | RightButtonReleased => (2, 0x111),
        MiddleButtonPressed | MiddleButtonReleased => (4, 0x112),
        XButton1Pressed | XButton1Released => (8, 0x113),
        XButton2Pressed | XButton2Released => (16, 0x114),
        Other => return None,
    })
}

impl InputElementImpl for ClientPlane {
    fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
        this.motion(e.get_position(Some(this)));
        e.set_handled(true);
    }

    fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
        let Some((bit, code)) = button(e.properties().pointer_update_kind) else {
            return;
        };
        let pt = e.get_position(Some(this));
        this.focus();
        this.motion(pt);
        if this.buttons.get() == 0 {
            e.pointer().capture(Some(&this.to_ref().upcast()));
            if e.key_modifiers().contains(KeyModifiers::ALT) && (bit == 1 || bit == 2) {
                if let Some(id) = this.window_at(pt) {
                    if let Some(bounds) = this.windows.borrow().get(&id).copied() {
                        this.server.send(Input::Activate(id));
                        this.drag.set(Some(Drag {
                            window: id,
                            start: pt,
                            bounds,
                            resize: bit == 2,
                        }));
                    }
                }
            }
        }
        this.buttons.set(this.buttons.get() | bit);
        if this.drag.get().is_none() {
            this.server.send(Input::Button {
                code,
                pressed: true,
                time: this.time(),
            });
        }
        e.set_handled(true);
    }

    fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
        let Some((bit, code)) = button(e.properties().pointer_update_kind) else {
            return;
        };
        if this.buttons.get() & bit == 0 {
            return;
        }
        let pt = e.get_position(Some(this));
        this.motion(pt);
        if this.drag.get().is_none() {
            this.server.send(Input::Button {
                code,
                pressed: false,
                time: this.time(),
            });
        }
        let remaining = this.buttons.get() & !bit;
        this.buttons.set(remaining);
        if remaining == 0 {
            this.drag.set(None);
            e.pointer().capture(None);
            this.motion(pt);
        }
        e.set_handled(true);
    }

    fn on_pointer_exited(this: &Self, _e: &PointerEventArgs) {
        if this.buttons.get() == 0 {
            this.server.send(Input::Leave { time: this.time() });
        }
    }

    fn on_pointer_capture_lost(this: &Self, e: &PointerCaptureLostEventArgs) {
        let buttons = this.buttons.replace(0);
        if this.drag.take().is_none() {
            for i in 0..5 {
                if buttons & (1 << i) != 0 {
                    this.server.send(Input::Button {
                        code: 0x110 + i,
                        pressed: false,
                        time: this.time(),
                    });
                }
            }
        }
        Self::parent_on_pointer_capture_lost(this, e);
    }

    fn on_pointer_wheel_changed(this: &Self, e: &PointerWheelEventArgs) {
        this.motion(e.get_position(Some(this)));
        let d = e.delta();
        // FerroUI wheel units are detents, positive upward; Wayland's axis
        // units are pixels, positive downward (ten pixels per detent).
        this.server.send(Input::Axis {
            horizontal: -d.x * 10.,
            vertical: -d.y * 10.,
            time: this.time(),
        });
        e.set_handled(true);
    }

    fn on_key_down(this: &Self, e: &KeyEventArgs) {
        this.key(e, true);
    }
    fn on_key_up(this: &Self, e: &KeyEventArgs) {
        this.key(e, false);
    }

    fn on_lost_focus(this: &Self, e: &FocusChangedEventArgs) {
        this.server.send(Input::KeyboardLeave { time: this.time() });
        Self::parent_on_lost_focus(this, e);
    }
}
