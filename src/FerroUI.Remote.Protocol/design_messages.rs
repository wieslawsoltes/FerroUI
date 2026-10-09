use crate::{bson_class, ferro_remote_message_guid};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UpdateXamlMessage {
    pub xaml: Option<String>,
    pub assembly_path: Option<String>,
    pub xaml_file_project_path: Option<String>,
}

ferro_remote_message_guid!(UpdateXamlMessage, "9AEC9A2E-6315-4066-B4BA-E9A9EFD0F8CC");
bson_class!(UpdateXamlMessage as "UpdateXamlMessage" {
    "Xaml" => xaml: Option<String>,
    "AssemblyPath" => assembly_path: Option<String>,
    "XamlFileProjectPath" => xaml_file_project_path: Option<String>,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UpdateXamlResultMessage {
    pub error: Option<String>,
    pub handle: Option<String>,
    pub exception: Option<ExceptionDetails>,
}

ferro_remote_message_guid!(UpdateXamlResultMessage, "B7A70093-0C5D-47FD-9261-22086D43A2E2");
bson_class!(UpdateXamlResultMessage as "UpdateXamlResultMessage" {
    "Error" => error: Option<String>,
    "Handle" => handle: Option<String>,
    "Exception" => exception: Option<ExceptionDetails>,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StartDesignerSessionMessage {
    pub session_id: Option<String>,
}

ferro_remote_message_guid!(StartDesignerSessionMessage, "854887CF-2694-4EB6-B499-7461B6FB96C7");
bson_class!(StartDesignerSessionMessage as "StartDesignerSessionMessage" {
    "SessionId" => session_id: Option<String>,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExceptionDetails {
    pub exception_type: Option<String>,
    pub message: Option<String>,
    pub line_number: Option<i32>,
    pub line_position: Option<i32>,
}

impl ExceptionDetails {
    /// `new ExceptionDetails()`.
    pub fn new() -> ExceptionDetails {
        ExceptionDetails::default()
    }

    /// `new ExceptionDetails(Exception e)`.
    ///
    /// Deviation (DEVIATIONS.md, Remote protocol): the original reads the
    /// name of the class of the exception by reflection, unwraps a
    /// `TargetInvocationException` and takes the line and the position from
    /// an `XmlException`. An error of Rust has no class name to read and this
    /// library does not know the error types of the markup crates, so the
    /// caller states the three: the name of the error type, the error (its
    /// text is the message) and, for an error of a document, its line and
    /// position.
    pub fn from_exception(
        exception_type: &str,
        e: &dyn std::error::Error,
        line_info: Option<(i32, i32)>,
    ) -> ExceptionDetails {
        ExceptionDetails {
            exception_type: Some(exception_type.to_string()),
            message: Some(e.to_string()),
            line_number: line_info.map(|(line_number, _)| line_number),
            line_position: line_info.map(|(_, line_position)| line_position),
        }
    }
}

bson_class!(ExceptionDetails as "ExceptionDetails" {
    "ExceptionType" => exception_type: Option<String>,
    "Message" => message: Option<String>,
    "LineNumber" => line_number: Option<i32>,
    "LinePosition" => line_position: Option<i32>,
});
