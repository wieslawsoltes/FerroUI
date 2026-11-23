//! `CoreWebView2`: the commands of a running engine.

use crate::engine::{IWebViewEngine, WebViewError};
use crate::json;
use ferroui_system::task::{Shared, SharedSender};
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// The scripts an engine has not answered yet, so that closing it answers them.
#[derive(Default)]
pub(crate) struct PendingScripts {
    next: Cell<u64>,
    senders: RefCell<HashMap<u64, SharedSender<Result<String, WebViewError>>>>,
}

impl PendingScripts {
    pub(crate) fn add(&self) -> (u64, Shared<Result<String, WebViewError>>) {
        let (sender, task) = Shared::channel();
        let id = self.next.get();
        self.next.set(id + 1);
        self.senders.borrow_mut().insert(id, sender);
        (id, task)
    }

    pub(crate) fn complete(&self, id: u64, result: Result<String, WebViewError>) {
        let sender = self.senders.borrow_mut().remove(&id);
        if let Some(sender) = sender {
            sender.send(result);
        }
    }

    pub(crate) fn close(&self) {
        let senders = std::mem::take(&mut *self.senders.borrow_mut());
        for (_, sender) in senders {
            sender.send(Err(WebViewError::Closed));
        }
    }
}

/// Runs a script on `engine`, answering through `scripts`.
pub(crate) fn run_script(
    engine: &Rc<dyn IWebViewEngine>,
    scripts: &Rc<PendingScripts>,
    id: u64,
    script: &str,
) {
    let answer = scripts.clone();
    engine.execute_script(script, Box::new(move |result| answer.complete(id, result)));
}

/// The commands of the engine of a `WebView2`, once it is ready. It stops working when the
/// control closes the engine: a command then does nothing, and a script is answered with
/// [`WebViewError::Closed`].
pub struct CoreWebView2 {
    engine: Rc<dyn IWebViewEngine>,
    scripts: Rc<PendingScripts>,
    closed: Cell<bool>,
    source: RefCell<String>,
    document_title: RefCell<String>,
}

impl CoreWebView2 {
    pub(crate) fn new(
        engine: Rc<dyn IWebViewEngine>,
        scripts: Rc<PendingScripts>,
        source: String,
    ) -> Rc<Self> {
        Rc::new(Self {
            engine,
            scripts,
            closed: Cell::new(false),
            source: RefCell::new(source),
            document_title: RefCell::new(String::new()),
        })
    }

    pub(crate) fn mark_closed(&self) {
        self.closed.set(true);
    }

    pub(crate) fn set_source(&self, value: &str) {
        value.clone_into(&mut self.source.borrow_mut());
    }

    pub(crate) fn set_document_title(&self, value: &str) {
        value.clone_into(&mut self.document_title.borrow_mut());
    }

    fn live(&self) -> Option<&Rc<dyn IWebViewEngine>> {
        (!self.closed.get()).then_some(&self.engine)
    }

    /// The address of the document.
    pub fn source(&self) -> String {
        self.source.borrow().clone()
    }

    /// The title of the document.
    pub fn document_title(&self) -> String {
        self.document_title.borrow().clone()
    }

    /// Goes to an address.
    pub fn navigate(&self, uri: &str) {
        if let Some(engine) = self.live() {
            engine.navigate(uri);
        }
    }

    /// Shows a document given as text.
    pub fn navigate_to_string(&self, html: &str) {
        if let Some(engine) = self.live() {
            engine.navigate_to_string(html);
        }
    }

    /// Loads the document again.
    pub fn reload(&self) {
        if let Some(engine) = self.live() {
            engine.reload();
        }
    }

    /// Stops what is loading.
    pub fn stop(&self) {
        if let Some(engine) = self.live() {
            engine.stop();
        }
    }

    /// Goes back in the history.
    pub fn go_back(&self) {
        if let Some(engine) = self.live() {
            engine.go_back();
        }
    }

    /// Goes forward in the history.
    pub fn go_forward(&self) {
        if let Some(engine) = self.live() {
            engine.go_forward();
        }
    }

    /// Runs a script in the document; the task ends with its result as JSON.
    pub fn execute_script_async(&self, script: &str) -> Shared<Result<String, WebViewError>> {
        let Some(engine) = self.live() else {
            return Shared::ready(Err(WebViewError::Closed));
        };
        let (id, task) = self.scripts.add();
        run_script(engine, &self.scripts, id, script);
        task
    }

    /// Sends the page a message given as JSON (`event.data` is the parsed value).
    pub fn post_web_message_as_json(&self, json: &str) {
        if let Some(engine) = self.live() {
            engine.post_web_message_as_json(json);
        }
    }

    /// Sends the page a string.
    pub fn post_web_message_as_string(&self, text: &str) {
        self.post_web_message_as_json(&json::quote(text));
    }
}
