//! Source layout shutdown callbacks, routed through a command outside view code.
use crate::view_models::MainWindowViewModel;
use ferroui_controls::{application_lifetimes::IClassicDesktopStyleApplicationLifetime, Window};
use ferroui_reactiveui::{ICommand, ReactiveCommand};
use std::rc::Rc;
pub(crate) struct DockLayoutLifetimeBehavior;
impl DockLayoutLifetimeBehavior {
    pub(crate) fn attach(
        window: &Window,
        lifetime: &dyn IClassicDesktopStyleApplicationLifetime,
        model: Rc<MainWindowViewModel>,
    ) {
        let close: Rc<dyn ICommand> =
            Rc::new(ReactiveCommand::create(move || model.close_layout()));
        let command = close.clone();
        window.closing(move |_| command.execute(None));
        lifetime.exit(Rc::new(move |_| close.execute(None)));
    }
}
