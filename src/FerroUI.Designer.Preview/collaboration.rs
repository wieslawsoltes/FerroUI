//! A room worker has its own process lifetime, independent from any design frame or tab.
use ferroui_designer_collaboration_host::{
    ErrorCode, HostError, Result, Role,
    editor::{EditorRoom, OpenRoom},
};
use ferroui_designer_language::lsp::{read_frame, write_frame};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io;

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Command {
    Open {
        options: OpenRoom,
    },
    Tick {
        refresh: bool,
    },
    Review,
    PrepareApply {
        revision: String,
        before: String,
    },
    Publish {
        revision: String,
        source: String,
    },
    Recovery,
    Invite {
        role: Role,
    },
    Member {
        actor: String,
        role: Role,
        revoked: bool,
    },
    Resolve {
        revision: String,
        local: String,
        resolved: Option<String>,
    },
    Checkpoint {
        revision: String,
        local: String,
        resolved: Option<String>,
    },
    Close,
}

pub(super) fn serve() -> io::Result<()> {
    let mut output = io::BufWriter::new(ferroui_mcp::reserve_stdout()?);
    let mut input = io::stdin().lock();
    let mut room = None;
    while let Some(message) = read_frame(&mut input)? {
        let id = &message["id"];
        if id.is_null() {
            continue;
        }
        let method = message["method"]
            .as_str()
            .filter(|_| message["jsonrpc"] == "2.0");
        let result = match method {
            Some("initialize") => Ok(json!({ "version": 1 })),
            Some("room") => serde_json::from_value(message["params"].clone())
                .map_err(invalid)
                .and_then(|command| apply(&mut room, command)),
            _ => Err(HostError::new(
                ErrorCode::Invalid,
                "Unknown collaboration request",
            )),
        };
        let reply = match result {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err(error) => json!({ "jsonrpc": "2.0", "id": id,
                "error": { "code": -32000, "message": error.message, "data": { "code": error.code } } }),
        };
        write_frame(&mut output, &reply)?;
    }
    Ok(())
}

fn apply(room: &mut Option<EditorRoom>, command: Command) -> Result<Value> {
    let room = match command {
        Command::Open { options } => {
            if room.is_some() {
                return Err(HostError::new(
                    ErrorCode::Conflict,
                    "Disconnect the current room first",
                ));
            }
            let mut opened = options.connect()?;
            opened.tick(false);
            let status = value(opened.status())?;
            *room = Some(opened);
            return Ok(status);
        }
        Command::Close => {
            *room = None;
            return Ok(Value::Null);
        }
        _ => room.as_mut().ok_or_else(|| {
            HostError::new(ErrorCode::NotFound, "Open a collaboration room first")
        })?,
    };
    match command {
        Command::Tick { refresh } => room.tick(refresh),
        Command::Review => return value(room.review()?),
        Command::PrepareApply { revision, before } => room.prepare_apply(&revision, &before)?,
        Command::Publish { revision, source } => room.publish(&revision, source)?,
        Command::Recovery => return value(room.recovery_source()?),
        Command::Invite { role } => return value(room.invite(role)?),
        Command::Member {
            actor,
            role,
            revoked,
        } => room.set_member(actor, role, revoked)?,
        Command::Resolve {
            revision,
            local,
            resolved,
        } => room.resolve(&revision, &local, resolved.as_deref())?,
        Command::Checkpoint {
            revision,
            local,
            resolved,
        } => room.checkpoint(&revision, &local, resolved)?,
        Command::Open { .. } | Command::Close => unreachable!(),
    }
    value(room.status())
}

fn value(result: impl Serialize) -> Result<Value> {
    serde_json::to_value(result).map_err(invalid)
}

fn invalid(error: serde_json::Error) -> HostError {
    HostError::new(ErrorCode::Invalid, error.to_string())
}
