//! Value and scope projection. Guest code receives opaque ids, never pointers.
use super::*;

pub(super) fn mode(ty: Option<&str>) -> Option<&'static str> {
    ty.filter(|t| t.starts_with("list<")).map(|_| "array")
}
fn field(v: &[Json], i: usize) -> Result<&Json, Error> {
    v.get(i)
        .ok_or_else(|| Error::new("truncated V8 host request"))
}
fn text(v: &[Json], i: usize) -> Result<&str, Error> {
    field(v, i)?
        .as_str()
        .ok_or_else(|| Error::new("expected V8 string"))
}
fn number(v: &[Json], i: usize) -> Result<u64, Error> {
    field(v, i)?
        .as_u64()
        .ok_or_else(|| Error::new("expected V8 handle"))
}

impl Realm {
    pub(super) fn encode(&self, instance: ScriptInstance, value: &Value) -> Result<Json, Error> {
        Ok(match value {
            Value::Null => json!(["n"]),
            Value::Undefined => json!(["u"]),
            Value::Bool(b) => json!(["b", b]),
            Value::String(s) => json!(["s", s]),
            Value::Number(n) => json!([
                "d",
                if *n == f64::INFINITY {
                    "Infinity".into()
                } else if *n == f64::NEG_INFINITY {
                    "-Infinity".into()
                } else {
                    n.to_string()
                }
            ]),
            Value::Geometry(shape) => json!([
                "geometry",
                shape.type_name(),
                shape
                    .components()
                    .into_iter()
                    .map(|n| self.encode(instance, &Value::Number(n)))
                    .collect::<Result<Vec<_>, _>>()?
            ]),
            Value::Method(method) => json!([
                "bound",
                self.tables
                    .borrow_mut()
                    .object_id(instance, method.receiver()),
                method.name()
            ]),
            Value::External(v) => json!(["js", self.handle(v)?]),
            Value::List(items) => json!([
                "a",
                items
                    .iter()
                    .map(|v| self.encode(instance, v))
                    .collect::<Result<Vec<_>, _>>()?
            ]),
            Value::Object(object) => json!([
                "h",
                self.tables.borrow_mut().object_id(instance, *object),
                "object"
            ]),
            v => {
                let mut tables = self.tables.borrow_mut();
                let Tables { dynamic, next, .. } = &mut *tables;
                let id = dynamic.project(v, next)?;
                json!([
                    "h",
                    id,
                    if matches!(v,Value::Record(r) if r.is_array()) {
                        "array"
                    } else if matches!(v, Value::Record(r) if r.is_geometry()) {
                        "geometry"
                    } else if matches!(v, Value::Record(r) if r.is_incubator()) {
                        "incubator"
                    } else {
                        "leased"
                    }
                ])
            }
        })
    }
    fn lookup(&self, ctx: &ScriptContext<'_, '_>, id: u64) -> Result<Value, Error> {
        let tables = self.tables.borrow();
        if let Some(value) = tables.dynamic.get(id) {
            return Ok(value);
        }
        tables
            .values
            .get(&id)
            .filter(|(instance, _)| *instance == ctx.instance())
            .map(|(_, v)| v.clone())
            .ok_or_else(|| Error::new("expired or foreign QML host reference"))
    }
    pub(super) fn decode(
        &self,
        ctx: &mut ScriptContext<'_, '_>,
        value: &Json,
    ) -> Result<Value, Error> {
        let v = value
            .as_array()
            .ok_or_else(|| Error::new("expected tagged WebScene value"))?;
        Ok(match text(v, 0)? {
            "u" => Value::Undefined,
            "n" => Value::Null,
            "b" => Value::Bool(
                field(v, 1)?
                    .as_bool()
                    .ok_or_else(|| Error::new("invalid WebScene boolean"))?,
            ),
            "d" => Value::Number(
                text(v, 1)?
                    .parse()
                    .map_err(|_| Error::new("invalid WebScene number"))?,
            ),
            "s" => Value::String(text(v, 1)?.into()),
            "geometry" => {
                let components = field(v, 2)?
                    .as_array()
                    .ok_or_else(|| Error::new("invalid geometry components"))?;
                if components.len() > 4 {
                    return Err(Error::new("invalid geometry arity"));
                }
                let mut numbers = Vec::with_capacity(components.len());
                for component in components {
                    let Value::Number(n) = self.decode(ctx, component)? else {
                        return Err(Error::new("expected numeric geometry component"));
                    };
                    numbers.push(n);
                }
                Value::Geometry(ferroui_qml::Geometry::from_components(
                    text(v, 1)?,
                    &numbers,
                )?)
            }
            "h" => self.lookup(ctx, number(v, 1)?)?,
            "a" => Value::List(
                field(v, 1)?
                    .as_array()
                    .ok_or_else(|| Error::new("invalid V8 array"))?
                    .iter()
                    .map(|v| self.decode(ctx, v))
                    .collect::<Result<Vec<_>, _>>()?,
            ),
            "o" => {
                let mut fields = BTreeMap::new();
                for pair in field(v, 1)?
                    .as_array()
                    .ok_or_else(|| Error::new("invalid V8 data object"))?
                {
                    let pair = pair
                        .as_array()
                        .ok_or_else(|| Error::new("invalid V8 data entry"))?;
                    fields.insert(
                        text(pair, 0)?.to_owned(),
                        self.decode(ctx, field(pair, 1)?)?,
                    );
                }
                ctx.record(fields)?
            }
            "bound" => {
                let Value::Object(owner) = self.lookup(ctx, number(v, 1)?)? else {
                    return Err(Error::new("invalid bound method receiver"));
                };
                Value::Method(ctx.bound_method(owner, text(v, 2)?)?)
            }
            "js" => {
                let id = number(v, 1)?;
                let mut handles = self.handles.borrow_mut();
                let existing = handles.get(&id).and_then(Weak::upgrade);
                let handle = existing.unwrap_or_else(|| {
                    let handle = Rc::new(GuestHandle {
                        engine: Rc::downgrade(&self.identity),
                        epoch: self.generation.get(),
                        id,
                        pending: Rc::downgrade(&self.pending),
                    });
                    handles.insert(id, Rc::downgrade(&handle));
                    handle
                });
                // Include the generation in the runtime identity after a restart.
                let token = self
                    .generation
                    .get()
                    .checked_mul(1u64 << 32)
                    .and_then(|base| {
                        (id <= u32::MAX as u64)
                            .then(|| base.checked_add(id))
                            .flatten()
                    })
                    .ok_or_else(|| Error::new("JavaScript handle space exhausted"))?;
                ctx.external(token, handle)
            }
            tag => return Err(Error::new(format!("unknown WebScene value tag {tag}"))),
        })
    }
    fn method(&self, ctx: &ScriptContext<'_, '_>, object: ObjectId, key: &str) -> Json {
        let id = self.tables.borrow_mut().object_id(ctx.instance(), object);
        let mut signature = ctx
            .argument_types(object, key)
            .into_iter()
            .map(mode)
            .collect::<Vec<_>>();
        if ctx.is_loader(object) && key == "setSource" {
            signature = vec![None, Some("loaderInitial")];
        }
        if ctx.is_component(object) {
            match key {
                "createObject" => signature = vec![None, Some("initial")],
                "incubateObject" => signature = vec![None, Some("incubatorInitial"), None],
                _ => {}
            }
        }
        if ctx.is_worker_script(object) && key == "sendMessage" {
            signature = vec![Some("data")];
        }
        if ctx.is_list_model(object) {
            let row = match key {
                "append" => Some(0),
                "insert" | "set" => Some(1),
                "setProperty" => Some(2),
                _ => None,
            };
            if let Some(index) = row {
                signature.resize(index + 1, None);
                signature[index] = Some("data");
            }
        }
        json!(["fn", id, key, signature, ctx.is_signal(object, key)])
    }
    fn install_binding(
        &self,
        ctx: &mut ScriptContext<'_, '_>,
        object: Option<ObjectId>,
        name: &str,
        input: &Json,
    ) -> Result<Json, Error> {
        let Value::External(callback) = self.decode(ctx, input)? else {
            return Err(Error::new("expected a guest binding function"));
        };
        ctx.install_binding(object, name, callback)?;
        Ok(Json::Null)
    }
    pub(super) fn host(&self, ctx: &mut ScriptContext<'_, '_>, v: &[Json]) -> Result<Json, Error> {
        ctx.consume_work()?;
        let op = text(v, 0)?;
        let id = number(v, 1)?;
        let instance = ctx.instance();
        if op == "callLater" {
            let owner = if id == 0 {
                None
            } else {
                let Value::Object(owner) = self.lookup(ctx, id)? else {
                    return Err(Error::new("invalid owner of deferred method"));
                };
                Some(owner)
            };
            let encoded = field(v, 3)?
                .as_array()
                .ok_or_else(|| Error::new("invalid deferred arguments"))?;
            let arguments = encoded
                .iter()
                .map(|value| self.decode(ctx, value))
                .collect::<Result<_, _>>()?;
            if let (Some(owner), Some(name)) = (owner, field(v, 2)?.as_str()) {
                let method = ctx.bound_method(owner, name)?;
                ctx.defer_method(method, arguments)?;
            } else {
                let Value::External(function) = self.decode(ctx, field(v, 2)?)? else {
                    return Err(Error::new("Qt.callLater expects a guest function"));
                };
                ctx.call_later(owner, function, arguments)?;
            }
            return Ok(Json::Null);
        }
        if matches!(
            op,
            "createComponent" | "createQmlObject" | "url" | "resolvedUrl"
        ) {
            let args = field(v, 2)?
                .as_array()
                .ok_or_else(|| Error::new("invalid Qt function arguments"))?
                .iter()
                .map(|arg| self.decode(ctx, arg))
                .collect::<Result<Vec<_>, _>>()?;
            let call = |ctx: &mut ScriptContext<'_, '_>, origin: Option<&str>| {
                if op == "createComponent" {
                    ctx.component_from_arguments(origin, &args)
                } else {
                    ctx.url_from_arguments(op, origin, &args)
                }
            };
            let origin = v.get(3).and_then(Json::as_str);
            let value = if op == "createQmlObject" {
                ctx.qml_from_arguments(&args)?
            } else if id == 0 {
                let origin = if op == "createComponent" {
                    origin.or(Some("https://qml.invalid/components/project/"))
                } else {
                    origin
                };
                call(ctx, origin)?
            } else {
                let scope = self
                    .tables
                    .borrow()
                    .scope_values
                    .get(&id)
                    .filter(|(owner, _)| *owner == instance)
                    .map(|(_, scope)| scope.clone())
                    .ok_or_else(|| Error::new("expired component scope"))?;
                ctx.in_scope(&scope, |ctx| call(ctx, origin))?
            };
            return self.encode(instance, &value);
        }
        if matches!(op, "loaderSource" | "construct" | "incubateObject") {
            let Value::Object(factory) = self.lookup(ctx, id)? else {
                return Err(Error::new("invalid Component"));
            };
            let target = self.decode(ctx, field(v, 2)?)?;
            let mut properties = BTreeMap::new();
            for p in field(v, 3)?
                .as_array()
                .ok_or_else(|| Error::new("invalid initial properties"))?
            {
                let pair = p
                    .as_array()
                    .ok_or_else(|| Error::new("invalid initial property"))?;
                let value = self.decode(ctx, field(pair, 2)?)?;
                let entry = match field(pair, 1)?.as_bool() {
                    Some(true) => {
                        let Value::External(callback) = value else {
                            return Err(Error::new("expected binding closure"));
                        };
                        ferroui_qml::InitialProperty::Binding(callback)
                    }
                    Some(false) => ferroui_qml::InitialProperty::Value(value),
                    _ => return Err(Error::new("invalid binding flag")),
                };
                properties.insert(text(pair, 0)?.into(), entry);
            }
            if op == "loaderSource" {
                let Value::String(url) = target else {
                    return Err(Error::new("Loader.setSource expects a URL"));
                };
                ctx.set_loader_source(factory, url, properties)?;
                return self.encode(instance, &Value::Undefined);
            }
            let parent = match target {
                Value::Object(id) => Some(id),
                Value::Null | Value::Undefined => None,
                _ => return Err(Error::new("invalid parent")),
            };
            let value = if op == "incubateObject" {
                let mode = self.decode(ctx, field(v, 4)?)?;
                let mode = if mode == Value::Number(1.) {
                    ferroui_qml::IncubationMode::Synchronous
                } else {
                    ferroui_qml::IncubationMode::Asynchronous
                };
                ctx.incubate_with_properties(factory, parent, properties, mode)?
            } else {
                ctx.create_with_properties(factory, parent, properties)?
            };
            return self.encode(instance, &value);
        }
        if matches!(op, "resolve" | "assign" | "scopeMode" | "bindScope") {
            let scope = self
                .tables
                .borrow()
                .scope_values
                .get(&id)
                .filter(|(owner, _)| *owner == instance)
                .map(|(_, s)| s.clone())
                .ok_or_else(|| Error::new("foreign JavaScript lexical scope"))?;
            let name = text(v, 2)?;
            return ctx.in_scope(&scope, |ctx| match op {
                "assign" => {
                    let value = self.decode(ctx, field(v, 3)?)?;
                    ctx.assign(name, value)?;
                    Ok(Json::Null)
                }
                "bindScope" => self.install_binding(ctx, None, name, field(v, 3)?),
                "scopeMode" => Ok(json!(mode(ctx.name_type(name)))),
                _ => {
                    if let Some(value) = ctx.resolve(name)? {
                        self.encode(instance, &value)
                    } else if let Some(object) = ctx.method_owner(name) {
                        Ok(self.method(ctx, object, name))
                    } else {
                        Ok(json!(["u"]))
                    }
                }
            });
        }
        let value = self.lookup(ctx, id)?;
        match op {
            "forceIncubation" => {
                let Value::Record(handle) = value else {
                    return Err(Error::new("invalid incubator handle"));
                };
                ctx.force_incubation(&handle)?;
                Ok(Json::Null)
            }
            "get" => {
                let name = text(v, 2)?;
                if let Value::Object(object) = value {
                    if ctx.is_method(object, name) {
                        return Ok(self.method(ctx, object, name));
                    }
                }
                let result = ctx.read_member(&value, name)?;
                self.encode(instance, &result)
            }
            "bind" => {
                let Value::Object(object) = value else {
                    return Err(Error::new(
                        "Qt.binding requires a writable QML object property",
                    ));
                };
                self.install_binding(ctx, Some(object), text(v, 2)?, field(v, 3)?)
            }
            "set" => {
                let input = self.decode(ctx, field(v, 3)?)?;
                ctx.write_member(&value, text(v, 2)?, input)?;
                Ok(Json::Null)
            }
            "mode" => Ok(json!(
                if matches!(value, Value::ModelRow(_) | Value::DelegateRow(_)) {
                    Some("data")
                } else if let Value::Object(obj) = value {
                    mode(ctx.property_type(obj, text(v, 2)?))
                } else {
                    None
                }
            )),
            "call" => {
                let Value::Object(object) = value else {
                    return Err(Error::new("method receiver is not a QML object"));
                };
                let args = field(v, 3)?
                    .as_array()
                    .ok_or_else(|| Error::new("expected method arguments"))?
                    .iter()
                    .map(|arg| self.decode(ctx, arg))
                    .collect::<Result<Vec<_>, _>>()?;
                let result = ctx.call(object, text(v, 2)?, args)?;
                self.encode(instance, &result)
            }
            "connect" | "disconnect" => {
                let Value::Object(sender) = value else {
                    return Err(Error::new("signal receiver is not a QML object"));
                };
                let signal = text(v, 2)?;
                let receiver = self.decode(ctx, field(v, 3)?)?;
                let Value::External(function) = self.decode(ctx, field(v, 4)?)? else {
                    return Err(Error::new("expected a guest function reference"));
                };
                if op == "connect" {
                    ctx.connect_signal(sender, signal, receiver, function)?;
                } else {
                    ctx.disconnect_signal(sender, signal, receiver, function)?;
                }
                Ok(Json::Null)
            }
            "geometryString" => Ok(json!(ctx.geometry_string(&value)?)),
            "keys" => Ok(json!(ctx.keys(&value)?)),
            _ => Err(Error::new(format!("unknown V8 host operation '{op}'"))),
        }
    }
}
