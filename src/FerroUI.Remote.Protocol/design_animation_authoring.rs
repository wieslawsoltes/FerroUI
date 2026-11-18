use crate::{bson_class, ferro_remote_message_guid};

/// Opt in alongside the animation transport. Older clients still receive the
/// playback catalog.
pub const DESIGN_ANIMATION_AUTHORING_QUERY: &str = "ferroui-design-keyframes";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignAnimationAuthoring {
    pub codec: String,
    pub tracks: Vec<DesignAnimationTrack>,
}

bson_class!(DesignAnimationAuthoring as "DesignAnimationAuthoring" {
    "Codec" => codec: String,
    "Tracks" => tracks: Vec<DesignAnimationTrack>,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignAnimationTrack {
    pub target: i64,
    pub property: String,
    pub keys: Vec<DesignAnimationKeyframe>,
}

bson_class!(DesignAnimationTrack as "DesignAnimationTrack" {
    "Target" => target: i64,
    "Property" => property: String,
    "Keys" => keys: Vec<DesignAnimationKeyframe>,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignAnimationKeyframe {
    pub time: f64,
    pub value: String,
    pub easing: String,
    pub value_kind: String,
}

bson_class!(DesignAnimationKeyframe as "DesignAnimationKeyframe" {
    "Time" => time: f64,
    "Value" => value: String,
    "Easing" => easing: String,
    "ValueKind" => value_kind: String,
});

/// Plan a draft against the application build's registered codec. This request
/// never commits application state or writes the source file.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignAnimationAuthoringRequest {
    pub revision: i64,
    pub request: i64,
    pub program: i64,
    pub target: i64,
    pub property: String,
    pub previous: Option<f64>,
    pub time: f64,
    pub value: String,
    pub easing: String,
}

ferro_remote_message_guid!(
    DesignAnimationAuthoringRequest,
    "D7B9804A-61AD-4BEC-9067-F4901FE7C340"
);
bson_class!(DesignAnimationAuthoringRequest as "DesignAnimationAuthoringRequest" {
    "Revision" => revision: i64,
    "Request" => request: i64,
    "Program" => program: i64,
    "Target" => target: i64,
    "Property" => property: String,
    "Previous" => previous: Option<f64>,
    "Time" => time: f64,
    "Value" => value: String,
    "Easing" => easing: String,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignAnimationAuthoringResult {
    pub revision: i64,
    pub request: i64,
    pub patch: Option<DesignAnimationSourcePatch>,
    pub samples: Vec<f64>,
    pub value_kind: String,
    pub error: Option<String>,
}

ferro_remote_message_guid!(
    DesignAnimationAuthoringResult,
    "51B25CB4-4324-4890-B1C0-5B81766D5673"
);
bson_class!(DesignAnimationAuthoringResult as "DesignAnimationAuthoringResult" {
    "Revision" => revision: i64,
    "Request" => request: i64,
    "Patch" => patch: Option<DesignAnimationSourcePatch>,
    "Samples" => samples: Vec<f64>,
    "ValueKind" => value_kind: String,
    "Error" => error: Option<String>,
});

/// UTF-16 offsets in the accepted source and the expected text at that span.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignAnimationSourcePatch {
    pub start: i32,
    pub end: i32,
    pub expected: String,
    pub replacement: String,
}

bson_class!(DesignAnimationSourcePatch as "DesignAnimationSourcePatch" {
    "Start" => start: i32,
    "End" => end: i32,
    "Expected" => expected: String,
    "Replacement" => replacement: String,
});
