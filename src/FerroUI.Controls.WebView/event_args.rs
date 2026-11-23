//! The arguments of the events of the control.

use crate::engine::{CoreWebView2WebErrorStatus, WebViewError};
use crate::json;
use std::cell::Cell;

/// The arguments of `NavigationStarting`.
#[derive(Debug)]
pub struct NavigationStartingEventArgs {
    /// Identifies the navigation in its completion.
    pub navigation_id: u64,
    /// Where it goes.
    pub uri: String,
    /// Whether the user started it in the page.
    pub is_user_initiated: bool,
    /// Whether it follows a redirect.
    pub is_redirected: bool,
    cancel: Cell<bool>,
}

impl NavigationStartingEventArgs {
    pub(crate) fn new(
        navigation_id: u64,
        uri: String,
        is_user_initiated: bool,
        is_redirected: bool,
    ) -> Self {
        Self {
            navigation_id,
            uri,
            is_user_initiated,
            is_redirected,
            cancel: Cell::new(false),
        }
    }

    /// Whether a handler canceled the navigation.
    pub fn cancel(&self) -> bool {
        self.cancel.get()
    }

    /// Cancels the navigation, or lets it go on.
    pub fn set_cancel(&self, value: bool) {
        self.cancel.set(value);
    }
}

/// The arguments of `NavigationCompleted`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NavigationCompletedEventArgs {
    /// The navigation that ended.
    pub navigation_id: u64,
    /// Whether it succeeded.
    pub is_success: bool,
    /// Why it failed.
    pub web_error_status: CoreWebView2WebErrorStatus,
}

/// The arguments of `WebMessageReceived`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WebMessageReceivedEventArgs {
    /// The address of the document that posted the message.
    pub source: String,
    /// The message as JSON.
    pub web_message_as_json: String,
}

impl WebMessageReceivedEventArgs {
    /// The message, when the page posted a string.
    pub fn try_get_web_message_as_string(&self) -> Option<String> {
        json::parse_string(&self.web_message_as_json)
    }
}

/// The arguments of `CoreWebView2Initialized`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoreWebView2InitializedEventArgs {
    /// Why the engine could not be started, or `None` when it is ready.
    pub exception: Option<WebViewError>,
}
