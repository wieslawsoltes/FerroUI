//! The `WebView2` control.

use crate::core_web_view2::{run_script, CoreWebView2, PendingScripts};
use crate::engine::{
    CoreWebView2WebErrorStatus, IWebViewEngine, IWebViewEngineEvents, IWebViewEngineFactory,
    WebViewError, WebViewNavigation,
};
use crate::engines;
use crate::event_args::{
    CoreWebView2InitializedEventArgs, NavigationCompletedEventArgs, NavigationStartingEventArgs,
    WebMessageReceivedEventArgs,
};
use crate::json;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_property, instantiate, DirectProperty,
    FerroLocator, FerroObjectImpl, FerroObjectImplExt, FerroProperty,
    FerroPropertyChangedEventArgs, LocatorExtensions, Ref, StyledElementImpl, StyledProperty,
    VisualImpl, WeakRef,
};
use ferroui_controls::platform::IPlatformHandle;
use ferroui_controls::{
    ControlImpl, NativeControlHost, NativeControlHostImpl, NativeControlHostImplExt,
};
use ferroui_system::events::Event;
use ferroui_system::task::{Shared, SharedSender};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// What the control was last told to show, shown again by a new engine.
#[derive(Clone)]
enum Content {
    Uri(String),
    Html(String),
}

/// A command given before the engine was ready.
enum Command {
    Show(Content),
    Reload,
    Stop,
    GoBack,
    GoForward,
    Script(u64, String),
    PostJson(String),
}

/// A web page in a native view of the platform.
#[repr(C)]
pub struct WebView2 {
    base: NativeControlHost,
    engine: RefCell<Option<Rc<dyn IWebViewEngine>>>,
    core: RefCell<Option<Rc<CoreWebView2>>>,
    is_ready: Cell<bool>,
    /// Why the engine of this attachment could not be started.
    failure: RefCell<Option<WebViewError>>,
    waiters: RefCell<Vec<SharedSender<Result<Rc<CoreWebView2>, WebViewError>>>>,
    commands: RefCell<Vec<Command>>,
    scripts: Rc<PendingScripts>,
    content: RefCell<Option<Content>>,
    /// The engine is reporting the address: the change of `Source` is not a new navigation.
    is_syncing_source: Cell<bool>,
    can_go_back: Cell<bool>,
    can_go_forward: Cell<bool>,
    core_web_view2_initialized: Event<CoreWebView2InitializedEventArgs>,
    navigation_starting: Event<NavigationStartingEventArgs>,
    navigation_completed: Event<NavigationCompletedEventArgs>,
    web_message_received: Event<WebMessageReceivedEventArgs>,
}

ferro_class!(WebView2: NativeControlHost);
ferro_class_info!(WebView2 { new: WebView2::new });
ferro_impl_classes!(
    WebView2: StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl WebView2 {
    ferro_property!(
        /// Defines the `Source` property.
        pub fn source_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<WebView2, _>("Source", None)
        }
    );

    ferro_property!(
        /// Defines the `CanGoBack` property.
        pub fn can_go_back_property() -> DirectProperty<WebView2, bool> {
            FerroProperty::register_direct::<WebView2, _>(
                "CanGoBack",
                |o| o.can_go_back(),
                None,
                false,
            )
        }
    );

    ferro_property!(
        /// Defines the `CanGoForward` property.
        pub fn can_go_forward_property() -> DirectProperty<WebView2, bool> {
            FerroProperty::register_direct::<WebView2, _>(
                "CanGoForward",
                |o| o.can_go_forward(),
                None,
                false,
            )
        }
    );

    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: NativeControlHost::construct(),
            engine: RefCell::new(None),
            core: RefCell::new(None),
            is_ready: Cell::new(false),
            failure: RefCell::new(None),
            waiters: RefCell::new(Vec::new()),
            commands: RefCell::new(Vec::new()),
            scripts: Rc::new(PendingScripts::default()),
            content: RefCell::new(None),
            is_syncing_source: Cell::new(false),
            can_go_back: Cell::new(false),
            can_go_forward: Cell::new(false),
            core_web_view2_initialized: Event::new(),
            navigation_starting: Event::new(),
            navigation_completed: Event::new(),
            web_message_received: Event::new(),
        }
    }

    /// Creates a web view.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The address of the document.
    pub fn source(&self) -> Option<String> {
        self.get_value(Self::source_property())
    }

    /// Goes to an address.
    pub fn set_source(&self, value: Option<String>) {
        self.set_value(Self::source_property(), value);
    }

    /// Whether the history has a document before this one.
    pub fn can_go_back(&self) -> bool {
        self.can_go_back.get()
    }

    /// Whether the history has a document after this one.
    pub fn can_go_forward(&self) -> bool {
        self.can_go_forward.get()
    }

    /// The commands of the engine, once it is ready.
    pub fn core_web_view2(&self) -> Option<Rc<CoreWebView2>> {
        self.is_ready
            .get()
            .then(|| self.core.borrow().clone())
            .flatten()
    }

    /// Raised when the engine is ready or could not be started.
    pub fn core_web_view2_initialized(&self) -> &Event<CoreWebView2InitializedEventArgs> {
        &self.core_web_view2_initialized
    }

    /// Raised before a navigation starts; a handler can cancel it.
    pub fn navigation_starting(&self) -> &Event<NavigationStartingEventArgs> {
        &self.navigation_starting
    }

    /// Raised when a navigation ended.
    pub fn navigation_completed(&self) -> &Event<NavigationCompletedEventArgs> {
        &self.navigation_completed
    }

    /// Raised when the page posts a message.
    pub fn web_message_received(&self) -> &Event<WebMessageReceivedEventArgs> {
        &self.web_message_received
    }

    /// The engine, once it is ready. The task waits for the control to be attached to a window.
    pub fn ensure_core_web_view2_async(&self) -> Shared<Result<Rc<CoreWebView2>, WebViewError>> {
        if let Some(core) = self.core_web_view2() {
            return Shared::ready(Ok(core));
        }
        if let Some(error) = self.failure.borrow().clone() {
            return Shared::ready(Err(error));
        }
        let (sender, task) = Shared::channel();
        self.waiters.borrow_mut().push(sender);
        task
    }

    /// Shows a document given as text.
    pub fn navigate_to_string(&self, html: &str) {
        let content = Content::Html(html.to_owned());
        *self.content.borrow_mut() = Some(content.clone());
        self.command(Command::Show(content));
    }

    /// Loads the document again.
    pub fn reload(&self) {
        self.command(Command::Reload);
    }

    /// Stops what is loading.
    pub fn stop(&self) {
        self.command(Command::Stop);
    }

    /// Goes back in the history.
    pub fn go_back(&self) {
        self.command(Command::GoBack);
    }

    /// Goes forward in the history.
    pub fn go_forward(&self) {
        self.command(Command::GoForward);
    }

    /// Runs a script in the document; the task ends with its result as JSON. Before the engine
    /// is ready the script waits for it.
    pub fn execute_script_async(&self, script: &str) -> Shared<Result<String, WebViewError>> {
        let (id, task) = self.scripts.add();
        self.command(Command::Script(id, script.to_owned()));
        task
    }

    /// Sends the page a message given as JSON.
    pub fn post_web_message_as_json(&self, json: &str) {
        self.command(Command::PostJson(json.to_owned()));
    }

    /// Sends the page a string.
    pub fn post_web_message_as_string(&self, text: &str) {
        self.post_web_message_as_json(&json::quote(text));
    }

    /// Runs a command now, or keeps it for the engine. Of the documents to show, the last wins.
    fn command(&self, command: Command) {
        let failure = self.failure.borrow().clone();
        if let Some(error) = failure {
            // Nothing will run it until the next attachment; a script must not wait for that.
            if let Command::Script(id, _) = command {
                self.scripts.complete(id, Err(error));
            }
            return;
        }
        let engine = self
            .is_ready
            .get()
            .then(|| self.engine.borrow().clone())
            .flatten();
        match engine {
            Some(engine) => self.run(&engine, command),
            None => {
                let mut commands = self.commands.borrow_mut();
                if matches!(command, Command::Show(_)) {
                    commands.retain(|queued| !matches!(queued, Command::Show(_)));
                }
                commands.push(command);
            }
        }
    }

    fn run(&self, engine: &Rc<dyn IWebViewEngine>, command: Command) {
        match command {
            Command::Show(Content::Uri(uri)) => engine.navigate(&uri),
            Command::Show(Content::Html(html)) => engine.navigate_to_string(&html),
            Command::Reload => engine.reload(),
            Command::Stop => engine.stop(),
            Command::GoBack => engine.go_back(),
            Command::GoForward => engine.go_forward(),
            Command::Script(id, script) => run_script(engine, &self.scripts, id, &script),
            Command::PostJson(json) => engine.post_web_message_as_json(&json),
        }
    }

    fn factory() -> Option<Rc<dyn IWebViewEngineFactory>> {
        FerroLocator::current()
            .get_service::<dyn IWebViewEngineFactory>()
            .or_else(engines::default_factory)
    }

    fn start_engine(
        &self,
        parent: Rc<dyn IPlatformHandle>,
    ) -> Result<Rc<dyn IPlatformHandle>, WebViewError> {
        *self.failure.borrow_mut() = None;
        let factory = Self::factory().ok_or_else(|| {
            WebViewError::Unavailable("this platform has no web engine".to_owned())
        })?;
        let events: Rc<dyn IWebViewEngineEvents> = Rc::new(EngineEvents {
            owner: self.to_ref().downgrade(),
        });
        let engine = factory.create(parent, events)?;
        let control = engine.native_control();
        *self.engine.borrow_mut() = Some(engine);
        Ok(control)
    }

    fn engine_initialized(&self, result: Result<(), WebViewError>) {
        let Some(engine) = self.engine.borrow().clone() else {
            return;
        };
        if self.is_ready.get() {
            return;
        }
        match result {
            Ok(()) => {
                let source = self.source().unwrap_or_default();
                let core = CoreWebView2::new(engine.clone(), self.scripts.clone(), source);
                *self.core.borrow_mut() = Some(core.clone());
                self.is_ready.set(true);
                // What a former engine showed comes back, unless a newer document is queued.
                let queued_show = self
                    .commands
                    .borrow()
                    .iter()
                    .any(|command| matches!(command, Command::Show(_)));
                if !queued_show {
                    let content = self.content.borrow().clone();
                    if let Some(content) = content {
                        self.run(&engine, Command::Show(content));
                    }
                }
                // Taken first: a command or a waiter can call back into the control.
                let commands = std::mem::take(&mut *self.commands.borrow_mut());
                for command in commands {
                    self.run(&engine, command);
                }
                let waiters = std::mem::take(&mut *self.waiters.borrow_mut());
                for waiter in waiters {
                    waiter.send(Ok(core.clone()));
                }
                self.core_web_view2_initialized
                    .raise(&CoreWebView2InitializedEventArgs { exception: None });
            }
            Err(error) => self.engine_failed(error),
        }
    }

    /// The engine could not be started: everything that waits for it is told.
    fn engine_failed(&self, error: WebViewError) {
        *self.failure.borrow_mut() = Some(error.clone());
        self.commands.borrow_mut().clear();
        self.scripts.close();
        let waiters = std::mem::take(&mut *self.waiters.borrow_mut());
        for waiter in waiters {
            waiter.send(Err(error.clone()));
        }
        self.core_web_view2_initialized
            .raise(&CoreWebView2InitializedEventArgs {
                exception: Some(error),
            });
    }

    fn close_engine(&self) {
        let engine = self.engine.borrow_mut().take();
        let core = self.core.borrow_mut().take();
        self.is_ready.set(false);
        if let Some(core) = core {
            core.mark_closed();
        }
        if let Some(engine) = engine {
            engine.close();
        }
        // Commands for the closed engine are dropped; the document comes back with the next.
        self.commands
            .borrow_mut()
            .retain(|command| !matches!(command, Command::Script(..)));
        self.scripts.close();
        self.set_history(false, false);
    }

    fn set_history(&self, can_go_back: bool, can_go_forward: bool) {
        self.set_and_raise_cell(Self::can_go_back_property(), &self.can_go_back, can_go_back);
        self.set_and_raise_cell(
            Self::can_go_forward_property(),
            &self.can_go_forward,
            can_go_forward,
        );
    }

    fn engine_source_changed(&self, uri: &str) {
        if let Some(core) = self.core.borrow().as_ref() {
            core.set_source(uri);
        }
        // A document given as text keeps its place: only an address replaces it.
        if !uri.is_empty() && !uri.starts_with("about:") && !uri.starts_with("data:") {
            *self.content.borrow_mut() = Some(Content::Uri(uri.to_owned()));
            self.is_syncing_source.set(true);
            self.set_source(Some(uri.to_owned()));
            self.is_syncing_source.set(false);
        }
    }
}

impl FerroObjectImpl for WebView2 {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        Self::parent_on_property_changed(this, change);
        if change.property() == Self::source_property().as_property()
            && !this.is_syncing_source.get()
        {
            match change.get_new_value::<Option<String>>() {
                Some(uri) => {
                    let content = Content::Uri(uri);
                    *this.content.borrow_mut() = Some(content.clone());
                    this.command(Command::Show(content));
                }
                None => {
                    let mut content = this.content.borrow_mut();
                    if matches!(*content, Some(Content::Uri(_))) {
                        *content = None;
                    }
                }
            }
        }
    }
}

impl NativeControlHostImpl for WebView2 {
    fn create_native_control_core(
        this: &Self,
        parent: Rc<dyn IPlatformHandle>,
    ) -> Rc<dyn IPlatformHandle> {
        match this.start_engine(parent.clone()) {
            Ok(control) => control,
            Err(error) => {
                this.engine_failed(error);
                // The space of the control stays a native child, as with no web engine at all.
                Self::parent_create_native_control_core(this, parent)
            }
        }
    }

    fn destroy_native_control_core(this: &Self, control: Rc<dyn IPlatformHandle>) {
        // The next attachment tries again.
        *this.failure.borrow_mut() = None;
        if this.engine.borrow().is_some() {
            this.close_engine();
        } else {
            Self::parent_destroy_native_control_core(this, control);
        }
    }
}

/// Carries what an engine reports to its control, if it is still there.
struct EngineEvents {
    owner: WeakRef<WebView2>,
}

impl IWebViewEngineEvents for EngineEvents {
    fn initialized(&self, result: Result<(), WebViewError>) {
        if let Some(owner) = self.owner.upgrade() {
            owner.engine_initialized(result);
        }
    }

    fn navigation_starting(&self, navigation: &WebViewNavigation) -> bool {
        let Some(owner) = self.owner.upgrade() else {
            return false;
        };
        let args = NavigationStartingEventArgs::new(
            navigation.id,
            navigation.uri.clone(),
            navigation.is_user_initiated,
            navigation.is_redirected,
        );
        owner.navigation_starting.raise(&args);
        !args.cancel()
    }

    fn navigation_completed(&self, id: u64, is_success: bool, status: CoreWebView2WebErrorStatus) {
        if let Some(owner) = self.owner.upgrade() {
            owner
                .navigation_completed
                .raise(&NavigationCompletedEventArgs {
                    navigation_id: id,
                    is_success,
                    web_error_status: status,
                });
        }
    }

    fn source_changed(&self, uri: &str) {
        if let Some(owner) = self.owner.upgrade() {
            owner.engine_source_changed(uri);
        }
    }

    fn history_changed(&self, can_go_back: bool, can_go_forward: bool) {
        if let Some(owner) = self.owner.upgrade() {
            owner.set_history(can_go_back, can_go_forward);
        }
    }

    fn document_title_changed(&self, title: &str) {
        if let Some(core) = self
            .owner
            .upgrade()
            .and_then(|owner| owner.core.borrow().clone())
        {
            core.set_document_title(title);
        }
    }

    fn web_message_received(&self, source: &str, json: &str) {
        if let Some(owner) = self.owner.upgrade() {
            owner
                .web_message_received
                .raise(&WebMessageReceivedEventArgs {
                    source: source.to_owned(),
                    web_message_as_json: json.to_owned(),
                });
        }
    }
}
