//! The sample host of the reusable native designer. UI, session, AST and tooling live in crates.
use ferroui_base::{
    FerroObjectImpl, Ref, ferro_class, ferro_class_info, ferro_impl_classes, instantiate,
};
use ferroui_controls::{Application, ApplicationImpl, ApplicationImplExt, NewApplication, Window};
use ferroui_designer::DesignerWorkspace;
use std::cell::RefCell;
use std::rc::Rc;

ferroui_base::ferro_thread_local! {
    static START_SOURCE:RefCell<Option<(String,Option<String>)>>=const{RefCell::new(None)};
    static START_PROJECT:RefCell<Option<std::path::PathBuf>>=const{RefCell::new(None)};
    static PREVIEW_OPTIONS:RefCell<Option<ferroui_designer::PreviewHostOptions>>=const{RefCell::new(None)};
}
pub fn set_initial_project(path: std::path::PathBuf) {
    START_PROJECT.with(|value| *value.borrow_mut() = Some(path));
}
/// Register application metadata and real code-behind factories before starting the host.
pub fn set_preview_options(options: ferroui_designer::PreviewHostOptions) {
    PREVIEW_OPTIONS.with(|value| *value.borrow_mut() = Some(options));
}
#[cfg(feature = "mcp")]
ferroui_base::ferro_thread_local! {
    static MCP_OPTIONS: RefCell<Option<ferroui_designer::mcp::Options>> = const { RefCell::new(None) };
}
#[cfg(feature = "mcp")]
pub fn set_mcp_options(options: ferroui_designer::mcp::Options) {
    MCP_OPTIONS.with(|value| *value.borrow_mut() = Some(options));
}

/// Supply source before creating the application. Each host can load a project using its own IO.
pub fn set_initial_source(source: String) {
    START_SOURCE.with(|value| *value.borrow_mut() = Some((source, None)));
}

/// Open a source document with its original URI so relative includes remain meaningful.
pub fn set_initial_document(source: String, uri: String) {
    START_SOURCE.with(|value| *value.borrow_mut() = Some((source, Some(uri))));
}
#[repr(C)]
pub struct App {
    base: Application,
    workspace: RefCell<Option<Rc<DesignerWorkspace>>>,
    #[cfg(feature = "mcp")]
    mcp: RefCell<Option<ferroui_designer::mcp::ServerHandle>>,
}
ferro_class!(App:Application);
ferro_class_info!(App { new: App::new });
ferro_impl_classes!(App:FerroObjectImpl);
impl App {
    pub fn new() -> Ref<Self> {
        instantiate(Self {
            base: Application::construct(),
            workspace: RefCell::new(None),
            #[cfg(feature = "mcp")]
            mcp: RefCell::new(None),
        })
    }
    pub fn workspace(&self) -> Option<Rc<DesignerWorkspace>> {
        self.workspace.borrow().clone()
    }
}
impl NewApplication for App {
    fn new_application() -> Ref<Self> {
        Self::new()
    }
}
impl ApplicationImpl for App {
    fn initialize(this: &Self) {
        ferroui_markup_xaml::register_types();
        ferroui_themes_simple::register_types();
        this.styles()
            .add(ferroui_themes_simple::SimpleTheme::new().as_style());
        this.set_requested_theme_variant(Some(ferroui_base::styling::ThemeVariant::dark()));
    }
    fn on_framework_initialization_completed(this: &Self) {
        let (source, uri) = START_SOURCE
            .with(|value| value.borrow_mut().take())
            .unwrap_or_else(|| (ferroui_designer::STARTER_XAML.to_owned(), None));
        let workspace = match uri {
            Some(uri) => DesignerWorkspace::with_uri(source, uri),
            None => DesignerWorkspace::new(source),
        }
        .unwrap_or_else(|error| panic!("Designer initialization: {error}"));
        if let Some(options) = PREVIEW_OPTIONS.with(|value| value.borrow_mut().take()) {
            if let Err(error) = workspace.session().borrow_mut().configure_preview(options) {
                eprintln!("Designer preview: {error}");
            }
        }
        if let Some(path) = START_PROJECT.with(|value| value.borrow_mut().take()) {
            if let Err(error) = workspace.open_solution(path) {
                eprintln!("Designer solution: {error}");
            }
        }
        if let Some(lifetime) = this.application_lifetime() {
            if let Some(desktop) = lifetime.as_classic_desktop_style_application_lifetime() {
                let window = Window::new();
                window.set_title(Some("FerroUI Designer Studio".into()));
                window.set_width(1440.0);
                window.set_height(960.0);
                window.set_min_width(960.0);
                window.set_min_height(640.0);
                window.set_content(Some(Rc::new(workspace.control())));
                let close_approved = Rc::new(std::cell::Cell::new(false));
                let weak_workspace = Rc::downgrade(&workspace);
                let weak_window = window.downgrade();
                window.closing(move |event| {
                    if close_approved.get() {
                        return;
                    }
                    let (Some(workspace), Some(window)) =
                        (weak_workspace.upgrade(), weak_window.upgrade())
                    else {
                        return;
                    };
                    let approved = close_approved.clone();
                    let weak_window = window.downgrade();
                    if workspace.request_close(move || {
                        approved.set(true);
                        if let Some(window) = weak_window.upgrade() {
                            window.close();
                        }
                    }) {
                        event.set_cancel(true);
                    }
                });
                desktop.set_main_window(Some(window));
            } else if let Some(single) = lifetime.as_single_view_application_lifetime() {
                single.set_main_view(Some(workspace.control()));
            }
        }
        #[cfg(feature = "mcp")]
        if let Some(options) = MCP_OPTIONS.with(|value| value.borrow_mut().take()) {
            match workspace.serve_workspace_mcp(options) {
                Ok(server) => *this.mcp.borrow_mut() = Some(server),
                Err(error) => eprintln!("Designer MCP: {error}"),
            }
        }
        *this.workspace.borrow_mut() = Some(workspace);
        Self::parent_on_framework_initialization_completed(this);
    }
}
