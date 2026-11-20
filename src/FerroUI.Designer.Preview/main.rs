//! A private stdio renderer process. The extension owns the authored buffer and
//! applies returned UTF-16 edits through the editor's undo/redo transaction.
mod capabilities;
mod document;
mod canvas_guides;
mod canvas_snap;
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
mod interaction;
mod inventory;
mod placement;
mod preview_checks;
mod projects;
mod property_fields;
mod protocol;
mod renderer;
mod spacing;
mod resources;
mod rsx_animation;
mod rsx_placement;
mod scenario;
mod scenario_images;
mod scenario_input;
mod scenario_stages;
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
        Some("--scenario") => scenario::command(),
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
        "configureProject" => serde_json::from_value(message["params"].clone())
            .map_err(|error| error.to_string()).and_then(|request| {
                let retired = projects.register(request)?;
                if let Some(renderer) = renderer.as_mut().filter(|renderer| retired.is_some()
                    && renderer.stage.as_ref().is_some_and(|stage| stage.project == retired)) {
                    renderer.stage = None;
                }
                Ok(Value::Null)
            }),
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
        "hostedListen" => hosted_relay::Relay::listen(message["params"]["nonce"].as_str().unwrap_or("")).map(|relay| {
            use ferroui_remote_protocol::designer::{
                DESIGN_ANIMATION_AUTHORING_QUERY, DESIGN_ANIMATION_CLIPS_QUERY, DESIGN_ANIMATION_CURVE_CHANNELS_QUERY,
                DESIGN_ANIMATION_CURVE_QUERY, DESIGN_ANIMATION_QUERY, DESIGN_ANIMATION_VERSION,
                DESIGN_STATE_ROUTES_QUERY, DESIGN_KEYFRAME_SELECTION_QUERY, DESIGN_CLIP_AUTHORING_QUERY,
                DESIGN_STATE_AUTHORING_QUERY,
            };
            let port = relay.port();
            *hosted = Some(relay);
            let query = [
                DESIGN_ANIMATION_QUERY,
                DESIGN_ANIMATION_AUTHORING_QUERY,
                DESIGN_KEYFRAME_SELECTION_QUERY,
                DESIGN_ANIMATION_CLIPS_QUERY,
                DESIGN_CLIP_AUTHORING_QUERY,
                DESIGN_ANIMATION_CURVE_QUERY,
                DESIGN_ANIMATION_CURVE_CHANNELS_QUERY,
                DESIGN_STATE_ROUTES_QUERY,
                DESIGN_STATE_AUTHORING_QUERY,
            ]
            .map(|key| format!("{key}={DESIGN_ANIMATION_VERSION}"))
            .join("&");
            json!({"port": port, "animationQuery": query})
        }),
        "hostedRender" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|design| serialize(hosted.as_mut().ok_or("Start an app-hosted session")?.render(design, projects))),
        "hostedFrame" => hosted.as_mut().ok_or_else(|| "Start an app-hosted session".into()).and_then(|relay| serialize(relay.frame())),
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
        "stagesPrototypeElements" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|request| serialize(stages.prototype_elements(request, projects))),
        "stagesPrototype" => serde_json::from_value(message["params"].clone()).map_err(|error| error.to_string())
            .and_then(|request| serialize(stages.playback.control(&stages.board, request))),
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
