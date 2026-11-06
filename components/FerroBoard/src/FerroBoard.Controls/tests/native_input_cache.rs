#[path = "support/headless.rs"]
mod headless;
use ferroboard_controls::input::{PasswordBoxInputTarget, TextBoxInputTarget};
use ferroboard_core::{editor::ITextInputTarget, text::Utf16Text};
use ferroui_base::{Ref, data::BindingPriority, threading::Dispatcher};
use ferroui_controls::TextBox;
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
#[derive(Clone, Copy, Default, Debug)]
struct Allocations {
    calls: usize,
    bytes: usize,
}
thread_local! {
    static TRACK:Cell<bool>=const {Cell::new(false)};
    static COUNTS:Cell<Allocations>=const {Cell::new(Allocations{calls:0,bytes:0})};
}
fn record(size: usize) {
    if TRACK.try_with(Cell::get).unwrap_or(false) {
        let _ = COUNTS.try_with(|c| {
            let v = c.get();
            c.set(Allocations {
                calls: v.calls + 1,
                bytes: v.bytes + size,
            });
        });
    }
}
struct Counted;
unsafe impl GlobalAlloc for Counted {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        record(l.size());
        unsafe { System.alloc(l) }
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        record(l.size());
        unsafe { System.alloc_zeroed(l) }
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        record(n);
        unsafe { System.realloc(p, l, n) }
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) }
    }
}
#[global_allocator]
static ALLOCATOR: Counted = Counted;
fn adapter(field: Ref<TextBox>, password: bool) -> (Rc<dyn ITextInputTarget>, Box<dyn Fn()>) {
    if password {
        let t = PasswordBoxInputTarget::new(field, None).unwrap();
        (t.clone(), Box::new(move || t.detach()))
    } else {
        let t = TextBoxInputTarget::new(field, None).unwrap();
        (t.clone(), Box::new(move || t.detach()))
    }
}
fn field(password: bool) -> Ref<TextBox> {
    let f = TextBox::new();
    if password {
        f.set_password_char('●');
    }
    f
}
#[test]
fn stable_native_text_and_selection_reads_do_not_allocate_full_buffer_copies() {
    headless::run(|| {
        let data = "a😀".repeat(8192);
        let mut measured = Vec::new();
        for password in [false, true] {
            let field = field(password);
            field.set_text(Some(&data));
            let (target, _owner) = adapter(field, password);
            for _ in 0..8 {
                std::hint::black_box(target.text());
                std::hint::black_box(target.selection_length());
            }
            COUNTS.with(|c| c.set(Allocations::default()));
            TRACK.with(|c| c.set(true));
            for _ in 0..64 {
                std::hint::black_box(target.text());
                std::hint::black_box(target.selection_start());
                std::hint::black_box(target.selection_length());
            }
            TRACK.with(|c| c.set(false));
            let counts = COUNTS.with(Cell::get);
            println!("password={password}, stable-read allocations={counts:?}");
            measured.push(counts);
            assert_eq!(target.text(), Utf16Text::from(data.as_str()));
        }
        for counts in measured {
            assert_eq!(
                counts.calls, 0,
                "stable adapter reads must not clone the native buffer: {counts:?}"
            );
        }
    });
}
#[test]
fn earlier_reentrant_text_changing_callbacks_observe_the_live_value() {
    headless::run(|| {
        for password in [false, true] {
            let field = field(password);
            let slot = Rc::new(RefCell::new(None::<Weak<dyn ITextInputTarget>>));
            let lookup = slot.clone();
            let weak_field = field.downgrade();
            let nested = Cell::new(false);
            let observed = Rc::new(RefCell::new(Vec::new()));
            let log = observed.clone();
            field.text_changing(move |_, _| {
                let target = lookup.borrow().as_ref().and_then(Weak::upgrade);
                if let Some(target) = target {
                    let current = target.text().to_utf8().unwrap();
                    log.borrow_mut().push(current.clone());
                    if current == "outer" && !nested.replace(true) {
                        weak_field.upgrade().unwrap().set_text(Some("inner"));
                        assert_eq!(target.text(), Utf16Text::from("inner"));
                    }
                }
            });
            let (target, _owner) = adapter(field.clone(), password);
            *slot.borrow_mut() = Some(Rc::downgrade(&target));
            field.set_text(Some("outer"));
            assert_eq!(&*observed.borrow(), &["outer", "inner"]);
            assert_eq!(target.text(), Utf16Text::from("inner"));
        }
    });
}
#[test]
fn style_current_local_clear_and_disposed_adapter_reads_remain_fresh() {
    headless::run(|| {
        for password in [false, true] {
            let field = field(password);
            let _style = field.set_value_with_priority(
                TextBox::text_property(),
                Some("style".to_owned()),
                BindingPriority::Style,
            );
            let (target, owner) = adapter(field.clone(), password);
            assert_eq!(target.text(), Utf16Text::from("style"));
            field.set_text(Some("local"));
            assert_eq!(target.text(), Utf16Text::from("local"));
            field.clear_value(TextBox::text_property());
            assert_eq!(target.text(), Utf16Text::from("style"));
            field.set_current_value(TextBox::text_property(), Some("current".to_owned()));
            assert_eq!(target.text(), Utf16Text::from("current"));
            owner();
            field.set_text(Some("after disposal"));
            assert_eq!(target.text(), Utf16Text::from("after disposal"));
            field.set_text(None);
            assert_eq!(target.text(), Utf16Text::default());
            Dispatcher::current_dispatcher().run_jobs(None);
        }
    });
}

use ferroui_base::{
    FerroObjectImpl, FerroObjectImplExt, FerroPropertyChangedEventArgs, StyledElementImpl,
    VisualImpl, input::InputElementImpl, interactivity::InteractiveImpl, layout::LayoutableImpl,
};
use ferroui_controls::{ControlImpl, TextBoxImpl, primitives::TemplatedControlImpl};
#[repr(C)]
struct EarlyTextBox {
    base: TextBox,
    target: RefCell<Option<Weak<dyn ITextInputTarget>>>,
    observed: RefCell<Vec<Utf16Text>>,
}
ferroui_base::ferro_class!(EarlyTextBox:TextBox);
ferroui_base::ferro_impl_classes!(EarlyTextBox:StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,TemplatedControlImpl,ControlImpl,TextBoxImpl);
impl FerroObjectImpl for EarlyTextBox {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        if change.property() == TextBox::text_property().as_property() {
            let target = this.target.borrow().as_ref().and_then(Weak::upgrade);
            if let Some(target) = target {
                this.observed.borrow_mut().push(target.text());
            }
        }
        Self::parent_on_property_changed(this, change);
    }
}
impl EarlyTextBox {
    fn new() -> Ref<Self> {
        ferroui_base::instantiate(Self {
            base: TextBox::construct(),
            target: RefCell::new(None),
            observed: RefCell::new(Vec::new()),
        })
    }
}
#[test]
fn derived_virtual_callbacks_read_fresh_text_before_the_parent_handler() {
    headless::run(|| {
        for password in [false, true] {
            let field = EarlyTextBox::new();
            if password {
                field.set_password_char('●');
            }
            let (target, _owner) = adapter(field.clone().upcast(), password);
            *field.target.borrow_mut() = Some(Rc::downgrade(&target));
            field.set_text(Some("virtual"));
            field.set_text(Some("latest"));
            assert_eq!(
                &*field.observed.borrow(),
                &[Utf16Text::from("virtual"), Utf16Text::from("latest")]
            );
            assert_eq!(target.text(), Utf16Text::from("latest"));
        }
    });
}
#[test]
fn selection_coercion_callbacks_see_new_text_before_text_changing() {
    headless::run(|| {
        for password in [false, true] {
            let field = field(password);
            field.set_text(Some("long initial value"));
            field.set_caret_index(18);
            let slot = Rc::new(RefCell::new(None::<Weak<dyn ITextInputTarget>>));
            let lookup = slot.clone();
            let observed = Rc::new(RefCell::new(Vec::new()));
            let log = observed.clone();
            let _subscription = field.property_changed(move |change| {
                if (change.property() == TextBox::selection_start_property().as_property()
                    || change.property() == TextBox::selection_end_property().as_property())
                    && let Some(target) = lookup.borrow().as_ref().and_then(Weak::upgrade)
                {
                    log.borrow_mut().push(target.text());
                }
            });
            let (target, _owner) = adapter(field.clone(), password);
            *slot.borrow_mut() = Some(Rc::downgrade(&target));
            field.set_text(Some("tiny"));
            assert!(!observed.borrow().is_empty());
            assert!(
                observed
                    .borrow()
                    .iter()
                    .all(|text| text == &Utf16Text::from("tiny"))
            );
            assert_eq!(target.text(), Utf16Text::from("tiny"));
        }
    });
}
