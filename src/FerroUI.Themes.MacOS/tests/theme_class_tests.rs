//! The rules of `tests/Themes.UnitTests/ThemeClassTests.cs` of the upstream project (as the
//! tests of the Fluent theme), for the theme class of the WinUI theme.

use super::support::*;
use crate::{MacOSTheme, ASSEMBLY};
use ferroui_base::data::core::ValueType;
use ferroui_base::metadata::{from_markup_value, IServiceProvider};
use ferroui_base::utilities::Uri;
use ferroui_base::{Ref, TypeInfo};
use ferroui_markup_xaml::FerroXamlLoader;
use std::rc::Rc;

#[test]
fn should_have_service_provider_ctor() {
    let _app = start_application();
    let markup = MacOSTheme::TYPE.markup().expect("the class has markup metadata");
    let service_provider = ValueType::of::<Option<Rc<dyn IServiceProvider>>>();
    assert!(markup
        .constructors
        .iter()
        .any(|constructor| constructor.parameters.len() == 1 && (constructor.parameters[0])() == service_provider));
}

#[test]
fn should_be_reachable_with_ctor() {
    let _app = start_application();
    let theme = MacOSTheme::with_service_provider(None);
    assert!(std::ptr::eq(MacOSTheme::TYPE, theme.get_type()));
    // The parameterless form the type table knows.
    let created = TypeInfo::find("FerroUI.Themes.MacOS", "MacOSTheme").and_then(|type_| type_.create_instance());
    assert!(created.is_some_and(|created| created.is::<MacOSTheme>()));
}

#[test]
fn should_be_reachable_with_ferres_loader() {
    let _app = start_application();
    let uri = Uri::absolute(&format!("ferres://{}/MacOSTheme.xaml", ASSEMBLY.name)).expect("Invalid TryLoad URL");
    let theme = FerroXamlLoader::load(&uri, None).unwrap_or_else(|error| panic!("{}", describe(&error)));
    assert!(from_markup_value::<Ref<MacOSTheme>>(&Some(theme)).is_some());
}
