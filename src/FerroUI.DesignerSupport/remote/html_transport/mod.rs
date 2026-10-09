//! The HTML transport of the previewer
//! (`FerroUI.DesignerSupport.Remote.HtmlTransport`): the connection of the
//! `html` method, which shows the preview in a web page, and the HTTP and
//! web socket server it serves the page with. The page itself is the web
//! application under `webapp`.

mod html_transport;
mod simple_web_socket_http_server;

pub use html_transport::HtmlWebSocketTransport;
pub use simple_web_socket_http_server::{
    SimpleWebSocket, SimpleWebSocketHttpRequest, SimpleWebSocketHttpServer, SimpleWebSocketMessage,
};
