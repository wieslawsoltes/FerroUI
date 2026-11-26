//! Shared Studio and editor boundary for authored, nonvisual column definitions.
use crate::{DesignerSession, NodeId, PreviewStatus};
use ferroui_controls::TableView;
use ferroui_controls_treedatagrid::TreeDataGrid;
use ferroui_designer_rsx::{RsxColumnDeclaration, RsxProjection, RsxSourceEdit};
use ferroui_designer_tools::{
    ColumnChange, TableColumns, TableKind, nearest_table, plan_column_change, table_columns,
};

pub(crate) struct ColumnAuthoring {
    pub(crate) model: TableColumns,
    pub(crate) source: Option<RsxColumnDeclaration>,
}

impl DesignerSession {
    pub fn authored_columns(&self, node: NodeId) -> Result<Option<TableColumns>, String> {
        Ok(self
            .column_authoring(node, None)?
            .map(|columns| columns.model))
    }

    pub(crate) fn column_authoring(
        &self,
        node: NodeId,
        projection: Option<&RsxProjection<'_>>,
    ) -> Result<Option<ColumnAuthoring>, String> {
        let Some(table) = nearest_table(self.document(), node) else {
            return Ok(None);
        };
        let authored = if projection.is_none() {
            self.rust
                .as_ref()
                .map(|rust| rust.projection())
                .transpose()?
        } else {
            None
        };
        let mut model = table_columns(self.document(), table).map_err(|error| error.to_string())?;
        let source = projection
            .or(authored.as_ref())
            .and_then(|projection| projection.map_column_declaration(self.document(), &mut model));
        let reason = if !matches!(self.preview_status(), PreviewStatus::Ready { revision } if *revision == self.document().revision())
        {
            Some("Wait for the current native preview before editing columns.")
        } else if !self.can_manipulate() || self.ensure_layer_manipulable(table).is_err() {
            Some("Open an editable table and unlock its layer to edit columns.")
        } else if !self.has_authored_columns(&model) {
            Some(
                "The current native table supplies its columns at runtime. Open their source to edit them.",
            )
        } else {
            None
        };
        if let Some(reason) = reason {
            model.reason.get_or_insert_with(|| reason.to_owned());
        }
        Ok(Some(ColumnAuthoring { model, source }))
    }

    pub fn edit_authored_columns(
        &mut self,
        table: NodeId,
        change: ColumnChange,
    ) -> Result<(), String> {
        self.ensure_writable().map_err(|error| error.to_string())?;
        if let Some(rust) = &self.rust {
            let edit = self.rust_column_plan(table, &rust.projection()?, &change)?;
            return self.apply_rust_column_plan(change.label(), edit);
        }
        let model = self
            .authored_columns(table)?
            .filter(|model| model.table == table)
            .ok_or("The selected table changed")?;
        if let Some(reason) = model.reason {
            return Err(reason);
        }
        let edits = plan_column_change(self.document(), table, &change)
            .map_err(|error| error.to_string())?;
        if !edits.is_empty() {
            self.apply(change.label(), edits)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    pub(crate) fn rust_column_plan(
        &self,
        table: NodeId,
        projection: &RsxProjection<'_>,
        change: &ColumnChange,
    ) -> Result<RsxSourceEdit, String> {
        self.ensure_writable().map_err(|error| error.to_string())?;
        let columns = self
            .column_authoring(table, Some(projection))?
            .filter(|columns| columns.model.table == table)
            .ok_or("The selected table changed")?;
        projection.plan_columns(self.document(), &columns.model, change)
    }

    pub(crate) fn apply_rust_column_plan(&mut self, label: &str, edit: RsxSourceEdit) -> Result<(), String> {
        if !edit.patches.is_empty() {
            self.commit_rust_declaration(label, edit).map_err(|error| error.to_string())?;
            self.changed(true);
        }
        Ok(())
    }

    fn has_authored_columns(&self, model: &TableColumns) -> bool {
        let Some(visual) = self.preview.visual(model.table) else {
            return false;
        };
        match model.kind {
            TableKind::TableView => visual
                .cast::<TableView>()
                .is_some_and(|table| table.columns().count() == model.columns.len()),
            TableKind::TreeDataGrid => visual.cast::<TreeDataGrid>().is_some_and(|table| {
                table.model().is_none()
                    && table.source() == table.generated_source()
                    && table.column_definitions().count() == model.columns.len()
            }),
        }
    }
}
