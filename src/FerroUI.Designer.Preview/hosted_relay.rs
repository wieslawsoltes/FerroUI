use crate::{
    hosted_animation,
    hosted_animation_restore::{Presentation, Restore, SourceMap, SourceMaps, SourceScope},
    hosted_frame,
    hosted_states::HostedStates,
    projects::Projects,
    protocol::{Image, Language, MAX_SOURCE_BYTES, Source},
    rsx_animation::AuthoredRsxFrame,
};
use ferroui_remote_protocol::designer::{
    DesignPaletteContextMessage,
    DesignTemplateCatalogMessage,
    DesignKeyframeSelectionResult,
    DesignPropertyTransitionsResult,
    DesignAnimationAuthoringResult, DesignAnimationCatalogMessage, DesignAnimationCurveResult, DesignAnimationResultMessage,
    DesignFrameMessage, DesignFrameReceivedMessage, StartDesignSourceSessionMessage, StartDesignerSessionMessage,
    UpdateDesignMessage, UpdateDesignResultMessage, UpdateXamlMessage,
};
use ferroui_remote_protocol::input::{
    MouseButton, PointerEventMessageBase, PointerMovedEventMessage, PointerPressedEventMessage,
    PointerReleasedEventMessage, ScrollEventMessage, TextInputEventMessage,
};
use ferroui_remote_protocol::viewport::{
    ClientRenderInfoMessage, ClientSupportedPixelFormatsMessage, PixelFormat,
};
use ferroui_remote_protocol::{
    BsonTcpTransport, DisposableServer, IFerroRemoteTransportConnection, Message, TcpTransportBase,
    exception_handler, message_handler,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::Duration;

mod animation;
pub(crate) mod authored;
pub(crate) mod palette;
mod sources;
pub(crate) mod styles;
mod rust_styles;
mod rust_templates;
pub(crate) mod templates;

type Connection = Arc<dyn IFerroRemoteTransportConnection>;
const STARTUP_TIMEOUT: Duration = Duration::from_secs(24);
const FRAME_TIMEOUT: Duration = Duration::from_secs(8);

#[derive(Default)]
struct Events {
    requested: Option<i64>,
    fixture: Option<String>,
    fixtures: Option<Vec<String>>,
    outcome: Option<Result<(), String>>,
    latest: Option<DesignFrameMessage>,
    palette: Option<DesignPaletteContextMessage>,
    templates: Option<DesignTemplateCatalogMessage>,
    animation_supported: bool,
    animation_catalog: Option<DesignAnimationCatalogMessage>,
    animation_request: Option<i64>,
    animation_result: Option<DesignAnimationResultMessage>,
    keyframe_result: Option<DesignAnimationAuthoringResult>,
    keyframe_selection: bool,
    keyframe_selection_result: Option<DesignKeyframeSelectionResult>,
    property_transitions: bool,
    property_transition_result: Option<DesignPropertyTransitionsResult>,
    component_keys: bool,
    component_key_batch: bool,
    curve_result: Option<DesignAnimationCurveResult>,
    authorable: BTreeSet<i64>,
    sampleable_keys: BTreeSet<(i64, i64, String)>,
    curve_programs: BTreeSet<i64>,
    disconnected: bool,
}

pub(crate) struct Relay {
    server: Option<DisposableServer>,
    incoming: mpsc::Receiver<(Connection, bool)>,
    connection: Option<Connection>,
    source_snapshots: bool,
    events: Arc<(Mutex<Events>, Condvar)>,
    sent: Option<(i64, i64)>,
    next_animation: i64,
    animation_sources: SourceMaps,
    authored_rsx: Option<AuthoredRsxFrame>,
    authored: Option<authored::AuthoredPreview>,
    templates: Option<templates::Templates>,
    size: (f64, f64),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Design {
    pub(crate) revision: i64,
    document: Source,
    assembly: String,
    project_path: String,
    fixture: Option<String>,
    restore_animation: Option<Restore>,
    select_clip_key: Option<String>,
    workspace: Option<sources::Workspace>,
}

#[derive(Serialize)]
pub(crate) struct WorkspaceFrame {
    frame: Option<Frame>,
    error: Option<String>,
    dependencies: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub(crate) struct Input {
    revision: i64,
    sequence: i64,
    x: f64,
    y: f64,
    #[serde(flatten)]
    kind: InputKind,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum InputKind {
    Click,
    Scroll { delta: f64 },
    Text { text: String },
}

#[derive(Serialize)]
pub(crate) struct Frame {
    revision: i64,
    sequence: i64,
    image: Image,
    fixture: Option<String>,
    fixtures: Option<Vec<String>>,
    #[serde(rename = "paletteAvailable")]
    palette_available: bool,
    #[serde(rename = "stylesAvailable")]
    styles_available: bool,
    #[serde(rename = "templatesAvailable")]
    templates_available: bool,
    #[serde(rename = "rustScope", skip_serializing_if = "Option::is_none")]
    rust_scope: Option<crate::rsx_animation::AuthoredRustScope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    animations: Option<serde_json::Value>,
    #[serde(rename = "animationPresentation", skip_serializing_if = "Option::is_none")]
    animation_presentation: Option<Presentation>,
}

impl Relay {
    pub(crate) fn listen(nonce: &str) -> Result<Self, String> {
        if nonce.len() != 64 || !nonce.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err("A 256-bit preview session identifier is required".into());
        }
        let nonce = Arc::<str>::from(nonce);
        let (tx, incoming) = mpsc::sync_channel(1);
        let events = Arc::new((Mutex::new(Events::default()), Condvar::new()));
        let shared = events.clone();
        let claimed = Arc::new(AtomicBool::new(false));
        let server = BsonTcpTransport::empty()
            .listen(IpAddr::V4(Ipv4Addr::LOCALHOST), 0, move |connection| {
                Self::accept(
                    connection,
                    nonce.clone(),
                    tx.clone(),
                    shared.clone(),
                    claimed.clone(),
                );
            })
            .map_err(|error| error.to_string())?;
        Ok(Self {
            server: Some(server),
            incoming,
            connection: None,
            source_snapshots: false,
            events,
            sent: None,
            next_animation: 0,
            animation_sources: SourceMaps::default(),
            authored_rsx: None,
            authored: None,
            templates: None,
            size: (0.0, 0.0),
        })
    }

    pub(crate) fn port(&self) -> u16 {
        self.server.as_ref().unwrap().local_addr().port()
    }

    fn accept(
        connection: Connection,
        nonce: Arc<str>,
        ready: mpsc::SyncSender<(Connection, bool)>,
        events: Arc<(Mutex<Events>, Condvar)>,
        claimed: Arc<AtomicBool>,
    ) {
        if claimed.load(Ordering::Acquire) {
            connection.dispose();
            return;
        }
        let (authorized, waiting) = mpsc::sync_channel(1);
        let authenticated = Arc::new(AtomicBool::new(false));
        let admitted = authenticated.clone();
        let weak = Arc::downgrade(&connection);
        let result = events.clone();
        connection.on_message(message_handler(move |_, message| {
            let Some(connection) = weak.upgrade() else {
                return;
            };
            if !admitted.load(Ordering::Acquire) {
                let start = message.downcast_ref::<StartDesignSourceSessionMessage>();
                let session_id = start.and_then(|start| start.session_id.as_deref()).or_else(|| message
                    .downcast_ref::<StartDesignerSessionMessage>()
                    .and_then(|start| start.session_id.as_deref()));
                if session_id != Some(nonce.as_ref())
                    || claimed.swap(true, Ordering::AcqRel)
                {
                    connection.dispose();
                    let _ = authorized.try_send(false);
                    return;
                }
                admitted.store(true, Ordering::Release);
                let _ = authorized.try_send(ready.try_send((connection.clone(), start.is_some())).is_ok());
                return;
            }
            let mut state = result.0.lock().unwrap();
            if let Some(catalog) = message.downcast_ref::<DesignAnimationCatalogMessage>() {
                if state.requested == Some(catalog.revision) {
                    state.animation_supported = true;
                    state.keyframe_selection = catalog.keyframe_selection == Some(true);
                    state.property_transitions = catalog.property_transition_owners.is_some();
                    state.component_keys = catalog.programs.iter().any(|program| {
                        program.authoring.as_ref().is_some_and(|authoring| authoring.tracks.iter()
                            .any(|track| track.keys.iter().any(|key| key.curve_values.is_some())))
                    });
                    state.component_key_batch = catalog.programs.iter().any(|program| {
                        program.authoring.as_ref().is_some_and(|authoring| authoring.component_batch == Some(true))
                    });
                    state.curve_programs = catalog.programs.iter().filter(|program| program.curve_tracks.is_some())
                        .map(|program| program.id).collect();
                    state.animation_catalog = Some(catalog.clone());
                }
            } else if let Some(outcome) = message.downcast_ref::<DesignAnimationResultMessage>() {
                if state.requested == Some(outcome.revision) && state.animation_request == Some(outcome.request) {
                    state.animation_result = Some(outcome.clone());
                    result.1.notify_all();
                }
            } else if let Some(outcome) = message.downcast_ref::<DesignAnimationAuthoringResult>() {
                if state.requested == Some(outcome.revision) && state.animation_request == Some(outcome.request) {
                    state.keyframe_result = Some(outcome.clone());
                    result.1.notify_all();
                }
            } else if let Some(outcome) = message.downcast_ref::<DesignKeyframeSelectionResult>() {
                if state.requested == Some(outcome.revision) && state.animation_request == Some(outcome.request) {
                    state.keyframe_selection_result = Some(outcome.clone());
                    result.1.notify_all();
                }
            } else if let Some(outcome) = message.downcast_ref::<DesignAnimationCurveResult>() {
                if state.requested == Some(outcome.revision) && state.animation_request == Some(outcome.request) {
                    state.curve_result = Some(outcome.clone());
                    result.1.notify_all();
                }
            } else if let Some(outcome) = message.downcast_ref::<DesignPropertyTransitionsResult>() {
                if state.requested == Some(outcome.revision) && state.animation_request == Some(outcome.request) {
                    state.property_transition_result = Some(outcome.clone());
                    result.1.notify_all();
                }
            } else if let Some(context) = message.downcast_ref::<DesignPaletteContextMessage>() {
                if state.requested == Some(context.revision) {
                    state.palette = Some(context.clone());
                }
            } else if let Some(catalog) = message.downcast_ref::<DesignTemplateCatalogMessage>() {
                if state.requested == Some(catalog.revision) {
                    state.templates = Some(catalog.clone());
                }
            } else if let Some(outcome) = message.downcast_ref::<UpdateDesignResultMessage>() {
                if state.requested == Some(outcome.revision) {
                    state.fixtures = outcome.fixtures.clone();
                    state.outcome = Some(match &outcome.result.error {
                        Some(error) => Err(error.clone()),
                        None if state.fixture.as_ref().is_some_and(|name| {
                            state.fixtures.as_ref().is_none_or(|available| !available.contains(name))
                        }) => Err("The application does not expose that preview fixture".into()),
                        None => Ok(()),
                    });
                    result.1.notify_all();
                }
            } else if let Some(frame) = message.downcast_ref::<DesignFrameMessage>() {
                if state.requested == Some(frame.revision) {
                    state.latest = Some(frame.clone());
                    result.1.notify_all();
                }
                drop(state);
                connection.send(Arc::new(DesignFrameReceivedMessage {
                    revision: frame.revision,
                    sequence_id: frame.frame.sequence_id,
                }));
            }
        }));
        connection.on_exception(exception_handler(move |_, _| {
            if authenticated.load(Ordering::Acquire) {
                events.0.lock().unwrap().disconnected = true;
                events.1.notify_all();
            }
        }));
        connection.start();
        if waiting.recv_timeout(STARTUP_TIMEOUT) != Ok(true) {
            connection.dispose();
        }
    }

    fn connection(&mut self) -> Result<Connection, String> {
        if let Some(connection) = &self.connection {
            return Ok(connection.clone());
        }
        let (connection, source_snapshots) = self.incoming.recv_timeout(STARTUP_TIMEOUT)
            .map_err(|_| "The Cargo binary did not start a FerroUI design session. Check its preview arguments and output.")?;
        if let Some(server) = self.server.take() {
            server.dispose();
        }
        send(
            &connection,
            Arc::new(ClientSupportedPixelFormatsMessage {
                formats: Some(vec![PixelFormat::Rgba8888]),
            }),
        )?;
        send(
            &connection,
            Arc::new(ClientRenderInfoMessage {
                dpi_x: 96.0,
                dpi_y: 96.0,
            }),
        )?;
        self.connection = Some(connection.clone());
        self.source_snapshots = source_snapshots;
        Ok(connection)
    }

    pub(crate) fn render(&mut self, design: Design, projects: &Projects) -> Result<Frame, String> {
        self.render_source(design, projects, &mut None)
    }

    pub(crate) fn render_workspace(&mut self, design: Design, projects: &Projects) -> WorkspaceFrame {
        let mut dependencies = None;
        match self.render_source(design, projects, &mut dependencies) {
            Ok(frame) => WorkspaceFrame { frame: Some(frame), error: None, dependencies },
            Err(error) => WorkspaceFrame { frame: None, error: Some(error), dependencies },
        }
    }

    fn render_source(&mut self, design: Design, projects: &Projects, dependencies: &mut Option<Vec<String>>) -> Result<Frame, String> {
        let revision = design.revision;
        if let Some(name) = &design.fixture {
            ferroui_designer_core::design_fixtures::validate_name(name)?;
        }
        if revision < 1
            || !design.document.uri.starts_with("file:")
            || design.document.source.len() > MAX_SOURCE_BYTES
            || design.assembly.is_empty()
            || !design
                .assembly
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
            || !design.project_path.starts_with('/')
            || design
                .project_path
                .split('/')
                .any(|part| matches!(part, "." | ".."))
        {
            return Err("Choose a valid preview document and Cargo assembly".into());
        }
        {
            let mut state = self.events.0.lock().unwrap();
            if state.disconnected {
                return Err("The app-hosted preview disconnected".into());
            }
            if state.requested.is_some_and(|old| revision <= old) {
                return Err("The hosted source revision expired".into());
            }
            *state = Events {
                requested: Some(revision),
                fixture: design.fixture.clone(),
                ..Events::default()
            };
        }
        self.authored = None;
        self.templates = None;
        self.animation_sources.retain(design.restore_animation.as_ref());
        let project = projects.source(&design.document)?;
        let rust = design.workspace.as_ref().and_then(|workspace| workspace.rust_sources(&design, project));
        if let Some(previous) = self.authored_rsx.as_ref()
            .filter(|previous| rust.is_none() && design.document.language == Language::Rust && previous.uri == design.document.uri)
            .and_then(AuthoredRsxFrame::module_sources)
        {
            *dependencies = Some(previous.dependencies().map(str::to_owned).collect());
            return Err("Repair the Rust module sources to update App Preview".into());
        }
        let catalog = crate::document::rsx_catalog(project).clone().with_project_effects();
        let document = crate::document::DesignDocument::parse_with_catalog(
            &design.document, project, rust.as_ref(), &catalog,
        )?;
        let prepared = design.workspace.as_ref().map(|workspace| {
            workspace.prepare(&design, &document.xaml, project.map(|project| &project.resources), dependencies)
        }).transpose()?;
        if let Some(rust) = &rust {
            dependencies.get_or_insert_with(Vec::new).extend(rust.dependencies().map(str::to_owned));
        }
        let scope = SourceScope {
            uri: design.document.uri.clone(),
            assembly: design.assembly.clone(),
            project_path: design.project_path.clone(),
            fixture: design.fixture.clone(),
            invocation: document.rsx.as_ref().map(|projection| projection.invocation.start),
        };
        let connection = self.connection()?;
        let source = UpdateXamlMessage {
            xaml: Some(document.xaml.source().to_owned()),
            assembly_path: Some(format!("{}.exe", design.assembly)),
            xaml_file_project_path: Some(design.project_path),
        };
        let update = UpdateDesignMessage { revision, source, fixture: design.fixture };
        let (message, authored): (Message, _) = match prepared {
            Some(sources) => sources.message(update, self.source_snapshots)?,
            None => (Arc::new(update), None),
        };
        send(&connection, message)?;
        let (mut state, _) = self
            .events
            .1
            .wait_timeout_while(self.events.0.lock().unwrap(), FRAME_TIMEOUT, |state| {
                !state.disconnected
                    && !matches!(state.outcome, Some(Err(_)))
                    && (state.outcome.is_none() || state.latest.is_none())
            })
            .unwrap();
        if let Some(Err(error)) = &state.outcome {
            return Err(error.clone());
        }
        if state.disconnected {
            return Err("The app-hosted preview disconnected".into());
        }
        if !matches!(state.outcome, Some(Ok(()))) || state.latest.is_none() {
            return Err("The application did not render this source revision".into());
        }
        let authored = if design.document.language == Language::Xaml {
            state.palette.take().zip(authored).map(|(context, graph)| {
                let schema = project.map_or_else(|| Arc::clone(crate::document::shared_schema()), |project| Arc::clone(&project.schema));
                authored::AuthoredPreview::new(document.xaml.clone(), design.document.uri.clone(), graph, context, schema)
            })
        } else {
            None
        };
        let templates = state.templates.take();
        let authored_rsx = AuthoredRsxFrame::capture(&document, &design.document.uri);
        let templates = templates.and_then(|catalog| match authored_rsx.as_ref() {
            Some(source) => templates::Templates::capture_rust(&document, source, catalog),
            None => authored.as_ref().map(|source| templates::Templates::capture(source, catalog)),
        });
        let states = state.animation_catalog.as_mut().map(|catalog| {
            if document.rsx.is_some() {
                if let Some(owners) = &mut catalog.property_transition_owners {
                    owners.retain(|owner| u64::try_from(owner.id).ok()
                        .and_then(|id| document.rsx_node(id)).is_some());
                }
            }
            HostedStates::capture(&document.xaml, catalog, authored_rsx.as_ref())
        });
        if let Some(catalog) = &state.animation_catalog {
            let authorable = catalog.programs.iter().filter(|program| program.authoring.is_some())
                .map(|program| program.id).collect();
            let sampleable_keys = catalog.programs.iter().flat_map(|program| {
                program.authoring.iter().flat_map(|authoring| &authoring.tracks)
                    .filter(|track| track.can_sample == Some(true))
                    .map(|track| (program.id, track.target, track.property.clone()))
            }).collect();
            state.authorable = authorable;
            state.sampleable_keys = sampleable_keys;
        }
        let source = state.animation_catalog.as_ref().and_then(|catalog| {
            let mut source = SourceMap::new(document.xaml, scope, catalog, states?)?;
            source.capture_clip_authoring(catalog, authored_rsx.as_ref());
            Some(source)
        });
        drop(state);
        let mut presentation = self.animation_sources.restore(design.restore_animation.as_ref(), source.as_ref());
        if let Some(id) = design.select_clip_key.as_deref().and_then(|key| source.as_ref()?.clip_with_source_key(key)) {
            presentation.get_or_insert_with(Presentation::default).selection = Some(id);
        }
        if let Some(presentation) = &mut presentation {
            if let Err(error) = self.restore_animation(&connection, revision, presentation) {
                // Later polling must not promote the candidate's unposed frame
                // after this render reported a failed animation sample.
                self.events.0.lock().unwrap().outcome = Some(Err(error.clone()));
                return Err(error);
            }
        }
        let mut frame = self.frame()?.ok_or("The application did not render this source revision")?;
        if let (Some(authored), Some(catalog)) = (&authored_rsx, &mut frame.animations) {
            authored.annotate_issues(catalog);
        }
        if let (Some(source), Some(catalog)) = (&source, &mut frame.animations) {
            source.annotate_clips(catalog);
            source.annotate_marker_scopes(catalog);
            source.annotate_clip_authoring(catalog);
            source.states.annotate(catalog);
        }
        self.animation_sources.deliver(source, revision, frame.sequence);
        self.templates = templates;
        self.authored_rsx = authored_rsx;
        self.authored = authored;
        frame.palette_available = self.authored.is_some();
        frame.styles_available = self.authored.is_some() || self.authored_rsx.is_some();
        frame.templates_available = self.templates.is_some();
        frame.rust_scope = self.authored_rsx.as_ref().map(|source| source.scope.clone());
        frame.animation_presentation = presentation;
        Ok(frame)
    }

    pub(crate) fn frame(&mut self) -> Result<Option<Frame>, String> {
        let (frame, fixture, fixtures, animations) = {
            let mut state = self.events.0.lock().unwrap();
            if state.disconnected {
                return Err("The app-hosted preview disconnected".into());
            }
            if !matches!(state.outcome, Some(Ok(()))) {
                return Ok(None);
            }
            let Some(frame) = state.latest.take() else { return Ok(None); };
            (frame, state.fixture.clone(), state.fixtures.clone(), state.animation_catalog.take())
        };
        let DesignFrameMessage { revision, frame } = frame;
        let image = hosted_frame::png(&frame)?;
        let sequence = frame.sequence_id;
        self.size = (
            f64::from(frame.width) * 96.0 / frame.dpi_x.max(1.0),
            f64::from(frame.height) * 96.0 / frame.dpi_y.max(1.0),
        );
        self.sent = Some((revision, sequence));
        self.animation_sources.observe(revision, sequence);
        Ok(Some(Frame {
            revision,
            sequence,
            image,
            fixture,
            fixtures,
            palette_available: self.authored.is_some(),
            styles_available: self.authored.is_some() || self.authored_rsx.is_some(),
            templates_available: self.templates.is_some(),
            rust_scope: self.authored_rsx.as_ref().map(|source| source.scope.clone()),
            animations: animations.map(hosted_animation::catalog),
            animation_presentation: None,
        }))
    }

    pub(crate) fn input(&self, input: Input) -> Result<(), String> {
        let connection = self
            .connection
            .as_ref()
            .ok_or("The hosted application is starting")?;
        let state = self.events.0.lock().unwrap();
        if self.sent != Some((input.revision, input.sequence))
            || state.requested != Some(input.revision)
            || state.disconnected
            || !matches!(state.outcome, Some(Ok(())))
        {
            return Err("The hosted frame changed. Wait for the current frame.".into());
        }
        drop(state);
        if !(0.0..=1.0).contains(&input.x) || !(0.0..=1.0).contains(&input.y) {
            return Err("Interact inside the current preview frame".into());
        }
        let base = PointerEventMessageBase {
            x: input.x * self.size.0,
            y: input.y * self.size.1,
            ..Default::default()
        };
        match input.kind {
            InputKind::Click => {
                send(
                    connection,
                    Arc::new(PointerMovedEventMessage { base: base.clone() }),
                )?;
                send(
                    connection,
                    Arc::new(PointerPressedEventMessage {
                        base: base.clone(),
                        button: MouseButton::Left,
                    }),
                )?;
                send(
                    connection,
                    Arc::new(PointerReleasedEventMessage {
                        base,
                        button: MouseButton::Left,
                    }),
                )
            }
            InputKind::Scroll { delta } if delta.is_finite() => send(
                connection,
                Arc::new(ScrollEventMessage {
                    base,
                    delta_x: 0.0,
                    delta_y: delta.clamp(-10.0, 10.0),
                }),
            ),
            InputKind::Text { text } if text.len() <= 8192 => send(
                connection,
                Arc::new(TextInputEventMessage {
                    text,
                    ..Default::default()
                }),
            ),
            _ => Err("The hosted input is outside the allowed range".into()),
        }
    }
}

fn send(connection: &Connection, message: Message) -> Result<(), String> {
    connection
        .send(message)
        .wait_timeout(Duration::from_secs(2))
        .ok_or("The preview connection timed out")?
        .map_err(|error| error.to_string())
}

impl Drop for Relay {
    fn drop(&mut self) {
        if let Some(server) = self.server.take() {
            server.dispose();
        }
        if let Some(connection) = self.connection.take() {
            connection.dispose();
        }
    }
}
