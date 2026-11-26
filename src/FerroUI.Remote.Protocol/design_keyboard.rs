use crate::{bson_class, ferro_remote_message_guid};

pub const DESIGN_KEYBOARD_QUERY: &str = "ferroui-design-keyboard";
pub const DESIGN_KEYBOARD_VERSION: &str = "1";

/// Deliver a key gesture or committed text to the mounted view's focus scope.
/// Omitting both starts focus traversal. A request may specify only one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignKeyboardRequest {
    pub revision: i64,
    pub request: i64,
    pub key: Option<String>,
    pub text: Option<String>,
}

ferro_remote_message_guid!(
    DesignKeyboardRequest,
    "D6765277-60C0-4B62-AA66-DABE6DDEBCA4"
);
bson_class!(DesignKeyboardRequest as "DesignKeyboardRequest" {
    "Revision" => revision: i64,
    "Request" => request: i64,
    "Key" => key: Option<String>,
    "Text" => text: Option<String>,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignKeyboardFocus {
    pub name: String,
    pub role: String,
}

bson_class!(DesignKeyboardFocus as "DesignKeyboardFocus" {
    "Name" => name: String,
    "Role" => role: String,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignKeyboardResult {
    pub revision: i64,
    pub request: i64,
    pub focus: Option<DesignKeyboardFocus>,
    pub error: Option<String>,
}

ferro_remote_message_guid!(DesignKeyboardResult, "18F7941E-FF77-4FC8-8DC5-D8DDA76C8AB1");
bson_class!(DesignKeyboardResult as "DesignKeyboardResult" {
    "Revision" => revision: i64,
    "Request" => request: i64,
    "Focus" => focus: Option<DesignKeyboardFocus>,
    "Error" => error: Option<String>,
});
