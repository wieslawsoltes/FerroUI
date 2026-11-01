#![cfg(feature = "javascript")]

use ferroui_qml_build::Compiler;
use ferroui_qml_compiler::{Binding, Expr, MemberKind, ScriptKind};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
};

struct Project(PathBuf);
impl Project {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "ferroui-qml-js-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn file(&self, name: &str, data: &str) {
        let path = self.0.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, data).unwrap();
    }
    fn fixture() -> Self {
        let p = Self::new();
        p.file(
            "Main.qml",
            r#"import QtQml
import "legacy.js" as Logic
import "a/tool.mjs" as Core
QtObject {
    property int value: Logic.next() + Core.get()
    property var later: () => import("./optional.mjs")
}
"#,
        );
        p.file(
            "legacy.js",
            ".pragma library\n.import 'b/other.js' as Private\nfunction next(){return Private.n}",
        );
        p.file("b/other.js", "var n=1");
        p.file("a/tool.mjs","import {b} from '../b/tool.mjs'; export const get=()=>1+b(); export const load=()=>import('../lazy.mjs');");
        p.file("b/tool.mjs","import {get} from '../a/tool.mjs'; export function b(){ return typeof get == 'function' ? 2 : 0; }");
        p.file("lazy.mjs", "throw new Error('delayed side effect');");
        p.file("optional.mjs", "export default 'only when requested';");
        p
    }
}
impl Drop for Project {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn embeds_the_static_and_dynamic_graph_without_eager_dynamic_evaluation() {
    let p = Project::fixture();
    let loaded = Compiler::new()
        .with_javascript()
        .load(p.0.join("Main.qml"))
        .unwrap();
    assert_eq!(loaded.document.scripts.len(), 6);
    for file in [
        "Main.qml",
        "legacy.js",
        "b/other.js",
        "a/tool.mjs",
        "b/tool.mjs",
        "optional.mjs",
        "lazy.mjs",
    ] {
        assert!(loaded
            .dependencies
            .contains(&p.0.join(file).canonicalize().unwrap()));
    }
    let resources = &loaded.document.scripts;
    let legacy = resources
        .values()
        .find(|r| r.kind == ScriptKind::Library)
        .unwrap();
    assert!(legacy.preload);
    assert_eq!(legacy.imports.len(), 1);
    assert!(!resources[&legacy.imports["Private"]].preload);
    let (first_key, first) = resources
        .iter()
        .find(|(_, r)| r.source.contains("1+b()"))
        .unwrap();
    assert!(first.preload);
    let second_key = &first.dependencies["../b/tool.mjs"];
    assert_ne!(first_key, second_key);
    assert_eq!(
        &resources[second_key].dependencies["../a/tool.mjs"],
        first_key
    );
    let lazy = &first.dependencies["../lazy.mjs"];
    assert!(!resources[lazy].preload);
    assert!(first.source.contains("import('../lazy.mjs')"));
    assert!(first.executable.contains("System.register"));
    assert!(first.executable.contains(&format!(".import(\"{lazy}\")")));
    let MemberKind::Property {
        value: Some(Binding::Expr(Expr::JavaScript(later))),
        ..
    } = &loaded.document.root.members[1].kind
    else {
        panic!()
    };
    let (optional, _) = resources
        .iter()
        .find(|(_, r)| r.source.contains("only when requested"))
        .unwrap();
    assert!(later.code().contains(optional));
    assert!(resources.values().filter(|r| r.preload).count() == 2);
}

#[test]
fn relocation_preserves_generated_output_and_malformed_imports_fail_the_build() {
    let a = Project::fixture();
    let b = Project::fixture();
    let compile = |p: &Path| {
        Compiler::new()
            .with_javascript()
            .compile(p.join("Main.qml"))
    };
    let source = compile(&a.0).unwrap().rust;
    assert_eq!(source, compile(&b.0).unwrap().rust);
    assert!(!source.contains(a.0.to_str().unwrap()));
    a.file("lazy.mjs", "export const bad = (");
    let error = compile(&a.0).err().unwrap().to_string();
    assert!(error.contains("lazy.mjs"), "{error}");
    a.file(
        "lazy.mjs",
        "export function importUnknown(name){return import(name)}",
    );
    // No guessed dynamic target is read at build time.
    assert!(compile(&a.0).is_ok());
}

#[test]
fn registers_live_exports_and_lowers_only_real_import_expressions() {
    let p = Project::new();
    p.file("Main.qml",r#"import QtQml
import "state.mjs" as State
import "classic.js" as Classic
QtObject {
    method: () => { let __ferroModuleContext = 9; return [import.meta.url, import("./deferred.mjs"), __ferroModuleContext] }
}"#);
    p.file(
        "state.mjs",
        r#"
        export {default as origin} from "./deferred.mjs";
        export let counter = 0;
        const _context = 3;
        export function advance(){ return ++counter + _context; }
        export const identity = import.meta.url;
        export async function later(){return import("./deferred.mjs");}
        await Promise.resolve();
    "#,
    );
    p.file("deferred.mjs", "export default import.meta.url");
    p.file(
        "classic.js",
        r#"function run(){
       const __ferroModuleContext = 42;
       const text = "import.meta and import('./absent.mjs')";
       return [__ferroModuleContext, import.meta.url, import("./deferred.mjs"), text]
    }"#,
    );
    let doc = Compiler::new()
        .with_javascript()
        .load(p.0.join("Main.qml"))
        .unwrap()
        .document;
    let classic = doc
        .scripts
        .values()
        .find(|r| r.kind == ScriptKind::Script)
        .unwrap();
    assert_eq!(
        classic.import_hook.as_deref(),
        Some("__ferroModuleContext_")
    );
    assert!(classic
        .executable
        .contains("__ferroModuleContext_.meta.url"));
    assert!(classic
        .executable
        .contains("__ferroModuleContext_.import(\"https://qml.invalid/"));
    assert!(classic
        .executable
        .contains("import.meta and import('./absent.mjs')"));
    assert!(classic.source.contains("import(\"./deferred.mjs\")"));
    let module = doc
        .scripts
        .values()
        .find(|r| r.source.contains("++counter"))
        .unwrap();
    assert!(module.executable.contains("System.register("));
    assert!(module.executable.contains(".meta.url"));
    // An async execute body is driven by observable Promise reactions. SWC
    // emits the generator helper before the module's System.register call.
    assert!(module.executable.contains("yield Promise.resolve()"));
    assert!(!module.executable.contains("execute: async function"));
    assert!(!module.executable.contains("import.meta"));
    let MemberKind::Binding {
        value: Binding::Expr(Expr::JavaScript(script)),
        ..
    } = &doc.root.members[0].kind
    else {
        panic!()
    };
    assert!(script.source.contains("import.meta"));
    assert!(script.code().contains("__ferroModuleContext_.meta.url"));
}

#[test]
fn extra_javascript_entries_are_lazy_relocatable_and_watched_with_their_literal_dependencies() {
    let compile = |p: &Project| {
        p.file(
            "Main.qml",
            "import QtQml\nQtObject { function load(name) { return import(name) } }",
        );
        p.file("plugins/en/index.mjs", "export {value} from './child.mjs';");
        p.file("plugins/en/child.mjs", "export const value='en';");
        p.file("plugins/pl/index.mjs", "export const value='pl';");
        p.file(
            "plugins/unlisted.mjs",
            "throw new Error('unlisted file must not be included');",
        );
        Compiler::new()
            .javascript_entry("plugins/pl/index.mjs")
            .javascript_entry("plugins/en/index.mjs")
            .compile(p.0.join("Main.qml"))
            .unwrap()
    };
    let a = Project::new();
    let b = Project::new();
    let result = compile(&a);
    assert_eq!(result.rust, compile(&b).rust);
    assert!(!result.rust.contains(a.0.to_str().unwrap()));
    for file in [
        "plugins/en/index.mjs",
        "plugins/en/child.mjs",
        "plugins/pl/index.mjs",
    ] {
        assert!(result
            .dependencies
            .contains(&a.0.join(file).canonicalize().unwrap()));
        assert!(result
            .rust
            .contains(&format!("https://qml.invalid/modules/project/{file}")));
    }
    assert!(!result
        .dependencies
        .contains(&a.0.join("plugins/unlisted.mjs").canonicalize().unwrap()));
    assert!(!result.rust.contains("unlisted file must not be included"));
    let error = Compiler::new()
        .javascript_entry(&b.0)
        .load(a.0.join("Main.qml"))
        .err()
        .unwrap();
    assert!(error.to_string().contains("relative"), "{error}");
    let outside = b.0.file_name().unwrap();
    let error = Compiler::new()
        .javascript_entry(Path::new("..").join(outside).join("plugins/pl/index.mjs"))
        .load(a.0.join("Main.qml"))
        .err()
        .unwrap();
    assert!(error.to_string().contains("escapes"), "{error}");
}

#[test]
fn worker_sources_are_linked_but_execute_in_a_separate_realm() {
    let project = Project::new();
    project.file(
        "Main.qml",
        r#"import QtQml
import QtQml.WorkerScript
QtObject { property WorkerScript task: WorkerScript { source: "actor.mjs" } }"#,
    );
    project.file("actor.mjs", "import {n} from './number.mjs'; WorkerScript.onMessage = m => WorkerScript.sendMessage(n + m);");
    project.file("number.mjs", "export const n = 2;");
    let loaded = Compiler::new()
        .with_javascript()
        .load(project.0.join("Main.qml"))
        .unwrap();
    assert_eq!(loaded.document.scripts.len(), 2);
    assert!(loaded.document.scripts.values().all(|s| !s.preload));
    assert!(loaded
        .dependencies
        .contains(&project.0.join("number.mjs").canonicalize().unwrap()));
}
