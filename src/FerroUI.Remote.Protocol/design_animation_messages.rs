use crate::{bson_class, ferro_remote_message_guid};
use crate::design_animation_authoring::{DesignAnimationAuthoring, DesignAnimationTrack};

/// An IDE opts in with `?ferroui-design-animation=1` on the TCP preview URI.
/// A host without this protocol still echoes its session and responds to ordinary design updates.
pub const DESIGN_ANIMATION_QUERY: &str = "ferroui-design-animation";
pub const DESIGN_ANIMATION_VERSION: &str = "1";
/// Include projected Rust state routes in the native preview animation catalog.
pub const DESIGN_STATE_ROUTES_QUERY: &str = "ferroui-design-state-routes";
/// Include elapsed clip timing and accept combined native playback requests.
pub const DESIGN_ANIMATION_CLIPS_QUERY: &str = "ferroui-design-clips";
/// Include live style owners even when an accepted design contains no clips.
pub const DESIGN_CLIP_AUTHORING_QUERY: &str = "ferroui-design-clip-authoring";
/// Expose XAML state routes, instantaneous state poses, and live state owners.
pub const DESIGN_STATE_AUTHORING_QUERY: &str = "ferroui-design-state-authoring";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignAnimationCatalogMessage {
    pub revision: i64,
    pub programs: Vec<DesignAnimationProgram>,
    pub targets: Vec<DesignAnimationTarget>,
    pub keyframe_selection: Option<bool>,
    pub clip_authoring: Option<bool>,
    pub state_authoring: Option<bool>,
}

ferro_remote_message_guid!(
    DesignAnimationCatalogMessage,
    "C094EFD2-D7FC-467D-AC22-8FA047C4B54B"
);
bson_class!(DesignAnimationCatalogMessage as "DesignAnimationCatalogMessage" {
    "Revision" => revision: i64,
    "Programs" => programs: Vec<DesignAnimationProgram>,
    "Targets" => targets: Vec<DesignAnimationTarget>,
    "KeyframeSelection" => keyframe_selection: Option<bool>,
    "ClipAuthoring" => clip_authoring: Option<bool>,
    "StateAuthoring" => state_authoring: Option<bool>,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignAnimationProgram {
    pub id: i64,
    pub name: String,
    pub selector: String,
    pub duration: f64,
    pub total_duration: Option<f64>,
    pub default_targets: Vec<i64>,
    pub compatible_targets: Vec<i64>,
    pub source_utf16: i32,
    pub custom_animator: bool,
    pub authoring: Option<DesignAnimationAuthoring>,
    pub curve_tracks: Option<Vec<DesignAnimationTrack>>,
    pub clip: Option<DesignAnimationClip>,
}

bson_class!(DesignAnimationProgram as "DesignAnimationProgram" {
    "Id" => id: i64,
    "Name" => name: String,
    "Selector" => selector: String,
    "Duration" => duration: f64,
    "TotalDuration" => total_duration: Option<f64>,
    "DefaultTargets" => default_targets: Vec<i64>,
    "CompatibleTargets" => compatible_targets: Vec<i64>,
    "SourceUtf16" => source_utf16: i32,
    "CustomAnimator" => custom_animator: bool,
    "Authoring" => authoring: Option<DesignAnimationAuthoring>,
    "CurveTracks" => curve_tracks: Option<Vec<DesignAnimationTrack>>,
    "Clip" => clip: Option<DesignAnimationClip>,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignAnimationClip {
    pub delay: f64,
    pub speed_ratio: f64,
    pub editable: bool,
}

bson_class!(DesignAnimationClip as "DesignAnimationClip" {
    "Delay" => delay: f64,
    "SpeedRatio" => speed_ratio: f64,
    "Editable" => editable: bool,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignAnimationClipBinding {
    pub program: i64,
    pub targets: Option<Vec<i64>>,
}

bson_class!(DesignAnimationClipBinding as "DesignAnimationClipBinding" {
    "Program" => program: i64,
    "Targets" => targets: Option<Vec<i64>>,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignAnimationTarget {
    pub id: i64,
    pub label: String,
    pub type_name: String,
}

bson_class!(DesignAnimationTarget as "DesignAnimationTarget" {
    "Id" => id: i64,
    "Label" => label: String,
    "TypeName" => type_name: String,
});

/// Apply an elapsed sample using either the authored selector or explicit live targets.
/// Either a program or a set of clips identifies the pose. Omitting both disposes the
/// animation layer. An explicit seek restarts the shared clock.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignAnimationRequestMessage {
    pub revision: i64,
    pub request: i64,
    pub program: Option<i64>,
    pub targets: Option<Vec<i64>>,
    pub elapsed: f64,
    pub restart: bool,
    pub clips: Option<Vec<DesignAnimationClipBinding>>,
}

ferro_remote_message_guid!(
    DesignAnimationRequestMessage,
    "BCDB99F8-61E0-4E92-A837-F8D18045ABB1"
);
bson_class!(DesignAnimationRequestMessage as "DesignAnimationRequestMessage" {
    "Revision" => revision: i64,
    "Request" => request: i64,
    "Program" => program: Option<i64>,
    "Targets" => targets: Option<Vec<i64>>,
    "Elapsed" => elapsed: f64,
    "Restart" => restart: bool,
    "Clips" => clips: Option<Vec<DesignAnimationClipBinding>>,
});

/// On success, only a rendered frame newer than `after_sequence` can satisfy the request.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignAnimationResultMessage {
    pub revision: i64,
    pub request: i64,
    pub elapsed: f64,
    pub completed: bool,
    pub after_sequence: i64,
    pub error: Option<String>,
}

ferro_remote_message_guid!(
    DesignAnimationResultMessage,
    "2E7AC05A-D953-4211-BC9E-F400AEF993CF"
);
bson_class!(DesignAnimationResultMessage as "DesignAnimationResultMessage" {
    "Revision" => revision: i64,
    "Request" => request: i64,
    "Elapsed" => elapsed: f64,
    "Completed" => completed: bool,
    "AfterSequence" => after_sequence: i64,
    "Error" => error: Option<String>,
});
