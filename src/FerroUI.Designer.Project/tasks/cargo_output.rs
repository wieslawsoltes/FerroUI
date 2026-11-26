use super::{OutputBuffer, OutputStream, TaskOutput, push_output};
use serde_json::Value;
use std::io::{BufRead, BufReader, Read};
use std::path::Path;
use std::sync::Mutex;

const MAX_MESSAGE: u64 = 4 * 1024 * 1024;

pub(super) fn read(reader: impl Read, output: &Mutex<OutputBuffer>) {
    let mut reader = BufReader::new(reader);
    let mut line = Vec::new();
    let mut oversized = false;
    loop {
        line.clear();
        let Ok(length) = reader
            .by_ref()
            .take(MAX_MESSAGE)
            .read_until(b'\n', &mut line)
        else {
            break;
        };
        if length == 0 {
            break;
        }
        let complete = line.ends_with(b"\n");
        if oversized {
            oversized = !complete;
            continue;
        }
        if length as u64 == MAX_MESSAGE && !complete {
            emit(output, "[Oversized Cargo output line omitted]\n".into());
            oversized = true;
            continue;
        }
        let Ok(message) = serde_json::from_slice::<Value>(&line) else {
            emit(output, String::from_utf8_lossy(&line).into_owned());
            continue;
        };
        match message["reason"].as_str() {
            Some("build-script-executed") => {
                if let Some(directory) = message["out_dir"]
                    .as_str()
                    .map(Path::new)
                    .filter(|path| path.is_absolute())
                {
                    let mut output = output.lock().unwrap_or_else(|error| error.into_inner());
                    if output.directories.len() <= crate::markup::MAX_DIRECTORIES {
                        output.directories.insert(directory.to_owned());
                    }
                }
            }
            Some("compiler-message") => {
                if let Some(rendered) = message["message"]["rendered"].as_str() {
                    emit(output, rendered.into());
                }
            }
            Some("compiler-artifact" | "build-finished") => (),
            _ => emit(output, String::from_utf8_lossy(&line).into_owned()),
        }
    }
}

fn emit(buffer: &Mutex<OutputBuffer>, text: String) {
    push_output(
        buffer,
        TaskOutput {
            stream: OutputStream::Stdout,
            text,
        },
    );
}
