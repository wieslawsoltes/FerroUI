//! Port of `FileUpload/UploadFileAction.cs`.

use super::upload_file_behavior_base::file_exists;
use ferroui_base::threading::{Dispatcher, DispatcherPriority};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_properties, instantiate, BoxedValue, FerroObjectImpl, FerroObjectImplExt,
    FerroProperty, Ref, StyledElementImpl, StyledProperty, Visual,
};
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;
use ferroui_xaml_interactivity::{InvokeCommandActionBase, StyledElementActionImpl};
use std::rc::Rc;

/// Asynchronously uploads a file to a specified URL and invokes a command when completed.
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference posts the file with `HttpClient` and runs
/// the command with the response. The framework has no HTTP client service, so the port sends no
/// request: the queued upload checks what the reference checks (enabled, a command, a file path,
/// a URL, an existing file) and then ends as the reference ends when the request fails (the error
/// is ignored and the command does not run).
#[repr(C)]
pub struct UploadFileAction {
    base: InvokeCommandActionBase,
}

ferro_class!(UploadFileAction: InvokeCommandActionBase);
ferro_class_info!(UploadFileAction { new: UploadFileAction::new });
ferroui_base::ferro_impl_classes!(UploadFileAction: StyledElementImpl);

ferro_properties! {
    impl UploadFileAction {
        /// Identifies the `FilePath` property.
        pub fn file_path_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<UploadFileAction, _>("FilePath", None)
        }

        /// Identifies the `Url` property.
        pub fn url_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<UploadFileAction, _>("Url", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for UploadFileAction {
        /// The constructor body: the event args are passed to the command.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);

            this.set_pass_event_args_to_command(true);
        }
    }
}

impl StyledElementActionImpl for UploadFileAction {
    fn execute(this: &Self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) -> Option<BoxedValue> {
        let is_visual = sender.and_then(object_of).is_some_and(|sender| sender.cast::<Visual>().is_some());
        if !is_visual {
            return Some(Rc::new(false));
        }

        let action: Ref<UploadFileAction> = this.to_ref();
        Dispatcher::ui_thread().post_local(move || action.upload_async(), DispatcherPriority::DEFAULT);

        Some(Rc::new(true))
    }
}

impl UploadFileAction {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InvokeCommandActionBase::construct() }
    }

    /// Creates an action.
    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    /// Gets or sets the path of the file to upload. This is a property of the object model.
    pub fn file_path(&self) -> Option<String> {
        self.get_value(Self::file_path_property())
    }

    pub fn set_file_path(&self, value: Option<String>) {
        self.set_value(Self::file_path_property(), value)
    }

    /// Gets or sets the destination URL. This is a property of the object model.
    pub fn url(&self) -> Option<String> {
        self.get_value(Self::url_property())
    }

    pub fn set_url(&self, value: Option<String>) {
        self.set_value(Self::url_property(), value)
    }

    fn upload_async(&self) {
        if !self.is_enabled() || self.command().is_none() {
            return;
        }

        let (Some(file_path), Some(_url)) = (self.file_path(), self.url()) else {
            return;
        };

        if !file_exists(&file_path) {
            return;
        }

        // Deviation (DEVIATIONS.md, Behaviors): no HTTP client service in the framework; the
        // request is not sent and, as for a failed request in the reference, the command does
        // not run.
    }
}
