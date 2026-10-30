//! The designer's visual language is separate from the document's application theme.
//!
//! Hosts can reuse these styles on their own chrome by adding the `designer-chrome`
//! class to a container. The styles never target the preview subtree and all resource
//! keys are prefixed with `Designer`, so an application's controls retain their theme.
use ferroui_base::styling::IStyle;
use ferroui_markup_xaml::FromXamlObject;
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;
use std::rc::Rc;

/// Give an editor-owned menu host local native theme aliases. Installing these on
/// the button (for flyouts) or ContextMenu also covers native submenu popups while
/// keeping the application's preview resources and control themes independent.
pub(crate) fn apply_designer_menu_resources(
    host: &ferroui_controls::Control,
    palette: &ferroui_controls::Control,
) {
    use ferroui_base::controls::ResourceDictionary;
    use ferroui_base::metadata::from_markup_value;
    use ferroui_base::Ref;

    let Some(value) = palette.try_find_resource(&"DesignerMenuResources".into(), None) else {
        return;
    };
    let Some(resources) = from_markup_value::<Ref<ResourceDictionary>>(&value) else {
        return;
    };
    for key in resources.keys() {
        host.resources().set(key.clone(), resources.get(&key));
    }
}

/// Load the reusable, scoped designer chrome styles and semantic color resources.
///
/// Add the returned style to the host's styles collection and mark only its tool
/// containers with `designer-chrome`. This supplements the host's control theme.
pub fn designer_chrome_style() -> Result<Rc<dyn IStyle>, String> {
    FerroRuntimeXamlLoader::register();
    ferroui_markup_xaml::register_types();
    ferroui_controls_treedatagrid::register_types();
    let loaded =
        FerroRuntimeXamlLoader::load(include_str!("DesignerChrome.xaml"), None, None, None, false)
            .map_err(|error| error.to_string())?;
    Rc::<dyn IStyle>::from_xaml_object(&loaded)
        .ok_or_else(|| "Designer chrome did not load a style collection".to_owned())
}
