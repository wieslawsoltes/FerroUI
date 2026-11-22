use crate::{bson_class, ferro_remote_message_guid};

/// Opt in to sampling the loaded app's scalar animation tracks.
pub const DESIGN_ANIMATION_CURVE_QUERY: &str = "ferroui-design-curves";
/// Ask the app to advertise component and unit-specific tracks alongside scalar tracks.
pub const DESIGN_ANIMATION_CURVE_CHANNELS_QUERY: &str = "ferroui-design-curve-channels";
/// Advertise effect component channels and typed effect keys to supporting clients.
pub const DESIGN_ANIMATION_EFFECTS_QUERY: &str = "ferroui-design-animation-effects";
/// Advertise native effect objects and their directional and opacity channels.
pub const DESIGN_ANIMATION_EFFECT_OBJECTS_QUERY: &str = "ferroui-design-animation-effect-objects";
/// Advertise registered effect literals and their named parameter channels.
pub const DESIGN_ANIMATION_REGISTERED_EFFECTS_QUERY: &str = "ferroui-design-animation-registered-effects";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignAnimationCurveRequest {
    pub revision: i64,
    pub request: i64,
    pub program: i64,
    pub target: i64,
    pub property: String,
    pub channel: Option<String>,
    pub start: f64,
    pub end: f64,
}

ferro_remote_message_guid!(
    DesignAnimationCurveRequest,
    "29312A91-E412-4881-A469-1B0F1739BC77"
);
bson_class!(DesignAnimationCurveRequest as "DesignAnimationCurveRequest" {
    "Revision" => revision: i64,
    "Request" => request: i64,
    "Program" => program: i64,
    "Target" => target: i64,
    "Property" => property: String,
    "Channel" => channel: Option<String>,
    "Start" => start: f64,
    "End" => end: f64,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignAnimationCurveResult {
    pub revision: i64,
    pub request: i64,
    pub points: Vec<DesignAnimationCurvePoint>,
    pub error: Option<String>,
}

ferro_remote_message_guid!(
    DesignAnimationCurveResult,
    "2D4276FD-9106-4657-9EBA-530413ED4F11"
);
bson_class!(DesignAnimationCurveResult as "DesignAnimationCurveResult" {
    "Revision" => revision: i64,
    "Request" => request: i64,
    "Points" => points: Vec<DesignAnimationCurvePoint>,
    "Error" => error: Option<String>,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignAnimationCurvePoint {
    pub time: f64,
    pub value: f64,
    pub break_before: Option<bool>,
}

bson_class!(DesignAnimationCurvePoint as "DesignAnimationCurvePoint" {
    "Time" => time: f64,
    "Value" => value: f64,
    "BreakBefore" => break_before: Option<bool>,
});
