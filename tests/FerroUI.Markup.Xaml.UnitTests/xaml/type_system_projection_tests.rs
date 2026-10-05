//! What the run-time type system projects from the metadata of the
//! controls library.

use ferroui_markup_xaml_loader::FerroXamlIlRuntimeCompiler;
use xamlx::type_system::{IXamlCustomAttribute, IXamlProperty, XamlValue};

fn property(type_name: &str, property_name: &str) -> std::rc::Rc<dyn IXamlProperty> {
    crate::register_types();
    let system = FerroXamlIlRuntimeCompiler::type_system();
    let type_ = system.as_type_system().find_type(type_name).unwrap_or_else(|| panic!("{type_name}"));
    let found = type_.properties().into_iter().find(|p| p.name() == property_name);
    found.unwrap_or_else(|| panic!("{type_name}.{property_name}"))
}

fn attribute(property: &dyn IXamlProperty, full_name: &str) -> Option<std::rc::Rc<dyn IXamlCustomAttribute>> {
    property.custom_attributes().into_iter().find(|a| a.type_().full_name() == full_name)
}

/// Attributes metadata declares for a registered property that is not an
/// attached one appear on the projected CLR-style property.
#[test]
fn attributes_of_registered_properties_are_projected_on_the_property() {
    let content = property("FerroUI.Controls.ContentControl", "Content");
    let depends_on = attribute(&*content, "FerroUI.Metadata.DependsOnAttribute").expect("[DependsOn] on Content");
    assert!(matches!(depends_on.parameters().first(), Some(XamlValue::String(name)) if name == "ContentTemplate"));
    // The content property is marked too.
    assert!(attribute(&*content, "FerroUI.Metadata.ContentAttribute").is_some());

    let target = property("FerroUI.Controls.Label", "Target");
    assert!(attribute(&*target, "FerroUI.Controls.ResolveByNameAttribute").is_some());
    assert!(attribute(&*target, "FerroUI.Metadata.DependsOnAttribute").is_none());
}
