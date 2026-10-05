//! Port of `CompilerExtensions/XamlIlPropertyInfoAccessorFactoryEmitter.cs`, reduced to its
//! data contract.
//!
//! Upstream the emitter loads, for a path element that reads a property through a property
//! info, the delegate `Func<WeakReference<object>, IPropertyInfo, IPropertyAccessor>` that
//! creates the accessor observing that property. There is no IL back end here: the three
//! `emit_load_*` methods return which factory a back end has to supply.

use std::rc::Rc;

use xamlx::ast::IXamlAstValueNode;

/// The accessor factory a compiled binding path element passes to
/// `CompiledBindingPathBuilder.Property` next to its property info: a
/// `Func<WeakReference<object>, IPropertyInfo, IPropertyAccessor>`.
#[derive(Clone)]
pub enum XamlIlPropertyAccessorFactory {
    /// `PropertyInfoAccessorFactory.CreateInpcPropertyAccessor` (static): an accessor that
    /// re-reads the property when its owner raises `INotifyPropertyChanged.PropertyChanged`
    /// for it.
    ///
    /// Upstream IL: `ldnull; ldftn CreateInpcPropertyAccessor; newobj Func<..>`.
    ///
    /// Rust runtime: `PropertyInfoAccessorFactory::create_inpc_property_accessor::<Owner>()`
    /// when the owner type raises property change notifications,
    /// `PropertyInfoAccessorFactory::create_plain_property_accessor()` otherwise.
    Inpc,
    /// `PropertyInfoAccessorFactory.CreateFerroPropertyAccessor` (static): an accessor that
    /// observes a registered property of a `FerroObject`.
    ///
    /// Upstream IL: `ldnull; ldftn CreateFerroPropertyAccessor; newobj Func<..>`.
    ///
    /// Rust runtime: none needed, `CompiledBindingPathBuilder::ferro_property` /
    /// `ferro_property_with` take the registered property alone.
    FerroProperty,
    /// An accessor for an indexer with a single `int` argument on a collection that raises
    /// `INotifyCollectionChanged`: besides property changes it re-reads the value when a
    /// collection change affects the index.
    ///
    /// Upstream IL: evaluate `index` as an `int`, create an instance of the generated closure
    /// class with it (`newobj Closure(int)`, the class stores the index in the field `_index`),
    /// then `ldftn Closure.CreateAccessor; newobj Func<..>`. The instance method
    /// `CreateAccessor(WeakReference<object> target, IPropertyInfo property)` returns
    /// `PropertyInfoAccessorFactory.CreateIndexerPropertyAccessor(target, property, _index)`.
    ///
    /// Rust runtime: `PropertyInfoAccessorFactory::create_indexer_property_accessor(index)`.
    Indexer {
        /// The constant index, a value node of type `System.Int32`.
        index: Rc<dyn IXamlAstValueNode>,
    },
}

impl XamlIlPropertyAccessorFactory {
    /// The method of `PropertyInfoAccessorFactory` the factory delegate ends up calling.
    pub fn factory_method_name(&self) -> &'static str {
        match self {
            XamlIlPropertyAccessorFactory::Inpc => {
                XamlIlPropertyInfoAccessorFactoryEmitter::CREATE_INPC_PROPERTY_ACCESSOR
            }
            XamlIlPropertyAccessorFactory::FerroProperty => {
                XamlIlPropertyInfoAccessorFactoryEmitter::CREATE_FERRO_PROPERTY_ACCESSOR
            }
            XamlIlPropertyAccessorFactory::Indexer { .. } => {
                XamlIlPropertyInfoAccessorFactoryEmitter::CREATE_INDEXER_PROPERTY_ACCESSOR
            }
        }
    }
}

/// `XamlIlPropertyInfoAccessorFactoryEmitter`. Upstream it is a configuration extra created
/// with the type builder of the indexer closure class; here it has no state:
/// `configuration.get_or_create_extra::<XamlIlPropertyInfoAccessorFactoryEmitter>()`.
#[derive(Default)]
pub struct XamlIlPropertyInfoAccessorFactoryEmitter;

impl XamlIlPropertyInfoAccessorFactoryEmitter {
    /// `IndexerClosureFactoryMethodName`: the instance method of the generated indexer closure
    /// class.
    pub const INDEXER_CLOSURE_FACTORY_METHOD_NAME: &'static str = "CreateAccessor";
    /// `IPropertyAccessor CreateInpcPropertyAccessor(WeakReference<object>, IPropertyInfo)`.
    pub const CREATE_INPC_PROPERTY_ACCESSOR: &'static str = "CreateInpcPropertyAccessor";
    /// `IPropertyAccessor CreateFerroPropertyAccessor(WeakReference<object>, IPropertyInfo)`.
    pub const CREATE_FERRO_PROPERTY_ACCESSOR: &'static str = "CreateFerroPropertyAccessor";
    /// `IPropertyAccessor CreateIndexerPropertyAccessor(WeakReference<object>, IPropertyInfo, int)`.
    pub const CREATE_INDEXER_PROPERTY_ACCESSOR: &'static str = "CreateIndexerPropertyAccessor";

    pub fn new() -> Self {
        Self
    }

    /// `EmitLoadInpcPropertyAccessorFactory(context, codeGen)`. Stack effect upstream: pushes
    /// the factory delegate.
    pub fn emit_load_inpc_property_accessor_factory(&self) -> XamlIlPropertyAccessorFactory {
        XamlIlPropertyAccessorFactory::Inpc
    }

    /// `EmitLoadFerroPropertyAccessorFactory(context, codeGen)`. Stack effect upstream: pushes
    /// the factory delegate.
    pub fn emit_load_ferro_property_accessor_factory(&self) -> XamlIlPropertyAccessorFactory {
        XamlIlPropertyAccessorFactory::FerroProperty
    }

    /// `EmitLoadIndexerAccessorFactory(context, codeGen, value)`. Stack effect upstream:
    /// evaluates `value` and pushes the factory delegate bound to it.
    pub fn emit_load_indexer_accessor_factory(
        &self,
        value: &Rc<dyn IXamlAstValueNode>,
    ) -> XamlIlPropertyAccessorFactory {
        XamlIlPropertyAccessorFactory::Indexer {
            index: value.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factories_name_the_runtime_factory_methods() {
        let emitter = XamlIlPropertyInfoAccessorFactoryEmitter::new();
        assert_eq!(
            emitter
                .emit_load_inpc_property_accessor_factory()
                .factory_method_name(),
            "CreateInpcPropertyAccessor"
        );
        assert_eq!(
            emitter
                .emit_load_ferro_property_accessor_factory()
                .factory_method_name(),
            "CreateFerroPropertyAccessor"
        );
        let index: Rc<dyn IXamlAstValueNode> = xamlx::ast::XamlAstTextNode::new(
            &xamlx::ast::XamlLineInfo::new(1, 1),
            "3",
            false,
        );
        let factory = emitter.emit_load_indexer_accessor_factory(&index);
        assert_eq!(factory.factory_method_name(), "CreateIndexerPropertyAccessor");
        match factory {
            XamlIlPropertyAccessorFactory::Indexer { index: stored } => {
                assert!(Rc::ptr_eq(&stored, &index));
            }
            _ => panic!("expected the indexer factory"),
        }
        assert_eq!(
            XamlIlPropertyInfoAccessorFactoryEmitter::INDEXER_CLOSURE_FACTORY_METHOD_NAME,
            "CreateAccessor"
        );
    }
}
