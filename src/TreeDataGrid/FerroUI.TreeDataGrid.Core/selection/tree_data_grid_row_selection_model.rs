//! `Selection/ITreeDataGridRowSelectionModel.cs` and `Selection/TreeDataGridRowSelectionModel.cs`:
//! the selection of rows of a source.

use super::{
    downcast_models, erase_models, ITreeDataGridSelection, ITreeSelectionModel, TreeSelectionModelBase,
    TreeSelectionModelHooks, TreeSelectionModelSelectionChangedEventArgsOf, UntypedModels,
};
use crate::accessor::TreeModel;
use crate::i_tree_data_grid_source::ITreeDataGridSourceOf;
use crate::model_collection::Models;
use ferroui_base::utilities::HandlerList;
use std::cell::Cell;
use std::rc::{Rc, Weak};

/// The selection of rows of a source (`TreeDataGridRowSelectionModel<TModel>`): a tree selection
/// model whose children are the source's child models, with `StateChanged`, raised once a change
/// of the source's collections has finished.
pub struct TreeDataGridRowSelectionModel<M: TreeModel> {
    base: Rc<TreeSelectionModelBase<M>>,
    source: Weak<dyn ITreeDataGridSourceOf<M>>,
    state_changed_pending: Cell<bool>,
    state_changed: HandlerList<dyn Fn()>,
}

impl<M: TreeModel> TreeDataGridRowSelectionModel<M> {
    /// The selection of the rows of `source`.
    pub fn new(source: &Rc<dyn ITreeDataGridSourceOf<M>>) -> Rc<Self> {
        let this = Rc::new_cyclic(|weak_self: &Weak<Self>| {
            let base = TreeSelectionModelBase::with_source(Some(source.items()));
            base.set_hooks(weak_self.clone() as Weak<dyn TreeSelectionModelHooks<M>>);
            Self {
                base,
                source: Rc::downgrade(source),
                state_changed_pending: Cell::new(false),
                state_changed: HandlerList::new(),
            }
        });
        let weak = Rc::downgrade(&this);
        let notify = move || {
            if let Some(this) = weak.upgrade() {
                this.notify_state_changed();
            }
        };
        let n = notify.clone();
        this.base.add_selection_changed(Rc::new(move |_| n()));
        let n = notify.clone();
        this.base.add_indexes_changed(Rc::new(move |_| n()));
        this.base.add_source_reset(Rc::new(move |_| notify()));
        this
    }

    /// The tree selection model (selected indexes and items, anchors, batches, events).
    pub fn base(&self) -> &Rc<TreeSelectionModelBase<M>> {
        &self.base
    }

    /// `StateChanged`: the selection, its indexes or its source changed.
    pub fn add_state_changed(&self, handler: Rc<dyn Fn()>) -> u64 {
        self.state_changed.add(handler)
    }

    pub fn remove_state_changed(&self, token: u64) {
        self.state_changed.remove(token);
    }

    /// The typed `SelectionChanged`.
    pub fn add_selection_changed(&self, handler: Rc<dyn Fn(&TreeSelectionModelSelectionChangedEventArgsOf<M>)>) -> u64 {
        self.base.add_selection_changed(handler)
    }

    pub fn remove_selection_changed(&self, token: u64) {
        self.base.remove_selection_changed(token);
    }

    /// The source of the selection, typed.
    pub fn typed_source(&self) -> Option<Models<M>> {
        self.base.source()
    }

    fn notify_state_changed(&self) {
        if self.base.is_source_collection_changing() {
            self.state_changed_pending.set(true);
        } else {
            self.raise_state_changed();
        }
    }

    fn raise_state_changed(&self) {
        for (_, handler) in self.state_changed.snapshot().iter() {
            handler();
        }
    }
}

impl<M: TreeModel> TreeSelectionModelHooks<M> for TreeDataGridRowSelectionModel<M> {
    fn get_children(&self, _model: &TreeSelectionModelBase<M>, node: &Rc<M>) -> Option<Models<M>> {
        self.source.upgrade().and_then(|source| source.get_model_children(node))
    }

    fn on_source_collection_change_finished(&self, _model: &TreeSelectionModelBase<M>) {
        if !self.state_changed_pending.replace(false) {
            return;
        }
        self.raise_state_changed();
    }
}

impl<M: TreeModel> ITreeDataGridSelection for TreeDataGridRowSelectionModel<M> {
    fn source(&self) -> Option<UntypedModels> {
        self.base.source().map(erase_models)
    }

    fn set_source(&self, value: Option<UntypedModels>) {
        let typed = value.map(|value| downcast_models::<M>(&value).expect("a source of the model type of the selection"));
        self.base.set_source(typed);
    }

    fn row_selection_model(&self) -> Option<Rc<dyn ITreeSelectionModel>> {
        Some(self.base.clone() as Rc<dyn ITreeSelectionModel>)
    }
}
