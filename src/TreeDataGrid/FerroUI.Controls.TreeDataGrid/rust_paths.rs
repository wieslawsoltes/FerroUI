//! The public Rust paths of the registered types of this crate, by the shortest path another crate
//! names it by; the emitter of Rust source of the markup compiler reads them.

ferroui_base::ferro_rust_paths! {
    classes: [
        crate::TreeDataGrid,
        crate::TreeDataGridBindingProbe,
        crate::automation::peers::TreeDataGridAutomationPeer,
        crate::automation::peers::TreeDataGridCellAutomationPeer,
        crate::automation::peers::TreeDataGridCheckBoxCellAutomationPeer,
        crate::automation::peers::TreeDataGridColumnHeaderAutomationPeer,
        crate::automation::peers::TreeDataGridColumnHeadersPresenterAutomationPeer,
        crate::automation::peers::TreeDataGridRowAutomationPeer,
        crate::primitives::TreeDataGridCell,
        crate::primitives::TreeDataGridCellsPresenter,
        crate::primitives::TreeDataGridCheckBoxCell,
        crate::primitives::TreeDataGridColumnHeader,
        crate::primitives::TreeDataGridColumnHeadersPresenter,
        crate::primitives::TreeDataGridExpanderCell,
        crate::primitives::TreeDataGridRow,
        crate::primitives::TreeDataGridRowsPresenter,
        crate::primitives::TreeDataGridTemplateCell,
        crate::primitives::TreeDataGridTextCell,
    ],
    types: [
        crate::converters::IndentConverter,
        crate::models::BeginEditGestures,
        crate::TreeDataGridCheckBoxColumn,
        crate::TreeDataGridColumns,
        crate::TreeDataGridHierarchicalExpanderColumn,
        crate::TreeDataGridRowDragEventArgs,
        crate::TreeDataGridRowDragStartedEventArgs,
        crate::TreeDataGridRowHeaderColumn,
        crate::TreeDataGridSelectionChangedEventArgs,
        crate::TreeDataGridSelectionMode,
        crate::TreeDataGridTemplateColumn,
        crate::TreeDataGridTextColumn,
    ],
    contracts: [
        crate::TreeDataGridColumn,
    ],
    generics: [
    ],
    generic_contracts: [
    ],
}
