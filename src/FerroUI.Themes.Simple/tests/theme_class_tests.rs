//! Port of `tests/Themes.UnitTests/ThemeClassTests.cs` (the Simple theme
//! instantiation).

use super::support::*;
use crate::{SimpleTheme, ASSEMBLY};
use ferroui_base::data::core::ValueType;
use ferroui_base::metadata::{from_markup_value, IServiceProvider};
use ferroui_base::utilities::Uri;
use ferroui_base::{Ref, TypeInfo};
use ferroui_markup_xaml::FerroXamlLoader;
use std::rc::Rc;

#[test]
fn should_have_service_provider_ctor() {
    let _app = start_application();
    let markup = SimpleTheme::TYPE.markup().expect("the class has markup metadata");
    let service_provider = ValueType::of::<Option<Rc<dyn IServiceProvider>>>();
    assert!(markup
        .constructors
        .iter()
        .any(|constructor| constructor.parameters.len() == 1 && (constructor.parameters[0])() == service_provider));
}

#[test]
fn should_be_reachable_with_ctor() {
    let _app = start_application();
    let theme = SimpleTheme::with_service_provider(None);
    assert!(std::ptr::eq(SimpleTheme::TYPE, theme.get_type()));
    // The parameterless form the type table knows.
    let created = TypeInfo::find("FerroUI.Themes.Simple", "SimpleTheme").and_then(|type_| type_.create_instance());
    assert!(created.is_some_and(|created| created.is::<SimpleTheme>()));
}

#[test]
fn should_be_reachable_with_asset_loader() {
    let _app = start_application();
    let uri = Uri::absolute(&format!("ferres://{}/SimpleTheme.xaml", ASSEMBLY.name)).expect("Invalid TryLoad URL");
    let theme = FerroXamlLoader::load(&uri, None).unwrap_or_else(|error| panic!("{}", describe(&error)));
    assert!(from_markup_value::<Ref<SimpleTheme>>(&Some(theme)).is_some());
}
