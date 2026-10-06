//! Extension methods for the logical tree, as inherent methods of
//! [`StyledElement`], the one implementation of `ILogical`.

use crate::{ObjectType, Ref, StyledElement};
use std::rc::Rc;

/// Iterates the logical ancestors of an element, from its parent to the
/// root.
pub struct LogicalAncestors {
    next: Option<Ref<StyledElement>>,
}

impl Iterator for LogicalAncestors {
    type Item = Ref<StyledElement>;

    fn next(&mut self) -> Option<Ref<StyledElement>> {
        let current = self.next.take()?;
        self.next = current.parent();
        Some(current)
    }
}

/// Iterates the logical descendants of an element in depth-first
/// pre-order.
pub struct LogicalDescendants {
    /// The snapshots of the children collections being enumerated, with the
    /// index of the next child to visit in each.
    stack: Vec<(Rc<Vec<Ref<StyledElement>>>, usize)>,
    pending: Option<Ref<StyledElement>>,
}

impl LogicalDescendants {
    fn push_children(&mut self, logical: &StyledElement) {
        let children = logical.logical_children().snapshot();
        if !children.is_empty() {
            self.stack.push((children, 0));
        }
    }
}

impl Iterator for LogicalDescendants {
    type Item = Ref<StyledElement>;

    fn next(&mut self) -> Option<Ref<StyledElement>> {
        if let Some(pending) = self.pending.take() {
            return Some(pending);
        }

        loop {
            let (children, index) = self.stack.last_mut()?;
            if *index >= children.len() {
                self.stack.pop();
                continue;
            }

            let child = children[*index].clone();
            *index += 1;
            self.push_children(&child);
            return Some(child);
        }
    }
}

impl StyledElement {
    /// Enumerates the ancestors of the element in the logical tree, from its
    /// parent to the root.
    pub fn get_logical_ancestors(&self) -> LogicalAncestors {
        LogicalAncestors { next: self.parent() }
    }

    /// Enumerates the element and its ancestors in the logical tree.
    pub fn get_self_and_logical_ancestors(&self) -> LogicalAncestors {
        LogicalAncestors { next: Some(self.to_ref()) }
    }

    /// Finds the first logical ancestor of class `T`, optionally starting
    /// with the element itself.
    pub fn find_logical_ancestor_of_type<T: ObjectType>(&self, include_self: bool) -> Option<Ref<T>> {
        let mut parent = if include_self { Some(self.to_ref()) } else { self.parent() };

        while let Some(current) = parent {
            if let Some(result) = current.clone().cast::<T>() {
                return Some(result);
            }
            parent = current.parent();
        }

        None
    }

    /// The logical children of the element.
    pub fn get_logical_children(&self) -> Rc<Vec<Ref<StyledElement>>> {
        self.logical_children().snapshot()
    }

    /// Enumerates the descendants of the element in the logical tree, in
    /// depth-first pre-order.
    pub fn get_logical_descendants(&self) -> LogicalDescendants {
        let mut result = LogicalDescendants { stack: Vec::new(), pending: None };
        result.push_children(self);
        result
    }

    /// Enumerates the element and its descendants in the logical tree, in
    /// depth-first pre-order.
    pub fn get_self_and_logical_descendants(&self) -> LogicalDescendants {
        let mut result = self.get_logical_descendants();
        result.pending = Some(self.to_ref());
        result
    }

    /// Finds the first logical descendant of class `T` (depth first),
    /// optionally starting with the element itself.
    pub fn find_logical_descendant_of_type<T: ObjectType>(&self, include_self: bool) -> Option<Ref<T>> {
        if include_self {
            if let Some(result) = self.to_ref().cast::<T>() {
                return Some(result);
            }
        }

        Self::find_descendant_of_type_core::<T>(self)
    }

    /// The logical parent of the element.
    pub fn get_logical_parent(&self) -> Option<Ref<StyledElement>> {
        self.parent()
    }

    /// The logical parent of the element, if it is of class `T`.
    pub fn get_logical_parent_of_type<T: ObjectType>(&self) -> Option<Ref<T>> {
        self.parent().and_then(|parent| parent.cast::<T>())
    }

    /// The children of the logical parent of the element, including the
    /// element itself; empty when the element has no logical parent.
    pub fn get_logical_siblings(&self) -> Rc<Vec<Ref<StyledElement>>> {
        match self.parent() {
            Some(parent) => parent.logical_children().snapshot(),
            None => Rc::default(),
        }
    }

    /// Whether the element is a logical ancestor of `target`.
    pub fn is_logical_ancestor_of(&self, target: Option<&StyledElement>) -> bool {
        let mut current = target.and_then(StyledElement::parent);

        while let Some(parent) = current {
            if std::ptr::eq::<StyledElement>(&*parent, self) {
                return true;
            }
            current = parent.parent();
        }

        false
    }

    fn find_descendant_of_type_core<T: ObjectType>(logical: &StyledElement) -> Option<Ref<T>> {
        let logical_children = logical.logical_children().snapshot();

        for child in logical_children.iter() {
            if let Some(result) = child.clone().cast::<T>() {
                return Some(result);
            }

            if let Some(child_result) = Self::find_descendant_of_type_core::<T>(child) {
                return Some(child_result);
            }
        }

        None
    }
}
