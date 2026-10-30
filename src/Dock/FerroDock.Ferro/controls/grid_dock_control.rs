// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::model_projection;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{Interactive, InteractiveImpl};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::utilities::FormatError;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::{
    primitives::{
        TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
    },
    ColumnDefinitions, Control, ControlImpl, Grid, ItemsControl, RowDefinitions,
};
/// Original native grid dock, loading its model definitions into the items panel.
#[repr(C)]
pub struct GridDockControl {
    base: TemplatedControl,
}
ferro_class!(GridDockControl:TemplatedControl);
ferro_class_info!(GridDockControl{new:GridDockControl::new,markup:{attributes:[TemplatePart("PART_ItemsControl",type(Ref<ItemsControl>))]}});
ferro_impl_classes!(GridDockControl:FerroObjectImpl,StyledElementImpl,VisualImpl,LayoutableImpl,InteractiveImpl,InputElementImpl,ControlImpl);
impl TemplatedControlImpl for GridDockControl {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);
        if let Some(items) = e.name_scope().find_as::<ItemsControl>("PART_ItemsControl") {
            items.add_handler(Control::loaded_event(), |sender, _| {
                // Malformed application grid definitions fail at the same event
                // boundary as the source; the native parsers themselves return Result.
                Self::items_control_on_loaded(sender).unwrap_or_else(|error| panic!("{error}"));
            });
        }
    }
}
impl GridDockControl {
    /// Constructs this original source class part for inheritance.
    pub fn construct() -> Self {
        Self {
            base: TemplatedControl::construct(),
        }
    }
    /// Creates the original native grid dock container.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
    fn items_control_on_loaded(sender: &Interactive) -> Result<(), FormatError> {
        if let Some(items) = sender.to_ref().cast::<ItemsControl>() {
            if let Some(grid) = items
                .items_panel_root()
                .and_then(|panel| panel.cast::<Grid>())
            {
                if let Some(model) = model_projection::dockable(items.data_context().as_ref())
                    .and_then(|model| model.as_grid_dock())
                {
                    if let Some(columns) = model.column_definitions() {
                        grid.set_column_definitions(ColumnDefinitions::parse(&columns)?);
                    }
                    if let Some(rows) = model.row_definitions() {
                        grid.set_row_definitions(RowDefinitions::parse(&rows)?);
                    }
                }
            }
        }
        Ok(())
    }
}
