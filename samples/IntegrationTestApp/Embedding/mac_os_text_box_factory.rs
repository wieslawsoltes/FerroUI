//! Port of `Embedding/MacOSTextBoxFactory.cs`.

use super::objc::{self, Class, Id, Imp, NSRange, Sel};
use super::{INativeTextBoxFactory, INativeTextBoxImpl, MacHelper, MacOSViewHandle};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::threading::DispatcherTimer;
use ferroui_controls::platform::IPlatformHandle;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use std::time::Duration;

pub struct MacOSTextBoxFactory;

impl INativeTextBoxFactory for MacOSTextBoxFactory {
    fn create_control(&self, _parent: Rc<dyn IPlatformHandle>) -> Rc<dyn INativeTextBoxImpl> {
        MacHelper::ensure_initialized();
        MacOSTextBox::new()
    }
}

/// The handlers of an event without arguments.
#[derive(Default)]
struct Handlers {
    list: Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
}

impl Handlers {
    fn add(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.list.borrow_mut().push(handler.clone());
        let list = Rc::downgrade(&self.list);
        Disposable::create(move || {
            if let Some(list) = list.upgrade() {
                let mut list = list.borrow_mut();
                if let Some(index) = list.iter().position(|other| Rc::ptr_eq(other, &handler)) {
                    list.remove(index);
                }
            }
        })
    }

    fn invoke(&self) {
        // The handlers run after the borrow ends: a handler may add or remove one.
        let handlers: Vec<Rc<dyn Fn()>> = self.list.borrow().clone();
        for handler in handlers {
            handler();
        }
    }
}

thread_local! {
    /// The text box of each native view of the class of the sample, by the address of the
    /// view: what the methods of the class find their text box with.
    static TEXT_BOXES: RefCell<HashMap<usize, Weak<MacOSTextBox>>> = RefCell::new(HashMap::new());
}

/// The name of the class of the native view: a subclass of `NSTextView`.
const CLASS_NAME: &str = "IntegrationTestAppNativeTextBox";

/// `MacOSTextBoxFactory.MacOSTextBox`: a text view of AppKit (the managed original derives
/// from the text view; here the view is of a class declared at run time, whose methods find
/// this object).
struct MacOSTextBox {
    view: Id,
    handle: Rc<MacOSViewHandle>,
    timer: Rc<DispatcherTimer>,
    context_menu_requested: Handlers,
    hovered: Handlers,
    pointer_exited: Handlers,
}

impl MacOSTextBox {
    fn new() -> Rc<MacOSTextBox> {
        let view = objc::send_id(objc::send_id(Self::class(), "alloc"), "init");
        assert!(!view.is_null(), "the native text view is created");
        // `TextStorage.Append(new("Native text box"))`.
        let storage = objc::send_id(view, "textStorage");
        let length = objc::send_usize(storage, "length");
        objc::replace_characters_in_range(storage, NSRange { location: length, length: 0 }, objc::ns_string("Native text box"));

        let timer = DispatcherTimer::new();
        timer.set_interval(Duration::from_millis(400));
        let this = Rc::new(MacOSTextBox {
            view,
            handle: Rc::new(MacOSViewHandle::new(view)),
            timer,
            context_menu_requested: Handlers::default(),
            hovered: Handlers::default(),
            pointer_exited: Handlers::default(),
        });
        {
            // The text box holds its timer: the handler of the timer holds the text box weakly.
            let weak = Rc::downgrade(&this);
            this.timer.tick(move |_| {
                if let Some(this) = weak.upgrade() {
                    this.hovered.invoke();
                    this.timer.stop();
                }
            });
        }
        TEXT_BOXES.with(|text_boxes| text_boxes.borrow_mut().insert(view as usize, Rc::downgrade(&this)));
        this
    }

    /// The class of the native view, declared on its first use.
    fn class() -> Class {
        let methods: [(&str, Imp, &str); 5] = [
            ("mouseEntered:", Self::imp(mouse_entered), "v@:@"),
            ("mouseExited:", Self::imp(mouse_exited), "v@:@"),
            ("mouseMoved:", Self::imp(mouse_moved), "v@:@"),
            ("rightMouseDown:", Self::imp(right_mouse_down), "v@:@"),
            ("rightMouseUp:", Self::imp(right_mouse_up), "v@:@"),
        ];
        // SAFETY: each implementation takes the receiver, the selector and one object and
        // returns nothing, as its type encoding states.
        unsafe { objc::declare_class(CLASS_NAME, Self::super_class(), &methods) }
    }

    fn super_class() -> Class {
        objc::class("NSTextView")
    }

    fn imp(method: extern "C" fn(Id, Sel, Id)) -> Imp {
        // SAFETY: the runtime calls the implementation with the signature its type encoding
        // states, which is the signature of `method`.
        unsafe { std::mem::transmute::<extern "C" fn(Id, Sel, Id), Imp>(method) }
    }

    /// The text box of the native view `view`.
    fn of(view: Id) -> Option<Rc<MacOSTextBox>> {
        TEXT_BOXES.with(|text_boxes| text_boxes.borrow().get(&(view as usize)).and_then(Weak::upgrade))
    }

    /// `base.<selector>(theEvent)`.
    fn call_base(view: Id, selector: Sel, the_event: Id) {
        // SAFETY: `view` is an object of the class of the sample, whose superclass is the text
        // view, which responds to the mouse messages with the event as their argument.
        unsafe { objc::send_super_void_id(view, Self::super_class(), selector, the_event) }
    }
}

impl Drop for MacOSTextBox {
    fn drop(&mut self) {
        let view = self.view as usize;
        // Not while the table is borrowed or gone (the thread ends): the entry is then left.
        let _ = TEXT_BOXES.try_with(|text_boxes| {
            if let Ok(mut text_boxes) = text_boxes.try_borrow_mut() {
                text_boxes.remove(&view);
            }
        });
    }
}

impl INativeTextBoxImpl for MacOSTextBox {
    fn handle(&self) -> Rc<dyn IPlatformHandle> {
        self.handle.clone()
    }

    fn text(&self) -> String {
        // The view is gone once its handle is destroyed.
        if self.handle.handle() == 0 {
            return String::new();
        }
        let storage = objc::send_id(self.view, "textStorage");
        objc::string(objc::send_id(storage, "string")).unwrap_or_default()
    }

    fn set_text(&self, value: &str) {
        if self.handle.handle() == 0 {
            return;
        }
        let storage = objc::send_id(self.view, "textStorage");
        let length = objc::send_usize(storage, "length");
        objc::replace_characters_in_range(storage, NSRange { location: 0, length }, objc::ns_string(value));
    }

    fn context_menu_requested(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.context_menu_requested.add(handler)
    }

    fn hovered(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.hovered.add(handler)
    }

    fn pointer_exited(&self, handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        self.pointer_exited.add(handler)
    }
}

/// `MouseEntered`.
extern "C" fn mouse_entered(view: Id, selector: Sel, the_event: Id) {
    if let Some(this) = MacOSTextBox::of(view) {
        this.timer.stop();
        this.timer.start();
    }
    MacOSTextBox::call_base(view, selector, the_event);
}

/// `MouseExited`.
extern "C" fn mouse_exited(view: Id, selector: Sel, the_event: Id) {
    if let Some(this) = MacOSTextBox::of(view) {
        this.timer.stop();
        this.pointer_exited.invoke();
    }
    MacOSTextBox::call_base(view, selector, the_event);
}

/// `MouseMoved`.
extern "C" fn mouse_moved(view: Id, selector: Sel, the_event: Id) {
    if let Some(this) = MacOSTextBox::of(view) {
        this.timer.stop();
        this.timer.start();
    }
    MacOSTextBox::call_base(view, selector, the_event);
}

/// `RightMouseDown`.
extern "C" fn right_mouse_down(view: Id, _selector: Sel, _the_event: Id) {
    if let Some(this) = MacOSTextBox::of(view) {
        this.context_menu_requested.invoke();
    }
}

/// `RightMouseUp`.
extern "C" fn right_mouse_up(_view: Id, _selector: Sel, _the_event: Id) {
    // Don't call base to prevent default action.
}
