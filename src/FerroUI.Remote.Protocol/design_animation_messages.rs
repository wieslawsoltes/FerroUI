use crate::{bson_class, ferro_remote_message_guid};

/// An IDE opts in with `?ferroui-design-animation=1` on the TCP preview URI.
/// A host without this protocol still echoes its session and responds to ordinary design updates.
pub const DESIGN_ANIMATION_QUERY: &str = "ferroui-design-animation";
pub const DESIGN_ANIMATION_VERSION: &str = "1";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignAnimationCatalogMessage {
    pub revision: i64,
    pub programs: Vec<DesignAnimationProgram>,
    pub targets: Vec<DesignAnimationTarget>,
}

ferro_remote_message_guid!(
    DesignAnimationCatalogMessage,
    "C094EFD2-D7FC-467D-AC22-8FA047C4B54B"
);
bson_class!(DesignAnimationCatalogMessage as "DesignAnimationCatalogMessage" {
    "Revision" => revision: i64,
    "Programs" => programs: Vec<DesignAnimationProgram>,
    "Targets" => targets: Vec<DesignAnimationTarget>,
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
/// A request without a program disposes the animation layer. An explicit seek restarts its clock.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DesignAnimationRequestMessage {
    pub revision: i64,
    pub request: i64,
    pub program: Option<i64>,
    pub targets: Option<Vec<i64>>,
    pub elapsed: f64,
    pub restart: bool,
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
