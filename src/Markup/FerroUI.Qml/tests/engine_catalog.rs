use ferroui_qml::{ir, Engine, Error, ScriptContext, ScriptEngine, TypeRegistry, Value};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

#[derive(Default)]
struct CatalogProbe {
    installs: RefCell<Vec<BTreeMap<String, bool>>>,
    fail_next: Cell<usize>,
}
impl ScriptEngine for CatalogProbe {
    fn evaluate(&self, _: &mut ScriptContext<'_, '_>, _: &ir::JavaScript) -> Result<Value, Error> {
        unreachable!("only native expressions in this fixture")
    }
    fn prepare_script_resources(&self, ctx: &mut ScriptContext<'_, '_>) -> Result<(), Error> {
        self.installs.borrow_mut().push(
            ctx.resources()
                .iter()
                .map(|(url, script)| (url.clone(), script.preload))
                .collect(),
        );
        if self.fail_next.get() > 0 {
            self.fail_next.set(self.fail_next.get() - 1);
            return Err(Error::new("injected catalog failure"));
        }
        Ok(())
    }
}

#[test]
fn shared_roots_install_only_new_or_promoted_resources_and_restore_failed_extensions() {
    let guest = Rc::new(CatalogProbe::default());
    let mut engine = Engine::with_script_engine(&TypeRegistry::new(), guest.clone()).unwrap();
    let mut doc = ir::parse("import QtQml\nQtObject {}").unwrap();
    let resource = ir::ScriptResource {
        executable: "var answer = 42;".into(),
        import_hook: None,
        preload: false,
        imports: Default::default(),
        source: "var answer = 42;".into(),
        kind: ir::ScriptKind::Script,
        dependencies: Default::default(),
        exports: vec!["answer".into()],
        writable_exports: vec![],
    };
    doc.scripts.insert("existing".into(), resource.clone());
    engine.load_root(&doc, Default::default()).unwrap();
    engine.load_root(&doc, Default::default()).unwrap();
    assert_eq!(guest.installs.borrow().len(), 1);

    let mut conflict = doc.clone();
    conflict.scripts.insert("a-new".into(), resource.clone());
    conflict
        .scripts
        .get_mut("existing")
        .unwrap()
        .exports
        .clear();
    assert!(engine.load_root(&conflict, Default::default()).is_err());
    assert_eq!(guest.installs.borrow().len(), 1);

    let mut failed =
        ir::parse("import QtQml\nQtObject { Component.onCompleted: missing() }").unwrap();
    failed.scripts = doc.scripts.clone();
    failed.scripts.get_mut("existing").unwrap().preload = true;
    failed.scripts.insert("new".into(), resource);
    assert!(engine.load_root(&failed, Default::default()).is_err());
    let previous = BTreeMap::from([("existing".into(), false)]);
    let extended = BTreeMap::from([("existing".into(), true), ("new".into(), false)]);
    assert_eq!(
        *guest.installs.borrow(),
        vec![previous.clone(), extended.clone(), previous]
    );
    doc.scripts = failed.scripts;
    engine.load_root(&doc, Default::default()).unwrap();
    doc.scripts.get_mut("existing").unwrap().preload = false;
    engine.load_root(&doc, Default::default()).unwrap();
    assert_eq!(guest.installs.borrow().len(), 4);
    assert_eq!(guest.installs.borrow().last(), Some(&extended));
    assert_eq!(engine.roots().count(), 4);
    // Both the failed update and the restoration can fail at a process
    // boundary. An unchanged root must then repair the guest before activation.
    let mut extension = doc.clone();
    extension
        .scripts
        .insert("another".into(), doc.scripts["new"].clone());
    guest.fail_next.set(2);
    assert!(engine.load_root(&extension, Default::default()).is_err());
    assert_eq!(guest.installs.borrow().len(), 6);
    engine.load_root(&doc, Default::default()).unwrap();
    engine.load_root(&doc, Default::default()).unwrap();
    assert_eq!(guest.installs.borrow().len(), 7);
}
