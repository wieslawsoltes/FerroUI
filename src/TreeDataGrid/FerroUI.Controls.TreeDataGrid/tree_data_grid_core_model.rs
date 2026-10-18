//! `TreeDataGrid.CoreModel.cs`: the framework-neutral source of the grid (`Model`), its
//! presentation options and the presentation of it, and the handle that carries a core source as
//! the value of a property ([`TreeDataGridSourceRef`]).
//!
//! The reference's `Model` (a core source) and `Source` (a source of its classic API) are two
//! entry points of one presentation in the port: both hold core sources, presented by the same
//! [`TreeDataGridSourcePresentation`](crate::presentation::TreeDataGridSourcePresentation).

use super::TreeDataGrid;
use crate::presentation::{create_presentation, PresentationRef};
use crate::selection::{register_tree_data_grid_model, register_tree_data_grid_source};
use crate::TreeDataGridSelectionChangedEventArgs;
use ferroui_base::data::core::ValueTypes;
use ferroui_treedatagrid_core::accessor::TreeModel;
use ferroui_treedatagrid_core::{FlatTreeDataGridSource, HierarchicalTreeDataGridSource, ITreeDataGridSource, ITreeDataGridSourceOf};
use std::ops::Deref;
use std::rc::Rc;

/// A shared core source as the value of a property of the grid (`Source`, `Model`): the core's
/// source trait object, compared by identity.
///
/// The conversions from the typed sources register the typed operations of their model type
/// ([`register_tree_data_grid_model`]), which the untyped presentation needs to create row and
/// cell selections and to follow the columns; a source handed over as an untyped
/// `Rc<dyn ITreeDataGridSource>` relies on its type having been registered.
#[derive(Clone)]
pub struct TreeDataGridSourceRef(Rc<dyn ITreeDataGridSource>);

impl TreeDataGridSourceRef {
    /// The handle of an untyped source.
    pub fn new(source: Rc<dyn ITreeDataGridSource>) -> Self {
        Self(source)
    }

    /// The handle of a typed source of models of `M`, registering the source and model types.
    pub fn of<M: TreeModel, S: ITreeDataGridSourceOf<M> + 'static>(source: Rc<S>) -> Self {
        register_tree_data_grid_source::<M, S>();
        Self(source as Rc<dyn ITreeDataGridSource>)
    }

    /// The source.
    pub fn source(&self) -> &Rc<dyn ITreeDataGridSource> {
        &self.0
    }

    /// The source as the concrete source type `T`.
    pub fn downcast<T: 'static>(&self) -> Option<Rc<T>> {
        ferroui_treedatagrid_core::downcast_source::<T>(&self.0)
    }

    /// Whether two handles hold the same source.
    pub fn ptr_eq(&self, other: &TreeDataGridSourceRef) -> bool {
        std::ptr::addr_eq(Rc::as_ptr(&self.0), Rc::as_ptr(&other.0))
    }

    /// Whether the handle holds `source`.
    pub fn is(&self, source: &Rc<dyn ITreeDataGridSource>) -> bool {
        std::ptr::addr_eq(Rc::as_ptr(&self.0), Rc::as_ptr(source))
    }

    /// Lets bindings deliver a value of the core's sources of models of `M` to a property of type
    /// `Option<TreeDataGridSourceRef>` (or `TreeDataGridSourceRef`) on the current thread, as
    /// `ItemsSource::register_binding_conversion` does for collections.
    pub fn register_binding_conversion<M: TreeModel>() {
        register_tree_data_grid_model::<M>();
        ValueTypes::register_conversion::<Rc<FlatTreeDataGridSource<M>>, TreeDataGridSourceRef>(|s| Some(s.clone().into()));
        ValueTypes::register_conversion::<Rc<FlatTreeDataGridSource<M>>, Option<TreeDataGridSourceRef>>(|s| {
            Some(Some(s.clone().into()))
        });
        ValueTypes::register_conversion::<Rc<HierarchicalTreeDataGridSource<M>>, TreeDataGridSourceRef>(|s| {
            Some(s.clone().into())
        });
        ValueTypes::register_conversion::<Rc<HierarchicalTreeDataGridSource<M>>, Option<TreeDataGridSourceRef>>(|s| {
            Some(Some(s.clone().into()))
        });
    }
}

impl PartialEq for TreeDataGridSourceRef {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other)
    }
}

impl std::fmt::Debug for TreeDataGridSourceRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TreeDataGridSourceRef({:p}, hierarchical = {})", Rc::as_ptr(&self.0) as *const (), self.0.is_hierarchical())
    }
}

impl Deref for TreeDataGridSourceRef {
    type Target = Rc<dyn ITreeDataGridSource>;

    fn deref(&self) -> &Rc<dyn ITreeDataGridSource> {
        &self.0
    }
}

impl From<Rc<dyn ITreeDataGridSource>> for TreeDataGridSourceRef {
    fn from(value: Rc<dyn ITreeDataGridSource>) -> Self {
        Self(value)
    }
}

impl<M: TreeModel> From<Rc<FlatTreeDataGridSource<M>>> for TreeDataGridSourceRef {
    fn from(value: Rc<FlatTreeDataGridSource<M>>) -> Self {
        Self::of::<M, FlatTreeDataGridSource<M>>(value)
    }
}

impl<M: TreeModel> From<Rc<HierarchicalTreeDataGridSource<M>>> for TreeDataGridSourceRef {
    fn from(value: Rc<HierarchicalTreeDataGridSource<M>>) -> Self {
        Self::of::<M, HierarchicalTreeDataGridSource<M>>(value)
    }
}

impl TreeDataGrid {
    /// The framework-neutral source (`Model`). The control owns view state, never this model.
    pub fn model(&self) -> Option<TreeDataGridSourceRef> {
        self.get_value(Self::model_property())
    }

    /// Sets [`model`](Self::model).
    pub fn set_model(&self, value: Option<TreeDataGridSourceRef>) {
        self.set_value(Self::model_property(), value)
    }

    /// The options of the presentation of the model (`PresentationOptions`).
    pub fn presentation_options(&self) -> Option<Rc<dyn crate::presentation::ITreeDataGridPresentationOptions>> {
        self.get_value(Self::presentation_options_property())
    }

    /// Sets [`presentation_options`](Self::presentation_options).
    pub fn set_presentation_options(&self, value: Option<Rc<dyn crate::presentation::ITreeDataGridPresentationOptions>>) {
        self.set_value(Self::presentation_options_property(), value)
    }

    /// The presentation of the source shown (`Presentation`).
    pub fn presentation(&self) -> Option<PresentationRef> {
        self.presentation.borrow().clone()
    }

    /// `UpdateCorePresentation`: a new presentation of the model, or the source shown when the
    /// model is cleared.
    pub(super) fn update_core_presentation(&self) {
        let Some(model) = self.model() else {
            self.update_active_source();
            return;
        };
        let old_source = self.source.borrow_mut().take();
        *self.explicit_source.borrow_mut() = None;
        if old_source.is_some() {
            self.raise_direct_property_changed(Self::source_property(), &old_source, &None);
        }
        let options = self.presentation_options();
        self.set_presentation(Some(create_presentation(model.source(), options.as_ref())));
        self.apply_selection_mode();
    }

    /// `ReleaseCorePresentation`: the presentation stops following its source while the control
    /// is detached. The reference suspends only the presentation of a model, its classic sources
    /// keeping no subscriptions of their own; the port's sources are all core sources, so the
    /// presentation of a `Source` is suspended too, and the control holds no subscription to its
    /// source while it is detached.
    pub(super) fn release_core_presentation(&self) {
        let presentation = self.presentation();
        if let Some(presentation) = presentation {
            presentation.suspend();
        }
    }

    /// `SetPresentation`.
    pub(super) fn set_presentation(&self, presentation: Option<PresentationRef>) {
        self.unsubscribe_source_events();
        let old = self.presentation.borrow().clone();
        if let (Some(old), Some(token)) = (&old, self.native_selection_token.take()) {
            old.remove_native_selection_changed(token);
        }
        *self.presentation.borrow_mut() = presentation.clone();
        if let Some(presentation) = &presentation {
            let weak = self.to_ref().downgrade();
            let token = presentation.add_native_selection_changed(Rc::new(move |e: &TreeDataGridSelectionChangedEventArgs| {
                if let Some(this) = weak.upgrade() {
                    this.on_native_selection_changed(e);
                }
            }));
            self.native_selection_token.set(Some(token));
        }
        self.set_columns(presentation.as_ref().map(|p| p.columns()));
        self.set_rows(presentation.as_ref().map(|p| p.rows()));
        if !self.is_attached_to_visual_tree.get() {
            if let Some(presentation) = &presentation {
                presentation.suspend();
            }
        }
        self.set_selection_interaction(presentation.as_ref().and_then(|p| p.selection_interaction()));
        self.subscribe_source_events();
        self.raise_direct_property_changed(Self::presentation_property(), &old, &presentation);
        if let Some(old) = old {
            let same = presentation.as_ref().is_some_and(|p| std::ptr::addr_eq(Rc::as_ptr(p), Rc::as_ptr(&old)));
            if !same {
                old.dispose();
            }
        }
    }
}
