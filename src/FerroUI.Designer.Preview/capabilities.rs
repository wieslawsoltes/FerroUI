use serde_json::{Value, json};

pub(crate) fn initialize() -> Value {
    let capabilities: serde_json::Map<String, Value> = [
        "isolatedDesign",
        "appHostedDesign",
        "hostedAnimationRestoration",
        "hostedPreviewRanges",
        "hostedClipAuthoring",
        "hostedStateAuthoring",
        "rsxLanguage",
        "stages",
        "prototypeEditing",
        "stageIncludes",
        "projectCatalog",
        "canvasInteraction",
        "canvasFocusOrder",
        "spacingHelpers",
        "selectionWrapping",
        "columnAuthoring",
        "itemTemplateAuthoring",
        "propertyTrace",
        "styleComposition",
        "projectInventory",
        "tokenHandoff",
        "designFixtures",
        "timelineTransport",
        "timelineRecording",
        "timelineGeometryRecording",
        "timelineKeyframeDrafts",
        "timelineKeyframeScale",
        "timelineKeyframeTransfer",
        "rsxKeyframeStructure",
        "rsxAnimationPrograms",
        "timelineCurves",
        "timelineCurveVelocity",
        "timelineCurveChannels",
        "timelineCombinedPlayback",
        "timelineProgramTargets",
        "timelineClipPlacement",
        "stateTransitionPreview",
        "rsxStateAuthoring",
        "rsxStateStructure",
    ]
    .into_iter()
    .map(|name| (name.to_owned(), Value::Bool(true)))
    .collect();
    json!({
        "version": crate::protocol::PROTOCOL_VERSION,
        "assets": crate::toolbox::catalog(),
        "canvasGuides": crate::canvas_guides::catalog(),
        "capabilities": capabilities,
    })
}
