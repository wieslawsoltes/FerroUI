//! Port of `TypeSystem/XamlLocalsPool.cs`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::exceptions::{XamlError, XamlResult};

use super::{IXamlLocal, IXamlType};

type LocalsList = RefCell<Vec<(Rc<dyn IXamlType>, Rc<dyn IXamlLocal>)>>;

pub struct XamlLocalsPool {
    local_factory: Box<dyn Fn(&Rc<dyn IXamlType>) -> Rc<dyn IXamlLocal>>,
    locals_pool: Rc<LocalsList>,
}

/// A local borrowed from the pool. Dropping it (the C# `Dispose`) returns the local to the pool.
pub struct PooledLocal {
    local: Option<Rc<dyn IXamlLocal>>,
    parent: Rc<LocalsList>,
    type_: Rc<dyn IXamlType>,
}

impl PooledLocal {
    /// Fails with `ObjectDisposedException` once the local has been returned to the pool.
    pub fn local(&self) -> XamlResult<Rc<dyn IXamlLocal>> {
        self.local
            .clone()
            .ok_or_else(|| XamlError::internal("ObjectDisposedException", "PooledLocal"))
    }

    pub fn dispose(&mut self) {
        if let Some(local) = self.local.take() {
            self.parent.borrow_mut().push((self.type_.clone(), local));
        }
    }
}

impl Drop for PooledLocal {
    fn drop(&mut self) {
        self.dispose();
    }
}

impl XamlLocalsPool {
    pub fn new(local_factory: impl Fn(&Rc<dyn IXamlType>) -> Rc<dyn IXamlLocal> + 'static) -> Self {
        Self {
            local_factory: Box::new(local_factory),
            locals_pool: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn get_local(&self, type_: &Rc<dyn IXamlType>) -> PooledLocal {
        let found = {
            let mut pool = self.locals_pool.borrow_mut();
            let index = pool.iter().position(|(t, _)| t.equals(&**type_));
            index.map(|c| pool.remove(c).1)
        };
        let local = found.unwrap_or_else(|| (self.local_factory)(type_));
        PooledLocal {
            local: Some(local),
            parent: self.locals_pool.clone(),
            type_: type_.clone(),
        }
    }
}
