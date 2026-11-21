//! Pure selected-framework reproducer: a named control whose template declares
//! a child name scope must not root itself through that scope's parent.
use ferroui_base::{Rect, Ref, Size, data::core::ValueTypes, threading::CancellationToken};
use ferroui_controls::{AppBuilder, Application, Button, Control, StackPanel, Window};
use ferroui_headless::{
    FerroHeadlessPlatformExtensions, FerroHeadlessPlatformOptions, FerroTestApplicationAttribute,
    FerroTestAssembly, HeadlessUnitTestSession,
};
fn build() -> AppBuilder {
    AppBuilder::configure::<Application>().use_headless(FerroHeadlessPlatformOptions::default())
}
const ASSEMBLY: FerroTestAssembly = FerroTestAssembly {
    name: "ChildNameScope.Lifetime.Reproducer",
    test_application: Some(FerroTestApplicationAttribute::new(build)),
    test_isolation: None,
};
#[test]
fn template_child_scope_must_not_keep_named_control_alive_after_document_drop() {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY)).dispatch(||{
 ferroui_markup_xaml::register_types();let app=Application::current().unwrap();let theme=ferroui_themes_simple::SimpleTheme::new().as_style();app.styles().add(theme.clone());
 let weak={
  let xaml=r#"<StackPanel xmlns="https://github.com/ferroui" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">
   <StackPanel.Resources><ControlTheme x:Key="{x:Type Button}" TargetType="Button"><Setter Property="Template"><ControlTemplate><Border><ContentPresenter x:Name="PART_ContentPresenter" Content="{TemplateBinding Content}" /></Border></ControlTemplate></Setter></ControlTheme></StackPanel.Resources>
   <Button x:Name="Named" Content="Named" />
  </StackPanel>"#;
  let value=ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(xaml,None).unwrap();let panel=ValueTypes::as_object(&*value).unwrap().cast::<StackPanel>().unwrap();let button:Ref<Button>=panel.children().get(0).cast().unwrap();let weak=button.downgrade();let w=Window::new();w.set_content(Some(Control::boxed(panel)));w.show();w.measure(Size::new(300.,100.));w.arrange(Rect::new(0.,0.,300.,100.));ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);assert!(button.visual_children().count()>0);w.close();w.set_content(None);weak
 };
 ferroui_base::threading::Dispatcher::ui_thread().run_jobs(None);assert!(weak.upgrade().is_none(),"template child scope retains parent scope, which retains the named template owner");app.styles().remove(theme);
},CancellationToken::none()).wait().unwrap();
}

#[test]
fn successful_inline_resource_include_must_release_its_declaration_context() {
    HeadlessUnitTestSession::get_or_start_for_assembly(Some(&ASSEMBLY)).dispatch(||{
 ferroui_markup_xaml::register_types();ferroui_base::platform::register_assets("includes.lifetime", &[]); ferroui_markup_xaml::FerroXamlLoader::register_compiled_xaml_with_documents("includes.lifetime", resource_dictionary, &["ferres://includes.lifetime/Resources.xaml"]);
 let weak={
  let xaml=r#"<StackPanel xmlns="https://github.com/ferroui"><StackPanel.Resources><ResourceDictionary><ResourceDictionary.MergedDictionaries><ResourceInclude Source="ferres://includes.lifetime/Resources.xaml" /></ResourceDictionary.MergedDictionaries></ResourceDictionary></StackPanel.Resources><Button Content="Native"/></StackPanel>"#;
  let value=ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(xaml,None).unwrap();let panel=ValueTypes::as_object(&*value).unwrap().cast::<StackPanel>().unwrap();panel.downgrade()
 };
 assert!(weak.upgrade().is_none(),"a successfully loaded ResourceInclude retains its markup declaration context/root");
},CancellationToken::none()).wait().unwrap();
}

fn resource_dictionary(
    _: Option<&std::rc::Rc<dyn ferroui_base::metadata::IServiceProvider>>,
    uri: &str,
) -> Result<Option<ferroui_base::BoxedValue>, ferroui_markup_xaml::XamlLoadError> {
    Ok(
        (uri == "ferres://includes.lifetime/Resources.xaml").then(|| {
            std::rc::Rc::new(ferroui_base::controls::ResourceDictionary::new())
                as ferroui_base::BoxedValue
        }),
    )
}
