use ferroui_designer_collaboration::{Conflict, Presence, Snapshot, SyncMessage, VersionVector};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

pub const HOST_PROTOCOL: u32 = 1;
pub const MAX_ROOM_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_TRANSFER_BYTES: usize = 64 * 1024 * 1024;
pub const TRANSFER_CHUNK_BYTES: usize = 64 * 1024;
pub const PRESENCE_TTL_MS: u64 = 15_000;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    EpochChanged,
    Invalid,
    Limit,
    Storage,
    Indeterminate,
    Cancelled,
    Transport,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostError {
    pub code: ErrorCode,
    pub message: String,
}
impl HostError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
    pub(crate) fn invalid(message: impl fmt::Display) -> Self {
        Self::new(ErrorCode::Invalid, message.to_string())
    }
    pub(crate) fn storage(error: impl fmt::Display) -> Self {
        Self::new(ErrorCode::Storage, error.to_string())
    }
}
impl fmt::Display for HostError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for HostError {}
impl From<ferroui_designer_collaboration::CollaborationError> for HostError {
    fn from(value: ferroui_designer_collaboration::CollaborationError) -> Self {
        Self::invalid(value.to_string())
    }
}
pub type Result<T> = std::result::Result<T, HostError>;

/// Redacted in Debug; serialize only into an explicitly protected credential file or invitation.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Secret(pub String);
impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret([redacted])")
    }
}
impl Secret {
    pub fn generate() -> Result<Self> {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(HostError::storage)?;
        Ok(Self(hex(&bytes)))
    }
    pub fn expose(&self) -> &str {
        &self.0
    }
    pub(crate) fn validate(&self) -> Result<()> {
        ensure(
            self.0.len() == 64 && self.0.bytes().all(|b| b.is_ascii_hexdigit()),
            ErrorCode::Invalid,
            "Credentials must be 256-bit hexadecimal tokens",
        )
    }
    pub(crate) fn digest(&self) -> String {
        hash(self.0.as_bytes())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Owner,
    Editor,
    Viewer,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Member {
    pub actor: String,
    pub display_name: String,
    pub role: Role,
    pub revoked: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct StoredMember {
    pub member: Member,
    pub token_hash: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Invite {
    pub role: Role,
    pub expires_at: u64,
    pub remaining: u32,
    pub redeemed: BTreeMap<String, String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct StoredRoom {
    pub protocol: u32,
    pub id: String,
    pub title: String,
    pub epoch: u64,
    pub snapshot: Snapshot,
    pub members: BTreeMap<String, StoredMember>,
    pub invites: BTreeMap<String, Invite>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Credentials {
    pub room: String,
    pub actor: String,
    pub token: Secret,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Endpoint {
    pub url: String,
    pub bearer: Secret,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Invitation {
    pub protocol: u32,
    pub endpoint: Endpoint,
    pub room: String,
    pub invite: Secret,
}
impl Invitation {
    pub fn encode(&self) -> Result<String> {
        serde_json::to_string(self).map_err(HostError::storage)
    }
    pub fn decode(value: &str) -> Result<Self> {
        ensure(
            value.len() <= 16 * 1024,
            ErrorCode::Limit,
            "Invitation is too large",
        )?;
        let invitation: Self = serde_json::from_str(value).map_err(HostError::invalid)?;
        ensure(
            invitation.protocol == HOST_PROTOCOL,
            ErrorCode::Invalid,
            "Unsupported invitation version",
        )?;
        invitation.invite.validate()?;
        invitation.endpoint.bearer.validate()?;
        Ok(invitation)
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RoomState {
    pub room: String,
    pub title: String,
    pub epoch: u64,
    pub snapshot: Snapshot,
    pub members: Vec<Member>,
    pub role: Role,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Exchange {
    pub epoch: u64,
    pub known: VersionVector,
    pub message: SyncMessage,
    pub presence: Option<Presence>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExchangeReply {
    pub epoch: u64,
    pub version: VersionVector,
    pub message: SyncMessage,
    pub conflicts: Vec<Conflict>,
    pub presence: Vec<Presence>,
    pub role: Role,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JoinRequest {
    pub room: String,
    pub invite: Secret,
    pub member_token: Secret,
    pub display_name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreateRequest {
    pub admin: Secret,
    pub owner_token: Secret,
    pub title: String,
    pub document_id: String,
    pub source: String,
    pub display_name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JoinedRoom {
    pub credentials: Credentials,
    pub state: RoomState,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Authority {
    Administrator { token: Secret },
    Member { credentials: Credentials },
    Invitation { room: String, token: Secret },
}
impl Authority {
    #[cfg(feature = "server")]
    pub(crate) fn key(&self) -> String {
        hash(
            serde_json::to_string(self)
                .expect("authority is serializable")
                .as_bytes(),
        )
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Request {
    Create {
        request: CreateRequest,
    },
    Join {
        request: JoinRequest,
    },
    State {
        credentials: Credentials,
    },
    Invite {
        credentials: Credentials,
        token: Secret,
        role: Role,
        expires_in: u64,
        uses: u32,
    },
    SetMember {
        credentials: Credentials,
        actor: String,
        role: Role,
        revoked: bool,
    },
    Exchange {
        credentials: Credentials,
        exchange: Exchange,
    },
    Checkpoint {
        credentials: Credentials,
        epoch: u64,
        expected: VersionVector,
        /// Explicit owner-reviewed source; required when the old operation log has conflicts.
        resolved_source: Option<String>,
    },
}
impl Request {
    pub fn authority(&self) -> Authority {
        match self {
            Self::Create { request } => Authority::Administrator {
                token: request.admin.clone(),
            },
            Self::Join { request } => Authority::Invitation {
                room: request.room.clone(),
                token: request.invite.clone(),
            },
            Self::State { credentials }
            | Self::Invite { credentials, .. }
            | Self::SetMember { credentials, .. }
            | Self::Exchange { credentials, .. }
            | Self::Checkpoint { credentials, .. } => Authority::Member {
                credentials: credentials.clone(),
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeerCheckpoint {
    pub protocol: u32,
    pub endpoint: Endpoint,
    pub credentials: Credentials,
    pub epoch: u64,
    pub snapshot: Snapshot,
    pub server_version: VersionVector,
    pub role: Role,
}

pub(crate) fn hash(bytes: &[u8]) -> String {
    hex(&Sha256::digest(bytes))
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
pub(crate) fn equal_secret(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.bytes()
            .zip(b.bytes())
            .fold(0u8, |acc, (a, b)| acc | (a ^ b))
            == 0
}
pub(crate) fn ensure(condition: bool, code: ErrorCode, message: impl Into<String>) -> Result<()> {
    if condition {
        Ok(())
    } else {
        Err(HostError::new(code, message))
    }
}
pub(crate) fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
pub(crate) fn bounded_name(value: &str, label: &str) -> Result<()> {
    ensure(
        !value.is_empty() && value.len() <= 256 && !value.chars().any(char::is_control),
        ErrorCode::Invalid,
        format!("{label} must be 1..256 non-control UTF-8 bytes"),
    )
}

/// Requests check cancellation immediately before the commit point. No acknowledgement precedes
/// a successful fsync/atomic rename. Callers may supply an MCP cancellation fence.
pub trait CommitFence {
    fn check(&self) -> Result<()>;
}
impl<F: Fn() -> Result<()>> CommitFence for F {
    fn check(&self) -> Result<()> {
        self()
    }
}
pub(crate) fn no_cancel() -> Result<()> {
    Ok(())
}
