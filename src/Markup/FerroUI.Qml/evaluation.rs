use super::*;
use crate::transaction::Transaction;
use ferroui_qml_compiler::Expr;

pub(crate) enum Flow {
    Next(Value),
    Return(Value),
}
impl Flow {
    pub(crate) fn value(self) -> Value {
        match self {
            Self::Next(v) | Self::Return(v) => v,
        }
    }
}

impl Transaction<'_> {
    fn lookup_property(
        &self,
        owner: ObjectId,
        root: ObjectId,
        name: &str,
    ) -> Result<SlotId, Error> {
        self.model
            .key(owner, name)
            .or_else(|_| self.model.key(root, name))
            .or_else(|_| self.model.key(self.model.root(), name))
    }

    fn name(
        &mut self,
        owner: ObjectId,
        root: ObjectId,
        name: &str,
        locals: &Locals,
        reads: &mut BTreeSet<SlotId>,
    ) -> Result<Value, Error> {
        if let Some(value) = locals.get(name) {
            return Ok(value.clone());
        }
        match name {
            "this" => return Ok(Value::Object(owner)),
            "parent" => {
                return Ok(self
                    .model
                    .parent(owner)
                    .map(Value::Object)
                    .unwrap_or(Value::Null))
            }
            "NaN" => return Ok(Value::Number(f64::NAN)),
            "Infinity" => return Ok(Value::Number(f64::INFINITY)),
            _ => (),
        }
        if let Ok(key) = self.model.key(owner, name) {
            return self.read(key, reads);
        }
        if let Some(id) = self.model.ids.get(name) {
            return Ok(Value::Object(*id));
        }
        self.read(self.lookup_property(owner, root, name)?, reads)
    }

    fn scope_id(&self, name: &str) -> Result<ObjectId, Error> {
        self.model
            .ids
            .get(name)
            .copied()
            .ok_or_else(|| Error::new(format!("unknown component scope '{name}'")))
    }

    pub(crate) fn expr(
        &mut self,
        owner: ObjectId,
        expression: &Expr,
        locals: &mut Locals,
        reads: &mut BTreeSet<SlotId>,
    ) -> Result<Value, Error> {
        use Expr::*;
        Ok(match expression {
            Null => Value::Null,
            Undefined => Value::Undefined,
            Bool(b) => Value::Bool(*b),
            Number(n) => Value::Number(*n),
            String(s) => Value::String(s.clone()),
            Name(name) => self.name(owner, self.model.root(), name, locals, reads)?,
            ScopedName { name, root } => {
                self.name(owner, self.scope_id(root)?, name, locals, reads)?
            }
            Array(items) => Value::List(
                items
                    .iter()
                    .map(|e| self.expr(owner, e, locals, reads))
                    .collect::<Result<_, _>>()?,
            ),
            Member(base, name) => {
                if let Name(group) = base.as_ref() {
                    if let Ok(key) = self.model.key(owner, &format!("{group}.{name}")) {
                        return self.read(key, reads);
                    }
                    if group == "Math" {
                        let constant = match name.as_str() {
                            "PI" => std::f64::consts::PI,
                            "E" => std::f64::consts::E,
                            other => {
                                return Err(Error::new(format!("unknown Math constant '{other}'")))
                            }
                        };
                        return Ok(Value::Number(constant));
                    }
                }
                let base = self.expr(owner, base, locals, reads)?;
                self.member(base, name, reads)?
            }
            Index(base, index) => {
                let base = self.expr(owner, base, locals, reads)?;
                let index = self.expr(owner, index, locals, reads)?;
                self.member(base, &index.js_string(), reads)?
            }
            Unary(op, expr) => {
                let value = self.expr(owner, expr, locals, reads)?;
                match op.as_str() {
                    "!" => Value::Bool(!value.truthy()),
                    "-" => Value::Number(-value.number()?),
                    "+" => Value::Number(value.number()?),
                    "~" => Value::Number(f64::from(!int32(value.number()?))),
                    "typeof" => Value::String(
                        match value {
                            Value::Number(_) => "number",
                            Value::Bool(_) => "boolean",
                            Value::String(_) => "string",
                            Value::Undefined => "undefined",
                            _ => "object",
                        }
                        .into(),
                    ),
                    _ => return Err(Error::new(format!("unsupported unary operator '{op}'"))),
                }
            }
            Binary(op, left, right) => {
                let a = self.expr(owner, left, locals, reads)?;
                match op.as_str() {
                    "&&" if !a.truthy() => a,
                    "||" if a.truthy() => a,
                    "??" if !matches!(a, Value::Null | Value::Undefined) => a,
                    "&&" | "||" | "??" => self.expr(owner, right, locals, reads)?,
                    _ => binary(op, a, self.expr(owner, right, locals, reads)?)?,
                }
            }
            Conditional(test, yes, no) => {
                let test = self.expr(owner, test, locals, reads)?;
                self.expr(owner, if test.truthy() { yes } else { no }, locals, reads)?
            }
            Call(target, arguments) => {
                let (receiver, name) = match target.as_ref() {
                    Name(name) => (
                        Some(self.call_receiver(owner, self.model.root(), name)),
                        name,
                    ),
                    ScopedName { name, root } => (
                        Some(self.call_receiver(owner, self.scope_id(root)?, name)),
                        name,
                    ),
                    Member(base, name) if matches!(base.as_ref(), Name(n) if n == "Math") => {
                        (None, name)
                    }
                    Member(base, name) => {
                        (Some(object(self.expr(owner, base, locals, reads)?)?), name)
                    }
                    _ => return Err(Error::new("expected a named QML method")),
                };
                let arguments: Vec<_> = arguments
                    .iter()
                    .map(|e| self.expr(owner, e, locals, reads))
                    .collect::<Result<_, _>>()?;
                match receiver {
                    Some(receiver) => self.invoke(receiver, name, arguments, reads)?,
                    None => math(name, &arguments)?,
                }
            }
        })
    }

    fn member(
        &mut self,
        base: Value,
        name: &str,
        reads: &mut BTreeSet<SlotId>,
    ) -> Result<Value, Error> {
        match base {
            Value::Object(id) if name == "parent" => Ok(self
                .model
                .parent(id)
                .map(Value::Object)
                .unwrap_or(Value::Null)),
            Value::Object(id) => self.read(self.model.key(id, name)?, reads),
            Value::List(values) if name == "length" => Ok(Value::Number(values.len() as f64)),
            Value::List(values) => Ok(name
                .parse::<usize>()
                .ok()
                .and_then(|i| values.get(i).cloned())
                .unwrap_or(Value::Undefined)),
            Value::String(s) if name == "length" => {
                Ok(Value::Number(s.encode_utf16().count() as f64))
            }
            other => Err(Error::new(format!("cannot read '{name}' from {other:?}"))),
        }
    }

    fn call_receiver(&self, owner: ObjectId, root: ObjectId, name: &str) -> ObjectId {
        for object in [owner, root] {
            let node = &self.model.nodes[object.index];
            if node.methods.contains_key(name)
                || node.signals.contains_key(name)
                || node.native.contains_key(name)
            {
                return object;
            }
        }
        self.model.root()
    }

    pub(crate) fn invoke(
        &mut self,
        owner: ObjectId,
        name: &str,
        arguments: Vec<Value>,
        reads: &mut BTreeSet<SlotId>,
    ) -> Result<Value, Error> {
        let node = self
            .model
            .node(owner)
            .ok_or_else(|| Error::new("foreign or unknown object"))?;
        if let Some(method) = node.methods.get(name).cloned() {
            let arguments = self.arguments(&method.parameters, arguments)?;
            return self.run(owner, &method, arguments, reads);
        }
        if let Some(method) = node.native.get(name) {
            let arguments = self.arguments(&method.parameters, arguments)?;
            let value = (method.callback)(&arguments)?;
            return self.model.typed(&method.result, value);
        }
        if node.signals.contains_key(name) {
            self.signal(owner, name, arguments)?;
            return Ok(Value::Undefined);
        }
        Err(Error::new(format!("unknown method or signal '{name}'")))
    }

    pub(crate) fn run(
        &mut self,
        owner: ObjectId,
        proc: &Procedure,
        args: Vec<Value>,
        reads: &mut BTreeSet<SlotId>,
    ) -> Result<Value, Error> {
        if self.depth >= 64 {
            return Err(Error::new("QML call stack limit exceeded"));
        }
        self.depth += 1;
        let mut locals = proc
            .parameters
            .iter()
            .map(|p| p.name.clone())
            .zip(args)
            .collect();
        let result = self.statements(owner, &proc.body, &mut locals, reads);
        self.depth -= 1;
        let value = match result? {
            Flow::Next(_) => Value::Undefined,
            Flow::Return(value) => value,
        };
        match proc.result.as_deref() {
            None | Some("var") => Ok(value),
            Some("void") if value == Value::Undefined => Ok(value),
            Some(ty) => self.model.typed(ty, value),
        }
    }

    pub(crate) fn statements(
        &mut self,
        owner: ObjectId,
        body: &[Stmt],
        locals: &mut Locals,
        reads: &mut BTreeSet<SlotId>,
    ) -> Result<Flow, Error> {
        let mut completion = Value::Undefined;
        for stmt in body {
            match stmt {
                Stmt::Expr(expr) => completion = self.expr(owner, expr, locals, reads)?,
                Stmt::Let { name, value } => {
                    let value = match value {
                        Some(expr) => self.expr(owner, expr, locals, reads)?,
                        None => Value::Undefined,
                    };
                    locals.insert(name.clone(), value);
                }
                Stmt::Return(value) => {
                    return Ok(Flow::Return(match value {
                        Some(expr) => self.expr(owner, expr, locals, reads)?,
                        None => Value::Undefined,
                    }))
                }
                Stmt::Assign { target, op, value } => {
                    let place = match target {
                        Expr::Name(name) | Expr::ScopedName { name, .. }
                            if locals.contains_key(name) =>
                        {
                            Place::Local(name.clone())
                        }
                        Expr::Name(name) => {
                            Place::Slot(self.lookup_property(owner, self.model.root(), name)?)
                        }
                        Expr::ScopedName { name, root } => {
                            Place::Slot(self.lookup_property(owner, self.scope_id(root)?, name)?)
                        }
                        Expr::Member(base, name) => {
                            let receiver = object(self.expr(owner, base, locals, reads)?)?;
                            Place::Slot(self.model.key(receiver, name)?)
                        }
                        Expr::Index(base, index) => {
                            let receiver = object(self.expr(owner, base, locals, reads)?)?;
                            let name = self.expr(owner, index, locals, reads)?.js_string();
                            Place::Slot(self.model.key(receiver, &name)?)
                        }
                        _ => return Err(Error::new("invalid assignment target")),
                    };
                    let previous = if op == "=" {
                        None
                    } else {
                        Some(match &place {
                            Place::Slot(key) => self.read(*key, reads)?,
                            Place::Local(name) => locals[name].clone(),
                        })
                    };
                    let value = self.expr(owner, value, locals, reads)?;
                    completion = match previous {
                        None => value,
                        Some(old) => binary(
                            op.strip_suffix('=')
                                .ok_or_else(|| Error::new("invalid assignment operator"))?,
                            old,
                            value,
                        )?,
                    };
                    match place {
                        Place::Local(name) => {
                            locals.insert(name, completion.clone());
                        }
                        Place::Slot(key) => self.write(key, completion.clone())?,
                    }
                }
                Stmt::If {
                    test,
                    then_body,
                    else_body,
                } => {
                    let yes = self.expr(owner, test, locals, reads)?.truthy();
                    match self.block(
                        owner,
                        if yes { then_body } else { else_body },
                        locals,
                        reads,
                    )? {
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        Flow::Next(v) => completion = v,
                    }
                }
                Stmt::Block(body) => match self.block(owner, body, locals, reads)? {
                    Flow::Return(v) => return Ok(Flow::Return(v)),
                    Flow::Next(v) => completion = v,
                },
            }
        }
        Ok(Flow::Next(completion))
    }

    fn block(
        &mut self,
        owner: ObjectId,
        body: &[Stmt],
        locals: &mut Locals,
        reads: &mut BTreeSet<SlotId>,
    ) -> Result<Flow, Error> {
        // Restore shadowed bindings on leaving a lexical block. Assignments to
        // the enclosing scope remain visible after the block.
        let shadowed: BTreeMap<_, _> = body
            .iter()
            .filter_map(|s| match s {
                Stmt::Let { name, .. } => Some((name.clone(), locals.get(name).cloned())),
                _ => None,
            })
            .collect();
        let flow = self.statements(owner, body, locals, reads)?;
        for (name, old) in shadowed {
            if let Some(value) = old {
                locals.insert(name, value);
            } else {
                locals.remove(&name);
            }
        }
        Ok(flow)
    }
}

enum Place {
    Slot(SlotId),
    Local(String),
}

fn object(value: Value) -> Result<ObjectId, Error> {
    if let Value::Object(id) = value {
        Ok(id)
    } else {
        Err(Error::new("expected a QML object"))
    }
}

fn int32(value: f64) -> i32 {
    value.trunc().rem_euclid(4294967296.0) as u32 as i32
}

fn loose_equal(a: &Value, b: &Value) -> Result<bool, Error> {
    if a == b {
        return Ok(true);
    }
    if matches!(a, Value::Null | Value::Undefined) || matches!(b, Value::Null | Value::Undefined) {
        return Ok(matches!(a, Value::Null | Value::Undefined)
            && matches!(b, Value::Null | Value::Undefined));
    }
    if matches!(
        (a, b),
        (Value::Number(_), Value::String(_))
            | (Value::String(_), Value::Number(_))
            | (Value::Bool(_), _)
            | (_, Value::Bool(_))
    ) {
        return Ok(a.number()? == b.number()?);
    }
    Ok(false)
}

fn binary(op: &str, a: Value, b: Value) -> Result<Value, Error> {
    Ok(match op {
        "+" if matches!(a, Value::String(_)) || matches!(b, Value::String(_)) => {
            Value::String(a.js_string() + &b.js_string())
        }
        "+" => Value::Number(a.number()? + b.number()?),
        "-" => Value::Number(a.number()? - b.number()?),
        "*" => Value::Number(a.number()? * b.number()?),
        "/" => Value::Number(a.number()? / b.number()?),
        "%" => Value::Number(a.number()? % b.number()?),
        "**" => Value::Number(a.number()?.powf(b.number()?)),
        "===" => Value::Bool(a == b),
        "!==" => Value::Bool(a != b),
        "==" => Value::Bool(loose_equal(&a, &b)?),
        "!=" => Value::Bool(!loose_equal(&a, &b)?),
        "<" | ">" | "<=" | ">=" => {
            let ordering = match (&a, &b) {
                (Value::String(a), Value::String(b)) => {
                    Some(a.encode_utf16().cmp(b.encode_utf16()))
                }
                _ => a.number()?.partial_cmp(&b.number()?),
            };
            Value::Bool(ordering.is_some_and(|o| match op {
                "<" => o.is_lt(),
                ">" => o.is_gt(),
                "<=" => o.is_le(),
                _ => o.is_ge(),
            }))
        }
        "&" => Value::Number((int32(a.number()?) & int32(b.number()?)) as f64),
        "|" => Value::Number((int32(a.number()?) | int32(b.number()?)) as f64),
        "^" => Value::Number((int32(a.number()?) ^ int32(b.number()?)) as f64),
        "<<" => {
            Value::Number(int32(a.number()?).wrapping_shl(int32(b.number()?) as u32 & 31) as f64)
        }
        ">>" => {
            Value::Number(int32(a.number()?).wrapping_shr(int32(b.number()?) as u32 & 31) as f64)
        }
        ">>>" => Value::Number(
            (int32(a.number()?) as u32).wrapping_shr(int32(b.number()?) as u32 & 31) as f64,
        ),
        _ => return Err(Error::new(format!("unsupported binary operator '{op}'"))),
    })
}

fn math(name: &str, args: &[Value]) -> Result<Value, Error> {
    let first = args.first().unwrap_or(&Value::Undefined).number()?;
    let second = args.get(1).unwrap_or(&Value::Undefined).number()?;
    let value = match name {
        "abs" => first.abs(),
        "sqrt" => first.sqrt(),
        "sin" => first.sin(),
        "cos" => first.cos(),
        "tan" => first.tan(),
        "floor" => first.floor(),
        "ceil" => first.ceil(),
        "trunc" => first.trunc(),
        "round" if (-0.5..0.0).contains(&first) || first == -0.5 => -0.0,
        "round" => {
            if first - first.floor() >= 0.5 {
                first.ceil()
            } else {
                first.floor()
            }
        }
        "pow" => first.powf(second),
        "atan2" => first.atan2(second),
        "min" | "max" => {
            let mut folded = if name == "min" {
                f64::INFINITY
            } else {
                f64::NEG_INFINITY
            };
            for arg in args {
                let n = arg.number()?;
                if n.is_nan() {
                    return Ok(Value::Number(n));
                }
                folded = if name == "min" {
                    folded.min(n)
                } else {
                    folded.max(n)
                };
            }
            folded
        }
        _ => return Err(Error::new(format!("unsupported Math function '{name}'"))),
    };
    Ok(Value::Number(value))
}
