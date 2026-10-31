//! ABI shared by generated Rust expressions and the source evaluator.
//! Reads performed by either backend pass through the same transaction.
use crate::evaluation::{math, object};
use crate::transaction::Transaction;
use crate::{Document, Error, Locals, ObjectId, SlotId, Value};
use std::collections::BTreeSet;

pub use crate::evaluation::{binary, unary, Flow};

/// An expression emitted as a native Rust function. The frame borrows the
/// active transaction only for the duration of one evaluation.
pub type Evaluator = fn(&mut Frame<'_, '_>) -> Result<Value, Error>;

pub type Script = fn(&mut Frame<'_, '_>) -> Result<Flow, Error>;

pub type FirstRead = Option<(&'static str, Option<&'static str>)>;

pub trait ComponentSource {
    fn first_reads(&self) -> &'static [FirstRead] {
        &[]
    }
    fn scripts(&self) -> &'static [Script] {
        &[]
    }
    fn document(&self) -> &Document;
    fn evaluators(&self) -> &'static [Evaluator] {
        &[]
    }
}
impl ComponentSource for Document {
    fn document(&self) -> &Document {
        self
    }
}

pub struct CompiledComponent {
    first_reads: &'static [FirstRead],
    document: Document,
    scripts: &'static [Script],
    evaluators: &'static [Evaluator],
}

impl CompiledComponent {
    pub fn with_first_reads(mut self, first_reads: &'static [FirstRead]) -> Self {
        self.first_reads = first_reads;
        self
    }

    pub fn new(
        document: Document,
        evaluators: &'static [Evaluator],
        scripts: &'static [Script],
    ) -> Self {
        Self {
            document,
            first_reads: &[],
            evaluators,
            scripts,
        }
    }
}
impl ComponentSource for CompiledComponent {
    fn first_reads(&self) -> &'static [FirstRead] {
        self.first_reads
    }
    fn scripts(&self) -> &'static [Script] {
        self.scripts
    }
    fn document(&self) -> &Document {
        &self.document
    }
    fn evaluators(&self) -> &'static [Evaluator] {
        self.evaluators
    }
}

pub struct Frame<'a, 'model> {
    pub(crate) tx: &'a mut Transaction<'model>,
    pub(crate) owner: ObjectId,
    pub(crate) locals: &'a mut Locals,
    pub(crate) reads: &'a mut BTreeSet<SlotId>,
}

pub enum Receiver {
    External(crate::ExternalValue),
    Callable(crate::ExternalValue),
    Array(crate::Record),
    ArrayConstructor,
    Object(ObjectId),
    Math,
    Qt,
}

impl Frame<'_, '_> {
    pub fn array(&mut self, items: Vec<Value>) -> Value {
        self.tx.new_array(items)
    }
    pub fn binary(&self, op: &str, a: Value, b: Value) -> Result<Value, Error> {
        self.tx.model.js_binary(op, a, b)
    }
    pub fn unary(&self, op: &str, v: Value) -> Result<Value, Error> {
        self.tx.model.js_unary(op, v)
    }

    pub fn record(&mut self, fields: std::collections::BTreeMap<String, Value>) -> Value {
        self.tx.new_record(fields)
    }
    fn root(&self, root: Option<&str>) -> Result<ObjectId, Error> {
        root.map_or_else(|| Ok(self.tx.model.root()), |id| self.tx.scope_id(id))
    }
    pub fn name(&mut self, name: &str, root: Option<&str>) -> Result<Value, Error> {
        self.tx
            .name(self.owner, self.root(root)?, name, self.locals, self.reads)
    }
    pub fn named_member(
        &mut self,
        base: &str,
        root: Option<&str>,
        name: &str,
    ) -> Result<Value, Error> {
        if !self.locals.contains_key(base) {
            if let Ok(key) = self.tx.scoped_key(self.owner, &format!("{base}.{name}")) {
                return self.tx.read(key, self.reads);
            }
            if base == "Math" {
                return Ok(Value::Number(match name {
                    "PI" => std::f64::consts::PI,
                    "E" => std::f64::consts::E,
                    other => return Err(Error::new(format!("unknown Math constant '{other}'"))),
                }));
            }
        }
        let value = self.name(base, root)?;
        self.member(value, name)
    }
    pub fn member(&mut self, base: Value, name: &str) -> Result<Value, Error> {
        self.tx.member(base, name, self.reads)
    }
    pub fn index(&mut self, base: Value, key: Value) -> Result<Value, Error> {
        self.member(base, &self.tx.model.value_string(&key)?)
    }
    pub fn local_receiver(&self, name: &str, root: Option<&str>) -> Result<Receiver, Error> {
        if let Some(local) = self.locals.get(name) {
            return match local {
                Value::External(value) => Ok(Receiver::Callable(value.clone())),
                _ => Err(Error::new(format!("'{name}' is not callable"))),
            };
        }
        Ok(Receiver::Object(self.tx.call_receiver(
            self.owner,
            self.root(root)?,
            name,
        )))
    }
    pub fn named_receiver(&mut self, name: &str, root: Option<&str>) -> Result<Receiver, Error> {
        if name == "Array" && !self.locals.contains_key(name) {
            Ok(Receiver::ArrayConstructor)
        } else if name == "Qt" && !self.locals.contains_key(name) {
            Ok(Receiver::Qt)
        } else if name == "Math" && !self.locals.contains_key(name) {
            Ok(Receiver::Math)
        } else {
            receiver(self.name(name, root)?)
        }
    }
    pub fn invoke(
        &mut self,
        receiver: Receiver,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value, Error> {
        match receiver {
            Receiver::Callable(value) => {
                self.tx
                    .with_external(self.owner, &value, self.reads, |engine, ctx| {
                        engine.call_external(ctx, &value, Value::Undefined, args)
                    })
            }
            Receiver::External(base) => {
                self.tx
                    .with_external(self.owner, &base, self.reads, |engine, ctx| {
                        let Value::External(callable) = engine.get_external(ctx, &base, name)?
                        else {
                            return Err(Error::new(format!("'{name}' is not callable")));
                        };
                        engine.call_external(ctx, &callable, Value::External(base.clone()), args)
                    })
            }
            Receiver::Array(array) => self.tx.array_method(&array, name, args),
            Receiver::ArrayConstructor if name == "isArray" => Ok(Value::Bool(
                matches!(args.first(),Some(Value::Record(r)) if r.is_array()),
            )),
            Receiver::ArrayConstructor => Err(Error::new(format!("unsupported Array.{name}"))),
            Receiver::Object(id) => self.tx.invoke(id, name, args, self.reads),
            Receiver::Qt => crate::enums::qt(name, &args),
            Receiver::Math => math(name, &args),
        }
    }
}
pub fn receiver(value: Value) -> Result<Receiver, Error> {
    if let Value::External(value) = value {
        return Ok(Receiver::External(value));
    }
    if let Value::Record(r) = &value {
        if r.is_array() {
            return Ok(Receiver::Array(r.clone()));
        }
    }
    object(value).map(Receiver::Object)
}
pub fn truthy(value: &Value) -> bool {
    value.truthy()
}

/// A resolved assignment target. Resolving the target precedes evaluation of
/// the right hand side, including for indexed and compound assignments.
pub struct Target {
    instance: u64,
    location: Location,
}
enum Location {
    External(crate::ExternalValue, String),
    ModelRole(crate::RowId, String),
    Local(String),
    Property(SlotId),
    Record(crate::Record, String),
}

impl Frame<'_, '_> {
    pub(crate) fn interpret(&mut self, expr: &crate::ir::Expr) -> Result<Value, Error> {
        self.tx.expr(self.owner, expr, self.locals, self.reads)
    }
    fn target(&self, location: Location) -> Target {
        Target {
            instance: self.owner.instance,
            location,
        }
    }
    pub fn target_name(&self, name: &str, root: Option<&str>) -> Result<Target, Error> {
        let location = if self.locals.contains_key(name) {
            Location::Local(name.to_owned())
        } else {
            Location::Property(
                self.tx
                    .lookup_property(self.owner, self.root(root)?, name)?,
            )
        };
        Ok(self.target(location))
    }
    pub fn target_named_member(
        &mut self,
        base: &str,
        root: Option<&str>,
        name: &str,
    ) -> Result<Target, Error> {
        if !self.locals.contains_key(base) {
            if let Ok(key) = self.tx.scoped_key(self.owner, &format!("{base}.{name}")) {
                return Ok(self.target(Location::Property(key)));
            }
        }
        let base = self.name(base, root)?;
        self.target_member(base, name)
    }
    pub fn target_member(&self, base: Value, name: &str) -> Result<Target, Error> {
        if let Value::External(value) = base {
            self.tx
                .model
                .validate_handles(&Value::External(value.clone()))?;
            return Ok(self.target(Location::External(value, name.into())));
        }
        if let Value::ModelRow(row) = base {
            return Ok(self.target(Location::ModelRole(row, name.to_owned())));
        }
        if let Value::Record(record) = base {
            self.tx.model.record_entries(&record)?;
            return Ok(self.target(Location::Record(record, name.into())));
        }
        let key = self.tx.scoped_key(object(base)?, name)?;
        Ok(self.target(Location::Property(key)))
    }
    pub fn target_index(&self, base: Value, key: Value) -> Result<Target, Error> {
        self.target_member(base, &self.tx.model.value_string(&key)?)
    }
    pub fn assign(
        &mut self,
        place: Target,
        op: &str,
        rhs: impl FnOnce(&mut Self) -> Result<Value, Error>,
    ) -> Result<Value, Error> {
        if place.instance != self.owner.instance {
            return Err(Error::new("foreign assignment target"));
        }
        let mut postfix = None;
        let value = if op == "=" {
            rhs(self)?
        } else {
            let old = match &place.location {
                Location::External(value, name) => {
                    self.tx
                        .with_external(self.owner, value, self.reads, |engine, ctx| {
                            engine.get_external(ctx, value, name)
                        })?
                }
                Location::ModelRole(row, name) => self.tx.read_role(*row, name, self.reads)?,
                Location::Local(name) => self
                    .locals
                    .get(name)
                    .ok_or_else(|| Error::new("local is out of scope"))?
                    .clone(),
                Location::Record(record, name) => self
                    .tx
                    .model
                    .record_entries(record)?
                    .get(name)
                    .cloned()
                    .unwrap_or(Value::Undefined),
                Location::Property(key) => self.tx.read(*key, self.reads)?,
            };
            match op {
                "post++" | "post--" | "++" | "--" => {
                    let number = old.number()?;
                    if op.starts_with("post") {
                        postfix = Some(Value::Number(number));
                    }
                    Value::Number(if op.ends_with("++") {
                        number + 1.0
                    } else {
                        number - 1.0
                    })
                }
                "&&=" if !old.truthy() => return Ok(old),
                "||=" if old.truthy() => return Ok(old),
                "??=" if !matches!(old, Value::Null | Value::Undefined) => return Ok(old),
                "&&=" | "||=" | "??=" => rhs(self)?,
                _ => {
                    let rhs = rhs(self)?;
                    self.binary(
                        op.strip_suffix('=')
                            .ok_or_else(|| Error::new("invalid assignment operator"))?,
                        old,
                        rhs,
                    )?
                }
            }
        };
        let completion = postfix.unwrap_or_else(|| value.clone());
        match place.location {
            Location::External(external, key) => {
                self.tx
                    .with_external(self.owner, &external, self.reads, |engine, ctx| {
                        engine.set_external(ctx, &external, &key, value)
                    })?
            }
            Location::ModelRole(row, name) => self.tx.write_role(row, &name, value)?,
            Location::Local(name) => {
                self.locals.insert(name, value);
            }
            Location::Record(record, key) => self.tx.set_record_field(&record, &key, value)?,
            Location::Property(key) => {
                self.tx.write(key, value)?;
                // A subsequent statement can observe a QML change handler's writes.
                // External observers still see only the committed transaction.
                self.tx.settle()?;
            }
        }
        Ok(completion)
    }
    pub fn declare(&mut self, name: &str, value: Value) {
        self.locals.insert(name.to_owned(), value);
    }
    pub fn iteration(&mut self) -> Result<(), Error> {
        self.tx.budget()
    }
    pub fn block(
        &mut self,
        names: &[&str],
        body: impl FnOnce(&mut Self) -> Result<Flow, Error>,
    ) -> Result<Flow, Error> {
        let previous: Vec<_> = names
            .iter()
            .map(|&name| (name, self.locals.get(name).cloned()))
            .collect();
        let result = body(self);
        for (name, value) in previous {
            if let Some(value) = value {
                self.locals.insert(name.to_owned(), value);
            } else {
                self.locals.remove(name);
            }
        }
        result
    }
}
