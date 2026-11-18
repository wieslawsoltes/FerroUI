use crate::{
    hosted_animation,
    hosted_frame,
    projects::Projects,
    protocol::{Image, Language, MAX_SOURCE_BYTES, Source},
};
use ferroui_remote_protocol::designer::{
    DesignAnimationAuthoringRequest, DesignAnimationAuthoringResult,
    DesignAnimationCatalogMessage, DesignAnimationRequestMessage, DesignAnimationResultMessage,
    DesignFrameMessage, DesignFrameReceivedMessage, StartDesignerSessionMessage,
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
    animation_supported: bool,
    animation_catalog: Option<DesignAnimationCatalogMessage>,
    animation_request: Option<i64>,
    animation_result: Option<DesignAnimationResultMessage>,
    keyframe_result: Option<DesignAnimationAuthoringResult>,
    authorable: BTreeSet<i64>,
    authored_xaml: bool,
    disconnected: bool,
}

pub(crate) struct Relay {
    server: Option<DisposableServer>,
    incoming: mpsc::Receiver<Connection>,
    connection: Option<Connection>,
    events: Arc<(Mutex<Events>, Condvar)>,
    sent: Option<(i64, i64)>,
    next_animation: i64,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    animations: Option<serde_json::Value>,
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
            events,
            sent: None,
            next_animation: 0,
            size: (0.0, 0.0),
        })
    }

    pub(crate) fn port(&self) -> u16 {
        self.server.as_ref().unwrap().local_addr().port()
    }

    fn accept(
        connection: Connection,
        nonce: Arc<str>,
        ready: mpsc::SyncSender<Connection>,
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
                if message
                    .downcast_ref::<StartDesignerSessionMessage>()
                    .and_then(|start| start.session_id.as_deref())
                    != Some(nonce.as_ref())
                    || claimed.swap(true, Ordering::AcqRel)
                {
                    connection.dispose();
                    let _ = authorized.try_send(false);
                    return;
                }
                admitted.store(true, Ordering::Release);
                let _ = authorized.try_send(ready.try_send(connection.clone()).is_ok());
                return;
            }
            let mut state = result.0.lock().unwrap();
            if let Some(catalog) = message.downcast_ref::<DesignAnimationCatalogMessage>() {
                if state.requested == Some(catalog.revision) {
                    state.animation_supported = true;
                    let mut catalog = catalog.clone();
                    if state.authored_xaml {
                        state.authorable = catalog.programs.iter().filter(|program| program.authoring.is_some())
                            .map(|program| program.id).collect();
                    } else {
                        catalog.programs.iter_mut().for_each(|program| program.authoring = None);
                    }
                    state.animation_catalog = Some(catalog);
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
        let connection = self.incoming.recv_timeout(STARTUP_TIMEOUT)
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
        Ok(connection)
    }

    pub(crate) fn render(&mut self, design: Design, projects: &Projects) -> Result<Frame, String> {
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
        let document = crate::document::DesignDocument::parse(&design.document, projects.source(&design.document)?)?;
        let connection = self.connection()?;
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
                authored_xaml: design.document.language == Language::Xaml,
                fixture: design.fixture.clone(),
                ..Events::default()
            };
        }
        let source = UpdateXamlMessage {
            xaml: Some(document.xaml.source().to_owned()),
            assembly_path: Some(format!("{}.exe", design.assembly)),
            xaml_file_project_path: Some(design.project_path),
        };
        send(
            &connection,
            Arc::new(UpdateDesignMessage { revision, source, fixture: design.fixture }),
        )?;
        let (state, _) = self
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
        drop(state);
        self.frame()?
            .ok_or_else(|| "The application did not render this source revision".into())
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
        Ok(Some(Frame {
            revision,
            sequence,
            image,
            fixture,
            fixtures,
            animations: animations.map(hosted_animation::catalog),
        }))
    }

    fn start_animation_request(&mut self, revision: i64, sequence: i64) -> Result<(Connection, i64), String> {
        if self.sent != Some((revision, sequence)) {
            return Err("The application preview frame changed".into());
        }
        let connection = self.connection.clone().ok_or("Open the application preview first")?;
        let state = self.events.0.lock().unwrap();
        if !state.animation_supported || state.requested != Some(revision)
            || !matches!(state.outcome, Some(Ok(()))) || state.disconnected
        {
            return Err("Open a current application preview with native animation support".into());
        }
        self.next_animation = self.next_animation.checked_add(1).ok_or("The animation session retired")?;
        Ok((connection, self.next_animation))
    }

    pub(crate) fn animate(&mut self, command: hosted_animation::Command) -> Result<hosted_animation::Sample, String> {
        let (connection, request) = self.start_animation_request(command.revision, command.sequence)?;
        {
            let mut state = self.events.0.lock().unwrap();
            state.animation_request = Some(request);
            state.animation_result = None;
        }
        send(&connection, Arc::new(DesignAnimationRequestMessage {
            revision: command.revision, request, program: command.program, targets: command.targets,
            elapsed: command.elapsed, restart: command.restart,
        }))?;
        let (mut state, _) = self
            .events
            .1
            .wait_timeout_while(self.events.0.lock().unwrap(), FRAME_TIMEOUT, |state| {
                !state.disconnected
                    && state.animation_result.as_ref().is_none_or(|result| {
                        result.error.is_none()
                            && state.latest.as_ref().is_none_or(|frame| {
                                frame.frame.sequence_id <= result.after_sequence
                            })
                    })
            })
            .unwrap();
        state.animation_request = None;
        let result = state.animation_result.take().ok_or("The application did not acknowledge the animation sample")?;
        if let Some(error) = result.error { return Err(error); }
        if state.latest.as_ref().is_none_or(|frame| frame.frame.sequence_id <= result.after_sequence) {
            return Err("The application did not render the requested animation sample".into());
        }
        drop(state);
        let frame = self.frame()?.ok_or("The application animation frame retired")?;
        Ok(hosted_animation::Sample { frame, elapsed: result.elapsed, completed: result.completed })
    }

    pub(crate) fn plan_keyframe(&mut self, draft: hosted_animation::KeyframeDraft) -> Result<serde_json::Value, String> {
        let (connection, request) = self.start_animation_request(draft.revision, draft.sequence)?;
        {
            let mut state = self.events.0.lock().unwrap();
            if !state.authorable.contains(&draft.program) {
                return Err("Choose an XAML program with a registered project animation codec".into());
            }
            state.animation_request = Some(request);
            state.keyframe_result = None;
        }
        send(&connection, Arc::new(DesignAnimationAuthoringRequest {
            revision: draft.revision, request, program: draft.program, target: draft.target,
            property: draft.property, previous: draft.previous, time: draft.time, value: draft.value, easing: draft.easing,
        }))?;
        let (mut state, _) = self.events.1.wait_timeout_while(self.events.0.lock().unwrap(), FRAME_TIMEOUT, |state| {
            !state.disconnected && state.keyframe_result.is_none()
        }).unwrap();
        state.animation_request = None;
        let result = state.keyframe_result.take().ok_or("The application did not return an animation edit plan")?;
        hosted_animation::keyframe(result)
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
