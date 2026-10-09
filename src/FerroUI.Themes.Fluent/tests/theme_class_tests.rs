//! Port of `tests/Themes.UnitTests/ThemeClassTests.cs` (the Fluent theme
//! instantiation).

use super::support::*;
use crate::{FluentTheme, ASSEMBLY};
use ferroui_base::data::core::ValueType;
use ferroui_base::metadata::{from_markup_value, IServiceProvider};
use ferroui_base::utilities::Uri;
use ferroui_base::{Ref, TypeInfo};
use ferroui_markup_xaml::FerroXamlLoader;
use std::rc::Rc;

#[test]
fn should_have_service_provider_ctor() {
    let _app = start_application();
    let markup = FluentTheme::TYPE.markup().expect("the class has markup metadata");
    let service_provider = ValueType::of::<Option<Rc<dyn IServiceProvider>>>();
    assert!(markup
        .constructors
        .iter()
        .any(|constructor| constructor.parameters.len() == 1 && (constructor.parameters[0])() == service_provider));
}

#[test]
fn should_be_reachable_with_ctor() {
    let _app = start_application();
    let theme = FluentTheme::with_service_provider(None);
    assert!(std::ptr::eq(FluentTheme::TYPE, theme.get_type()));
    // The parameterless form the type table knows.
    let created = TypeInfo::find("FerroUI.Themes.Fluent", "FluentTheme").and_then(|type_| type_.create_instance());
    assert!(created.is_some_and(|created| created.is::<FluentTheme>()));
}

#[test]
fn should_be_reachable_with_ferres_loader() {
    let _app = start_application();
    let uri = Uri::absolute(&format!("ferres://{}/FluentTheme.xaml", ASSEMBLY.name)).expect("Invalid TryLoad URL");
    let theme = FerroXamlLoader::load(&uri, None).unwrap_or_else(|error| panic!("{}", describe(&error)));
    assert!(from_markup_value::<Ref<FluentTheme>>(&Some(theme)).is_some());
}
