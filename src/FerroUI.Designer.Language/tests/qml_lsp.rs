//! Exercise the editor entry point, including framing and shutdown.
#![cfg(feature = "qml")]
use ferroui_designer_language::{
    lsp::{read_frame, write_frame},
    LineIndex,
};
use serde_json::{json, Value};
use std::io::BufReader;
use std::process::{Command, Stdio};

struct Client {
    child: std::process::Child,
    input: std::process::ChildStdin,
    output: BufReader<std::process::ChildStdout>,
    id: u32,
}
const URI: &str = "file:///projects/Main.qml";
impl Client {
    fn new() -> Self {
        Self::with_workspace(json!({}))
    }
    fn with_workspace(params: Value) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ferroui-qml-lsp"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let output = BufReader::new(child.stdout.take().unwrap());
        let mut client = Self {
            child,
            input,
            output,
            id: 0,
        };
        assert_eq!(
            client.query("initialize", params)["capabilities"]["textDocumentSync"]["change"],
            2
        );
        client
    }
    fn send(&mut self, method: &str, params: Value, request: bool) -> Value {
        let mut message = json!({"jsonrpc":"2.0", "method":method, "params":params});
        if request {
            self.id += 1;
            message["id"] = self.id.into();
        }
        write_frame(&mut self.input, &message).unwrap();
        read_frame(&mut self.output).unwrap().unwrap()
    }
    fn query(&mut self, method: &str, params: Value) -> Value {
        let result = self.send(method, params, true);
        assert_eq!(result["id"], self.id, "{result}");
        assert!(result.get("error").is_none(), "{result}");
        result["result"].clone()
    }
    fn open(&mut self, uri: &str, text: &str) {
        let msg = self.send(
            "textDocument/didOpen",
            json!({"textDocument":{"uri":uri,"text":text,"version":1}}),
            false,
        );
        assert_eq!(msg["params"]["diagnostics"], json!([]), "{msg}");
    }
    fn change(&mut self, version: u32, changes: Value) -> Value {
        self.send(
            "textDocument/didChange",
            json!({"textDocument":{"uri":URI,"version":version},"contentChanges":changes}),
            false,
        )
    }
    fn at(&mut self, method: &str, source: &str, at: usize) -> Value {
        let position = LineIndex::new(source).position(at).unwrap();
        self.query(
            method,
            json!({"textDocument":{"uri":URI},"position":position}),
        )
    }
    fn finish(mut self) {
        assert!(self.query("shutdown", json!({})).is_null());
        write_frame(&mut self.input, &json!({"jsonrpc":"2.0","method":"exit"})).unwrap();
        drop(self.input);
        assert!(self.child.wait().unwrap().success());
    }
}
#[test]
fn scoped_navigation_and_symbols_follow_open_documents() {
    let source = "import QtQuick\r\nimport \"parts\" as Parts\r\nItem {\r\n  id: root\r\n  property string /* title */ title: '😀'\r\n  function greet(x) { let title = x; return title + root.title }\r\n  property string label: title + 'title'\r\n  Parts.Badge {}\r\n}";
    let mut client = Client::new();
    client.open(URI, source);
    let badge = "file:///projects/parts/Badge.qml";
    client.open(badge, "import QtQuick\nRectangle {}");
    let definition = |c: &mut Client, pos| c.at("textDocument/definition", source, pos);
    let title = source.find("title: '").unwrap();
    let index = LineIndex::new(source);
    assert_eq!(
        definition(&mut client, source.find("root.title").unwrap() + 6)["range"]["start"],
        json!(index.position(title).unwrap())
    );
    assert_eq!(
        definition(&mut client, source.find("root.title").unwrap() + 1)["range"]["start"]["line"],
        3
    );
    assert!(definition(&mut client, source.find("return title").unwrap() + 8).is_null());
    assert!(definition(&mut client, source.rfind("'title'").unwrap() + 2).is_null());
    assert_eq!(
        definition(&mut client, source.find("label: title").unwrap() + 8)["range"]["start"],
        json!(index.position(title).unwrap())
    );
    assert_eq!(
        definition(&mut client, source.find("Parts.Badge").unwrap() + 7)["uri"],
        badge
    );
    let symbols = client.query(
        "textDocument/documentSymbol",
        json!({"textDocument":{"uri":URI}}),
    );
    assert_eq!(symbols[0]["name"], "Item (root)");
    assert_eq!(
        symbols[0]["children"][1]["selectionRange"]["start"],
        json!(index.position(title).unwrap())
    );
    let folds = client.query(
        "textDocument/foldingRange",
        json!({"textDocument":{"uri":URI}}),
    );
    assert_eq!(folds[0]["startLine"], 2);
    let closed = client.send(
        "textDocument/didClose",
        json!({"textDocument":{"uri":badge}}),
        false,
    );
    assert_eq!(closed["params"]["diagnostics"], json!([]));
    assert!(definition(&mut client, source.find("Parts.Badge").unwrap() + 7).is_null());
    client.finish();
}

#[test]
fn completions_and_diagnostics_survive_partial_utf16_edits() {
    let mut client = Client::new();
    let source="import QtQuick as Q\r\nQ.Item { id: root; property string emoji: '😀'; property real v: root.width }";
    client.open(URI, source);
    let diagnostic = client.change(2, json!([{"text":source.replace("root.width","root.")}]));
    assert_eq!(diagnostic["params"]["version"], 2);
    assert_eq!(diagnostic["params"]["diagnostics"][0]["severity"], 1);
    let broken = source.replace("root.width", "root.");
    let items = client.at(
        "textDocument/completion",
        &broken,
        broken.rfind("root.").unwrap() + 5,
    );
    assert!(items
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["label"] == "width"));
    assert!(items
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["label"] == "emoji"));
    let stale = client.change(2, json!([{"text":"stale"}]));
    assert_eq!(stale["method"], "window/logMessage");
    let split = LineIndex::new(&broken)
        .position(broken.find('😀').unwrap())
        .unwrap();
    let split = json!({"line":split.line,"character":split.character+1});
    let invalid = client.change(3, json!([{"range":{"start":split,"end":split},"text":"x"}]));
    assert_eq!(invalid["method"], "window/logMessage");
    let at = LineIndex::new(&broken)
        .position(broken.rfind("root.").unwrap() + 5)
        .unwrap();
    let fixed = client.change(
        3,
        json!([{"range":{"start":at,"end":at},"rangeLength":0,"text":"width"}]),
    );
    assert_eq!(fixed["params"]["diagnostics"], json!([]));
    let items = client.at(
        "textDocument/completion",
        source,
        source.find("Q.Item").unwrap() + 2,
    );
    assert!(items
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["label"] == "Rectangle"));
    assert!(items
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["label"] == "QtObject"));
    let hover = client.at(
        "textDocument/hover",
        source,
        source.find("Q.Item").unwrap() + 3,
    );
    assert_eq!(hover["contents"]["value"], "QtQuick.Item");
    client.finish();
}

#[test]
fn workspace_file_navigation_prefers_an_unsaved_buffer() {
    let dir = std::env::temp_dir().join(format!("ferroui-qml-lsp-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let dir_url = url::Url::from_directory_path(&dir).unwrap();
    let badge = dir_url.join("Badge.qml").unwrap();
    std::fs::write(badge.to_file_path().unwrap(), "import QtQuick\nItem {}").unwrap();
    let uri = dir_url.join("Main.qml").unwrap();
    let mut c = Client::with_workspace(
        json!({"workspaceFolders":[{"name":"project","uri":dir_url.as_str()}]}),
    );
    c.open(uri.as_str(), "import QtQuick\nBadge {}");
    let query = json!({"textDocument":{"uri":uri.as_str()},"position":{"line":1,"character":1}});
    let first = c.query("textDocument/definition", query.clone());
    assert_eq!(first["uri"], badge.as_str());
    assert_eq!(first["range"]["start"]["line"], 1);
    c.open(badge.as_str(), "import QtQuick\n\nRectangle {}");
    assert_eq!(
        c.query("textDocument/definition", query.clone())["range"]["start"]["line"],
        2
    );
    c.send(
        "textDocument/didClose",
        json!({"textDocument":{"uri":badge.as_str()}}),
        false,
    );
    assert_eq!(c.query("textDocument/definition", query.clone()), first);
    let items = c.query("textDocument/completion", query);
    assert!(items
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["label"] == "Badge"));
    c.finish();
    std::fs::remove_file(badge.to_file_path().unwrap()).unwrap();
    std::fs::remove_dir(&dir).unwrap();
}

#[test]
fn inherited_member_navigation_and_completion_use_the_defining_file() {
    let mut client = Client::new();
    client.open(
        "file:///projects/Badge.qml",
        "import QtQuick\nRectangle { property int votes: 1 }",
    );
    let main = "import QtQuick\nBadge { id: badge; votes: 2; property int tally: badge.votes }";
    client.open(URI, main);
    for at in [main.find("votes:").unwrap(), main.rfind("votes").unwrap()] {
        let result = client.at("textDocument/definition", main, at + 1);
        assert_eq!(result["uri"], "file:///projects/Badge.qml");
        assert_eq!(result["range"]["start"], json!({"line":1,"character":25}));
    }
    let broken = main.replace("badge.votes", "badge.");
    client.change(2, json!([{"text":broken}]));
    let items = client.at(
        "textDocument/completion",
        &broken,
        broken.rfind("badge.").unwrap() + 6,
    );
    for label in ["votes", "width", "color"] {
        assert!(
            items
                .as_array()
                .unwrap()
                .iter()
                .any(|i| i["label"] == label),
            "{label}"
        );
    }
    client.finish();
}
