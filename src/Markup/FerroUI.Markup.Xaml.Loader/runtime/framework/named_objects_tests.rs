//! Regression: x:Name can register a native FerroObject without a Name property.
use ferroui_base::{
    controls::NameScope, ferro_class, ferro_class_info, ferro_impl_classes, ferro_properties,
    instantiate, media::ScaleTransform, FerroObject, FerroObjectImpl, FerroProperty, Ref,
    StyledElement, StyledElementImpl, StyledProperty,
};
#[repr(C)]
struct NativeNameScopeRoot {
    base: StyledElement,
}
ferro_class!(NativeNameScopeRoot:StyledElement);
ferro_class_info!(NativeNameScopeRoot {new:NativeNameScopeRoot::new,markup:{namespace:"FerroUI.NamedObjects.Tests",content:Content}});
ferro_impl_classes!(NativeNameScopeRoot:FerroObjectImpl,StyledElementImpl);
ferro_properties! {impl NativeNameScopeRoot {
    fn content_property()->StyledProperty<Option<Ref<FerroObject>>> {FerroProperty::register::<Self,_>("Content",None)}
}}
impl NativeNameScopeRoot {
    fn new() -> Ref<Self> {
        instantiate(Self {
            base: StyledElement::construct(),
        })
    }
    fn content(&self) -> Option<Ref<FerroObject>> {
        self.get_value(Self::content_property())
    }
    fn set_content(&self, value: Option<Ref<FerroObject>>) {
        self.set_value(Self::content_property(), value)
    }
}
fn load(xml: &str) -> Result<Ref<NativeNameScopeRoot>, ferroui_markup_xaml::XamlLoadException> {
    ferroui_markup_xaml::register_types();
    ferroui_base::TypeInfo::register_namespaces(&[(
        NativeNameScopeRoot::TYPE.module_path(),
        "FerroUI.NamedObjects.Tests",
    )]);
    ferroui_base::TypeInfo::register_all(&[NativeNameScopeRoot::TYPE]);
    crate::runtime::interpreter::tests::classes::register();
    let value = crate::FerroRuntimeXamlLoader::parse(xml, None)?;
    Ok(ferroui_base::data::core::ValueTypes::as_object(&*value)
        .unwrap()
        .cast::<NativeNameScopeRoot>()
        .unwrap())
}
const XML:&str="<local:NativeNameScopeRoot xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml' xmlns:local='using:FerroUI.NamedObjects.Tests' x:Name='root'><ScaleTransform x:Name='transform' ScaleX='1.25' ScaleY='2.5'/></local:NativeNameScopeRoot>";
#[test]
fn nonstyled_runtime_name_scope_keeps_actual_native_identity_and_ordinary_root_name() {
    let _scope = ferroui_base::FerroLocator::enter_scope();
    let root = load(XML).unwrap();
    assert_eq!(root.name().as_deref(), Some("root"));
    let scope = NameScope::get_name_scope(&root).unwrap();
    let transform = scope
        .find("transform")
        .unwrap()
        .cast::<ScaleTransform>()
        .unwrap();
    assert!(root.content().unwrap().ptr_eq(&transform));
    assert_eq!(transform.scale_x(), 1.25);
    assert_eq!(transform.scale_y(), 2.5);
    assert!(scope.find("root").unwrap().ptr_eq(&root));
    let other = load(XML).unwrap();
    assert!(!NameScope::get_name_scope(&other)
        .unwrap()
        .find("transform")
        .unwrap()
        .ptr_eq(&transform));
    let weak = root.downgrade();
    drop(root);
    assert!(
        weak.upgrade().is_none(),
        "a scope holds a weak reference to its attached root"
    );
    assert!(scope.find("transform").unwrap().ptr_eq(&transform));
}
#[test]
fn nonstyled_runtime_duplicate_name_remains_a_load_error() {
    let _scope = ferroui_base::FerroLocator::enter_scope();
    let result = load(&XML.replace("x:Name='transform'", "x:Name='root'"));
    assert!(
        result
            .err()
            .unwrap()
            .to_string()
            .contains("already registered"),
        "duplicates must fail at registration, not unrelated type/property resolution"
    );
}
