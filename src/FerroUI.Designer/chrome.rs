//! The designer's visual language is separate from the document's application theme.
//!
//! Hosts can reuse these styles on their own chrome by adding the `designer-chrome`
//! class to a container. The styles never target the preview subtree and all resource
//! keys are prefixed with `Designer`, so an application's controls retain their theme.
use ferroui_base::styling::IStyle;
use ferroui_markup_xaml::FromXamlObject;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::rc::Rc;

/// Load the reusable, scoped designer chrome styles and semantic color resources.
///
/// Add the returned style to the host's styles collection and mark only its tool
/// containers with `designer-chrome`. This supplements the host's control theme.
pub fn designer_chrome_style() -> Result<Rc<dyn IStyle>, String> {
    FerroRuntimeXamlLoader::register();
    ferroui_markup_xaml::register_types();
    let loaded = FerroRuntimeXamlLoader::load(
        include_str!("DesignerChrome.xaml"),
        None,
        None,
        None,
        false,
    )
    .map_err(|error| error.to_string())?;
    Rc::<dyn IStyle>::from_xaml_object(&loaded)
        .ok_or_else(|| "Designer chrome did not load a style collection".to_owned())
}
