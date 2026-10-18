//! `TreeDataGridAutomationPeerTests.cs`: the peer of the grid, its control type and the selection
//! provider it exposes for row selections (the source's own and a custom one), not for cell
//! selections.

use super::automation_peer_test_helper::{create_flat_target, create_peer, FlatRowModel, FlatTargetOptions};
use super::support::{run_jobs, start_app};
use crate::automation::peers::{TreeDataGridAutomationPeer, TreeDataGridRowAutomationPeer};
use crate::TreeDataGrid;
use ferroui_base::StaticType;
use ferroui_controls::automation::peers::AutomationControlType;
use ferroui_controls::automation::provider::ISelectionProvider;
use ferroui_treedatagrid_core::selection::{
    downcast_models, erase_models, ITreeDataGridSelection, ITreeSelectionModel, TreeSelectionModelBase, UntypedModels,
};
use ferroui_treedatagrid_core::{ITreeDataGridSource, IndexPath, Models, TreeModel};
use std::rc::Rc;

#[test]
fn should_create_data_grid_peer() {
    let _app = start_app();
    let grid = TreeDataGrid::new();
    let peer = create_peer::<TreeDataGridAutomationPeer>(&grid);

    assert_eq!(AutomationControlType::DataGrid, peer.get_automation_control_type());
    assert!(peer.is_control_element());
    assert!(peer.is_content_element());
}

#[test]
fn should_expose_selection_provider_for_row_selection() {
    let _app = start_app();
    let (target, _source, _items, root) =
        create_flat_target(FlatTargetOptions { single_select: false, ..FlatTargetOptions::default() });
    let peer = create_peer::<TreeDataGridAutomationPeer>(&target);
    let selection_provider = peer.get_provider::<dyn ISelectionProvider>().expect("a selection provider");

    assert!(selection_provider.can_select_multiple());
    assert!(!selection_provider.is_selection_required());

    root.close();
}

#[test]
fn should_not_expose_selection_provider_for_cell_selection() {
    let _app = start_app();
    let (target, _source, _items, root) = create_flat_target(FlatTargetOptions {
        single_select: false,
        use_cell_selection: true,
        ..FlatTargetOptions::default()
    });
    let peer = create_peer::<TreeDataGridAutomationPeer>(&target);

    assert!(peer.get_provider::<dyn ISelectionProvider>().is_none());

    root.close();
}

#[test]
fn should_return_selected_row_peers_for_realized_selected_rows() {
    let _app = start_app();
    let (target, source, _items, root) =
        create_flat_target(FlatTargetOptions { single_select: true, ..FlatTargetOptions::default() });
    let peer = create_peer::<TreeDataGridAutomationPeer>(&target);
    let selection_provider = peer.get_provider::<dyn ISelectionProvider>().expect("a selection provider");

    let model_index = source.rows().row_index_to_model_index(0);
    assert_ne!(IndexPath::default(), model_index);

    let selection_model = target.row_selection().expect("a row selection model");
    selection_model.select(&model_index);
    root.update_layout();
    run_jobs();

    let selected_peers = selection_provider.get_selection();

    assert_eq!(1, selected_peers.len());
    assert!(std::ptr::eq(selected_peers[0].get_type(), <TreeDataGridRowAutomationPeer as StaticType>::TYPE));

    root.close();
}

#[test]
fn should_expose_selection_provider_for_custom_legacy_row_selection() {
    let _app = start_app();
    let (target, source, items, root) =
        create_flat_target(FlatTargetOptions { single_select: true, ..FlatTargetOptions::default() });
    let selection = NonInteractiveRowSelectionModel::<FlatRowModel>::new(items.clone() as Models<FlatRowModel>);
    source.set_selection(Some(selection.clone() as Rc<dyn ITreeDataGridSelection>));
    let peer = create_peer::<TreeDataGridAutomationPeer>(&target);
    let selection_provider = peer.get_provider::<dyn ISelectionProvider>().expect("a selection provider");

    selection.select(&source.rows().row_index_to_model_index(0));
    root.update_layout();
    run_jobs();

    assert_eq!(1, selection_provider.get_selection().len());

    root.close();
}

/// `NonInteractiveRowSelectionModel<T>`: a tree selection model of a flat collection that is not
/// the source's row selection type, so that the grid presents it through its tree selection model
/// (`ITreeDataGridRowSelectionModel<T>` of the reference). The model has no hooks, so its nodes
/// have no children (`GetChildren(node) => null`).
struct NonInteractiveRowSelectionModel<M: TreeModel> {
    base: Rc<TreeSelectionModelBase<M>>,
}

impl<M: TreeModel> NonInteractiveRowSelectionModel<M> {
    fn new(source: Models<M>) -> Rc<Self> {
        Rc::new(Self { base: TreeSelectionModelBase::with_source(Some(source)) })
    }

    fn select(&self, index: &IndexPath) {
        self.base.select(index);
    }
}

impl<M: TreeModel> ITreeDataGridSelection for NonInteractiveRowSelectionModel<M> {
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
