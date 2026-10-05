//! Port of `CompilerExtensions/XamlIlClrPropertyInfoHelper.cs`, reduced to its data contract.
//!
//! Upstream `XamlIlClrPropertyInfoEmitter` generates, on a per-assembly helper type, one static
//! method per distinct CLR property that lazily creates and caches the property info object
//! (`ClrPropertyInfo` or `ClrPropertyInfo<TSource, TValue>`) compiled binding paths and
//! `IProvideValueTarget.TargetProperty` refer to, together with the getter and setter thunks
//! that object wraps. There is no IL back end here: [`XamlIlClrPropertyInfoEmitter::emit`] and
//! [`XamlIlClrPropertyInfoEmitter::emit_typed`] return a description of the property info a
//! back end has to materialise, and keep the upstream cache so that a property is described
//! (and materialised) once.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use xamlx::ast::IXamlAstValueNode;
use xamlx::exceptions::{XamlError, XamlResult};
use xamlx::transform::TransformerConfiguration;
use xamlx::type_system::{IXamlMethod, IXamlProperty, IXamlType};

use crate::compiler_extensions::transformers::FerroXamlIlWellKnownTypesExtensions;

/// What the generated getter does with the value the property getter returned before it
/// returns it as `object`.
#[derive(Clone)]
pub enum XamlIlClrPropertyGetterResult {
    /// A reference type: returned as is.
    Reference,
    /// `System.Boolean`: upstream returns one of two cached boxed booleans (static fields
    /// `BooleanBox!True` / `BooleanBox!False` of the helper type, created once in its static
    /// constructor) so that reading a boolean property does not allocate.
    CachedBoxedBoolean,
    /// Any other value type: boxed (`box T`).
    Boxed(Rc<dyn IXamlType>),
}

/// The getter thunk of an untyped property info: `object Getter(object target)`.
///
/// # What a back end has to materialise (upstream IL)
///
/// 1. When [`pass_this`](Self::pass_this): load `target` and convert it to the receiver type
///    [`method`](Self::method)`.declaring_type()`: `unbox` (a managed pointer to the value
///    inside the box) when [`this_is_value_type`](Self::this_is_value_type), `castclass`
///    otherwise. A static getter gets no receiver.
/// 2. Evaluate every indexer argument of the owning [`XamlIlClrPropertyInfo`], in order, as a
///    value of the argument node's own type.
/// 3. Call [`method`](Self::method).
/// 4. Convert the result as [`result`](Self::result) says and return it.
#[derive(Clone)]
pub struct XamlIlClrPropertyInfoGetter {
    pub method: Rc<dyn IXamlMethod>,
    /// `!property.Getter.IsStatic`.
    pub pass_this: bool,
    /// `property.Getter.DeclaringType.IsValueType`.
    pub this_is_value_type: bool,
    pub result: XamlIlClrPropertyGetterResult,
}

/// The setter thunk of an untyped property info: `void Setter(object target, object value)`.
///
/// # What a back end has to materialise (upstream IL)
///
/// 1. The receiver and the indexer arguments exactly as for [`XamlIlClrPropertyInfoGetter`]
///    (steps 1 and 2), with the setter method.
/// 2. Load `value` and convert it to [`value_type`](Self::value_type), the setter parameter
///    that follows the indexer arguments: `unbox.any` when
///    [`value_is_value_type`](Self::value_is_value_type), `castclass` otherwise.
/// 3. Call [`method`](Self::method), discarding a result if it has one.
#[derive(Clone)]
pub struct XamlIlClrPropertyInfoSetter {
    pub method: Rc<dyn IXamlMethod>,
    /// `!property.Setter.IsStatic`.
    pub pass_this: bool,
    /// `property.Setter.DeclaringType.IsValueType`.
    pub this_is_value_type: bool,
    /// `property.Setter.Parameters[indexerArguments.Count]`.
    pub value_type: Rc<dyn IXamlType>,
    pub value_is_value_type: bool,
}

/// "The property info for CLR property `property`" (optionally an indexer property applied to
/// constant arguments): what `XamlIlClrPropertyInfoEmitter.Emit` leaves on the stack, an
/// `IPropertyInfo`.
///
/// # What a back end has to materialise (upstream IL)
///
/// One object per distinct description (the emitter hands out the same `Rc` for the same
/// property, see [`XamlIlClrPropertyInfoEmitter::emit`]), created on first use and cached
/// (upstream: static field `<name>!Field` read through the static method `<name>!Property`):
///
/// `new ClrPropertyInfo(property.Name, getter, setter, typeof(property.PropertyType))`
///
/// where `getter` is a `Func<object, object>` over [`XamlIlClrPropertyInfoGetter`] (null when
/// the property has no getter) and `setter` an `Action<object, object>` over
/// [`XamlIlClrPropertyInfoSetter`] (null when it has no setter).
///
/// Rust runtime: `ferroui_base::data::core::ClrPropertyInfo` (`ClrPropertyInfo::new`,
/// `read_only`, `read_write`, and the `_fallible` variants for indexers whose access can fail).
pub struct XamlIlClrPropertyInfo {
    /// The base name of the generated members: the cache key, or the key followed by `_` and a
    /// generated identifier part when another property with the same key was described before.
    /// Upstream names the members `<name>!Field`, `<name>!Getter`, `<name>!Setter` and
    /// `<name>!Property`.
    pub name: String,
    pub property: Rc<dyn IXamlProperty>,
    /// The constant indexer arguments (empty for a plain property).
    pub indexer_arguments: Vec<Rc<dyn IXamlAstValueNode>>,
    pub getter: Option<XamlIlClrPropertyInfoGetter>,
    pub setter: Option<XamlIlClrPropertyInfoSetter>,
}

impl XamlIlClrPropertyInfo {
    /// The first constructor argument: `property.Name`.
    pub fn property_name(&self) -> String {
        self.property.name()
    }

    /// The last constructor argument: `property.PropertyType`.
    pub fn property_type(&self) -> Rc<dyn IXamlType> {
        self.property.property_type()
    }
}

/// "The typed property info for CLR property `property`": what
/// `XamlIlClrPropertyInfoEmitter.EmitTyped` leaves on the stack, an
/// `IPropertyInfo<TSource, TValue>`.
///
/// # What a back end has to materialise (upstream IL)
///
/// One object per distinct description, created on first use and cached (upstream: static
/// field `<name>!Field` read through the static method `<name>!Property`):
///
/// `new ClrPropertyInfo<TSource, TValue>(property.Name, getter, setter)`
///
/// * `getter`: `Func<TSource, TValue>` that calls [`getter`](Self::getter) on its argument (a
///   static getter is called without it) and returns the value unboxed; null without a getter.
/// * `setter`: `Action<TSource, TValue>` that calls [`setter`](Self::setter) on its first
///   argument with its second (a static setter gets only the value), discarding a result;
///   null without a setter.
///
/// Rust runtime: `ferroui_base::data::core::TypedClrPropertyInfo<S, V>`
/// (`CompiledBindingPathBuilder::typed_property` / `typed_property_info`).
pub struct XamlIlTypedClrPropertyInfo {
    /// The base name of the generated members: `<key>!Typed`, or `<key>!Typed_<identifier>`
    /// when another property with the same key was described before.
    pub name: String,
    pub property: Rc<dyn IXamlProperty>,
    /// `TSource`: the declaring type of the getter (or, without one, of the setter).
    pub source_type: Rc<dyn IXamlType>,
    /// `TValue`: `property.PropertyType`.
    pub value_type: Rc<dyn IXamlType>,
    /// `IPropertyInfo<TSource, TValue>`: the type of the value left on the stack.
    pub property_info_type: Rc<dyn IXamlType>,
    /// `ClrPropertyInfo<TSource, TValue>`: the type to construct.
    pub clr_property_info_type: Rc<dyn IXamlType>,
    pub getter: Option<Rc<dyn IXamlMethod>>,
    pub setter: Option<Rc<dyn IXamlMethod>>,
}

/// `XamlIlClrPropertyInfoEmitter`: hands out property info descriptions, one per distinct
/// property. Upstream it is a configuration extra created with the type builder of the helper
/// type; here it needs no builder: `configuration.get_or_create_extra::<XamlIlClrPropertyInfoEmitter>()`.
#[derive(Default)]
pub struct XamlIlClrPropertyInfoEmitter {
    fields: RefCell<HashMap<String, Vec<Rc<XamlIlClrPropertyInfo>>>>,
    typed_fields: RefCell<HashMap<String, Vec<Rc<XamlIlTypedClrPropertyInfo>>>>,
}

fn declaring_type_of(property: &dyn IXamlProperty) -> XamlResult<Rc<dyn IXamlType>> {
    property
        .getter()
        .or_else(|| property.setter())
        .map(|accessor| accessor.declaring_type())
        .ok_or_else(|| {
            XamlError::invalid_operation(format!(
                "Couldn't get declaring type for property {}",
                property.name()
            ))
        })
}

fn same_accessor(a: Option<Rc<dyn IXamlMethod>>, b: Option<Rc<dyn IXamlMethod>>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(a), Some(b)) => a.equals(&*b),
        _ => false,
    }
}

/// The cache lookup of `GetCachedPropertyInfoMethod`: an entry is reused when its property has
/// the same getter and the same setter.
fn same_accessors(a: &dyn IXamlProperty, b: &dyn IXamlProperty) -> bool {
    same_accessor(a.getter(), b.getter()) && same_accessor(a.setter(), b.setter())
}

impl XamlIlClrPropertyInfoEmitter {
    pub fn new() -> Self {
        Self::default()
    }

    /// `GetKey(property, indexerArgumentsKey)`:
    /// `<declaring type full name>.<property name>`, followed by `[<indexer arguments key>]`
    /// for an indexer.
    pub fn get_key(
        property: &dyn IXamlProperty,
        indexer_arguments_key: Option<&str>,
    ) -> XamlResult<String> {
        let declaring_type = declaring_type_of(property)?;

        let base_key = format!("{}.{}", declaring_type.get_full_name(), property.name());

        Ok(match indexer_arguments_key {
            None => base_key,
            Some(indexer_arguments_key) => format!("{base_key}[{indexer_arguments_key}]"),
        })
    }

    /// `Emit(context, codeGen, property, indexerArguments, indexerArgumentsKey)`: the untyped
    /// property info of `property`.
    ///
    /// Upstream emits a call of the generated `<name>!Property` method (stack effect: pushes
    /// one `IPropertyInfo`) and returns `IPropertyInfo`. Here the description is returned; the
    /// same `Rc` is returned for the same key and accessors, as upstream reuses the method.
    ///
    /// As upstream, a cache hit ignores the indexer argument nodes: they are part of the
    /// identity only through `indexer_arguments_key`.
    pub fn emit(
        &self,
        configuration: &TransformerConfiguration,
        property: &Rc<dyn IXamlProperty>,
        indexer_arguments: Option<&[Rc<dyn IXamlAstValueNode>]>,
        indexer_arguments_key: Option<&str>,
    ) -> XamlResult<Rc<XamlIlClrPropertyInfo>> {
        let indexer_arguments = indexer_arguments.unwrap_or(&[]);
        let boolean = configuration.well_known_types().boolean.clone();
        let key = Self::get_key(&**property, indexer_arguments_key)?;

        let cached_count = {
            let fields = self.fields.borrow();
            let cached = fields.get(&key);
            if let Some(entry) = cached.and_then(|list| {
                list.iter()
                    .find(|entry| same_accessors(&*entry.property, &**property))
            }) {
                return Ok(entry.clone());
            }
            cached.map_or(0, Vec::len)
        };

        let name = if cached_count == 0 {
            key.clone()
        } else {
            format!(
                "{key}_{}",
                configuration.identifier_generator.generate_identifier_part()
            )
        };

        let getter = property.getter().map(|method| {
            let return_type = method.return_type();
            let result = if return_type.equals(&*boolean) {
                XamlIlClrPropertyGetterResult::CachedBoxedBoolean
            } else if return_type.is_value_type() {
                XamlIlClrPropertyGetterResult::Boxed(return_type)
            } else {
                XamlIlClrPropertyGetterResult::Reference
            };
            XamlIlClrPropertyInfoGetter {
                pass_this: !method.is_static(),
                this_is_value_type: method.declaring_type().is_value_type(),
                method,
                result,
            }
        });

        let setter = match property.setter() {
            None => None,
            Some(method) => {
                let value_index = indexer_arguments.len();
                let value_type = method.parameters().get(value_index).cloned().ok_or_else(|| {
                    XamlError::internal(
                        "ArgumentOutOfRangeException",
                        format!(
                            "The setter of {key} doesn't have a value parameter after {value_index} indexer argument(s)"
                        ),
                    )
                })?;
                Some(XamlIlClrPropertyInfoSetter {
                    pass_this: !method.is_static(),
                    this_is_value_type: method.declaring_type().is_value_type(),
                    value_is_value_type: value_type.is_value_type(),
                    value_type,
                    method,
                })
            }
        };

        let info = Rc::new(XamlIlClrPropertyInfo {
            name,
            property: property.clone(),
            indexer_arguments: indexer_arguments.to_vec(),
            getter,
            setter,
        });
        self.fields
            .borrow_mut()
            .entry(key)
            .or_default()
            .push(info.clone());
        Ok(info)
    }

    /// `EmitTyped(context, codeGen, property)`: the typed property info of `property`.
    ///
    /// Upstream emits a call of the generated `<name>!Property` method (stack effect: pushes
    /// one `IPropertyInfo<TSource, TValue>`) and returns that type. Here the description is
    /// returned; the same `Rc` is returned for the same key and accessors.
    pub fn emit_typed(
        &self,
        configuration: &TransformerConfiguration,
        property: &Rc<dyn IXamlProperty>,
    ) -> XamlResult<Rc<XamlIlTypedClrPropertyInfo>> {
        let types = configuration.try_get_ferro_types()?;
        let source_type = declaring_type_of(&**property)?;
        let value_type = property.property_type();

        let key = Self::get_key(&**property, None)?;

        let cached_count = {
            let typed_fields = self.typed_fields.borrow();
            let cached = typed_fields.get(&key);
            if let Some(entry) = cached.and_then(|list| {
                list.iter()
                    .find(|entry| same_accessors(&*entry.property, &**property))
            }) {
                return Ok(entry.clone());
            }
            cached.map_or(0, Vec::len)
        };

        let type_arguments = [source_type.clone(), value_type.clone()];
        let property_info_type = types.i_property_info_t.make_generic_type(&type_arguments)?;
        // Only construct the generic types on a cache miss: they're not needed when an
        // existing property info method is reused.
        let clr_property_info_type = types.clr_property_info_t.make_generic_type(&type_arguments)?;

        let name = if cached_count == 0 {
            format!("{key}!Typed")
        } else {
            format!(
                "{key}!Typed_{}",
                configuration.identifier_generator.generate_identifier_part()
            )
        };

        let info = Rc::new(XamlIlTypedClrPropertyInfo {
            name,
            property: property.clone(),
            source_type,
            value_type,
            property_info_type,
            clr_property_info_type,
            getter: property.getter(),
            setter: property.setter(),
        });
        self.typed_fields
            .borrow_mut()
            .entry(key)
            .or_default()
            .push(info.clone());
        Ok(info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::bindings::create_bindings_test_framework;

    fn property(type_: &Rc<dyn IXamlType>, name: &str) -> Rc<dyn IXamlProperty> {
        type_
            .properties()
            .into_iter()
            .find(|p| p.name() == name)
            .expect("property")
    }

    #[test]
    fn keys_name_the_declaring_type_the_property_and_the_indexer_arguments() {
        let fw = create_bindings_test_framework();
        let window = fw.t("FerroUI.Controls.Window");
        let title = property(&window, "Title");
        assert_eq!(
            XamlIlClrPropertyInfoEmitter::get_key(&*title, None).expect("key"),
            "FerroUI.Controls.Window,FerroUI.Controls.Title"
        );
        assert_eq!(
            XamlIlClrPropertyInfoEmitter::get_key(&*title, Some("1,2")).expect("key"),
            "FerroUI.Controls.Window,FerroUI.Controls.Title[1,2]"
        );
    }

    #[test]
    fn properties_with_the_same_key_and_other_accessors_get_their_own_description() {
        let fw = create_bindings_test_framework();
        let definition = fw.t("System.Collections.ObjectModel.ObservableCollection`1");
        let of_string = definition
            .make_generic_type(&[fw.t("System.String")])
            .expect("type");
        let of_int = definition
            .make_generic_type(&[fw.t("System.Int32")])
            .expect("type");
        let string_item = property(&of_string, "Item");
        let int_item = property(&of_int, "Item");

        let emitter = XamlIlClrPropertyInfoEmitter::new();
        let first = emitter
            .emit(&fw.configuration, &string_item, None, Some("0"))
            .expect("info");
        let second = emitter
            .emit(&fw.configuration, &int_item, None, Some("0"))
            .expect("info");
        assert!(!Rc::ptr_eq(&first, &second));
        let key = "System.Collections.ObjectModel.ObservableCollection`1,System.Runtime.Item[0]";
        assert_eq!(first.name, key);
        // The second description with the same key gets a generated suffix.
        assert!(second.name.starts_with(&format!("{key}_")));
        assert!(second.name.len() > key.len() + 1);
        // Both are found again.
        assert!(Rc::ptr_eq(
            &first,
            &emitter
                .emit(&fw.configuration, &property(&of_string, "Item"), None, Some("0"))
                .expect("info")
        ));
        assert!(Rc::ptr_eq(
            &second,
            &emitter
                .emit(&fw.configuration, &property(&of_int, "Item"), None, Some("0"))
                .expect("info")
        ));
        // Another indexer argument key is another description.
        let other_index = emitter
            .emit(&fw.configuration, &string_item, None, Some("1"))
            .expect("info");
        assert!(!Rc::ptr_eq(&first, &other_index));

        // The typed descriptions are cached separately, with their own names.
        let typed = emitter.emit_typed(&fw.configuration, &string_item).expect("typed");
        assert_eq!(
            typed.name,
            "System.Collections.ObjectModel.ObservableCollection`1,System.Runtime.Item!Typed"
        );
        let typed_int = emitter.emit_typed(&fw.configuration, &int_item).expect("typed");
        assert!(typed_int.name.starts_with(
            "System.Collections.ObjectModel.ObservableCollection`1,System.Runtime.Item!Typed_"
        ));
        assert!(Rc::ptr_eq(
            &typed,
            &emitter.emit_typed(&fw.configuration, &string_item).expect("typed")
        ));
    }

    #[test]
    fn an_indexer_setter_takes_the_value_after_the_indexer_arguments() {
        let fw = create_bindings_test_framework();
        let list = fw
            .t("System.Collections.Generic.List`1")
            .make_generic_type(&[fw.t("System.Int32")])
            .expect("type");
        let item = property(&list, "Item");
        let index: Rc<dyn IXamlAstValueNode> = xamlx::ast::XamlAstTextNode::with_type(
            &xamlx::ast::XamlLineInfo::new(1, 1),
            "0",
            false,
            Some(fw.t("System.Int32")),
        );
        let emitter = XamlIlClrPropertyInfoEmitter::new();
        let info = emitter
            .emit(&fw.configuration, &item, Some(&[index]), Some("0"))
            .expect("info");
        assert_eq!(info.indexer_arguments.len(), 1);
        let setter = info.setter.as_ref().expect("setter");
        assert_eq!(setter.value_type.full_name(), "System.Int32");
        assert!(setter.value_is_value_type);
        assert!(matches!(
            info.getter.as_ref().expect("getter").result,
            XamlIlClrPropertyGetterResult::Boxed(_)
        ));

        // Described as a plain property, the "value" parameter would be the index: the
        // setter of an indexer needs its indexer arguments.
        let plain = XamlIlClrPropertyInfoEmitter::new()
            .emit(&fw.configuration, &item, None, None)
            .expect("info");
        assert_eq!(
            plain.setter.as_ref().expect("setter").value_type.full_name(),
            "System.Int32"
        );
    }
}
