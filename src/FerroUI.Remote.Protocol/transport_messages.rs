use crate::{bson_class, ferro_remote_message_guid};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HtmlTransportStartedMessage {
    pub uri: Option<String>,
}

ferro_remote_message_guid!(HtmlTransportStartedMessage, "53778004-78fa-4381-8ec3-176a6f2328b6");
bson_class!(HtmlTransportStartedMessage as "HtmlTransportStartedMessage" {
    "Uri" => uri: Option<String>,
});
