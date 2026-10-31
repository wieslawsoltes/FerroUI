//! WebScene.NativeEngine ABI 3 backend for QML. One bounded worker owns the V8
//! realm. QML reads, writes, calls and notifications execute on the original
//! FerroUI transaction thread, including nested evaluations on the same stack.
mod framing;
mod host;
mod modules;
mod process;
mod promise;
use ferroui_qml::{
    ir, Error, ExternalValue, ObjectId, ScriptContext, ScriptEngine, ScriptInstance, ScriptScope,
    Value,
};
use serde_json::{json, Value as Json};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    path::{Path, PathBuf},
    rc::{Rc, Weak},
    time::{Duration, Instant},
};

#[derive(Default)]
struct Tables {
    next: u64,
    scopes: BTreeMap<(ScriptScope, String, BTreeMap<String, String>), u64>,
    scope_values: BTreeMap<u64, (ScriptInstance, ScriptScope)>,
    scope_names: BTreeMap<u64, Rc<Vec<String>>>,
    objects: BTreeMap<ObjectId, u64>,
    other: BTreeMap<ScriptInstance, Vec<(Value, u64)>>,
    values: BTreeMap<u64, (ScriptInstance, Value)>,
    modules: Option<Rc<modules::Modules>>,
    prepared: bool,
}
impl Tables {
    fn allocate(&mut self) -> u64 {
        self.next += 1;
        self.next
    }
    fn scope(
        &mut self,
        ctx: &mut ScriptContext<'_, '_>,
        lexical: ScriptScope,
        script: &ir::JavaScript,
    ) -> Result<(u64, Rc<Vec<String>>), Error> {
        let key = (
            lexical.clone(),
            script.origin.clone(),
            script.imports.clone(),
        );
        if let Some(id) = self.scopes.get(&key) {
            return Ok((*id, self.scope_names[id].clone()));
        }
        for (alias, target) in &script.imports {
            if !ctx.resources().contains_key(target) {
                return Err(Error::new(format!(
                    "missing embedded script for import '{alias}'"
                )));
            }
        }
        // Exclude arguments belonging to the activating function. The V8
        // function owns those variables, while this lexical scope is reusable.
        let names = Rc::new(ctx.in_scope(&lexical, |scope| Ok(scope.names()))?);
        let id = self.allocate();
        self.scope_values.insert(id, (ctx.instance(), lexical));
        self.scope_names.insert(id, names.clone());
        self.scopes.insert(key, id);
        Ok((id, names))
    }
    fn host_id(&mut self, instance: ScriptInstance, value: &Value) -> u64 {
        if let Value::Object(object) = value {
            if let Some(id) = self.objects.get(object) {
                return *id;
            }
            let id = self.allocate();
            self.objects.insert(*object, id);
            self.values.insert(id, (instance, value.clone()));
            id
        } else {
            if let Some((_, id)) = self
                .other
                .get(&instance)
                .and_then(|all| all.iter().find(|(old, _)| old == value))
            {
                return *id;
            }
            let id = self.allocate();
            self.other
                .entry(instance)
                .or_default()
                .push((value.clone(), id));
            self.values.insert(id, (instance, value.clone()));
            id
        }
    }
}

/// Explicit worker and native library paths make application packaging independent
/// of the current directory, platform DLL search paths and build-time network access.
pub struct WebSceneEngine {
    executable: PathBuf,
    library: PathBuf,
    timeout: Duration,
    realms: RefCell<BTreeMap<ScriptInstance, Rc<Realm>>>,
}

// Worker-global V8 state (including ESM bindings, host proxies, pending jobs
// and native code caches) belongs to one QML instance.
struct Realm {
    executable: PathBuf,
    library: PathBuf,
    timeout: Duration,
    worker: RefCell<Option<process::Worker>>,
    deadline: Cell<Option<Instant>>,
    tables: RefCell<Tables>,
    generation: Cell<u64>,
    handles: RefCell<BTreeMap<u64, Weak<GuestHandle>>>,
    pending: Rc<RefCell<Vec<(u64, u64)>>>,
    identity: Rc<()>,
}
struct GuestHandle {
    engine: Weak<()>,
    epoch: u64,
    id: u64,
    pending: Weak<RefCell<Vec<(u64, u64)>>>,
}
impl Drop for GuestHandle {
    fn drop(&mut self) {
        if let Some(q) = self.pending.upgrade() {
            q.borrow_mut().push((self.epoch, self.id));
        }
    }
}
impl WebSceneEngine {
    pub fn new(worker: impl AsRef<Path>, library: impl AsRef<Path>) -> Result<Self, Error> {
        let file = |path: &Path| {
            path.canonicalize()
                .and_then(|p| {
                    if p.is_file() {
                        Ok(p)
                    } else {
                        Err(std::io::Error::other("not a regular file"))
                    }
                })
                .map_err(|e| Error::new(format!("{}: {e}", path.display())))
        };
        Ok(Self {
            executable: file(worker.as_ref())?,
            library: file(library.as_ref())?,
            timeout: Duration::from_secs(10),
            realms: RefCell::default(),
        })
    }
    /// Read FERROUI_WEBSCENE_LIBRARY and optionally FERROUI_WEBSCENE_WORKER.
    /// By default the worker is located beside the application executable.
    pub fn from_env() -> Result<Self, Error> {
        let library = std::env::var_os("FERROUI_WEBSCENE_LIBRARY").ok_or_else(|| {
            Error::new("set FERROUI_WEBSCENE_LIBRARY to the WebScene native library")
        })?;
        let worker = match std::env::var_os("FERROUI_WEBSCENE_WORKER") {
            Some(file) => file.into(),
            None => std::env::current_exe()
                .map_err(|e| Error::new(e.to_string()))?
                .with_file_name(format!(
                    "ferroui-qml-webscene-worker{}",
                    std::env::consts::EXE_SUFFIX
                )),
        };
        Self::new(worker, library)
    }
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    fn realm(&self, id: ScriptInstance) -> Rc<Realm> {
        self.realms
            .borrow_mut()
            .entry(id)
            .or_insert_with(|| {
                Rc::new(Realm {
                    executable: self.executable.clone(),
                    library: self.library.clone(),
                    timeout: self.timeout,
                    worker: Default::default(),
                    deadline: Cell::new(None),
                    tables: Default::default(),
                    generation: Cell::new(1),
                    handles: Default::default(),
                    pending: Default::default(),
                    identity: Rc::new(()),
                })
            })
            .clone()
    }
}

impl ScriptEngine for WebSceneEngine {
    fn evaluate(
        &self,
        ctx: &mut ScriptContext<'_, '_>,
        script: &ir::JavaScript,
    ) -> Result<Value, Error> {
        self.realm(ctx.instance()).evaluate(ctx, script)
    }
    fn get_external(
        &self,
        ctx: &mut ScriptContext<'_, '_>,
        v: &ExternalValue,
        name: &str,
    ) -> Result<Value, Error> {
        self.realm(ctx.instance()).get_external(ctx, v, name)
    }
    fn set_external(
        &self,
        ctx: &mut ScriptContext<'_, '_>,
        v: &ExternalValue,
        name: &str,
        input: Value,
    ) -> Result<(), Error> {
        self.realm(ctx.instance()).set_external(ctx, v, name, input)
    }
    fn call_external(
        &self,
        ctx: &mut ScriptContext<'_, '_>,
        v: &ExternalValue,
        receiver: Value,
        args: Vec<Value>,
    ) -> Result<Value, Error> {
        self.realm(ctx.instance())
            .call_external(ctx, v, receiver, args, None)
    }
    fn evaluate_binding(
        &self,
        ctx: &mut ScriptContext<'_, '_>,
        callback: &ExternalValue,
        receiver: ObjectId,
        ty: &str,
    ) -> Result<Value, Error> {
        self.realm(ctx.instance()).call_external(
            ctx,
            callback,
            Value::Object(receiver),
            Vec::new(),
            host::mode(Some(ty)),
        )
    }
    fn observe_promise(
        &self,
        ctx: &mut ScriptContext<'_, '_>,
        value: &ExternalValue,
        timeout: Duration,
    ) -> Result<ferroui_qml::PromiseState, Error> {
        self.realm(ctx.instance()).observe(ctx, value, timeout)
    }
    fn is_reference_live(&self, value: &ExternalValue) -> bool {
        self.realms
            .borrow()
            .get(&value.instance())
            .is_some_and(|realm| realm.handle(value).is_ok())
    }
    fn release_instance(&self, id: ScriptInstance) {
        self.realms.borrow_mut().remove(&id);
    }
}
impl Realm {
    fn poison(&self, err: Error) -> Error {
        self.worker.borrow_mut().take();
        self.generation.set(self.generation.get() + 1);
        self.tables.borrow_mut().prepared = false;
        self.handles.borrow_mut().clear();
        err
    }
    fn read(&self, deadline: Instant) -> Result<Json, Error> {
        let result = self
            .worker
            .borrow()
            .as_ref()
            .ok_or_else(|| Error::new("WebScene worker unavailable"))?
            .read(deadline);
        result.map_err(|e| self.poison(e))
    }
    fn send(&self, value: Json, deadline: Instant) -> Result<(), Error> {
        let result = self
            .worker
            .borrow_mut()
            .as_mut()
            .ok_or_else(|| Error::new("WebScene worker unavailable"))?
            .send(value, deadline);
        result.map_err(|e| self.poison(e))
    }
    fn ensure_worker(&self, deadline: Instant) -> Result<(), Error> {
        if self.worker.borrow().is_some() {
            return Ok(());
        }
        *self.worker.borrow_mut() = Some(process::Worker::spawn(&self.executable, &self.library)?);
        let hello = self.read(deadline)?;
        if hello["ready"] != true {
            return Err(self.poison(Error::new(
                hello["failure"]
                    .as_str()
                    .unwrap_or("invalid WebScene worker handshake"),
            )));
        }
        Ok(())
    }

    fn invoke(&self, ctx: &mut ScriptContext<'_, '_>, mut command: Json) -> Result<Value, Error> {
        let outer = self.deadline.get();
        let deadline = outer.unwrap_or_else(|| {
            Instant::now()
                .checked_add(self.timeout)
                .unwrap_or_else(Instant::now)
        });
        self.ensure_worker(deadline)?;
        if outer.is_none() {
            self.deadline.set(Some(deadline));
            let released = self
                .pending
                .take()
                .into_iter()
                .filter_map(|(epoch, id)| {
                    (epoch == self.generation.get()
                        && self
                            .handles
                            .borrow()
                            .get(&id)
                            .and_then(Weak::upgrade)
                            .is_none())
                    .then_some(id)
                })
                .collect::<Vec<_>>();
            for id in &released {
                self.handles.borrow_mut().remove(id);
            }
            command["release"] = json!(released);
        }
        let result = (|| {
            if outer.is_some() {
                self.send(json!({"nested":command}), deadline)?;
            } else {
                self.send(command, deadline)?;
            }
            loop {
                let msg = self.read(deadline)?;
                if let Some(error) = msg.get("failure") {
                    return Err(Error::new(format!("WebScene: {error}")));
                }
                if let Some(value) = msg.get("result") {
                    return self.result(ctx, value);
                }
                let host = msg
                    .get("host")
                    .and_then(Json::as_array)
                    .ok_or_else(|| self.poison(Error::new("invalid WebScene protocol frame")))?;
                if host.first().and_then(Json::as_str) == Some("returned") {
                    if outer.is_none() {
                        return Err(self.poison(Error::new("unexpected reentrant completion")));
                    }
                    return self.result(
                        ctx,
                        host.get(1)
                            .ok_or_else(|| Error::new("missing nested result"))?,
                    );
                }
                let answer = match self.host(ctx, host) {
                    Ok(value) => json!({"ok":value}),
                    Err(e) => json!({"error":e.message}),
                };
                self.send(answer, deadline)?;
            }
        })();
        if outer.is_none() {
            self.deadline.set(None)
        }
        result
    }
    fn result(&self, ctx: &mut ScriptContext<'_, '_>, result: &Json) -> Result<Value, Error> {
        if let Some(e) = result.get("error") {
            return Err(Error::new(e.as_str().unwrap_or("V8 exception")));
        }
        self.decode(
            ctx,
            result
                .get("ok")
                .ok_or_else(|| Error::new("malformed WebScene evaluation result"))?,
        )
    }
    fn handle(&self, external: &ExternalValue) -> Result<u64, Error> {
        let h = external
            .downcast::<GuestHandle>()
            .ok_or_else(|| Error::new("foreign JavaScript value"))?;
        if h.epoch != self.generation.get() || !h.engine.ptr_eq(&Rc::downgrade(&self.identity)) {
            return Err(Error::new("stale JavaScript reference after worker reset"));
        }
        Ok(h.id)
    }
}
impl Realm {
    fn evaluate(
        &self,
        ctx: &mut ScriptContext<'_, '_>,
        script: &ir::JavaScript,
    ) -> Result<Value, Error> {
        let instance = ctx.instance();
        let (root, scope, names, component_scope, component_names, catalog) = {
            let mut tables = self.tables.borrow_mut();
            let root = tables.host_id(instance, &Value::Object(ctx.root()));
            let (scope, names) = tables.scope(ctx, ctx.capture_scope(), script)?;
            let (component_scope, component_names) =
                tables.scope(ctx, ctx.capture_component_scope(), script)?;
            // Every component in this instance uses the same linked resource catalog.
            // Only its aliases and lexical owner vary. Nested evaluations run after
            // the worker has installed the outer call's catalog.
            let catalog = if !tables.prepared && self.deadline.get().is_none() {
                if tables.modules.is_none() {
                    tables.modules = Some(Rc::new(modules::Modules::prepare(ctx.resources())?));
                }
                tables.modules.clone()
            } else {
                None
            };
            (
                root,
                scope,
                names,
                component_scope,
                component_names,
                catalog,
            )
        };
        let mut args = serde_json::Map::new();
        for (name, value) in ctx.arguments() {
            args.insert(name.clone(), self.encode(instance, value)?);
        }
        let form = match script.form {
            ir::ScriptForm::Expression => "expr",
            ir::ScriptForm::BindingBlock => "block",
            _ => "body",
        };
        let mut command = json!({
            "op": "eval",
            "scope": scope,
            "componentScope": component_scope,
            "componentNames": &*component_names,
            "graph": {"names": script.imports, "context": [&script.origin, &root.to_string()]},
            "source": script.code(),
            "hook": script.import_hook,
            "form": form,
            "owner": self.encode(instance, &Value::Object(ctx.owner()))?,
            "names": &*names,
            "args": args,
            "parameters": ctx.parameter_names(),
            "origin": script.origin,
            "mode": host::mode(ctx.expected_type()),
        });
        if let Some(catalog) = catalog.as_ref().filter(|modules| !modules.files.is_empty()) {
            command["prepare"] = json!({"resources": catalog.manifest, "names": script.imports});
            command["files"] = json!(catalog.files);
        }
        let result = self.invoke(ctx, command);
        if result.is_ok() && catalog.is_some() {
            self.tables.borrow_mut().prepared = true;
        }
        result
    }

    fn get_external(
        &self,
        ctx: &mut ScriptContext<'_, '_>,
        value: &ExternalValue,
        name: &str,
    ) -> Result<Value, Error> {
        self.invoke(
            ctx,
            json!({"op":"get","target":["js",self.handle(value)?],"name":name}),
        )
    }
    fn set_external(
        &self,
        ctx: &mut ScriptContext<'_, '_>,
        value: &ExternalValue,
        name: &str,
        input: Value,
    ) -> Result<(), Error> {
        let input = self.encode(ctx.instance(), &input)?;
        self.invoke(
            ctx,
            json!({"op":"set","target":["js",self.handle(value)?],"name":name,"value":input}),
        )
        .map(drop)
    }
    fn call_external(
        &self,
        ctx: &mut ScriptContext<'_, '_>,
        value: &ExternalValue,
        receiver: Value,
        args: Vec<Value>,
        mode: Option<&str>,
    ) -> Result<Value, Error> {
        let args = args
            .iter()
            .map(|v| self.encode(ctx.instance(), v))
            .collect::<Result<Vec<_>, _>>()?;
        let receiver = self.encode(ctx.instance(), &receiver)?;
        self.invoke(ctx,json!({"op":"call","target":["js",self.handle(value)?],"receiver":receiver,"args":args,"mode":mode}))
    }
}
