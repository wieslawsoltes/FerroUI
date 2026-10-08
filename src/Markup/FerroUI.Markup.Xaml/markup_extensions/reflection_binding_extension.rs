//! Port of `MarkupExtensions/ReflectionBindingExtension.cs`.

use crate::ServiceProviderExtensions;
use ferroui_base::data::{BindingBase, BindingExpressionBase, ReflectionBinding};
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::{ferro_markup_type, FerroObject, FerroProperty, Ref};
use std::ops::Deref;
use std::rc::Rc;

/// `{ReflectionBinding Path}`: a binding whose path is resolved at run
/// time, as markup declares it. Providing its value creates the binding
/// with the context of the place it is declared in: the type resolver, the
/// default anchor and the name scope.
///
/// The managed class derives from the reflection binding class; here it
/// holds one ([`base`](Self::base)) and dereferences to it, so the
/// inherited properties are read and set on the extension itself.
pub struct ReflectionBindingExtension {
    base: Rc<ReflectionBinding>,
}

impl ReflectionBindingExtension {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { base: ReflectionBinding::empty() })
    }

    /// Creates the extension with a path.
    pub fn with_path(path: &str) -> Rc<Self> {
        Rc::new(Self { base: ReflectionBinding::new(path) })
    }

    /// The reflection binding this extension is (the base class part).
    pub fn base(&self) -> &Rc<ReflectionBinding> {
        &self.base
    }

    /// Creates the binding the extension describes.
    pub fn provide_value(&self, service_provider: &Rc<dyn IServiceProvider>) -> Rc<ReflectionBinding> {
        let resolver_provider = service_provider.clone();
        let binding = ReflectionBinding::empty();
        binding
            .set_type_resolver(Some(Rc::new(move |namespace, name| resolver_provider.resolve_type(namespace, name).ok())));
        binding.set_converter(self.converter());
        binding.set_converter_culture(self.converter_culture());
        binding.set_converter_parameter(self.converter_parameter());
        binding.set_element_name(self.element_name());
        binding.set_fallback_value(self.fallback_value());
        binding.set_mode(self.mode());
        binding.set_path(self.path());
        binding.set_priority(self.priority());
        binding.set_delay(self.delay());
        binding.set_source(self.source());
        binding.set_string_format(self.string_format());
        binding.set_relative_source(self.relative_source());
        binding.set_default_anchor(service_provider.get_default_anchor().map(|anchor| anchor.downgrade()));
        binding.set_target_null_value(self.target_null_value());
        binding.set_name_scope(service_provider.get_name_scope().map(|scope| Rc::downgrade(&scope)));
        binding.set_update_source_trigger(self.update_source_trigger());
        binding
    }
}

impl Deref for ReflectionBindingExtension {
    type Target = ReflectionBinding;

    fn deref(&self) -> &ReflectionBinding {
        &self.base
    }
}

impl BindingBase for ReflectionBindingExtension {
    fn create_instance(
        &self,
        target: &FerroObject,
        target_property: Option<&'static FerroProperty>,
        anchor: Option<&Ref<FerroObject>>,
    ) -> Rc<dyn BindingExpressionBase> {
        self.base.create_instance(target, target_property, anchor)
    }

    /// The extension itself; the binding class it derives from is its `base()`.
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

crate::identity_eq!(ReflectionBindingExtension);

type This = Rc<ReflectionBindingExtension>;

// The inherited properties are the ones the reflection binding class
// declares: they are reached through `base`.
ferro_markup_type!(class ReflectionBindingExtension {
    this: Rc<ReflectionBindingExtension>,
    handles: [Rc<ReflectionBindingExtension>, Option<Rc<ReflectionBindingExtension>>],
    base: Rc<ReflectionBinding>,
    constructors: [
        () => ReflectionBindingExtension::new,
        (String) => |path: String| ReflectionBindingExtension::with_path(&path),
    ],
    methods: [
        fn ProvideValue(Rc<dyn IServiceProvider>) -> Rc<ReflectionBinding> =>
            |this: &This, service_provider: Rc<dyn IServiceProvider>| this.provide_value(&service_provider),
    ],
});
