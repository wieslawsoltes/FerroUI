//! Port of `Transform/XamlContextBase.cs`.

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::IXamlAstNode;
use crate::exceptions::{XamlError, XamlResult};

/// State shared by the transformation and emit contexts: a typed item bag and the stack of
/// parent nodes of the node being processed.
#[derive(Default)]
pub struct XamlContextBase {
    items: RefCell<HashMap<TypeId, Rc<dyn Any>>>,
    parent_nodes: RefCell<Vec<Rc<dyn IXamlAstNode>>>,
}

impl XamlContextBase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_item<T: 'static>(&self) -> XamlResult<Rc<T>> {
        self.try_get_item::<T>().ok_or_else(|| {
            XamlError::internal(
                "KeyNotFoundException",
                format!(
                    "The given key '{}' was not present in the dictionary.",
                    std::any::type_name::<T>()
                ),
            )
        })
    }

    pub fn get_or_create_item<T: Default + 'static>(&self) -> Rc<T> {
        if let Some(found) = self.try_get_item::<T>() {
            return found;
        }
        let created = Rc::new(T::default());
        self.items
            .borrow_mut()
            .insert(TypeId::of::<T>(), created.clone());
        created
    }

    pub fn try_get_item<T: 'static>(&self) -> Option<Rc<T>> {
        let item = self.items.borrow().get(&TypeId::of::<T>()).cloned();
        item.and_then(|i| i.downcast::<T>().ok())
    }

    pub fn set_item<T: 'static>(&self, item: Rc<T>) {
        self.items.borrow_mut().insert(TypeId::of::<T>(), item);
    }

    /// The parents of the current node, innermost first.
    pub fn parent_nodes(&self) -> Vec<Rc<dyn IXamlAstNode>> {
        self.parent_nodes.borrow().iter().rev().cloned().collect()
    }

    /// The innermost parent: `ParentNodes().FirstOrDefault()`.
    pub fn first_parent_node(&self) -> Option<Rc<dyn IXamlAstNode>> {
        self.parent_nodes.borrow().last().cloned()
    }

    /// `ParentNodes().Any()`.
    pub fn has_parent_nodes(&self) -> bool {
        !self.parent_nodes.borrow().is_empty()
    }

    pub fn push_parent(&self, node: Rc<dyn IXamlAstNode>) {
        self.parent_nodes.borrow_mut().push(node);
    }

    pub fn pop_parent(&self) -> Option<Rc<dyn IXamlAstNode>> {
        self.parent_nodes.borrow_mut().pop()
    }
}
