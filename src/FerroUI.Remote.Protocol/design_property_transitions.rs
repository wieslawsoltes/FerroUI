use crate::design_animation_authoring::DesignAnimationSourcePatch;
use crate::{bson_class, ferro_remote_message_guid};

/// Ask the application to resolve transition properties in its native registry.
pub const DESIGN_PROPERTY_TRANSITIONS_QUERY: &str = "ferroui-design-property-transitions";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignPropertyTransitionsRequest {
    pub revision: i64,
    pub request: i64,
    pub owner: i64,
    pub target_type: Option<String>,
    /// A Designer.Animation property transition command for XAML sources.
    /// Omit it to read the native property choices.
    pub command: Option<String>,
}

ferro_remote_message_guid!(
    DesignPropertyTransitionsRequest,
    "FD80E1E5-D4B7-46F1-93EE-77DB98D81153"
);
bson_class!(DesignPropertyTransitionsRequest as "DesignPropertyTransitionsRequest" {
    "Revision" => revision: i64,
    "Request" => request: i64,
    "Owner" => owner: i64,
    "TargetType" => target_type: Option<String>,
    "Command" => command: Option<String>,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignPropertyTransitionsResult {
    pub revision: i64,
    pub request: i64,
    pub properties: Vec<DesignTransitionProperty>,
    pub patch: Option<DesignAnimationSourcePatch>,
    pub error: Option<String>,
}

ferro_remote_message_guid!(
    DesignPropertyTransitionsResult,
    "2852370B-1720-4BEB-B629-EECC105E88BD"
);
bson_class!(DesignPropertyTransitionsResult as "DesignPropertyTransitionsResult" {
    "Revision" => revision: i64,
    "Request" => request: i64,
    "Properties" => properties: Vec<DesignTransitionProperty>,
    "Patch" => patch: Option<DesignAnimationSourcePatch>,
    "Error" => error: Option<String>,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignTransitionProperty {
    pub property: String,
    pub label: String,
    pub type_name: String,
}

bson_class!(DesignTransitionProperty as "DesignTransitionProperty" {
    "Property" => property: String,
    "Label" => label: String,
    "TypeName" => type_name: String,
});
