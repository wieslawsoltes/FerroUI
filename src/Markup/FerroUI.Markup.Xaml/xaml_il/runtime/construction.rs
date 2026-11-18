//! Execution of compiler-resolved construction programs. Registers carry the original object
//! handles. Property imports call the same typed setter as native markup.
use super::compiled::{
    bind_dynamic_resource, bind_template_binding, invoked, register_name_value, PropertyFn,
};
use super::XamlIlContext;
use crate::{markup_extensions::CompiledBindingExtension, XamlLoadError};
use ferroui_base::controls::INameScope;
use ferroui_base::data::CompiledBindingPath;
use ferroui_base::{FerroObject, Ref, StyledElement};
use ferroui_markup::construction_opcodes as op;
use std::{any::Any, rc::Rc};

pub enum ConstructionImport {
    New(fn() -> Ref<FerroObject>),
    ObjectLiteral(fn(&[u8]) -> Result<Ref<FerroObject>, XamlLoadError>),
    Path(&'static super::binding_program::BindingPathProgram),
    Selector(&'static super::compiled::TypedSelectorProgram),
    DeferredType(fn() -> ferroui_base::data::core::ValueType),
    Deferred(super::compiled::DeferredBuild),
    Float(fn(&FerroObject, f64)),
    Int(fn(&FerroObject, i32)),
    Four(fn(&FerroObject, [f64; 4])),
    String(fn(&FerroObject, &str)),
    Constant(fn(&FerroObject)),
    Object(fn(&FerroObject, &Ref<FerroObject>) -> Result<(), XamlLoadError>),
    RegisterSetter(fn(&FerroObject, ConstructionInput<'_>, i32, i32) -> Result<(), XamlLoadError>),
    SetterValue(fn(ConstructionInput<'_>) -> Result<ferroui_base::metadata::MarkupValue, XamlLoadError>),
    Markup(fn(&Ref<FerroObject>) -> Result<ferroui_base::metadata::MarkupValue, XamlLoadError>),
    Property(PropertyFn),
    TargetProperty(fn() -> ferroui_base::metadata::MarkupValue),
    Namespaces(fn() -> super::compiled::XmlNamespaceTable),
    ThemeParts(fn() -> super::compiled::ThemeParts),
    MoveToMarkup(fn(ConstructionRegister) -> Result<ferroui_base::metadata::MarkupValue, XamlLoadError>),
    LiteralValue(
        fn(
            &mut ConstructionArguments<'_, '_>,
        ) -> Result<ferroui_base::metadata::MarkupValue, XamlLoadError>,
    ),
    LiteralSetter(
        fn(&FerroObject, &mut ConstructionArguments<'_, '_>) -> Result<(), XamlLoadError>,
    ),
    ResourceSetter(
        fn(
            &Ref<FerroObject>,
            ferroui_base::metadata::MarkupValue,
            i32,
            i32,
        ) -> Result<(), XamlLoadError>,
    ),
    Call(
        fn(
            &mut ConstructionArguments<'_, '_>,
        ) -> Result<Option<ConstructionRegister>, XamlLoadError>,
    ),
    BoxObject(fn(&Ref<FerroObject>) -> Result<ferroui_base::BoxedValue, XamlLoadError>),
}

/// Strings have their own index space so every typed operation stores only its tag
/// and one pointer. The two tables have the same reachability as their factory.
pub struct ConstructionImports {
    pub operations: &'static [ConstructionImport],
    pub strings: &'static [&'static str],
}

impl ConstructionImports {
    pub const EMPTY: Self = Self {
        operations: &[],
        strings: &[],
    };

    fn text(&self, code: &mut &[u8]) -> Result<&'static str, XamlLoadError> {
        self.strings
            .get(integer(code)? as usize)
            .copied()
            .ok_or_else(invalid)
    }
}

pub struct ConstructionProgram {
    pub code: &'static [u8],
    pub imports: &'static ConstructionImports,
}

#[cold]
fn invalid() -> XamlLoadError {
    XamlLoadError::with_message("Invalid compiled XAML construction program or import")
}

fn integer(code: &mut &[u8]) -> Result<u32, XamlLoadError> {
    ferroui_markup::compiled_operands::read_unsigned(code).map_err(|_| invalid())
}

pub fn compiled_path_geometry(mut bytes: &[u8]) -> Result<Ref<FerroObject>, XamlLoadError> {
    let commands = std::iter::from_fn(|| {
        (!bytes.is_empty()).then(|| ferroui_markup::path_opcodes::read(&mut bytes).map_err(|_| invalid()))
    });
    Ok(ferroui_base::media::StreamGeometry::try_from_commands(commands)?.upcast())
}

pub fn construction_object<T: ferroui_base::ObjectType>(
    value: Option<Ref<FerroObject>>,
) -> Result<Ref<T>, XamlLoadError> {
    value
        .ok_or_else(invalid)?
        .downcast::<T>()
        .map_err(|_| invalid())
}

/// Complete population of an existing class instance. All document-specific
/// construction instructions and their imports are held in the descriptor.
pub struct ObjectDocumentProgram {
    pub document: &'static super::compiled::DocumentInfo,
    pub construction: ConstructionProgram,
    pub root_value:
        fn(&Ref<FerroObject>) -> Result<ferroui_base::metadata::MarkupValue, XamlLoadError>,
}

impl ObjectDocumentProgram {
    #[inline(never)]
    pub fn populate(
        &self,
        parent: Option<Rc<dyn ferroui_base::metadata::IServiceProvider>>,
        root: &Ref<FerroObject>,
    ) -> Result<(), XamlLoadError> {
        let root = root.clone();
        let context =
            super::compiled::populate_context(parent, self.document, (self.root_value)(&root)?);
        let scope = context.name_scope_field();
        self.construction.apply(
            &[ConstructionInput::Object(&root)],
            scope.as_ref(),
            Some(&context),
        )
    }
}

/// Native objects keep their canonical identity. Other values retain their exact Rust
/// representation, including nullable and reference-counted collection types.
pub enum ConstructionRegister {
    /// Vacated by a native consumer after construction finishes.
    Empty,
    Object(Ref<FerroObject>),
    Value(Box<dyn Any>),
}

#[derive(Clone, Copy)]
pub enum ConstructionInput<'a> {
    Object(&'a Ref<FerroObject>),
    Value(&'a dyn Any),
}

impl<'a> ConstructionInput<'a> {
    pub fn object_ref<T: ferroui_base::ObjectType>(self) -> Result<&'a Ref<T>, XamlLoadError> {
        match self {
            Self::Object(object) => object.cast_ref::<T>().ok_or_else(invalid),
            Self::Value(_) => Err(invalid()),
        }
    }

    pub fn value<T: 'static>(self) -> Result<&'a T, XamlLoadError> {
        match self {
            Self::Value(value) => value.downcast_ref().ok_or_else(invalid),
            Self::Object(_) => Err(invalid()),
        }
    }
}

pub fn construction_result(
    result: Option<ConstructionRegister>,
) -> Result<ConstructionRegister, XamlLoadError> {
    result.ok_or_else(invalid)
}

impl ConstructionRegister {
    fn input(&self) -> Option<ConstructionInput<'_>> {
        match self {
            Self::Object(object) => Some(ConstructionInput::Object(object)),
            Self::Value(value) => Some(ConstructionInput::Value(value.as_ref())),
            Self::Empty => None,
        }
    }

    pub fn into_object<T: ferroui_base::ObjectType>(self) -> Result<Ref<T>, XamlLoadError> {
        match self {
            Self::Object(value) => construction_object(Some(value)),
            Self::Value(_) | Self::Empty => Err(invalid()),
        }
    }

    pub fn into_value<T: 'static>(self) -> Result<T, XamlLoadError> {
        match self {
            Self::Value(value) => value.downcast().map(|value| *value).map_err(|_| invalid()),
            Self::Object(_) | Self::Empty => Err(invalid()),
        }
    }
}

/// Operands of a compiler-resolved member call, borrowed for the duration of its import.
pub struct ConstructionArguments<'a, 'code> {
    encoded: &'code [u8],
    imports: &'static ConstructionImports,
    inputs: &'a [ConstructionInput<'a>],
    created: &'a [ConstructionRegister],
    context: Option<&'a Rc<XamlIlContext>>,
    pub line: i32,
    pub column: i32,
}

impl<'a> ConstructionArguments<'a, '_> {
    pub fn context(&self) -> Result<&'a Rc<XamlIlContext>, XamlLoadError> {
        self.context.ok_or_else(invalid)
    }

    pub fn text(&mut self) -> Result<&'static str, XamlLoadError> {
        self.imports.text(&mut self.encoded)
    }

    pub fn double(&mut self) -> Result<f64, XamlLoadError> {
        ferroui_markup::compiled_operands::read_number(&mut self.encoded).map_err(|_| invalid())
    }

    pub fn int(&mut self) -> Result<i32, XamlLoadError> {
        ferroui_markup::compiled_operands::read_signed(&mut self.encoded).map_err(|_| invalid())
    }

    pub fn uint(&mut self) -> Result<u32, XamlLoadError> {
        Ok(u32::from_le_bytes(
            ferroui_markup::compiled_operands::take(&mut self.encoded).map_err(|_| invalid())?,
        ))
    }

    pub fn boolean(&mut self) -> Result<bool, XamlLoadError> {
        match self.encoded.split_first() {
            Some((&value @ 0..=1, rest)) => {
                self.encoded = rest;
                Ok(value != 0)
            }
            _ => Err(invalid()),
        }
    }

    pub fn object<T: ferroui_base::ObjectType>(&mut self) -> Result<Ref<T>, XamlLoadError> {
        object(self.inputs, self.created, &mut self.encoded)?
            .cast::<T>()
            .ok_or_else(invalid)
    }

    /// Reads a receiver without creating an extra strong handle around a user accessor.
    pub fn object_ref<T: ferroui_base::ObjectType>(&mut self) -> Result<&'a Ref<T>, XamlLoadError> {
        object(self.inputs, self.created, &mut self.encoded)?.cast_ref::<T>().ok_or_else(invalid)
    }

    pub fn value<T: 'static>(&mut self) -> Result<&'a T, XamlLoadError> {
        value(self.inputs, self.created, &mut self.encoded)
    }
}

fn value<'a, T: 'static>(
    inputs: &[ConstructionInput<'a>],
    created: &'a [ConstructionRegister],
    code: &mut &[u8],
) -> Result<&'a T, XamlLoadError> {
    register(inputs, created, code)?.value()
}

fn register<'a>(
    inputs: &[ConstructionInput<'a>],
    created: &'a [ConstructionRegister],
    code: &mut &[u8],
) -> Result<ConstructionInput<'a>, XamlLoadError> {
    let i = integer(code)? as usize;
    match i & 1 {
        0 => inputs.get(i / 2).copied(),
        _ => created.get(i / 2).and_then(ConstructionRegister::input),
    }
    .ok_or_else(invalid)
}

fn object<'a>(
    inputs: &[ConstructionInput<'a>],
    created: &'a [ConstructionRegister],
    code: &mut &[u8],
) -> Result<&'a Ref<FerroObject>, XamlLoadError> {
    match register(inputs, created, code)? {
        ConstructionInput::Object(value) => Ok(value),
        ConstructionInput::Value(_) => Err(invalid()),
    }
}

/// A program's internal locals stay alive until the enclosing Rust scope ends, with the
/// same reverse declaration-order destruction as native XAML construction.
pub struct ConstructionObjects(Vec<ConstructionRegister>);

impl Drop for ConstructionObjects {
    fn drop(&mut self) {
        while self.0.pop().is_some() {}
    }
}

/// Reuse the execution vector to hold locals until the native scope ends. Native
/// consumers take individual registers. A guard drops each intervening range
/// in reverse declaration order at the corresponding point in the native scope.
pub struct ConstructionResults(Vec<ConstructionRegister>);

impl ConstructionResults {
    pub fn reader(&mut self) -> ConstructionReader<'_> {
        ConstructionReader(&mut self.0)
    }
}

pub struct ConstructionReader<'a>(&'a mut [ConstructionRegister]);

pub struct HeldConstructionRegisters<'a>(&'a mut [ConstructionRegister]);

impl Drop for HeldConstructionRegisters<'_> {
    #[inline(never)]
    fn drop(&mut self) {
        for slot in self.0.iter_mut().rev() {
            *slot = ConstructionRegister::Empty;
        }
    }
}

impl<'a> ConstructionReader<'a> {
    #[inline(never)]
    pub fn hold(&mut self, count: usize) -> Result<HeldConstructionRegisters<'a>, XamlLoadError> {
        let (held, rest) = std::mem::take(&mut self.0)
            .split_at_mut_checked(count)
            .ok_or_else(invalid)?;
        self.0 = rest;
        Ok(HeldConstructionRegisters(held))
    }

    fn take_one(&mut self) -> Result<ConstructionRegister, XamlLoadError> {
        let (next, rest) = std::mem::take(&mut self.0)
            .split_first_mut()
            .ok_or_else(invalid)?;
        self.0 = rest;
        Ok(std::mem::replace(next, ConstructionRegister::Empty))
    }

    #[inline(never)]
    pub fn next_object<T: ferroui_base::ObjectType>(&mut self) -> Result<Ref<T>, XamlLoadError> {
        self.take_one()?.into_object()
    }

    #[inline(never)]
    pub fn next_value<T: 'static>(&mut self) -> Result<T, XamlLoadError> {
        self.take_one()?.into_value()
    }
}

impl ConstructionProgram {
    #[inline(never)]
    pub fn results(
        &self,
        inputs: &[ConstructionInput<'_>],
        scope: Option<&Rc<dyn INameScope>>,
        context: Option<&Rc<XamlIlContext>>,
    ) -> Result<ConstructionResults, XamlLoadError> {
        self.run(inputs, scope, context).map(ConstructionResults)
    }

    #[inline(never)]
    pub fn hold(
        &self,
        inputs: &[ConstructionInput<'_>],
        scope: Option<&Rc<dyn INameScope>>,
        context: Option<&Rc<XamlIlContext>>,
    ) -> Result<ConstructionObjects, XamlLoadError> {
        self.run(inputs, scope, context).map(ConstructionObjects)
    }

    #[inline(never)]
    pub fn apply(
        &self,
        inputs: &[ConstructionInput<'_>],
        scope: Option<&Rc<dyn INameScope>>,
        context: Option<&Rc<XamlIlContext>>,
    ) -> Result<(), XamlLoadError> {
        self.hold(inputs, scope, context).map(drop)
    }

    /// External registers borrow their native bindings. Even indices name external registers
    /// and odd indices name constructed objects, in construction order.
    #[inline(never)]
    pub fn run(
        &self,
        inputs: &[ConstructionInput<'_>],
        scope: Option<&Rc<dyn INameScope>>,
        context: Option<&Rc<XamlIlContext>>,
    ) -> Result<Vec<ConstructionRegister>, XamlLoadError> {
        use ConstructionImport as Import;
        let mut code = self.code;
        let mut created = Vec::new();
        let import = |code: &mut &[u8]| {
            self.imports
                .operations
                .get(integer(code)? as usize)
                .ok_or_else(invalid)
        };
        let result = (|| {
            while let Some((&opcode, rest)) = code.split_first() {
                code = rest;
                if opcode == op::PUSH_NAMESPACE {
                    let Import::Namespaces(table) = import(&mut code)? else { return Err(invalid()) };
                    let scope = super::compiled::push_namespace_scope(context.ok_or_else(invalid)?, table());
                    created.push(ConstructionRegister::Value(Box::new(scope)));
                    continue;
                }
                if opcode == op::DROP_NAMESPACE {
                    let slot = created.get_mut(integer(&mut code)? as usize).ok_or_else(invalid)?;
                    let guard = std::mem::replace(slot, ConstructionRegister::Empty).into_value::<super::XamlStaticProviderScope>()?;
                    drop(guard);
                    continue;
                }
                if opcode == op::MOVE_TO_MARKUP {
                    let slot = integer(&mut code)? as usize;
                    let Import::MoveToMarkup(convert) = import(&mut code)? else {
                        return Err(invalid());
                    };
                    let slot = created.get_mut(slot).ok_or_else(invalid)?;
                    let value = convert(std::mem::replace(slot, ConstructionRegister::Empty))?;
                    created.push(ConstructionRegister::Value(Box::new(value)));
                    continue;
                }
                if opcode == op::NEW_BINDING {
                    let path = value::<CompiledBindingPath>(inputs, &created, &mut code)?;
                    created.push(ConstructionRegister::Value(Box::new(
                        CompiledBindingExtension::with_path(path.clone()),
                    )));
                    continue;
                }
                if opcode == op::DEFER {
                    let Import::DeferredType(result_type) = import(&mut code)? else {
                        return Err(invalid());
                    };
                    let Import::Deferred(build) = import(&mut code)? else {
                        return Err(invalid());
                    };
                    let line = integer(&mut code)? as i32;
                    let column = integer(&mut code)? as i32;
                    let deferred = super::compiled::defer(
                        result_type(),
                        context.ok_or_else(invalid)?,
                        *build,
                        line,
                        column,
                    )?;
                    created.push(ConstructionRegister::Value(Box::new(deferred)));
                    continue;
                }
                if opcode == op::BUILD_SELECTOR {
                    let Import::Selector(program) = import(&mut code)? else {
                        return Err(invalid());
                    };
                    created.push(ConstructionRegister::Value(Box::new(
                        program.build_selector()?,
                    )));
                    continue;
                }
                if opcode == op::OBJECT_LITERAL {
                    let Import::ObjectLiteral(build) = import(&mut code)? else { return Err(invalid()) };
                    let len = integer(&mut code)? as usize;
                    let (literal, rest) = code.split_at_checked(len).ok_or_else(invalid)?;
                    code = rest;
                    created.push(ConstructionRegister::Object(build(literal)?));
                    continue;
                }
                if opcode == op::NEW {
                    let Import::New(make) = import(&mut code)? else {
                        return Err(invalid());
                    };
                    created.push(ConstructionRegister::Object(make()));
                    continue;
                }
                if matches!(opcode, op::BEGIN_SETTER | op::BEGIN_PARENTED_SETTER) {
                    let Import::Property(get) = import(&mut code)? else { return Err(invalid()) };
                    let property = get();
                    let setter = if opcode == op::BEGIN_PARENTED_SETTER {
                        super::compiled::new_setter_with_parent(context.ok_or_else(invalid)?, property)
                    } else {
                        super::compiled::new_setter(property)
                    };
                    created.push(ConstructionRegister::Value(Box::new(setter)));
                    continue;
                }
                if opcode == op::POP_PARENT {
                    context.ok_or_else(invalid)?.pop_parent();
                    continue;
                }
                if matches!(opcode, op::SET_TARGET_PROPERTY | op::CLEAR_TARGET_PROPERTY) {
                    let target = if opcode == op::SET_TARGET_PROPERTY {
                        let Import::TargetProperty(make) = import(&mut code)? else {
                            return Err(invalid());
                        };
                        make()
                    } else {
                        None
                    };
                    context.ok_or_else(invalid)?.set_target_property(target);
                    continue;
                }
                if matches!(opcode, op::CALL | op::CALL_RESULT | op::BUILD_PATH) {
                    let operation = import(&mut code)?;
                    let line = integer(&mut code)? as i32;
                    let column = integer(&mut code)? as i32;
                    let length = integer(&mut code)? as usize;
                    let (encoded, rest) = code.split_at_checked(length).ok_or_else(invalid)?;
                    code = rest;
                    let mut args = ConstructionArguments {
                        encoded,
                        imports: self.imports,
                        inputs,
                        created: &created,
                        context,
                        line,
                        column,
                    };
                    let value = match (opcode, operation) {
                        (op::BUILD_PATH, Import::Path(path)) => {
                            let mut captures = Vec::new();
                            while !args.encoded.is_empty() {
                                captures
                                    .push(args.value::<ferroui_base::data::CompiledBindingPath>()?);
                            }
                            Some(ConstructionRegister::Value(Box::new(
                                path.build(scope, &captures, line, column)?,
                            )))
                        }
                        (op::CALL | op::CALL_RESULT, Import::Call(call)) => call(&mut args)?,
                        _ => return Err(invalid()),
                    };
                    if !args.encoded.is_empty() {
                        return Err(invalid());
                    }
                    if opcode != op::CALL {
                        created.push(value.ok_or_else(invalid)?);
                    }
                    continue;
                }
                if opcode == op::COMPLETE_PLAIN_SCOPE {
                    super::compiled::complete_root_name_scope(
                        None,
                        scope,
                        integer(&mut code)? as i32,
                        integer(&mut code)? as i32,
                    )?;
                    continue;
                }
                let target = object(inputs, &created, &mut code)?;
                if opcode == op::APPLY_THEME_UNIT {
                    let Import::ThemeParts(active) = import(&mut code)? else { return Err(invalid()) };
                    let unit = integer(&mut code)?;
                    let run = integer(&mut code)? as usize;
                    let dictionary = target.cast_ref::<ferroui_base::controls::ResourceDictionary>().ok_or_else(invalid)?;
                    super::compiled::apply_theme_unit(active(), unit, run, context.ok_or_else(invalid)?, dictionary)?;
                    continue;
                }
                match opcode {
                    op::FINISH_SETTER | op::FINISH_PARENTED_SETTER => {
                        let style = target.cast_ref::<ferroui_base::styling::StyleBase>().ok_or_else(invalid)?;
                        let setter = value::<Rc<ferroui_base::styling::Setter>>(inputs, &created, &mut code)?;
                        let Import::SetterValue(convert) = import(&mut code)? else { return Err(invalid()) };
                        let item = convert(register(inputs, &created, &mut code)?)?;
                        if opcode == op::FINISH_PARENTED_SETTER {
                            super::compiled::add_setter_value_with_parent(context.ok_or_else(invalid)?, style, setter, item);
                        } else {
                            super::compiled::add_setter_value(style, setter, item);
                        }
                    }
                    op::SET_REGISTER => {
                        let Import::RegisterSetter(write) = import(&mut code)? else { return Err(invalid()) };
                        let source = register(inputs, &created, &mut code)?;
                        let line = integer(&mut code)? as i32;
                        let column = integer(&mut code)? as i32;
                        write(target, source, line, column)?;
                    }
                    op::ADD_STYLE_DYNAMIC => {
                        let Import::Property(property) = import(&mut code)? else {
                            return Err(invalid());
                        };
                        let key = self.imports.text(&mut code)?;
                        let style = target
                            .cast::<ferroui_base::styling::StyleBase>()
                            .ok_or_else(invalid)?;
                        super::compiled::add_dynamic_resource_setter_with_parent(
                            context.ok_or_else(invalid)?,
                            &style,
                            property(),
                            key,
                        );
                    }
                    op::ADD_STYLE_LITERAL | op::SET_PROPERTY_LITERAL => {
                        let property = if opcode == op::ADD_STYLE_LITERAL {
                            let Import::Property(property) = import(&mut code)? else {
                                return Err(invalid());
                            };
                            Some(property())
                        } else {
                            None
                        };
                        let operation = import(&mut code)?;
                        let length = integer(&mut code)? as usize;
                        let (encoded, rest) = code.split_at_checked(length).ok_or_else(invalid)?;
                        code = rest;
                        let mut args = ConstructionArguments {
                            encoded,
                            imports: self.imports,
                            inputs,
                            created: &created,
                            context,
                            line: 0,
                            column: 0,
                        };
                        match (operation, property) {
                            (Import::LiteralValue(read), Some(property)) => {
                                let style = target
                                    .cast::<ferroui_base::styling::StyleBase>()
                                    .ok_or_else(invalid)?;
                                super::compiled::add_setter(&style, property, read(&mut args)?);
                            }
                            (Import::LiteralSetter(set), None) => set(target, &mut args)?,
                            _ => return Err(invalid()),
                        }
                        if !args.encoded.is_empty() {
                            return Err(invalid());
                        }
                    }
                    op::STATIC_RESOURCE => {
                        use super::compiled as rt;
                        use crate::markup_extensions::StaticResourceExtension;
                        let key = self.imports.text(&mut code)?;
                        let Import::Property(property) = import(&mut code)? else {
                            return Err(invalid());
                        };
                        let Import::ResourceSetter(set) = import(&mut code)? else {
                            return Err(invalid());
                        };
                        let line = integer(&mut code)? as i32;
                        let column = integer(&mut code)? as i32;
                        let extension =
                            StaticResourceExtension::with_resource_key(rt::string_object(key));
                        let provided = rt::provide_value_invoked(
                            context.ok_or_else(invalid)?,
                            rt::property_value(property()),
                            &extension,
                            StaticResourceExtension::__markup_ProvideValue_0,
                            line,
                            column,
                        )?;
                        set(target, rt::to_value(provided), line, column)?;
                        // Native construction keeps the extension and its key in scope until return.
                        created.push(ConstructionRegister::Value(Box::new(extension)));
                    }
                    op::PROVIDE_BINDING => {
                        let extension =
                            value::<Rc<CompiledBindingExtension>>(inputs, &created, &mut code)?;
                        let Import::Property(property) = import(&mut code)? else {
                            return Err(invalid());
                        };
                        let line = integer(&mut code)? as i32;
                        let column = integer(&mut code)? as i32;
                        let context = context.ok_or_else(invalid)?;
                        context.set_target_property(super::compiled::property_value(property()));
                        let provided = extension
                            .provide_value(Some(&super::compiled::service_provider(context)));
                        context.set_target_property(None);
                        super::compiled::bind(target, property(), provided, line, column)?;
                    }
                    op::COMPLETE_SCOPE => {
                        let styled = target.cast::<StyledElement>().ok_or_else(invalid)?;
                        super::compiled::complete_root_name_scope(
                            Some(&styled),
                            scope,
                            integer(&mut code)? as i32,
                            integer(&mut code)? as i32,
                        )?;
                    }
                    op::BEGIN_INIT => target
                        .cast::<StyledElement>()
                        .ok_or_else(invalid)?
                        .begin_init(),
                    op::END_INIT => {
                        let line = integer(&mut code)? as i32;
                        let column = integer(&mut code)? as i32;
                        invoked(
                            target
                                .cast::<StyledElement>()
                                .ok_or_else(invalid)?
                                .try_end_init(),
                            line,
                            column,
                        )?;
                    }
                    op::DYNAMIC_RESOURCE => {
                        let Import::Property(property) = import(&mut code)? else {
                            return Err(invalid());
                        };
                        let key = self.imports.text(&mut code)?;
                        let line = integer(&mut code)? as i32;
                        let column = integer(&mut code)? as i32;
                        bind_dynamic_resource(
                            context.ok_or_else(invalid)?,
                            target,
                            property(),
                            key,
                            line,
                            column,
                        )?;
                    }
                    op::PUSH_PARENT | op::INTERMEDIATE_ROOT => {
                        let Import::Markup(value) = import(&mut code)? else {
                            return Err(invalid());
                        };
                        let context = context.ok_or_else(invalid)?;
                        if opcode == op::PUSH_PARENT {
                            context.push_parent(value(target)?);
                        } else {
                            context.set_intermediate_root_object(value(target)?);
                        }
                    }
                    op::TEMPLATE_BIND | op::TEMPLATE_BIND_FROM => {
                        let Import::Property(first) = import(&mut code)? else {
                            return Err(invalid());
                        };
                        let Import::Property(second) = import(&mut code)? else {
                            return Err(invalid());
                        };
                        let line = integer(&mut code)? as i32;
                        let column = integer(&mut code)? as i32;
                        let first = first();
                        let second = second();
                        if opcode == op::TEMPLATE_BIND_FROM {
                            super::compiled::bind_template_binding_from(
                                target, first, second, line, column,
                            )?;
                        } else {
                            bind_template_binding(target, first, second, line, column)?;
                        }
                    }
                    op::REGISTER_NAME => {
                        let name = self.imports.text(&mut code)?;
                        let Import::BoxObject(box_object) = import(&mut code)? else {
                            return Err(invalid());
                        };
                        let line = integer(&mut code)? as i32;
                        let column = integer(&mut code)? as i32;
                        register_name_value(scope, name, box_object(target)?, line, column)?;
                    }
                    op::SET_INT => {
                        let Import::Int(set) = import(&mut code)? else {
                            return Err(invalid());
                        };
                        set(
                            target,
                            ferroui_markup::compiled_operands::read_signed(&mut code)
                                .map_err(|_| invalid())?,
                        );
                    }
                    op::SET_FOUR => {
                        let Import::Four(set) = import(&mut code)? else {
                            return Err(invalid());
                        };
                        let mut values = [0.; 4];
                        for value in &mut values {
                            *value = ferroui_markup::compiled_operands::read_number(&mut code)
                                .map_err(|_| invalid())?;
                        }
                        set(target, values);
                    }
                    op::SET_F64 => {
                        let Import::Float(set) = import(&mut code)? else {
                            return Err(invalid());
                        };
                        set(
                            target,
                            ferroui_markup::compiled_operands::read_number(&mut code)
                                .map_err(|_| invalid())?,
                        );
                    }
                    op::SET_STRING => {
                        let Import::String(set) = import(&mut code)? else {
                            return Err(invalid());
                        };
                        let text = self.imports.text(&mut code)?;
                        set(target, text);
                    }
                    op::SET_CONSTANT => {
                        let Import::Constant(set) = import(&mut code)? else {
                            return Err(invalid());
                        };
                        set(target);
                    }
                    op::SET_OBJECT => {
                        let Import::Object(set) = import(&mut code)? else {
                            return Err(invalid());
                        };
                        set(target, object(inputs, &created, &mut code)?)?;
                    }
                    _ => return Err(invalid()),
                }
            }
            Ok(())
        })();
        if let Err(error) = result {
            while created.pop().is_some() {}
            return Err(error);
        }
        Ok(created)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::{controls::NameScope, Thickness};
    use ferroui_controls::{primitives::TemplatedControl, Border, Button};

    #[test]
    fn register_setter_checks_the_source_and_preserves_the_original_position() {
        const WRITE: ConstructionProgram = ConstructionProgram {
            code: &[op::SET_REGISTER, 0, 0, 2, 9, 17],
            imports: &ConstructionImports {
                strings: &[],
                operations: &[ConstructionImport::RegisterSetter(|target, source, line, column| {
                    assert_eq!((line, column), (9, 17));
                    target.set_value(ferroui_controls::Decorator::padding_property(), *source.value::<Thickness>()?);
                    Ok(())
                })],
            },
        };
        let border = Border::new();
        let target = ConstructionInput::Object(border.upcast_ref());
        let padding = Thickness::uniform(3.0);
        WRITE.apply(&[target, ConstructionInput::Value(&padding)], None, None).unwrap();
        assert_eq!(border.padding(), padding);
        assert!(WRITE.apply(&[target, target], None, None).is_err());
        assert!(WRITE.apply(&[target, ConstructionInput::Value(&3_i32)], None, None).is_err());
        assert!(WRITE.apply(&[target], None, None).is_err());
        assert_eq!(border.padding(), padding);
    }

    #[test]
    fn selected_theme_runs_observe_lexical_namespaces_and_restore_them_on_error() {
        use super::super::compiled::{create_context, ThemeUnit};
        use super::super::IFerroXamlIlXmlNamespaceInfoProvider;
        use ferroui_base::controls::ResourceDictionary;
        use ferroui_base::metadata::IServiceProvider;
        fn scoped(context: &Rc<XamlIlContext>) -> bool {
            let provider: Rc<dyn IServiceProvider> = context.clone();
            provider.get_service_of::<Rc<dyn IFerroXamlIlXmlNamespaceInfoProvider>>()
                .unwrap().xml_namespaces().contains_key("unit")
        }
        fn apply(context: &Rc<XamlIlContext>, dictionary: &Ref<ResourceDictionary>) -> Result<(), XamlLoadError> {
            assert!(scoped(context));
            dictionary.add("selected", super::super::compiled::to_value(1_i32));
            Ok(())
        }
        static SELECTED: ThemeUnit = ThemeUnit { index: 7, name: "selected", runs: &[apply] };
        static PARTS: super::super::compiled::ThemeParts = &[&SELECTED];
        const IMPORTS: ConstructionImports = ConstructionImports {
            strings: &[],
            operations: &[
                ConstructionImport::Namespaces(|| &[("unit", &[("Lexical.Views", "App", &[])])]),
                ConstructionImport::ThemeParts(|| PARTS),
                ConstructionImport::Call(|_| Err(XamlLoadError::with_message("failed after registration"))),
            ],
        };
        const SUCCESS: &[u8] = &[
            op::PUSH_NAMESPACE, 0, op::APPLY_THEME_UNIT, 0, 1, 4, 0,
            op::APPLY_THEME_UNIT, 0, 1, 7, 0, op::DROP_NAMESPACE, 0,
        ];
        const FAILURE: &[u8] = &[
            op::PUSH_NAMESPACE, 0, op::APPLY_THEME_UNIT, 0, 1, 7, 0,
            op::CALL, 2, 0, 0, 0, op::DROP_NAMESPACE, 0,
        ];
        for (code, succeeds) in [(SUCCESS, true), (FAILURE, false)] {
            let context = create_context(None, None, &[]);
            let dictionary = ResourceDictionary::new();
            let program = ConstructionProgram { imports: &IMPORTS, code };
            assert_eq!(program.apply(&[ConstructionInput::Object(dictionary.upcast_ref())], None, Some(&context)).is_ok(), succeeds);
            assert_eq!(dictionary.count(), 1);
            assert!(!scoped(&context));
        }
    }

    #[test]
    fn converting_a_last_use_register_transfers_its_owned_value() {
        use ferroui_base::metadata::{from_markup_value, MarkupValue};
        let program = ConstructionProgram {
            code: &[op::CALL_RESULT, 0, 0, 0, 0, op::MOVE_TO_MARKUP, 0, 1],
            imports: &ConstructionImports {
                strings: &[],
                operations: &[
                    ConstructionImport::Call(|_| Ok(Some(ConstructionRegister::Value(Box::new(Rc::new(47_i32)))))),
                    ConstructionImport::MoveToMarkup(|register| {
                        let owned = register.into_value::<Rc<i32>>()?;
                        assert_eq!(Rc::strong_count(&owned), 1);
                        Ok(super::super::compiled::to_value(owned))
                    }),
                ],
            },
        };
        let mut registers = program.run(&[], None, None).unwrap();
        let result = registers.pop().unwrap().into_value::<MarkupValue>().unwrap();
        assert_eq!(*from_markup_value::<Rc<i32>>(&result).unwrap(), 47);
        assert!(matches!(registers.as_slice(), [ConstructionRegister::Empty]));
    }

    #[test]
    fn extension_target_clears_after_success_and_remains_set_on_failure() {
        const IMPORTS: ConstructionImports = ConstructionImports {
            strings: &[],
            operations: &[
                ConstructionImport::TargetProperty(|| super::super::compiled::to_value(47_i32)),
                ConstructionImport::Call(|args| {
                    let target = args.context()?.target_property().unwrap();
                    assert_eq!(target.downcast_ref::<i32>(), Some(&47));
                    Ok(None)
                }),
                ConstructionImport::Call(|_| Err(XamlLoadError::with_message("extension failed"))),
            ],
        };
        const fn scope(callback: u8) -> [u8; 8] {
            [op::SET_TARGET_PROPERTY, 0, op::CALL, callback, 0, 0, 0, op::CLEAR_TARGET_PROPERTY]
        }
        for (code, succeeds) in [(&const { scope(1) }, true), (&const { scope(2) }, false)] {
            let context = super::super::compiled::create_context(None, None, &[]);
            let program = ConstructionProgram { imports: &IMPORTS, code };
            assert_eq!(program.apply(&[], None, Some(&context)).is_ok(), succeeds);
            assert_eq!(context.target_property().is_none(), succeeds);
        }
    }

    #[test]
    fn default_template_binding_choice_evaluates_source_first_and_stays_live() {
        use ferroui_base::metadata::{from_markup_value, MarkupValue};
        use ferroui_controls::{presenters::ContentPresenter, ContentControl};
        ferroui_base::ferro_thread_local! {
            static READS: std::cell::RefCell<Vec<&'static str>> = std::cell::RefCell::new(Vec::new());
        }
        static BIND: ConstructionProgram = ConstructionProgram {
            code: &[op::TEMPLATE_BIND_FROM, 0, 0, 1, 14, 9],
            imports: &ConstructionImports {
                strings: &[],
                operations: &[
                    ConstructionImport::Property(|| {
                        READS.with(|r| r.borrow_mut().push("source"));
                        ContentControl::content_property()
                    }),
                    ConstructionImport::Property(|| {
                        READS.with(|r| r.borrow_mut().push("target"));
                        ContentPresenter::content_property()
                    }),
                ],
            },
        };
        crate::register_types();
        let parent = Button::new();
        let presenter = ContentPresenter::new();
        BIND.apply(
            &[ConstructionInput::Object(
                presenter.upcast_ref::<FerroObject>(),
            )],
            None,
            None,
        )
        .unwrap();
        presenter.set_templated_parent(Some(parent.upcast_ref::<FerroObject>().clone()));
        let text = |v: &str| -> MarkupValue { Some(Rc::new(v.to_owned())) };
        let read = || from_markup_value::<String>(&presenter.content()).unwrap();
        parent.set_content(text("first"));
        assert_eq!(read(), "first");
        presenter.set_content(text("local"));
        parent.set_content(text("second"));
        assert_eq!(read(), "local");
        presenter.clear_value(ContentPresenter::content_property());
        assert_eq!(read(), "second");
        parent.set_content(None);
        assert!(presenter.content().is_none());
        READS.with(|r| assert_eq!(*r.borrow(), ["source", "target"]));
    }

    #[test]
    fn result_ranges_drop_in_reverse_order_around_a_native_local() {
        struct Tracked(usize, Rc<std::cell::RefCell<Vec<usize>>>);
        impl Drop for Tracked {
            fn drop(&mut self) {
                self.1.borrow_mut().push(self.0);
            }
        }
        let order = Rc::new(std::cell::RefCell::new(Vec::new()));
        let mut frame = ConstructionResults(
            (0..6)
                .map(|i| ConstructionRegister::Value(Box::new(Tracked(i, order.clone()))))
                .collect(),
        );
        {
            let mut reader = frame.reader();
            let (_first, _native, _last) = (
                reader.hold(2).unwrap(),
                reader.next_value::<Tracked>().unwrap(),
                reader.hold(3).unwrap(),
            );
            assert!(order.borrow().is_empty());
        }
        assert_eq!(order.borrow().as_slice(), &[5, 4, 3, 2, 1, 0]);
        drop(frame);
        assert_eq!(order.borrow().len(), 6);
    }

    #[test]
    fn constructed_template_keeps_named_identity_and_live_property_binding() {
        crate::register_types();
        static PROGRAM: ConstructionProgram = ConstructionProgram {
            code: &[
                op::NEW,
                0,
                op::BEGIN_INIT,
                1,
                op::REGISTER_NAME,
                1,
                0,
                1,
                18,
                7,
                op::TEMPLATE_BIND,
                1,
                2,
                3,
                19,
                7,
                op::END_INIT,
                1,
                18,
                2,
            ],
            imports: &ConstructionImports {
                strings: &["PART_Border"],
                operations: &[
                    ConstructionImport::New(|| Border::new().upcast()),
                    ConstructionImport::BoxObject(|v| {
                        Ok(Rc::new(construction_object::<Border>(Some(v.clone()))?))
                    }),
                    ConstructionImport::Property(|| {
                        ferroui_controls::Decorator::padding_property()
                    }),
                    ConstructionImport::Property(|| TemplatedControl::padding_property()),
                ],
            },
        };
        let scope: Rc<dyn INameScope> = Rc::new(NameScope::new());
        let mut built = PROGRAM.run(&[], Some(&scope), None).unwrap();
        let border = built.pop().unwrap().into_object::<Border>().unwrap();
        let registered = scope.find("PART_Border").unwrap();
        assert!(registered.ptr_eq(border.upcast_ref::<FerroObject>()));
        let button = Button::new();
        border.set_templated_parent(Some(button.upcast_ref::<FerroObject>().clone()));
        for padding in [3., 17.] {
            button.set_padding(Thickness::uniform(padding));
            assert_eq!(border.padding(), Thickness::uniform(padding));
        }
        let duplicate = PROGRAM.run(&[], Some(&scope), None).err().unwrap();
        assert!(
            duplicate.message().contains("line 18 position 7"),
            "{duplicate}"
        );
    }

    #[test]
    fn selector_instruction_produces_an_exact_value_and_checks_the_import() {
        use super::super::compiled::{SelectorImports, TypedSelectorProgram};
        use ferroui_base::styling::Selector;
        use ferroui_markup::style_opcodes::{self as style, selector as s};
        static SELECTOR: TypedSelectorProgram = TypedSelectorProgram::new(
            &[style::VERSION, s::CLASS, 0, s::END],
            SelectorImports {
                strings: &["selected"],
                ..SelectorImports::EMPTY
            },
        );
        static GOOD: ConstructionProgram = ConstructionProgram {
            code: &[op::BUILD_SELECTOR, 0],
            imports: &ConstructionImports {
                operations: &[ConstructionImport::Selector(&SELECTOR)],
                ..ConstructionImports::EMPTY
            },
        };
        let selector = GOOD
            .run(&[], None, None)
            .unwrap()
            .pop()
            .unwrap()
            .into_value::<Selector>()
            .unwrap();
        assert_eq!(selector.to_string(), ".selected");
        for code in [
            &[op::BUILD_SELECTOR][..],
            &[op::BUILD_SELECTOR, 0],
            &[op::BUILD_SELECTOR, 1],
        ] {
            let invalid = ConstructionProgram {
                code,
                imports: &ConstructionImports {
                    operations: &[ConstructionImport::Float(|_, _| {})],
                    ..ConstructionImports::EMPTY
                },
            };
            assert!(invalid.run(&[], None, None).is_err());
        }
    }

    #[test]
    fn reports_invalid_operands_without_reading_past_program_or_imports() {
        let root = Button::new();
        for code in [
            &[op::NEW][..],
            &[op::NEW, 255, 255, 255, 255, 16],
            &[op::BEGIN_INIT, 1],
            &[op::NEW, 0],
            &[op::SET_F64, 0, 1, 255, 5],
            &[op::BEGIN_SETTER],
            &[op::BEGIN_PARENTED_SETTER, 0],
            &[op::FINISH_SETTER, 0, 1],
            &[op::FINISH_PARENTED_SETTER, 1],
            &[255, 0],
        ] {
            // Tests use static images like the generated modules do.
            let program = ConstructionProgram {
                code,
                imports: &ConstructionImports {
                    operations: &[
                        ConstructionImport::Int(|_, _| {}),
                        ConstructionImport::Float(|_, _| {}),
                    ],
                    ..ConstructionImports::EMPTY
                },
            };
            let result = program.run(
                &[ConstructionInput::Object(root.upcast_ref::<FerroObject>())],
                None,
                None,
            );
            assert!(result.is_err(), "{code:?}");
        }
    }
}
