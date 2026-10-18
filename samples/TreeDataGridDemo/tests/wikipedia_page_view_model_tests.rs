//! Port of `TreeDataGridDemo.Tests/WikipediaPageViewModelTests.cs`: the load of the content of
//! the Wikipedia page through a client over a stub handler.
//!
//! `await target.LoadingTask` is [`LoadingTask::run_to_completion`]: the request is sent from a
//! thread of its own and the content arrives through the dispatcher of the test's thread.
//!
//! [`LoadingTask::run_to_completion`]: crate::view_models::LoadingTask::run_to_completion

use super::support::start_application;
use crate::models::{HttpMessageHandler, HttpRequestException, HttpRequestMessage, HttpResponseMessage, WikipediaHttpClient};
use crate::view_models::WikipediaPageViewModel;
use ferroui_treedatagrid_core::ITreeDataGridSource;
use std::sync::{Arc, Mutex};

#[test]
fn load_content_sends_user_agent_and_populates_rows() {
    let _app = start_application();
    let request: Arc<Mutex<Option<HttpRequestMessage>>> = Arc::new(Mutex::new(None));
    let handler = {
        let request = request.clone();
        StubHttpMessageHandler::new(move |x| {
            *request.lock().unwrap() = Some(x.clone());
            json_response(
                r#"
                {
                  "selected": [
                    {
                      "text": "An event",
                      "year": 2026,
                      "pages": [
                        {
                          "type": "standard",
                          "titles": { "normalized": "A page" },
                          "extract": "An extract"
                        }
                      ]
                    }
                  ]
                }
                "#,
            )
        })
    };
    let client = WikipediaHttpClient::create(Some(handler));
    let target = WikipediaPageViewModel::with_http_client(client);

    target.loading_task().run_to_completion();

    let request = request.lock().unwrap().clone();
    let request = request.expect("the request of the content");
    assert!(request.user_agent().contains("FerroUITreeDataGridSample"));
    assert_eq!(1, target.source().rows().count());
    assert_eq!(None, target.load_error());
}

#[test]
fn load_content_reports_http_failures() {
    let _app = start_application();
    let handler = StubHttpMessageHandler::new(|_| HttpResponseMessage::new(403));
    let client = WikipediaHttpClient::create(Some(handler));
    let target = WikipediaPageViewModel::with_http_client(client);

    target.loading_task().run_to_completion();

    assert_eq!(0, target.source().rows().count());
    let load_error = target.load_error().unwrap_or_default();
    assert!(load_error.starts_with("Unable to load Wikipedia content:"), "{load_error}");
}

#[test]
fn load_content_ignores_events_without_pages() {
    let _app = start_application();
    let handler = StubHttpMessageHandler::new(|_| {
        json_response(
            r#"
            {
              "selected": [
                { "text": "No associated page", "year": 2026 },
                {
                  "text": "An event with a page",
                  "year": 2025,
                  "pages": [
                    {
                      "titles": { "normalized": "A page" },
                      "extract": "An extract"
                    }
                  ]
                }
              ]
            }
            "#,
        )
    });
    let client = WikipediaHttpClient::create(Some(handler));
    let target = WikipediaPageViewModel::with_http_client(client);

    target.loading_task().run_to_completion();

    assert_eq!(1, target.source().rows().count());
    assert_eq!(None, target.load_error());
}

fn json_response(json: &str) -> HttpResponseMessage {
    HttpResponseMessage::new(200).with_string_content(json, "application/json")
}

/// A handler that answers every request with the response `response_factory` makes of it.
struct StubHttpMessageHandler {
    response_factory: Box<dyn Fn(&HttpRequestMessage) -> HttpResponseMessage + Send + Sync>,
}

impl StubHttpMessageHandler {
    fn new(response_factory: impl Fn(&HttpRequestMessage) -> HttpResponseMessage + Send + Sync + 'static) -> Arc<dyn HttpMessageHandler> {
        Arc::new(Self { response_factory: Box::new(response_factory) })
    }
}

impl HttpMessageHandler for StubHttpMessageHandler {
    fn send(&self, request: &HttpRequestMessage) -> Result<HttpResponseMessage, HttpRequestException> {
        Ok((self.response_factory)(request))
    }
}
