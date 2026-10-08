//! Not from upstream: the source information of compiled documents
//! (`CreateSourceInfo`, `compiled_xaml_source_info.rs`) is the source
//! information the run-time loader attaches to the same documents: of the root,
//! of the objects it creates and of the resources of a dictionary.

use std::rc::Rc;

use ferroui_base::controls::{IResourceDictionary, ResourceDictionary};
use ferroui_base::data::core::ValueTypes;
use ferroui_base::utilities::Uri;
use ferroui_base::{BoxedValue, Ref};
use ferroui_controls::testing::{TestServices, UnitTestApplication};
use ferroui_controls::UserControl;
use ferroui_markup_xaml::diagnostics::XamlSourceInfo;
use ferroui_markup_xaml::xaml_il::runtime::XamlIlRuntimeHelpers;
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

use crate::documents::SOURCE_INFO_DOCUMENTS;
use crate::{compiled_xaml_source_info, ASSEMBLY};

/// The source information of the root, of its resource dictionary and of each resource.
fn source_infos(root: &Ref<UserControl>) -> Vec<Option<XamlSourceInfo>> {
    let resources: Ref<ResourceDictionary> = root.resources();
    let dictionary: Rc<dyn IResourceDictionary> = resources.clone().into();
    vec![
        XamlSourceInfo::get_xaml_source_info(&(Rc::new(root.clone()) as BoxedValue)),
        XamlSourceInfo::get_xaml_source_info(&(Rc::new(resources) as BoxedValue)),
        XamlSourceInfo::get_xaml_source_info_for_key(&dictionary, "brush1"),
        XamlSourceInfo::get_xaml_source_info_for_key(&dictionary, "brush2"),
    ]
}

#[test]
fn compiled_source_information_is_the_run_time_loaders() {
    let _app = UnitTestApplication::start(TestServices::styled_window());
    crate::register_types();
    FerroRuntimeXamlLoader::register();

    let compiled = compiled_xaml_source_info::build_mergeresourceinclude_works_with_single_resource_xaml(Some(
        XamlIlRuntimeHelpers::create_root_service_provider_v3(None),
    ))
    .expect("the compiled document loads");

    let documents = SOURCE_INFO_DOCUMENTS
        .iter()
        .map(|(name, xaml)| {
            let uri = Uri::absolute(&format!("ferres://{}/{name}", ASSEMBLY.name)).expect("a valid URI");
            let mut document = RuntimeXamlLoaderDocument::with_base_uri(Some(uri), xaml);
            document.document = Some(name.to_string());
            document
        })
        .collect();
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&ASSEMBLY);
    configuration.set_create_source_info(true);
    let loaded = FerroRuntimeXamlLoader::load_group(documents, Some(configuration)).expect("the documents load");
    let loaded = loaded[1]
        .as_ref()
        .and_then(|value| ValueTypes::as_object(&**value))
        .and_then(|object| object.cast::<UserControl>())
        .expect("a user control");

    let expected = source_infos(&loaded);
    assert!(expected.iter().all(Option::is_some), "{expected:?}");
    assert_eq!(source_infos(&compiled), expected);
}
