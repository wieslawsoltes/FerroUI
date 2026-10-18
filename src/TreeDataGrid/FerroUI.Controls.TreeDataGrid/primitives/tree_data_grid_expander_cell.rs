//! `Primitives/TreeDataGridExpanderCell.cs`: the cell of an expander column: the indent of its row,
//! the expander, and the cell of the inner column it hosts.

use super::{TreeDataGridCell, TreeDataGridCellImpl, TreeDataGridCellImplExt, TreeDataGridElementFactory};
use crate::models::CellRef;
use crate::selection::ITreeDataGridSelectionInteraction;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{
    ferro_class, ferro_property, instantiate, DirectProperty, FerroObjectImpl, FerroProperty, Ref, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::primitives::{TemplateAppliedEventArgs, TemplatedControlImpl};
use ferroui_controls::{ControlImpl, Decorator};
#[allow(unused_imports)]
use ferroui_treedatagrid_core::models::IExpander;
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The cell of an expander column: an [`IExpanderCellPresentation`](crate::models::IExpanderCellPresentation)
/// model.
#[repr(C)]
pub struct TreeDataGridExpanderCell {
    base: TreeDataGridCell,
    content_container: RefCell<Option<Ref<Decorator>>>,
    content_type: Cell<Option<TypeId>>,
    factory: RefCell<Option<Rc<TreeDataGridElementFactory>>>,
    indent: Cell<i32>,
    is_expanded: Cell<bool>,
    /// The expander cell model, and the token of the handler of its property changes.
    model: RefCell<Option<(CellRef, Option<u64>)>>,
    show_expander: Cell<bool>,
}

ferro_class!(TreeDataGridExpanderCell: TreeDataGridCell);
ferroui_base::ferro_class_info!(TreeDataGridExpanderCell { new: TreeDataGridExpanderCell::new });
ferroui_base::ferro_impl_classes!(
    TreeDataGridExpanderCell: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

ferroui_base::ferro_properties! { impl TreeDataGridExpanderCell {
    ferro_property!(
        /// Defines the `Indent` property.
        pub fn indent_property() -> DirectProperty<TreeDataGridExpanderCell, i32> {
            FerroProperty::register_direct::<TreeDataGridExpanderCell, _>("Indent", |o| o.indent(), None, 0)
        }
    );

    ferro_property!(
        /// Defines the `IsExpanded` property.
        pub fn is_expanded_property() -> DirectProperty<TreeDataGridExpanderCell, bool> {
            FerroProperty::register_direct::<TreeDataGridExpanderCell, _>(
                "IsExpanded",
                |o| o.is_expanded(),
                Some(|o, v| o.set_is_expanded(v)),
                false,
            )
        }
    );

    ferro_property!(
        /// Defines the `ShowExpander` property.
        pub fn show_expander_property() -> DirectProperty<TreeDataGridExpanderCell, bool> {
            FerroProperty::register_direct::<TreeDataGridExpanderCell, _>(
                "ShowExpander",
                |o| o.show_expander(),
                None,
                false,
            )
        }
    );
} }

impl TemplatedControlImpl for TreeDataGridExpanderCell {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        *this.content_container.borrow_mut() = e.name_scope().find_as::<Decorator>("PART_Content");
        let factory = this.factory.borrow().clone();
        if let Some(factory) = factory {
            this.update_content(&factory);
        }
    }
}

impl TreeDataGridCellImpl for TreeDataGridExpanderCell {
    fn realize(
        this: &Self,
        factory: &Rc<TreeDataGridElementFactory>,
        selection: Option<&dyn ITreeDataGridSelectionInteraction>,
        model: &CellRef,
        column_index: i32,
        row_index: i32,
    ) {
        if this.model.borrow().is_some() {
            panic!("Cell is already realized.");
        }

        match model.as_expander_cell() {
            Some(expander_model) => {
                *this.factory.borrow_mut() = Some(factory.clone());
                this.set_indent(expander_model.row().as_indented().map_or(0, |row| row.indent()));
                this.set_show_expander(expander_model.show_expander());

                // We can't go via `set_is_expanded` here as that contains the implementation for
                // changing the expanded state by user action; it signals to the model that the
                // state is changed but here we need to update our state from the model.
                this.set_and_raise_cell(Self::is_expanded_property(), &this.is_expanded, expander_model.is_expanded());

                let token = model.as_notify_property_changed().map(|notifier| {
                    let weak = this.to_ref().downgrade();
                    notifier.property_changed().add(Rc::new(move |property_name: &str| {
                        if let Some(this) = weak.upgrade() {
                            this.model_property_changed(property_name);
                        }
                    }))
                });
                *this.model.borrow_mut() = Some((model.clone(), token));
            }
            None => panic!("Invalid cell model."),
        }

        Self::parent_realize(this, factory, selection, model, column_index, row_index);
        this.update_content(factory);
    }

    fn unrealize(this: &Self) {
        let model = this.model.borrow_mut().take();
        if let Some((model, Some(token))) = model {
            if let Some(notifier) = model.as_notify_property_changed() {
                notifier.property_changed().remove(token);
            }
        }

        let factory = this.factory.borrow().clone();
        if let Some(factory) = factory {
            this.update_content(&factory);
        }
        Self::parent_unrealize(this);
    }
}

impl TreeDataGridExpanderCell {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TreeDataGridCell::construct(),
            content_container: RefCell::new(None),
            content_type: Cell::new(None),
            factory: RefCell::new(None),
            indent: Cell::new(0),
            is_expanded: Cell::new(false),
            model: RefCell::new(None),
            show_expander: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The indent level of the cell's row.
    pub fn indent(&self) -> i32 {
        self.indent.get()
    }

    fn set_indent(&self, value: i32) {
        self.set_and_raise_cell(Self::indent_property(), &self.indent, value);
    }

    /// Whether the row is expanded. Setting it expands or collapses the row through the model.
    pub fn is_expanded(&self) -> bool {
        self.is_expanded.get()
    }

    pub fn set_is_expanded(&self, value: bool) {
        let model = self.expander_model();
        if let Some(model) = model {
            if let Some(expander) = model.as_expander_cell() {
                expander.set_is_expanded(value);
            }
        }
    }

    /// Whether the expander is shown.
    pub fn show_expander(&self) -> bool {
        self.show_expander.get()
    }

    fn set_show_expander(&self, value: bool) {
        self.set_and_raise_cell(Self::show_expander_property(), &self.show_expander, value);
    }

    fn expander_model(&self) -> Option<CellRef> {
        self.model.borrow().as_ref().map(|(model, _)| model.clone())
    }

    fn update_content(&self, factory: &Rc<TreeDataGridElementFactory>) {
        let Some(content_container) = self.content_container.borrow().clone() else {
            return;
        };

        let inner_model = self.expander_model().and_then(|model| model.as_expander_cell().and_then(|cell| cell.content()));

        if let Some(inner_model) = inner_model {
            let content_type = inner_model.as_any().type_id();

            if Some(content_type) != self.content_type.get() {
                let element = factory.get_or_create_element(&inner_model as &dyn Any, self);
                element.set_is_visible(true);
                content_container.set_child(element);
                self.content_type.set(Some(content_type));
            }

            if let Some(inner_cell) = content_container.child().and_then(|child| child.cast::<TreeDataGridCell>()) {
                inner_cell.realize(factory, None, &inner_model, self.column_index(), self.row_index());
            }
        } else if let Some(inner_cell) = content_container.child().and_then(|child| child.cast::<TreeDataGridCell>()) {
            inner_cell.unrealize();
        }
    }

    fn model_property_changed(&self, property_name: &str) {
        let Some(model) = self.expander_model() else { return };
        let Some(expander) = model.as_expander_cell() else { return };

        if property_name == "IsExpanded" {
            self.set_and_raise_cell(Self::is_expanded_property(), &self.is_expanded, expander.is_expanded());
        }
        if property_name == "ShowExpander" {
            self.set_show_expander(expander.show_expander());
        }
    }
}
