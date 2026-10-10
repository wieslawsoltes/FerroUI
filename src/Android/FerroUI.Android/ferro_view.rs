//! The view that shows content of the framework in the view hierarchy.
//!
//! The view of the system is a class of the Java layer (`FerroView`, a
//! frame layout), which forwards what the system tells a view; this is the
//! object behind it.

use crate::interop::java::{call_int, call_long, call_void, new_object, JavaClass, JavaObject, JavaValue};
use crate::interop::natives::{next_handle, FERRO_VIEW};
use crate::platform::skia_platform::TopLevelImpl;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::embedding::EmbeddableControlRoot;
use ferroui_controls::platform::ITopLevelImpl;
use ferroui_controls::TopLevel;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

thread_local! {
    static VIEWS: RefCell<HashMap<i64, Rc<FerroView>>> = RefCell::new(HashMap::new());
}

/// `View.VISIBLE`.
const VISIBLE: i32 = 0;

pub struct FerroView {
    handle: i64,
    java: JavaObject,
    root: RefCell<Option<Ref<EmbeddableControlRoot>>>,
    view: Rc<TopLevelImpl>,

    is_rendering: Cell<bool>,
    surface_created: Cell<bool>,
}

impl FerroView {
    /// Creates a view in `context` (an activity, or another context of the
    /// system).
    pub fn new(context: &JavaObject) -> Rc<FerroView> {
        // The constructor of the Java view creates this object (`native_create`).
        let java = new_object(
            &JavaClass::find(FERRO_VIEW),
            "(Landroid/content/Context;)V",
            &[JavaValue::Object(Some(context))],
        );
        let handle = call_long(&java, "getNativeHandle", "()J", &[]);
        match Self::from_handle(handle) {
            Some(view) => view,
            None => panic!("Unknown error: the initialization of the view has failed."),
        }
    }

    /// The view the Java view with `handle` belongs to.
    pub(crate) fn from_handle(handle: i64) -> Option<Rc<FerroView>> {
        VIEWS.with(|views| views.borrow().get(&handle).cloned())
    }

    /// The constructor of the Java view `java` runs, in `context`: creates
    /// the object of the view and returns its number.
    pub(crate) fn native_create(java: JavaObject, context: JavaObject) -> i64 {
        // `ViewImpl` of the reference: a top-level that raises the lost focus of the
        // top-level when its view loses the focus.
        let view = TopLevelImpl::new(&context, false);
        view.set_focus_change(Some(Rc::new({
            let view = Rc::downgrade(&view);
            move |has_focus: bool| {
                if !has_focus {
                    let lost_focus = view.upgrade().and_then(|view| view.lost_focus());
                    if let Some(lost_focus) = lost_focus {
                        lost_focus();
                    }
                }
            }
        })));

        if let Some(surface_view) = view.view() {
            call_void(&java, "addView", "(Landroid/view/View;)V", &[JavaValue::Object(Some(&surface_view))]);
        }

        let top_level_impl: Rc<dyn ITopLevelImpl> = view.clone();
        let root = EmbeddableControlRoot::with_impl(top_level_impl);
        root.prepare();

        // The transparent background and the first configuration follow in the constructor
        // of the Java view, once this function has returned the number of the object.

        let handle = next_handle();
        let this = Rc::new(FerroView {
            handle,
            java,
            root: RefCell::new(Some(root)),
            view,
            is_rendering: Cell::new(false),
            surface_created: Cell::new(false),
        });

        if let Some(internal_view) = this.view.internal_view() {
            let weak: Weak<FerroView> = Rc::downgrade(&this);
            internal_view.surface_window_created(Rc::new({
                let weak = weak.clone();
                move || {
                    if let Some(this) = weak.upgrade() {
                        this.internal_view_surface_window_created();
                    }
                }
            }));
            internal_view.surface_window_destroyed(Rc::new(move || {
                if let Some(this) = weak.upgrade() {
                    this.internal_view_surface_window_destroyed();
                }
            }));
        }

        // Stage 3 of docs/porting/android-platform.md: the accessibility helper of the view.

        VIEWS.with(|views| views.borrow_mut().insert(handle, this));
        handle
    }

    fn internal_view_surface_window_created(&self) {
        self.surface_created.set(true);

        if call_int(&self.java, "getVisibility", "()I", &[]) == VISIBLE {
            self.on_visibility_changed(true);

            let root = self.root.borrow().clone();
            if let Some(root) = root {
                root.invalidate_measure();
            }
            call_void(&self.java, "invalidate", "()V", &[]);
        }
    }

    fn internal_view_surface_window_destroyed(&self) {
        self.on_visibility_changed(false);
        self.surface_created.set(false);
    }

    /// The Java view.
    pub fn java_object(&self) -> &JavaObject {
        &self.java
    }

    pub(crate) fn top_level_impl(&self) -> &Rc<TopLevelImpl> {
        &self.view
    }

    /// The top-level of the view; `None` once the view is disposed.
    pub fn top_level(&self) -> Option<Ref<TopLevel>> {
        self.root.borrow().clone().map(|root| root.upcast())
    }

    /// The content of the view.
    pub fn content(&self) -> Option<BoxedValue> {
        self.root.borrow().as_ref().and_then(|root| root.content())
    }

    /// Sets the content of the view.
    pub fn set_content(&self, value: Option<BoxedValue>) {
        let root = self.root.borrow().clone();
        if let Some(root) = root {
            root.set_content(value);
        }
    }

    /// Releases the top-level of the view. The view cannot be used
    /// afterwards.
    pub fn dispose(&self) {
        self.on_visibility_changed(false);
        self.surface_created.set(false);
        let root = self.root.borrow_mut().take();
        if let Some(root) = root {
            root.dispose();
        }
        let removed = VIEWS.with(|views| views.borrow_mut().remove(&self.handle));
        drop(removed);
    }

    pub(crate) fn on_visibility_changed(&self, is_visible: bool) {
        let root = self.root.borrow().clone();
        let Some(root) = root else {
            return;
        };
        if !self.surface_created.get() {
            return;
        }

        if is_visible && !self.is_rendering.get() {
            self.is_rendering.set(true);
            root.start_rendering();

            if let Some(insets_manager) = self.view.insets_manager() {
                insets_manager.apply_status_bar_state();
            }
        } else if !is_visible && self.is_rendering.get() {
            self.is_rendering.set(false);
            root.stop_rendering();
        }
    }

    /// The configuration of the view changed, or the view was attached to
    /// its window. `night` is whether the configuration is in night mode.
    pub(crate) fn send_configuration_changed(&self, has_configuration: bool, night: bool) {
        if let Some(insets_manager) = self.view.insets_manager() {
            insets_manager.set_default_system_light_mode(!(has_configuration && night));
        }
        if has_configuration {
            self.view.screens().on_changed();
        }
    }

    /// The layout of the view tree changed (`GlobalLayoutListener` of the
    /// activity in the reference).
    pub(crate) fn on_global_layout(&self) {
        self.view.resize(ITopLevelImpl::client_size(&*self.view));
    }
}
