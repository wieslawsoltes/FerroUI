//! `Primitives/TreeDataGridTemplateCell.cs`: a cell that shows the value of a template cell model
//! with the cell template of its column, and edits it with the column's editing template.
//!
//! The data context of a realized template cell is its cell model (the reference's
//! `DataContext = model`). A cell model is a `Rc<dyn ICell>`, which is not an untyped value itself,
//! so the data context holds it in a [`TemplateCellDataContext`].

use super::{TreeDataGridCell, TreeDataGridCellImpl, TreeDataGridCellImplExt, TreeDataGridElementFactory};
use crate::models::{CellRef, TemplateCell};
use crate::selection::ITreeDataGridSelectionInteraction;
use ferroui_base::input::{FocusChangedEventArgs, InputElement, InputElementImpl, InputElementImplExt};
use ferroui_base::interactivity::{InteractiveImpl, RoutedEventHandlerToken};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::logical_tree::LogicalTreeAttachmentEventArgs;
use ferroui_base::{
    ferro_class, ferro_property, instantiate, BoxedValue, DirectProperty, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledElementImplExt, VisualImpl, VisualImplExt, VisualTreeAttachmentEventArgs,
};
use ferroui_controls::presenters::ContentPresenter;
use ferroui_controls::primitives::{TemplateAppliedEventArgs, TemplatedControlImpl, TemplatedControlImplExt};
use ferroui_controls::templates::{IDataTemplate, IRecyclingDataTemplate, ITemplateWithParam};
use ferroui_controls::{Control, ControlImpl, TopLevel};
use std::cell::RefCell;
use std::rc::Rc;

/// The data context of a realized [`TreeDataGridTemplateCell`]: its cell model.
#[derive(Clone)]
pub struct TemplateCellDataContext(pub CellRef);

impl PartialEq for TemplateCellDataContext {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

/// A cell that shows a [`TemplateCell`] model.
#[repr(C)]
pub struct TreeDataGridTemplateCell {
    base: TreeDataGridCell,
    content: RefCell<Option<BoxedValue>>,
    content_template: RefCell<Option<Rc<dyn IDataTemplate>>>,
    editing_template: RefCell<Option<Rc<dyn IDataTemplate>>>,
    /// The editing content presenter of the template, and the token of its `LostFocus` handler.
    editing_content_presenter: RefCell<Option<(Ref<ContentPresenter>, RoutedEventHandlerToken)>>,
    source_content_template: RefCell<Option<Rc<dyn IDataTemplate>>>,
}

ferro_class!(TreeDataGridTemplateCell: TreeDataGridCell);
ferroui_base::ferro_class_info!(TreeDataGridTemplateCell { new: TreeDataGridTemplateCell::new });
ferroui_base::ferro_impl_classes!(
    TreeDataGridTemplateCell: FerroObjectImpl,
    LayoutableImpl,
    InteractiveImpl,
    ControlImpl
);

ferroui_base::ferro_properties! { impl TreeDataGridTemplateCell {
    ferro_property!(
        /// Defines the `Content` property.
        pub fn content_property() -> DirectProperty<TreeDataGridTemplateCell, Option<BoxedValue>> {
            FerroProperty::register_direct::<TreeDataGridTemplateCell, _>("Content", |o| o.content(), None, None)
        }
    );

    ferro_property!(
        /// Defines the `ContentTemplate` property.
        pub fn content_template_property() -> DirectProperty<TreeDataGridTemplateCell, Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register_direct::<TreeDataGridTemplateCell, _>(
                "ContentTemplate",
                |o| o.content_template(),
                None,
                None,
            )
        }
    );

    ferro_property!(
        /// Defines the `EditingTemplate` property.
        pub fn editing_template_property() -> DirectProperty<TreeDataGridTemplateCell, Option<Rc<dyn IDataTemplate>>> {
            FerroProperty::register_direct::<TreeDataGridTemplateCell, _>(
                "EditingTemplate",
                |o| o.editing_template(),
                None,
                None,
            )
        }
    );
} }

impl StyledElementImpl for TreeDataGridTemplateCell {
    fn on_attached_to_logical_tree(this: &Self, e: &LogicalTreeAttachmentEventArgs) {
        Self::parent_on_attached_to_logical_tree(this, e);

        // A detached cell can be recycled into a different template column while retaining its
        // previous ContentTemplate, so always resolve the current model's templates here.
        if let Some(model) = this.template_cell_model() {
            if let Some(cell) = model.as_any().downcast_ref::<TemplateCell>() {
                this.apply_cell_templates(cell);
            }
        }
    }

    fn on_data_context_changed(this: &Self) {
        Self::parent_on_data_context_changed(this);

        // If the data context is null, we're unrealized. Don't clear the content template for
        // unrealized cells because this will mean that when the cell is realized again the
        // template will need to be rebuilt, slowing everything down.
        let model = this.template_cell_model();
        match model.as_ref().and_then(|model| model.as_any().downcast_ref::<TemplateCell>()) {
            Some(cell) => {
                this.set_content(cell.value());

                if this.is_attached_to_logical_tree() {
                    this.apply_cell_templates(cell);
                }
            }
            None => this.clear_content_if_detached(),
        }
    }
}

impl VisualImpl for TreeDataGridTemplateCell {
    fn on_detached_from_visual_tree(this: &Self, e: &VisualTreeAttachmentEventArgs) {
        Self::parent_on_detached_from_visual_tree(this, e);

        // Recycled cells are normally hidden and kept parented so their control subtree and
        // applied styles can be reused. Clear the content only when the cell itself has been
        // removed from both trees; ancestor detachment doesn't change either parent.
        this.clear_content_if_detached();
    }
}

impl InputElementImpl for TreeDataGridTemplateCell {
    fn on_lost_focus(this: &Self, e: &FocusChangedEventArgs) {
        if this.end_edit_if_focus_lost() {
            Self::parent_on_lost_focus(this, e);
        }
    }
}

impl TemplatedControlImpl for TreeDataGridTemplateCell {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        Self::parent_on_apply_template(this, e);

        let previous = this.editing_content_presenter.borrow_mut().take();
        if let Some((presenter, token)) = previous {
            presenter.remove_handler(InputElement::lost_focus_event(), token);
        }

        let presenter = e.name_scope().find_as::<ContentPresenter>("PART_EditingContentPresenter");

        if let Some(presenter) = presenter {
            presenter.update_child();

            let focus = presenter
                .get_visual_descendants()
                .find_map(|visual| visual.cast::<InputElement>().filter(|element| element.focusable()));
            if let Some(focus) = focus {
                focus.focus();
            }

            let weak = this.to_ref().downgrade();
            let token = presenter.add_handler(InputElement::lost_focus_event(), move |_, _| {
                if let Some(this) = weak.upgrade() {
                    this.end_edit_if_focus_lost();
                }
            });
            *this.editing_content_presenter.borrow_mut() = Some((presenter, token));
        }
    }
}

impl TreeDataGridCellImpl for TreeDataGridTemplateCell {
    fn realize(
        this: &Self,
        factory: &Rc<TreeDataGridElementFactory>,
        selection: Option<&dyn ITreeDataGridSelectionInteraction>,
        model: &CellRef,
        column_index: i32,
        row_index: i32,
    ) {
        this.set_data_context(Some(Rc::new(TemplateCellDataContext(model.clone())) as BoxedValue));
        Self::parent_realize(this, factory, selection, model, column_index, row_index);
    }

    fn unrealize(this: &Self) {
        this.set_data_context(None);
        Self::parent_unrealize(this);
    }
}

impl TreeDataGridTemplateCell {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: TreeDataGridCell::construct(),
            content: RefCell::new(None),
            content_template: RefCell::new(None),
            editing_template: RefCell::new(None),
            editing_content_presenter: RefCell::new(None),
            source_content_template: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// The value of the cell model, the content the cell template is applied to.
    pub fn content(&self) -> Option<BoxedValue> {
        self.content.borrow().clone()
    }

    fn set_content(&self, value: Option<BoxedValue>) {
        if self.set_and_raise(Self::content_property(), &self.content, value) {
            self.raise_cell_value_changed();
        }
    }

    /// The template that shows the content.
    pub fn content_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.content_template.borrow().clone()
    }

    pub fn set_content_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_and_raise(Self::content_template_property(), &self.content_template, value);
    }

    /// The template that edits the content.
    pub fn editing_template(&self) -> Option<Rc<dyn IDataTemplate>> {
        self.editing_template.borrow().clone()
    }

    pub fn set_editing_template(&self, value: Option<Rc<dyn IDataTemplate>>) {
        self.set_and_raise(Self::editing_template_property(), &self.editing_template, value);
    }

    /// Clears the content of an unrealized cell (`FinalizeUnrealize`).
    pub fn finalize_unrealize(&self) {
        if self.data_context().is_none() {
            self.set_content(None);
        }
    }

    /// The cell model of the data context.
    fn template_cell_model(&self) -> Option<CellRef> {
        self.data_context()
            .and_then(|data_context| data_context.downcast_ref::<TemplateCellDataContext>().map(|context| context.0.clone()))
    }

    fn end_edit_if_focus_lost(&self) -> bool {
        let new_focus = TopLevel::get_top_level(Some(self))
            .and_then(|top_level| top_level.focus_manager().get_focused_element())
            .and_then(|focused| focused.cast::<Control>());

        if let Some(new_focus) = new_focus {
            if !self.is_descendent(&new_focus) {
                self.end_edit();
                return true;
            }
        }

        false
    }

    fn is_descendent(&self, c: &Control) -> bool {
        if self.is_visual_ancestor_of(c) {
            return true;
        }

        // If the control is not a direct visual descendent, then check to make sure it's not
        // hosted in a popup that is a descendent of the cell.
        if let Some(host) = TopLevel::get_top_level(Some(c)).and_then(|top_level| top_level.parent()).and_then(|parent| parent.cast::<Control>()) {
            return self.is_visual_ancestor_of(&host);
        }

        false
    }

    fn clear_content_if_detached(&self) {
        if self.data_context().is_none() && self.parent().is_none() && self.visual_parent().is_none() {
            self.finalize_unrealize();
        }
    }

    /// Resolves the templates of the cell model for this cell.
    fn apply_cell_templates(&self, cell: &TemplateCell) {
        let anchor: &Control = self;
        let get_cell_template = cell.get_cell_template();
        self.set_cell_template(Some(get_cell_template(anchor)));
        self.set_editing_template(cell.get_cell_editing_template().map(|get_editing_template| get_editing_template(anchor)));
    }

    fn set_cell_template(&self, template: Option<Rc<dyn IDataTemplate>>) {
        let same = match (&*self.source_content_template.borrow(), &template) {
            (Some(a), Some(b)) => Rc::ptr_eq(a, b),
            (None, None) => true,
            _ => false,
        };
        if same {
            return;
        }

        *self.source_content_template.borrow_mut() = template.clone();
        let content_template = match template {
            Some(template) if template.as_recycling_data_template().is_some() => {
                Some(Rc::new(ReattachableRecyclingDataTemplate { inner: template, last_control: RefCell::new(None) })
                    as Rc<dyn IDataTemplate>)
            }
            template => template,
        };
        self.set_content_template(content_template);
    }
}

/// Wraps a recycling cell template so that the control it built last is reused when the content
/// presenter asks for a control without one to recycle, as when a cell that kept its content
/// subtree is realized again.
struct ReattachableRecyclingDataTemplate {
    inner: Rc<dyn IDataTemplate>,
    last_control: RefCell<Option<Ref<Control>>>,
}

impl ITemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>> for ReattachableRecyclingDataTemplate {
    fn build(&self, param: &Option<BoxedValue>) -> Option<Ref<Control>> {
        self.build_with_existing(param.as_ref(), None)
    }
}

impl IDataTemplate for ReattachableRecyclingDataTemplate {
    fn match_(&self, data: Option<&BoxedValue>) -> bool {
        self.inner.match_(data)
    }

    fn as_recycling_data_template(&self) -> Option<&dyn IRecyclingDataTemplate> {
        Some(self)
    }
}

impl IRecyclingDataTemplate for ReattachableRecyclingDataTemplate {
    fn build_with_existing(&self, data: Option<&BoxedValue>, existing: Option<Ref<Control>>) -> Option<Ref<Control>> {
        let existing = existing.or_else(|| self.last_control.borrow().clone());
        let control = match self.inner.as_recycling_data_template() {
            Some(inner) => inner.build_with_existing(data, existing),
            None => self.inner.build(&data.cloned()),
        };
        *self.last_control.borrow_mut() = control.clone();
        control
    }
}
