use super::EditorRoom;
use crate::client::PeerTransport;
use crate::model::*;
use crate::{HostedServer, McpPeer, ReconnectingPeer, load_private, save_private, storage};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OpenRoom {
    pub directory: PathBuf,
    pub uri: String,
    pub start: StartRoom,
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum StartRoom {
    Host {
        title: String,
        source: String,
        display_name: String,
        public_url: Option<String>,
        bind: Option<SocketAddr>,
    },
    Join {
        invitation: String,
        display_name: String,
    },
    Resume,
}

#[derive(Serialize, Deserialize)]
struct LocalHost {
    administrator: Secret,
    address: SocketAddr,
    public_url: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum Pending {
    Host {
        request: CreateRequest,
    },
    Join {
        endpoint: Endpoint,
        request: JoinRequest,
    },
}

#[derive(Serialize, Deserialize)]
struct SavedRoom {
    uri: String,
    display_name: String,
    title: String,
    local_host: Option<LocalHost>,
    pending: Option<Pending>,
}

impl OpenRoom {
    pub fn connect(self) -> Result<EditorRoom> {
        let (directory, lock) = storage::open_client_directory(&self.directory)?;
        let record_path = directory.join("session.json");
        let checkpoint = directory.join("peer.json");
        let (mut record, server) = match self.start {
            StartRoom::Resume => {
                let record: SavedRoom = load_private(&record_path)?;
                ensure(
                    record.uri == self.uri,
                    ErrorCode::Conflict,
                    "The saved room belongs to a different local document",
                )?;
                let server = record
                    .local_host
                    .as_ref()
                    .map(|host| host.start(&directory))
                    .transpose()?;
                (record, server)
            }
            start => {
                ensure(
                    !record_path.exists() && !checkpoint.exists(),
                    ErrorCode::Conflict,
                    "This sharing session is already saved. Resume it to recover its room and outbox.",
                )?;
                begin(start, self.uri, &directory)?
            }
        };
        let (peer, members) = if checkpoint.exists() {
            (ReconnectingPeer::restore(&checkpoint)?, Vec::new())
        } else {
            bootstrap(&mut record, server.as_ref(), &checkpoint)?
        };
        if record.pending.take().is_some() {
            save_private(&record_path, &record)?;
        }
        let public_url = record.local_host.and_then(|host| host.public_url);
        let storage = RoomStorage {
            directory,
            _lock: lock,
            _server: server,
            public_url,
        };
        Ok(EditorRoom::new(
            peer,
            storage,
            record.title,
            record.display_name,
            members,
        ))
    }
}

fn begin(
    start: StartRoom,
    uri: String,
    directory: &Path,
) -> Result<(SavedRoom, Option<HostedServer>)> {
    let (record, server) = match start {
        StartRoom::Host {
            title,
            source,
            display_name,
            public_url,
            bind,
        } => {
            bounded_name(&title, "Room title")?;
            bounded_name(&display_name, "Display name")?;
            if let Some(url) = &public_url {
                crate::validate_public_endpoint_url(url)?;
            }
            ensure(
                bind.is_none_or(|address| address.ip().is_loopback()),
                ErrorCode::Invalid,
                "Use a loopback bind address and an HTTPS public endpoint",
            )?;
            ferroui_designer_core::Document::new(source.clone()).map_err(HostError::invalid)?;
            let administrator = Secret::generate()?;
            let server = HostedServer::start(directory.join("rooms"), &administrator, bind)?;
            let address = bound_address(&server)?;
            let request = CreateRequest {
                admin: administrator.clone(),
                owner_token: Secret::generate()?,
                title: title.clone(),
                document_id: Secret::generate()?.0,
                source,
                display_name: display_name.clone(),
            };
            (
                SavedRoom {
                    uri,
                    display_name,
                    title,
                    local_host: Some(LocalHost {
                        administrator,
                        address,
                        public_url,
                    }),
                    pending: Some(Pending::Host { request }),
                },
                Some(server),
            )
        }
        StartRoom::Join {
            invitation,
            display_name,
        } => {
            bounded_name(&display_name, "Display name")?;
            let invitation = Invitation::decode(&invitation).map_err(|error| {
                HostError::new(error.code, "Supply a valid FerroUI room invitation")
            })?;
            crate::validate_endpoint_url(&invitation.endpoint.url)?;
            let request = JoinRequest {
                room: invitation.room,
                invite: invitation.invite,
                member_token: Secret::generate()?,
                display_name: display_name.clone(),
            };
            (
                SavedRoom {
                    uri,
                    display_name,
                    title: "Shared XAML".into(),
                    local_host: None,
                    pending: Some(Pending::Join {
                        endpoint: invitation.endpoint,
                        request,
                    }),
                },
                None,
            )
        }
        StartRoom::Resume => unreachable!("resume already dispatched"),
    };
    // Retain the create/redeem key before sending the first request so a lost reply can be retried.
    save_private(&directory.join("session.json"), &record)?;
    Ok((record, server))
}

fn bootstrap(
    record: &mut SavedRoom,
    server: Option<&HostedServer>,
    checkpoint: &Path,
) -> Result<(ReconnectingPeer, Vec<Member>)> {
    let pending = record
        .pending
        .as_ref()
        .ok_or_else(|| HostError::storage("Saved room has no peer checkpoint"))?;
    let (endpoint, request) = match pending {
        Pending::Host { request } => (
            server
                .ok_or_else(|| HostError::storage("Local room host is unavailable"))?
                .endpoint(),
            Request::Create {
                request: request.clone(),
            },
        ),
        Pending::Join { endpoint, request } => (
            endpoint.clone(),
            Request::Join {
                request: request.clone(),
            },
        ),
    };
    let mut transport = McpPeer::new(endpoint.clone())?;
    let joined: JoinedRoom =
        serde_json::from_value(transport.request(request)?).map_err(HostError::invalid)?;
    record.title = joined.state.title.clone();
    let members = joined.state.members.clone();
    let mut peer = ReconnectingPeer::with_transport(endpoint, joined, Box::new(transport))?;
    peer.set_checkpoint_path(checkpoint)?;
    Ok((peer, members))
}

fn bound_address(server: &HostedServer) -> Result<SocketAddr> {
    server
        .endpoint()
        .url
        .strip_prefix("http://")
        .and_then(|url| url.strip_suffix("/mcp"))
        .and_then(|address| address.parse().ok())
        .ok_or_else(|| HostError::storage("Could not determine the local room address"))
}

impl LocalHost {
    fn start(&self, directory: &Path) -> Result<HostedServer> {
        ensure(
            self.address.ip().is_loopback(),
            ErrorCode::Invalid,
            "Saved host must bind to loopback",
        )?;
        HostedServer::start(
            directory.join("rooms"),
            &self.administrator,
            Some(self.address),
        )
    }
}

pub(super) struct RoomStorage {
    pub directory: PathBuf,
    pub _server: Option<HostedServer>,
    pub _lock: File,
    pub public_url: Option<String>,
}
