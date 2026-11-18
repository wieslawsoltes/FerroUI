//! A private stdio renderer process. The extension owns the authored buffer and
//! applies returned UTF-16 edits through the editor's undo/redo transaction.
mod document;
mod edits;
mod placement;
mod protocol;
mod renderer;
mod snapshot;
mod toolbox;

use ferroui_designer_language::lsp::{read_frame, write_frame};
use renderer::Renderer;
use serde::Serialize;
use serde_json::{json, Value};
use std::io::{self, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    match serve() {
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
    let mut renderer = None;
    while let Some(message) = read_frame(&mut input)? {
        respond(message, &mut renderer, &mut output)?;
    }
    Ok(())
}

fn respond(
    message: Value,
    renderer: &mut Option<Renderer>,
    output: &mut impl Write,
) -> io::Result<()> {
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
        "initialize" => Ok(json!({"version": protocol::PROTOCOL_VERSION, "assets": toolbox::catalog()})),
        "render" => serde_json::from_value(message["params"].clone())
            .map_err(|e| e.to_string())
            .and_then(|input| {
                serialize(snapshot::render(
                    renderer.get_or_insert_with(Renderer::new),
                    &input,
                ))
            }),
        "edit" => serde_json::from_value(message["params"].clone())
            .map_err(|e| e.to_string())
            .and_then(|input| serialize(edits::edit(&input, renderer.as_ref()))),
        "placement" => serde_json::from_value::<protocol::PlacementRequest>(message["params"].clone())
            .map_err(|e| e.to_string())
            .and_then(|input| {
                let stage = renderer.as_ref().ok_or("Render the design before placing controls")?.stage(input.frame)?;
                serialize(stage.plan(&input.placement).map(|(_, hint)| hint))
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
