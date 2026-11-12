use super::{impl_animated_server_object, IAnimatedServerObject, IServerObject, ServerCompositionVisual, ServerCompositor, ServerObject};
use crate::rendering::composition::generated::{
    ServerCompositionVisualCollectionHooks, ServerCompositionVisualCollectionProps,
};
use crate::platform::LtrbRect;
use crate::rendering::composition::transport::BatchStreamReader;
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};
use std::time::Duration;

/// From this many children a collection keeps a table of them.
const TABLE_FROM: usize = 32;

/// The bounds in the table of a child that has none: they meet nothing.
const NOWHERE: LtrbRect = LtrbRect::new(f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY);

#[cfg(test)]
crate::ferro_thread_local! {
    /// Whether no collection keeps a table: the walks visit every child, as upstream does.
    static WITHOUT_TABLES: Cell<bool> = const { Cell::new(false) };
    /// How many children the update pass and the render walk left out by a table.
    static LEFT_OUT: Cell<(usize, usize)> = const { Cell::new((0, 0)) };
}

/// Makes the collections that read their children from now on keep no table (or keep one again).
#[cfg(test)]
pub(crate) fn set_without_children_tables(value: bool) {
    WITHOUT_TABLES.with(|without| without.set(value));
}

/// How many children the update pass and the render walk left out by a table since the last call.
#[cfg(test)]
pub(crate) fn take_children_left_out() -> (usize, usize) {
    LEFT_OUT.with(|left_out| left_out.replace((0, 0)))
}

/// What a collection of many children keeps of each of them beside the visual, in the order of the
/// children, for the walks of the tree to leave children out without a look at the visual (not from
/// upstream; docs/porting/desktop-performance.md, section 9.48). A walk that visits every child of a
/// container of tens of thousands reads every one of them in each frame, for the few that changed
/// or that are in the dirty region.
#[derive(Default)]
struct ChildrenTable {
    /// Where each child is in the space of the holder (its `transformed_sub_tree_bounds`); `NOWHERE`
    /// for a child without bounds.
    bounds: Vec<LtrbRect>,
    /// Whether the update pass has something to do in the child or below it.
    dirty: Vec<bool>,
    /// Whether the table is that of the children. One that a child wrote to which is not where it
    /// says in the list is not, until the list is read again; neither is that of a few children.
    current: bool,
}

/// Server-side counterpart of the children collection of a visual.
pub struct ServerCompositionVisualCollection {
    object: ServerObject,
    props: ServerCompositionVisualCollectionProps,
    /// The items as visuals: what the tree walks iterate.
    typed: RefCell<Rc<Vec<Rc<ServerCompositionVisual>>>>,
    table: RefCell<ChildrenTable>,
}

impl ServerCompositionVisualCollection {
    pub fn new(compositor: &Rc<ServerCompositor>) -> Rc<ServerCompositionVisualCollection> {
        Rc::new_cyclic(|this: &Weak<ServerCompositionVisualCollection>| {
            let owner: Weak<dyn IAnimatedServerObject> = this.clone();
            ServerCompositionVisualCollection {
                object: ServerObject::new(compositor, owner),
                props: ServerCompositionVisualCollectionProps::new(),
                typed: RefCell::new(Rc::new(Vec::new())),
                table: RefCell::new(ChildrenTable::default()),
            }
        })
    }

    /// A snapshot of the items.
    pub fn list(&self) -> Rc<Vec<Rc<ServerCompositionVisual>>> {
        self.typed.borrow().clone()
    }

    pub fn count(&self) -> usize {
        self.typed.borrow().len()
    }

    /// Makes the table of the children that were read, when they are many.
    fn read_table(&self) {
        let children = self.typed.borrow();
        let mut table = self.table.borrow_mut();
        table.bounds.clear();
        table.dirty.clear();
        #[cfg(test)]
        let keeps = !WITHOUT_TABLES.with(|without| without.get());
        #[cfg(not(test))]
        let keeps = true;
        table.current = keeps && children.len() >= TABLE_FROM;
        if !table.current {
            table.bounds.shrink_to_fit();
            table.dirty.shrink_to_fit();
            return;
        }
        table.bounds.reserve_exact(children.len());
        table.dirty.reserve_exact(children.len());
        for (index, child) in children.iter().enumerate() {
            child.set_index_in_parent(index);
            table.bounds.push(child.transformed_sub_tree_bounds().unwrap_or(NOWHERE));
            table.dirty.push(child.update_pass_comes_here());
        }
    }

    /// Writes the row of `child` in the table. Nothing is written without a table, and a child
    /// that is not where it says ends the table.
    fn with_row(&self, child: &ServerCompositionVisual, write: impl FnOnce(&mut ChildrenTable, usize)) {
        let mut table = self.table.borrow_mut();
        if !table.current {
            return;
        }
        let index = child.index_in_parent();
        if self.typed.borrow().get(index).is_some_and(|at| std::ptr::eq(Rc::as_ptr(at), child)) {
            write(&mut table, index);
        } else {
            table.current = false;
        }
    }

    /// The update pass has something to do in `child` or below it.
    pub(super) fn child_is_dirty(&self, child: &ServerCompositionVisual) {
        self.with_row(child, |table, index| table.dirty[index] = true);
    }

    /// The update pass was at `child`: it has nothing left to do there, and `bounds` are those it
    /// computed for the child, when it did.
    pub(super) fn child_is_updated(&self, child: &ServerCompositionVisual, bounds: Option<Option<LtrbRect>>) {
        self.with_row(child, |table, index| {
            table.dirty[index] = false;
            if let Some(bounds) = bounds {
                table.bounds[index] = bounds.unwrap_or(NOWHERE);
            }
        });
    }

    /// The first child at or after `from` that the update pass has something to do in or below:
    /// `from` without a table, the number of children when there is none.
    pub(super) fn next_dirty_child(&self, from: usize) -> usize {
        let table = self.table.borrow();
        if !table.current {
            return from;
        }
        let next = table.dirty[from..].iter().position(|dirty| *dirty).map_or(table.dirty.len(), |at| from + at);
        debug_assert!(
            self.typed.borrow()[from..next].iter().all(|child| !child.update_pass_comes_here()),
            "a child the update pass has to come to is not marked in the table of the children"
        );
        #[cfg(test)]
        LEFT_OUT.with(|left_out| left_out.set((left_out.get().0 + next - from, left_out.get().1)));
        next
    }

    /// The bounds of all children united, in the space of the holder: what the update pass unites
    /// child by child when it visits every one. `None` for a few children, which have no table and
    /// are all visited.
    pub(super) fn bounds_of_children(&self) -> Option<Option<LtrbRect>> {
        let table = self.table.borrow();
        if !table.current {
            // A table that ended while the pass was in the children (none does, as the tree is
            // sent) may have left children out: the visuals have what the table had.
            let children = self.typed.borrow();
            return (children.len() >= TABLE_FROM).then(|| {
                children.iter().fold(None, |all, child| LtrbRect::full_union(all, child.transformed_sub_tree_bounds()))
            });
        }
        // A child that is nowhere changes nothing.
        let mut all = NOWHERE;
        for bounds in &table.bounds {
            all.left = all.left.min(bounds.left);
            all.top = all.top.min(bounds.top);
            all.right = all.right.max(bounds.right);
            all.bottom = all.bottom.max(bounds.bottom);
        }
        Some((all != NOWHERE).then_some(all))
    }

    /// Whether the collection has a table of its children.
    #[inline]
    pub(super) fn has_table(&self) -> bool {
        self.table.borrow().current
    }

    /// The first child at or after `from` whose bounds meet `clip` or `origin` (both in the space
    /// of the holder): `from` without a table, the number of children when there is none.
    pub(super) fn next_child_in(&self, from: usize, clip: LtrbRect, origin: LtrbRect) -> usize {
        let table = self.table.borrow();
        if !table.current {
            return from;
        }
        let meets = |bounds: &LtrbRect| clip.intersects(*bounds) || origin.intersects(*bounds);
        let next = table.bounds[from..].iter().position(meets).map_or(table.bounds.len(), |at| from + at);
        #[cfg(test)]
        LEFT_OUT.with(|left_out| left_out.set((left_out.get().0, left_out.get().1 + next - from)));
        next
    }
}

impl ServerCompositionVisualCollectionHooks for ServerCompositionVisualCollection {}

impl_animated_server_object!(ServerCompositionVisualCollection, object);

impl IServerObject for ServerCompositionVisualCollection {
    fn deserialize_changes_core(&self, reader: &mut BatchStreamReader<'_>, committed_at: Duration) {
        self.props.deserialize_changes_core(self, reader, committed_at);
        *self.typed.borrow_mut() = Rc::new(self.props.list().items::<ServerCompositionVisual>());
        self.read_table();
    }

    fn dispose(&self) {
        self.props.list().clear();
        *self.typed.borrow_mut() = Rc::new(Vec::new());
        self.read_table();
    }

    fn get_props(&self, type_id: TypeId) -> Option<&dyn Any> {
        self.props.find_props(type_id)
    }

    fn as_animated(self: Rc<Self>) -> Option<Rc<dyn IAnimatedServerObject>> {
        Some(self)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any_rc(self: Rc<Self>) -> Rc<dyn Any> {
        self
    }
}
