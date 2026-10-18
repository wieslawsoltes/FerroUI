//! `Models/TreeDataGrid/DragInfo.cs`: what an automatic drag and drop of rows carries
//! (`TreeDataGrid::auto_drag_drop_rows`): the source the rows are dragged from and their model
//! indexes.
//!
//! The reference keeps the source of its classic API and the core source apart; the port has one
//! data layer, so both are the core source.

use crate::presentation::TreeDataGridPresentation;
use ferroui_base::input::{DataFormat, DataFormatOf};
use ferroui_treedatagrid_core::{ITreeDataGridSource, IndexPath};
use std::any::Any;
use std::rc::Rc;

/// `DragInfo`.
pub struct DragInfo {
    source: Option<Rc<dyn ITreeDataGridSource>>,
    source_identity: Rc<dyn Any>,
    indexes: Vec<IndexPath>,
}

impl DragInfo {
    /// The data format that carries the drag token in a data transfer.
    pub fn data_format() -> DataFormatOf<String> {
        DataFormat::create_string_application_format("TreeDataGridDragInfo")
    }

    /// The drag of the rows at `indexes` of `source`.
    pub fn new(source: Rc<dyn ITreeDataGridSource>, indexes: Vec<IndexPath>) -> Self {
        let source_identity: Rc<dyn Any> = source.clone();
        Self { source: Some(source), source_identity, indexes }
    }

    /// The drag of the rows at `indexes` of the source `presentation` presents, identified by the
    /// presentation's identity. The source is `source` when the caller has it, otherwise the
    /// identity itself when it holds a shared source handle (`Rc<dyn ITreeDataGridSource>`).
    pub fn from_presentation(
        presentation: &dyn TreeDataGridPresentation,
        source: Option<Rc<dyn ITreeDataGridSource>>,
        indexes: Vec<IndexPath>,
    ) -> Self {
        let source_identity = presentation.source_identity();
        let source = source.or_else(|| source_identity.downcast_ref::<Rc<dyn ITreeDataGridSource>>().cloned());
        Self { source, source_identity, indexes }
    }

    /// The source the rows are dragged from.
    ///
    /// # Panics
    /// When the drag carries no source (`InvalidOperationException`).
    pub fn source(&self) -> Rc<dyn ITreeDataGridSource> {
        match &self.source {
            Some(source) => source.clone(),
            None => panic!("This drag originates from the Core API. Use Model to access its source."),
        }
    }

    /// The core source the rows are dragged from, if the drag carries one (`Model`).
    pub fn model(&self) -> Option<Rc<dyn ITreeDataGridSource>> {
        self.source.clone()
    }

    /// The identity of the dragged source, to tell whether a drop targets the same source.
    pub fn source_identity(&self) -> &Rc<dyn Any> {
        &self.source_identity
    }

    /// The model indexes of the dragged rows.
    pub fn indexes(&self) -> &[IndexPath] {
        &self.indexes
    }
}
