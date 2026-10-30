//! Ported from `tests/Xaml.Behaviors.Interactions.UnitTests/Core/ChangePropertyActionTests.cs` of
//! Xaml.Behaviors (with its windows `ChangePropertyAction001.xaml` and
//! `ChangePropertyAction002.xaml`, loaded with the run-time loader).
//!
//! Adapted: the plain objects of the tests (`WriteOnlyTarget`, `ReadWriteTarget`) declare their
//! properties in markup metadata, where the action finds them; the owner of the attached property
//! of the tests is a static type registered in the type table (the reference finds it among the
//! loaded types). The rendered frames the reference saves are not taken.

use crate::tests::support::{as_bool, boxed, click, find, load, panics, start_app, text, window_with};
use crate::{ChangePropertyAction, ReversibleActionExecution};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::BindingPriority;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
use ferroui_base::reactive::IDisposable;
use ferroui_base::styling::{Selectors, Setter, Style};
use ferroui_base::{
    ferro_class, ferro_markup_type, ferro_properties, instantiate, AttachedProperty, BoxedValue, DirectProperty,
    FerroObject, FerroObjectExtensions, FerroObjectImpl, FerroProperty, Ref, StaticType, StyledElementImpl,
    StyledProperty, TypeInfo,
};
use ferroui_controls::{Border, Button, Control, Grid, TextBlock, TextBox, Window};
use ferroui_xaml_interactivity::helpers::ReversiblePropertyChange;
use ferroui_xaml_interactivity::{ActionCollection, IReversibleAction, StyledElementAction, StyledElementActionImpl};
use std::cell::RefCell;
use std::rc::Rc;

/// `AttachedPropertyOwner`: the owner of an attached property of borders.
pub(crate) struct AttachedPropertyOwner;

ferroui_base::ferro_static_type!(AttachedPropertyOwner);

ferro_properties! {
    impl AttachedPropertyOwner {
        pub fn value_property() -> AttachedProperty<i32> {
            FerroProperty::register_attached::<AttachedPropertyOwner, Border, i32>("Value", 0)
        }
    }
}

/// `ThrowingReversibleAction`: a reversible action that fails when it is applied.
#[repr(C)]
pub(crate) struct ThrowingReversibleAction {
    base: StyledElementAction,
}

ferro_class!(ThrowingReversibleAction: StyledElementAction);
ferroui_base::ferro_impl_classes!(ThrowingReversibleAction: FerroObjectImpl, StyledElementImpl);

impl StyledElementActionImpl for ThrowingReversibleAction {
    fn execute(_this: &Self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        panic!("Apply failed.");
    }
}

impl ThrowingReversibleAction {
    fn static_constructor() {
        <dyn IReversibleAction>::register::<ThrowingReversibleAction>();
    }

    fn new() -> Ref<Self> {
        instantiate(Self { base: StyledElementAction::construct() })
    }
}

impl ferroui_xaml_interactivity::IAction for ThrowingReversibleAction {
    fn execute(&self, sender: Option<&BoxedValue>, parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        StyledElementAction::execute(self, sender, parameter)
    }
}

impl IReversibleAction for ThrowingReversibleAction {
    fn execute_reversibly(&self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        panic!("Apply failed.");
    }

    fn revert(&self, _sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        Some(Rc::new(true))
    }
}

/// `WriteOnlyTarget`: a plain object with a property that can only be written.
pub(crate) struct WriteOnlyTarget {
    written_value: RefCell<Option<String>>,
}

impl PartialEq for WriteOnlyTarget {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl WriteOnlyTarget {
    fn new() -> Rc<Self> {
        Rc::new(Self { written_value: RefCell::new(None) })
    }

    fn set_value(&self, value: Option<String>) {
        *self.written_value.borrow_mut() = value;
    }

    fn written_value(&self) -> Option<String> {
        self.written_value.borrow().clone()
    }
}

ferro_markup_type!(class WriteOnlyTarget {
    handles: [Rc<WriteOnlyTarget>],
    this: Rc<WriteOnlyTarget>,
    properties: [
        Value: Option<String> {
            set: |target: &Rc<WriteOnlyTarget>, value: Option<String>| target.set_value(value)
        },
    ],
});

/// `ReadWriteTarget`: a plain object with a property that can be read and written.
pub(crate) struct ReadWriteTarget {
    value: RefCell<Option<String>>,
}

impl PartialEq for ReadWriteTarget {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ReadWriteTarget {
    fn new(value: &str) -> Rc<Self> {
        Rc::new(Self { value: RefCell::new(Some(value.to_string())) })
    }

    fn value(&self) -> Option<String> {
        self.value.borrow().clone()
    }

    fn set_value(&self, value: Option<String>) {
        *self.value.borrow_mut() = value;
    }
}

ferro_markup_type!(class ReadWriteTarget {
    handles: [Rc<ReadWriteTarget>],
    this: Rc<ReadWriteTarget>,
    properties: [
        Value: Option<String> {
            get: |target: &Rc<ReadWriteTarget>| target.value(),
            set: |target: &Rc<ReadWriteTarget>, value: Option<String>| target.set_value(value)
        },
    ],
});

/// `DirectPropertyTarget`: an object with a direct property.
#[repr(C)]
pub(crate) struct DirectPropertyTarget {
    base: FerroObject,
    value: RefCell<Option<String>>,
}

ferro_class!(DirectPropertyTarget: FerroObject);
ferroui_base::ferro_impl_classes!(DirectPropertyTarget: FerroObjectImpl);

ferro_properties! {
    impl DirectPropertyTarget {
        pub fn value_property() -> DirectProperty<DirectPropertyTarget, Option<String>> {
            FerroProperty::register_direct::<DirectPropertyTarget, _>(
                "Value",
                |target| target.value(),
                Some(|target, value| target.set_target_value(value)),
                None,
            )
        }
    }
}

impl DirectPropertyTarget {
    fn new() -> Ref<Self> {
        instantiate(Self { base: FerroObject::construct(), value: RefCell::new(None) })
    }

    fn value(&self) -> Option<String> {
        self.value.borrow().clone()
    }

    fn set_target_value(&self, value: Option<String>) {
        self.set_and_raise(Self::value_property(), &self.value, value);
    }
}

/// `StyledPropertySource`: an object with a styled property.
#[repr(C)]
pub(crate) struct StyledPropertySource {
    base: FerroObject,
}

ferro_class!(StyledPropertySource: FerroObject);
ferroui_base::ferro_impl_classes!(StyledPropertySource: FerroObjectImpl);

ferro_properties! {
    impl StyledPropertySource {
        pub fn value_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<StyledPropertySource, _>("Value", None)
        }
    }
}

impl StyledPropertySource {
    fn new() -> Ref<Self> {
        instantiate(Self { base: FerroObject::construct() })
    }

    fn set_source_value(&self, value: Option<String>) {
        self.set_value(Self::value_property(), value)
    }
}

/// Registers the types of the tests that the action finds by name.
fn register_test_types() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        TypeInfo::register_all(&[<AttachedPropertyOwner as StaticType>::TYPE]);
        MarkupType::register_all(&[<WriteOnlyTarget as MarkupTyped>::MARKUP, <ReadWriteTarget as MarkupTyped>::MARKUP]);
    });
}

/// `new ChangePropertyAction { PropertyName = property_name, Value = value }`.
fn change_property_action(property_name: &str, value: BoxedValue) -> Ref<ChangePropertyAction> {
    let action = ChangePropertyAction::new();
    action.set_property_name(Some(property_name.to_string()));
    action.set_action_value(Some(value));
    action
}

/// `ExecuteReversibly` through the contract (`((IReversibleAction)action).ExecuteReversibly`).
fn execute_reversibly(action: &ChangePropertyAction, sender: Option<&BoxedValue>) -> bool {
    as_bool(IReversibleAction::execute_reversibly(action, sender, None))
}

fn create_text_style(class_name: &str, value: &str) -> Ref<Style> {
    Style::with_setters(
        Selectors::of_type::<TextBlock>().class(class_name),
        [Setter::new(TextBlock::text_property(), Some(value.to_string()))],
    )
}

/// Regular property.
#[test]
fn change_property_action_001() {
    let _app = start_app();
    let window = load::<Window>("ChangePropertyAction001.xaml", include_str!("ChangePropertyAction001.xaml"));

    window.show();

    // Click
    click(&find::<Button>(&window, "TargetButton"));

    assert_eq!(Some("Updated Text".to_string()), find::<TextBox>(&window, "TargetTextBox").text());
}

/// Attached property.
#[test]
fn change_property_action_002() {
    let _app = start_app();
    let window = load::<Window>("ChangePropertyAction002.xaml", include_str!("ChangePropertyAction002.xaml"));

    window.show();

    // Click
    click(&find::<Button>(&window, "TargetButton"));

    assert_eq!(12.0, find::<TextBox>(&window, "TargetTextBox").font_size());
}

#[test]
fn change_property_action_updates_attached_property_when_owner_type_name_collides() {
    let _app = start_app();
    let _ = Grid::column_property();
    let target = Border::new();
    let action = change_property_action("(Grid.Column)", Rc::new(2i32));

    let result = action.execute(Some(&boxed(&target)), None);

    assert!(as_bool(result));
    assert_eq!(2, Grid::get_column(&target));
}

#[test]
fn change_property_action_finds_attached_property_registered_for_target_type() {
    let _app = start_app();
    register_test_types();
    let target = Border::new();
    let action = change_property_action("(AttachedPropertyOwner.Value)", Rc::new(42i32));

    let result = action.execute(Some(&boxed(&target)), None);

    assert!(as_bool(result));
    assert_eq!(42, target.get_value(AttachedPropertyOwner::value_property()));
}

#[test]
fn change_property_action_does_not_match_inherited_property_from_wrong_owner() {
    let _app = start_app();
    let original_data_context = boxed(&FerroObject::new());
    let target = Border::new();
    target.set_data_context(Some(original_data_context.clone()));
    let action = change_property_action("(SomeOwner.DataContext)", boxed(&FerroObject::new()));

    let result = action.execute(Some(&boxed(&target)), None);

    assert!(!as_bool(result));
    assert!(ValueTypes::identity_equals(Some(&original_data_context), target.data_context().as_ref()));
}

#[test]
fn change_property_action_matches_inherited_property_from_requested_owner() {
    let _app = start_app();
    let updated_data_context = boxed(&FerroObject::new());
    let target = Border::new();
    let action = change_property_action("(StyledElement.DataContext)", updated_data_context.clone());

    let result = action.execute(Some(&boxed(&target)), None);

    assert!(as_bool(result));
    assert!(ValueTypes::identity_equals(Some(&updated_data_context), target.data_context().as_ref()));
}

#[test]
fn revert_preserves_styled_property_source() {
    let _app = start_app();
    let target = TextBlock::new();
    target.classes().add("first");

    let window = window_with(Some(target.clone().upcast::<Control>()));
    window.styles().add(create_text_style("first", "First"));
    window.styles().add(create_text_style("second", "Second"));
    window.show();

    let action = change_property_action("Text", text("Applied"));

    assert_eq!(Some("First".to_string()), target.text());
    assert!(execute_reversibly(&action, Some(&boxed(&target))));
    assert_eq!(Some("Applied".to_string()), target.text());
    assert!(as_bool(action.revert(Some(&boxed(&target)), None)));
    assert_eq!(Some("First".to_string()), target.text());

    target.classes().remove("first");
    target.classes().add("second");

    assert_eq!(Some("Second".to_string()), target.text());
    window.close();
}

#[test]
fn revert_restores_latest_styled_property_source_value() {
    let _app = start_app();
    let target = TextBlock::new();
    target.classes().add("first");

    let window = window_with(Some(target.clone().upcast::<Control>()));
    window.styles().add(create_text_style("first", "First"));
    window.styles().add(create_text_style("second", "Second"));
    window.show();
    let action = change_property_action("Text", text("Applied"));

    assert!(execute_reversibly(&action, Some(&boxed(&target))));
    target.classes().remove("first");
    target.classes().add("second");

    assert_eq!(Some("Applied".to_string()), target.text());
    assert!(as_bool(action.revert(Some(&boxed(&target)), None)));
    assert_eq!(Some("Second".to_string()), target.text());
    window.close();
}

#[test]
fn execute_reversibly_does_not_apply_to_direct_property() {
    let _app = start_app();
    let source = StyledPropertySource::new();
    source.set_source_value(Some("Original".to_string()));
    let target = DirectPropertyTarget::new();
    let binding = target.bind_typed(
        DirectPropertyTarget::value_property(),
        source.get_observable(StyledPropertySource::value_property()),
        BindingPriority::LocalValue,
    );
    let action = ChangePropertyAction::new();
    action.set_target_object(Some(boxed(&target)));
    action.set_property_name(Some("Value".to_string()));
    action.set_action_value(Some(text("Applied")));

    assert!(!execute_reversibly(&action, None));
    assert_eq!(Some("Original".to_string()), target.value());
    source.set_source_value(Some("Latest".to_string()));
    assert_eq!(Some("Latest".to_string()), target.value());
    assert!(!as_bool(action.revert(None, None)));

    binding.dispose();
}

#[test]
fn execute_preserves_legacy_local_value_semantics() {
    let _app = start_app();
    let target = TextBlock::new();
    target.classes().add("first");

    let window = window_with(Some(target.clone().upcast::<Control>()));
    window.styles().add(create_text_style("first", "First"));
    window.styles().add(create_text_style("second", "Second"));
    window.show();

    let action = change_property_action("Text", text("Applied"));

    assert!(as_bool(action.execute(Some(&boxed(&target)), None)));
    target.classes().remove("first");
    target.classes().add("second");

    assert_eq!(Some("Applied".to_string()), target.text());
    assert!(!as_bool(action.revert(Some(&boxed(&target)), None)));
    window.close();
}

#[test]
fn execute_updates_write_only_clr_property() {
    let _app = start_app();
    register_test_types();
    let target = WriteOnlyTarget::new();
    let action = ChangePropertyAction::new();
    action.set_target_object(Some(Rc::new(target.clone())));
    action.set_property_name(Some("Value".to_string()));
    action.set_action_value(Some(text("Updated")));

    assert!(as_bool(action.execute(None, None)));
    assert_eq!(Some("Updated".to_string()), target.written_value());
    assert!(!as_bool(action.revert(None, None)));
}

#[test]
fn revert_preserves_remaining_active_property_action() {
    let _app = start_app();
    let target = TextBlock::new();
    target.set_text(Some("Original"));
    let sender = boxed(&target);
    let first = change_property_action("Text", text("First"));
    let second = change_property_action("Text", text("Second"));

    assert!(execute_reversibly(&first, Some(&sender)));
    assert!(execute_reversibly(&second, Some(&sender)));
    assert_eq!(Some("Second".to_string()), target.text());

    assert!(as_bool(first.revert(Some(&sender), None)));
    assert_eq!(Some("Second".to_string()), target.text());
    assert!(as_bool(second.revert(Some(&sender), None)));
    assert_eq!(Some("Original".to_string()), target.text());

    assert!(execute_reversibly(&first, Some(&sender)));
    assert!(execute_reversibly(&second, Some(&sender)));
    first.set_action_value(Some(text("UpdatedFirst")));
    assert!(execute_reversibly(&first, Some(&sender)));
    assert_eq!(Some("Second".to_string()), target.text());
    assert!(as_bool(second.revert(Some(&sender), None)));
    assert_eq!(Some("UpdatedFirst".to_string()), target.text());
    assert!(as_bool(first.revert(Some(&sender), None)));
    assert_eq!(Some("Original".to_string()), target.text());
}

#[test]
fn revert_preserves_remaining_active_clr_property_action() {
    let _app = start_app();
    register_test_types();
    let target = ReadWriteTarget::new("Original");
    let target_object: BoxedValue = Rc::new(target.clone());
    let first = ChangePropertyAction::new();
    first.set_target_object(Some(target_object.clone()));
    first.set_property_name(Some("Value".to_string()));
    first.set_action_value(Some(text("First")));
    let second = ChangePropertyAction::new();
    second.set_target_object(Some(target_object));
    second.set_property_name(Some("Value".to_string()));
    second.set_action_value(Some(text("Second")));

    assert!(execute_reversibly(&first, None));
    assert!(execute_reversibly(&second, None));
    assert!(as_bool(first.revert(None, None)));
    assert_eq!(Some("Second".to_string()), target.value());
    assert!(as_bool(second.revert(None, None)));
    assert_eq!(Some("Original".to_string()), target.value());
}

#[test]
fn revert_coordinates_reflection_and_typed_property_actions() {
    let _app = start_app();
    let target = TextBlock::new();
    target.set_text(Some("Original"));
    let reflection_action = change_property_action("Text", text("Reflection"));
    let typed_change: ReversiblePropertyChange<Ref<TextBlock>, Option<String>> = ReversiblePropertyChange::new("Text");

    assert!(execute_reversibly(&reflection_action, Some(&boxed(&target))));
    assert!(typed_change.apply(
        &target,
        Some("Typed".to_string()),
        |item| item.text(),
        |item, value| item.set_text(value.as_deref()),
    ));
    assert_eq!(Some("Typed".to_string()), target.text());

    assert!(as_bool(reflection_action.revert(Some(&boxed(&target)), None)));
    assert_eq!(Some("Typed".to_string()), target.text());
    assert!(typed_change.revert(|item: &Ref<TextBlock>, value: Option<String>| item.set_text(value.as_deref())));
    assert_eq!(Some("Original".to_string()), target.text());
}

#[test]
fn reversible_action_execution_unwinds_applied_actions_when_later_action_throws() {
    let _app = start_app();
    let target = TextBlock::new();
    target.set_text(Some("Original"));
    let sender = boxed(&target);
    let property_action = change_property_action("Text", text("Applied"));
    let actions = ActionCollection::new();
    actions.add(property_action.clone().upcast());
    actions.add(ThrowingReversibleAction::new().upcast());

    assert!(panics(|| {
        ReversibleActionExecution::execute(Some(&sender), Some(&actions), None);
    }));
    assert_eq!(Some("Original".to_string()), target.text());

    assert!(execute_reversibly(&property_action, Some(&sender)));
    assert!(as_bool(property_action.revert(Some(&sender), None)));
    assert_eq!(Some("Original".to_string()), target.text());
}

// The classes `Grid` and `SomeOwner` of the reference are plain classes that only collide by name
// with the owners the tests name; they are not registered types, so nothing collides here.
