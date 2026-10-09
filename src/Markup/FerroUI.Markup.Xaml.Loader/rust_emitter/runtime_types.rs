//! [`EmitTypes`] of the run-time type system: a Rust type is its `TypeId`,
//! and everything the emitter asks is answered by the registries of the
//! process (the runtime types of the classes, the markup metadata, the
//! property registry, the untyped value conversions) and by what the
//! `compiler-metadata` feature of the base crate records for the emitter
//! (the public Rust paths, the typed functions, the accessors of property
//! definitions).

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::data::CompiledBindingPath;
use ferroui_base::metadata::{property_accessors, rust_path_of_type, IServiceProvider, MarkupType, MarkupTyped, PropertyAccessor};
use ferroui_base::{BoxedValue, FerroProperty, StyledElement, TypeInfo};
use ferroui_markup_xaml::converters::ITypeDescriptorContext;
use ferroui_markup_xaml::xaml_il::runtime::DeferredContent;
use xamlx::type_system::{IXamlConstructor, IXamlField, IXamlMethod, IXamlType};

use crate::runtime::type_system::{
    DeclaredMember, RuntimeConstructor, RuntimeField, RuntimeFieldValue, RuntimeInvoker, RuntimeMethod, RuntimeType,
};

use super::emit_types::{
    ConstructorInfo, DeclaredKind, DeclaredMethod, EmitClass, EmitFunction, EmitMarkup, EmitProperty, EmitTypes, EnumMember, FieldInfo,
    FieldValue, FrameworkType, Handle, Known, MethodInfo, TypeKey,
};

/// A public Rust path from the registries as an absolute path
/// (`::ferroui_controls::Border`), which no item of the including module can
/// shadow.
fn absolute(path: &str) -> String {
    match path.starts_with("::") {
        true => path.to_string(),
        false => format!("::{path}"),
    }
}

fn id_of(key: TypeKey<'_>) -> Option<TypeId> {
    match key {
        TypeKey::Id(id) => Some(id),
        TypeKey::Text(_) => None,
    }
}

fn handle_of(handle: ValueType) -> Handle<'static> {
    Handle { key: TypeKey::Id(handle.id()), object: handle.is_object(), name: handle.name() }
}

/// A class of the run-time type system.
pub struct RuntimeClass(&'static TypeInfo);

/// The markup metadata of a type of the run-time type system.
pub struct RuntimeMarkup(&'static MarkupType);

/// A registered property of the run-time type system.
pub struct RuntimeEmitProperty(&'static FerroProperty);

thread_local! {
    /// The descriptions handed out so far, by the address of what they describe: one per
    /// class, metadata and property of the process, kept as long as the registries they
    /// describe (the life of the thread).
    static CLASSES: RefCell<HashMap<usize, &'static RuntimeClass>> = RefCell::new(HashMap::new());
    static MARKUP: RefCell<HashMap<usize, &'static RuntimeMarkup>> = RefCell::new(HashMap::new());
    static PROPERTIES: RefCell<HashMap<usize, &'static RuntimeEmitProperty>> = RefCell::new(HashMap::new());
}

/// The description of the class `class`.
pub fn class(class: &'static TypeInfo) -> &'static RuntimeClass {
    CLASSES.with(|known| *known.borrow_mut().entry(class as *const TypeInfo as usize).or_insert_with(|| Box::leak(Box::new(RuntimeClass(class)))))
}

fn markup(markup: &'static MarkupType) -> &'static RuntimeMarkup {
    MARKUP.with(|known| *known.borrow_mut().entry(markup as *const MarkupType as usize).or_insert_with(|| Box::leak(Box::new(RuntimeMarkup(markup)))))
}

fn property(property: &'static FerroProperty) -> &'static RuntimeEmitProperty {
    PROPERTIES
        .with(|known| *known.borrow_mut().entry(property as *const FerroProperty as usize).or_insert_with(|| Box::leak(Box::new(RuntimeEmitProperty(property)))))
}

impl RuntimeClass {
    /// The runtime type of the class.
    pub fn type_info(&self) -> &'static TypeInfo {
        self.0
    }
}

/// The runtime type of a class the run-time type system described.
pub fn type_info_of(class: &dyn EmitClass) -> Option<&'static TypeInfo> {
    class.as_any().downcast_ref::<RuntimeClass>().map(RuntimeClass::type_info)
}

impl EmitClass for RuntimeClass {
    fn name(&self) -> &str {
        self.0.name()
    }
    fn full_name(&self) -> String {
        self.0.full_name()
    }
    fn rust_path(&self) -> Option<&str> {
        self.0.rust_path()
    }
    fn handle(&self) -> Option<TypeKey<'_>> {
        self.0.handle().map(TypeKey::Id)
    }
    fn has_default_constructor(&self) -> bool {
        self.0.default_constructor().is_some()
    }
    fn is_assignable_from(&self, other: &dyn EmitClass) -> bool {
        type_info_of(other).is_some_and(|other| self.0.is_assignable_from(other))
    }
    fn same(&self, other: &dyn EmitClass) -> bool {
        type_info_of(other).is_some_and(|other| std::ptr::eq(self.0, other))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl EmitMarkup for RuntimeMarkup {
    fn full_name(&self) -> String {
        self.0.full_name()
    }
    fn rust_path(&self) -> Option<&str> {
        self.0.rust_path()
    }
    fn rust_path_is_trait(&self) -> bool {
        self.0.rust_path_is_trait()
    }
    fn handles(&self) -> Vec<TypeKey<'_>> {
        self.0.handles.iter().map(|handle| TypeKey::Id(handle().id())).collect()
    }
    fn handle_is_shared(&self) -> bool {
        self.0.handle().is_some_and(|handle| handle.name().starts_with("alloc::rc::Rc<"))
    }
    fn nullable(&self) -> Option<TypeKey<'_>> {
        self.0.nullable.map(|nullable| TypeKey::Id(nullable().id()))
    }
    fn interfaces(&self) -> Vec<TypeKey<'_>> {
        self.0.interfaces.iter().map(|interface| TypeKey::Id(interface().id())).collect()
    }
    fn base_type(&self) -> Option<&dyn EmitMarkup> {
        self.0.base_type().map(|base| markup(base) as &dyn EmitMarkup)
    }
    fn this(&self) -> Option<TypeKey<'_>> {
        self.0.this.map(|this| TypeKey::Id(this().id()))
    }
    fn value(&self) -> Option<TypeKey<'_>> {
        self.0.value.map(|value| TypeKey::Id(value().id()))
    }
    fn is_flags(&self) -> bool {
        self.0.is_flags
    }
    fn enum_members(&self) -> Vec<EnumMember<'_>> {
        self.0.enum_members.iter().map(|member| EnumMember { name: member.name, value: member.value, rust_variant: member.rust_variant }).collect()
    }
    fn flags_compose(&self, value: i64) -> bool {
        self.0.enum_from_value.is_some_and(|from_value| from_value(value).is_some())
    }
    fn same(&self, other: &dyn EmitMarkup) -> bool {
        other.as_any().downcast_ref::<RuntimeMarkup>().is_some_and(|other| std::ptr::eq(self.0, other.0))
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn handle(&self) -> Option<TypeKey<'_>> {
        self.0.handle().map(|handle| TypeKey::Id(handle.id()))
    }
}

impl EmitProperty for RuntimeEmitProperty {
    fn name(&self) -> &str {
        self.0.name()
    }
    fn owner_name(&self) -> String {
        self.0.owner_type().name().to_string()
    }
    fn is_direct(&self) -> bool {
        self.0.is_direct()
    }
    fn is_read_only(&self) -> bool {
        self.0.is_read_only()
    }
    fn property_type(&self) -> TypeKey<'_> {
        TypeKey::Id(self.0.property_type())
    }
    fn property_type_name(&self) -> String {
        self.0.property_type_name().to_string()
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn runtime_type(type_: &dyn IXamlType) -> Option<&RuntimeType> {
    type_.as_any().downcast_ref::<RuntimeType>()
}

fn emit_function(emit: ferroui_base::metadata::MarkupEmit) -> EmitFunction {
    EmitFunction { function: emit.function.to_string(), fallible: emit.fallible }
}

/// [`EmitTypes`] of the run-time type system; see the module.
pub struct RuntimeEmitTypes;

impl EmitTypes for RuntimeEmitTypes {
    fn known(&self, known: Known) -> TypeKey<'_> {
        TypeKey::Id(match known {
            Known::String => TypeId::of::<String>(),
            Known::OptionString => TypeId::of::<Option<String>>(),
            Known::Bool => TypeId::of::<bool>(),
            Known::Char => TypeId::of::<char>(),
            Known::I8 => TypeId::of::<i8>(),
            Known::U8 => TypeId::of::<u8>(),
            Known::I16 => TypeId::of::<i16>(),
            Known::U16 => TypeId::of::<u16>(),
            Known::I32 => TypeId::of::<i32>(),
            Known::U32 => TypeId::of::<u32>(),
            Known::I64 => TypeId::of::<i64>(),
            Known::U64 => TypeId::of::<u64>(),
            Known::F32 => TypeId::of::<f32>(),
            Known::F64 => TypeId::of::<f64>(),
            Known::Object => TypeId::of::<Option<BoxedValue>>(),
            Known::ServiceProvider => TypeId::of::<Rc<dyn IServiceProvider>>(),
            Known::OptionServiceProvider => TypeId::of::<Option<Rc<dyn IServiceProvider>>>(),
            Known::Selector => TypeId::of::<ferroui_base::styling::Selector>(),
            Known::OptionSelector => TypeId::of::<Option<ferroui_base::styling::Selector>>(),
            Known::OptionUri => TypeId::of::<Option<ferroui_base::utilities::Uri>>(),
            Known::Class => TypeId::of::<&'static TypeInfo>(),
            Known::OptionClass => TypeId::of::<Option<&'static TypeInfo>>(),
            Known::ValueType => TypeId::of::<ValueType>(),
            Known::OptionValueType => TypeId::of::<Option<ValueType>>(),
            Known::TypeId => TypeId::of::<TypeId>(),
            Known::OptionTypeId => TypeId::of::<Option<TypeId>>(),
            Known::BindingPriority => TypeId::of::<ferroui_base::data::BindingPriority>(),
            Known::DeferredContent => TypeId::of::<Rc<DeferredContent>>(),
            Known::CompiledBindingPath => TypeId::of::<CompiledBindingPath>(),
            Known::Property => TypeId::of::<&'static FerroProperty>(),
            Known::OptionProperty => TypeId::of::<Option<&'static FerroProperty>>(),
            Known::Delegate => TypeId::of::<ferroui_base::metadata::MarkupDelegate>(),
            Known::TypeDescriptorContext => TypeId::of::<Rc<dyn ITypeDescriptorContext>>(),
            Known::OptionTypeDescriptorContext => TypeId::of::<Option<Rc<dyn ITypeDescriptorContext>>>(),
        })
    }

    fn option_of_known(&self, known: Known) -> TypeKey<'_> {
        TypeKey::Id(match known {
            Known::String => TypeId::of::<Option<String>>(),
            Known::OptionString => TypeId::of::<Option<Option<String>>>(),
            Known::Bool => TypeId::of::<Option<bool>>(),
            Known::Char => TypeId::of::<Option<char>>(),
            Known::I8 => TypeId::of::<Option<i8>>(),
            Known::U8 => TypeId::of::<Option<u8>>(),
            Known::I16 => TypeId::of::<Option<i16>>(),
            Known::U16 => TypeId::of::<Option<u16>>(),
            Known::I32 => TypeId::of::<Option<i32>>(),
            Known::U32 => TypeId::of::<Option<u32>>(),
            Known::I64 => TypeId::of::<Option<i64>>(),
            Known::U64 => TypeId::of::<Option<u64>>(),
            Known::F32 => TypeId::of::<Option<f32>>(),
            Known::F64 => TypeId::of::<Option<f64>>(),
            Known::Object => TypeId::of::<Option<Option<BoxedValue>>>(),
            Known::ServiceProvider => TypeId::of::<Option<Rc<dyn IServiceProvider>>>(),
            Known::OptionServiceProvider => TypeId::of::<Option<Option<Rc<dyn IServiceProvider>>>>(),
            Known::Selector => TypeId::of::<Option<ferroui_base::styling::Selector>>(),
            Known::OptionSelector => TypeId::of::<Option<Option<ferroui_base::styling::Selector>>>(),
            Known::OptionUri => TypeId::of::<Option<Option<ferroui_base::utilities::Uri>>>(),
            Known::Class => TypeId::of::<Option<&'static TypeInfo>>(),
            Known::OptionClass => TypeId::of::<Option<Option<&'static TypeInfo>>>(),
            Known::ValueType => TypeId::of::<Option<ValueType>>(),
            Known::OptionValueType => TypeId::of::<Option<Option<ValueType>>>(),
            Known::TypeId => TypeId::of::<Option<TypeId>>(),
            Known::OptionTypeId => TypeId::of::<Option<Option<TypeId>>>(),
            Known::BindingPriority => TypeId::of::<Option<ferroui_base::data::BindingPriority>>(),
            Known::DeferredContent => TypeId::of::<Option<Rc<DeferredContent>>>(),
            Known::CompiledBindingPath => TypeId::of::<Option<CompiledBindingPath>>(),
            Known::Property => TypeId::of::<Option<&'static FerroProperty>>(),
            Known::OptionProperty => TypeId::of::<Option<Option<&'static FerroProperty>>>(),
            Known::Delegate => TypeId::of::<Option<ferroui_base::metadata::MarkupDelegate>>(),
            Known::TypeDescriptorContext => TypeId::of::<Option<Rc<dyn ITypeDescriptorContext>>>(),
            Known::OptionTypeDescriptorContext => TypeId::of::<Option<Option<Rc<dyn ITypeDescriptorContext>>>>(),
        })
    }

    fn is_own(&self, type_: &dyn IXamlType) -> bool {
        runtime_type(type_).is_some()
    }

    fn class_of(&self, type_: &dyn IXamlType) -> Option<&dyn EmitClass> {
        runtime_type(type_).and_then(RuntimeType::type_info).map(|info| class(info) as &dyn EmitClass)
    }

    fn markup_of(&self, type_: &dyn IXamlType) -> Option<&dyn EmitMarkup> {
        runtime_type(type_).and_then(RuntimeType::markup).map(|found| markup(found) as &dyn EmitMarkup)
    }

    fn metadata_of(&self, type_: &dyn IXamlType) -> Option<&dyn EmitMarkup> {
        let runtime = runtime_type(type_)?;
        let found = runtime.markup().or_else(|| MarkupType::find_by_handle(runtime.handle()?.id()))?;
        Some(markup(found))
    }

    fn handle_of(&self, type_: &dyn IXamlType) -> Option<Handle<'_>> {
        runtime_type(type_).and_then(RuntimeType::handle).map(handle_of)
    }

    fn class_by_handle(&self, key: TypeKey<'_>) -> Option<(&dyn EmitClass, bool)> {
        TypeInfo::find_by_handle(id_of(key)?).map(|(info, nullable)| (class(info) as &dyn EmitClass, nullable))
    }

    fn markup_by_handle(&self, key: TypeKey<'_>) -> Option<&dyn EmitMarkup> {
        MarkupType::find_by_handle(id_of(key)?).map(|found| markup(found) as &dyn EmitMarkup)
    }

    fn element_ref_class(&self, key: TypeKey<'_>) -> Option<(&dyn EmitClass, bool)> {
        ValueTypes::element_ref_class(id_of(key)?).map(|(info, nullable)| (class(info) as &dyn EmitClass, nullable))
    }

    fn primitive_type_name(&self, key: TypeKey<'_>) -> Option<&'static str> {
        let id = id_of(key)?;
        macro_rules! primitives {
            ($($type_:ty),*) => {
                [$((TypeId::of::<$type_>(), ::std::stringify!($type_))),*]
            };
        }
        let table = primitives!(bool, char, i8, u8, i16, u16, i32, u32, i64, u64, f32, f64);
        if id == TypeId::of::<String>() {
            // Not a primitive of the language: named by its full path, as generated code names
            // the items of the prelude.
            return Some("::std::string::String");
        }
        table.iter().find(|(known, _)| *known == id).map(|(_, name)| *name)
    }

    fn is_assignable(&self, from: TypeKey<'_>, to: TypeKey<'_>) -> bool {
        match (id_of(from), id_of(to)) {
            (Some(from), Some(to)) => ValueTypes::is_assignable(ValueType::new(from, ""), ValueType::new(to, "")),
            _ => false,
        }
    }

    fn null_converts_to(&self, target: TypeKey<'_>) -> bool {
        let Some(target) = id_of(target) else { return false };
        matches!(
            ValueTypes::try_convert(None, ValueType::new(target, "")),
            Some(Some(null)) if null.value_type_id() == target
        )
    }

    fn nullable_inner(&self, key: TypeKey<'_>) -> Option<TypeKey<'_>> {
        ValueTypes::nullable_inner(ValueType::new(id_of(key)?, "")).map(|inner| TypeKey::Id(inner.id()))
    }

    fn is_styled_element(&self, class: &dyn EmitClass) -> bool {
        type_info_of(class).is_some_and(|class| StyledElement::TYPE.is_assignable_from(class))
    }

    fn has_class_document(&self, class: &dyn EmitClass) -> bool {
        type_info_of(class).is_some_and(crate::FerroRuntimeXamlLoader::has_class_document)
    }

    fn framework_path(&self, type_: FrameworkType) -> Option<String> {
        let path = match type_ {
            FrameworkType::Setter => <ferroui_base::styling::Setter as MarkupTyped>::MARKUP.rust_path(),
            FrameworkType::SetterBase => <dyn ferroui_base::styling::SetterBase as MarkupTyped>::MARKUP.rust_path(),
            FrameworkType::StyleBase => ferroui_base::styling::StyleBase::TYPE.rust_path(),
        };
        path.map(absolute)
    }

    fn method(&self, method: &dyn IXamlMethod) -> Option<MethodInfo<'_>> {
        let runtime = method.as_any().downcast_ref::<RuntimeMethod>()?;
        let declared = runtime.declared().map(|declared| {
            let (kind, returns) = match declared {
                DeclaredMember::Getter(property) => (DeclaredKind::Getter, Some((property.type_)())),
                DeclaredMember::StaticGetter(property) => (DeclaredKind::StaticGetter, Some((property.type_)())),
                DeclaredMember::Setter(_) => (DeclaredKind::Setter, None),
                DeclaredMember::StaticSetter(_) => (DeclaredKind::StaticSetter, None),
                DeclaredMember::Method(method) => (DeclaredKind::Method, method.return_type.map(|return_type| return_type())),
                DeclaredMember::Parse(markup) => (DeclaredKind::Parse, markup.value.map(|value| value())),
            };
            DeclaredMethod { kind, emit: declared.emit().map(emit_function), returns: returns.map(|handle| TypeKey::Id(handle.id())) }
        });
        Some(MethodInfo::new(
            runtime.name.clone(),
            runtime.is_static,
            runtime.declaring_type.upgrade().map(|declaring| declaring as Rc<dyn IXamlType>),
            runtime.parameters.clone(),
            runtime.parameter_handles.iter().map(|handle| handle.map(handle_of)).collect(),
            declared,
            matches!(runtime.invoker, RuntimeInvoker::Dynamic(_)),
        ))
    }

    fn constructor(&self, constructor: &dyn IXamlConstructor) -> Option<ConstructorInfo<'_>> {
        let runtime = constructor.as_any().downcast_ref::<RuntimeConstructor>()?;
        Some(ConstructorInfo {
            declaring_type: runtime.declaring_type.upgrade().map(|declaring| declaring as Rc<dyn IXamlType>),
            parameters: runtime.parameters.clone(),
            parameter_handles: runtime.parameter_handles.iter().map(|handle| handle.map(handle_of)).collect(),
            declared: runtime.declared.map(|declared| declared.emit.map(emit_function)),
            built: matches!(runtime.invoker, RuntimeInvoker::Dynamic(_)),
        })
    }

    fn field(&self, field: &dyn IXamlField) -> Option<FieldInfo<'_>> {
        let runtime = field.as_any().downcast_ref::<RuntimeField>()?;
        let value = match runtime.value {
            RuntimeFieldValue::Declared(declared) => FieldValue::Declared {
                name: declared.name.to_string(),
                emit: declared.emit.map(emit_function),
                type_: TypeKey::Id((declared.type_)().id()),
            },
            RuntimeFieldValue::EnumMember(_) => FieldValue::EnumMember,
            _ => FieldValue::Other,
        };
        Some(FieldInfo {
            declaring_type: runtime.declaring_type.upgrade().map(|declaring| declaring as Rc<dyn IXamlType>),
            property: runtime.ferro_property().map(|found| property(found) as &dyn EmitProperty),
            value,
        })
    }

    fn property_definition(&self, property: &dyn EmitProperty, preferred: Option<&dyn EmitClass>) -> Result<String, String> {
        let property = property
            .as_any()
            .downcast_ref::<RuntimeEmitProperty>()
            .ok_or_else(|| "the property is not one of the run-time type system".to_string())?
            .0;
        let accessors = property_accessors(property, preferred.and_then(type_info_of));
        let chosen = accessors.iter().find_map(|accessor: &PropertyAccessor| {
            accessor
                .public
                .then(|| rust_path_of_type(accessor.impl_type))
                .flatten()
                .map(|path| format!("{}::{}()", absolute(path), accessor.name))
        });
        chosen.ok_or_else(|| match accessors.is_empty() {
            true => format!("no accessor of the property {} is recorded", property.name()),
            false => format!("no accessor of the property {} is public and declared by a type with a public Rust path", property.name()),
        })
    }
}
