//! Port of the pinned RibbonModelMerger.cs. Undo is explicit; Drop never unmerges.
use super::*;
use crate::RibbonMergeAction;
use crate::collections::ObservableCollection;
use std::{cell::RefCell, rc::Rc};
type Undo = Rc<dyn Fn()>;
type MergeChildren<T> = fn(&RibbonModelMerger, T, T);
/// Sealed source class: mutation notifications and user callbacks run without an undo-log borrow.
pub struct RibbonModelMerger {
    undo: RefCell<Vec<Undo>>,
}
impl RibbonModelMerger {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            undo: RefCell::new(vec![]),
        })
    }
    pub fn merge(
        target: Option<Rc<dyn RibbonModelImpl>>,
        source: Option<Rc<dyn RibbonModelImpl>>,
    ) -> Rc<Self> {
        let target = target.expect("ArgumentNull: target");
        let source = source.expect("ArgumentNull: source");
        let merger = Self::new();
        merger.merge_core(target, source);
        merger
    }
    pub fn unmerge(&self) {
        // Do not take/clear the log before callbacks. Source failures keep every
        // entry; reentrant calls can clear it and invalidate an outer index.
        let mut index = self.undo.borrow().len();
        while index > 0 {
            index -= 1;
            let action = self.undo.borrow()[index].clone();
            action();
        }
        // Clear only after successful actions. Release captured user objects
        // outside the borrow: a native Drop implementation may call back here.
        let released = std::mem::take(&mut *self.undo.borrow_mut());
        drop(released);
    }
    fn merge_core(&self, target: Rc<dyn RibbonModelImpl>, source: Rc<dyn RibbonModelImpl>) {
        self.merge_list(target.contextual_groups(), source.contextual_groups(), None);
        self.merge_list(target.tabs(), source.tabs(), Some(Self::merge_tab));
        self.merge_items(target.quick_access_items(), source.quick_access_items());
        self.merge_items(target.tab_strip_items(), source.tab_strip_items());
        self.merge_list(target.backstage().items(), source.backstage().items(), None);
        let candidates = source.quick_access_candidates().to_vec();
        for candidate in candidates.iter().cloned() {
            if target.quick_access_candidates().iter().all(|c| {
                c.expect("NullReference: candidate").id()
                    != candidate.as_ref().expect("NullReference: candidate").id()
            }) {
                self.insert(target.quick_access_candidates(), candidate);
            }
        }
        if let Some(source_catalog) = source.command_catalog()
            && !target
                .command_catalog()
                .is_some_and(|c| Rc::ptr_eq(&c, &source_catalog))
        {
            if target.command_catalog().is_none() {
                target.set_command_catalog(Some(source_catalog.clone()));
                self.undo.borrow_mut().push(Rc::new(move || {
                    if target
                        .command_catalog()
                        .is_some_and(|c| Rc::ptr_eq(&c, &source_catalog))
                    {
                        target.set_command_catalog(None);
                    }
                }));
            } else {
                let catalog = target.command_catalog().expect("source observed catalog");
                let descriptors = source_catalog.commands();
                for descriptor in &descriptors {
                    if catalog.find(Some(&descriptor.id())).is_none() {
                        catalog.register(Some(descriptor.clone()));
                        let descriptor = descriptor.clone();
                        let catalog = catalog.clone();
                        self.undo.borrow_mut().push(Rc::new(move || {
                            if catalog
                                .find(Some(&descriptor.id()))
                                .is_some_and(|v| Rc::ptr_eq(&v, &descriptor))
                            {
                                catalog.unregister(Some(&descriptor.id()));
                            }
                        }));
                    }
                }
            }
        }
    }
    fn merge_tab(&self, existing: RibbonTabReference, source: RibbonTabReference) {
        self.merge_list(
            existing.expect("NullReference: tab").groups(),
            source.expect("NullReference: tab").groups(),
            Some(Self::merge_group),
        );
    }
    fn merge_group(&self, existing: RibbonGroupReference, source: RibbonGroupReference) {
        self.merge_items(
            existing.expect("NullReference: group").items(),
            source.expect("NullReference: group").items(),
        );
    }
    fn merge_list<T: MergeNode>(
        &self,
        target: ObservableCollection<T>,
        source: ObservableCollection<T>,
        merge_children: Option<MergeChildren<T>>,
    ) {
        let snapshot = source.to_vec();
        for node in snapshot.iter().cloned() {
            let value = node.node().expect("NullReference: node");
            let existing = if value.id().is_none_or(|id| id.is_empty()) {
                None
            } else {
                target
                    .iter()
                    .find(|n| n.node().expect("NullReference: target node").id() == value.id())
            };
            match (value.merge_action(), existing) {
                (RibbonMergeAction::Remove, Some(existing)) => {
                    self.remove(target.clone(), existing)
                }
                (RibbonMergeAction::Remove, None) => {}
                (RibbonMergeAction::Replace, Some(existing)) => {
                    self.replace(target.clone(), existing, node)
                }
                (RibbonMergeAction::Merge, Some(existing)) => {
                    if let Some(children) = merge_children {
                        children(self, existing, node);
                    }
                }
                _ => self.insert(target.clone(), node),
            }
        }
    }
    fn merge_items(
        &self,
        target: ObservableCollection<RibbonItemReference>,
        source: ObservableCollection<RibbonItemReference>,
    ) {
        self.merge_list(target, source, Some(Self::merge_item_children));
    }
    fn merge_item_children(&self, existing: RibbonItemReference, source: RibbonItemReference) {
        let existing = existing.map(|v| v as Rc<dyn RibbonNodeModelImpl>);
        let source = source.map(|v| v as Rc<dyn RibbonNodeModelImpl>);
        if let (Some(target), Some(source)) = (
            cast::<dyn RibbonButtonGroupModelImpl>(existing.clone()),
            cast::<dyn RibbonButtonGroupModelImpl>(source.clone()),
        ) {
            self.merge_items(target.items(), source.items());
        } else if let (Some(target), Some(source)) = (
            cast::<dyn RibbonDropDownButtonModelImpl>(existing.clone()),
            cast::<dyn RibbonDropDownButtonModelImpl>(source.clone()),
        ) {
            self.merge_list(target.menu_items(), source.menu_items(), None);
        } else if let (Some(target), Some(source)) = (
            cast::<dyn RibbonGalleryModelImpl>(existing),
            cast::<dyn RibbonGalleryModelImpl>(source),
        ) {
            self.merge_list(target.items(), source.items(), None);
            self.merge_list(target.menu_items(), source.menu_items(), None);
        }
    }
    fn insert<T: MergeNode>(&self, list: ObservableCollection<T>, node: T) {
        let order = node.node().map_or(0, |n| n.order());
        let mut index = list.count();
        if order != 0 {
            let mut i = 0;
            while i < list.count() {
                if list.get(i).node().map_or(0, |v| v.order()) > order {
                    index = i;
                    break;
                }
                i += 1;
            }
        }
        list.insert(index, node.clone());
        self.undo.borrow_mut().push(Rc::new(move || {
            list.remove(&node);
        }));
    }
    fn remove<T: MergeNode>(&self, list: ObservableCollection<T>, node: T) {
        let Some(index) = list.index_of(&node) else {
            return;
        };
        let previous = if index > 0 {
            list.get(index - 1)
        } else {
            T::default()
        };
        list.remove_at(index);
        self.undo.borrow_mut().push(Rc::new(move || {
            let anchor = if previous.node().is_none() {
                None
            } else {
                list.index_of(&previous)
            };
            list.insert(
                anchor.map_or_else(|| index.min(list.count()), |v| v + 1),
                node.clone(),
            );
        }));
    }
    fn replace<T: MergeNode>(&self, list: ObservableCollection<T>, existing: T, replacement: T) {
        // A non-reflexive Equals can yield -1. Still call the setter so its
        // reentrancy check precedes the invalid-index failure, as in the source.
        let index = list.index_of(&existing).unwrap_or(usize::MAX);
        list.set(index, replacement.clone());
        self.undo.borrow_mut().push(Rc::new(move || {
            if let Some(current) = list.index_of(&replacement) {
                list.set(current, existing.clone());
            }
        }));
    }
}
trait MergeNode: Clone + PartialEq + Default + 'static {
    fn node(&self) -> RibbonNodeReference;
}
macro_rules! merge_node {($($ty:ty),*)=>{$(impl MergeNode for Option<Rc<$ty>> {fn node(&self)->RibbonNodeReference{self.clone().map(|v|v as Rc<dyn RibbonNodeModelImpl>)}})*};}
merge_node!(
    dyn RibbonNodeModelImpl,
    dyn RibbonItemModelImpl,
    dyn RibbonTabModelImpl,
    dyn RibbonGroupModelImpl,
    dyn RibbonContextualGroupModelImpl,
    dyn RibbonBackstageItemModelImpl,
    dyn RibbonGalleryItemModelImpl
);
fn cast<T: ?Sized + super::RibbonModelView>(node: RibbonNodeReference) -> Option<Rc<T>> {
    try_cast_node::<T>(node).expect("Core model class metadata is undeclared")
}
