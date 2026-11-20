use crate::{bson_class, ferro_remote_message_guid};
use crate::viewport_messages::FrameMessage;

/// An editor update tagged with its authored buffer revision. Legacy preview clients
/// can continue to use UpdateXamlMessage and receive untagged frames.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UpdateDesignMessage {
    pub revision: i64,
    pub source: UpdateXamlMessage,
    pub fixture: Option<String>,
}

ferro_remote_message_guid!(UpdateDesignMessage, "58FA5FA8-5669-4C04-80FA-8F573BA45ACD");
bson_class!(UpdateDesignMessage as "UpdateDesignMessage" {
    "Revision" => revision: i64,
    "Source" => source: UpdateXamlMessage,
    "Fixture" => fixture: Option<String>,
});

pub const DESIGN_SOURCE_SNAPSHOT_QUERY: &str = "ferroui-design-sources";
pub const DESIGN_SOURCE_SNAPSHOT_VERSION: &str = "1";

/// Sent in place of the legacy start only when the relay requests this version.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StartDesignSourceSessionMessage {
    pub session_id: Option<String>,
}

ferro_remote_message_guid!(StartDesignSourceSessionMessage, "7D76A249-3594-4C51-956D-6E4E8C1CF1F6");
bson_class!(StartDesignSourceSessionMessage as "StartDesignSourceSessionMessage" {
    "SessionId" => session_id: Option<String>,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignIncludedSource {
    pub uri: String,
    pub xaml: String,
}

bson_class!(DesignIncludedSource as "DesignIncludedSource" {
    "Uri" => uri: String,
    "Xaml" => xaml: String,
});

/// Build one candidate from a consistent snapshot of the primary view and its includes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct UpdateDesignSourcesMessage {
    pub update: UpdateDesignMessage,
    pub base_uri: String,
    pub includes: Vec<DesignIncludedSource>,
}

ferro_remote_message_guid!(UpdateDesignSourcesMessage, "408BAE3B-C488-4DAC-9255-E3AD3C99D2C5");
bson_class!(UpdateDesignSourcesMessage as "UpdateDesignSourcesMessage" {
    "Update" => update: UpdateDesignMessage,
    "BaseUri" => base_uri: String,
    "Includes" => includes: Vec<DesignIncludedSource>,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct UpdateDesignResultMessage {
    pub revision: i64,
    pub result: UpdateXamlResultMessage,
    pub fixtures: Option<Vec<String>>,
}

ferro_remote_message_guid!(UpdateDesignResultMessage, "CB085DBB-7231-4C16-8797-D8D97BCF9341");
bson_class!(UpdateDesignResultMessage as "UpdateDesignResultMessage" {
    "Revision" => revision: i64,
    "Result" => result: UpdateXamlResultMessage,
    "Fixtures" => fixtures: Option<Vec<String>>,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignFrameMessage {
    pub revision: i64,
    pub frame: FrameMessage,
}

ferro_remote_message_guid!(DesignFrameMessage, "F7176EE5-1A68-4ECB-A986-8F461D21C13C");
bson_class!(DesignFrameMessage as "DesignFrameMessage" {"Revision" => revision: i64, "Frame" => frame: FrameMessage});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignFrameReceivedMessage {
    pub revision: i64,
    pub sequence_id: i64,
}

ferro_remote_message_guid!(DesignFrameReceivedMessage, "1E9E50A0-F95D-48C0-AB8B-B2C800ACC09C");
bson_class!(DesignFrameReceivedMessage as "DesignFrameReceivedMessage" {"Revision" => revision: i64, "SequenceId" => sequence_id: i64});

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
