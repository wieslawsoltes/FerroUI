use super::*;
use crate::transaction::Transaction;
use ferroui_qml_compiler::Expr;

pub enum Flow {
    Next(Value),
    Return(Value),
    Break,
    Continue,
}
impl Flow {
    pub(crate) fn value(self) -> Result<Value, Error> {
        match self {
            Self::Next(v) | Self::Return(v) => Ok(v),
            Self::Break | Self::Continue => Err(Error::new("loop control outside a loop")),
        }
    }
}

impl Transaction<'_> {
    pub(crate) fn lookup_property(
        &self,
        owner: ObjectId,
        root: ObjectId,
        name: &str,
    ) -> Result<SlotId, Error> {
        self.scoped_key(owner, name)
            .or_else(|_| self.scoped_key(root, name))
            .or_else(|_| self.scoped_key(self.model.root(), name))
    }

    pub(crate) fn name(
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
        if let Some(id) = self.model.ids.get(name) {
            return Ok(Value::Object(*id));
        }
        if let Ok(key) = self.scoped_key(owner, name) {
            return self.read(key, reads);
        }
        if let Some(value) = self.model.nodes[owner.index].type_scope.member(name) {
            return Ok(value);
        }
        self.read(self.lookup_property(owner, root, name)?, reads)
    }

    pub(crate) fn scope_id(&self, name: &str) -> Result<ObjectId, Error> {
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
        self.with_stack(1, |tx| {
            tx.evaluate_expression(owner, expression, locals, reads)
        })
    }

    fn evaluate_expression(
        &mut self,
        owner: ObjectId,
        expression: &Expr,
        locals: &mut Locals,
        reads: &mut BTreeSet<SlotId>,
    ) -> Result<Value, Error> {
        use Expr::*;
        Ok(match expression {
            Native(index) => {
                let eval = *self
                    .model
                    .native_functions
                    .get(*index)
                    .ok_or_else(|| Error::new("invalid compiled expression index"))?;
                eval(&mut compiled::Frame {
                    tx: self,
                    owner,
                    locals,
                    reads,
                })?
            }
            Null => Value::Null,
            Undefined => Value::Undefined,
            Bool(b) => Value::Bool(*b),
            Number(n) => Value::Number(*n),
            String(s) => Value::String(s.clone()),
            Name(name) => self.name(owner, self.model.root(), name, locals, reads)?,
            ScopedName { name, root } => {
                self.name(owner, self.scope_id(root)?, name, locals, reads)?
            }
            Record(fields) => {
                let mut values = BTreeMap::new();
                for (key, expr) in fields {
                    values.insert(key.clone(), self.expr(owner, expr, locals, reads)?);
                }
                self.new_record(values)
            }
            Array(items) => {
                let items = items
                    .iter()
                    .map(|e| self.expr(owner, e, locals, reads))
                    .collect::<Result<Vec<_>, _>>()?;
                self.new_array(items)
            }
            Member(base, name) => {
                if let Some((group, root)) = lexical_name(base) {
                    return compiled::Frame {
                        tx: self,
                        owner,
                        locals,
                        reads,
                    }
                    .named_member(group, root, name);
                }
                let base = self.expr(owner, base, locals, reads)?;
                self.member(base, name, reads)?
            }
            Index(base, index) => {
                let base = self.expr(owner, base, locals, reads)?;
                let index = self.expr(owner, index, locals, reads)?;
                self.member(base, &self.model.value_string(&index)?, reads)?
            }
            Unary(op, expr) => {
                let value = self.expr(owner, expr, locals, reads)?;
                self.model.js_unary(op, value)?
            }
            Binary(op, left, right) => {
                let a = self.expr(owner, left, locals, reads)?;
                match op.as_str() {
                    "&&" if !a.truthy() => a,
                    "||" if a.truthy() => a,
                    "??" if !matches!(a, Value::Null | Value::Undefined) => a,
                    "&&" | "||" | "??" => self.expr(owner, right, locals, reads)?,
                    _ => {
                        let b = self.expr(owner, right, locals, reads)?;
                        self.model.js_binary(op, a, b)?
                    }
                }
            }
            Conditional(test, yes, no) => {
                let test = self.expr(owner, test, locals, reads)?;
                self.expr(owner, if test.truthy() { yes } else { no }, locals, reads)?
            }
            Call(target, arguments) => {
                let (receiver, name) = match target.as_ref() {
                    Member(base, name) => {
                        let receiver = if let Some((group, root)) = lexical_name(base) {
                            compiled::Frame {
                                tx: self,
                                owner,
                                locals,
                                reads,
                            }
                            .named_receiver(group, root)?
                        } else {
                            compiled::receiver(self.expr(owner, base, locals, reads)?)?
                        };
                        (receiver, name.as_str())
                    }
                    other => {
                        let (name, root) = lexical_name(other)
                            .ok_or_else(|| Error::new("expected a named QML method"))?;
                        (
                            compiled::Frame {
                                tx: self,
                                owner,
                                locals,
                                reads,
                            }
                            .local_receiver(name, root)?,
                            name,
                        )
                    }
                };
                let arguments: Vec<_> = arguments
                    .iter()
                    .map(|e| self.expr(owner, e, locals, reads))
                    .collect::<Result<_, _>>()?;
                compiled::Frame {
                    tx: self,
                    owner,
                    locals,
                    reads,
                }
                .invoke(receiver, name, arguments)?
            }
        })
    }

    pub(crate) fn member(
        &mut self,
        base: Value,
        name: &str,
        reads: &mut BTreeSet<SlotId>,
    ) -> Result<Value, Error> {
        match base {
            Value::ModelRow(row) => self.read_role(row, name, reads),
            Value::Record(record) => Ok(self
                .model
                .record_entries(&record)?
                .get(name)
                .cloned()
                .unwrap_or(Value::Undefined)),
            Value::Namespace(namespace) => namespace
                .member(name)
                .ok_or_else(|| Error::new(format!("unknown type or enum member '{name}'"))),
            Value::Object(id) if name == "parent" => Ok(self
                .model
                .parent(id)
                .map(Value::Object)
                .unwrap_or(Value::Null)),
            Value::Object(id) => self.read(self.scoped_key(id, name)?, reads),
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

    pub(crate) fn call_receiver(&self, owner: ObjectId, root: ObjectId, name: &str) -> ObjectId {
        for object in [owner, root] {
            let node = &self.model.nodes[object.index];
            if node.methods.contains_key(name)
                || node.signals.contains_key(name)
                || node.native.contains_key(name)
                || (self.model.models.contains_key(&object) && list_model::is_method(name))
                || (self.model.timers.is_timer(object)
                    && matches!(name, "start" | "stop" | "restart"))
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
        self.budget()?;
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
        if self.model.models.contains_key(&owner) && list_model::is_method(name) {
            return self.model_call(owner, name, &arguments, reads);
        }
        if self.model.timers.is_timer(owner) && matches!(name, "start" | "stop" | "restart") {
            return self.timer_method(owner, name, &arguments);
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
        let mut locals = proc
            .parameters
            .iter()
            .map(|p| p.name.clone())
            .zip(args)
            .collect();
        let result = self.with_scope(proc.scope, |tx| {
            tx.with_stack(4, |tx| tx.statements(owner, &proc.body, &mut locals, reads))
        });
        let value = match result? {
            Flow::Next(_) => Value::Undefined,
            Flow::Return(value) => value,
            Flow::Break | Flow::Continue => return Err(Error::new("loop control outside a loop")),
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
                Stmt::Native(index) => {
                    let script = *self
                        .model
                        .native_scripts
                        .get(*index)
                        .ok_or_else(|| Error::new("invalid compiled script index"))?;
                    return script(&mut compiled::Frame {
                        tx: self,
                        owner,
                        locals,
                        reads,
                    });
                }
                Stmt::Assign { target, op, value } => {
                    let mut frame = compiled::Frame {
                        tx: self,
                        owner,
                        locals,
                        reads,
                    };
                    let place = match target {
                        Expr::Name(name) => frame.target_name(name, None)?,
                        Expr::ScopedName { name, root } => frame.target_name(name, Some(root))?,
                        Expr::Member(base, name) => {
                            if let Some((base, scope)) = lexical_name(base) {
                                frame.target_named_member(base, scope, name)?
                            } else {
                                let base = frame.interpret(base)?;
                                frame.target_member(base, name)?
                            }
                        }
                        Expr::Index(base, index) => {
                            let base = frame.interpret(base)?;
                            let key = frame.interpret(index)?;
                            frame.target_index(base, key)?
                        }
                        _ => return Err(Error::new("invalid assignment target")),
                    };
                    completion = frame.assign(place, op, |f| f.interpret(value))?;
                }
                Stmt::Break => return Ok(Flow::Break),
                Stmt::Continue => return Ok(Flow::Continue),
                Stmt::While {
                    test,
                    body,
                    post_test,
                } => {
                    match self.iterate(owner, Some(test), body, None, *post_test, locals, reads)? {
                        Flow::Next(v) => completion = v,
                        flow => return Ok(flow),
                    }
                }
                Stmt::For {
                    init,
                    test,
                    update,
                    body,
                } => {
                    let previous = match init.as_deref() {
                        Some(Stmt::Let { name, .. }) => {
                            Some((name.clone(), locals.get(name).cloned()))
                        }
                        _ => None,
                    };
                    if let Some(init) = init {
                        self.statements(owner, std::slice::from_ref(init), locals, reads)?;
                    }
                    let flow = self.iterate(
                        owner,
                        test.as_ref(),
                        body,
                        update.as_deref(),
                        false,
                        locals,
                        reads,
                    )?;
                    if let Some((name, previous)) = previous {
                        if let Some(value) = previous {
                            locals.insert(name, value);
                        } else {
                            locals.remove(&name);
                        }
                    }
                    match flow {
                        Flow::Next(v) => completion = v,
                        flow => return Ok(flow),
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
                        Flow::Next(v) => completion = v,
                        flow => return Ok(flow),
                    }
                }
                Stmt::Block(body) => match self.block(owner, body, locals, reads)? {
                    Flow::Next(v) => completion = v,
                    flow => return Ok(flow),
                },
            }
        }
        Ok(Flow::Next(completion))
    }

    #[allow(clippy::too_many_arguments)]
    fn iterate(
        &mut self,
        owner: ObjectId,
        test: Option<&Expr>,
        body: &[Stmt],
        update: Option<&Stmt>,
        mut skip_first_test: bool,
        locals: &mut Locals,
        reads: &mut BTreeSet<SlotId>,
    ) -> Result<Flow, Error> {
        let mut completion = Value::Undefined;
        loop {
            self.budget()?;
            if !skip_first_test {
                if let Some(test) = test {
                    if !self.expr(owner, test, locals, reads)?.truthy() {
                        break;
                    }
                }
            }
            skip_first_test = false;
            match self.block(owner, body, locals, reads)? {
                Flow::Next(v) => completion = v,
                Flow::Continue => (),
                Flow::Break => break,
                flow @ Flow::Return(_) => return Ok(flow),
            }
            if let Some(update) = update {
                self.statements(owner, std::slice::from_ref(update), locals, reads)?;
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

pub(crate) fn object(value: Value) -> Result<ObjectId, Error> {
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

pub fn binary(op: &str, a: Value, b: Value) -> Result<Value, Error> {
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

pub(crate) fn math(name: &str, args: &[Value]) -> Result<Value, Error> {
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

pub fn unary(op: &str, value: Value) -> Result<Value, Error> {
    Ok(match op {
        "void" => Value::Undefined,
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
    })
}

fn lexical_name(expr: &Expr) -> Option<(&str, Option<&str>)> {
    match expr {
        Expr::Name(name) => Some((name, None)),
        Expr::ScopedName { name, root } => Some((name, Some(root))),
        _ => None,
    }
}
