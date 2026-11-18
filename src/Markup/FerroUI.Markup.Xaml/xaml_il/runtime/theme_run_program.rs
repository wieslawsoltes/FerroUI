//! The instructions and imports of a single extracted dictionary run. Activation uses
//! the caller's dictionary and context, including any lexical namespace scope. Deferred
//! factory imports are registered in declaration order and run only on resource lookup.
use super::{compiled as rt, XamlIlContext};
use crate::XamlLoadError;
use ferroui_base::controls::{IDeferredContent, IResourceDictionary, ResourceDictionary};
use ferroui_base::data::core::ValueType;
use ferroui_base::metadata::MarkupValue;
use ferroui_base::{Ref, TypeInfo};
use ferroui_markup::{compiled_operands as wire, theme_run_opcodes as op};
use std::rc::Rc;

type Result<T> = std::result::Result<T, XamlLoadError>;

pub trait ScheduledRows: Sync {
    fn apply(
        &'static self,
        opcode: u8,
        dictionary: &Ref<ResourceDictionary>,
        context: &Rc<XamlIlContext>,
        document: &'static rt::DocumentInfo,
    ) -> Result<()>;
}

// A retained row program references just the implementation for its concrete row type.
macro_rules! row_methods {
    ($ty:ty, $( $tag:path => $method:ident $(($ctx:ident, $doc:ident))? ),+) => {
        impl ScheduledRows for rt::TypedResourceProgram<$ty> {
            #[allow(unused_variables)]
            fn apply(&'static self, opcode: u8, dictionary: &Ref<ResourceDictionary>, context: &Rc<XamlIlContext>, document: &'static rt::DocumentInfo) -> Result<()> {
                match opcode {
                    $( $tag => { $(let ($ctx, $doc) = (context, document);)? self.$method(dictionary $(, $ctx, $doc)?) }, )+
                    _ => Err(invalid()),
                }
            }
        }
    }
}
row_methods!(rt::ResourceAliasRow, op::ALIAS => add_resource_aliases(c, d));
row_methods!(rt::DynamicBrushRow, op::DYNAMIC_BRUSH => add_dynamic_brushes(c, d));
row_methods!(rt::StaticBrushRow, op::STATIC_BRUSH => add_static_brushes(c, d));
row_methods!(rt::LiteralBrushRow, op::LITERAL_BRUSH => add_literal_brushes(c, d));
row_methods!(rt::ValueRow<f64>, op::DOUBLE => add_double_resources);
row_methods!(rt::ValueRow<[f64; 4]>, op::THICKNESS => add_thickness_resources, op::CORNER_RADIUS => add_corner_radius_resources);
row_methods!(rt::ValueRow<u32>, op::COLOR => add_color_resources);
row_methods!(rt::ValueRow<&'static str>, op::TEXT => add_string_resources);

pub enum ThemeRunImport {
    Rows(&'static dyn ScheduledRows),
    Text(&'static str),
    Type(&'static TypeInfo),
    Factory(rt::DeferredBuild),
    Constant(fn() -> MarkupValue),
    Parse(fn(&str, i32, i32) -> Result<MarkupValue>),
    FromDouble(fn(f64) -> MarkupValue),
    FromI64(fn(i64) -> MarkupValue),
}

#[cold]
fn invalid() -> XamlLoadError {
    XamlLoadError::with_message("Invalid compiled XAML dictionary run or import")
}

#[inline(never)]
pub fn apply_theme_run(
    mut code: &[u8],
    imports: &[ThemeRunImport],
    document: &'static rt::DocumentInfo,
    context: &Rc<XamlIlContext>,
    dictionary: &Ref<ResourceDictionary>,
) -> Result<()> {
    if wire::take(&mut code) != Ok([op::VERSION]) {
        return Err(invalid());
    }
    macro_rules! operand {
        ($variant:ident) => {{
            let index = wire::read_unsigned(&mut code).map_err(|_| invalid())? as usize;
            match imports.get(index) {
                Some(ThemeRunImport::$variant(value)) => *value,
                _ => return Err(invalid()),
            }
        }};
    }
    let pos = |bytes: &mut &[u8]| wire::read_signed(bytes).map_err(|_| invalid());
    while !code.is_empty() {
        let [opcode] = wire::take(&mut code).map_err(|_| invalid())?;
        match opcode {
            op::ALIAS..=op::TEXT => operand!(Rows).apply(opcode, dictionary, context, document)?,
            op::DEFER_TEXT => {
                let key = operand!(Text);
                let factory = operand!(Factory);
                rt::add_deferred_resource(
                    dictionary,
                    key,
                    ValueType::object(),
                    context,
                    factory,
                    pos(&mut code)?,
                    pos(&mut code)?,
                )?;
            }
            op::DEFER_TYPE => {
                let key = operand!(Type);
                let factory = operand!(Factory);
                let declaration = (pos(&mut code)?, pos(&mut code)?);
                let insertion = (pos(&mut code)?, pos(&mut code)?);
                let deferred = rt::defer(
                    ValueType::object(),
                    context,
                    factory,
                    declaration.0,
                    declaration.1,
                )?;
                let result = ResourceDictionary::__markup_AddDeferred_1(
                    dictionary,
                    rt::boxed(key),
                    deferred as Rc<dyn IDeferredContent>,
                );
                rt::invoked(result, insertion.0, insertion.1)?;
            }
            op::CONSTANT | op::BOOL | op::INT | op::PARSE | op::FROM_DOUBLE | op::FROM_I64 => {
                let key = operand!(Text);
                let line = pos(&mut code)?;
                let column = pos(&mut code)?;
                // In the source these locals are evaluated before the interface cast.
                let prepared = match opcode {
                    op::PARSE => Some(operand!(Parse)(
                        operand!(Text),
                        pos(&mut code)?,
                        pos(&mut code)?,
                    )?),
                    op::FROM_I64 => Some(operand!(FromI64)(i64::from_le_bytes(
                        wire::take(&mut code).map_err(|_| invalid())?,
                    ))),
                    _ => None,
                };
                let target: Rc<dyn IResourceDictionary> =
                    rt::cast(dictionary.clone(), line, column)?;
                let key = rt::string_object(key);
                let value = match opcode {
                    op::CONSTANT => operand!(Constant)(),
                    op::INT => rt::to_object(pos(&mut code)?),
                    op::BOOL => match wire::take(&mut code) {
                        Ok([0]) => rt::to_object(false),
                        Ok([1]) => rt::to_object(true),
                        _ => return Err(invalid()),
                    },
                    op::FROM_DOUBLE => {
                        operand!(FromDouble)(wire::read_number(&mut code).map_err(|_| invalid())?)
                    }
                    _ => prepared.ok_or_else(invalid)?,
                };
                rt::invoked(
                    <dyn IResourceDictionary>::__markup_Add_0(&target, key, value),
                    line,
                    column,
                )?;
            }
            _ => return Err(invalid()),
        }
    }
    Ok(())
}
