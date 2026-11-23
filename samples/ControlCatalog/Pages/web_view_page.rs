//! Not from upstream: the page of the `WebView2` control. It shows a local document and passes a
//! message each way between the page and the application.

use crate::markup::{content_page_class, xaml_class};
use ferroui_base::{ferro_class_info, instantiate, Ref};
use ferroui_controls::{ContentPage, TextBlock};
use ferroui_controls_webview::{CoreWebView2InitializedEventArgs, WebMessageReceivedEventArgs, WebView2};
use std::rc::Rc;

const DOCUMENT: &str = "<!doctype html><html><head><title>FerroUI web view</title></head>\
<body style=\"font-family:sans-serif\"><h3>A document given as text</h3>\
<button id=\"send\">Send a message to the application</button><p id=\"reply\"></p><script>\
document.getElementById('send').onclick=function(){chrome.webview.postMessage('Hello from the page');};\
chrome.webview.addEventListener('message',function(e){document.getElementById('reply').textContent=e.data;});\
</script></body></html>";

#[repr(C)]
pub struct WebViewPage {
    base: ContentPage,
}

content_page_class!(WebViewPage);
ferro_class_info!(WebViewPage { new: WebViewPage::new });
xaml_class!(WebViewPage, "/Pages/WebViewPage.xaml");

impl WebViewPage {
    pub fn construct() -> Self {
        Self { base: ContentPage::construct() }
    }

    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        this.initialize_component();

        let status = this.get_control::<TextBlock>("Status");
        let view = this.get_control::<WebView2>("View");

        let weak_status = status.downgrade();
        view.core_web_view2_initialized().add(Rc::new(move |args: &CoreWebView2InitializedEventArgs| {
            if let Some(status) = weak_status.upgrade() {
                let text = match &args.exception {
                    None => "The web engine is ready.".to_owned(),
                    Some(error) => format!("The web engine is not available: {error}"),
                };
                status.set_text(Some(text.as_str()));
            }
        }));
        let weak_status = status.downgrade();
        let weak_view = view.downgrade();
        view.web_message_received().add(Rc::new(move |args: &WebMessageReceivedEventArgs| {
            if let (Some(status), Some(message)) = (weak_status.upgrade(), args.try_get_web_message_as_string()) {
                status.set_text(Some(format!("The page says: {message}").as_str()));
            }
            if let Some(view) = weak_view.upgrade() {
                view.post_web_message_as_string("Hello from the application");
            }
        }));
        view.navigate_to_string(DOCUMENT);

        this
    }
}
