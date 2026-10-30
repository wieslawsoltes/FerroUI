use crate::model::*;
use ferroui_designer_collaboration::{DocumentReplica, Presence, SharedEdit, MAX_MESSAGE_BYTES};
use serde_json::{json, Value};
use std::io::Read;
use std::path::Path;
use std::time::{Duration, Instant};

/// Replaceable peer transport; all UI, offline queue and storage semantics are independent of HTTP.
pub trait PeerTransport: Send {
    fn request(&mut self, request: Request) -> Result<Value>;
}
/// A bounded synchronous MCP client, intended for a worker thread. HTTPS validates certificates;
/// plain HTTP is restricted to loopback. Redirects never forward room or transport credentials.
pub struct McpPeer {
    endpoint: Endpoint,
    agent: ureq::Agent,
    session: Option<String>,
    next_id: u64,
}
impl McpPeer {
    pub fn new(endpoint: Endpoint) -> Result<Self> {
        endpoint.bearer.validate()?;
        crate::validate_endpoint_url(&endpoint.url)?;
        Ok(Self {
            endpoint,
            agent: ureq::AgentBuilder::new()
                .redirects(0)
                .timeout(Duration::from_secs(20))
                .timeout_connect(Duration::from_secs(5))
                .build(),
            session: None,
            next_id: 0,
        })
    }
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }
    fn post(&self, message: &Value, session: Option<&str>) -> Result<(Value, Option<String>)> {
        let body = serde_json::to_string(message).map_err(HostError::invalid)?;
        ensure(
            body.len() <= 1024 * 1024,
            ErrorCode::Limit,
            "MCP request exceeds transport limit",
        )?;
        let mut request = self
            .agent
            .post(&self.endpoint.url)
            .set("Content-Type", "application/json")
            .set("Accept", "application/json, text/event-stream")
            .set(
                "Authorization",
                &format!("Bearer {}", self.endpoint.bearer.expose()),
            )
            .set("MCP-Protocol-Version", "2025-11-25");
        if let Some(session) = session {
            request = request.set("Mcp-Session-Id", session);
        }
        let response = request.send_string(&body).map_err(|error| match error {
            ureq::Error::Status(401 | 403, _) => HostError::new(
                ErrorCode::Unauthorized,
                "Collaboration endpoint rejected the bearer credential",
            ),
            ureq::Error::Status(404, _) => {
                HostError::new(ErrorCode::NotFound, "MCP session expired; reconnect")
            }
            _ => HostError::new(ErrorCode::Transport, error.to_string()),
        })?;
        let session = response.header("Mcp-Session-Id").map(str::to_owned);
        if response.status() == 202 {
            return Ok((Value::Null, session));
        }
        let mut bytes = Vec::new();
        response
            .into_reader()
            .take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| HostError::new(ErrorCode::Transport, error.to_string()))?;
        ensure(
            bytes.len() <= 2 * 1024 * 1024,
            ErrorCode::Limit,
            "MCP response exceeds the client limit",
        )?;
        let result: Value = serde_json::from_slice(&bytes)
            .map_err(|error| HostError::new(ErrorCode::Transport, error.to_string()))?;
        if let Some(error) = result.get("error") {
            return Err(HostError::new(
                ErrorCode::Transport,
                error["message"].as_str().unwrap_or("MCP protocol error"),
            ));
        }
        if let Some(id) = message.get("id") {
            ensure(
                result.get("id") == Some(id),
                ErrorCode::Transport,
                "MCP response ID does not match the request",
            )?;
        }
        Ok((result["result"].clone(), session))
    }
    fn initialize(&mut self) -> Result<()> {
        self.next_id = self
            .next_id
            .checked_add(1)
            .ok_or_else(|| HostError::invalid("Request sequence exhausted"))?;
        let(result,session)=self.post(&json!({"jsonrpc":"2.0","id":self.next_id,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"FerroUI Designer","version":env!("CARGO_PKG_VERSION")}}}),None)?;
        ensure(
            result["protocolVersion"] == "2025-11-25",
            ErrorCode::Transport,
            "Unexpected MCP protocol version",
        )?;
        self.session = Some(session.ok_or_else(|| {
            HostError::new(ErrorCode::Transport, "MCP endpoint returned no session")
        })?);
        self.post(
            &json!({"jsonrpc":"2.0","method":"notifications/initialized","params":{}}),
            self.session.as_deref(),
        )?;
        Ok(())
    }
    fn tool(&mut self, name: &str, args: Value) -> Result<Value> {
        for attempt in 0..2 {
            if self.session.is_none() {
                self.initialize()?;
            }
            self.next_id = self
                .next_id
                .checked_add(1)
                .ok_or_else(|| HostError::invalid("Request sequence exhausted"))?;
            let result=self.post(&json!({"jsonrpc":"2.0","id":self.next_id,"method":"tools/call","params":{"name":name,"arguments":args}}),self.session.as_deref());
            match result {
                Err(error) if error.code == ErrorCode::NotFound && attempt == 0 => {
                    self.session = None;
                    continue;
                }
                Err(error) => return Err(error),
                Ok((result, _)) => {
                    if result["isError"] == true {
                        return Err(HostError::new(
                            ErrorCode::Transport,
                            result["content"][0]["text"]
                                .as_str()
                                .unwrap_or("MCP service error"),
                        ));
                    }
                    let payload: Value = if let Some(value) = result.get("structuredContent") {
                        value.clone()
                    } else {
                        serde_json::from_str(result["content"][0]["text"].as_str().ok_or_else(
                            || HostError::new(ErrorCode::Transport, "MCP result contains no JSON"),
                        )?)
                        .map_err(HostError::invalid)?
                    };
                    if let Some(error) = payload.get("error") {
                        return Err(
                            serde_json::from_value(error.clone()).map_err(HostError::invalid)?
                        );
                    }
                    return payload.get("ok").cloned().ok_or_else(|| {
                        HostError::new(
                            ErrorCode::Transport,
                            "Collaboration result envelope missing",
                        )
                    });
                }
            }
        }
        Err(HostError::new(
            ErrorCode::Transport,
            "Could not reconnect MCP session",
        ))
    }
    fn request_large(&mut self, request: &Request, encoded: &str) -> Result<Value> {
        let authority = request.authority();
        let ticket=self.tool("collaboration_transfer",json!({"action":"begin","authority":authority,"bytes":encoded.len(),"sha256":hash(encoded.as_bytes())}))?;
        let id = ticket["id"]
            .as_str()
            .ok_or_else(|| HostError::invalid("Upload ticket missing"))?
            .to_owned();
        let result = (|| {
            let mut offset = 0;
            while offset < encoded.len() {
                let mut end = (offset + TRANSFER_CHUNK_BYTES).min(encoded.len());
                while !encoded.is_char_boundary(end) {
                    end -= 1;
                }
                self.tool("collaboration_transfer",json!({"action":"append","authority":authority,"id":id,"offset":offset,"data":&encoded[offset..end]}))?;
                offset = end;
            }
            self.tool(
                "collaboration_transfer",
                json!({"action":"commit","authority":authority,"id":id}),
            )
        })();
        let _ = self.tool(
            "collaboration_transfer",
            json!({"action":"release","authority":authority,"id":id}),
        );
        result
    }
    fn download(&mut self, result: Value, authority: Authority) -> Result<Value> {
        if let Some(value) = result.get("value") {
            return Ok(value.clone());
        }
        let ticket = &result["download"];
        let id = ticket["id"]
            .as_str()
            .ok_or_else(|| HostError::invalid("Download ticket missing"))?
            .to_owned();
        let bytes = ticket["bytes"]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .filter(|n| *n <= MAX_TRANSFER_BYTES)
            .ok_or_else(|| HostError::invalid("Download size exceeds limit"))?;
        let checksum = ticket["sha256"]
            .as_str()
            .ok_or_else(|| HostError::invalid("Download checksum missing"))?;
        let result = (|| {
            let mut data = String::with_capacity(bytes);
            while data.len() < bytes {
                let chunk = self.tool(
                    "collaboration_transfer",
                    json!({"action":"read","authority":authority,"id":id,"offset":data.len()}),
                )?;
                let text = chunk["data"]
                    .as_str()
                    .ok_or_else(|| HostError::invalid("Chunk missing"))?;
                ensure(
                    !text.is_empty()
                        && text.len() <= TRANSFER_CHUNK_BYTES
                        && data.len() + text.len() <= bytes,
                    ErrorCode::Invalid,
                    "Download chunk length is invalid",
                )?;
                data.push_str(text);
                ensure(
                    chunk["next"].as_u64() == Some(data.len() as u64),
                    ErrorCode::Invalid,
                    "Download offset mismatch",
                )?;
            }
            ensure(
                equal_secret(&hash(data.as_bytes()), checksum),
                ErrorCode::Invalid,
                "Download checksum mismatch",
            )?;
            serde_json::from_str(&data).map_err(HostError::invalid)
        })();
        let _ = self.tool(
            "collaboration_transfer",
            json!({"action":"release","authority":authority,"id":id}),
        );
        result
    }
}
impl PeerTransport for McpPeer {
    fn request(&mut self, request: Request) -> Result<Value> {
        let encoded = serde_json::to_string(&request).map_err(HostError::invalid)?;
        ensure(
            encoded.len() <= MAX_TRANSFER_BYTES,
            ErrorCode::Limit,
            "Request exceeds transfer limit",
        )?;
        let result = if encoded.len() <= 256 * 1024 {
            self.tool("collaboration_request", json!({"request":request}))?
        } else {
            self.request_large(&request, &encoded)?
        };
        self.download(result, request.authority())
    }
}

/// A durable offline outbox is derived from the operation set and the last acknowledged server
/// vector. Repeating an exchange after a dropped response is safe and never deletes local work.
pub struct ReconnectingPeer {
    transport: Box<dyn PeerTransport>,
    endpoint: Endpoint,
    credentials: Credentials,
    epoch: u64,
    replica: DocumentReplica,
    server_version: ferroui_designer_collaboration::VersionVector,
    role: Role,
    retry_at: Instant,
    failures: u32,
    last_error: Option<HostError>,
    checkpoint_path: Option<std::path::PathBuf>,
    indeterminate: bool,
}
impl ReconnectingPeer {
    pub fn new(endpoint: Endpoint, joined: JoinedRoom) -> Result<Self> {
        let transport = Box::new(McpPeer::new(endpoint.clone())?);
        Self::with_transport(endpoint, joined, transport)
    }
    pub fn with_transport(
        endpoint: Endpoint,
        joined: JoinedRoom,
        transport: Box<dyn PeerTransport>,
    ) -> Result<Self> {
        let replica = DocumentReplica::from_snapshot(
            joined.credentials.actor.clone(),
            joined.state.snapshot,
        )?;
        let server_version = replica.version();
        Ok(Self {
            transport,
            endpoint,
            credentials: joined.credentials,
            epoch: joined.state.epoch,
            replica,
            server_version,
            role: joined.state.role,
            retry_at: Instant::now(),
            failures: 0,
            last_error: None,
            checkpoint_path: None,
            indeterminate: false,
        })
    }
    pub fn restore(path: impl AsRef<Path>) -> Result<Self> {
        let checkpoint: PeerCheckpoint = crate::load_private(path.as_ref())?;
        ensure(
            checkpoint.protocol == HOST_PROTOCOL,
            ErrorCode::Invalid,
            "Unsupported peer checkpoint",
        )?;
        let transport = Box::new(McpPeer::new(checkpoint.endpoint.clone())?);
        let replica = DocumentReplica::from_snapshot(
            checkpoint.credentials.actor.clone(),
            checkpoint.snapshot,
        )?;
        Ok(Self {
            transport,
            endpoint: checkpoint.endpoint,
            credentials: checkpoint.credentials,
            epoch: checkpoint.epoch,
            replica,
            server_version: checkpoint.server_version,
            role: checkpoint.role,
            retry_at: Instant::now(),
            failures: 0,
            last_error: None,
            checkpoint_path: Some(path.as_ref().into()),
            indeterminate: false,
        })
    }
    pub fn set_checkpoint_path(&mut self, path: impl AsRef<Path>) -> Result<()> {
        self.check_available()?;
        let previous = self.checkpoint_path.replace(path.as_ref().into());
        if let Err(error) = self.persist() {
            self.checkpoint_path = previous;
            return Err(error);
        }
        Ok(())
    }
    pub fn checkpoint(&self) -> PeerCheckpoint {
        PeerCheckpoint {
            protocol: HOST_PROTOCOL,
            endpoint: self.endpoint.clone(),
            credentials: self.credentials.clone(),
            epoch: self.epoch,
            snapshot: self.replica.snapshot(),
            server_version: self.server_version.clone(),
            role: self.role,
        }
    }
    fn check_available(&self) -> Result<()> {
        ensure(
            !self.indeterminate,
            ErrorCode::Indeterminate,
            "Checkpoint durability was indeterminate; restore the saved peer before continuing",
        )
    }
    fn persist(&mut self) -> Result<()> {
        self.check_available()?;
        if let Some(path) = &self.checkpoint_path {
            if let Err(error) = crate::save_private(path, &self.checkpoint()) {
                self.indeterminate = error.code == ErrorCode::Indeterminate;
                self.last_error = Some(error.clone());
                return Err(error);
            }
        }
        Ok(())
    }
    pub fn document(&self) -> &DocumentReplica {
        &self.replica
    }
    pub fn role(&self) -> Role {
        self.role
    }
    pub fn pending_operations(&self) -> usize {
        self.replica
            .pending_operation_count(&self.server_version, &self.credentials.actor)
    }
    pub fn last_error(&self) -> Option<&HostError> {
        self.last_error.as_ref()
    }
    pub fn submit(&mut self, label: impl Into<String>, edits: Vec<SharedEdit>) -> Result<()> {
        self.check_available()?;
        ensure(
            self.role != Role::Viewer,
            ErrorCode::Forbidden,
            "Viewers cannot author document changes",
        )?;
        let mut next = self.replica.clone();
        next.submit(label, edits)?;
        // Roll back in-memory mutation if persisting the outbox fails.
        let previous = std::mem::replace(&mut self.replica, next);
        if let Err(error) = self.persist() {
            self.replica = previous;
            return Err(error);
        }
        Ok(())
    }
    pub fn sync(&mut self, presence: Option<Presence>) -> Result<ExchangeReply> {
        self.persist()?;
        let message = self.replica.changes_since_bounded(
            &self.server_version,
            Some(&self.credentials.actor),
            MAX_MESSAGE_BYTES,
        )?;
        let request = Request::Exchange {
            credentials: self.credentials.clone(),
            exchange: Exchange {
                epoch: self.epoch,
                known: self.replica.version(),
                message,
                presence,
            },
        };
        let result = self.transport.request(request).and_then(|value| {
            serde_json::from_value::<ExchangeReply>(value).map_err(HostError::invalid)
        });
        match result {
            Ok(reply) => {
                ensure(
                    reply.epoch == self.epoch,
                    ErrorCode::EpochChanged,
                    "Room checkpoint changed; keep local edits until explicit rebase",
                )?;
                let mut replica = self.replica.clone();
                replica.merge(reply.message.clone())?;
                let previous = self.checkpoint();
                self.replica = replica;
                self.server_version = reply.version.clone();
                self.role = reply.role;
                if let Err(error) = self.persist() {
                    self.replica = DocumentReplica::from_snapshot(
                        self.credentials.actor.clone(),
                        previous.snapshot,
                    )?;
                    self.server_version = previous.server_version;
                    self.role = previous.role;
                    return Err(error);
                }
                self.failures = 0;
                self.last_error = None;
                self.retry_at = Instant::now() + Duration::from_millis(250);
                Ok(reply)
            }
            Err(error) => {
                self.failures = self.failures.saturating_add(1);
                self.retry_at = Instant::now()
                    + Duration::from_millis((250u64 << self.failures.min(7)).min(30_000));
                self.last_error = Some(error.clone());
                Err(error)
            }
        }
    }
    pub fn poll(&mut self, presence: Option<Presence>) -> Option<Result<ExchangeReply>> {
        if Instant::now() < self.retry_at {
            None
        } else {
            Some(self.sync(presence))
        }
    }
    /// An explicit CAS rebase onto a new epoch. The caller supplies the remote source they
    /// reviewed and a resolved source; stale resolutions fail without losing the old outbox.
    pub fn rebase(
        &mut self,
        state: RoomState,
        expected_remote: &str,
        resolved: &str,
    ) -> Result<()> {
        self.check_available()?;
        ensure(
            state.room == self.credentials.room,
            ErrorCode::Invalid,
            "Snapshot belongs to another room",
        )?;
        let mut next =
            DocumentReplica::from_snapshot(self.credentials.actor.clone(), state.snapshot)?;
        ensure(
            next.source() == expected_remote,
            ErrorCode::Conflict,
            "Remote source differs from the reviewed snapshot",
        )?;
        let version = next.version();
        if resolved != expected_remote {
            ensure(
                state.role != Role::Viewer,
                ErrorCode::Forbidden,
                "Viewers cannot rebase changes",
            )?;
            next.submit(
                "Resolve checkpoint conflict",
                vec![SharedEdit::ReplaceSource {
                    expected_source: expected_remote.into(),
                    source: resolved.into(),
                }],
            )?;
        }
        let old = self.checkpoint();
        self.replica = next;
        self.epoch = state.epoch;
        self.server_version = version;
        self.role = state.role;
        if let Err(error) = self.persist() {
            self.replica =
                DocumentReplica::from_snapshot(self.credentials.actor.clone(), old.snapshot)?;
            self.epoch = old.epoch;
            self.server_version = old.server_version;
            self.role = old.role;
            return Err(error);
        }
        self.retry_at = Instant::now();
        self.last_error = None;
        Ok(())
    }
}
