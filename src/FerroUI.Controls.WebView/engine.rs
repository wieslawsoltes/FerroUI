//! What the control asks of a web engine, and what an engine tells the control.

use ferroui_controls::platform::IPlatformHandle;
use std::fmt;
use std::rc::Rc;

/// Why a web view could not do what it was asked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WebViewError {
    /// This platform or machine has no usable web engine; the text says what is missing.
    Unavailable(String),
    /// The engine reported a failure.
    Engine(String),
    /// The engine was closed before it answered.
    Closed,
}

impl fmt::Display for WebViewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unavailable(reason) => write!(f, "no web engine is available: {reason}"),
            Self::Engine(reason) => write!(f, "the web engine failed: {reason}"),
            Self::Closed => f.write_str("the web engine was closed"),
        }
    }
}

impl std::error::Error for WebViewError {}

/// Why a navigation failed (the values of `CoreWebView2WebErrorStatus` that every engine can
/// report; anything else is [`Unknown`](Self::Unknown)).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CoreWebView2WebErrorStatus {
    /// No more specific status is known.
    #[default]
    Unknown,
    /// The certificate of the server is not valid.
    CertificateInvalid,
    /// The server could not be reached.
    ServerUnreachable,
    /// The request timed out.
    Timeout,
    /// The connection was aborted or reset.
    ConnectionAborted,
    /// The device is offline.
    Disconnected,
    /// The host name could not be resolved.
    HostNameNotResolved,
    /// The navigation was canceled.
    OperationCanceled,
    /// A redirect failed.
    RedirectFailed,
}

/// A navigation an engine is about to start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebViewNavigation {
    /// Identifies the navigation in its completion.
    pub id: u64,
    /// Where it goes.
    pub uri: String,
    /// Whether the user started it in the page.
    pub is_user_initiated: bool,
    /// Whether it follows a redirect.
    pub is_redirected: bool,
}

/// Receives the answer of a script: its result as JSON.
pub type ScriptCompletion = Box<dyn FnOnce(Result<String, WebViewError>)>;

/// What an engine tells the control, on the UI thread.
pub trait IWebViewEngineEvents {
    /// The engine is ready for commands, or could not be started.
    fn initialized(&self, result: Result<(), WebViewError>);
    /// A navigation is about to start. Returns `false` when it is canceled.
    fn navigation_starting(&self, navigation: &WebViewNavigation) -> bool;
    /// A navigation ended.
    fn navigation_completed(&self, id: u64, is_success: bool, status: CoreWebView2WebErrorStatus);
    /// The address of the document changed.
    fn source_changed(&self, uri: &str);
    /// What the history can do changed.
    fn history_changed(&self, can_go_back: bool, can_go_forward: bool);
    /// The title of the document changed.
    fn document_title_changed(&self, title: &str);
    /// The page posted a message, as JSON, from the document at `source`.
    fn web_message_received(&self, source: &str, json: &str);
}

/// A web engine with one native view.
pub trait IWebViewEngine {
    /// The native view, a child of the parent the engine was made for.
    fn native_control(&self) -> Rc<dyn IPlatformHandle>;
    /// Ends the engine and destroys its view. No event is raised after it.
    fn close(&self);
    /// Goes to an address.
    fn navigate(&self, uri: &str);
    /// Shows a document given as text.
    fn navigate_to_string(&self, html: &str);
    /// Loads the document again.
    fn reload(&self);
    /// Stops what is loading.
    fn stop(&self);
    /// Goes back in the history.
    fn go_back(&self);
    /// Goes forward in the history.
    fn go_forward(&self);
    /// Runs a script in the document and answers with its result as JSON.
    fn execute_script(&self, script: &str, completion: ScriptCompletion);
    /// Sends the page a message given as JSON.
    fn post_web_message_as_json(&self, json: &str);
}

/// Makes the engine of a control.
pub trait IWebViewEngineFactory {
    /// Makes an engine whose view is a child of `parent`. It reports
    /// [`initialized`](IWebViewEngineEvents::initialized) once, after this call has returned.
    fn create(
        &self,
        parent: Rc<dyn IPlatformHandle>,
        events: Rc<dyn IWebViewEngineEvents>,
    ) -> Result<Rc<dyn IWebViewEngine>, WebViewError>;
}
