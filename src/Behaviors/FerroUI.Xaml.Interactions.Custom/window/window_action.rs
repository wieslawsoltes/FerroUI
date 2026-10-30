//! Port of `Window/WindowAction.cs`.

use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_markup_enum, ferro_properties, instantiate, BoxedValue, FerroObjectImpl,
    FerroProperty, Ref, StyledElementImpl, StyledProperty,
};
use ferroui_controls::{Control, TopLevel, Window, WindowState};
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};

/// Defines the window action types.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum WindowActionType {
    /// Closes the window.
    #[default]
    Close,

    /// Minimizes the window.
    Minimize,

    /// Maximizes the window.
    Maximize,

    /// Restores the window.
    Restore,

    /// Toggles full screen mode.
    ToggleFullScreen,
}

impl std::fmt::Display for WindowActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Close => "Close",
            Self::Minimize => "Minimize",
            Self::Maximize => "Maximize",
            Self::Restore => "Restore",
            Self::ToggleFullScreen => "ToggleFullScreen",
        })
    }
}

ferro_markup_enum!(WindowActionType { Close, Minimize, Maximize, Restore, ToggleFullScreen });

/// An action that performs common window operations.
#[repr(C)]
pub struct WindowAction {
    base: StyledElementAction,
}

ferro_class!(WindowAction: StyledElementAction);
ferro_class_info!(WindowAction { new: WindowAction::new });
ferroui_base::ferro_impl_classes!(WindowAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl WindowAction {
        /// Identifies the `ActionType` property.
        pub fn action_type_property() -> StyledProperty<WindowActionType> {
            FerroProperty::register::<WindowAction, _>("ActionType", WindowActionType::Close)
        }
    }
}

impl StyledElementActionImpl for WindowAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let source = crate::part_c::sender_as::<Control>(sender)?;

        let window = TopLevel::get_top_level(Some(crate::part_c::control_as_visual(&source)))
            .and_then(|top_level| top_level.cast::<Window>())?;

        match this.action_type() {
            WindowActionType::Close => window.close(),
            WindowActionType::Minimize => {
                Dispatcher::ui_thread()
                    .post_local(move || window.set_window_state(WindowState::Minimized), DispatcherPriority::DEFAULT);
            }
            WindowActionType::Maximize => {
                Dispatcher::ui_thread()
                    .post_local(move || window.set_window_state(WindowState::Maximized), DispatcherPriority::DEFAULT);
            }
            WindowActionType::Restore => {
                Dispatcher::ui_thread()
                    .post_local(move || window.set_window_state(WindowState::Normal), DispatcherPriority::DEFAULT);
            }
            WindowActionType::ToggleFullScreen => {
                Dispatcher::ui_thread().post_local(
                    move || {
                        if window.window_state() == WindowState::FullScreen {
                            window.set_window_state(WindowState::Normal);
                        } else {
                            window.set_window_state(WindowState::FullScreen);
                        }
                    },
                    DispatcherPriority::DEFAULT,
                );
            }
        }

        None
    }
}

impl WindowAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the type of action to perform. This is a property of the object model.
    pub fn action_type(&self) -> WindowActionType {
        self.get_value(Self::action_type_property())
    }

    pub fn set_action_type(&self, value: WindowActionType) {
        self.set_value(Self::action_type_property(), value)
    }
}
