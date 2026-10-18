//! `Primitives/TreeDataGridTextCell.cs`: a cell that shows the text of a text cell model and edits
//! it in a text box.

use super::{TreeDataGridCell, TreeDataGridCellImpl, TreeDataGridCellImplExt, TreeDataGridElementFactory};
use crate::models::CellRef;
use crate::selection::ITreeDataGridSelectionInteraction;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::media::{TextAlignment, TextTrimming, TextWrapping};
use ferroui_base::{
    ferro_class, ferro_property, instantiate, DirectProperty, FerroObjectImpl, FerroProperty, Ref, StyledElementImpl,
    VisualImpl,
};
use ferroui_controls::primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt};
use ferroui_controls::{ControlImpl, TextBox};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A cell that shows the text of an [`ITextCell`](crate::models::ITextCell) model.
#[repr(C)]
pub struct TreeDataGridTextCell {
    base: TreeDataGridCell,
    value: RefCell<Option<String>>,
    edit: RefCell<Option<Ref<TextBox>>>,
    model_value_changing: Cell<bool>,
    text_trimming: RefCell<Rc<dyn TextTrimming>>,
    text_wrapping: Cell<TextWrapping>,
    text_alignment: Cell<TextAlignment>,
}

ferro_class!(TreeDataGridTextCell: TreeDataGridCell);
ferroui_base::ferro_class_info!(TreeDataGridTextCell { new: TreeDataGridTextCell::new });
ferroui_base::ferro_impl_classes!(
    TreeDataGridTextCell: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

ferroui_base::ferro_properties! { impl TreeDataGridTextCell {
    ferro_property!(
        /// Defines the `TextTrimming` property.
        pub fn text_trimming_property() -> DirectProperty<TreeDataGridTextCell, Rc<dyn TextTrimming>> {
            FerroProperty::register_direct::<TreeDataGridTextCell, _>(
                "TextTrimming",
                |o| o.text_trimming(),
                None,
                <dyn TextTrimming>::character_ellipsis(),
            )
        }
    );

    ferro_property!(
        /// Defines the `TextWrapping` property.
        pub fn text_wrapping_property() -> DirectProperty<TreeDataGridTextCell, TextWrapping> {
            FerroProperty::register_direct::<TreeDataGridTextCell, _>(
                "TextWrapping",
                |o| o.text_wrapping(),
                None,
                TextWrapping::NoWrap,
            )
        }
    );

    ferro_property!(
        /// Defines the `Value` property.
        pub fn value_property() -> DirectProperty<TreeDataGridTextCell, Option<String>> {
            FerroProperty::register_direct::<TreeDataGridTextCell, _>(
                "Value",
                |o| o.value(),
                Some(|o, v| o.set_value(v)),
                None,
            )
        }
    );

    ferro_property!(
        /// Defines the `TextAlignment` property.
        pub fn text_alignment_property() -> DirectProperty<TreeDataGridTextCell, TextAlignment> {
            FerroProperty::register_direct::<TreeDataGridTextCell, _>(
                "TextAlignment",
                |o| o.text_alignment(),
                Some(|o, v| o.set_text_alignment(v)),
                TextAlignment::Left,
            )
        }
    );
} }

impl TemplatedControlImpl for TreeDataGridTextCell {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        let edit = e.name_scope().find_as::<TextBox>("PART_Edit");
        *this.edit.borrow_mut() = edit.clone();

        if let Some(edit) = edit {
            edit.select_all();
            edit.focus();
        }
    }
}

impl TreeDataGridCellImpl for TreeDataGridTextCell {
    fn realize(
        this: &Self,
        factory: &Rc<TreeDataGridElementFactory>,
        selection: Option<&dyn ITreeDataGridSelectionInteraction>,
        model: &CellRef,
        column_index: i32,
        row_index: i32,
    ) {
        let text_cell = model.as_text_cell();
        this.set_value(text_cell.and_then(|cell| cell.text()));
        this.set_text_trimming(text_cell.map_or_else(<dyn TextTrimming>::character_ellipsis, |cell| cell.text_trimming()));
        this.set_text_wrapping(text_cell.map_or(TextWrapping::NoWrap, |cell| cell.text_wrapping()));
        this.set_text_alignment(text_cell.map_or(TextAlignment::Left, |cell| cell.text_alignment()));
        Self::parent_realize(this, factory, selection, model, column_index, row_index);
        this.subscribe_to_model_changes();
    }

    fn unrealize(this: &Self) {
        this.unsubscribe_from_model_changes();
        Self::parent_unrealize(this);
    }

    fn update_value(this: &Self) {
        let text = this.model().and_then(|model| model.as_text_cell().and_then(|cell| cell.text()));
        this.set_value(text);
    }

    fn on_model_property_changed(this: &Self, property_name: &str) {
        Self::parent_on_model_property_changed(this, property_name);

        if property_name == "Value" {
            /// Clears the flag that keeps the value from being written back to the model.
            struct ModelValueChanging<'a>(&'a Cell<bool>);

            impl Drop for ModelValueChanging<'_> {
                fn drop(&mut self) {
                    self.0.set(false);
                }
            }

            this.model_value_changing.set(true);
            let _changing = ModelValueChanging(&this.model_value_changing);
            this.update_value();
        }
    }
}

impl TreeDataGridTextCell {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TreeDataGridCell::construct(),
            value: RefCell::new(None),
            edit: RefCell::new(None),
            model_value_changing: Cell::new(false),
            text_trimming: RefCell::new(<dyn TextTrimming>::character_ellipsis()),
            text_wrapping: Cell::new(TextWrapping::NoWrap),
            text_alignment: Cell::new(TextAlignment::Left),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// How the text is trimmed when it overflows the cell.
    pub fn text_trimming(&self) -> Rc<dyn TextTrimming> {
        self.text_trimming.borrow().clone()
    }

    pub fn set_text_trimming(&self, value: Rc<dyn TextTrimming>) {
        self.set_and_raise(Self::text_trimming_property(), &self.text_trimming, value);
    }

    /// How the text wraps.
    pub fn text_wrapping(&self) -> TextWrapping {
        self.text_wrapping.get()
    }

    pub fn set_text_wrapping(&self, value: TextWrapping) {
        self.set_and_raise_cell(Self::text_wrapping_property(), &self.text_wrapping, value);
    }

    /// The text of the cell. Setting it writes the text to the model.
    pub fn value(&self) -> Option<String> {
        self.value.borrow().clone()
    }

    pub fn set_value(&self, value: Option<String>) {
        if self.set_and_raise(Self::value_property(), &self.value, value) {
            if let Some(model) = self.model() {
                if let Some(cell) = model.as_text_cell() {
                    if !self.model_value_changing.get() {
                        cell.set_text(self.value());
                    }
                    self.raise_cell_value_changed();
                }
            }
        }
    }

    /// How the text is aligned.
    pub fn text_alignment(&self) -> TextAlignment {
        self.text_alignment.get()
    }

    pub fn set_text_alignment(&self, value: TextAlignment) {
        self.set_and_raise_cell(Self::text_alignment_property(), &self.text_alignment, value);
    }
}
