//! ABI shared by generated Rust expressions and the source evaluator.
//! Reads performed by either backend pass through the same transaction.
use crate::evaluation::{math, object};
use crate::transaction::Transaction;
use crate::{Document, Error, Locals, ObjectId, SlotId, Value};
use std::collections::BTreeSet;

pub use crate::evaluation::{binary, unary};

/// An expression emitted as a native Rust function. The frame borrows the
/// active transaction only for the duration of one evaluation.
pub type Evaluator = fn(&mut Frame<'_, '_>) -> Result<Value, Error>;

pub trait ComponentSource {
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
    document: Document,
    evaluators: &'static [Evaluator],
}

impl CompiledComponent {
    pub fn new(document: Document, evaluators: &'static [Evaluator]) -> Self {
        Self {
            document,
            evaluators,
        }
    }
}
impl ComponentSource for CompiledComponent {
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
    pub(crate) locals: &'a Locals,
    pub(crate) reads: &'a mut BTreeSet<SlotId>,
}

pub enum Receiver {
    Object(ObjectId),
    Math,
    Qt,
}

impl Frame<'_, '_> {
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
            if let Ok(key) = self.tx.model.key(self.owner, &format!("{base}.{name}")) {
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
        self.member(base, &key.js_string())
    }
    pub fn local_receiver(&self, name: &str, root: Option<&str>) -> Result<Receiver, Error> {
        Ok(Receiver::Object(self.tx.call_receiver(
            self.owner,
            self.root(root)?,
            name,
        )))
    }
    pub fn named_receiver(&mut self, name: &str, root: Option<&str>) -> Result<Receiver, Error> {
        if name == "Qt" && !self.locals.contains_key(name) {
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
            Receiver::Object(id) => self.tx.invoke(id, name, args, self.reads),
            Receiver::Qt => crate::enums::qt(name, &args),
            Receiver::Math => math(name, &args),
        }
    }
}
pub fn receiver(value: Value) -> Result<Receiver, Error> {
    object(value).map(Receiver::Object)
}
pub fn truthy(value: &Value) -> bool {
    value.truthy()
}
