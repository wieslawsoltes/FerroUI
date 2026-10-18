//! `TreeDataGridTemplateCellTests.cs`.

use super::support::{run_jobs, same_value, start_app, test_window, tree_data_grid_template_cell_template, TreeAttachmentCounter};
use crate::models::{CellRef, CellTemplateGetter, TemplateCell};
use crate::primitives::{TreeDataGridElementFactory, TreeDataGridTemplateCell};
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::InteractiveImpl;
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::{ferro_class, instantiate, BoxedValue, FerroObjectImpl, Ref, StyledElementImpl, VisualImpl};
use ferroui_controls::primitives::{
    TemplateAppliedEventArgs, TemplatedControl, TemplatedControlImpl, TemplatedControlImplExt,
};
use ferroui_controls::templates::{
    FuncControlTemplate, FuncDataTemplate, IDataTemplate, IRecyclingDataTemplate, ITemplateWithParam,
};
use ferroui_controls::{Border, CheckBox, Control, ControlImpl, StackPanel, TextBlock, TextBox, Window};
use std::cell::Cell;
use std::rc::Rc;

fn new_value() -> Option<BoxedValue> {
    Some(Rc::new(()) as BoxedValue)
}

fn data_template(build: impl Fn() -> Ref<Control> + 'static) -> Rc<dyn IDataTemplate> {
    FuncDataTemplate::new(|_| true, move |_, _| Some(build()), false)
}

fn getter(template: Rc<dyn IDataTemplate>) -> CellTemplateGetter {
    Rc::new(move |_: &Control| template.clone())
}

/// `TrackingDataTemplate`: a recycling template that reuses the control it is given.
struct TrackingDataTemplate;

impl ITemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>> for TrackingDataTemplate {
    fn build(&self, param: &Option<BoxedValue>) -> Option<Ref<Control>> {
        self.build_with_existing(param.as_ref(), None)
    }
}

impl IDataTemplate for TrackingDataTemplate {
    fn match_(&self, _data: Option<&BoxedValue>) -> bool {
        true
    }

    fn as_recycling_data_template(&self) -> Option<&dyn IRecyclingDataTemplate> {
        Some(self)
    }
}

impl IRecyclingDataTemplate for TrackingDataTemplate {
    fn build_with_existing(&self, _data: Option<&BoxedValue>, existing: Option<Ref<Control>>) -> Option<Ref<Control>> {
        existing.or_else(|| Some(TrackingControl::new().upcast()))
    }
}

/// `TrackingControl`: a templated control that counts the applications of its template.
#[repr(C)]
struct TrackingControl {
    base: TemplatedControl,
    apply_template_count: Cell<i32>,
}

ferro_class!(TrackingControl: TemplatedControl);
ferroui_base::ferro_impl_classes!(
    TrackingControl: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl
);

impl TemplatedControlImpl for TrackingControl {
    fn on_apply_template(this: &Self, e: &TemplateAppliedEventArgs) {
        this.apply_template_count.set(this.apply_template_count.get() + 1);
        Self::parent_on_apply_template(this, e);
    }
}

impl TrackingControl {
    fn new() -> Ref<Self> {
        let control = instantiate(Self { base: TemplatedControl::construct(), apply_template_count: Cell::new(0) });
        control.set_template(Some(FuncControlTemplate::for_type::<TrackingControl>(|_, _| Border::new().upcast())));
        control
    }
}

fn single_tracking_control(cell: &TreeDataGridTemplateCell) -> Ref<TrackingControl> {
    let controls: Vec<Ref<TrackingControl>> =
        cell.get_visual_descendants().filter_map(|visual| visual.cast::<TrackingControl>()).collect();
    assert_eq!(1, controls.len());
    controls.into_iter().next().expect("a tracking control")
}

#[test]
fn recycled_cell_uses_new_columns_template_when_realized_while_detached() {
    let _app = start_app();
    let first_template = data_template(|| TextBlock::new().upcast());
    let second_template = data_template(|| Border::new().upcast());
    let first_editing_template = data_template(|| TextBox::new().upcast());
    let second_editing_template = data_template(|| CheckBox::new().upcast());
    let first_model = TemplateCell::new(new_value(), getter(first_template), Some(getter(first_editing_template)), None);
    let second_model =
        TemplateCell::new(new_value(), getter(second_template), Some(getter(second_editing_template)), None);
    let factory = TreeDataGridElementFactory::new();
    let cell = TreeDataGridTemplateCell::new();
    let panel = StackPanel::new();
    let window = Window::new();
    window.set_content(Some(Control::boxed(&panel)));

    let first: CellRef = first_model.clone();
    cell.realize(&factory, None, &first, 0, 0);
    panel.children().add(&cell);
    window.show();

    let content_template = cell.content_template().expect("a content template");
    assert!(content_template.build(&first_model.value()).and_then(|c| c.cast::<TextBlock>()).is_some());
    let editing_template = cell.editing_template().expect("an editing template");
    assert!(editing_template.build(&first_model.value()).and_then(|c| c.cast::<TextBox>()).is_some());

    panel.children().remove(&cell);
    cell.unrealize();

    assert!(cell.content().is_none());

    let second: CellRef = second_model.clone();
    cell.realize(&factory, None, &second, 3, 0);
    panel.children().add(&cell);

    let content_template = cell.content_template().expect("a content template");
    assert!(content_template.build(&second_model.value()).and_then(|c| c.cast::<Border>()).is_some());
    let editing_template = cell.editing_template().expect("an editing template");
    assert!(editing_template.build(&second_model.value()).and_then(|c| c.cast::<CheckBox>()).is_some());

    window.close();
}

#[test]
fn retained_unrealized_cell_keeps_its_templated_control_parented() {
    let _app = start_app();
    let template: Rc<dyn IDataTemplate> = Rc::new(TrackingDataTemplate);
    let first_model = TemplateCell::new(new_value(), getter(template.clone()), None, None);
    let second_model = TemplateCell::new(new_value(), getter(template), None, None);
    let factory = TreeDataGridElementFactory::new();
    let cell = TreeDataGridTemplateCell::new();
    cell.set_template(Some(tree_data_grid_template_cell_template()));
    let panel = StackPanel::new();
    panel.children().add(&cell);
    let window = test_window(Some(panel.clone().upcast()), None);

    let first: CellRef = first_model.clone();
    cell.realize(&factory, None, &first, 0, 0);
    window.update_layout();
    run_jobs();

    let content = single_tracking_control(&cell);
    let logical_parent = content.parent();
    let visual_parent = content.visual_parent();
    let counter = TreeAttachmentCounter::new();
    counter.watch(&content);

    // The presenter base retains recyclable controls by hiding them while leaving their logical
    // and visual parents intact.
    cell.unrealize();
    cell.set_is_visible(false);
    run_jobs();

    assert!(same_value(first_model.value().as_ref(), cell.content().as_ref()));
    assert_eq!(logical_parent, content.parent());
    assert_eq!(visual_parent, content.visual_parent());
    assert_eq!(content, single_tracking_control(&cell));

    cell.finalize_unrealize();

    assert!(cell.content().is_none());
    assert_eq!(logical_parent, content.parent());
    assert_eq!(visual_parent, content.visual_parent());
    assert_eq!(content, single_tracking_control(&cell));

    cell.set_is_visible(true);
    let second: CellRef = second_model.clone();
    cell.realize(&factory, None, &second, 0, 1);
    window.update_layout();
    run_jobs();

    assert_eq!(content, single_tracking_control(&cell));
    assert_eq!(1, content.apply_template_count.get());
    counter.assert_none();

    cell.unrealize();
    cell.set_is_visible(false);
    panel.children().remove(&cell);

    assert!(cell.content().is_none());

    window.close();
}
