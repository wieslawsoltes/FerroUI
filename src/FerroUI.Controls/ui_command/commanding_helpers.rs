//! WinUI's `CommandingHelpers` (`dxaml/xcp/dxaml/lib/CommandingHelpers.cpp`,
//! `controls/dev/dll/CommandingHelpers.cpp` and the `OnCommandChanged` of `ButtonBase_Partial.cpp`,
//! `AppBarButtonHelpers.h`, `MenuFlyoutItem_Partial.cpp` and `SwipeItem.cpp` of microsoft-ui-xaml
//! commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c) Microsoft Corporation, see
//! NOTICE.md): a command source takes the properties of an [`XamlUICommand`] it has not set
//! itself.
//!
//! The sources call [`command_changed`] when their command changes. It does nothing until the
//! first `XamlUICommand` is created ([`install`]), so an application without one links none of
//! this module.

use super::XamlUICommand;
use crate::automation::AutomationProperties;
use crate::command_bar::{CommandBarButton, CommandBarToggleButton};
use crate::primitives::HeaderedSelectingItemsControl;
use crate::swipe_control::SwipeItem;
use crate::{Button, ContentControl, Control, MenuItem, ToolTip};
use ferroui_base::data::BindingPriority;
use ferroui_base::input::{ICommand, KeyGesture};
use ferroui_base::reactive::{Disposable, IDisposable, IObservable};
use ferroui_base::{
    BoxedValue, FerroObject, FerroObjectExtensions, StyledElement, StyledProperty, StyledValue, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// The kind of a command source: what of the command it takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UICommandSource {
    /// A button (WinUI's `ButtonBase`): the label as the content, unless the button is a command
    /// bar button, which takes the label and the icon (WinUI's `AppBarButton`).
    Button,
    /// A menu item (WinUI's `MenuFlyoutItem`): the label as the header and the icon.
    MenuItem,
    /// A swipe item: the label as the text and the icon source.
    SwipeItem,
}

type Hook = fn(&FerroObject, UICommandSource, Option<&Rc<dyn ICommand>>);

/// What a source took from its command: undone when the command changes
/// (`ClearBindingIfSet`).
struct SourceBindings {
    source: WeakRef<FerroObject>,
    undo: Vec<Rc<dyn IDisposable>>,
}

ferroui_base::ferro_thread_local! {
    /// The work of [`command_changed`], set by the first `XamlUICommand`.
    static HOOK: Cell<Option<Hook>> = const { Cell::new(None) };
    static BINDINGS: RefCell<Vec<SourceBindings>> = const { RefCell::new(Vec::new()) };
}

/// Called by a command source when its command changes to `command`.
#[inline]
pub(crate) fn command_changed(source: &FerroObject, kind: UICommandSource, command: Option<&Rc<dyn ICommand>>) {
    if let Some(hook) = HOOK.get() {
        hook(source, kind, command);
    }
}

/// Turns [`command_changed`] on: called when an `XamlUICommand` is created.
pub(super) fn install() {
    if HOOK.get().is_none() {
        HOOK.set(Some(apply));
    }
}

fn apply(source: &FerroObject, kind: UICommandSource, command: Option<&Rc<dyn ICommand>>) {
    // ClearBindingIfSet for the previous command: what the source took from it goes away.
    let previous = BINDINGS.with_borrow_mut(|bindings| {
        let mut previous = None;
        bindings.retain_mut(|entry| match entry.source.upgrade() {
            Some(object) if std::ptr::eq::<FerroObject>(&*object, source) => {
                previous = Some(std::mem::take(&mut entry.undo));
                false
            }
            Some(_) => true,
            None => false,
        });
        previous
    });
    for undo in previous.into_iter().flatten() {
        undo.dispose();
    }
    let Some(ui_command) = command.and_then(XamlUICommand::from_command) else {
        return;
    };
    let mut undo = Vec::new();
    match kind {
        UICommandSource::Button => bind_button(source, &ui_command, &mut undo),
        UICommandSource::MenuItem => bind_menu_item(source, &ui_command, &mut undo),
        UICommandSource::SwipeItem => bind_swipe_item(source, &ui_command, &mut undo),
    }
    if !undo.is_empty() {
        BINDINGS.with_borrow_mut(|bindings| {
            bindings.push(SourceBindings { source: source.to_ref().downgrade(), undo });
        });
    }
}

/// Binds `property` of `source` to `observable` as its local value (WinUI's direct binding).
fn bind<T: StyledValue>(
    source: &FerroObject,
    property: &'static StyledProperty<T>,
    observable: Rc<dyn IObservable<T>>,
    undo: &mut Vec<Rc<dyn IDisposable>>,
) {
    undo.push(source.bind(property, observable, BindingPriority::LocalValue));
}

fn is_unset_text(value: Option<&str>) -> bool {
    value.is_none_or(str::is_empty)
}

fn is_unset_content(value: Option<&BoxedValue>) -> bool {
    value.is_none_or(|value| value.downcast_ref::<String>().is_some_and(String::is_empty))
}

fn label_as_content(command: &XamlUICommand) -> Rc<dyn IObservable<Option<BoxedValue>>> {
    command.get_observable_with(XamlUICommand::label_property(), |label| Some(Rc::new(label) as BoxedValue))
}

fn label_as_text(command: &XamlUICommand) -> Rc<dyn IObservable<Option<String>>> {
    command.get_observable_with(XamlUICommand::label_property(), Some)
}

/// `IconSourceToIconSourceElementConverter`: a new icon element of the command's icon source.
fn icon_as_element(command: &XamlUICommand) -> Rc<dyn IObservable<Option<BoxedValue>>> {
    command.get_observable_with(XamlUICommand::icon_source_property(), |icon| {
        icon.and_then(|icon| icon.create_icon_element()).map(Control::boxed)
    })
}

/// `BindToKeyboardAcceleratorsIfUnset`: the gesture of the first enabled accelerator becomes the
/// source's hot key (and a menu item's input gesture), when it has none; the value is taken when
/// the command is set (WinUI copies the accelerators then), and removed with the command when it
/// is still the one taken.
fn bind_key_gesture(
    source: &FerroObject,
    command: &XamlUICommand,
    property: &'static StyledProperty<Option<KeyGesture>>,
    undo: &mut Vec<Rc<dyn IDisposable>>,
) {
    let Some(gesture) = command.first_key_gesture() else {
        return;
    };
    if source.get_value(property).is_some() {
        return;
    }
    source.set_value(property, Some(gesture));
    let weak = source.to_ref().downgrade();
    undo.push(Disposable::create(move || {
        if let Some(source) = weak.upgrade() {
            if source.get_value(property) == Some(gesture) {
                source.clear_value(property);
            }
        }
    }));
}

/// `BindToAccessKeyIfUnset` and `BindToDescriptionPropertiesIfUnset`: the access key, the help
/// text and the tool tip of a control.
fn bind_element_properties(source: &FerroObject, command: &XamlUICommand, undo: &mut Vec<Rc<dyn IDisposable>>) {
    let Some(element) = source.to_ref().cast::<StyledElement>() else {
        return;
    };
    if is_unset_text(AutomationProperties::get_access_key(&element).as_deref()) {
        let access_key = command.get_observable_with(XamlUICommand::access_key_property(), Some);
        bind(source, AutomationProperties::access_key_property(), access_key, undo);
    }
    if is_unset_text(AutomationProperties::get_help_text(&element).as_deref()) {
        let help_text = command.get_observable_with(XamlUICommand::description_property(), Some);
        bind(source, AutomationProperties::help_text_property(), help_text, undo);
    }
    if let Some(control) = element.cast::<Control>() {
        if ToolTip::get_tip(&control).is_none() {
            // An empty description shows no tool tip.
            let tip = command.get_observable_with(XamlUICommand::description_property(), |description| {
                (!description.is_empty()).then(|| Rc::new(description) as BoxedValue)
            });
            bind(source, ToolTip::tip_property(), tip, undo);
        }
    }
}

fn bind_button(source: &FerroObject, command: &XamlUICommand, undo: &mut Vec<Rc<dyn IDisposable>>) {
    let object = source.to_ref();
    if let Some(button) = object.cast::<CommandBarButton>() {
        // AppBarButtonHelpers::OnCommandChanged: the label and the icon, not the content.
        if is_unset_text(button.label().as_deref()) {
            bind(source, CommandBarButton::label_property(), label_as_text(command), undo);
        }
        if button.icon().is_none() {
            bind(source, CommandBarButton::icon_property(), icon_as_element(command), undo);
        }
    } else if let Some(button) = object.cast::<CommandBarToggleButton>() {
        if is_unset_text(button.label().as_deref()) {
            bind(source, CommandBarToggleButton::label_property(), label_as_text(command), undo);
        }
        if button.icon().is_none() {
            bind(source, CommandBarToggleButton::icon_property(), icon_as_element(command), undo);
        }
    } else if let Some(button) = object.cast::<ContentControl>() {
        if is_unset_content(button.content().as_ref()) {
            bind(source, ContentControl::content_property(), label_as_content(command), undo);
        }
    }
    bind_key_gesture(source, command, Button::hot_key_property(), undo);
    bind_element_properties(source, command, undo);
}

fn bind_menu_item(source: &FerroObject, command: &XamlUICommand, undo: &mut Vec<Rc<dyn IDisposable>>) {
    let Some(item) = source.to_ref().cast::<MenuItem>() else {
        return;
    };
    if is_unset_content(item.header().as_ref()) {
        bind(source, HeaderedSelectingItemsControl::header_property(), label_as_content(command), undo);
    }
    if item.icon().is_none() {
        bind(source, MenuItem::icon_property(), icon_as_element(command), undo);
    }
    bind_key_gesture(source, command, MenuItem::hot_key_property(), undo);
    bind_key_gesture(source, command, MenuItem::input_gesture_property(), undo);
    bind_element_properties(source, command, undo);
}

fn bind_swipe_item(source: &FerroObject, command: &XamlUICommand, undo: &mut Vec<Rc<dyn IDisposable>>) {
    let Some(item) = source.to_ref().cast::<SwipeItem>() else {
        return;
    };
    // SwipeItem::OnPropertyChanged: the text and the icon source.
    if item.text().is_empty() {
        bind(source, SwipeItem::text_property(), command.get_observable(XamlUICommand::label_property()), undo);
    }
    if item.icon_source().is_none() {
        let icon_source = command.get_observable(XamlUICommand::icon_source_property());
        bind(source, SwipeItem::icon_source_property(), icon_source, undo);
    }
}
