//! WinUI's `SwipeItems`, `SwipeItem` and `SwipeItemInvokedEventArgs`
//! (`controls/dev/SwipeControl/SwipeItems.cpp`, `SwipeItem.cpp`, `SwipeItemInvokedEventArgs.cpp`,
//! `SwipeControl.idl` of microsoft-ui-xaml commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT,
//! Copyright (c) Microsoft Corporation, see NOTICE.md).

use super::SwipeControl;
use crate::IconSource;
use ferroui_base::collections::FerroList;
use ferroui_base::input::ICommand;
use ferroui_base::media::IBrush;
use ferroui_base::reactive::{Disposable, Subscription};
use ferroui_base::utilities::HandlerList;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_type, ferro_properties, instantiate, BoxedValue,
    FerroObject, FerroObjectImpl, FerroProperty, Ref, StyledProperty,
};
use std::rc::Rc;

/// How the items of a side of a [`SwipeControl`] behave (WinUI's `SwipeMode`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SwipeMode {
    /// Swiping reveals the items; the user then invokes one.
    #[default]
    Reveal = 0,
    /// Swiping past the threshold invokes the single item.
    Execute = 1,
}

ferroui_base::ferro_markup_enum!(SwipeMode { Reveal, Execute }, { namespace: "FerroUI.Controls" });

/// What a [`SwipeControl`] does after one of its items is invoked (WinUI's
/// `SwipeBehaviorOnInvoked`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum SwipeBehaviorOnInvoked {
    /// Closes: the same as [`Close`](Self::Close) (WinUI's default).
    #[default]
    Auto = 0,
    /// Closes the swipe control.
    Close = 1,
    /// Leaves the swipe control open.
    RemainOpen = 2,
}

ferroui_base::ferro_markup_enum!(SwipeBehaviorOnInvoked { Auto, Close, RemainOpen }, { namespace: "FerroUI.Controls" });

/// Why a change of a [`SwipeItems`] was refused: execute items have one item at most (WinUI
/// throws `hresult_invalid_argument`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExecuteItemsHaveOneItem;

impl std::fmt::Display for ExecuteItemsHaveOneItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Execute items should only have one item.")
    }
}

impl std::error::Error for ExecuteItemsHaveOneItem {}

/// The arguments of [`SwipeItem::invoked`] (WinUI's `SwipeItemInvokedEventArgs`).
#[derive(Clone, Debug, PartialEq)]
pub struct SwipeItemInvokedEventArgs {
    swipe_control: Ref<SwipeControl>,
}

impl SwipeItemInvokedEventArgs {
    /// The swipe control whose item was invoked.
    pub fn swipe_control(&self) -> Ref<SwipeControl> {
        self.swipe_control.clone()
    }
}

ferro_markup_type!(class SwipeItemInvokedEventArgs {
    namespace: "FerroUI.Controls",
    handles: [SwipeItemInvokedEventArgs],
    properties: [
        SwipeControl: Ref<SwipeControl> { get: |args: &SwipeItemInvokedEventArgs| args.swipe_control() },
    ],
});

type InvokedHandler = dyn Fn(&Ref<SwipeItem>, &SwipeItemInvokedEventArgs);

/// A command of a side of a [`SwipeControl`] (WinUI's `SwipeItem`): drawn as a button with its
/// [`text`](Self::text), [`icon_source`](Self::icon_source), [`background`](Self::background) and
/// [`foreground`](Self::foreground); invoking it raises [`invoked`](Self::invoked), runs its
/// command and closes the control unless [`behavior_on_invoked`](Self::behavior_on_invoked) is
/// [`SwipeBehaviorOnInvoked::RemainOpen`].
#[repr(C)]
pub struct SwipeItem {
    base: FerroObject,
    invoked: HandlerList<InvokedHandler>,
}

ferro_class!(SwipeItem: FerroObject);
ferro_class_info!(SwipeItem { new: SwipeItem::new, markup: { namespace: "FerroUI.Controls" } });
ferro_impl_classes!(SwipeItem: FerroObjectImpl);

ferro_properties! { impl SwipeItem {
    /// Defines the `Text` property: the label of the item. Default empty.
    pub fn text_property() -> StyledProperty<String> {
        FerroProperty::register::<SwipeItem, _>("Text", String::new())
    }

    /// Defines the `IconSource` property: the icon of the item. Default `None`.
    pub fn icon_source_property() -> StyledProperty<Option<Ref<IconSource>>> {
        FerroProperty::register::<SwipeItem, _>("IconSource", None)
    }

    /// Defines the `Background` property: the background of the item (the theme's
    /// `SwipeItemBackground` when `None`).
    pub fn background_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<SwipeItem, _>("Background", None)
    }

    /// Defines the `Foreground` property: the foreground of the item (the theme's
    /// `SwipeItemForeground` when `None`).
    pub fn foreground_property() -> StyledProperty<Option<Rc<dyn IBrush>>> {
        FerroProperty::register::<SwipeItem, _>("Foreground", None)
    }

    /// Defines the `Command` property: the command run when the item is invoked.
    pub fn command_property() -> StyledProperty<Option<Rc<dyn ICommand>>> {
        FerroProperty::register::<SwipeItem, _>("Command", None)
    }

    /// Defines the `CommandParameter` property.
    pub fn command_parameter_property() -> StyledProperty<Option<BoxedValue>> {
        FerroProperty::register::<SwipeItem, _>("CommandParameter", None)
    }

    /// Defines the `BehaviorOnInvoked` property. Default [`SwipeBehaviorOnInvoked::Auto`].
    pub fn behavior_on_invoked_property() -> StyledProperty<SwipeBehaviorOnInvoked> {
        FerroProperty::register::<SwipeItem, _>("BehaviorOnInvoked", SwipeBehaviorOnInvoked::Auto)
    }
} }

impl SwipeItem {
    pub fn construct() -> Self {
        Self { base: FerroObject::construct(), invoked: HandlerList::new() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates an item with a text.
    pub fn with_text(text: impl Into<String>) -> Ref<Self> {
        let item = Self::new();
        item.set_text(text);
        item
    }

    pub fn text(&self) -> String {
        self.get_value(Self::text_property())
    }

    pub fn set_text(&self, value: impl Into<String>) {
        self.set_value(Self::text_property(), value.into());
    }

    pub fn icon_source(&self) -> Option<Ref<IconSource>> {
        self.get_value(Self::icon_source_property())
    }

    pub fn set_icon_source(&self, value: Option<Ref<IconSource>>) {
        self.set_value(Self::icon_source_property(), value);
    }

    pub fn background(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::background_property())
    }

    pub fn set_background(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::background_property(), value);
    }

    pub fn foreground(&self) -> Option<Rc<dyn IBrush>> {
        self.get_value(Self::foreground_property())
    }

    pub fn set_foreground(&self, value: Option<Rc<dyn IBrush>>) {
        self.set_value(Self::foreground_property(), value);
    }

    pub fn command(&self) -> Option<Rc<dyn ICommand>> {
        self.get_value(Self::command_property())
    }

    pub fn set_command(&self, value: Option<Rc<dyn ICommand>>) {
        self.set_value(Self::command_property(), value);
    }

    pub fn command_parameter(&self) -> Option<BoxedValue> {
        self.get_value(Self::command_parameter_property())
    }

    pub fn set_command_parameter(&self, value: Option<BoxedValue>) {
        self.set_value(Self::command_parameter_property(), value);
    }

    pub fn behavior_on_invoked(&self) -> SwipeBehaviorOnInvoked {
        self.get_value(Self::behavior_on_invoked_property())
    }

    pub fn set_behavior_on_invoked(&self, value: SwipeBehaviorOnInvoked) {
        self.set_value(Self::behavior_on_invoked_property(), value);
    }

    /// Adds a handler of `Invoked`. Dropping the returned guard removes it;
    /// [`Subscription::detach`] keeps it for the life of the item.
    pub fn invoked(&self, handler: impl Fn(&Ref<SwipeItem>, &SwipeItemInvokedEventArgs) + 'static) -> Subscription {
        let token = self.invoked.add(Rc::new(handler));
        let weak = self.to_ref().downgrade();
        Subscription::new(Disposable::create(move || {
            if let Some(this) = weak.upgrade() {
                this.invoked.remove(token);
            }
        }))
    }

    /// Invokes the item from `swipe_control` (WinUI's `InvokeSwipe`): raises `Invoked`, runs the
    /// command when it can run, and closes the control for `Auto` and `Close`.
    pub fn invoke_swipe(&self, swipe_control: &Ref<SwipeControl>) {
        let this = self.to_ref();
        let args = SwipeItemInvokedEventArgs { swipe_control: swipe_control.clone() };
        for (_, handler) in self.invoked.snapshot().iter() {
            handler(&this, &args);
        }
        if let Some(command) = self.command() {
            let parameter = self.command_parameter();
            if command.can_execute(parameter.as_ref()) {
                command.execute(parameter.as_ref());
            }
        }
        if matches!(self.behavior_on_invoked(), SwipeBehaviorOnInvoked::Close | SwipeBehaviorOnInvoked::Auto) {
            swipe_control.close();
        }
    }
}

/// The items of a side of a [`SwipeControl`] with their [`mode`](Self::mode) (WinUI's
/// `SwipeItems`, a vector of [`SwipeItem`]s).
///
/// Execute items have one item at most: [`try_add`](Self::try_add) and
/// [`try_set_mode`](Self::try_set_mode) refuse a change that breaks it (WinUI throws);
/// [`add`](Self::add) logs and ignores such an item. Items added to [`items`](Self::items)
/// directly (as markup does) are not checked; a swipe control uses the first item of execute
/// items.
#[repr(C)]
pub struct SwipeItems {
    base: FerroObject,
    items: FerroList<Ref<SwipeItem>>,
}

ferro_class!(SwipeItems: FerroObject);
ferro_class_info!(SwipeItems {
    new: SwipeItems::new,
    markup: {
        namespace: "FerroUI.Controls",
        content: Items,
        properties: [
            Items: FerroList<Ref<SwipeItem>> { get: SwipeItems::items },
        ],
    },
});
ferro_impl_classes!(SwipeItems: FerroObjectImpl);

ferro_properties! { impl SwipeItems {
    /// Defines the `Mode` property. Default [`SwipeMode::Reveal`].
    pub fn mode_property() -> StyledProperty<SwipeMode> {
        FerroProperty::register::<SwipeItems, _>("Mode", SwipeMode::Reveal)
    }
} }

impl SwipeItems {
    pub fn construct() -> Self {
        Self { base: FerroObject::construct(), items: FerroList::new() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Creates items of a mode.
    pub fn with_mode(mode: SwipeMode) -> Ref<Self> {
        let items = Self::new();
        items.set_value(Self::mode_property(), mode);
        items
    }

    /// The items (the vector of WinUI's `SwipeItems`).
    pub fn items(&self) -> FerroList<Ref<SwipeItem>> {
        self.items.clone()
    }

    pub fn count(&self) -> usize {
        self.items.count()
    }

    pub fn get(&self, index: usize) -> Option<Ref<SwipeItem>> {
        self.items.snapshot().get(index).cloned()
    }

    pub fn mode(&self) -> SwipeMode {
        self.get_value(Self::mode_property())
    }

    /// Sets the mode; execute mode with more than one item is refused.
    pub fn try_set_mode(&self, value: SwipeMode) -> Result<(), ExecuteItemsHaveOneItem> {
        if value == SwipeMode::Execute && self.count() > 1 {
            return Err(ExecuteItemsHaveOneItem);
        }
        self.set_value(Self::mode_property(), value);
        Ok(())
    }

    /// Adds an item; a second item of execute items is refused.
    pub fn try_add(&self, item: Ref<SwipeItem>) -> Result<(), ExecuteItemsHaveOneItem> {
        if self.mode() == SwipeMode::Execute && self.count() > 0 {
            return Err(ExecuteItemsHaveOneItem);
        }
        self.items.add(item);
        Ok(())
    }

    /// Adds an item; a second item of execute items is logged and ignored.
    pub fn add(&self, item: Ref<SwipeItem>) {
        if let Err(error) = self.try_add(item) {
            super::swipe_control::warn(&error.to_string());
        }
    }
}
