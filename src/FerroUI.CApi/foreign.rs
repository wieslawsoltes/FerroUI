//! Controls written in another language (docs/c-abi.md, section 15; #974): a class registered at
//! run time on one of a few common bases, whose virtual members forward to C function pointers.
//!
//! **No cost for Rust applications.** Nothing here changes the class model, the property system
//! or the hot paths of layout, styling and rendering. Each base has one Rust class of this crate
//! (`ForeignControl: Control`, `ForeignPanel: Panel`, `ForeignDecorator: Decorator`,
//! `ForeignContentControl: ContentControl`), written as any Rust control is (`ferro_class!`),
//! whose overrides call the callbacks of the class of the instance. A class registered at run
//! time ([`ferro_class_register`]) is a [`TypeInfo`] of its own, made with the public
//! constructors of the type system, whose base is the Rust class of its base, and a copy of the
//! virtual table of that Rust class whose type slot names the new type: an instance answers
//! `get_type()` with the registered class, so styles, the tables of types and `ferro_create` by
//! name see it as any class.
//!
//! **Fast calls.** The callbacks of a class are a table of function pointers set once
//! ([`ferro_class_on_measure`], ...), and every instance holds a pointer to its class and the data
//! pointer of the language: a virtual call reads two fields and calls one function, with no
//! lookup and no allocation. A callback the class does not set falls through to the base class
//! with no call into the language. Measure and arrange pass scalars and take the size back through
//! two `double *` (a language that cannot write memory calls [`ferro_layout_result`] instead).
//!
//! **Rules for a callback** (docs/bindings-design.md, section 2.12): a plain C function pointer
//! with the data pointer of the instance; synchronous on the UI thread; it may call the ABI again
//! (measure children, read properties, draw): no state of the library is borrowed while it runs;
//! it returns a status (`0` or more is success), and a negative status is recorded as the failure
//! and the base behaviour is used. Nothing may unwind out of a callback.

use crate::error::{status, status_or_value, Error, Result};
use crate::handles;
use crate::thread::{self, UiCell};
use crate::{
    FerroHandle, FerroStatus, FERRO_INVALID_ARGUMENT, FERRO_INVALID_STATE, FERRO_NOT_FOUND, FERRO_TYPE_MISMATCH,
};
use ferroui_base::controls::NameScopeRef;
use ferroui_base::input::{
    InputElementImpl, InputElementImplExt, KeyEventArgs, PointerEventArgs, PointerPressedEventArgs,
    PointerReleasedEventArgs,
};
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{Layoutable, LayoutableImpl, LayoutableImplExt};
use ferroui_base::media::immutable::{ImmutablePen, ImmutableSolidColorBrush};
use ferroui_base::media::{
    BoxShadows, DrawingContext, FlowDirection, FormattedText, Geometry, IBrush, IPen, PushedState, Typeface,
};
use ferroui_base::utilities::Locale;
use ferroui_base::{
    ferro_class, ferro_impl_classes, ferro_overrides, instantiate, FerroObject, FerroObjectImpl, FerroObjectImplExt,
    FerroObjectVTable, FerroProperty, FerroPropertyChangedEventArgs, FerroPropertyRegistry, Matrix, ObjectType, Point,
    Rect, Ref, RoundedRect, Size, StyledElementImpl, StyledPropertyOptions, TypeIdentity, TypeInfo, Upcast, VisualImpl,
    VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt};
use ferroui_controls::{ContentControl, ContentControlImpl, Control, ControlImpl, Decorator, Panel, PanelImpl};
use std::cell::{Cell, RefCell};
use std::ffi::{c_char, c_void};
use std::rc::Rc;

// ---- the callbacks -----------------------------------------------------------------------------

/// Called once for every new instance of a class, with the data of the class
/// ([`ferro_class_set_data`]) and the instance (borrowed for the call); the callback gives the
/// instance its own data with [`ferro_object_set_data`].
pub type FerroCreateFn = Option<unsafe extern "C" fn(class_data: *mut c_void, object: FerroHandle) -> FerroStatus>;

/// Called when an instance goes away, with its data, so that the language releases it.
pub type FerroReleaseDataFn = Option<unsafe extern "C" fn(data: *mut c_void)>;

/// Measure and arrange: the available (final) width and height in, the desired (used) size out
/// through `*width` and `*height` (or [`ferro_layout_result`]).
pub type FerroLayoutFn = Option<
    unsafe extern "C" fn(
        data: *mut c_void,
        object: FerroHandle,
        width_in: f64,
        height_in: f64,
        width: *mut f64,
        height: *mut f64,
    ) -> FerroStatus,
>;

/// Render: `context` names the drawing context of the call for the `ferro_draw_*` functions; it
/// is valid until the callback returns.
pub type FerroRenderFn =
    Option<unsafe extern "C" fn(data: *mut c_void, object: FerroHandle, context: i64) -> FerroStatus>;

/// A notification with one integer: a property of the class changed (`arg` is its property ID),
/// the instance was attached to or detached from the visual tree, its template was applied.
pub type FerroNotifyFn = Option<unsafe extern "C" fn(data: *mut c_void, object: FerroHandle, arg: i64) -> FerroStatus>;

/// A pointer event: `kind` (`FERRO_POINTER_PRESSED`, `_RELEASED`, `_MOVED`), the position relative
/// to the instance and the key modifiers. A status of `1` marks the event as handled.
pub type FerroPointerFn = Option<
    unsafe extern "C" fn(
        data: *mut c_void,
        object: FerroHandle,
        kind: i32,
        x: f64,
        y: f64,
        modifiers: i64,
    ) -> FerroStatus,
>;

/// A key event: `kind` (`FERRO_KEY_DOWN`, `FERRO_KEY_UP`), the key (the value of the `Key`
/// enumeration of the model) and the key modifiers. A status of `1` marks it as handled.
pub type FerroKeyFn = Option<
    unsafe extern "C" fn(data: *mut c_void, object: FerroHandle, kind: i32, key: i64, modifiers: i64) -> FerroStatus,
>;

/// A pointer was pressed on the instance.
pub const FERRO_POINTER_PRESSED: i32 = 1;
/// A pointer was released on the instance.
pub const FERRO_POINTER_RELEASED: i32 = 2;
/// A pointer moved over the instance.
pub const FERRO_POINTER_MOVED: i32 = 3;
/// A key went down while the instance had the focus.
pub const FERRO_KEY_DOWN: i32 = 1;
/// A key went up while the instance had the focus.
pub const FERRO_KEY_UP: i32 = 2;

/// The base of a class: `Control`.
pub const FERRO_BASE_CONTROL: i32 = 1;
/// The base of a class: `Panel` (children).
pub const FERRO_BASE_PANEL: i32 = 2;
/// The base of a class: `Decorator` (one child).
pub const FERRO_BASE_DECORATOR: i32 = 3;
/// The base of a class: `ContentControl` (a content and a template).
pub const FERRO_BASE_CONTENT_CONTROL: i32 = 4;

/// A registered property makes the instance measure again when it changes.
pub const FERRO_PROPERTY_AFFECTS_MEASURE: i64 = 1;
/// A registered property makes the instance arrange again when it changes.
pub const FERRO_PROPERTY_AFFECTS_ARRANGE: i64 = 2;
/// A registered property makes the instance render again when it changes.
pub const FERRO_PROPERTY_AFFECTS_RENDER: i64 = 4;
/// A registered property is inherited down the tree.
pub const FERRO_PROPERTY_INHERITS: i64 = 8;
/// A registered property is attached: it can be set on any control (`Owner.Name`).
pub const FERRO_PROPERTY_ATTACHED: i64 = 16;

/// The callbacks of a class: set once, read on every virtual call.
#[derive(Default)]
struct Callbacks {
    create: Cell<FerroCreateFn>,
    measure: Cell<FerroLayoutFn>,
    arrange: Cell<FerroLayoutFn>,
    render: Cell<FerroRenderFn>,
    property_changed: Cell<FerroNotifyFn>,
    attached: Cell<FerroNotifyFn>,
    detached: Cell<FerroNotifyFn>,
    apply_template: Cell<FerroNotifyFn>,
    pointer: Cell<FerroPointerFn>,
    key: Cell<FerroKeyFn>,
}

/// A property registered on a class: what a change of it does.
#[derive(Clone, Copy)]
struct OwnProperty {
    property: &'static FerroProperty,
    id: i64,
    flags: i64,
}

/// A class registered at run time. Made once, never freed (types live for the process).
struct ForeignClass {
    name: &'static str,
    type_info: &'static TypeInfo,
    /// The virtual table of the instances: the one of the Rust class of the base, with the type
    /// of the object replaced.
    vtable: *const (),
    base: i32,
    class_data: Cell<*mut c_void>,
    callbacks: Callbacks,
    properties: RefCell<Vec<OwnProperty>>,
}

impl ForeignClass {
    /// What the class registered for `property`, if anything.
    #[inline]
    fn own(&self, property: &FerroProperty) -> Option<OwnProperty> {
        self.properties.borrow().iter().find(|own| std::ptr::eq(own.property, property)).copied()
    }
}

/// The classes by index (the index of the constructor of a class).
static CLASSES: UiCell<Vec<&'static ForeignClass>> = UiCell::new(Vec::new());

/// What every foreign instance holds: its class and its data.
#[derive(Default)]
struct Instance {
    class: Cell<Option<&'static ForeignClass>>,
    data: Cell<*mut c_void>,
    release: Cell<FerroReleaseDataFn>,
    /// The handle of the instance last passed to a callback: checked before it is used again.
    handle: Cell<FerroHandle>,
    /// The name scope of the applied template (a content control), for [`ferro_template_part`].
    template: RefCell<Option<NameScopeRef>>,
}

impl Drop for Instance {
    fn drop(&mut self) {
        if let Some(release) = self.release.get() {
            // SAFETY: the language gave this function for this data.
            unsafe { release(self.data.get()) };
        }
    }
}

/// Runs `call` with a handle of `object`: the one remembered while it is still the handle of the
/// object (an index and a generation compared), the handle the object has, or one made for the
/// call (released after it).
#[inline]
fn with_handle<R>(object: &FerroObject, instance: &Instance, call: impl FnOnce(FerroHandle) -> R) -> R {
    let address = handles::address(object);
    let remembered = instance.handle.get();
    if remembered != 0 && handles::is_handle_of(remembered, address) {
        return call(remembered);
    }
    if let Some(handle) = handles::find(address) {
        instance.handle.set(handle);
        return call(handle);
    }
    let Ok(handle) = handles::insert(object.to_ref()) else {
        return call(0);
    };
    let result = call(handle);
    let _ = handles::release(handle);
    result
}

#[cold]
fn callback_failed(class: &ForeignClass, hook: &str, code: FerroStatus) {
    let code = if code < 0 { code } else { crate::FERRO_ERROR };
    Error::new(code, format!("the {hook} callback of the class {} failed ({code})", class.name)).record();
}

/// Calls a notification callback and records its failure.
#[inline]
fn notify(
    object: &FerroObject,
    instance: &Instance,
    class: &ForeignClass,
    hook: &str,
    callback: FerroNotifyFn,
    arg: i64,
) {
    if let Some(callback) = callback {
        // SAFETY: the language gave a function of the signature.
        let code = with_handle(object, instance, |handle| unsafe { callback(instance.data.get(), handle, arg) });
        if code < 0 {
            callback_failed(class, hook, code);
        }
    }
}

/// The out-parameters of the measure and arrange callbacks that run (innermost last), for
/// [`ferro_layout_result`].
static LAYOUT_RESULTS: UiCell<Vec<(*mut f64, *mut f64)>> = UiCell::new(Vec::new());

type LayoutCallback = unsafe extern "C" fn(*mut c_void, FerroHandle, f64, f64, *mut f64, *mut f64) -> FerroStatus;

/// Calls a layout callback: `None` when it fails or gives no size.
#[inline]
fn call_layout(
    object: &FerroObject,
    instance: &Instance,
    class: &ForeignClass,
    hook: &str,
    callback: LayoutCallback,
    size: Size,
) -> Option<Size> {
    let mut width = f64::NAN;
    let mut height = f64::NAN;
    let out = (&mut width as *mut f64, &mut height as *mut f64);
    let _ = LAYOUT_RESULTS.with(|results| results.push(out));
    let code = with_handle(object, instance, |handle| {
        // SAFETY: the language gave a function of the signature; the pointers are to locals that
        // live until the call returns.
        unsafe { callback(instance.data.get(), handle, size.width, size.height, out.0, out.1) }
    });
    let _ = LAYOUT_RESULTS.with(|results| results.pop());
    if code < 0 {
        callback_failed(class, hook, code);
        return None;
    }
    if width.is_nan() || height.is_nan() {
        callback_failed(class, hook, crate::FERRO_ERROR);
        return None;
    }
    Some(Size::new(width, height))
}

/// Gives the size of the measure or arrange callback that runs now (the innermost), for a
/// language that cannot write through the `double *` of the callback. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_layout_result(width: f64, height: f64) -> FerroStatus {
    status(|| {
        let target = LAYOUT_RESULTS.with(|results| results.last().copied())?;
        let (w, h) = target.ok_or_else(|| Error::new(FERRO_INVALID_STATE, "no measure or arrange callback runs"))?;
        // SAFETY: the pointers are to locals of the running call of a callback (popped when it
        // returns).
        unsafe {
            *w = width;
            *h = height;
        }
        Ok(())
    })
}

/// Calls the render callback with a drawing context the `ferro_draw_*` functions reach while it
/// runs; what the callback pushed and did not pop is popped after it.
fn call_render(
    object: &FerroObject,
    instance: &Instance,
    class: &ForeignClass,
    callback: unsafe extern "C" fn(*mut c_void, FerroHandle, i64) -> FerroStatus,
    context: &mut DrawingContext<'_>,
) {
    let pointer = context as *mut DrawingContext<'_> as *mut DrawingContext<'static>;
    let Ok(token) = DRAWING.with(|drawing| {
        drawing.contexts.push((pointer, Vec::new()));
        drawing.contexts.len() as i64
    }) else {
        return;
    };
    // SAFETY: the language gave a function of the signature.
    let code = with_handle(object, instance, |handle| unsafe { callback(instance.data.get(), handle, token) });
    let pushed = DRAWING.with(|drawing| drawing.contexts.pop().map(|(_, pushed)| pushed)).ok().flatten();
    for state in pushed.into_iter().flatten().rev() {
        context.pop(state);
    }
    if code < 0 {
        callback_failed(class, "render", code);
    }
}

// ---- the Rust classes of the bases -------------------------------------------------------------

/// The overrides every foreign class has, whatever its base: each reads the class of the instance
/// and calls its callback, or falls through to the base.
macro_rules! foreign_class {
    ($name:ident : $base:ident, [$($defaults:ident),*]) => {
        #[repr(C)]
        pub(crate) struct $name {
            base: $base,
            instance: Instance,
        }

        ferro_class!($name: $base);
        ferro_impl_classes!($name: StyledElementImpl, InteractiveImpl, ControlImpl $(, $defaults)*);

        impl $name {
            fn construct_instance() -> Ref<FerroObject> {
                instantiate($name { base: $base::construct(), instance: Instance::default() }).upcast()
            }

            fn class(&self) -> Option<&'static ForeignClass> {
                self.instance.class.get()
            }
        }

        ferro_overrides! { impl FerroObjectImpl for $name {
            fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
                Self::parent_on_property_changed(this, change);
                let Some(class) = this.class() else {
                    return;
                };
                let Some(own) = class.own(change.property()) else {
                    return;
                };
                let layoutable: &Layoutable = this.upcast();
                if own.flags & FERRO_PROPERTY_AFFECTS_MEASURE != 0 {
                    layoutable.invalidate_measure();
                }
                if own.flags & FERRO_PROPERTY_AFFECTS_ARRANGE != 0 {
                    layoutable.invalidate_arrange();
                }
                if own.flags & FERRO_PROPERTY_AFFECTS_RENDER != 0 {
                    let visual: &ferroui_base::Visual = this.upcast();
                    visual.invalidate_visual();
                }
                notify(
                    this.upcast(),
                    &this.instance,
                    class,
                    "property changed",
                    class.callbacks.property_changed.get(),
                    own.id,
                );
            }
        } }

        ferro_overrides! { impl VisualImpl for $name {
            fn render(this: &Self, context: &mut DrawingContext) {
                Self::parent_render(this, context);
                if let Some(class) = this.class() {
                    if let Some(callback) = class.callbacks.render.get() {
                        call_render(this.upcast(), &this.instance, class, callback, context);
                    }
                }
            }

            fn on_attached_to_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
                Self::parent_on_attached_to_visual_tree(this, e);
                if let Some(class) = this.class() {
                    notify(this.upcast(), &this.instance, class, "attached", class.callbacks.attached.get(), 0);
                }
            }

            fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
                Self::parent_on_detached_from_visual_tree(this, e);
                if let Some(class) = this.class() {
                    notify(this.upcast(), &this.instance, class, "detached", class.callbacks.detached.get(), 0);
                }
            }
        } }

        ferro_overrides! { impl LayoutableImpl for $name {
            fn measure_override(this: &Self, available_size: Size) -> Size {
                if let Some(class) = this.class() {
                    if let Some(callback) = class.callbacks.measure.get() {
                        let measured = call_layout(this.upcast(), &this.instance, class, "measure", callback, available_size);
                        if let Some(size) = measured {
                            return size;
                        }
                    }
                }
                Self::parent_measure_override(this, available_size)
            }

            fn arrange_override(this: &Self, final_size: Size) -> Size {
                if let Some(class) = this.class() {
                    if let Some(callback) = class.callbacks.arrange.get() {
                        let arranged = call_layout(this.upcast(), &this.instance, class, "arrange", callback, final_size);
                        if let Some(size) = arranged {
                            return size;
                        }
                    }
                }
                Self::parent_arrange_override(this, final_size)
            }
        } }

        ferro_overrides! { impl InputElementImpl for $name {
            fn on_pointer_pressed(this: &Self, e: &PointerPressedEventArgs) {
                Self::parent_on_pointer_pressed(this, e);
                pointer(this, &this.instance, FERRO_POINTER_PRESSED, e);
            }

            fn on_pointer_released(this: &Self, e: &PointerReleasedEventArgs) {
                Self::parent_on_pointer_released(this, e);
                pointer(this, &this.instance, FERRO_POINTER_RELEASED, e);
            }

            fn on_pointer_moved(this: &Self, e: &PointerEventArgs) {
                Self::parent_on_pointer_moved(this, e);
                pointer(this, &this.instance, FERRO_POINTER_MOVED, e);
            }

            fn on_key_down(this: &Self, e: &KeyEventArgs) {
                Self::parent_on_key_down(this, e);
                key(this.upcast(), &this.instance, FERRO_KEY_DOWN, e);
            }

            fn on_key_up(this: &Self, e: &KeyEventArgs) {
                Self::parent_on_key_up(this, e);
                key(this.upcast(), &this.instance, FERRO_KEY_UP, e);
            }
        } }
    };
}

/// Calls the pointer callback of the class of an instance.
fn pointer<T: Upcast<FerroObject> + Upcast<ferroui_base::Visual>>(
    this: &T,
    instance: &Instance,
    kind: i32,
    e: &PointerEventArgs,
) {
    let Some(class) = instance.class.get() else {
        return;
    };
    let Some(callback) = class.callbacks.pointer.get() else {
        return;
    };
    let visual: &ferroui_base::Visual = this.upcast();
    let position = e.get_position(Some(visual));
    let modifiers = i64::from(e.key_modifiers().bits());
    let object: &FerroObject = this.upcast();
    // SAFETY: the language gave a function of the signature.
    let code = with_handle(object, instance, |handle| unsafe {
        callback(instance.data.get(), handle, kind, position.x, position.y, modifiers)
    });
    if code < 0 {
        callback_failed(class, "pointer", code);
    } else if code == 1 {
        e.set_handled(true);
    }
}

/// Calls the key callback of the class of an instance.
fn key(object: &FerroObject, instance: &Instance, kind: i32, e: &KeyEventArgs) {
    let Some(class) = instance.class.get() else {
        return;
    };
    let Some(callback) = class.callbacks.key.get() else {
        return;
    };
    let modifiers = i64::from(e.key_modifiers.bits());
    let key = e.key as i64;
    // SAFETY: the language gave a function of the signature.
    let code =
        with_handle(object, instance, |handle| unsafe { callback(instance.data.get(), handle, kind, key, modifiers) });
    if code < 0 {
        callback_failed(class, "key", code);
    } else if code == 1 {
        e.set_handled(true);
    }
}

foreign_class!(ForeignControl: Control, []);
foreign_class!(ForeignPanel: Panel, [PanelImpl]);
foreign_class!(ForeignDecorator: Decorator, []);
foreign_class!(ForeignContentControl: ContentControl, [ContentControlImpl]);

ferro_overrides! { impl TemplatedControlImpl for ForeignContentControl {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        *this.instance.template.borrow_mut() = Some(e.name_scope().clone());
        if let Some(class) = this.class() {
            notify(this.upcast(), &this.instance, class, "apply template", class.callbacks.apply_template.get(), 0);
        }
    }
} }

/// The owner of the properties registered at run time: a class outside the hierarchy of every
/// other class, so that a property is seen only on the class that registers it (it is registered
/// on that class too).
#[repr(C)]
pub(crate) struct ForeignProperties {
    base: FerroObject,
}

ferro_class!(ForeignProperties: FerroObject);
ferro_impl_classes!(ForeignProperties: FerroObjectImpl);

/// The instance part of a foreign object, `None` for another object.
fn instance_of(object: &Ref<FerroObject>) -> Option<(&Instance, i32)> {
    let class = object.get_type();
    if class.derives_from(ForeignControl::__IDENTITY) {
        let this: &ForeignControl = object.downcast_ref()?;
        return Some((&this.instance, FERRO_BASE_CONTROL));
    }
    if class.derives_from(ForeignPanel::__IDENTITY) {
        let this: &ForeignPanel = object.downcast_ref()?;
        return Some((&this.instance, FERRO_BASE_PANEL));
    }
    if class.derives_from(ForeignDecorator::__IDENTITY) {
        let this: &ForeignDecorator = object.downcast_ref()?;
        return Some((&this.instance, FERRO_BASE_DECORATOR));
    }
    if class.derives_from(ForeignContentControl::__IDENTITY) {
        let this: &ForeignContentControl = object.downcast_ref()?;
        return Some((&this.instance, FERRO_BASE_CONTENT_CONTROL));
    }
    None
}

// ---- classes -----------------------------------------------------------------------------------

/// The most classes a process registers: each has a constructor of its own, a function, so that
/// the type tables and markup create it as any class.
const MAX_CLASSES: usize = 64;

fn construct(index: usize) -> Ref<FerroObject> {
    let class = CLASSES.with(|classes| classes.get(index).copied()).ok().flatten();
    let Some(class) = class else {
        // Not reachable: a constructor is given to a type only with its class.
        return FerroObject::new();
    };
    let object = match class.base {
        FERRO_BASE_PANEL => ForeignPanel::construct_instance(),
        FERRO_BASE_DECORATOR => ForeignDecorator::construct_instance(),
        FERRO_BASE_CONTENT_CONTROL => ForeignContentControl::construct_instance(),
        _ => ForeignControl::construct_instance(),
    };
    // SAFETY: the table is a copy of the table of the Rust class of the base, the most-derived
    // class of the object, with only its type slot changed; the self reference is set already
    // (`instantiate`), so the weak reference given here is not kept.
    unsafe { (*object).attach(std::rc::Weak::<()>::new(), class.vtable) };
    if let Some((instance, _)) = instance_of(&object) {
        instance.class.set(Some(class));
        if let Some(create) = class.callbacks.create.get() {
            // SAFETY: the language gave a function of the signature.
            let code = with_handle(&object, instance, |handle| unsafe { create(class.class_data.get(), handle) });
            if code < 0 {
                callback_failed(class, "create", code);
            }
        }
    }
    object
}

macro_rules! constructors {
    ($($index:literal)*) => {
        [$({
            fn make() -> Ref<FerroObject> {
                construct($index)
            }
            make as fn() -> Ref<FerroObject>
        }),*]
    };
}

static CONSTRUCTORS: [fn() -> Ref<FerroObject>; MAX_CLASSES] = constructors!(
    0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 21 22 23 24 25 26 27 28 29 30 31 32 33 34 35 36 37 38 39
    40 41 42 43 44 45 46 47 48 49 50 51 52 53 54 55 56 57 58 59 60 61 62 63
);

/// The virtual table of the instances of a class on the base `T`: a copy of `T`'s with the type.
fn table_of<T: ObjectType>(type_info: &'static TypeInfo) -> *const () {
    let table: &'static mut T::VTable = Box::leak(Box::new(*T::vtable()));
    let root = table as *mut T::VTable as *mut FerroObjectVTable;
    // SAFETY: the table of a class starts with the table of the root class (`ObjectType`).
    unsafe { (*root).type_info = type_info };
    root as *const ()
}

/// Registers the class `name` (UTF-8, `length` bytes, negative for NUL-terminated) on the base
/// `base` (`FERRO_BASE_CONTROL`, `_PANEL`, `_DECORATOR`, `_CONTENT_CONTROL`) and returns its type
/// ID, or a negative status. The class is created by name ([`ferro_create`](crate::ferro_create)
/// with [`ferro_type_id`](crate::ferro_type_id)), in frames and wherever a class is found by name;
/// its virtual members call the callbacks set with `ferro_class_on_*`, and fall through to the
/// base where none is set. At most 64 classes per process. UI thread only.
#[no_mangle]
pub unsafe extern "C" fn ferro_class_register(name: *const c_char, length: i64, base: i32) -> i64 {
    status_or_value(|| {
        thread::check()?;
        // SAFETY: the caller passes a string as `ferro_type_id` documents.
        let name = unsafe { crate::strings::borrowed(name, length) }?.trim();
        if name.is_empty() || name.contains('.') {
            return Err(Error::new(FERRO_INVALID_ARGUMENT, "a class name is a name without a namespace"));
        }
        if TypeInfo::find_by_name(name).is_some() {
            return Err(Error::new(FERRO_INVALID_STATE, format!("a type is named {name} already")));
        }
        let index = CLASSES.with(|classes| classes.len())?;
        if index >= MAX_CLASSES {
            return Err(Error::new(FERRO_INVALID_STATE, "64 classes are registered already"));
        }
        let name: &'static str = Box::leak(name.to_owned().into_boxed_str());
        let identity: &'static TypeIdentity = Box::leak(Box::new(TypeIdentity::new(name)));
        let (parent, init): (&'static TypeInfo, fn()) = match base {
            FERRO_BASE_CONTROL => (ForeignControl::TYPE, <ForeignControl as ObjectType>::register_class),
            FERRO_BASE_PANEL => (ForeignPanel::TYPE, <ForeignPanel as ObjectType>::register_class),
            FERRO_BASE_DECORATOR => (ForeignDecorator::TYPE, <ForeignDecorator as ObjectType>::register_class),
            FERRO_BASE_CONTENT_CONTROL => {
                (ForeignContentControl::TYPE, <ForeignContentControl as ObjectType>::register_class)
            }
            _ => return Err(Error::new(FERRO_INVALID_ARGUMENT, format!("{base} is not a base of a foreign class"))),
        };
        let type_info: &'static TypeInfo = Box::leak(Box::new(
            TypeInfo::new(name, Some(parent))
                .with_identity(identity)
                .with_module_path(module_path!())
                .with_class_init(init)
                .with_constructor(Some(CONSTRUCTORS[index])),
        ));
        // Without the tables of types the type carries no constructor (the by-use mode,
        // docs/c-abi.md, section 16): it is registered beside it.
        TypeInfo::register_constructor_of(type_info, CONSTRUCTORS[index]);
        let vtable = match base {
            FERRO_BASE_PANEL => table_of::<ForeignPanel>(type_info),
            FERRO_BASE_DECORATOR => table_of::<ForeignDecorator>(type_info),
            FERRO_BASE_CONTENT_CONTROL => table_of::<ForeignContentControl>(type_info),
            _ => table_of::<ForeignControl>(type_info),
        };
        let class: &'static ForeignClass = Box::leak(Box::new(ForeignClass {
            name,
            type_info,
            vtable,
            base,
            class_data: Cell::new(std::ptr::null_mut()),
            callbacks: Callbacks::default(),
            properties: RefCell::new(Vec::new()),
        }));
        init();
        TypeInfo::register(type_info);
        CLASSES.with(|classes| classes.push(class))?;
        crate::ids::intern_type(type_info).ok_or_else(|| Error::new(FERRO_INVALID_STATE, "too many types"))
    })
}

/// The foreign class of a type ID.
fn class_of(type_id: i64) -> Result<&'static ForeignClass> {
    thread::check()?;
    let type_info = crate::ids::type_info(type_id)?;
    CLASSES
        .with(|classes| classes.iter().copied().find(|class| std::ptr::eq(class.type_info, type_info)))?
        .ok_or_else(|| Error::new(FERRO_NOT_FOUND, format!("the type {} is not a foreign class", type_info.name())))
}

/// Sets the callback that gives each new instance of the class its data (`NULL` for none).
/// UI thread only.
#[no_mangle]
pub extern "C" fn ferro_class_on_create(class_id: i64, callback: FerroCreateFn) -> FerroStatus {
    status(|| {
        class_of(class_id)?.callbacks.create.set(callback);
        Ok(())
    })
}

/// Sets the measure callback of the class (`NULL`: the base measures). UI thread only.
#[no_mangle]
pub extern "C" fn ferro_class_on_measure(class_id: i64, callback: FerroLayoutFn) -> FerroStatus {
    status(|| {
        class_of(class_id)?.callbacks.measure.set(callback);
        Ok(())
    })
}

/// Sets the arrange callback of the class (`NULL`: the base arranges). UI thread only.
#[no_mangle]
pub extern "C" fn ferro_class_on_arrange(class_id: i64, callback: FerroLayoutFn) -> FerroStatus {
    status(|| {
        class_of(class_id)?.callbacks.arrange.set(callback);
        Ok(())
    })
}

/// Sets the render callback of the class (`NULL`: the base renders alone). The base renders
/// first (the background of a panel); the callback draws over it. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_class_on_render(class_id: i64, callback: FerroRenderFn) -> FerroStatus {
    status(|| {
        class_of(class_id)?.callbacks.render.set(callback);
        Ok(())
    })
}

/// Sets the callback called when a property the class registered changes, with its ID. UI
/// thread only.
#[no_mangle]
pub extern "C" fn ferro_class_on_property_changed(class_id: i64, callback: FerroNotifyFn) -> FerroStatus {
    status(|| {
        class_of(class_id)?.callbacks.property_changed.set(callback);
        Ok(())
    })
}

/// Sets the callback called when an instance is attached to the visual tree. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_class_on_attached(class_id: i64, callback: FerroNotifyFn) -> FerroStatus {
    status(|| {
        class_of(class_id)?.callbacks.attached.set(callback);
        Ok(())
    })
}

/// Sets the callback called when an instance is detached from the visual tree. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_class_on_detached(class_id: i64, callback: FerroNotifyFn) -> FerroStatus {
    status(|| {
        class_of(class_id)?.callbacks.detached.set(callback);
        Ok(())
    })
}

/// Sets the callback called when the template of an instance is applied (a class on
/// `FERRO_BASE_CONTENT_CONTROL`); [`ferro_template_part`] finds its parts. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_class_on_apply_template(class_id: i64, callback: FerroNotifyFn) -> FerroStatus {
    status(|| {
        class_of(class_id)?.callbacks.apply_template.set(callback);
        Ok(())
    })
}

/// Sets the callback of pointer presses, releases and moves on an instance. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_class_on_pointer(class_id: i64, callback: FerroPointerFn) -> FerroStatus {
    status(|| {
        class_of(class_id)?.callbacks.pointer.set(callback);
        Ok(())
    })
}

/// Sets the callback of key presses and releases on an instance. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_class_on_key(class_id: i64, callback: FerroKeyFn) -> FerroStatus {
    status(|| {
        class_of(class_id)?.callbacks.key.set(callback);
        Ok(())
    })
}

/// Sets the data of the class, given to its create callback. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_class_set_data(class_id: i64, data: *mut c_void) -> FerroStatus {
    status(|| {
        class_of(class_id)?.class_data.set(data);
        Ok(())
    })
}

/// Sets the data of an instance of a foreign class, passed to every callback of the instance, and
/// the function that releases it when the instance goes away (`NULL` for none). UI thread only.
#[no_mangle]
pub extern "C" fn ferro_object_set_data(
    object: FerroHandle,
    data: *mut c_void,
    release: FerroReleaseDataFn,
) -> FerroStatus {
    status(|| {
        let target = handles::get(object)?;
        let (instance, _) = instance_of(&target)
            .ok_or_else(|| Error::new(FERRO_TYPE_MISMATCH, "the object is not of a foreign class"))?;
        instance.data.set(data);
        instance.release.set(release);
        Ok(())
    })
}

/// The data of an instance of a foreign class (`NULL` for none or on a failure, recorded). UI
/// thread only.
#[no_mangle]
pub extern "C" fn ferro_object_data(object: FerroHandle) -> *mut c_void {
    let found = handles::get(object).and_then(|target| {
        instance_of(&target)
            .map(|(instance, _)| instance.data.get())
            .ok_or_else(|| Error::new(FERRO_TYPE_MISMATCH, "the object is not of a foreign class"))
    });
    found.unwrap_or_else(|error| {
        error.record();
        std::ptr::null_mut()
    })
}

// ---- properties --------------------------------------------------------------------------------

/// Registers a property of the foreign class `owner` (a type ID of [`ferro_class_register`]) and
/// returns its property ID, or a negative status. `kind` is a kind of signal (`FERRO_SIGNAL_I64`,
/// `_F64`, `_BOOL`, `_TEXT`); the default is `default_number` (an integer or a boolean as a
/// number) or `default_text` (UTF-8, NUL-terminated, `NULL` for no text). `flags` combine
/// `FERRO_PROPERTY_AFFECTS_MEASURE`, `_ARRANGE`, `_RENDER`, `_INHERITS` and `_ATTACHED` (an
/// attached property, `Owner.Name`, that every control takes). The property is a styled property
/// as the ones of the framework: the setters and getters of the ABI, frames, signals, styles,
/// bindings and animations use it. UI thread only.
#[no_mangle]
pub unsafe extern "C" fn ferro_property_register(
    owner: i64,
    name: *const c_char,
    length: i64,
    kind: i32,
    default_number: f64,
    default_text: *const c_char,
    flags: i64,
) -> i64 {
    status_or_value(|| {
        let class = class_of(owner)?;
        // SAFETY: the caller passes strings as `ferro_set_str` documents.
        let name = unsafe { crate::strings::borrowed(name, length) }?.trim().to_owned();
        if name.is_empty() || name.contains('.') {
            return Err(Error::new(FERRO_INVALID_ARGUMENT, "a property name is a name without its owner"));
        }
        let text = if default_text.is_null() {
            None
        } else {
            // SAFETY: as above.
            Some(unsafe { crate::strings::borrowed(default_text, -1) }?.to_owned())
        };
        let attached = flags & FERRO_PROPERTY_ATTACHED != 0;
        let inherits = flags & FERRO_PROPERTY_INHERITS != 0;
        let owner = class.type_info;
        let property = match kind {
            crate::FERRO_SIGNAL_I64 => register(owner, &name, default_number as i64, inherits, attached),
            crate::FERRO_SIGNAL_F64 => register(owner, &name, default_number, inherits, attached),
            crate::FERRO_SIGNAL_BOOL => register(owner, &name, default_number != 0.0, inherits, attached),
            crate::FERRO_SIGNAL_TEXT => register(owner, &name, text, inherits, attached),
            _ => return Err(Error::new(FERRO_INVALID_ARGUMENT, format!("{kind} is not a kind of property"))),
        };
        let qualified = if attached { format!("{}.{name}", class.name) } else { name };
        // SAFETY: a pointer and the length of a Rust string.
        let id = unsafe { crate::ferro_property_id(qualified.as_ptr().cast(), qualified.len() as i64) };
        if id < 0 {
            return Ok(id);
        }
        class.properties.borrow_mut().push(OwnProperty { property, id, flags });
        Ok(id)
    })
}

/// A styled (or attached) property of `T` owned by [`ForeignProperties`] and registered on the
/// class `owner` too, where it is found by name (and on its subclasses; `Owner.Name` for an
/// attached one).
fn register<T: ferroui_base::StyledValue>(
    owner: &'static TypeInfo,
    name: &str,
    default: T,
    inherits: bool,
    attached: bool,
) -> &'static FerroProperty {
    let options = StyledPropertyOptions::new(default).inherits(inherits);
    let registry = FerroPropertyRegistry::instance();
    if attached {
        let property = FerroProperty::register_attached_with::<ForeignProperties, Control, T>(name, options);
        registry.register(owner, property);
        property.as_property()
    } else {
        let property = FerroProperty::register_with::<ForeignProperties, T>(name, options);
        registry.register(owner, property);
        property.as_property()
    }
}

// ---- layout helpers ----------------------------------------------------------------------------

fn layoutable(object: FerroHandle) -> Result<Ref<Layoutable>> {
    handles::get(object)?
        .cast::<Layoutable>()
        .ok_or_else(|| Error::new(FERRO_TYPE_MISMATCH, "the object is not a layoutable element"))
}

/// Measures `object` (a child, from a measure callback) at the available size. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_layout_measure(object: FerroHandle, width: f64, height: f64) -> FerroStatus {
    status(|| {
        layoutable(object)?.measure(Size::new(width, height));
        Ok(())
    })
}

/// Arranges `object` (a child, from an arrange callback) in the rectangle. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_layout_arrange(object: FerroHandle, x: f64, y: f64, width: f64, height: f64) -> FerroStatus {
    status(|| {
        layoutable(object)?.arrange(Rect::new(x, y, width, height));
        Ok(())
    })
}

/// The desired width of `object` after its measure (`0` on a failure, recorded). UI thread only.
#[no_mangle]
pub extern "C" fn ferro_layout_desired_width(object: FerroHandle) -> f64 {
    layoutable(object).map(|element| element.desired_size().width).unwrap_or_else(|error| {
        error.record();
        0.0
    })
}

/// The desired height of `object` after its measure (`0` on a failure, recorded). UI thread only.
#[no_mangle]
pub extern "C" fn ferro_layout_desired_height(object: FerroHandle) -> f64 {
    layoutable(object).map(|element| element.desired_size().height).unwrap_or_else(|error| {
        error.record();
        0.0
    })
}

/// The width `object` was arranged at (its bounds; `0` on a failure, recorded): what a render
/// callback of a language without instance data draws in. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_layout_bounds_width(object: FerroHandle) -> f64 {
    layoutable(object).map(|element| element.bounds().width).unwrap_or_else(|error| {
        error.record();
        0.0
    })
}

/// The height `object` was arranged at (its bounds; `0` on a failure, recorded). UI thread only.
#[no_mangle]
pub extern "C" fn ferro_layout_bounds_height(object: FerroHandle) -> f64 {
    layoutable(object).map(|element| element.bounds().height).unwrap_or_else(|error| {
        error.record();
        0.0
    })
}

/// The number of children of a panel (`0` for another object, recorded). UI thread only.
#[no_mangle]
pub extern "C" fn ferro_panel_child_count(object: FerroHandle) -> i64 {
    status_or_value(|| {
        let panel = handles::get(object)?
            .cast::<Panel>()
            .ok_or_else(|| Error::new(FERRO_TYPE_MISMATCH, "the object is not a panel"))?;
        Ok(panel.children().count() as i64)
    })
}

/// The child `index` of a panel, with one reference counted (release it), or `0` (recorded). UI
/// thread only.
#[no_mangle]
pub extern "C" fn ferro_panel_child(object: FerroHandle, index: i64) -> FerroHandle {
    let found = (|| -> Result<FerroHandle> {
        let panel = handles::get(object)?
            .cast::<Panel>()
            .ok_or_else(|| Error::new(FERRO_TYPE_MISMATCH, "the object is not a panel"))?;
        let children = panel.children();
        let index = usize::try_from(index)
            .ok()
            .filter(|&index| index < children.count())
            .ok_or_else(|| Error::new(FERRO_INVALID_ARGUMENT, format!("{index} is not the index of a child")))?;
        handles::insert(children.get(index).upcast())
    })();
    found.unwrap_or_else(|error| {
        error.record();
        0
    })
}

/// The part named `name` of the applied template of `object` (a foreign class on
/// `FERRO_BASE_CONTENT_CONTROL`), with one reference counted (release it), or `0` with
/// [`FERRO_NOT_FOUND`]. From the apply-template callback on. UI thread only.
#[no_mangle]
pub unsafe extern "C" fn ferro_template_part(object: FerroHandle, name: *const c_char, length: i64) -> FerroHandle {
    let found = (|| -> Result<FerroHandle> {
        // SAFETY: the caller passes a string as `ferro_set_str` documents.
        let name = unsafe { crate::strings::borrowed(name, length) }?;
        let target = handles::get(object)?;
        let (instance, _) = instance_of(&target)
            .ok_or_else(|| Error::new(FERRO_TYPE_MISMATCH, "the object is not of a foreign class"))?;
        let part = instance.template.borrow().as_ref().and_then(|scope| scope.0.find(name));
        let part = part.ok_or_else(|| Error::new(FERRO_NOT_FOUND, format!("the template has no part named {name}")))?;
        handles::insert(part)
    })();
    found.unwrap_or_else(|error| {
        error.record();
        0
    })
}

// ---- drawing -----------------------------------------------------------------------------------

/// The drawing contexts of the render callbacks that run (innermost last) with what each pushed,
/// and the brushes and pens made for drawing.
struct Drawing {
    contexts: Vec<(*mut DrawingContext<'static>, Vec<PushedState>)>,
    brushes: Vec<(u32, Rc<dyn IBrush>)>,
    pens: Vec<(u32, u64, Rc<dyn IPen>)>,
}

static DRAWING: UiCell<Drawing> = UiCell::new(Drawing { contexts: Vec::new(), brushes: Vec::new(), pens: Vec::new() });

/// A solid brush of the colour `argb` (`0xAARRGGBB`) for the `ferro_draw_*` functions: its ID (a
/// positive number), the same for the same colour, or a negative status. Make the brushes of a
/// control once (at its creation) and draw with their IDs. UI thread only.
#[no_mangle]
pub extern "C" fn ferro_draw_brush(argb: i64) -> i64 {
    status_or_value(|| {
        let color = argb as u32;
        DRAWING.with(|drawing| {
            if let Some(index) = drawing.brushes.iter().position(|(known, _)| *known == color) {
                return index as i64 + 1;
            }
            let brush: Rc<dyn IBrush> = Rc::new(ImmutableSolidColorBrush::from_uint32(color));
            drawing.brushes.push((color, brush));
            drawing.brushes.len() as i64
        })
    })
}

/// A solid pen of the colour `argb` and the thickness `thickness`: its ID, or a negative status.
/// UI thread only.
#[no_mangle]
pub extern "C" fn ferro_draw_pen(argb: i64, thickness: f64) -> i64 {
    status_or_value(|| {
        let color = argb as u32;
        let bits = thickness.to_bits();
        DRAWING.with(|drawing| {
            if let Some(index) = drawing.pens.iter().position(|(c, t, _)| *c == color && *t == bits) {
                return index as i64 + 1;
            }
            let pen: Rc<dyn IPen> = Rc::new(ImmutablePen::from_uint32(color, thickness));
            drawing.pens.push((color, bits, pen));
            drawing.pens.len() as i64
        })
    })
}

/// Runs `draw` on the drawing context `context` (a token of a render callback that runs) with the
/// brush and the pen of the IDs (`0` for none).
fn draw(
    context: i64,
    brush: i64,
    pen: i64,
    draw: impl FnOnce(&mut DrawingContext<'static>, Option<&Rc<dyn IBrush>>, Option<&Rc<dyn IPen>>, &mut Vec<PushedState>),
) -> FerroStatus {
    status(|| {
        DRAWING.with(|drawing| {
            let current = drawing.contexts.len() as i64;
            if context != current || current == 0 {
                return Err(Error::new(
                    FERRO_INVALID_STATE,
                    "the drawing context is not the one of the running render",
                ));
            }
            let pick = |id: i64, len: usize| -> Result<Option<usize>> {
                match id {
                    0 => Ok(None),
                    id if id > 0 && (id as usize) <= len => Ok(Some(id as usize - 1)),
                    _ => Err(Error::new(FERRO_INVALID_ARGUMENT, format!("{id} is not a brush or a pen"))),
                }
            };
            let Drawing { contexts, brushes, pens } = drawing;
            let brush = pick(brush, brushes.len())?.map(|index| &brushes[index].1);
            let pen = pick(pen, pens.len())?.map(|index| &pens[index].2);
            let (pointer, pushed) = contexts.last_mut().expect("a context runs");
            // SAFETY: the context of the running render callback: it lives until the callback
            // returns, when it is taken off this list.
            let target = unsafe { &mut **pointer };
            draw(target, brush, pen, pushed);
            Ok(())
        })?
    })
}

/// Fills (`brush`) and strokes (`pen`) a rectangle, its corners rounded by `radius_x` and
/// `radius_y`. UI thread only, in a render callback.
#[no_mangle]
pub extern "C" fn ferro_draw_rect(
    context: i64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    radius_x: f64,
    radius_y: f64,
    brush: i64,
    pen: i64,
) -> FerroStatus {
    draw(context, brush, pen, |target, brush, pen, _| {
        let rect = RoundedRect::from_radius_xy(Rect::new(x, y, width, height), radius_x, radius_y);
        target.draw_rounded_rectangle(brush, pen, rect, &BoxShadows::default());
    })
}

/// Fills and strokes the ellipse in the rectangle. UI thread only, in a render callback.
#[no_mangle]
pub extern "C" fn ferro_draw_ellipse(
    context: i64,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    brush: i64,
    pen: i64,
) -> FerroStatus {
    draw(context, brush, pen, |target, brush, pen, _| target.draw_ellipse(brush, pen, Rect::new(x, y, width, height)))
}

/// Strokes a line with the pen. UI thread only, in a render callback.
#[no_mangle]
pub extern "C" fn ferro_draw_line(context: i64, x1: f64, y1: f64, x2: f64, y2: f64, pen: i64) -> FerroStatus {
    if pen == 0 {
        return Error::new(FERRO_INVALID_ARGUMENT, "a line needs a pen").record();
    }
    draw(context, 0, pen, |target, _, pen, _| {
        if let Some(pen) = pen {
            target.draw_line(pen, Point::new(x1, y1), Point::new(x2, y2));
        }
    })
}

/// Fills and strokes the geometry of the path data `data` (the mini-language of markup: `M 0,0 L
/// 10,10 Z`, UTF-8, `length` bytes, negative for NUL-terminated). UI thread only, in a render
/// callback.
#[no_mangle]
pub unsafe extern "C" fn ferro_draw_path(
    context: i64,
    data: *const c_char,
    length: i64,
    brush: i64,
    pen: i64,
) -> FerroStatus {
    status(|| {
        // SAFETY: the caller passes a string as `ferro_set_str` documents.
        let text = unsafe { crate::strings::borrowed(data, length) }?;
        let geometry = Geometry::parse(text)
            .map_err(|error| Error::new(crate::FERRO_INVALID_VALUE, format!("the path is not valid: {error}")))?;
        match draw(context, brush, pen, |target, brush, pen, _| target.draw_geometry(brush, pen, &geometry)) {
            code if code < 0 => Err(Error::new(code, crate::error::last_message())),
            _ => Ok(()),
        }
    })
}

/// Draws `text` (UTF-8, `length` bytes, negative for NUL-terminated) with its top left at `x`,
/// `y`, in the default typeface at `size`, with the brush. UI thread only, in a render callback.
#[no_mangle]
pub unsafe extern "C" fn ferro_draw_text(
    context: i64,
    text: *const c_char,
    length: i64,
    x: f64,
    y: f64,
    size: f64,
    brush: i64,
) -> FerroStatus {
    // SAFETY: the caller passes a string as `ferro_set_str` documents.
    let text = match unsafe { crate::strings::borrowed(text, length) } {
        Ok(text) => text,
        Err(error) => return error.record(),
    };
    if !(size > 0.0 && size < 1.0e5) {
        return Error::new(FERRO_INVALID_ARGUMENT, "the font size is not a positive number").record();
    }
    draw(context, brush, 0, |target, brush, _, _| {
        let formatted = FormattedText::new(
            text,
            Locale::UNKNOWN,
            FlowDirection::LeftToRight,
            Typeface::default_typeface(),
            size,
            brush.cloned(),
        );
        target.draw_text(&formatted, Point::new(x, y));
    })
}

/// Clips what is drawn next to the rectangle, until [`ferro_draw_pop`]. UI thread only, in a
/// render callback.
#[no_mangle]
pub extern "C" fn ferro_draw_push_clip(context: i64, x: f64, y: f64, width: f64, height: f64) -> FerroStatus {
    draw(context, 0, 0, |target, _, _, pushed| pushed.push(target.push_clip(Rect::new(x, y, width, height))))
}

/// Transforms what is drawn next by the matrix (`m11 m12 m21 m22 dx dy`), until
/// [`ferro_draw_pop`]. UI thread only, in a render callback.
#[no_mangle]
pub extern "C" fn ferro_draw_push_transform(
    context: i64,
    m11: f64,
    m12: f64,
    m21: f64,
    m22: f64,
    dx: f64,
    dy: f64,
) -> FerroStatus {
    draw(context, 0, 0, |target, _, _, pushed| {
        pushed.push(target.push_transform(Matrix::new(m11, m12, m21, m22, dx, dy)));
    })
}

/// Draws what is drawn next with the opacity, until [`ferro_draw_pop`]. UI thread only, in a
/// render callback.
#[no_mangle]
pub extern "C" fn ferro_draw_push_opacity(context: i64, opacity: f64) -> FerroStatus {
    draw(context, 0, 0, |target, _, _, pushed| pushed.push(target.push_opacity(opacity)))
}

/// Pops the clip, transform or opacity pushed last in the context. UI thread only, in a render
/// callback.
#[no_mangle]
pub extern "C" fn ferro_draw_pop(context: i64) -> FerroStatus {
    let mut empty = false;
    let code = draw(context, 0, 0, |target, _, _, pushed| match pushed.pop() {
        Some(state) => target.pop(state),
        None => empty = true,
    });
    if code == crate::FERRO_OK && empty {
        return Error::new(FERRO_INVALID_STATE, "nothing is pushed in the context").record();
    }
    code
}
