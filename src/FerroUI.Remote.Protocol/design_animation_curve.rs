use crate::{bson_class, ferro_remote_message_guid};

/// Opt in to sampling the loaded app's scalar animation tracks.
pub const DESIGN_ANIMATION_CURVE_QUERY: &str = "ferroui-design-curves";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignAnimationCurveRequest {
    pub revision: i64,
    pub request: i64,
    pub program: i64,
    pub target: i64,
    pub property: String,
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
}

bson_class!(DesignAnimationCurvePoint as "DesignAnimationCurvePoint" {
    "Time" => time: f64,
    "Value" => value: f64,
});
