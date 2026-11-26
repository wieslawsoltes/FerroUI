//! Collect contacts from a Studio card when a device is selected explicitly.
use crate::{PreviewStagePointerDevice, PreviewStagePointerInput, PreviewStagePointerPhase};
use ferroui_base::input::{IPointer, InputElement, PointerEventArgs, PointerType};
use ferroui_base::media::SolidColorBrush;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{Ref, WeakRef};
use ferroui_controls::{Border, Control, Grid};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

struct Contact {
    pointer: Rc<dyn IPointer>,
    input: PreviewStagePointerInput,
}

pub(super) struct StagePointerSurface {
    surface: WeakRef<Border>,
    host: WeakRef<Control>,
    contacts: RefCell<BTreeMap<u32, Contact>>,
    send: Rc<dyn Fn(PreviewStagePointerInput)>,
}

impl StagePointerSurface {
    pub(super) fn attach(
        content: &Ref<Grid>,
        host: &Ref<Control>,
        name: String,
        send: impl Fn(PreviewStagePointerInput) + 'static,
    ) -> Rc<Self> {
        let surface = Border::new();
        surface.set_name(Some(name));
        surface.set_background(Some(SolidColorBrush::parse("Transparent").unwrap().into()));
        let this = Rc::new(Self {
            surface: surface.downgrade(),
            host: host.downgrade(),
            contacts: RefCell::new(BTreeMap::new()),
            send: Rc::new(send),
        });
        use PreviewStagePointerPhase::{Cancel, Down, Move, Up};
        let weak = Rc::downgrade(&this);
        surface.add_handler(InputElement::pointer_pressed_event(), move |_, event| {
            if event.properties().is_left_button_pressed {
                if let Some(surface) = weak.upgrade() {
                    surface.route(event, Down);
                }
            }
        });
        let weak = Rc::downgrade(&this);
        surface.add_handler(InputElement::pointer_moved_event(), move |_, event| {
            if let Some(surface) = weak.upgrade() {
                surface.route(event, Move);
            }
        });
        let weak = Rc::downgrade(&this);
        surface.add_handler(InputElement::pointer_released_event(), move |_, event| {
            if let Some(surface) = weak.upgrade() {
                surface.route(event, Up);
            }
        });
        let weak = Rc::downgrade(&this);
        surface.add_handler(
            InputElement::pointer_capture_lost_event(),
            move |_, event| {
                if let Some(surface) = weak.upgrade() {
                    let contact = surface
                        .contacts
                        .borrow_mut()
                        .remove(&(event.pointer().id() as u32));
                    if let Some(contact) = contact {
                        (surface.send)(PreviewStagePointerInput {
                            phase: Cancel,
                            ..contact.input
                        });
                    }
                }
            },
        );
        content.children().add(surface.upcast::<Control>());
        this
    }

    pub(super) fn cancel(&self) {
        let contacts = self.contacts.take();
        if contacts.is_empty() {
            return;
        }
        let send = self.send.clone();
        for Contact { pointer, input } in contacts.into_values() {
            self.release(&*pointer);
            // Card retirement can occur while the workbench is updating its views.
            let send = send.clone();
            Dispatcher::ui_thread().post_local(
                move || {
                    send(PreviewStagePointerInput {
                        phase: PreviewStagePointerPhase::Cancel,
                        ..input
                    });
                },
                DispatcherPriority::INPUT,
            );
        }
    }

    fn route(&self, event: &PointerEventArgs, phase: PreviewStagePointerPhase) {
        use PreviewStagePointerPhase::{Down, Up};
        let Some(host) = self.host.upgrade() else {
            return;
        };
        let Some(surface) = self.surface.upgrade() else {
            return;
        };
        let size = host.bounds().size();
        if size.width <= 0.0 || size.height <= 0.0 {
            return;
        }
        let pointer = event.pointer();
        let contact = pointer.id() as u32;
        let position = event.get_position(Some(&host));
        let x = position.x / size.width;
        let y = position.y / size.height;
        let properties = event.properties();
        let input = PreviewStagePointerInput {
            contact,
            phase,
            x,
            y,
            device: match pointer.type_() {
                PointerType::Mouse => PreviewStagePointerDevice::Mouse,
                PointerType::Touch => PreviewStagePointerDevice::Touch,
                PointerType::Pen => PreviewStagePointerDevice::Pen,
            },
            pressure: Some(properties.pressure),
            tilt_x: Some(properties.x_tilt),
            tilt_y: Some(properties.y_tilt),
            twist: Some(properties.twist),
        };
        if phase == Down {
            if !(0.0..=1.0).contains(&x) || !(0.0..=1.0).contains(&y) {
                return;
            }
            event.prevent_gesture_recognition();
            pointer.capture(Some(surface.upcast_ref()));
            self.contacts.borrow_mut().insert(
                contact,
                Contact {
                    pointer: pointer.clone(),
                    input,
                },
            );
        } else {
            let mut contacts = self.contacts.borrow_mut();
            let Some(existing) = contacts.get_mut(&contact) else {
                return;
            };
            existing.input = input;
            if phase == Up {
                contacts.remove(&contact);
            }
        }
        event.set_handled(true);
        (self.send)(input);
        if phase == Up {
            self.release(&**pointer);
        }
    }

    fn release(&self, pointer: &dyn IPointer) {
        if self
            .surface
            .upgrade()
            .is_some_and(|surface| pointer.captured().as_ref() == Some(surface.upcast_ref()))
        {
            pointer.capture(None);
        }
    }
}
