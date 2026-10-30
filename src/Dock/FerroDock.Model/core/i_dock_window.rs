// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.

use crate::controls::*;
use crate::core::*;
use std::rc::Rc;

/// The `IDockWindow` docking contract.
pub trait IDockWindow {
    /// Boxes the concrete enclosing reference for native class projections without cloning state.
    #[doc(hidden)]
    fn boxed_reference(self: Rc<Self>) -> crate::BoxedValue
    where
        Self: 'static,
    {
        Rc::new(self)
    }

    /// Native projection of the original concrete runtime name without reflection.
    #[doc(hidden)]
    fn runtime_type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
            .rsplit("::")
            .next()
            .expect("compiler type name")
    }

    /// Resolves the original optional property notifier without requiring it on every window.
    fn as_notify_property_changed(
        &self,
    ) -> Option<&dyn ferroui_system::component_model::INotifyPropertyChanged> {
        None
    }

    /// Gets or sets id.
    fn id(&self) -> String;
    /// Sets `Id`.
    fn set_id(&self, value: String);
    /// Gets or sets window X coordinate.
    fn x(&self) -> f64;
    /// Sets `X`.
    fn set_x(&self, value: f64);
    /// Gets or sets window X coordinate.
    fn y(&self) -> f64;
    /// Sets `Y`.
    fn set_y(&self, value: f64);
    /// Gets or sets window width.
    fn width(&self) -> f64;
    /// Sets `Width`.
    fn set_width(&self, value: f64);
    /// Gets or sets window height.
    fn height(&self) -> f64;
    /// Sets `Height`.
    fn set_height(&self, value: f64);
    /// Gets or sets the current window state.
    fn window_state(&self) -> DockWindowState;
    /// Sets `WindowState`.
    fn set_window_state(&self, value: DockWindowState);
    /// Gets or sets whether this window appears on top of all other windows.
    fn topmost(&self) -> bool;
    /// Sets `Topmost`.
    fn set_topmost(&self, value: bool);
    /// Gets or sets window title.
    fn title(&self) -> String;
    /// Sets `Title`.
    fn set_title(&self, value: String);
    /// Gets or sets the owner resolution mode.
    fn owner_mode(&self) -> DockWindowOwnerMode;
    /// Sets `OwnerMode`.
    fn set_owner_mode(&self, value: DockWindowOwnerMode);
    /// Gets or sets the parent window.
    fn parent_window(&self) -> Option<Rc<dyn IDockWindow>>;
    /// Sets `ParentWindow`.
    fn set_parent_window(&self, value: Option<Rc<dyn IDockWindow>>);
    /// Gets or sets a value indicating whether the window should be presented modally.
    fn is_modal(&self) -> bool;
    /// Sets `IsModal`.
    fn set_is_modal(&self, value: bool);
    /// Gets or sets a value indicating whether the window should appear in the taskbar.
    fn show_in_taskbar(&self) -> Option<bool>;
    /// Sets `ShowInTaskbar`.
    fn set_show_in_taskbar(&self, value: Option<bool>);
    /// Gets or sets window owner dockable.
    fn owner(&self) -> Option<Rc<dyn IDockable>>;
    /// Sets `Owner`.
    fn set_owner(&self, value: Option<Rc<dyn IDockable>>);
    /// Gets or sets dock factory.
    fn factory(&self) -> Option<Rc<dyn IFactory>>;
    /// Sets `Factory`.
    fn set_factory(&self, value: Option<Rc<dyn IFactory>>);
    /// Gets or sets layout.
    fn layout(&self) -> Option<Rc<dyn IRootDock>>;
    /// Sets `Layout`.
    fn set_layout(&self, value: Option<Rc<dyn IRootDock>>);
    /// Gets or sets dock window.
    fn host(&self) -> Option<Rc<dyn IHostWindow>>;
    /// Sets `Host`.
    fn set_host(&self, value: Option<Rc<dyn IHostWindow>>);
    /// Called when the window is closed.
    fn on_close(&self) -> bool;
    /// Called before the window dragging start.
    fn on_move_drag_begin(&self) -> bool;
    /// Called when the window is dragged.
    fn on_move_drag(&self);
    /// Called after the window dragging ended.
    fn on_move_drag_end(&self);
    /// Saves window properties.
    fn save(&self);
    /// Presents window.
    fn present(&self, is_dialog: bool);
    /// Exits window.
    fn exit(&self);
    /// Activates the window.
    fn set_active(&self);
}

impl PartialEq for dyn IDockWindow {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self as *const Self, other as *const Self)
    }
}
