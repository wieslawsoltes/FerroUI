use crate::design_animation_authoring::DesignAnimationSourcePatch;
use crate::{bson_class, ferro_remote_message_guid};

/// Request grouped key authoring in the app's registered animation codec.
pub const DESIGN_KEYFRAME_SELECTION_QUERY: &str = "ferroui-design-keyframe-selection";

/// The command is the portable Designer.Animation keyframe selection JSON.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignKeyframeSelectionRequest {
    pub revision: i64,
    pub request: i64,
    pub program: i64,
    pub command: String,
    /// A projected source host needs the app's accepted typed timeline.
    pub projected: bool,
}

ferro_remote_message_guid!(
    DesignKeyframeSelectionRequest,
    "98B491F9-0A74-443A-AA95-A07FCEF91C72"
);
bson_class!(DesignKeyframeSelectionRequest as "DesignKeyframeSelectionRequest" {
    "Revision" => revision: i64,
    "Request" => request: i64,
    "Program" => program: i64,
    "Command" => command: String,
    "Projected" => projected: bool,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignKeyframeSelectionResult {
    pub revision: i64,
    pub request: i64,
    pub patch: Option<DesignAnimationSourcePatch>,
    pub keys: Option<String>,
    pub clipboard: Option<String>,
    pub timeline: Option<String>,
    pub error: Option<String>,
}

ferro_remote_message_guid!(
    DesignKeyframeSelectionResult,
    "EEE58C45-00F9-40D6-9369-DC72A6919308"
);
bson_class!(DesignKeyframeSelectionResult as "DesignKeyframeSelectionResult" {
    "Revision" => revision: i64,
    "Request" => request: i64,
    "Patch" => patch: Option<DesignAnimationSourcePatch>,
    "Keys" => keys: Option<String>,
    "Clipboard" => clipboard: Option<String>,
    "Timeline" => timeline: Option<String>,
    "Error" => error: Option<String>,
});
