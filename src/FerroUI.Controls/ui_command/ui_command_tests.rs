//! Tests of `XamlUICommand`, `StandardUICommand` and the command sources that take their
//! properties (the behaviour of `CommandingHelpers.cpp` and of the WinUI Gallery pages
//! XamlUICommand and StandardUICommand).

use super::*;
use crate::automation::AutomationProperties;
use crate::{Button, CommandBarButton, CommandBarToggleButton, MenuItem, SwipeItem, Symbol, SymbolIcon};
use crate::{SymbolIconSource, ToolTip};
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::input::{ICommand, Key, KeyGesture, KeyModifiers};
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{BoxedValue, Ref};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn custom_command() -> Ref<XamlUICommand> {
    let command = XamlUICommand::with_label("Custom XamlUICommand");
    command.set_description("This is a custom command");
    let icon = SymbolIconSource::new();
    icon.set_symbol(Symbol::Favorite);
    command.set_icon_source(Some(icon.upcast()));
    command.keyboard_accelerators().add(KeyboardAccelerator::with_key(Key::D, KeyModifiers::CONTROL));
    command
}

fn text(value: &Option<BoxedValue>) -> Option<String> {
    value.as_ref().and_then(|value| value.downcast_ref::<String>().cloned())
}

fn symbol_of(icon: &Option<BoxedValue>) -> Option<Symbol> {
    let control = icon.as_ref().and_then(crate::Control::from_boxed)?;
    control.cast::<SymbolIcon>().map(|icon| icon.symbol())
}

struct Recording {
    executed: Rc<RefCell<Vec<Option<BoxedValue>>>>,
}

impl ICommand for Recording {
    fn can_execute(&self, parameter: Option<&BoxedValue>) -> bool {
        parameter.and_then(|p| p.downcast_ref::<String>().cloned()).as_deref() != Some("no")
    }

    fn execute(&self, parameter: Option<&BoxedValue>) {
        self.executed.borrow_mut().push(parameter.cloned());
    }

    fn can_execute_changed(&self, _handler: Rc<dyn Fn()>) -> Rc<dyn IDisposable> {
        Disposable::empty()
    }
}

#[test]
fn execute_raises_execute_requested_then_runs_the_wrapped_command() {
    let command = XamlUICommand::new();
    let order = Rc::new(RefCell::new(Vec::new()));
    let seen = order.clone();
    let _handler = command.execute_requested(move |_, args| {
        seen.borrow_mut()
            .push(format!("requested {:?}", args.parameter().and_then(|p| p.downcast_ref::<String>().cloned())));
    });
    let executed = Rc::new(RefCell::new(Vec::new()));
    command.set_command(Some(Rc::new(Recording { executed: executed.clone() })));
    let parameter: BoxedValue = Rc::new("List item 3".to_string());
    command.as_command().execute(Some(&parameter));
    assert_eq!(vec!["requested Some(\"List item 3\")".to_string()], *order.borrow());
    assert_eq!(1, executed.borrow().len());
}

#[test]
fn can_execute_asks_the_handlers_and_the_wrapped_command() {
    let command = XamlUICommand::new();
    assert!(command.can_execute(None));
    let allowed = Rc::new(Cell::new(true));
    let read = allowed.clone();
    let _handler = command.can_execute_requested(move |_, args| {
        assert!(args.can_execute());
        args.set_can_execute(read.get());
    });
    allowed.set(false);
    assert!(!command.can_execute(None));
    allowed.set(true);
    command.set_command(Some(Rc::new(Recording { executed: Rc::default() })));
    let no: BoxedValue = Rc::new("no".to_string());
    assert!(!command.can_execute(Some(&no)));
    assert!(command.can_execute(None));
}

#[test]
fn notify_can_execute_changed_raises_can_execute_changed() {
    let command = XamlUICommand::new();
    let count = Rc::new(Cell::new(0));
    let counted = count.clone();
    let subscription = command.as_command().can_execute_changed(Rc::new(move || counted.set(counted.get() + 1)));
    command.notify_can_execute_changed();
    assert_eq!(1, count.get());
    subscription.dispose();
    command.notify_can_execute_changed();
    assert_eq!(1, count.get());
}

#[test]
fn the_command_value_is_one_value_and_converts_from_the_object() {
    let command = custom_command();
    let first = command.as_command();
    let second = command.as_command();
    assert!(Rc::ptr_eq(&first, &second));
    assert!(XamlUICommand::from_command(&first).is_some_and(|found| found == command));
    // Markup sets the object on a command property (`Command="{StaticResource ...}"`).
    let boxed: BoxedValue = Rc::new(command.clone());
    let cast = ValueTypes::try_cast(&boxed, ValueType::of::<Rc<dyn ICommand>>()).expect("a command");
    let cast = cast.downcast_ref::<Rc<dyn ICommand>>().expect("an Rc<dyn ICommand>").clone();
    assert!(Rc::ptr_eq(&first, &cast));
    let standard: BoxedValue = Rc::new(StandardUICommand::with_kind(StandardUICommandKind::Delete));
    assert!(ValueTypes::try_cast(&standard, ValueType::of::<Option<Rc<dyn ICommand>>>()).is_some());
}

#[test]
fn a_button_takes_the_label_accelerator_and_description_it_has_not_set() {
    let command = custom_command();
    let button = Button::new();
    button.set_command(Some(command.as_command()));
    assert_eq!(Some("Custom XamlUICommand".to_string()), text(&button.content()));
    assert_eq!(Some(KeyGesture::new(Key::D, KeyModifiers::CONTROL)), button.hot_key());
    assert_eq!(Some("This is a custom command".to_string()), AutomationProperties::get_help_text(&button));
    assert_eq!(Some("This is a custom command".to_string()), text(&ToolTip::get_tip(&button)));
    // A later change of the command reaches the button.
    command.set_label("Renamed");
    assert_eq!(Some("Renamed".to_string()), text(&button.content()));
    // Another command takes what the first gave back.
    button.set_command(None);
    assert_eq!(None, button.content());
    assert_eq!(None, button.hot_key());
    assert_eq!(None, ToolTip::get_tip(&button));
}

#[test]
fn a_button_keeps_what_it_set_itself() {
    let button = Button::new();
    button.set_content(Some(Rc::new("Mine".to_string()) as BoxedValue));
    button.set_hot_key(Some(KeyGesture::from_key(Key::F2)));
    button.set_command(Some(custom_command().as_command()));
    assert_eq!(Some("Mine".to_string()), text(&button.content()));
    assert_eq!(Some(KeyGesture::from_key(Key::F2)), button.hot_key());
    button.set_command(None);
    assert_eq!(Some(KeyGesture::from_key(Key::F2)), button.hot_key());
}

#[test]
fn a_command_bar_button_takes_the_label_and_the_icon_not_the_content() {
    let command = custom_command();
    let button = CommandBarButton::new();
    button.set_command(Some(command.as_command()));
    assert_eq!(Some("Custom XamlUICommand".to_string()), button.label());
    assert_eq!(Some(Symbol::Favorite), symbol_of(&button.icon()));
    assert_eq!(None, button.content());
    assert_eq!(Some(KeyGesture::new(Key::D, KeyModifiers::CONTROL)), button.hot_key());
    let toggle = CommandBarToggleButton::new();
    toggle.set_command(Some(command.as_command()));
    assert_eq!(Some("Custom XamlUICommand".to_string()), toggle.label());
    assert_eq!(Some(Symbol::Favorite), symbol_of(&toggle.icon()));
    // Execute raises ExecuteRequested, as the gallery's AppBarButton does when clicked.
    let fired = Rc::new(Cell::new(false));
    let seen = fired.clone();
    let _handler = command.execute_requested(move |_, _| seen.set(true));
    button.command().expect("a command").execute(None);
    assert!(fired.get());
}

#[test]
fn a_menu_item_takes_the_header_icon_and_input_gesture() {
    let command = StandardUICommand::with_kind(StandardUICommandKind::Delete);
    let item = MenuItem::new();
    item.set_command(Some(command.as_command()));
    assert_eq!(Some("Delete".to_string()), text(&item.header()));
    assert_eq!(Some(Symbol::Delete), symbol_of(&item.icon()));
    assert_eq!(Some(KeyGesture::from_key(Key::Delete)), item.input_gesture());
    assert_eq!(Some(KeyGesture::from_key(Key::Delete)), item.hot_key());
    assert_eq!(Some("Delete the selected content".to_string()), AutomationProperties::get_help_text(&item));
}

#[test]
fn a_swipe_item_takes_the_text_and_the_icon_source() {
    let command = StandardUICommand::with_kind(StandardUICommandKind::Delete);
    let item = SwipeItem::new();
    item.set_command(Some(command.as_command()));
    assert_eq!("Delete", item.text());
    let icon = item.icon_source().and_then(|icon| icon.cast::<SymbolIconSource>()).expect("a symbol icon source");
    assert_eq!(Symbol::Delete, icon.symbol());
}

#[test]
fn a_standard_command_describes_its_kind() {
    let command = StandardUICommand::with_kind(StandardUICommandKind::Save);
    assert_eq!("Save", command.label());
    assert_eq!("Save your changes", command.description());
    let accelerators = command.keyboard_accelerators().snapshot();
    assert_eq!(1, accelerators.len());
    assert_eq!(Some(KeyGesture::new(Key::S, KeyModifiers::CONTROL)), accelerators[0].key_gesture());
    let icon = command.icon_source().and_then(|icon| icon.cast::<SymbolIconSource>()).expect("a symbol");
    assert_eq!(Symbol::Save, icon.symbol());
    // A change of the kind changes what the kind set.
    command.set_kind(StandardUICommandKind::Share);
    assert_eq!("Share", command.label());
    assert!(command.keyboard_accelerators().snapshot().is_empty());
    command.set_kind(StandardUICommandKind::Undo);
    assert_eq!(Some(KeyGesture::new(Key::Z, KeyModifiers::CONTROL)), command.first_key_gesture());
}

#[test]
fn a_standard_command_keeps_what_the_application_set() {
    let command = StandardUICommand::with_kind(StandardUICommandKind::Copy);
    command.set_label("Copy link");
    command.keyboard_accelerators().clear();
    command.keyboard_accelerators().add(KeyboardAccelerator::with_key(Key::L, KeyModifiers::CONTROL));
    command.set_kind(StandardUICommandKind::Cut);
    assert_eq!("Copy link", command.label());
    assert_eq!("Remove the selected content and put it on the clipboard", command.description());
    assert_eq!(Some(KeyGesture::new(Key::L, KeyModifiers::CONTROL)), command.first_key_gesture());
    assert_eq!(StandardUICommandKind::None, StandardUICommand::new().kind());
    assert_eq!("", StandardUICommand::new().label());
}

#[test]
fn a_disabled_accelerator_is_not_taken() {
    let command = XamlUICommand::with_label("Run");
    let accelerator = KeyboardAccelerator::with_key(Key::R, KeyModifiers::CONTROL);
    accelerator.set_is_enabled(false);
    command.keyboard_accelerators().add(accelerator);
    command.keyboard_accelerators().add(KeyboardAccelerator::with_key(Key::F5, KeyModifiers::NONE));
    assert_eq!(Some(KeyGesture::from_key(Key::F5)), command.first_key_gesture());
}
