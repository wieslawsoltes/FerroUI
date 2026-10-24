//! Not from the reference: the styles document of the crate (`Styles.xaml`), compiled by the build
//! of the crate, is what an include of its URI loads.

use crate::tests::support::start_app;
use crate::ASSEMBLY;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::styling::Styles;
use ferroui_base::Ref;
use ferroui_markup_xaml::{RuntimeXamlLoaderConfiguration, RuntimeXamlLoaderDocument};
use ferroui_markup_xaml_loader::FerroRuntimeXamlLoader;

#[test]
fn the_styles_of_the_crate_load_by_their_uri() {
    let _app = start_app();
    FerroRuntimeXamlLoader::register();

    let xaml = "<Styles xmlns=\"https://github.com/ferroui\">\
                  <StyleInclude Source=\"ferres://FerroUI.Xaml.Interactions.DragAndDrop/Styles.xaml\" />\
                </Styles>";
    let mut configuration = RuntimeXamlLoaderConfiguration::new();
    configuration.local_assembly = Some(&ASSEMBLY);
    let loaded = FerroRuntimeXamlLoader::load_document(RuntimeXamlLoaderDocument::new(xaml), Some(configuration))
        .unwrap_or_else(|error| match error.inner_exception() {
            Some(inner) => panic!("Styles.xaml failed to load: {}\n ---> {inner}", error.message()),
            None => panic!("Styles.xaml failed to load: {}", error.message()),
        });

    let styles = from_markup_value::<Ref<Styles>>(&Some(loaded)).expect("a Styles");
    assert_eq!(1, styles.count());
}
