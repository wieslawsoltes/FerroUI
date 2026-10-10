//! An activity that shows a view of the framework.
//!
//! The activity of the system is a class of the Java layer
//! (`FerroActivity`), which forwards its lifecycle; this is the object
//! behind it, from `onCreate` to `onDestroy`.
//!
//! Stage 2 of docs/porting/android-platform.md: the back button
//! (`OnBackPressed`, `OnBackInvoked`, `BackRequested`, the back pressed
//! callback), the results of activities and of permission requests, and
//! the intents (`OnNewIntent`, `HandleIntent`: protocol and file
//! activation). The Java activity does not forward them yet.

use crate::ferro_main_activity::FerroMainActivity;
use crate::ferro_view::FerroView;
use crate::i_ferro_activity::IFerroActivity;
use crate::interop::java::{call_boolean, call_object, call_void, string_of, JavaObject, JavaValue};
use crate::interop::natives::next_handle;
use crate::platform::AndroidActivatableLifetime;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{BoxedValue, FerroLocator, LocatorExtensions};
use ferroui_controls::application_lifetimes::{ActivatedEventArgs, ActivationKind};
use ferroui_controls::{Application, Control};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

thread_local! {
    static ACTIVITIES: RefCell<HashMap<i64, Rc<FerroActivity>>> = RefCell::new(HashMap::new());
}

type ActivatedHandlers = Rc<HandlerList<dyn Fn(&ActivatedEventArgs)>>;

/// Common implementation of an android activity that is integrated with
/// views of the framework. The main activity of an application is a
/// [`FerroMainActivity`].
pub struct FerroActivity {
    handle: i64,
    java: JavaObject,
    is_main_activity: bool,
    on_activated: ActivatedHandlers,
    on_deactivated: ActivatedHandlers,
    content: RefCell<Option<BoxedValue>>,
    content_view_set: Cell<bool>,
    view: RefCell<Option<Rc<FerroView>>>,
}

/// Whether two contents are the same object (`_content != value`).
fn same_content(a: &Option<BoxedValue>, b: &Option<BoxedValue>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => match (Control::from_boxed(a), Control::from_boxed(b)) {
            (Some(a), Some(b)) => a == b,
            (None, None) => Rc::ptr_eq(a, b),
            _ => false,
        },
        _ => false,
    }
}

fn subscribe(handlers: &ActivatedHandlers, handler: Rc<dyn Fn(&ActivatedEventArgs)>) -> Rc<dyn IDisposable> {
    let token = handlers.add(handler);
    let handlers = handlers.clone();
    Disposable::create(move || {
        handlers.remove(token);
    })
}

fn raise(handlers: &ActivatedHandlers, event_args: ActivatedEventArgs) {
    for (_, handler) in handlers.snapshot().iter() {
        handler(&event_args);
    }
}

impl FerroActivity {
    /// The activity the Java activity with `handle` belongs to.
    pub(crate) fn from_handle(handle: i64) -> Option<Rc<FerroActivity>> {
        ACTIVITIES.with(|activities| activities.borrow().get(&handle).cloned())
    }

    /// The main activity that was resumed last and is not destroyed.
    ///
    /// Not from the reference, where an application derives its own class
    /// from the main activity and so has the object: an application written
    /// in Rust reaches its activity through this.
    pub fn current_main_activity() -> Option<Rc<FerroActivity>> {
        let lifetime = FerroLocator::current().get_service::<AndroidActivatableLifetime>()?;
        let handle = lifetime.current_main_activity()?.as_ferro_activity_handle()?;
        Self::from_handle(handle)
    }

    /// The activity of the system.
    pub fn java_object(&self) -> &JavaObject {
        &self.java
    }

    /// Whether this is the main activity of the application.
    pub fn is_main_activity(&self) -> bool {
        self.is_main_activity
    }

    /// The view of the activity; `None` once the activity is destroyed.
    pub fn view(&self) -> Option<Rc<FerroView>> {
        self.view.borrow().clone()
    }

    pub(crate) fn set_view(&self, view: Option<Rc<FerroView>>) {
        *self.view.borrow_mut() = view;
    }

    /// `onCreate` of the Java activity `java`, before the activity of the
    /// system is created: creates the object of the activity and its view,
    /// and returns the number of the object.
    pub(crate) fn on_create(java: JavaObject, is_main_activity: bool) -> i64 {
        let handle = next_handle();
        let this = Rc::new(FerroActivity {
            handle,
            java,
            is_main_activity,
            on_activated: Rc::new(HandlerList::new()),
            on_deactivated: Rc::new(HandlerList::new()),
            content: RefCell::new(None),
            content_view_set: Cell::new(false),
            view: RefCell::new(None),
        });
        ACTIVITIES.with(|activities| activities.borrow_mut().insert(handle, this.clone()));

        let content = this.content.borrow().clone();
        this.initialize_view(content);
        handle
    }

    /// `onCreate`, after the activity of the system was created.
    pub(crate) fn on_created(self: &Rc<Self>) {
        if let Some(activatable_lifetime) = FerroLocator::current().get_service::<AndroidActivatableLifetime>() {
            let activity: Rc<dyn IFerroActivity> = self.clone();
            activatable_lifetime.set_current_intend_activity(Some(activity));
        }

        // Stage 2: `HandleIntent(Intent)`.
    }

    pub(crate) fn on_stop(&self) {
        raise(&self.on_deactivated, ActivatedEventArgs::new(ActivationKind::Background));

        // Stage 2: the back pressed callback is removed here.
    }

    pub(crate) fn on_start(&self) {
        raise(&self.on_activated, ActivatedEventArgs::new(ActivationKind::Background));

        // Stage 2: the back pressed callback is added here.
    }

    pub(crate) fn on_resume(self: &Rc<Self>) {
        // The cutout mode of the window is set by the Java activity, before this call.

        // We inform the ContentView that it has become visible. OnVisibleChanged() sometimes doesn't get called.
        let view = self.view();
        if let Some(view) = view {
            view.on_visibility_changed(true);
        }

        if self.is_main_activity {
            FerroMainActivity::on_resume(self);
        }
    }

    pub(crate) fn on_destroy(self: &Rc<Self>) {
        let view = self.view.borrow_mut().take();
        if let Some(view) = view {
            if self.content_view_set.get() {
                call_void(view.java_object(), "removeGlobalLayoutListener", "()V", &[]);
            }
            view.set_content(None);
            view.dispose();
        }

        if self.is_main_activity {
            FerroMainActivity::on_destroy(self);
        }

        // Not in the reference, which keeps the last activity that was created as the
        // activity of intents until another one is: a destroyed activity is let go.
        if let Some(activatable_lifetime) = FerroLocator::current().get_service::<AndroidActivatableLifetime>() {
            let is_current = activatable_lifetime
                .current_intend_activity()
                .is_some_and(|current| current.as_ferro_activity_handle() == Some(self.handle));
            if is_current {
                activatable_lifetime.set_current_intend_activity(None);
            }
        }

        let removed = ACTIVITIES.with(|activities| activities.borrow_mut().remove(&self.handle));
        drop(removed);
    }

    fn initialize_view(self: &Rc<Self>, initial_content: Option<BoxedValue>) {
        if self.is_main_activity {
            FerroMainActivity::initialize_view(self, initial_content);
            return;
        }

        if Application::current().is_none() {
            panic!("The application was not initialized. Make sure you have created a FerroMainActivity.");
        }

        let view = FerroView::new(&self.java);
        view.set_content(initial_content);
        self.set_view(Some(view));
    }

    // ---- what an application written in Rust needs of its activity; not from the reference ----

    /// Finishes the activity (`Activity.finish`).
    pub fn finish(&self) {
        call_void(&self.java, "finish", "()V", &[]);
    }

    /// The directory of the files of the application
    /// (`Context.getFilesDir`).
    pub fn files_dir(&self) -> Option<String> {
        let directory = call_object(&self.java, "getFilesDir", "()Ljava/io/File;", &[])?;
        let path = call_object(&directory, "getAbsolutePath", "()Ljava/lang/String;", &[])?;
        Some(string_of(&path))
    }

    /// A string extra of the intent that started the activity
    /// (`getIntent().getStringExtra(name)`).
    pub fn intent_string_extra(&self, name: &str) -> Option<String> {
        let intent = call_object(&self.java, "getIntent", "()Landroid/content/Intent;", &[])?;
        let value = call_object(
            &intent,
            "getStringExtra",
            "(Ljava/lang/String;)Ljava/lang/String;",
            &[JavaValue::String(name)],
        )?;
        Some(string_of(&value))
    }
}

impl IFerroActivity for FerroActivity {
    fn content(&self) -> Option<BoxedValue> {
        self.content.borrow().clone()
    }

    fn set_content(&self, value: Option<BoxedValue>) {
        if same_content(&self.content.borrow(), &value) {
            return;
        }
        let previous = self.content.replace(value.clone());
        drop(previous);

        let view = self.view();
        if let Some(view) = view {
            if !self.content_view_set.replace(true) {
                call_void(
                    &self.java,
                    "setContentView",
                    "(Landroid/view/View;)V",
                    &[JavaValue::Object(Some(view.java_object()))],
                );

                // By default, the view isn't focused if the activity is created anew, so we force focus.
                call_boolean(view.java_object(), "requestFocus", "()Z", &[]);

                call_void(view.java_object(), "addGlobalLayoutListener", "()V", &[]);
            }

            view.set_content(value);
        }
    }

    fn activated(&self, handler: Rc<dyn Fn(&ActivatedEventArgs)>) -> Rc<dyn IDisposable> {
        subscribe(&self.on_activated, handler)
    }

    fn deactivated(&self, handler: Rc<dyn Fn(&ActivatedEventArgs)>) -> Rc<dyn IDisposable> {
        subscribe(&self.on_deactivated, handler)
    }

    fn move_task_to_back(&self, non_root: bool) -> bool {
        call_boolean(&self.java, "moveTaskToBack", "(Z)Z", &[JavaValue::Boolean(non_root)])
    }

    fn as_ferro_activity_handle(&self) -> Option<i64> {
        Some(self.handle)
    }
}
