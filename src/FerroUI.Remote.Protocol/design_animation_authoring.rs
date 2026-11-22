use crate::{bson_class, ferro_remote_message_guid};

/// Opt in alongside the animation transport. Older clients still receive the
/// playback catalog.
pub const DESIGN_ANIMATION_AUTHORING_QUERY: &str = "ferroui-design-keyframes";
/// Allow authoring requests to capture a value from the native track sampler.
pub const DESIGN_KEYFRAME_SAMPLING_QUERY: &str = "ferroui-design-keyframe-sampling";

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
    pub curve_channels: Option<Vec<String>>,
    pub can_sample: Option<bool>,
}

bson_class!(DesignAnimationTrack as "DesignAnimationTrack" {
    "Target" => target: i64,
    "Property" => property: String,
    "Keys" => keys: Vec<DesignAnimationKeyframe>,
    "CurveChannels" => curve_channels: Option<Vec<String>>,
    "CanSample" => can_sample: Option<bool>,
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
    pub sample: Option<bool>,
    pub sample_target: Option<i64>,
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
    "Sample" => sample: Option<bool>,
    "SampleTarget" => sample_target: Option<i64>,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignAnimationAuthoringResult {
    pub revision: i64,
    pub request: i64,
    pub patch: Option<DesignAnimationSourcePatch>,
    pub samples: Vec<f64>,
    pub value_kind: String,
    pub sampled_key: Option<DesignAnimationKeyframe>,
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
    "SampledKey" => sampled_key: Option<DesignAnimationKeyframe>,
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

impl DesignAnimationSourcePatch {
    /// Convert a checked byte edit to the editor's UTF-16 coordinate space.
    pub fn from_utf8(source: &str, span: std::ops::Range<usize>, mut replacement: String) -> Result<Self, String> {
        let (mut start, mut end) = (span.start, span.end);
        if source.get(start..end).is_none() {
            return Err("The animation edit has an invalid source range".into());
        }
        let splits_line_ending = |offset: usize| {
            offset > 0 && source.as_bytes().get(offset - 1..offset + 1) == Some(b"\r\n")
        };
        // Editors treat CRLF as one line break. Keep the complete line ending in the patch.
        if splits_line_ending(start) {
            start -= 1;
            replacement.insert(0, '\r');
        }
        if splits_line_ending(end) {
            end += 1;
            replacement.push('\n');
        }
        let offset = |byte| i32::try_from(source[..byte].encode_utf16().count())
            .map_err(|_| "The animation edit exceeds the editor's source range".to_owned());
        Ok(Self { start: offset(start)?, end: offset(end)?, expected: source[start..end].into(), replacement })
    }
}

#[cfg(test)]
mod patch_tests {
    use super::DesignAnimationSourcePatch;

    #[test]
    fn checked_patch_uses_utf16_and_keeps_crlf_boundaries() {
        let source = "🧪\r\nview";
        let patch = DesignAnimationSourcePatch::from_utf8(source, 5..5, "draft".into()).unwrap();
        assert_eq!((patch.start, patch.end), (2, 4));
        assert_eq!(patch.expected, "\r\n");
        assert_eq!(patch.replacement, "\rdraft\n");
        assert!(DesignAnimationSourcePatch::from_utf8(source, 1..2, String::new()).is_err());
    }
}

bson_class!(DesignAnimationSourcePatch as "DesignAnimationSourcePatch" {
    "Start" => start: i32,
    "End" => end: i32,
    "Expected" => expected: String,
    "Replacement" => replacement: String,
});
