// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use ferroui_reactiveui::ICommand;
use ferroui_system::component_model::INotifyPropertyChanged;
use ferroui_system::threading::tasks::Task;
use ferroui_system::IDisposable;
use std::rc::Rc;
/// An asynchronous source busy/reload handler.
pub type BusyAction = Rc<dyn Fn() -> Task<()>>;
/// Per-host busy state and reload contract used by source overlay controls.
pub trait IDockBusyService: INotifyPropertyChanged {
    /// Gets whether this host has an active busy scope.
    fn is_busy(&self) -> bool;
    /// Gets the latest busy message.
    fn message(&self) -> Option<String>;
    /// Gets reload button visibility, independent of handler availability.
    fn is_reload_visible(&self) -> bool;
    /// Sets reload button visibility.
    fn set_is_reload_visible(&self, value: bool);
    /// Gets whether a reload handler is installed.
    fn can_reload(&self) -> bool;
    /// Gets the ReactiveCommand that executes reload work.
    fn reload_command(&self) -> Rc<dyn ICommand>;
    /// Begins a scope, ended only by explicit handle disposal.
    fn begin(&self, message: String) -> Rc<dyn IDisposable>;
    /// Runs work eagerly within a scope, ending it on success, error or cancellation.
    fn run_async(&self, message: String, action: BusyAction) -> Task<()>;
    /// Posts a busy message update.
    fn update_message(&self, message: Option<String>);
    /// Installs or clears the reload handler.
    fn set_reload_handler(&self, handler: Option<BusyAction>);
}

impl PartialEq for dyn IDockBusyService {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}
