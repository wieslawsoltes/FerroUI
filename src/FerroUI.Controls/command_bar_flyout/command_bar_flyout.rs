//! WinUI's `CommandBarFlyout` (`controls/dev/CommandBarFlyout/CommandBarFlyout.cpp` of
//! microsoft-ui-xaml commit `7e4b0c2096be402f51cc8f7d203ebf67cff183d0`; MIT, Copyright (c)
//! Microsoft Corporation, see NOTICE.md).

use super::CommandBarFlyoutCommandBar;
use crate::command_bar::{
    CommandBarButton, CommandBarElementList, CommandBarOverflowButtonVisibility, CommandBarToggleButton,
    ICommandBarElement,
};
use crate::flyouts::{
    FlyoutBaseImpl, FlyoutBaseImplExt, FlyoutPresenter, FlyoutShowMode, PopupFlyoutBase, PopupFlyoutBaseImpl,
    PopupFlyoutBaseImplExt,
};
use crate::Control;
use ferroui_base::collections::FerroList;
use ferroui_base::interactivity::RoutedEventHandlerToken;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::utilities::CancelEventArgs;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, FerroObjectImpl, FerroProperty, Ref, StyledProperty,
    Thickness, WeakRef,
};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// A flyout of commands (WinUI's `CommandBarFlyout`): its [`primary_commands`](Self::primary_commands)
/// are a row of buttons, its [`secondary_commands`](Self::secondary_commands) a menu below them
/// that the "more" button opens.
///
/// The flyout shows a [`CommandBarFlyoutCommandBar`] with the same elements. Shown with the
/// standard show mode, the bar opens with the flyout (the secondary commands are shown); shown
/// with the transient show mode, it shows the primary commands until the "more" button opens it.
/// [`always_expanded`](Self::always_expanded) keeps the secondary commands shown without the "more"
/// button. A click of a secondary command closes the flyout, unless the command opens a flyout of
/// its own.
#[repr(C)]
pub struct CommandBarFlyout {
    base: PopupFlyoutBase,
    primary_commands: CommandBarElementList,
    secondary_commands: CommandBarElementList,
    command_bar: RefCell<Option<WeakRef<CommandBarFlyoutCommandBar>>>,
    /// The click handlers of the secondary commands that close the flyout
    /// (`SetSecondaryCommandsToCloseWhenExecuted`).
    secondary_clicks: RefCell<Vec<(WeakRef<Control>, RoutedEventHandlerToken)>>,
    /// The flyout itself hides after the bar's close (`m_isClosingAfterCloseAnimation`).
    is_closing: Cell<bool>,
}

ferro_class!(CommandBarFlyout: PopupFlyoutBase);
ferro_class_info!(CommandBarFlyout {
    new: CommandBarFlyout::new,
    markup: {
        namespace: "FerroUI.Controls",
        content: PrimaryCommands,
        properties: [
            PrimaryCommands: CommandBarElementList { get: CommandBarFlyout::primary_commands },
            SecondaryCommands: CommandBarElementList { get: CommandBarFlyout::secondary_commands },
        ],
    },
});

impl FerroObjectImpl for CommandBarFlyout {}

ferro_properties! { impl CommandBarFlyout {
    /// Defines the `AlwaysExpanded` property: the secondary commands are shown whenever the flyout
    /// is, without the "more" button. Default `false`.
    pub fn always_expanded_property() -> StyledProperty<bool> {
        FerroProperty::register::<CommandBarFlyout, _>("AlwaysExpanded", false)
    }
} }

impl FlyoutBaseImpl for CommandBarFlyout {
    // The Opened handler of the constructor.
    fn on_opened(this: &Self) {
        Self::parent_on_opened(this);
    }

    // The Closed handler of the constructor: the bar closes with the flyout.
    fn on_closed(this: &Self) {
        if let Some(bar) = this.command_bar() {
            if bar.is_open() {
                bar.set_is_open(false);
            }
        }
        Self::parent_on_closed(this);
    }
}

impl PopupFlyoutBaseImpl for CommandBarFlyout {
    // CommandBarFlyout::CreatePresenter: a flyout presenter without chrome around a new command bar
    // with the flyout's elements.
    fn create_presenter(this: &Self) -> Ref<Control> {
        let bar = CommandBarFlyoutCommandBar::new();
        this.sync_commands(&bar);
        let presenter = FlyoutPresenter::new();
        presenter.set_background(None);
        presenter.set_border_brush(None);
        presenter.set_min_width(0.0);
        presenter.set_max_width(f64::INFINITY);
        presenter.set_min_height(0.0);
        presenter.set_max_height(f64::INFINITY);
        presenter.set_border_thickness(Thickness::uniform(0.0));
        presenter.set_padding(Thickness::uniform(0.0));
        presenter.set_corner_radius(Default::default());
        presenter.set_content(Some(Control::boxed(bar.clone())));
        // m_commandBarOpenedRevoker: once the bar is opened, the flyout is a standard one.
        let weak = this.to_ref().downgrade();
        bar.opened(move |_, _| {
            if let Some(flyout) = weak.upgrade() {
                flyout.set_show_mode(FlyoutShowMode::Standard);
            }
        });
        bar.set_owning_flyout(&this.to_ref());
        *this.command_bar.borrow_mut() = Some(bar.downgrade());
        presenter.upcast()
    }

    // The Opening handler of the constructor.
    fn on_opening(this: &Self, args: &CancelEventArgs) {
        Self::parent_on_opening(this, args);
        if args.cancel() {
            return;
        }
        let Some(bar) = this.command_bar() else {
            return;
        };
        if this.always_expanded() {
            bar.set_overflow_button_visibility(CommandBarOverflowButtonVisibility::Collapsed);
            this.set_show_mode(FlyoutShowMode::Standard);
        } else {
            bar.set_overflow_button_visibility(CommandBarOverflowButtonVisibility::Auto);
        }
        // QueueCallbackForCompositionRendering: the bar opens once the popup is shown.
        let weak = this.to_ref().downgrade();
        Dispatcher::ui_thread().post_local(
            move || {
                let Some(flyout) = weak.upgrade() else {
                    return;
                };
                if flyout.is_open() && flyout.show_mode() == FlyoutShowMode::Standard {
                    if let Some(bar) = flyout.command_bar() {
                        bar.set_is_open(true);
                    }
                }
            },
            DispatcherPriority::RENDER,
        );
    }

    // The Closing handler of the constructor: the bar closes first.
    fn on_closing(this: &Self, args: &CancelEventArgs) {
        Self::parent_on_closing(this, args);
        if args.cancel() || this.is_closing.get() {
            return;
        }
        if let Some(bar) = this.command_bar() {
            bar.set_is_open(false);
        }
    }
}

impl CommandBarFlyout {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self {
            base: PopupFlyoutBase::construct(),
            primary_commands: FerroList::new(),
            secondary_commands: FerroList::new(),
            command_bar: RefCell::new(None),
            secondary_clicks: RefCell::new(Vec::new()),
            is_closing: Cell::new(false),
        }
    }

    pub fn new() -> Ref<Self> {
        let flyout = instantiate(Self::construct());
        // The constructor: changes of the flyout's lists reach the bar.
        for list in [flyout.primary_commands(), flyout.secondary_commands()] {
            let weak = flyout.downgrade();
            list.add_collection_changed(Rc::new(move |_| {
                if let Some(flyout) = weak.upgrade() {
                    if let Some(bar) = flyout.command_bar() {
                        flyout.sync_commands(&bar);
                    }
                }
            }));
        }
        flyout
    }

    /// The commands of the row (WinUI's `PrimaryCommands`).
    pub fn primary_commands(&self) -> CommandBarElementList {
        self.primary_commands.clone()
    }

    /// The commands of the menu below the row (WinUI's `SecondaryCommands`).
    pub fn secondary_commands(&self) -> CommandBarElementList {
        self.secondary_commands.clone()
    }

    pub fn always_expanded(&self) -> bool {
        self.get_value(Self::always_expanded_property())
    }

    pub fn set_always_expanded(&self, value: bool) {
        self.set_value(Self::always_expanded_property(), value);
    }

    /// The command bar the flyout shows, once it was created.
    pub fn command_bar(&self) -> Option<Ref<CommandBarFlyoutCommandBar>> {
        self.command_bar.borrow().as_ref().and_then(WeakRef::upgrade)
    }

    /// `SharedHelpers::CopyVector` of both lists, then `SetSecondaryCommandsToCloseWhenExecuted`.
    fn sync_commands(&self, bar: &Ref<CommandBarFlyoutCommandBar>) {
        for (source, target) in
            [(&self.primary_commands, bar.primary_commands()), (&self.secondary_commands, bar.secondary_commands())]
        {
            let elements = source.snapshot();
            if *target.snapshot() != *elements {
                target.clear();
                for element in elements.iter() {
                    target.add(element.clone());
                }
            }
        }
        self.set_secondary_commands_to_close_when_executed();
    }

    /// A click of a secondary button without a flyout, or of a secondary toggle button, hides the
    /// flyout (WinUI hides on the toggle button's Checked and Unchecked, which its click raises).
    fn set_secondary_commands_to_close_when_executed(&self) {
        for (control, token) in self.secondary_clicks.borrow_mut().drain(..) {
            if let Some(control) = control.upgrade() {
                control.remove_handler(crate::Button::click_event(), token);
            }
        }
        let mut clicks = Vec::new();
        for element in self.secondary_commands.snapshot().iter() {
            let Some(control) = element_control(element) else {
                continue;
            };
            let closes = match control.cast::<CommandBarButton>() {
                Some(button) => button.flyout().is_none(),
                None => control.cast::<CommandBarToggleButton>().is_some(),
            };
            if !closes {
                continue;
            }
            let weak = self.to_ref().downgrade();
            let token = control.add_handler(crate::Button::click_event(), move |_, _| {
                if let Some(flyout) = weak.upgrade() {
                    flyout.hide();
                }
            });
            clicks.push((control.downgrade(), token));
        }
        *self.secondary_clicks.borrow_mut() = clicks;
    }
}

/// The control of a command bar element, if it is one.
fn element_control(element: &Rc<dyn ICommandBarElement>) -> Option<Ref<Control>> {
    element.as_object()?.to_ref().cast::<Control>()
}
