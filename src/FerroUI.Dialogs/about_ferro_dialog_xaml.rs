//! The class of the document `AboutFerroDialog.xaml`.

use crate::markup::load_component;
use ferroui_base::input::InputElementImpl;
use ferroui_base::interactivity::{IRoutedEventArgs, InteractiveImpl, RoutedEventArgs};
use ferroui_base::layout::LayoutableImpl;
use ferroui_base::threading::Dispatcher;
use ferroui_base::utilities::{DateTime, Uri};
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, instantiate, BoxedValue, FerroObjectImpl, Ref,
    StyledElementImpl, VisualImpl,
};
use ferroui_controls::platform::PlatformManager;
use ferroui_controls::primitives::TemplatedControlImpl;
use ferroui_controls::{
    ContentControlImpl, Control, ControlImpl, TopLevelImpl, Window, WindowBaseImpl, WindowImpl,
};
use std::rc::Rc;
use std::sync::LazyLock;

/// The address the button of the dialog opens.
const PROJECT_URL: &str = "https://github.com/wieslawsoltes/FerroUI";

/// The version of the library (the assembly version of the original): the
/// four components of a version, missing ones zero.
static S_VERSION: LazyLock<[u32; 4]> = LazyLock::new(|| {
    let mut components = [0u32; 4];
    let numeric = env!("CARGO_PKG_VERSION").split(['-', '+']).next().unwrap_or_default();
    for (component, text) in components.iter_mut().zip(numeric.split('.')) {
        *component = text.parse().unwrap_or(0);
    }
    components
});

/// The about dialog of the framework: the logo, the version and a link to
/// the project.
#[repr(C)]
pub struct AboutFerroDialog {
    base: Window,
}

ferro_class!(AboutFerroDialog: Window);
ferro_impl_classes!(
    AboutFerroDialog: FerroObjectImpl,
    StyledElementImpl,
    VisualImpl,
    LayoutableImpl,
    InteractiveImpl,
    InputElementImpl,
    ControlImpl,
    TemplatedControlImpl,
    ContentControlImpl,
    TopLevelImpl,
    WindowBaseImpl,
    WindowImpl
);
ferro_class_info!(AboutFerroDialog {
    new: AboutFerroDialog::new,
    markup: {
        static_properties: [
            Version: String { get: AboutFerroDialog::version },
            IsDevelopmentBuild: bool { get: AboutFerroDialog::is_development_build },
            Copyright: String { get: AboutFerroDialog::copyright },
        ],
        methods: [
            fn Button_OnClick(Option<BoxedValue>, Rc<dyn IRoutedEventArgs>) =>
                |this: &Ref<AboutFerroDialog>, sender: Option<BoxedValue>, args: Rc<dyn IRoutedEventArgs>| {
                    this.button_on_click(&sender, args.as_routed_event_args())
                },
        ],
    },
});

impl AboutFerroDialog {
    /// The rooted asset path of the document of the class.
    pub const DOCUMENT_PATH: &'static str = "/AboutFerroDialog.xaml";

    /// The URI of the document of the class.
    pub const DOCUMENT_URI: &'static str = "ferres://FerroUI.Dialogs/AboutFerroDialog.xaml";

    pub fn construct() -> Self {
        Self { base: Window::construct(PlatformManager::create_window()) }
    }

    /// Creates the dialog and populates it from its document.
    ///
    /// # Panics
    /// Panics when no windowing platform is registered, and when the
    /// document fails to load.
    pub fn new() -> Ref<Self> {
        let this = instantiate(Self::construct());
        load_component(&this, Self::DOCUMENT_PATH);
        this.set_data_context(Some(Control::boxed(this.clone())));
        this
    }

    /// The version of the library: `v` and the major and minor versions.
    pub fn version() -> String {
        format!("v{}.{}", S_VERSION[0], S_VERSION[1])
    }

    /// Whether the library is a development build (revision 999).
    pub fn is_development_build() -> bool {
        S_VERSION[3] == 999
    }

    /// The copyright line, with the current year.
    pub fn copyright() -> String {
        format!("© {} The FerroUI Project", DateTime::now().year())
    }

    fn button_on_click(&self, _sender: &Option<BoxedValue>, _e: &RoutedEventArgs) {
        let url = Uri::absolute(PROJECT_URL).expect("the project address is an absolute URI");
        let launcher = self.launcher();
        Dispatcher::ui_thread().invoke_async_task_local(move || async move {
            launcher.launch_uri_async(&url).await;
        });
    }
}
