use crate::{
    hosted_frame,
    projects::Projects,
    protocol::{Image, MAX_SOURCE_BYTES, Source},
};
use ferroui_remote_protocol::designer::{
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
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};
use std::time::Duration;

type Connection = Arc<dyn IFerroRemoteTransportConnection>;
const WAIT_FOR_APP: Duration = Duration::from_secs(8);

#[derive(Default)]
struct Events {
    requested: Option<i64>,
    fixture: Option<String>,
    fixtures: Option<Vec<String>>,
    outcome: Option<Result<(), String>>,
    latest: Option<DesignFrameMessage>,
    disconnected: bool,
}

pub(crate) struct Relay {
    server: Option<DisposableServer>,
    incoming: mpsc::Receiver<Connection>,
    connection: Option<Connection>,
    events: Arc<(Mutex<Events>, Condvar)>,
    sent: Option<(i64, i64)>,
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
            if let Some(outcome) = message.downcast_ref::<UpdateDesignResultMessage>() {
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
        if waiting.recv_timeout(WAIT_FOR_APP) != Ok(true) {
            connection.dispose();
        }
    }

    fn connection(&mut self) -> Result<Connection, String> {
        if let Some(connection) = &self.connection {
            return Ok(connection.clone());
        }
        let connection = self.incoming.recv_timeout(WAIT_FOR_APP)
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
            .wait_timeout_while(self.events.0.lock().unwrap(), WAIT_FOR_APP, |state| {
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
        let (frame, fixture, fixtures) = {
            let mut state = self.events.0.lock().unwrap();
            if state.disconnected {
                return Err("The app-hosted preview disconnected".into());
            }
            if !matches!(state.outcome, Some(Ok(()))) {
                return Ok(None);
            }
            (state.latest.take(), state.fixture.clone(), state.fixtures.clone())
        };
        let Some(DesignFrameMessage { revision, frame }) = frame else {
            return Ok(None);
        };
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
