//! The same validated selector stream can be embedded in a ControlTheme or executed on its own.
use super::*;

#[derive(Clone, Copy)]
pub struct SelectorImports {
    pub strings: &'static [&'static str],
    pub identities: &'static [&'static TypeIdentity],
    pub properties: &'static [PropertyFn],
    pub values: &'static [fn() -> BoxedValue],
}

impl SelectorImports {
    pub const EMPTY: Self = Self {
        strings: &[],
        identities: &[],
        properties: &[],
        values: &[],
    };
}

/// Witness for a registered property's Rust value type when its selector value is boxed.
pub trait SelectorProperty {
    type Value;
}

impl<T: ferroui_base::PropertyValue> SelectorProperty for ferroui_base::StyledProperty<T> {
    type Value = T;
}

impl<T: ferroui_base::PropertyValue> SelectorProperty for ferroui_base::AttachedProperty<T> {
    type Value = T;
}

impl<O: ferroui_base::ObjectType, T: ferroui_base::PropertyValue> SelectorProperty for ferroui_base::DirectProperty<O, T> {
    type Value = T;
}

impl<T: ferroui_base::PropertyValue> SelectorProperty for ferroui_base::DirectPropertyBase<T> {
    type Value = T;
}

#[inline]
pub fn selector_operand<P: SelectorProperty>(_: fn() -> &'static P, value: P::Value) -> P::Value {
    value
}

pub struct TypedSelectorProgram {
    code: &'static [u8],
    imports: SelectorImports,
    valid: OnceLock<ReadResult<usize>>,
}

impl TypedSelectorProgram {
    pub const fn new(code: &'static [u8], imports: SelectorImports) -> Self {
        Self {
            code,
            imports,
            valid: OnceLock::new(),
        }
    }

    pub fn build_selector(&self) -> Result<Selector, XamlLoadError> {
        self.build_selector_with(0, &mut |_, _| Err(invalid("missing selector conversion")))
    }

    /// The generated closure evaluates only the indicated conversion, in its original lexical
    /// scope. It calls the property accessor between the setup statements and the value expression.
    #[inline(never)]
    pub fn build_selector_with(&self, slots: usize, convert: &mut SelectorConversion<'_>) -> Result<Selector, XamlLoadError> {
        let [op::VERSION, bytes @ ..] = self.code else {
            return Err(invalid("invalid selector program header"));
        };
        let expected = self
            .valid
            .get_or_init(|| {
                let mut remaining = bytes;
                let count = SelectorReader {
                    bytes: &mut remaining,
                    imports: self.imports,
                }
                .validate()?;
                if remaining.is_empty() {
                    Ok(count)
                } else {
                    Err("data after selector end")
                }
            })
            .map_err(invalid)?;
        if slots != expected {
            return Err(invalid("wrong number of selector conversions"));
        }
        SelectorReader {
            bytes: &mut &*bytes,
            imports: self.imports,
        }
        .build(convert)
    }
}

type SelectorConversion<'a> = dyn FnMut(usize, PropertyFn) -> Result<(&'static FerroProperty, BoxedValue), XamlLoadError> + 'a;

fn invalid(reason: &'static str) -> XamlLoadError {
    XamlLoadError::with_message(format!("Invalid compiled XAML selector: {reason}"))
}

enum SelectorOp {
    Select(SelectorStep),
    CapturedEquals(PropertyFn, usize),
    Push,
    Not,
    Or(usize),
    EndSelector,
}

pub(super) struct SelectorReader<'b, 'a> {
    pub(super) bytes: &'b mut &'a [u8],
    pub(super) imports: SelectorImports,
}

impl SelectorReader<'_, '_> {
    fn next(&mut self) -> ReadResult<SelectorOp> {
        let op = wire::take::<1>(self.bytes)?[0];
        let step = match op {
            s::END => {
                return Ok(SelectorOp::EndSelector);
            }
            s::PUSH => return Ok(SelectorOp::Push),
            s::NOT => return Ok(SelectorOp::Not),
            s::OR => return Ok(SelectorOp::Or(wire::read_unsigned(self.bytes)? as usize)),
            s::NESTING => SelectorStep::Nesting,
            s::CLASS => SelectorStep::Class(import(self.bytes, self.imports.strings)?),
            s::NAME => SelectorStep::Name(import(self.bytes, self.imports.strings)?),
            s::TEMPLATE => SelectorStep::Template,
            s::CHILD => SelectorStep::Child,
            s::DESCENDANT => SelectorStep::Descendant,
            s::OF_TYPE => SelectorStep::OfType(import(self.bytes, self.imports.identities)?),
            s::IS => SelectorStep::Is(import(self.bytes, self.imports.identities)?),
            s::NTH | s::NTH_LAST => {
                let step = wire::read_signed(self.bytes)?;
                let offset = wire::read_signed(self.bytes)?;
                if op == s::NTH {
                    SelectorStep::NthChild(step, offset)
                } else {
                    SelectorStep::NthLastChild(step, offset)
                }
            }
            s::CAPTURED_EQUALS => {
                return Ok(SelectorOp::CapturedEquals(
                    import(self.bytes, self.imports.properties)?,
                    wire::read_unsigned(self.bytes)? as usize,
                ))
            }
            s::EQUALS => SelectorStep::PropertyEquals {
                property: import(self.bytes, self.imports.properties)?,
                value: import(self.bytes, self.imports.values)?,
            },
            _ => return Err("unknown selector step"),
        };
        Ok(SelectorOp::Select(step))
    }

    pub(super) fn validate(&mut self) -> ReadResult<usize> {
        let mut captures = 0;
        let mut selectors = Vec::new();
        let mut has_selector = false;
        loop {
            match self.next()? {
                SelectorOp::Select(SelectorStep::Child | SelectorStep::Descendant | SelectorStep::Template) if !has_selector => {
                    return Err("selector combinator without a previous selector")
                }
                SelectorOp::Select(SelectorStep::Class(name) | SelectorStep::Name(name)) if name.trim().is_empty() => {
                    return Err("empty selector name")
                }
                SelectorOp::CapturedEquals(_, slot) => {
                    if slot != captures {
                        return Err("invalid selector conversion slot");
                    }
                    captures += 1;
                    has_selector = true;
                }
                SelectorOp::Select(_) => has_selector = true,
                SelectorOp::Push => selectors.push(std::mem::take(&mut has_selector)),
                SelectorOp::Not => {
                    if !has_selector || selectors.pop().is_none() {
                        return Err("missing negated selector");
                    }
                }
                SelectorOp::Or(n) => {
                    let start = selectors.len().checked_sub(n).ok_or("missing alternative selector")?;
                    if n < 2 || !has_selector || selectors[start + 1..].contains(&false) {
                        return Err("an or selector needs at least two nonempty alternatives");
                    }
                    selectors.truncate(start);
                }
                SelectorOp::EndSelector => {
                    if !selectors.is_empty() || !has_selector {
                        return Err("incomplete selector expression");
                    }
                    return Ok(captures);
                }
            }
        }
    }

    /// The validation pass checks the complete stream before any property or value import runs.
    pub(super) fn build(&mut self, convert: &mut SelectorConversion<'_>) -> Result<Selector, XamlLoadError> {
        let mut selectors: Vec<Option<Selector>> = Vec::new();
        let mut current = None;
        loop {
            match self.next().map_err(invalid)? {
                SelectorOp::CapturedEquals(property, slot) => {
                    let (property, value) = convert(slot, property)?;
                    current = Some(Selectors::property_equals_untyped(current.take(), property, value));
                }
                SelectorOp::Select(step) => current = Some(selector_leaf(current.take(), &step)),
                SelectorOp::Push => selectors.push(current.take()),
                SelectorOp::Not => {
                    let previous = selectors.pop().expect("validated negated selector");
                    current = match current.take() {
                        Some(arg) => Some(Selectors::not(previous, arg)),
                        None => previous,
                    };
                }
                SelectorOp::Or(n) => {
                    // The saved chains contain the enclosing chain then n-1 alternatives;
                    // the last alternative is current. The original Selectors::or starts anew.
                    let start = selectors.len() - n;
                    let alternatives = selectors.drain(start..).skip(1).flatten().chain(current.take());
                    current = Some(Selectors::or(alternatives));
                }
                SelectorOp::EndSelector => return current.ok_or_else(|| invalid("incomplete selector expression")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::ObjectType;
    use ferroui_controls::Button;

    #[test]
    fn standalone_selector_tracks_type_class_negation_and_property_changes() {
        // Button[IsEnabled=True]:not(.excluded), .alternative
        let program = TypedSelectorProgram::new(
            &[
                op::VERSION,
                s::PUSH,
                s::OF_TYPE,
                0,
                s::EQUALS,
                0,
                0,
                s::PUSH,
                s::CLASS,
                0,
                s::NOT,
                s::PUSH,
                s::CLASS,
                1,
                s::OR,
                2,
                s::END,
            ],
            SelectorImports {
                strings: &["excluded", "alternative"],
                identities: &[<Button as ObjectType>::IDENTITY],
                properties: &[|| ferroui_base::input::InputElement::is_enabled_property()],
                values: &[|| Rc::new(true)],
            },
        );
        let selector = program.build_selector().unwrap();
        let equivalent = Selectors::or([
            Selectors::not(
                Some(Selectors::property_equals_untyped(
                    Some(Selectors::of_type_identity(None, <Button as ObjectType>::IDENTITY)),
                    ferroui_base::input::InputElement::is_enabled_property(),
                    Rc::new(true),
                )),
                Selectors::class(None, "excluded"),
            ),
            Selectors::class(None, "alternative"),
        ]);
        assert_eq!(selector.to_string(), equivalent.to_string());
        let button = Button::new();
        let matches = || selector.match_(&button, None, false).is_match();
        assert!(matches());
        button.classes().add("excluded");
        assert!(!matches());
        button.classes().remove("excluded");
        button.set_is_enabled(false);
        assert!(!matches());
        button.classes().add("alternative");
        assert!(matches());
    }

    #[test]
    fn malformed_standalone_streams_never_evaluate_an_import() {
        static IMPORTS: SelectorImports = SelectorImports {
            strings: &[""],
            identities: &[],
            properties: &[|| panic!("invalid program invoked a property")],
            values: &[|| panic!("invalid program invoked a value")],
        };
        for code in [
            &[][..],
            &[99],
            &[op::VERSION, s::END],
            &[op::VERSION, s::NAME, 0, s::END],
            &[op::VERSION, s::NESTING, s::CHILD],
            &[op::VERSION, s::CHILD, s::END],
            &[op::VERSION, s::NESTING, s::END, s::END],
            &[op::VERSION, s::OF_TYPE, 0, s::END],
            &[op::VERSION, s::PUSH, s::NOT, s::END],
            &[op::VERSION, s::PUSH, s::NESTING, s::OR, 2, s::END],
            &[op::VERSION, s::EQUALS, 0, 0, 255, s::END],
        ] {
            let program = TypedSelectorProgram::new(code, IMPORTS);
            for _ in 0..2 {
                assert!(
                    program
                        .build_selector()
                        .unwrap_err()
                        .message()
                        .contains("Invalid compiled XAML selector"),
                    "{code:?}"
                );
            }
        }
    }
}

#[cfg(test)]
mod lexical_tests {
    use super::*;

    #[test]
    fn scoped_conversions_evaluate_in_order_and_preserve_the_first_error() {
        ferroui_base::ferro_thread_local! {
            static TRACE: std::cell::RefCell<Vec<&'static str>> = std::cell::RefCell::new(Vec::new());
        }
        fn property() -> &'static FerroProperty {
            TRACE.with(|v| v.borrow_mut().push("property"));
            ferroui_base::input::InputElement::is_enabled_property()
        }
        fn value() -> BoxedValue {
            TRACE.with(|v| v.borrow_mut().push("closed value"));
            Rc::new(true)
        }
        let program = TypedSelectorProgram::new(
            &[
                op::VERSION,
                s::EQUALS,
                0,
                0,
                s::CAPTURED_EQUALS,
                0,
                0,
                s::CAPTURED_EQUALS,
                0,
                1,
                s::END,
            ],
            SelectorImports {
                properties: &[property],
                values: &[value],
                ..SelectorImports::EMPTY
            },
        );
        // Validation runs completely before the two kinds of imports, even on reuse.
        assert!(program.build_selector().is_err());
        TRACE.with(|v| assert!(v.borrow().is_empty()));
        let error = program
            .build_selector_with(2, &mut |slot, property| {
                if slot == 1 {
                    return Err(XamlLoadError::with_message("conversion at line 8 position 23"));
                }
                TRACE.with(|v| v.borrow_mut().push("source setup"));
                let property = property();
                TRACE.with(|v| v.borrow_mut().push("converted value"));
                Ok((property, Rc::new(false)))
            })
            .unwrap_err();
        assert_eq!(error.message(), "conversion at line 8 position 23");
        TRACE.with(|v| {
            assert_eq!(
                *v.borrow(),
                ["property", "closed value", "source setup", "property", "converted value"]
            )
        });
        let bad = TypedSelectorProgram::new(
            &[op::VERSION, s::CAPTURED_EQUALS, 0, 1, s::END],
            SelectorImports {
                properties: &[property],
                ..SelectorImports::EMPTY
            },
        );
        assert!(bad
            .build_selector_with(2, &mut |_, _| panic!("invalid slot invoked"))
            .is_err());
    }
}
