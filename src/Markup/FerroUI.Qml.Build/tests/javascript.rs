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
    assert!(first.source.contains(&format!("import(\"{lazy}\")")));
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
    assert!(later.source.contains(optional));
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
    assert!(compile(&a.0)
        .err()
        .unwrap()
        .to_string()
        .contains("string literal"));
}
