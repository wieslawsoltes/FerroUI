//! The network action and trigger (`Network/`, namespace `FerroUI.Xaml.Interactions.Network`),
//! behind the feature `network`: they use sockets and threads, which the browser does not have.

mod http_request_action;
mod network_information_trigger;

pub use http_request_action::HttpRequestAction;
pub use network_information_trigger::NetworkInformationTrigger;
