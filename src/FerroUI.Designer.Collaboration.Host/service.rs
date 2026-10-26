use crate::model::*;
use crate::RoomStore;
use ferroui_mcp::{RequestContext, ServiceToolProvider};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct Transfer {
    owner: String,
    data: String,
    expected: usize,
    checksum: String,
    created: Instant,
    download: bool,
}
/// Authenticated chunking keeps individual MCP messages below the existing transport limit.
/// Tickets expire after five minutes; failed commits may be retried with the same upload.
pub struct CollaborationService {
    store: Arc<RoomStore>,
    transfers: Mutex<BTreeMap<String, Transfer>>,
}
impl CollaborationService {
    pub fn new(store: Arc<RoomStore>) -> Self {
        Self {
            store,
            transfers: Mutex::new(BTreeMap::new()),
        }
    }
    pub fn store(&self) -> &Arc<RoomStore> {
        &self.store
    }
    fn transfer_lock(&self) -> Result<std::sync::MutexGuard<'_, BTreeMap<String, Transfer>>> {
        let mut transfers = self
            .transfers
            .lock()
            .map_err(|_| HostError::storage("Transfer lock poisoned"))?;
        transfers.retain(|_, value| value.created.elapsed() < Duration::from_secs(300));
        Ok(transfers)
    }
    fn reserve(transfers: &BTreeMap<String, Transfer>, bytes: usize) -> Result<()> {
        ensure(
            bytes <= MAX_TRANSFER_BYTES
                && transfers.len() < 32
                && transfers.values().map(|t| t.expected).sum::<usize>() + bytes
                    <= 256 * 1024 * 1024,
            ErrorCode::Limit,
            "Transfer capacity reached; retry after active transfers finish",
        )
    }
    fn dispatch(&self, request: Request, fence: &dyn CommitFence) -> Result<Value> {
        match request {
            Request::Create { request } => value(self.store.create_checked(request, fence)?),
            Request::Join { request } => value(self.store.join_checked(request, fence)?),
            Request::State { credentials } => value(self.store.state(&credentials)?),
            Request::Invite {
                credentials,
                token,
                role,
                expires_in,
                uses,
            } => {
                self.store
                    .invite_checked(&credentials, token, role, expires_in, uses, fence)?;
                Ok(json!({"saved":true}))
            }
            Request::SetMember {
                credentials,
                actor,
                role,
                revoked,
            } => {
                self.store
                    .set_member_checked(&credentials, &actor, role, revoked, fence)?;
                Ok(json!({"saved":true}))
            }
            Request::Exchange {
                credentials,
                exchange,
            } => value(self.store.exchange_checked(&credentials, exchange, fence)?),
            Request::Checkpoint {
                credentials,
                epoch,
                expected,
                resolved_source,
            } => value(self.store.checkpoint_resolved_checked(
                &credentials,
                epoch,
                &expected,
                resolved_source.as_deref(),
                fence,
            )?),
        }
    }
    fn response(&self, result: Value, owner: String) -> Result<Value> {
        let data = serde_json::to_string(&result).map_err(HostError::storage)?;
        if data.len() <= 256 * 1024 {
            return Ok(json!({"value":result}));
        }
        let mut transfers = self.transfer_lock()?;
        Self::reserve(&transfers, data.len())?;
        let id = Secret::generate()?.0;
        let bytes = data.len();
        let checksum = hash(data.as_bytes());
        transfers.insert(
            id.clone(),
            Transfer {
                owner,
                data,
                expected: bytes,
                checksum: checksum.clone(),
                created: Instant::now(),
                download: true,
            },
        );
        Ok(json!({"download":{"id":id,"bytes":bytes,"sha256":checksum}}))
    }
    fn execute(&self, name: &str, args: &Value, context: &RequestContext) -> Result<Value> {
        let fence = || {
            context
                .check_cancelled()
                .map_err(|error| HostError::new(ErrorCode::Cancelled, error))
        };
        match name {
            "collaboration_request" => {
                let request: Request = decode(&args["request"])?;
                let authority = request.authority();
                self.store.authorize_scope(&authority)?;
                let result = self.dispatch(request, &fence)?;
                self.response(result, authority.key())
            }
            "collaboration_transfer" => {
                let authority: Authority = decode(&args["authority"])?;
                self.store.authorize_scope(&authority)?;
                let action = string(args, "action")?;
                match action {
                    "begin" => {
                        let bytes = number(args, "bytes")?;
                        let checksum = string(args, "sha256")?;
                        ensure(
                            checksum.len() == 64
                                && checksum.bytes().all(|byte| byte.is_ascii_hexdigit()),
                            ErrorCode::Invalid,
                            "Invalid transfer checksum",
                        )?;
                        let mut transfers = self.transfer_lock()?;
                        Self::reserve(&transfers, bytes)?;
                        let id = Secret::generate()?.0;
                        transfers.insert(
                            id.clone(),
                            Transfer {
                                owner: authority.key(),
                                data: String::new(),
                                expected: bytes,
                                checksum: checksum.into(),
                                created: Instant::now(),
                                download: false,
                            },
                        );
                        Ok(json!({"id":id}))
                    }
                    "append" => {
                        let id = string(args, "id")?;
                        let offset = number(args, "offset")?;
                        let data = string(args, "data")?;
                        ensure(
                            data.len() <= TRANSFER_CHUNK_BYTES,
                            ErrorCode::Limit,
                            "Transfer chunk is too large",
                        )?;
                        let mut transfers = self.transfer_lock()?;
                        let transfer = transfers
                            .get_mut(id)
                            .filter(|t| !t.download && t.owner == authority.key())
                            .ok_or_else(|| {
                                HostError::new(
                                    ErrorCode::NotFound,
                                    "Upload ticket expired or belongs to another member",
                                )
                            })?;
                        if offset < transfer.data.len() {
                            ensure(
                                transfer.data.get(offset..offset.saturating_add(data.len()))
                                    == Some(data),
                                ErrorCode::Conflict,
                                "Chunk retry differs from the saved bytes",
                            )?;
                            return Ok(json!({"offset":transfer.data.len()}));
                        }
                        ensure(
                            offset == transfer.data.len()
                                && offset.saturating_add(data.len()) <= transfer.expected,
                            ErrorCode::Invalid,
                            "Chunk offset or length is invalid",
                        )?;
                        transfer.data.push_str(data);
                        Ok(json!({"offset":transfer.data.len()}))
                    }
                    "commit" => {
                        let id = string(args, "id")?;
                        let request: Request = {
                            let transfers = self.transfer_lock()?;
                            let transfer = transfers
                                .get(id)
                                .filter(|t| !t.download && t.owner == authority.key())
                                .ok_or_else(|| {
                                    HostError::new(ErrorCode::NotFound, "Upload ticket expired")
                                })?;
                            ensure(
                                transfer.data.len() == transfer.expected
                                    && equal_secret(
                                        &hash(transfer.data.as_bytes()),
                                        &transfer.checksum,
                                    ),
                                ErrorCode::Invalid,
                                "Upload is incomplete or checksum differs",
                            )?;
                            serde_json::from_str(&transfer.data).map_err(HostError::invalid)?
                        };
                        ensure(
                            request.authority().key() == authority.key(),
                            ErrorCode::Forbidden,
                            "Upload authority differs from request authority",
                        )?;
                        let result = self.dispatch(request, &fence)?;
                        self.response(result, authority.key())
                    }
                    "read" => {
                        let id = string(args, "id")?;
                        let offset = number(args, "offset")?;
                        let transfers = self.transfer_lock()?;
                        let transfer = transfers
                            .get(id)
                            .filter(|t| t.download && t.owner == authority.key())
                            .ok_or_else(|| {
                                HostError::new(ErrorCode::NotFound, "Download ticket expired")
                            })?;
                        ensure(
                            offset <= transfer.data.len() && transfer.data.is_char_boundary(offset),
                            ErrorCode::Invalid,
                            "Download offset must be a UTF-8 boundary",
                        )?;
                        let end = chunk_end(&transfer.data, offset);
                        Ok(
                            json!({"data":&transfer.data[offset..end],"next":end,"complete":end==transfer.data.len()}),
                        )
                    }
                    "release" => {
                        let id = string(args, "id")?;
                        let mut transfers = self.transfer_lock()?;
                        if transfers
                            .get(id)
                            .is_some_and(|t| t.owner == authority.key())
                        {
                            transfers.remove(id);
                        }
                        Ok(json!({"released":true}))
                    }
                    _ => Err(HostError::invalid("Unknown transfer action")),
                }
            }
            _ => Err(HostError::invalid("Unknown collaboration tool")),
        }
    }
}
impl ServiceToolProvider for CollaborationService {
    fn tools(&self) -> Vec<Value> {
        vec![
            json!({"name":"collaboration_request","description":"Durable authenticated designer rooms: create/join/state/invite/set_member/exchange/checkpoint. Each request includes a room credential or creation administrator token.","inputSchema":{"type":"object","properties":{"request":{"type":"object"}},"required":["request"],"additionalProperties":false}}),
            json!({"name":"collaboration_transfer","description":"Authenticated, bounded UTF-8 chunks for requests and snapshots larger than one MCP message. Begin, append, commit, read or release a ticket.","inputSchema":{"type":"object","properties":{"authority":{"type":"object"},"action":{"type":"string","enum":["begin","append","commit","read","release"]},"id":{"type":"string"},"bytes":{"type":"integer","minimum":0},"sha256":{"type":"string"},"offset":{"type":"integer","minimum":0},"data":{"type":"string"}},"required":["authority","action"],"additionalProperties":false}}),
        ]
    }
    fn validate(&self, name: &str, args: &Value) -> std::result::Result<(), String> {
        if !args.is_object() {
            return Err("Arguments must be an object".into());
        }
        match name {
            "collaboration_request" => {
                decode::<Request>(&args["request"]).map_err(|e| e.to_string())?;
            }
            "collaboration_transfer" => {
                decode::<Authority>(&args["authority"]).map_err(|e| e.to_string())?;
                if !args["action"].is_string() {
                    return Err("Transfer action is required".into());
                }
            }
            _ => return Err("Unknown collaboration tool".into()),
        };
        Ok(())
    }
    fn call(
        &self,
        name: &str,
        args: &Value,
        context: &RequestContext,
    ) -> std::result::Result<Value, String> {
        // Typed application errors remain machine-readable through the existing MCP result.
        Ok(match self.execute(name, args, context) {
            Ok(value) => json!({"ok":value}),
            Err(error) => json!({"error":error}),
        })
    }
}
pub(crate) fn chunk_end(data: &str, offset: usize) -> usize {
    let mut end = (offset + TRANSFER_CHUNK_BYTES).min(data.len());
    while !data.is_char_boundary(end) {
        end -= 1;
    }
    end
}
fn decode<T: serde::de::DeserializeOwned>(value: &Value) -> Result<T> {
    serde_json::from_value(value.clone()).map_err(HostError::invalid)
}
fn value(value: impl serde::Serialize) -> Result<Value> {
    serde_json::to_value(value).map_err(HostError::storage)
}
fn string<'a>(args: &'a Value, name: &str) -> Result<&'a str> {
    args[name]
        .as_str()
        .ok_or_else(|| HostError::invalid(format!("{name} must be a string")))
}
fn number(args: &Value, name: &str) -> Result<usize> {
    args[name]
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| HostError::invalid(format!("{name} must be a nonnegative integer")))
}

/// Own this handle for the service lifetime. The HTTP endpoint retains MCP's loopback and
/// Host/Origin checks; deploy an authenticated TLS reverse proxy for remote access.
pub struct HostedServer {
    pub store: Arc<RoomStore>,
    handle: ferroui_mcp::ServerHandle,
}
impl HostedServer {
    pub fn start(
        storage: impl AsRef<Path>,
        admin: &Secret,
        address: Option<SocketAddr>,
    ) -> Result<Self> {
        let store = Arc::new(RoomStore::open(&storage, admin)?);
        let token_path = storage.as_ref().join("transport-key.json");
        let token: Secret = if token_path.exists() {
            crate::load_private(&token_path)?
        } else {
            let token = Secret::generate()?;
            crate::save_private(&token_path, &token)?;
            token
        };
        token.validate()?;
        let options = ferroui_mcp::Options {
            transport: ferroui_mcp::Transport::Http(
                address.unwrap_or_else(|| SocketAddr::from((Ipv4Addr::LOCALHOST, 0))),
            ),
            permissions: ferroui_mcp::Permissions {
                allow_property_writes: true,
                allow_input: false,
            },
            capture_logs: false,
            dispatch_timeout: Duration::from_secs(30),
            ..Default::default()
        };
        let handle = ferroui_mcp::serve_service_with_token(
            options,
            Arc::new(CollaborationService::new(store.clone())),
            token.0,
        )
        .map_err(HostError::storage)?;
        Ok(Self { store, handle })
    }
    pub fn endpoint(&self) -> Endpoint {
        Endpoint {
            url: format!(
                "http://{}/mcp",
                self.handle.address.expect("HTTP service address")
            ),
            bearer: Secret(self.handle.token.clone().expect("HTTP service token")),
        }
    }
    pub fn is_running(&self) -> bool {
        self.handle.is_running()
    }
}
