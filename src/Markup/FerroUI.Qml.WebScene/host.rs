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
            Value::External(v) => json!(["js", self.handle(v)?]),
            Value::List(items) => json!([
                "a",
                items
                    .iter()
                    .map(|v| self.encode(instance, v))
                    .collect::<Result<Vec<_>, _>>()?
            ]),
            v => {
                let id = self.tables.borrow_mut().host_id(instance, v);
                json!([
                    "h",
                    id,
                    if matches!(v,Value::Record(r) if r.is_array()) {
                        "array"
                    } else {
                        "object"
                    }
                ])
            }
        })
    }
    fn lookup(&self, ctx: &ScriptContext<'_, '_>, id: u64) -> Result<Value, Error> {
        self.tables
            .borrow()
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
        let id = self
            .tables
            .borrow_mut()
            .host_id(ctx.instance(), &Value::Object(object));
        let mut signature = ctx
            .argument_types(object, key)
            .into_iter()
            .map(mode)
            .collect::<Vec<_>>();
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
    pub(super) fn host(&self, ctx: &mut ScriptContext<'_, '_>, v: &[Json]) -> Result<Json, Error> {
        ctx.consume_work()?;
        let op = text(v, 0)?;
        let id = number(v, 1)?;
        let instance = ctx.instance();
        if matches!(op, "resolve" | "assign" | "scopeMode") {
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
            "set" => {
                let input = self.decode(ctx, field(v, 3)?)?;
                ctx.write_member(&value, text(v, 2)?, input)?;
                Ok(Json::Null)
            }
            "mode" => Ok(json!(if let Value::Object(obj) = value {
                mode(ctx.property_type(obj, text(v, 2)?))
            } else {
                None
            })),
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
            "keys" => Ok(json!(ctx.keys(&value)?)),
            _ => Err(Error::new(format!("unknown V8 host operation '{op}'"))),
        }
    }
}
