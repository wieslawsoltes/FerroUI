use ferroui_base::controls::INameScope;
use ferroui_base::data::converters::IValueConverter;
use ferroui_base::data::core::parsers::TypeResolver;
use ferroui_base::data::{
    BindingBase, BindingExpressionBase, BindingMode, BindingPriority, ReflectionBinding, RelativeSource,
    UpdateSourceTrigger,
};
use ferroui_base::{BoxedValue, FerroObject, FerroProperty, Ref, WeakRef};
use std::ops::Deref;
use std::rc::{Rc, Weak};

/// A binding with a string path, resolved at run time. This is the binding
/// that markup creates when bindings are not compiled.
///
/// The class derives from [`ReflectionBinding`], whose properties it has.
/// Like it, a binding is a mutable reference object shared as `Rc<Binding>`,
/// which converts to `Rc<dyn BindingBase>`; handles compare by identity.
#[derive(Default)]
pub struct Binding {
    base: ReflectionBinding,
}

/// Compares by identity (reference equality), as the reference type this
/// mirrors.
impl PartialEq for Binding {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Binding {
    /// Creates a binding without a path: binds to the source itself.
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    /// Creates a binding with the given path.
    pub fn with_path(path: &str) -> Rc<Self> {
        Rc::new(Self { base: ReflectionBinding::construct(path) })
    }

    /// Creates a binding with the given path and mode.
    pub fn with_path_and_mode(path: &str, mode: BindingMode) -> Rc<Self> {
        Self::with_path(path).with_mode(mode)
    }

    /// The chaining form of `set_converter`, for a converter.
    pub fn with_converter(self: Rc<Self>, converter: Rc<dyn IValueConverter>) -> Rc<Self> {
        self.set_converter(Some(converter));
        self
    }
}

/// The chaining forms of the setters of the base class.
macro_rules! chaining_setters {
    ($($with:ident => $set:ident: $ty:ty,)*) => {
        impl Binding {
            $(
                /// The chaining form of the setter.
                pub fn $with(self: Rc<Self>, value: $ty) -> Rc<Self> {
                    self.$set(value);
                    self
                }
            )*
        }
    };
}

chaining_setters! {
    with_converter_value => set_converter: Option<Rc<dyn IValueConverter>>,
    with_converter_parameter => set_converter_parameter: Option<BoxedValue>,
    with_delay => set_delay: i32,
    with_element_name => set_element_name: Option<String>,
    with_fallback_value => set_fallback_value: Option<BoxedValue>,
    with_mode => set_mode: BindingMode,
    with_priority => set_priority: BindingPriority,
    with_relative_source => set_relative_source: Option<Rc<RelativeSource>>,
    with_source => set_source: Option<BoxedValue>,
    with_string_format => set_string_format: Option<String>,
    with_target_null_value => set_target_null_value: Option<BoxedValue>,
    with_update_source_trigger => set_update_source_trigger: UpdateSourceTrigger,
    with_type_resolver => set_type_resolver: Option<TypeResolver>,
    with_default_anchor => set_default_anchor: Option<WeakRef<FerroObject>>,
    with_name_scope => set_name_scope: Option<Weak<dyn INameScope>>,
}

impl Deref for Binding {
    type Target = ReflectionBinding;

    fn deref(&self) -> &ReflectionBinding {
        &self.base
    }
}

impl BindingBase for Binding {
    fn create_instance(
        &self,
        target: &FerroObject,
        target_property: Option<&'static FerroProperty>,
        anchor: Option<&Ref<FerroObject>>,
    ) -> Rc<dyn BindingExpressionBase> {
        self.base.create_instance(target, target_property, anchor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::StyledElement;

    #[test]
    fn binding_with_empty_path_binds_to_data_context() {
        let target = StyledElement::new();
        let value: BoxedValue = Rc::new("foo".to_string());
        target.set_data_context(Some(value));
        let source = StyledElement::new();
        // Bind one element's data context to the other's through `$parent`-free
        // source binding: the path is empty, the source is explicit.
        let binding = Binding::new();
        binding.set_source(Some(Rc::new(target.clone().upcast::<FerroObject>())));
        binding.set_path("DataContext".to_string());
        source.bind_binding(StyledElement::data_context_property(), &binding);
        let result = source.data_context().expect("a data context");
        assert_eq!(result.downcast_ref::<String>().map(String::as_str), Some("foo"));
    }

    #[test]
    fn malformed_path_is_reported_as_error() {
        let target = StyledElement::new();
        let binding = Binding::with_path("Foo.[");
        assert!(binding.try_create_instance(&target, None, None).is_err());
    }
}
