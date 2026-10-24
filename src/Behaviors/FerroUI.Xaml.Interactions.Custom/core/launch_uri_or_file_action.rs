//! Port of `Core/LaunchUriOrFileAction.cs`.

use ferroui_base::input::LocalBoxFuture;
use ferroui_base::platform::storage::ILauncher;
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::Uri;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroProperty, Ref,
    StyledElementImpl, StyledProperty, Visual,
};
use ferroui_controls::TopLevel;
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;
use ferroui_xaml_interactivity::{StyledElementAction, StyledElementActionImpl};
use std::rc::Rc;

/// An action that will launch a process to open a file or URI. For files, this action will
/// launch the default program for the given file extension. A URI will open in a web browser.
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference starts a process with shell execution
/// (`Process.Start`). The port opens the path with the launcher of the top level of the sender
/// (the platform service of the framework that opens a URI or a file with its default program):
/// an absolute URI is launched as a URI, any other path as a file. The action returns `false`
/// when the sender is not in a top level.
#[repr(C)]
pub struct LaunchUriOrFileAction {
    base: StyledElementAction,
}

ferro_class!(LaunchUriOrFileAction: StyledElementAction);
ferro_class_info!(LaunchUriOrFileAction { new: LaunchUriOrFileAction::new });
ferroui_base::ferro_impl_classes!(LaunchUriOrFileAction: FerroObjectImpl, StyledElementImpl);

ferro_properties! {
    impl LaunchUriOrFileAction {
        /// Identifies the `Path` property.
        pub fn path_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<LaunchUriOrFileAction, _>("Path", None)
        }
    }
}

impl StyledElementActionImpl for LaunchUriOrFileAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        if !this.is_enabled() {
            return Some(Rc::new(false));
        }

        let target = this.get_value(Self::path_property());
        let Some(target) = target.filter(|target| !target.trim().is_empty()) else {
            return Some(Rc::new(false));
        };

        let visual = sender.and_then(object_of).and_then(|sender| sender.cast::<Visual>());
        let Some(top_level) = visual.and_then(|visual| TopLevel::get_top_level(Some(&*visual))) else {
            return Some(Rc::new(false));
        };

        let launcher = top_level.launcher();
        let launch = match Uri::absolute(&target) {
            // A one letter scheme is the drive of a Windows path.
            Ok(uri) if uri.scheme().len() > 1 => Some(launcher.launch_uri_async(&uri)),
            _ => Self::launch_file(&*launcher, &target),
        };
        let Some(launch) = launch else {
            return Some(Rc::new(false));
        };

        drop(Dispatcher::ui_thread().to_task_scheduler().start_local(async move {
            let _ = launch.await;
        }));

        Some(Rc::new(true))
    }
}

impl LaunchUriOrFileAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: StyledElementAction::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the file or URI to open. This is a property of the object model.
    pub fn path(&self) -> Option<String> {
        self.get_value(Self::path_property())
    }

    pub fn set_path(&self, value: Option<String>) {
        self.set_value(Self::path_property(), value)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn launch_file(
        launcher: &(dyn ILauncher + 'static),
        path: &str,
    ) -> Option<LocalBoxFuture<bool>> {
        Some(launcher.launch_file_info_async(std::path::Path::new(path)))
    }

    /// The browser has no local files to open.
    #[cfg(target_arch = "wasm32")]
    fn launch_file(
        _launcher: &(dyn ILauncher + 'static),
        _path: &str,
    ) -> Option<LocalBoxFuture<bool>> {
        None
    }
}
