//! The host binary (docs/osprey-native-host.md, section 2).
//!
//! ```text
//! ferroui-osprey-native [arguments]        the application's window; the arguments go to it
//! ferroui-osprey-native --smoke            the probe's smoke run without a window
//! ferroui-osprey-native --smoke <script>   an application's recorded smoke.jsonl without a window
//! ```

fn main() {
    std::process::exit(run());
}

#[cfg(osprey_app)]
fn run() -> i32 {
    use ferroui_osprey_native::boundary::Linked;
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let linked = match Linked::new() {
        Ok(linked) => Box::new(linked),
        Err(problem) => {
            eprintln!("the Osprey application could not be loaded: {problem}");
            return 1;
        }
    };
    if arguments.first().map(String::as_str) == Some("--smoke") {
        let context = ferroui_osprey_native::launch_context(&[]);
        let result = match arguments.get(1) {
            Some(path) => std::fs::read_to_string(path)
                .map_err(|problem| format!("could not read {path}: {problem}"))
                .and_then(|script| ferroui_osprey_native::smoke::replay(linked, &context, &script))
                .map(|steps| format!("smoke ok ({steps} steps)")),
            None => ferroui_osprey_native::smoke::probe(linked, &context)
                .map(|()| "smoke ok".to_owned()),
        };
        return match result {
            Ok(message) => {
                println!("{message}");
                0
            }
            Err(problem) => {
                eprintln!("smoke failed: {problem}");
                1
            }
        };
    }
    ferroui_osprey_native::desktop::run(linked, &arguments)
}

#[cfg(not(osprey_app))]
fn run() -> i32 {
    eprintln!(
        "no Osprey application is linked: build with OSPREY_NATIVE_APP_DIR naming the folder of libOspreyApp.a \
         and OSPREY_RUNTIME_DIR the folder of the Osprey runtime archives (docs/osprey-native-host.md)"
    );
    2
}
