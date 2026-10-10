//! An activity that shows a view of the framework.
//!
//! The activity of the system is a class of the Java layer
//! (`FerroActivity`), which forwards its lifecycle; this is the object
//! behind it, from `onCreate` to `onDestroy`.
//!

use crate::back_pressed_callback::BackPressedCallback;
use crate::ferro_main_activity::FerroMainActivity;
use crate::ferro_view::FerroView;
use crate::i_activity_result_handler::{ActivityResultHandler, IActivityResultHandler, RequestPermissionsResultHandler};
use crate::i_android_navigation_service::{AndroidBackRequestedEventArgs, IActivityNavigationService};
use crate::i_ferro_activity::IFerroActivity;
use crate::interop::java::{call_boolean, call_object, call_void, string_of, JavaObject, JavaRef, JavaValue};
use crate::interop::natives::{next_handle, sdk_int};
use crate::platform::AndroidActivatableLifetime;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::utilities::{HandlerList, Uri, UriKind};
use ferroui_base::{BoxedValue, FerroLocator, LocatorExtensions};
use ferroui_controls::application_lifetimes::{
    ActivatedEventArgs, ActivationKind, FileActivatedEventArgs, ProtocolActivatedEventArgs,
};
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
    current_back_pressed_callback: RefCell<Option<BackPressedCallback>>,
    should_navigate_back: Cell<bool>,
    activity_result: RefCell<Option<ActivityResultHandler>>,
    request_permissions_result: RefCell<Option<RequestPermissionsResultHandler>>,
    back_requested: Rc<HandlerList<dyn Fn(&AndroidBackRequestedEventArgs)>>,
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

    /// The activity whose activity of the system is `java`.
    pub(crate) fn from_java(java: &dyn JavaRef) -> Option<Rc<FerroActivity>> {
        ACTIVITIES.with(|activities| {
            activities.borrow().values().find(|activity| activity.java.is_same_object(java)).cloned()
        })
    }

    /// Gets whether to call the default back handler after our back handler is called.
    pub(crate) fn should_navigate_back(&self) -> bool {
        self.should_navigate_back.replace(false)
    }

    /// `onBackPressed` of the activity below API 33. The answer is whether
    /// the request was handled; the base class is called when it was not.
    pub(crate) fn on_back_pressed(&self) -> bool {
        let event_args = AndroidBackRequestedEventArgs::new();

        self.raise_back_requested(&event_args);

        event_args.handled()
    }

    /// The back callback of the activity (API 33): the answer is whether
    /// the default action of the system follows.
    pub(crate) fn handle_on_back_pressed(self: &Rc<Self>) -> bool {
        // A callback is registered between `onStart` and `onStop`; a system that calls
        // `onBackPressed` instead finds it here as well.
        let callback = self.current_back_pressed_callback.borrow().as_ref().map(|_| BackPressedCallback::new(self));
        match callback {
            Some(callback) => callback.handle_on_back_pressed(),
            None => true,
        }
    }

    pub fn on_back_invoked(&self) {
        let event_args = AndroidBackRequestedEventArgs::new();

        self.raise_back_requested(&event_args);

        self.should_navigate_back.set(!event_args.handled());
    }

    fn raise_back_requested(&self, event_args: &AndroidBackRequestedEventArgs) {
        for (_, handler) in self.back_requested.snapshot().iter() {
            handler(event_args);
        }
    }

    /// `onNewIntent` of the activity, and the intent it was created with:
    /// `android_uri` is the data of the intent (an `android.net.Uri`).
    pub(crate) fn handle_intent(&self, android_uri: Option<JavaObject>) {
        let Some(android_uri) = android_uri else {
            return;
        };
        if !call_boolean(&android_uri, "isAbsolute", "()Z", &[]) {
            return;
        }
        let Some(text) = call_object(&android_uri, "toString", "()Ljava/lang/String;", &[]) else {
            return;
        };
        let Some(uri) = Uri::try_create(&string_of(&text), UriKind::Absolute) else {
            return;
        };

        if uri.scheme() == "file" || uri.scheme() == "content" {
            let item = crate::platform::storage::android_storage_item::create_item(&self.java, android_uri);
            raise(&self.on_activated, FileActivatedEventArgs::new(vec![item]).into());
        } else {
            raise(&self.on_activated, ProtocolActivatedEventArgs::new(uri).into());
        }
    }

    pub(crate) fn on_activity_result(&self, request_code: i32, result_code: i32, data: Option<JavaObject>) {
        let activity_result = self.activity_result.borrow().clone();
        if let Some(activity_result) = activity_result {
            activity_result(request_code, result_code, data.as_ref());
        }
    }

    pub(crate) fn on_request_permissions_result(&self, request_code: i32, permissions: &[String], grant_results: &[i32]) {
        let request_permissions_result = self.request_permissions_result.borrow().clone();
        if let Some(request_permissions_result) = request_permissions_result {
            request_permissions_result(request_code, permissions, grant_results);
        }
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
            current_back_pressed_callback: RefCell::new(None),
            should_navigate_back: Cell::new(false),
            activity_result: RefCell::new(None),
            request_permissions_result: RefCell::new(None),
            back_requested: Rc::new(HandlerList::new()),
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

        // `HandleIntent(Intent)` follows: the Java activity calls it with the data of its
        // intent.
    }

    pub(crate) fn on_stop(&self) {
        raise(&self.on_deactivated, ActivatedEventArgs::new(ActivationKind::Background));

        if sdk_int() >= 33 {
            let removed = self.current_back_pressed_callback.borrow_mut().take();
            if removed.is_some() {
                call_void(&self.java, "removeBackCallback", "()V", &[]);
            }
        }
    }

    pub(crate) fn on_start(self: &Rc<Self>) {
        raise(&self.on_activated, ActivatedEventArgs::new(ActivationKind::Background));

        if sdk_int() >= 33 {
            *self.current_back_pressed_callback.borrow_mut() = Some(BackPressedCallback::new(self));
            call_void(&self.java, "addBackCallback", "()V", &[]);
        }
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

impl IActivityResultHandler for FerroActivity {
    fn activity_result(&self) -> Option<ActivityResultHandler> {
        self.activity_result.borrow().clone()
    }

    fn set_activity_result(&self, value: Option<ActivityResultHandler>) {
        *self.activity_result.borrow_mut() = value;
    }

    fn request_permissions_result(&self) -> Option<RequestPermissionsResultHandler> {
        self.request_permissions_result.borrow().clone()
    }

    fn set_request_permissions_result(&self, value: Option<RequestPermissionsResultHandler>) {
        *self.request_permissions_result.borrow_mut() = value;
    }
}

impl IActivityNavigationService for FerroActivity {
    fn back_requested(&self, handler: Rc<dyn Fn(&AndroidBackRequestedEventArgs)>) -> Rc<dyn IDisposable> {
        let token = self.back_requested.add(handler);
        let back_requested = self.back_requested.clone();
        Disposable::create(move || {
            back_requested.remove(token);
        })
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
