//! Byte protocol for an isolated WorkerScript process. Module code and the QML
//! application never execute in the same V8 realm.
use super::*;

pub(super) fn run(
    engine: &abi::Engine,
    resources: &Mutex<Resources>,
    key: &str,
) -> Result<(), String> {
    let init = framing::receive(&mut io::stdin().lock()).map_err(|e| e.to_string())?;
    let files: BTreeMap<String, String> =
        serde_json::from_value(init["files"].clone()).map_err(|e| e.to_string())?;
    if !files.keys().all(|url| url.starts_with(MODULES)) {
        return Err("invalid worker resource URL".into());
    }
    {
        let mut state = resources.lock().map_err(|_| "poisoned resource table")?;
        state.files = files;
        state.rpc_enabled = true;
    }
    let loader = format!(
        "({})({})",
        include_str!("modules.js"),
        include_str!("vendor/systemjs-core.js")
    );
    let bootstrap = format!("Object.defineProperty(globalThis, '__ferroWorker', {{value: ({})({loader},{key})}}); 'ready'",include_str!("script_vm.js"));
    engine.eval(&bootstrap)?;
    let start =
        json!({"url": init["url"], "resources": init["resources"], "classic": init["classic"]});
    engine.eval(&format!("__ferroWorker.load({start}); 'pending'"))?;
    finish(engine)?;
    loop {
        // V8 may synchronously read an acknowledgement on its native thread.
        // Release stdin before handing the current message to the engine.
        let command = match framing::receive(&mut io::stdin().lock()) {
            Ok(command) => command,
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(e.to_string()),
        };
        let message = command.get("message").ok_or("missing worker message")?;
        engine.eval(&format!("__ferroWorker.receive({message}); 'pending'"))?;
        finish(engine)?;
    }
}

fn finish(engine: &abi::Engine) -> Result<(), String> {
    loop {
        match engine.eval("__ferroWorker.result()")?.as_str() {
            "done" => {
                return framing::send(&mut io::stdout().lock(), &json!({"done": true}))
                    .map_err(|e| e.to_string())
            }
            "pending" => std::thread::sleep(std::time::Duration::from_millis(1)),
            reason => return Err(reason.to_owned()),
        }
    }
}
