//! Port of `Extensions.cs`: what markup extensions and converters ask a
//! service provider for.

use crate::object_casts::{as_object, is_data_context_provider};
use crate::xaml_il::runtime::{IFerroXamlIlControlTemplateProvider, IFerroXamlIlParentStackProvider};
use crate::{EagerParentStackEnumerator, FromXamlObject, IRootObjectProvider, IUriContext, IXamlTypeResolver, XamlLoadException};
use ferroui_base::controls::{INameScope, NameScopeRef};
use ferroui_base::data::core::expression_nodes::CastTarget;
use ferroui_base::metadata::IServiceProvider;
use ferroui_base::styling::IStyle;
use ferroui_base::utilities::Uri;
use ferroui_base::{FerroObject, Ref};
use ferroui_controls::Control;
use std::rc::Rc;

/// The lookups built on the services of a markup service provider.
///
/// (`GetService<T>` of the managed original is
/// [`get_service_of`](ferroui_base::metadata::IServiceProvider) of the
/// service provider contract itself.)
pub trait ServiceProviderExtensions {
    /// The service used through the handle type `T`. Panics if the provider
    /// has no such service: a markup runtime without it is misconfigured.
    fn get_required_service<T: Clone + 'static>(&self) -> T;

    /// The base URI of the document being loaded.
    fn get_context_base_uri(&self) -> Option<Uri>;

    /// The name scope in effect. A provider may publish it as a contract
    /// handle or as the property-value handle; both are asked for.
    fn get_name_scope(&self) -> Option<Rc<dyn INameScope>>;

    /// The nearest parent that is a `T`.
    fn get_first_parent<T: FromXamlObject>(&self) -> Option<T>;

    /// The outermost parent that is a `T`.
    fn get_last_parent<T: FromXamlObject>(&self) -> Option<T>;

    /// The parents that are a `T`, the nearest first.
    fn get_parents<T: FromXamlObject>(&self) -> Vec<T>;

    /// Whether the content of a control template is being built.
    fn is_in_control_template(&self) -> bool;

    /// Resolves a type name with the namespaces of the document.
    fn resolve_type(&self, namespace_prefix: Option<&str>, type_: &str) -> Result<CastTarget, XamlLoadException>;

    /// An object from which bindings whose target is not a control can
    /// locate a data context, named controls and style resources.
    fn get_default_anchor(&self) -> Option<Ref<FerroObject>>;
}

impl ServiceProviderExtensions for dyn IServiceProvider + '_ {
    fn get_required_service<T: Clone + 'static>(&self) -> T {
        match self.get_service_of::<T>() {
            Some(service) => service,
            None => panic!("Service {} hasn't been registered", std::any::type_name::<T>()),
        }
    }

    fn get_context_base_uri(&self) -> Option<Uri> {
        self.get_service_of::<Rc<dyn IUriContext>>()?.base_uri()
    }

    fn get_name_scope(&self) -> Option<Rc<dyn INameScope>> {
        self.get_service_of::<Rc<dyn INameScope>>().or_else(|| self.get_service_of::<NameScopeRef>().map(|scope| scope.0))
    }

    fn get_first_parent<T: FromXamlObject>(&self) -> Option<T> {
        let provider = self.get_service_of::<Rc<dyn IFerroXamlIlParentStackProvider>>()?;
        match provider.clone().as_eager_parent_stack_provider() {
            Some(eager) => EagerParentStackEnumerator::new(Some(eager)).try_get_next_of_type::<T>(),
            None => provider.parents().iter().find_map(T::from_xaml_object),
        }
    }

    fn get_last_parent<T: FromXamlObject>(&self) -> Option<T> {
        let provider = self.get_service_of::<Rc<dyn IFerroXamlIlParentStackProvider>>()?;
        match provider.clone().as_eager_parent_stack_provider() {
            Some(eager) => {
                let mut enumerator = EagerParentStackEnumerator::new(Some(eager));
                let mut last_typed_parent = None;

                while let Some(typed_parent) = enumerator.try_get_next_of_type::<T>() {
                    last_typed_parent = Some(typed_parent);
                }

                last_typed_parent
            }
            None => provider.parents().iter().rev().find_map(T::from_xaml_object),
        }
    }

    fn get_parents<T: FromXamlObject>(&self) -> Vec<T> {
        let Some(provider) = self.get_service_of::<Rc<dyn IFerroXamlIlParentStackProvider>>() else {
            return Vec::new();
        };
        match provider.clone().as_eager_parent_stack_provider() {
            Some(eager) => {
                let mut enumerator = EagerParentStackEnumerator::new(Some(eager));
                let mut parents = Vec::new();

                while let Some(typed_parent) = enumerator.try_get_next_of_type::<T>() {
                    parents.push(typed_parent);
                }

                parents
            }
            None => provider.parents().iter().filter_map(T::from_xaml_object).collect(),
        }
    }

    fn is_in_control_template(&self) -> bool {
        self.get_service_of::<Rc<dyn IFerroXamlIlControlTemplateProvider>>().is_some()
    }

    fn resolve_type(&self, namespace_prefix: Option<&str>, type_: &str) -> Result<CastTarget, XamlLoadException> {
        let Some(tr) = self.get_service_of::<Rc<dyn IXamlTypeResolver>>() else {
            return Err(XamlLoadException::with_message("Service IXamlTypeResolver hasn't been registered"));
        };
        match namespace_prefix {
            Some(prefix) if !prefix.is_empty() => tr.resolve(&format!("{prefix}:{type_}")),
            _ => tr.resolve(type_),
        }
    }

    fn get_default_anchor(&self) -> Option<Ref<FerroObject>> {
        // If the target is not a control, so we need to find an anchor that will let us look
        // up named controls and style resources. First look for the closest Control in
        // the context.
        let mut anchor: Option<Ref<FerroObject>> = self.get_first_parent::<Ref<Control>>().map(Ref::upcast);

        if anchor.is_none() {
            // Try to find a data context provider, this was added to allow us to find
            // a datacontext for Application class when using NativeMenuItems.
            anchor = self.get_first_parent::<DataContextProvider>().map(|provider| provider.0);
        }

        // If a control was not found, then try to find the highest-level style as the XAML
        // file could be a XAML file containing only styles.
        anchor
            .or_else(|| {
                let root = self.get_service_of::<Rc<dyn IRootObjectProvider>>()?.root_object()?;
                style_object(&<Rc<dyn IStyle>>::from_xaml_object(&root)?)
            })
            .or_else(|| style_object(&self.get_last_parent::<Rc<dyn IStyle>>()?))
    }
}

/// An object that provides a data context.
struct DataContextProvider(Ref<FerroObject>);

impl FromXamlObject for DataContextProvider {
    fn from_xaml_object(value: &ferroui_base::BoxedValue) -> Option<Self> {
        as_object(value).filter(is_data_context_provider).map(DataContextProvider)
    }
}

/// A style as an anchor object. Anchors are objects of the class model; a
/// style that is not one (an include) cannot anchor a binding.
fn style_object(style: &Rc<dyn IStyle>) -> Option<Ref<FerroObject>> {
    style.as_object().map(FerroObject::to_ref)
}
