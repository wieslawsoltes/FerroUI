//! Resolved ControlTheme programs. Each factory only links its own compile-time imports.
//! Validate the stream once before creating a theme. The interpreter holds a style stack and
//! a selector stack for nested Not/Or expressions. It never retains a theme or build context.
use super::*;
use ferroui_markup::{
    compiled_operands as wire,
    style_opcodes::{self as op, member as m, selector as s, value as v},
};
use std::sync::OnceLock;
use wire::ReadResult;

#[path = "selector_program.rs"]
mod selectors;
pub use selectors::{selector_operand, SelectorImports, SelectorProperty, TypedSelectorProgram};
use selectors::SelectorReader;

/// Local imports of a single control theme. Functions use their original typed ABI.
pub struct TypedStyleImports {
    pub strings: &'static [&'static str],
    pub types: &'static [&'static TypeInfo],
    pub identities: &'static [&'static TypeIdentity],
    pub properties: &'static [PropertyFn],
    pub selector_values: &'static [fn() -> BoxedValue],
    pub templates: &'static [DeferredBuild],
    pub expressions: &'static [fn() -> MarkupValue],
    pub computed: &'static [ThemeValueFn],
}

impl TypedStyleImports {
    pub const EMPTY: Self = Self {
        strings: &[],
        types: &[],
        identities: &[],
        properties: &[],
        selector_values: &[],
        templates: &[],
        expressions: &[],
        computed: &[],
    };
}

pub struct TypedStyleProgram {
    code: &'static [u8],
    imports: TypedStyleImports,
    valid: OnceLock<ReadResult<()>>,
}

enum Instruction {
    Target(&'static TypeInfo),
    Base(ThemeKey, (i32, i32)),
    Set(ThemeSetter),
    Style { parent: bool, selector: bool },
    EndStyle((i32, i32)),
}

struct Reader<'a> {
    bytes: &'a [u8],
    imports: &'a TypedStyleImports,
    position: (i32, i32),
}

fn import<T: Copy>(bytes: &mut &[u8], pool: &[T]) -> ReadResult<T> {
    pool.get(wire::read_unsigned(bytes)? as usize).copied().ok_or("unknown import")
}

impl Reader<'_> {
    fn byte(&mut self) -> ReadResult<u8> {
        Ok(wire::take::<1>(&mut self.bytes)?[0])
    }

    fn position(&mut self) -> ReadResult<(i32, i32)> {
        self.position.0 = self.position.0.wrapping_add(wire::read_signed(&mut self.bytes)?);
        self.position.1 = self.position.1.wrapping_add(wire::read_signed(&mut self.bytes)?);
        Ok(self.position)
    }

    fn value(&mut self) -> ReadResult<ThemeValue> {
        let op = self.byte()?;
        Ok(match op {
            v::DYNAMIC => ThemeValue::Dynamic(import(&mut self.bytes, self.imports.strings)?),
            v::STATIC => {
                let key = import(&mut self.bytes, self.imports.strings)?;
                let (line, position) = self.position()?;
                ThemeValue::Static { key, line, position }
            }
            v::TEMPLATE => {
                let flags = self.byte()?;
                if flags > 3 {
                    return Err("unknown template flags");
                }
                let target_type = if flags & 2 != 0 { Some(import(&mut self.bytes, self.imports.types)?) } else { None };
                let content_type = import(&mut self.bytes, self.imports.types)?;
                let build = import(&mut self.bytes, self.imports.templates)?;
                let (line, position) = self.position()?;
                ThemeValue::Template(ThemeTemplate { target_type, content_type, build, parent: flags & 1 != 0, line, position })
            }
            v::NULL => ThemeValue::Null,
            v::TRUE | v::FALSE => ThemeValue::Bool(op == v::TRUE),
            v::INT => ThemeValue::Int(wire::read_signed(&mut self.bytes)?),
            v::DOUBLE => ThemeValue::Double(wire::read_number(&mut self.bytes)?),
            v::TEXT => ThemeValue::Text(import(&mut self.bytes, self.imports.strings)?),
            v::THICKNESS | v::CORNER => {
                let mut n = [0.0; 4];
                for component in &mut n {
                    *component = wire::read_number(&mut self.bytes)?;
                }
                if op == v::THICKNESS {
                    ThemeValue::Thickness(n)
                } else {
                    ThemeValue::CornerRadius(n)
                }
            }
            v::EXPRESSION => ThemeValue::Expression(import(&mut self.bytes, self.imports.expressions)?),
            v::COMPUTED => ThemeValue::Computed(import(&mut self.bytes, self.imports.computed)?),
            v::ABSENT => ThemeValue::Absent,
            _ => return Err("unknown setter value"),
        })
    }

    fn next(&mut self) -> ReadResult<Instruction> {
        let op = self.byte()?;
        Ok(match op {
            m::TARGET => Instruction::Target(import(&mut self.bytes, self.imports.types)?),
            m::BASE_TEXT | m::BASE_TYPE => {
                let key = if op == m::BASE_TEXT {
                    ThemeKey::Text(import(&mut self.bytes, self.imports.strings)?)
                } else {
                    ThemeKey::Type(import(&mut self.bytes, self.imports.types)?)
                };
                Instruction::Base(key, self.position()?)
            }
            m::SET | m::SET_CREATED | m::SET_PARENTED => {
                let form = match op {
                    m::SET => SetterForm::Plain,
                    m::SET_CREATED => SetterForm::Created,
                    _ => SetterForm::Parented,
                };
                let property = import(&mut self.bytes, self.imports.properties)?;
                Instruction::Set(ThemeSetter { form, property, value: self.value()? })
            }
            m::STYLE | m::STYLE_PARENTED => {
                let selector = match self.byte()? {
                    0 => false,
                    1 => true,
                    _ => return Err("unknown style flags"),
                };
                Instruction::Style { parent: op == m::STYLE_PARENTED, selector }
            }
            m::END_STYLE => Instruction::EndStyle(self.position()?),
            _ => return Err("unknown style member"),
        })
    }
}

impl TypedStyleProgram {
    pub const fn new(code: &'static [u8], imports: TypedStyleImports) -> Self {
        Self { code, imports, valid: OnceLock::new() }
    }

    fn selectors<'b, 'a>(&self, bytes: &'b mut &'a [u8]) -> SelectorReader<'b, 'a> {
        SelectorReader {
            bytes,
            imports: SelectorImports {
                strings: self.imports.strings, identities: self.imports.identities,
                properties: self.imports.properties, values: self.imports.selector_values,
            },
        }
    }

    fn reader(&self) -> ReadResult<(bool, Reader<'_>)> {
        let [op::VERSION, parent @ 0..=1, bytes @ ..] = self.code else {
            return Err("invalid style program header");
        };
        Ok((*parent != 0, Reader { bytes, imports: &self.imports, position: (0, 0) }))
    }

    fn validate(&self) -> ReadResult<()> {
        let (_, mut r) = self.reader()?;
        let mut styles: usize = 0;
        while !r.bytes.is_empty() {
            match r.next()? {
                Instruction::Style { selector, .. } => {
                    if selector && self.selectors(&mut r.bytes).validate()? != 0 {
                        return Err("lexical selector operand in a closed control theme");
                    }
                    styles += 1;
                }
                Instruction::EndStyle(_) => styles = styles.checked_sub(1).ok_or("unmatched style end")?,
                Instruction::Target(_) | Instruction::Base(..) if styles != 0 => return Err("theme member in a nested style"),
                _ => {}
            }
        }
        if styles != 0 {
            return Err("unterminated style or selector");
        }
        Ok(())
    }

    /// Builds a fresh theme with its declaration context, maintaining the source statement order.
    #[inline(never)]
    pub fn build(&self, context: &Rc<XamlIlContext>) -> Result<MarkupValue, XamlLoadError> {
        fn invalid(reason: &'static str) -> XamlLoadError {
            XamlLoadError::with_message(format!("Invalid compiled XAML control theme: {reason}"))
        }
        self.valid.get_or_init(|| self.validate()).map_err(invalid)?;
        let (parent, mut r) = self.reader().map_err(invalid)?;
        let theme = ControlTheme::new();
        if parent {
            context.push_parent(to_value(theme.clone()));
        }
        let mut styles: Vec<(Ref<Style>, bool)> = Vec::new();
        while !r.bytes.is_empty() {
            let style = styles.last().map_or_else(|| theme.upcast_ref::<StyleBase>(), |(s, _)| s.upcast_ref::<StyleBase>());
            match r.next().map_err(invalid)? {
                Instruction::Target(t) => ControlTheme::__markup_set_TargetType(&theme, Some(t)),
                Instruction::Base(key, (l, p)) => based_on(context, &theme, key, l, p)?,
                Instruction::Set(setter) => add_theme_setter(context, style, &setter)?,
                Instruction::Style { parent, selector } => {
                    let nested = Style::new();
                    if parent {
                        context.push_parent(to_value(nested.clone()));
                    }
                    if selector {
                        let selector = self.selectors(&mut r.bytes).build(&mut |_, _| Err(invalid("unexpected lexical selector operand")))?;
                        Style::__markup_set_Selector(&nested, Some(selector));
                        nested.enable_selector_index();
                    }
                    styles.push((nested, parent));
                }
                Instruction::EndStyle((l, p)) => {
                    let (nested, parent) = styles.pop().expect("validated nested style");
                    if parent {
                        context.pop_parent();
                    }
                    let owner =
                        styles.last().map_or_else(|| theme.upcast_ref::<StyleBase>(), |(s, _)| s.upcast_ref::<StyleBase>());
                    StyleBase::__markup_Add_1(owner, cast(nested, l, p)?);
                }
            }
        }
        if parent {
            context.pop_parent();
        }
        Ok(to_value(theme))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_constants_and_absent_setter_without_type_conversion() {
        let mut bytes = vec![v::NULL, v::FALSE, v::TRUE, v::INT, 129, 2, v::DOUBLE, 255];
        bytes.extend((-0.0_f64).to_le_bytes());
        bytes.extend([v::TEXT, 0, v::THICKNESS, 0, 32, 33, 254, v::CORNER, 32, 34, 35, 36, v::ABSENT]);
        let imports = TypedStyleImports { strings: &["word"], ..TypedStyleImports::EMPTY };
        let mut reader = Reader { bytes: &bytes, imports: &imports, position: (0, 0) };
        assert!(matches!(reader.value(), Ok(ThemeValue::Null)));
        assert!(matches!(reader.value(), Ok(ThemeValue::Bool(false))));
        assert!(matches!(reader.value(), Ok(ThemeValue::Bool(true))));
        assert!(matches!(reader.value(), Ok(ThemeValue::Int(-129))));
        assert!(matches!(reader.value(), Ok(ThemeValue::Double(value)) if value.to_bits() == (-0.0_f64).to_bits()));
        assert!(matches!(reader.value(), Ok(ThemeValue::Text("word"))));
        assert!(matches!(reader.value(), Ok(ThemeValue::Thickness([-32.0, 0.0, 1.0, 222.0]))));
        assert!(matches!(reader.value(), Ok(ThemeValue::CornerRadius([0.0, 2.0, 3.0, 4.0]))));
        assert!(matches!(reader.value(), Ok(ThemeValue::Absent)));
        assert!(reader.bytes.is_empty());
    }
}
