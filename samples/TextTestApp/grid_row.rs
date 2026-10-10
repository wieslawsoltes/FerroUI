//! Port of `GridRow.cs`: a grid that is one row of a table. Every child gets a column of its
//! own, whose width is shared with the same column of the other rows of the size scope.

use ferroui_base::collections::NotifyCollectionChangedEventArgs;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::{LayoutableImpl, VerticalAlignment};
use ferroui_base::{ferro_class, ferro_class_info, ferro_impl_classes, instantiate, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl};
use ferroui_controls::{ColumnDefinition, Control, ControlImpl, Grid, PanelImpl, PanelImplExt};

#[repr(C)]
pub struct GridRow {
    base: Grid,
}

ferro_class!(GridRow: Grid);
ferro_impl_classes!(
    GridRow: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);
ferro_class_info!(GridRow { new: GridRow::new });

impl PanelImpl for GridRow {
    fn children_changed(this: &Self, e: &NotifyCollectionChangedEventArgs<'_, Ref<Control>>) {
        Self::parent_children_changed(this, e);

        let children = this.children();
        let column_definitions = this.column_definitions();

        while children.count() > column_definitions.count() {
            let column_definition = ColumnDefinition::new();
            column_definition.set_shared_size_group(Some(&format!("c{}", column_definitions.count())));
            column_definitions.add(column_definition);
        }

        for i in 0..children.count() {
            let child = children.get(i);
            Grid::set_column(&child, i as i32);
            // Every child of a panel is a control, which is a layoutable.
            child.set_vertical_alignment(VerticalAlignment::Center);
        }
    }
}

impl GridRow {
    pub fn construct() -> Self {
        Self { base: Grid::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}
