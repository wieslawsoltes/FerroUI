//! A private stdio renderer process. The extension owns the authored buffer and
//! applies returned UTF-16 edits through the editor's undo/redo transaction.
mod asset_source;
mod capabilities;
mod document;
mod canvas_guides;
mod canvas_snap;
mod collaboration;
mod design_handoff;
mod editor_mcp;
mod edit_selection;
mod edits;
mod event_fields;
mod hosted_animation;
mod hosted_animation_restore;
mod hosted_frame;
mod hosted_relay;
mod hosted_states;
mod inspection;
mod layer_edits;
mod interaction;
mod inventory;
mod placement;
mod preview_checks;
mod projects;
mod project_setup;
mod published_assets;
mod property_fields;
mod protocol;
mod renderer;
mod spacing;
mod resources;
mod rsx_animation;
mod rsx_placement;
mod scenario;
mod scenario_cli;
mod scenario_images;
mod scenario_input;
mod scenario_review;
mod scenario_stages;
mod scenario_suite_report;
mod snapshot;
mod stages;
mod stages_prototype;
mod stages_sources;
mod studio;
mod toolbox;

use ferroui_designer_language::lsp::{read_frame, write_frame};
use renderer::Renderer;
use serde::Serialize;
use serde_json::{json, Value};
use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let result = match std::env::args().nth(1).as_deref() {
        None => serve(),
        Some("--mcp-editor") => editor_mcp::serve(),
        Some("--collaboration") => collaboration::serve(),
        Some("--scenario") => scenario_cli::command(false),
        Some("--scenario-suite") => scenario_cli::command(true),
        Some(_) => Err(io::Error::new(io::ErrorKind::InvalidInput, "Unknown preview mode")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("FerroUI designer preview: {error}");
            ExitCode::FAILURE
        }
    }
}

fn serve() -> io::Result<()> {
    let mut output = io::BufWriter::new(ferroui_mcp::reserve_stdout()?);
    let mut input = io::stdin().lock();
    let mut session = Session::default();
    while let Some(message) = read_frame(&mut input)? {
        respond(message, &mut session, &mut output)?;
    }
    Ok(())
}

#[derive(Default)]
struct Session {
    renderer: Option<Renderer>,
    hosted: Option<hosted_relay::Relay>,
    stages: stages::Stages,
    projects: projects::Projects,
}

fn respond(
    message: Value,
    session: &mut Session,
    output: &mut impl Write,
) -> io::Result<()> {
    let Session { renderer, hosted, stages, projects } = session;
    let id = &message["id"];
    let Some(method) = message["method"]
        .as_str()
        .filter(|_| message["jsonrpc"] == "2.0")
    else {
        return write_frame(
            output,
            &json!({"jsonrpc":"2.0", "id":id, "error":{"code":-32600,"message":"Invalid request"}}),
        );
    };
    if id.is_null() {
        return Ok(());
    }
    let result = match method {
        "initialize" => Ok(capabilities::initialize()),
        "canvasGuideEdit" => serde_json::from_value::<canvas_guides::Request>(message["params"].clone())
            .map_err(|error| error.to_string()).and_then(|request| serialize(request.plan())),
        "canvasSnap" => serde_json::from_value::<canvas_snap::Request>(message["params"].clone())
            .map_err(|error| error.to_string()).and_then(|request| {
                let stage = renderer.as_ref().ok_or("Render the design before snapping")?.stage(request.frame)?;
                serialize(request.resolve(stage))
            }),
        "designTokens" => serde_json::from_value::<ferroui_designer_tools::token_handoff::Request>(message["params"].clone())
            .map_err(|error| error.to_string()).and_then(|request| serialize(request.plan().map_err(|error| error.to_string()))),
        "designHandoff" => serde_json::from_value(message["params"].clone())
            .map_err(|error| error.to_string()).and_then(|request| design_handoff::run(projects, request)),
        "configureProject" => serde_json::from_value(message["params"].clone())
            .map_err(|error| error.to_string()).and_then(|request| {
                if let (Some(project), Some(renderer)) = (projects.register(request)?, renderer.as_mut()) {
                    renderer.retire_project(project);
                }
                Ok(Value::Null)
            }),
        "publishedAssets" => serde_json::from_value(message["params"].clone())
            .map_err(|error| error.to_string()).and_then(|query| published_assets::query(projects, query)),
        "rsxLanguage" => match message["params"]["source"].as_str() {
            Some(source) if source.len() <= protocol::MAX_SOURCE_BYTES => {
                let params = &message["params"];
                projects.document(params["project"].as_u64(), params["uri"].as_str().unwrap_or(""))
                    .and_then(|project| serialize(Ok(match project {
                        Some(project) => project.rsx.analyze(source),
                        None => ferroui_designer_rsx::analyze_rsx(source),
                    })))
            }
            _ => Err("RSX language source must be at most 1 MiB".into()),
        },
        "projectInventory" => message["params"]["workspaceRoot"].as_str()
            .ok_or_else(|| "Select a local workspace folder".to_owned())
            .and_then(|root| serialize(inventory::scan(std::path::Path::new(root)))),
        "projectTemplateCatalog" => Ok(ferroui_designer_project::project_templates::catalog()),
        "projectTemplatePlan" | "projectTemplateCreate" => project_setup::template(method, message["params"].clone()),
        "projectToolchain" => project_setup::toolchain(message["params"].clone()),
        "hostedListen" => hosted_relay::Relay::listen(message["params"]["nonce"].as_str().unwrap_or("")).map(|relay| {
            use ferroui_remote_protocol::designer::{
                DESIGN_ANIMATION_AUTHORING_QUERY, DESIGN_ANIMATION_BOX_SHADOWS_QUERY, DESIGN_ANIMATION_BRUSHES_QUERY, DESIGN_ANIMATION_CLIPS_QUERY, DESIGN_ANIMATION_CURVE_CHANNELS_QUERY,
                DESIGN_ANIMATION_CURVE_QUERY, DESIGN_ANIMATION_EFFECTS_QUERY, DESIGN_ANIMATION_EFFECT_OBJECTS_QUERY,
                DESIGN_ANIMATION_QUERY, DESIGN_ANIMATION_REGISTERED_EFFECTS_QUERY, DESIGN_ANIMATION_TRANSFORM_LISTS_QUERY, DESIGN_ANIMATION_VERSION,
                DESIGN_STATE_ROUTES_QUERY, DESIGN_KEYFRAME_SELECTION_QUERY, DESIGN_CLIP_AUTHORING_QUERY,
                DESIGN_STATE_AUTHORING_QUERY, DESIGN_KEYFRAME_SAMPLING_QUERY, DESIGN_KEYFRAME_COMPONENTS_QUERY,
                DESIGN_PROPERTY_TRANSITIONS_QUERY,
                DESIGN_PALETTE_QUERY, DESIGN_PALETTE_VERSION,
                DESIGN_SOURCE_SNAPSHOT_QUERY, DESIGN_SOURCE_SNAPSHOT_VERSION,
                DESIGN_TEMPLATES_QUERY, DESIGN_TEMPLATES_VERSION,
            };
            let port = relay.port();
            *hosted = Some(relay);
            let animation = [
                DESIGN_ANIMATION_QUERY,
                DESIGN_ANIMATION_AUTHORING_QUERY,
                DESIGN_KEYFRAME_SELECTION_QUERY,
                DESIGN_KEYFRAME_SAMPLING_QUERY,
                DESIGN_KEYFRAME_COMPONENTS_QUERY,
                DESIGN_ANIMATION_CLIPS_QUERY,
                DESIGN_CLIP_AUTHORING_QUERY,
                DESIGN_ANIMATION_CURVE_QUERY,
                DESIGN_ANIMATION_CURVE_CHANNELS_QUERY,
                DESIGN_ANIMATION_EFFECTS_QUERY,
                DESIGN_ANIMATION_EFFECT_OBJECTS_QUERY,
                DESIGN_ANIMATION_REGISTERED_EFFECTS_QUERY,
                DESIGN_ANIMATION_BOX_SHADOWS_QUERY,
                DESIGN_ANIMATION_TRANSFORM_LISTS_QUERY,
                DESIGN_ANIMATION_BRUSHES_QUERY,
                DESIGN_STATE_ROUTES_QUERY,
                DESIGN_STATE_AUTHORING_QUERY,
                DESIGN_PROPERTY_TRANSITIONS_QUERY,
            ]
            .map(|key| format!("{key}={DESIGN_ANIMATION_VERSION}"))
            .join("&");
            let query = format!("{animation}&{DESIGN_SOURCE_SNAPSHOT_QUERY}={DESIGN_SOURCE_SNAPSHOT_VERSION}&{DESIGN_PALETTE_QUERY}={DESIGN_PALETTE_VERSION}&{DESIGN_TEMPLATES_QUERY}={DESIGN_TEMPLATES_VERSION}");
            json!({"port": port, "animationQuery": query})
        }),
        "hostedRender" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|design| serialize(hosted.as_mut().ok_or("Start an app-hosted session")?.render(design, projects))),
        "hostedRenderWorkspace" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|design| serialize(Ok(hosted.as_mut().ok_or("Start an app-hosted session")?.render_workspace(design, projects)))),
        "hostedFrame" => hosted.as_mut().ok_or_else(|| "Start an app-hosted session".into()).and_then(|relay| serialize(relay.frame())),
        "hostedPalette" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|request| hosted.as_ref().ok_or("Start an app-hosted session")?.palette(request)),
        "hostedStyles" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|request| hosted.as_ref().ok_or("Start an app-hosted session")?.styles(request)),
        "hostedTemplates" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|request| hosted.as_ref().ok_or("Start an app-hosted session")?.templates(request)),
        "hostedInput" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|input| serialize(hosted.as_ref().ok_or("Start an app-hosted session")?.input(input))),
        "hostedAnimate" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|command| serialize(hosted.as_mut().ok_or("Start an app-hosted session")?.animate(command))),
        "hostedKeyframe" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|draft| hosted.as_mut().ok_or("Start an app-hosted session")?.plan_keyframe(draft)),
        "hostedKeyframeSelection" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|draft| hosted.as_mut().ok_or("Start an app-hosted session")?.plan_keyframe_selection(draft)),
        "hostedState" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|draft| hosted.as_ref().ok_or("Start an app-hosted session")?.plan_state(draft)),
        "hostedStateGroup" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|draft| hosted.as_ref().ok_or("Start an app-hosted session")?.plan_state_group(draft)),
        "hostedPropertyTransitions" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|request| hosted.as_mut().ok_or("Start an app-hosted session")?.property_transitions(request)),
        "hostedClipPlacement" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|draft| hosted.as_ref().ok_or("Start an app-hosted session")?.plan_clip_placement(draft)),
        "hostedClipEdit" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|draft| hosted.as_ref().ok_or("Start an app-hosted session")?.plan_clip_edit(draft)),
        "hostedCurve" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|request| serialize(hosted.as_mut().ok_or("Start an app-hosted session")?.curve(request))),
        "stagesSync" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|request| {
                renderer.get_or_insert_with(Renderer::new);
                serialize(stages.sync(request, projects))
            }),
        "stagesEdit" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|request| serialize(stages.edit(request, projects))),
        "stagesImage" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|request| serialize(stages.image(request))),
        "stagesChecks" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|request| serialize(stages.checks(request, projects))),
        "stagesStateOptions" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|request| serialize(stages.state_options(request))),
        "stagesFixtureOptions" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|request| serialize(stages.fixture_options(request))),
        "stagesPointer" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|request| stages.pointer(request, projects)),
        "stagesKeyboard" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|request| serialize(stages.keyboard(request))),
        "stagesPointerInput" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|request| serialize(stages.pointer_input(request))),
        "stagesCancelEditorPointerInput" => {
            stages.cancel_editor_pointer_input();
            Ok(Value::Null)
        }
        "stagesPrototypeElements" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|request| serialize(stages.prototype_elements(request, projects))),
        "stagesPrototype" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|request| serialize(stages.control_prototype(request))),
        "stagesPrototypeImage" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|request| serialize(stages.playback.image(request))),
        "stagesPrototypeState" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|request| serialize(stages.playback.state(request))),
        "stagesPrototypeInput" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|request| serialize(stages.playback.input(request))),
        "render" => serde_json::from_value(message["params"].clone())
            .map_err(|e| e.to_string())
            .and_then(|input: protocol::Render| {
                let project = projects.source(&input.document)?;
                serialize(snapshot::render(
                    renderer.get_or_insert_with(Renderer::new),
                    &input,
                    project,
                ))
            }),
        "edit" => serde_json::from_value(message["params"].clone())
            .map_err(|e| e.to_string())
            .and_then(|input| serialize(edits::edit(&input, renderer.as_ref(), projects))),
        "rustModuleSource" => serde_json::from_value(message["params"].clone())
            .map_err(|error| error.to_string())
            .and_then(|check| serialize(studio::verify_rust_module_source(renderer.as_ref().ok_or("Render the current Rust invocation first")?, check))),
        "inspect" => serde_json::from_value(message["params"].clone())
            .map_err(|e| e.to_string())
            .and_then(|input| serialize(inspection::inspect(&input, renderer.as_ref(), projects))),
        "resources" => serde_json::from_value(message["params"].clone())
            .map_err(|e| e.to_string())
            .and_then(|input| serialize(resources::list(&input, renderer.as_ref(), projects))),
        "placement" => serde_json::from_value::<protocol::PlacementRequest>(message["params"].clone())
            .map_err(|e| e.to_string())
            .and_then(|input| {
                let stage = renderer.as_ref().ok_or("Render the design before placing controls")?.stage(input.frame)?;
                serialize(stage.plan(&input.placement, input.recording).map(|(_, hint)| hint))
            }),
        "studioTools" => Ok(studio::catalog()),
        "studioPresent" => serde_json::from_value::<studio::Present>(message["params"].clone())
            .map_err(|error| error.to_string()).and_then(|request| serialize(studio::present(renderer.as_mut().ok_or("Open a designer first")?, request))),
        "canvasInput" => serde_json::from_value(message["params"].clone())
            .map_err(|error| error.to_string()).and_then(|request| serialize(interaction::input(renderer.as_mut().ok_or("Open a designer first")?, request))),
        "studioQuery" => serde_json::from_value::<studio::Query>(message["params"].clone())
            .map_err(|e| e.to_string())
            .and_then(|query| {
                let stage = renderer.as_ref().ok_or("Open a designer first")?.stage(query.frame)?;
                studio::query(stage, &query)
            }),
        "studioEdit" => serde_json::from_value::<studio::Edit>(message["params"].clone())
            .map_err(|e| e.to_string())
            .and_then(|edit| {
                let renderer = renderer.as_mut().ok_or("Open a designer first")?;
                serialize(studio::edit(renderer, edit, projects))
            }),
        _ => {
            return write_frame(
                output,
                &json!({"jsonrpc":"2.0", "id":id, "error":{"code":-32601,"message":"Unknown method"}}),
            )
        }
    };
    let response = match result {
        Ok(result) => json!({"jsonrpc":"2.0", "id":id, "result":result}),
        Err(error) => json!({"jsonrpc":"2.0", "id":id, "error":{"code":-32000,"message":error}}),
    };
    write_frame(output, &response)
}

fn serialize<T: Serialize>(result: Result<T, String>) -> Result<Value, String> {
    serde_json::to_value(result?).map_err(|e| e.to_string())
}
