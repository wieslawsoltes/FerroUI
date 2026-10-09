use crate::remote::PreviewerWindowImpl;
use ferroui_base::metadata::{from_markup_value, MarkupAssembly};
use ferroui_base::platform::ASSET_SCHEME;
use ferroui_base::reactive::IDisposable;
use ferroui_base::styling::IStyle;
use ferroui_base::utilities::{Uri, UriKind};
use ferroui_base::{BoxedValue, FerroLocator, FerroObject, LocatorExtensions, Ref};
use ferroui_controls::embedding::offscreen::OffscreenTopLevelImplBase;
use ferroui_controls::platform::PlatformManager;
use ferroui_controls::templates::IDataTemplate;
use ferroui_controls::{Control, Design, PreviewTarget, SizeToContent, Window};
use ferroui_markup_xaml::{
    IRuntimeXamlLoader, RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument, XamlLoadException,
};
use std::path::Path;
use std::rc::Rc;

/// The metadata entry of an assembly that says whether its bindings are
/// compiled by default.
const USE_COMPILED_BINDINGS_BY_DEFAULT: &str = "FerroUseCompiledBindingsByDefault";

/// `using (PlatformManager.DesignerMode())`.
struct DesignerMode(Rc<dyn IDisposable>);

impl Drop for DesignerMode {
    fn drop(&mut self) {
        self.0.dispose();
    }
}

/// Loads the text of a XAML document into the window the previewer shows.
pub struct DesignWindowLoader;

impl DesignWindowLoader {
    /// `LoadDesignerWindow(xaml, assemblyPath, xamlFileProjectPath)`: with a
    /// render scaling of one.
    pub fn load_designer_window(
        xaml: &str,
        assembly_path: Option<&str>,
        xaml_file_project_path: Option<&str>,
    ) -> Result<Ref<Window>, XamlLoadException> {
        Self::load_designer_window_with_scaling(xaml, assembly_path, xaml_file_project_path, 1.0)
    }

    /// `LoadDesignerWindow(xaml, assemblyPath, xamlFileProjectPath, renderScaling)`.
    ///
    /// The document is loaded by the run-time XAML loader in design mode.
    /// What it builds is previewed as `Design.CreatePreviewWithControl`
    /// decides: a window is the window, a control is the content of a new
    /// window, a style, a resource dictionary or a data template is hosted
    /// in the control `Design.PreviewWith` names, and anything else is a
    /// message. The design-time properties of the document (`Design.Width`,
    /// `Design.Height`, `Design.DataContext`, `Design.DesignStyle`) are
    /// applied to the window, which is then shown.
    ///
    /// # Panics
    /// Panics when no run-time XAML loader is registered.
    pub fn load_designer_window_with_scaling(
        xaml: &str,
        assembly_path: Option<&str>,
        xaml_file_project_path: Option<&str>,
        render_scaling: f64,
    ) -> Result<Ref<Window>, XamlLoadException> {
        let window;
        {
            let _designer_mode = DesignerMode(PlatformManager::designer_mode());
            let loader = FerroLocator::current().get_required_service::<dyn IRuntimeXamlLoader>();

            let mut base_uri = None;
            if let Some(assembly_path) = assembly_path {
                let xaml_file_project_path = xaml_file_project_path.unwrap_or("/Fake.xaml");
                //Fabricate fake Uri
                let uri = format!(
                    "{ASSET_SCHEME}://{}{xaml_file_project_path}",
                    get_file_name_without_extension(assembly_path)
                );
                base_uri = Some(Uri::new(&uri, UriKind::Absolute).map_err(|e| {
                    XamlLoadException::with_inner(format!("Invalid URI: the URI '{uri}' of the document: {e}"), e)
                })?);
            }

            // Deviation (DEVIATIONS.md, Designer support): the original
            // loads the assembly from the path. The application is linked
            // into the process that previews it, so the assembly is the
            // registered one of that name, if there is one.
            let local_asm =
                assembly_path.and_then(|assembly_path| MarkupAssembly::find(get_file_name_without_extension(assembly_path)));
            let use_compiled_bindings =
                local_asm.and_then(|local_asm| local_asm.metadata_value(USE_COMPILED_BINDINGS_BY_DEFAULT));

            let mut configuration = RuntimeXamlLoaderConfiguration::new();
            configuration.local_assembly = local_asm;
            configuration.design_mode = true;
            configuration.use_compiled_bindings_by_default = use_compiled_bindings.is_some_and(parse_bool);
            let loaded = loader.load(RuntimeXamlLoaderDocument::with_base_uri(base_uri, xaml), configuration)?;

            let control = Design::create_preview_with_control(&preview_target(&loaded))
                .expect("a preview is a control or a message that says why there is none");
            window = match control.cast::<Window>() {
                Some(window) => window,
                None => {
                    let window = Window::new();
                    window.set_content(Some(Control::boxed(control.clone())));
                    window
                }
            };

            // `window.PlatformImpl is OffscreenTopLevelImplBase offscreenImpl`:
            // the window of the previewer, or another offscreen implementation.
            if let Some(platform_impl) = window.platform_impl() {
                let platform_impl = platform_impl.as_any();
                if let Some(previewer_impl) = platform_impl.downcast_ref::<PreviewerWindowImpl>() {
                    previewer_impl.set_render_scaling(render_scaling);
                } else if let Some(offscreen_impl) = platform_impl.downcast_ref::<OffscreenTopLevelImplBase>() {
                    offscreen_impl.set_render_scaling(render_scaling);
                }
            }

            Design::apply_design_mode_properties(&window, &control);

            if !window.is_set(Window::size_to_content_property()) {
                if window.width().is_nan() {
                    window.set_size_to_content(window.size_to_content() | SizeToContent::WIDTH);
                }

                if window.height().is_nan() {
                    window.set_size_to_content(window.size_to_content() | SizeToContent::HEIGHT);
                }
            }
        }
        window.show();
        Ok(window)
    }
}

/// `Path.GetFileNameWithoutExtension`.
fn get_file_name_without_extension(path: &str) -> &str {
    Path::new(path).file_stem().and_then(|name| name.to_str()).unwrap_or("")
}

/// `bool.TryParse(text, out var value) && value`.
fn parse_bool(text: &str) -> bool {
    text.trim().eq_ignore_ascii_case("true")
}

/// What `Design.CreatePreviewWithControl(object)` tests its argument for:
/// the loaded object as a data template, as an object of the object model
/// (a control, a style, a resource dictionary, the application) or as a
/// style that is not one.
fn preview_target(loaded: &BoxedValue) -> PreviewTarget {
    let value = Some(loaded.clone());
    if let Some(template) = from_markup_value::<Rc<dyn IDataTemplate>>(&value) {
        return PreviewTarget::DataTemplate(template);
    }
    if let Some(object) = from_markup_value::<Ref<FerroObject>>(&value) {
        return PreviewTarget::Object(object);
    }
    if let Some(style) = from_markup_value::<Rc<dyn IStyle>>(&value) {
        return PreviewTarget::Style(style);
    }
    PreviewTarget::Other
}

#[cfg(test)]
mod tests {
    // Not from upstream: the upstream project tests the loader only through
    // the previewer process. The documents are loaded by the run-time
    // loader into windows of the windowing platform of the previewer.
    use super::*;
    use ferroui_base::layout::Layoutable;
    use ferroui_base::StyledElement;
    use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};
    use ferroui_controls::{Border, StackPanel, TextBlock, UserControl};
    use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
    use crate::remote::previewer_windowing_platform::PreviewerWindowingPlatform;
    use crate::remote::test_connection::TestConnection;

    const XMLNS: &str = "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' \
                         xmlns:d='http://schemas.microsoft.com/expression/blend/2008'";

    /// A unit test application with the run-time loader and the windowing
    /// platform of the previewer: in the designer mode a window is the
    /// embeddable window of the platform, which the mock platform of the
    /// test services does not have.
    fn start() -> UnitTestApplicationScope {
        ferroui_markup_xaml::register_types();
        let scope = UnitTestApplication::start(TestServices::styled_window());
        FerroRuntimeXamlLoader::register();
        PreviewerWindowingPlatform::reset_for_unit_tests();
        PreviewerWindowingPlatform::initialize(TestConnection::new());
        scope
    }

    fn load(xaml: &str) -> Ref<Window> {
        DesignWindowLoader::load_designer_window(xaml, None, None).expect("the document loads")
    }

    fn content_of(window: &Window) -> Ref<Control> {
        Control::from_boxed(&window.content().expect("the window has content")).expect("the content is a control")
    }

    #[test]
    fn a_document_whose_root_is_a_window_is_the_window() {
        let app = start();
        let window = load(&format!("<Window {XMLNS} Title='Previewed' Width='300' Height='200'><Border Name='inner'/></Window>"));
        assert_eq!(Some("Previewed".to_string()), window.title());
        assert!(content_of(&window).is::<Border>());
        assert!(window.is_visible());
        // A window with a size does not size itself to its content.
        assert_eq!(SizeToContent::MANUAL, window.size_to_content());
        window.close();
        app.dispose();
    }

    #[test]
    fn a_window_without_a_size_sizes_itself_to_its_content() {
        let app = start();
        let window = load(&format!("<Window {XMLNS}><Border Width='40' Height='30'/></Window>"));
        assert_eq!(SizeToContent::WIDTH_AND_HEIGHT, window.size_to_content());
        window.close();

        let window = load(&format!("<Window {XMLNS} Width='300'><Border Width='40' Height='30'/></Window>"));
        assert_eq!(SizeToContent::HEIGHT, window.size_to_content());
        window.close();

        // A window that states how it sizes itself is left as it is.
        let window = load(&format!("<Window {XMLNS} SizeToContent='Manual'><Border/></Window>"));
        assert_eq!(SizeToContent::MANUAL, window.size_to_content());
        window.close();
        app.dispose();
    }

    #[test]
    fn a_document_whose_root_is_a_control_is_the_content_of_a_window() {
        let app = start();
        let window = load(&format!("<UserControl {XMLNS}><TextBlock Text='Hello'/></UserControl>"));
        let content = content_of(&window);
        let user_control = content.cast::<UserControl>().expect("the previewed control");
        let text = Control::from_boxed(&user_control.content().unwrap()).unwrap().cast::<TextBlock>().unwrap();
        assert_eq!(Some("Hello".to_string()), text.text());
        assert_eq!(SizeToContent::WIDTH_AND_HEIGHT, window.size_to_content());
        assert!(window.is_visible());
        window.close();
        app.dispose();
    }

    #[test]
    fn the_design_size_and_the_design_data_context_of_the_document_are_those_of_the_window() {
        let app = start();
        let window = load(&format!(
            "<UserControl {XMLNS} d:DesignWidth='320' d:DesignHeight='240'><Border/></UserControl>"
        ));
        assert_eq!(320.0, window.width());
        assert_eq!(240.0, window.height());
        // With both sizes the window does not size itself to its content.
        assert_eq!(SizeToContent::MANUAL, window.size_to_content());
        window.close();

        let window = load(&format!(
            "<UserControl {XMLNS} Design.Width='200' Design.DataContext='design data'><Border/></UserControl>"
        ));
        assert_eq!(200.0, window.width());
        assert!(window.height().is_nan());
        assert_eq!(SizeToContent::HEIGHT, window.size_to_content());
        let data_context = window.get_value(StyledElement::data_context_property()).expect("the design data context");
        assert_eq!(Some(&"design data".to_string()), data_context.downcast_ref::<String>());
        // The size of the document itself is not the design size.
        assert!(content_of(&window).get_value(Layoutable::width_property()).is_nan());
        window.close();
        app.dispose();
    }

    #[test]
    fn a_control_that_names_a_preview_host_is_previewed_as_itself() {
        let app = start();
        // `Design.SetPreviewWith(target, control)` records nothing for a
        // target that is a visual ("not a supported scenario without
        // templates"), so the control is previewed as any other control.
        let window = load(&format!(
            "<UserControl {XMLNS}><Design.PreviewWith><Border Name='host' Padding='20'/></Design.PreviewWith></UserControl>"
        ));
        let content = content_of(&window);
        assert_eq!(None, content.name());
        assert!(content.is::<UserControl>());
        window.close();
        app.dispose();
    }

    #[test]
    fn a_style_is_previewed_in_the_control_it_names() {
        let app = start();
        let window = load(&format!(
            "<Style {XMLNS} Selector='Border'><Design.PreviewWith><Border Name='host'/></Design.PreviewWith>\
             <Setter Property='Width' Value='50'/></Style>"
        ));
        let host = content_of(&window);
        assert_eq!(Some("host".to_string()), host.name());
        assert_eq!(1, host.styles().count());
        window.close();

        // Without a host the preview says how to give one.
        let window = load(&format!("<Style {XMLNS} Selector='Border'><Setter Property='Width' Value='50'/></Style>"));
        let message = content_of(&window).cast::<StackPanel>().expect("the lines of the message");
        let first = message.children().get(0).cast::<TextBlock>().unwrap();
        assert_eq!(Some("Styles can't be previewed without Design.PreviewWith. Add".to_string()), first.text());
        window.close();
        app.dispose();
    }

    #[test]
    fn a_resource_dictionary_is_previewed_in_the_control_it_names() {
        let app = start();
        let window = load(&format!(
            "<ResourceDictionary {XMLNS}><Design.PreviewWith><Border Name='host'/></Design.PreviewWith>\
             <SolidColorBrush x:Key='accent' Color='Red'/></ResourceDictionary>"
        ));
        let host = content_of(&window);
        assert_eq!(Some("host".to_string()), host.name());
        assert_eq!(1, host.resources().merged_dictionaries().count());
        window.close();

        let window = load(&format!("<ResourceDictionary {XMLNS}><SolidColorBrush x:Key='accent' Color='Red'/></ResourceDictionary>"));
        let message = content_of(&window).cast::<StackPanel>().expect("the lines of the message");
        let first = message.children().get(0).cast::<TextBlock>().unwrap();
        assert_eq!(
            Some("ResourceDictionaries can't be previewed without Design.PreviewWith. Add".to_string()),
            first.text()
        );
        window.close();
        app.dispose();
    }

    #[test]
    fn a_document_that_does_not_load_is_an_error() {
        let app = start();
        let error = DesignWindowLoader::load_designer_window(&format!("<Window {XMLNS}><NoSuchControl/></Window>"), None, None)
            .expect_err("the type of the element is unknown");
        assert!(!error.message().is_empty());
        assert!(DesignWindowLoader::load_designer_window("<Window", None, None).is_err());
        app.dispose();
    }

    #[test]
    fn the_name_of_the_assembly_and_the_path_of_the_document_make_the_base_uri() {
        assert_eq!("MyApp", get_file_name_without_extension("/build/bin/MyApp.dll"));
        assert_eq!("my-app", get_file_name_without_extension("target/debug/my-app"));
        assert_eq!("", get_file_name_without_extension(""));
        assert!(parse_bool(" True "));
        assert!(!parse_bool("yes"));
    }
}
