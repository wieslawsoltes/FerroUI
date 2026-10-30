//! Port of `FileUpload/UploadFileBehaviorBase.cs`.

use ferroui_base::{
    ferro_class, ferro_properties, BoxedValue, FerroObjectImpl, FerroObjectImplExt, FerroProperty, StyledElementImpl,
    StyledProperty, Visual,
};
use ferroui_xaml_interactivity::helpers::member_lookup::object_of;
use ferroui_xaml_interactivity::{InvokeCommandBehaviorBase, StyledElementBehaviorImpl};

/// Base class for behaviors that upload a file to a URL.
///
/// The class is abstract: a deriving class calls [`execute`](Self::execute) when its event fires.
///
/// Deviation (DEVIATIONS.md, Behaviors): the reference posts the file with `HttpClient` and runs
/// the command with the response. The framework has no HTTP client service, so the port sends no
/// request: [`execute`](Self::execute) checks what the reference checks (enabled, a command, a
/// file path, a URL, an existing file) and then ends as the reference ends when the request fails
/// (the error is ignored and the command does not run).
#[repr(C)]
pub struct UploadFileBehaviorBase {
    base: InvokeCommandBehaviorBase,
}

ferro_class!(UploadFileBehaviorBase: InvokeCommandBehaviorBase);
ferroui_base::ferro_impl_classes!(UploadFileBehaviorBase: StyledElementImpl, StyledElementBehaviorImpl);

ferro_properties! {
    impl UploadFileBehaviorBase {
        /// Identifies the `FilePath` property.
        pub fn file_path_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<UploadFileBehaviorBase, _>("FilePath", None)
        }

        /// Identifies the `Url` property.
        pub fn url_property() -> StyledProperty<Option<String>> {
            FerroProperty::register::<UploadFileBehaviorBase, _>("Url", None)
        }
    }
}

ferroui_base::ferro_overrides! {
    impl FerroObjectImpl for UploadFileBehaviorBase {
        /// The constructor body: the event args are passed to the command.
        fn constructed(this: &Self) {
            Self::parent_constructed(this);

            this.set_pass_event_args_to_command(true);
        }
    }
}

impl UploadFileBehaviorBase {
    /// Creates the class data; see [`ferroui_base::FerroObject::construct`].
    pub fn construct() -> Self {
        Self { base: InvokeCommandBehaviorBase::construct() }
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

    /// Executes the upload using the provided sender (the control that triggered the behavior)
    /// and command parameter.
    pub fn execute(&self, sender: Option<&BoxedValue>, _parameter: Option<&BoxedValue>) {
        let is_visual = sender.and_then(object_of).is_some_and(|sender| sender.cast::<Visual>().is_some());
        if !is_visual {
            return;
        }

        self.upload_async();
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

/// `File.Exists(path)`.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn file_exists(path: &str) -> bool {
    std::path::Path::new(path).is_file()
}

/// The browser has no local files.
#[cfg(target_arch = "wasm32")]
pub(crate) fn file_exists(_path: &str) -> bool {
    false
}
