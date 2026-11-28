//! Application fixture compiled independently with rustc and no framework crate.
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
type Trace = Rc<RefCell<Vec<String>>>;
type Hook = Rc<dyn Fn()>;

pub enum DomainValue {
    Payload {
        name: &'static str,
        mode: &'static str,
        state: Cell<i32>,
        trace: Trace,
        hook: RefCell<Option<Hook>>,
    },
    Other {
        trace: Trace,
    },
}
impl DomainValue {
    pub fn new(name: &'static str, mode: &'static str, trace: Trace) -> Rc<Self> {
        Rc::new(Self::Payload {
            name,
            mode,
            state: Cell::new(0),
            trace,
            hook: RefCell::default(),
        })
    }
    pub fn other(trace: Trace) -> Rc<Self> {
        Rc::new(Self::Other { trace })
    }
    pub fn name(&self) -> &'static str {
        match self {
            Self::Payload { name, .. } => name,
            Self::Other { .. } => "other",
        }
    }
    pub fn state(&self) -> &Cell<i32> {
        match self {
            Self::Payload { state, .. } => state,
            _ => panic!("payload required"),
        }
    }
    pub fn set_hook(&self, value: Hook) {
        match self {
            Self::Payload { hook, .. } => *hook.borrow_mut() = Some(value),
            _ => panic!("payload required"),
        }
    }
}
impl PartialEq for DomainValue {
    fn eq(&self, other: &Self) -> bool {
        let trace = match self {
            Self::Payload { trace, .. } | Self::Other { trace } => trace,
        };
        trace
            .borrow_mut()
            .push(format!("equals:{}:{}", self.name(), other.name()));
        if let Self::Payload { hook, .. } = self {
            let hook = hook.borrow().clone();
            if let Some(hook) = hook {
                hook();
            }
        }
        match self {
            Self::Payload { mode, .. } => match *mode {
                "false" => false,
                "value" => matches!(other, Self::Payload { .. }),
                "cross" => matches!(other, Self::Other { .. }),
                _ => std::ptr::eq(self, other),
            },
            Self::Other { .. } => std::ptr::eq(self, other),
        }
    }
}
