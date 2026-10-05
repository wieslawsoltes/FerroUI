//! Tests of the reference semantics of the controls object model: the
//! collections of controls are shared handles with identity equality, the
//! contracts markup asks for are reachable from class handles, and a binding
//! expression class can be defined outside the crate that owns the base
//! class.

use crate::documents::{Bold, Inline, InlineCollection, InlineUIContainer, Run, Span};
use crate::templates::DataTemplates;
use crate::{
    Application, Border, ColumnDefinition, ColumnDefinitions, ContainerClearingEventArgs,
    ContainerIndexChangedEventArgs, ContainerPreparedEventArgs, Control, Controls, Flyout, Grid, GridLength, ItemCollection,
    ItemsControl, Panel, RowDefinitions,
};
use ferroui_base::controls::{Classes, IResourceHost, ResourceHostRef};
use ferroui_base::data::core::{Publish, UntypedBindingExpression, UntypedBindingExpressionBase};
use ferroui_base::data::{BindingBase, BindingExpressionBase, BindingPriority};
use ferroui_base::metadata::{from_markup_value, into_markup_value, IAddChild};
use ferroui_base::styling::{IStyleHost, StyleHostRef};
use ferroui_base::{BoxedValue, FerroObject, FerroProperty, Ref, StyledElement};
use std::cell::Cell;
use std::rc::Rc;

fn boxed<T: PartialEq + 'static>(value: T) -> BoxedValue {
    Rc::new(value)
}

#[test]
fn panel_children_are_returned_by_handle() {
    let panel = Panel::new();
    let children = panel.children();
    children.add(Border::new());
    assert_eq!(panel.children().count(), 1);
    assert!(children == panel.children());
    assert!(children != Controls::new());

    let value = boxed(panel.children());
    value.downcast_ref::<Controls>().expect("the children").add(Border::new());
    assert_eq!(panel.children().count(), 2);
    assert_eq!(panel.get_visual_children().len(), 2);
}

#[test]
fn items_of_an_items_control_are_returned_by_handle() {
    let target = ItemsControl::new();
    let items = target.items();
    items.add(Some(boxed("foo".to_string())));
    assert_eq!(target.items().count(), 1);
    assert!(items == target.items());
    assert!(items != ItemsControl::new().items());

    let value = boxed(target.items());
    value.downcast_ref::<ItemCollection>().expect("the items").add(Some(boxed("bar".to_string())));
    assert_eq!(target.item_count(), 2);
}

#[test]
fn data_templates_are_returned_by_handle() {
    let control = Control::new();
    let templates = control.data_templates();
    assert!(templates == control.data_templates());
    assert!(templates != DataTemplates::new());
    assert!(control.is_data_templates_initialized());
}

#[test]
fn grid_definitions_are_handles_and_parse_from_text() {
    let grid = Grid::new();
    let columns = grid.column_definitions();
    columns.add(ColumnDefinition::with_width(GridLength::parse("2*").unwrap()));
    assert_eq!(grid.column_definitions().count(), 1);
    assert!(columns == grid.column_definitions());

    let parsed = ColumnDefinitions::parse("Auto,*,100").unwrap();
    assert_eq!(parsed.count(), 3);
    assert!(parsed != ColumnDefinitions::parse("Auto,*,100").unwrap());
    grid.set_column_definitions(parsed.clone());
    assert!(grid.column_definitions() == parsed);

    let rows = RowDefinitions::parse("Auto,*").unwrap();
    let value = boxed(rows.clone());
    assert!(value.downcast_ref::<RowDefinitions>() == Some(&rows));
    grid.set_row_definitions(rows.clone());
    assert!(grid.row_definitions() == rows);
    assert_eq!(grid.row_definitions().count(), 2);
}

#[test]
fn flyout_presenter_classes_are_returned_by_handle() {
    let flyout = Flyout::new();
    let classes = flyout.flyout_presenter_classes();
    classes.add("foo");
    assert!(flyout.flyout_presenter_classes().contains("foo"));
    assert!(classes == flyout.flyout_presenter_classes());
    assert!(classes != Classes::new());
}

#[test]
fn inlines_are_a_handle() {
    let span = Span::new();
    let inlines = span.inlines();
    inlines.add(Run::with_text(Some("foo")));
    assert_eq!(span.inlines().count(), 1);
    assert!(inlines == span.inlines());
    assert!(inlines != InlineCollection::new());
}

#[test]
fn span_adds_inline_control_and_text_children() {
    let span = Span::new();

    let add_inline = Span::as_add_child_of_inline(span.clone());
    add_inline.add_child(Run::with_text(Some("foo")).upcast());

    let add_control = Span::as_add_child_of_control(span.clone());
    let border = Border::new();
    add_control.add_child(border.clone().upcast());

    let add_text = Span::as_add_child_of_string(span.clone());
    add_text.add_child("bar".to_string());

    let inlines = span.inlines().to_vec();
    assert_eq!(inlines.len(), 3);
    assert_eq!(inlines[0].cast::<Run>().expect("a run").text().as_deref(), Some("foo"));
    let container = inlines[1].cast::<InlineUIContainer>().expect("a container");
    assert!(container.child() == Some(border.upcast()));
    assert_eq!(inlines[2].cast::<Run>().expect("a run").text().as_deref(), Some("bar"));
}

#[test]
fn span_contracts_are_reachable_from_untyped_class_handles() {
    // A class deriving from the span inherits its contracts.
    let bold = into_markup_value(Bold::new());
    let add_text = from_markup_value::<Rc<dyn IAddChild<String>>>(&bold).expect("adds text");
    add_text.add_child("foo".to_string());
    assert!(from_markup_value::<Rc<dyn IAddChild<Ref<Inline>>>>(&bold).is_some());
    assert!(from_markup_value::<Rc<dyn IAddChild<Ref<Control>>>>(&bold).is_some());

    let bold = from_markup_value::<Ref<Bold>>(&bold).expect("the element");
    assert_eq!(bold.inlines().count(), 1);
}

#[test]
fn application_contracts_are_reachable_from_untyped_class_handles() {
    let application = Application::new();
    let untyped = into_markup_value(application.clone());

    let resource_host = from_markup_value::<Rc<dyn IResourceHost>>(&untyped).expect("a resource host");
    assert!(*resource_host == *application.as_resource_host());
    let style_host = from_markup_value::<Rc<dyn IStyleHost>>(&untyped).expect("a style host");
    assert!(*style_host == *application.as_style_host());
    assert!(
        from_markup_value::<ResourceHostRef>(&untyped) == Some(ResourceHostRef::Other(application.as_resource_host()))
    );
    assert!(from_markup_value::<StyleHostRef>(&untyped).is_some());

    let other = Application::new();
    assert!(*resource_host != *other.as_resource_host());
}

#[test]
fn container_event_args_can_be_held_in_untyped_values() {
    let container: Ref<Control> = Border::new().upcast();

    let prepared = ContainerPreparedEventArgs::new(container.clone(), 1);
    assert!(boxed(prepared.clone()).downcast_ref::<ContainerPreparedEventArgs>() == Some(&prepared));
    assert!(prepared != ContainerPreparedEventArgs::new(container.clone(), 2));

    let clearing = ContainerClearingEventArgs::new(container.clone());
    assert!(boxed(clearing.clone()).downcast_ref::<ContainerClearingEventArgs>() == Some(&clearing));

    let changed = ContainerIndexChangedEventArgs::new(container.clone(), 1, 2);
    assert!(boxed(changed.clone()).downcast_ref::<ContainerIndexChangedEventArgs>() == Some(&changed));
    assert!(changed != ContainerIndexChangedEventArgs::new(container, 2, 1));
}

// --- a binding expression class defined in this crate -------------------------

/// Publishes a constant while it runs: the smallest class deriving from the
/// untyped binding expression base.
struct ConstantExpression {
    base: UntypedBindingExpressionBase,
    value: BoxedValue,
    starts: Rc<Cell<u32>>,
    stops: Rc<Cell<u32>>,
}

impl UntypedBindingExpression for ConstantExpression {
    fn base(&self) -> &UntypedBindingExpressionBase {
        &self.base
    }

    fn description(&self) -> String {
        "Constant".to_string()
    }

    fn start_core(&self) {
        self.starts.set(self.starts.get() + 1);
        self.base.publish_value(Publish::Value(Some(self.value.clone())), None, false);
    }

    fn stop_core(&self) {
        self.stops.set(self.stops.get() + 1);
    }
}

ferroui_base::impl_untyped_binding_expression!(ConstantExpression);

struct ConstantBinding {
    value: BoxedValue,
    starts: Rc<Cell<u32>>,
    stops: Rc<Cell<u32>>,
}

impl BindingBase for ConstantBinding {
    fn create_instance(
        &self,
        _target: &FerroObject,
        _target_property: Option<&'static FerroProperty>,
        _anchor: Option<&Ref<FerroObject>>,
    ) -> Rc<dyn BindingExpressionBase> {
        let expression = Rc::new_cyclic(|this: &std::rc::Weak<ConstantExpression>| {
            let this: std::rc::Weak<dyn BindingExpressionBase> = this.clone();
            ConstantExpression {
                base: UntypedBindingExpressionBase::new(this, BindingPriority::LocalValue, None, false),
                value: self.value.clone(),
                starts: self.starts.clone(),
                stops: self.stops.clone(),
            }
        });
        expression
    }
}

#[test]
fn a_binding_expression_class_can_be_defined_outside_the_base_crate() {
    let starts = Rc::new(Cell::new(0));
    let stops = Rc::new(Cell::new(0));
    // The published value holds exactly the value type of the target
    // property.
    let constant: Option<BoxedValue> = Some(boxed("constant".to_string()));
    let binding = ConstantBinding { value: boxed(constant), starts: starts.clone(), stops: stops.clone() };

    let target = StyledElement::new();
    let expression = target.bind_binding(StyledElement::data_context_property(), &binding);

    assert_eq!(starts.get(), 1);
    let value = target.data_context().expect("a value");
    assert_eq!(value.downcast_ref::<String>().map(String::as_str), Some("constant"));
    assert_eq!(expression.priority(), BindingPriority::LocalValue);
    assert!(expression.as_untyped().is_some_and(|e| e.description() == "Constant"));

    expression.dispose();
    assert_eq!(stops.get(), 1);
    assert!(target.data_context().is_none());
}
