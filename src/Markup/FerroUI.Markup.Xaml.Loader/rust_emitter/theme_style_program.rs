//! One compact, independently linkable program per deferred ControlTheme factory.
use super::*;
use ferroui_markup::{
    compiled_operands as wire,
    style_opcodes::{self as op, member as m, selector as s, value as v},
};
use std::fmt::Write;

#[derive(Default)]
struct Program {
    code: Vec<u8>,
    position: (i32, i32),
    strings: Vec<String>,
    types: Vec<String>,
    identities: Vec<String>,
    properties: Vec<String>,
    selector_values: Vec<String>,
    templates: Vec<String>,
    expressions: Vec<String>,
    computed: Vec<String>,
}

fn import(code: &mut Vec<u8>, pool: &mut Vec<String>, item: &str) {
    let index = pool.iter().position(|v| v == item).unwrap_or_else(|| {
        pool.push(item.to_owned());
        pool.len() - 1
    });
    wire::unsigned(code, index.try_into().expect("style imports fit in 32 bits"));
}

impl Program {
    fn position(&mut self, next: (i32, i32)) {
        wire::signed(&mut self.code, next.0.wrapping_sub(self.position.0));
        wire::signed(&mut self.code, next.1.wrapping_sub(self.position.1));
        self.position = next;
    }

    fn selector(&mut self, pool: &mut StylePool, steps: &[Step]) {
        for step in steps {
            match step {
                Step::Nesting => self.code.push(s::NESTING),
                Step::Template => self.code.push(s::TEMPLATE),
                Step::Child => self.code.push(s::CHILD),
                Step::Descendant => self.code.push(s::DESCENDANT),
                Step::Class(name) | Step::Name(name) => {
                    self.code.push(if matches!(step, Step::Class(_)) { s::CLASS } else { s::NAME });
                    import(&mut self.code, &mut self.strings, name);
                }
                Step::Is(t) | Step::OfType(t) => {
                    self.code.push(if matches!(step, Step::Is(_)) { s::IS } else { s::OF_TYPE });
                    import(&mut self.code, &mut self.identities, t);
                }
                Step::NthChild(stride, offset) | Step::NthLastChild(stride, offset) => {
                    self.code.push(if matches!(step, Step::NthChild(..)) { s::NTH } else { s::NTH_LAST });
                    wire::signed(&mut self.code, *stride);
                    wire::signed(&mut self.code, *offset);
                }
                Step::PropertyEquals { property, value } => {
                    self.code.push(s::EQUALS);
                    import(&mut self.code, &mut self.properties, &pool.property(property));
                    import(&mut self.code, &mut self.selector_values, &pool.selector_value(value));
                }
                Step::CapturedEquals { property, slot } => {
                    self.code.push(s::CAPTURED_EQUALS);
                    import(&mut self.code, &mut self.properties, &pool.property(property));
                    wire::unsigned(&mut self.code, (*slot).try_into().expect("selector captures fit in 32 bits"));
                }
                Step::Not(argument) => {
                    self.code.push(s::PUSH);
                    self.selector(pool, argument);
                    self.code.push(s::NOT);
                }
                Step::Or(alternatives) => {
                    for alternative in alternatives {
                        self.code.push(s::PUSH);
                        self.selector(pool, alternative);
                    }
                    self.code.push(s::OR);
                    wire::unsigned(&mut self.code, alternatives.len().try_into().expect("selector alternatives fit in 32 bits"));
                }
            }
        }
    }

    fn constant(&mut self, constant: &Constant) {
        match constant {
            Constant::Null => self.code.push(v::NULL),
            Constant::Bool(value) => self.code.push(if *value { v::TRUE } else { v::FALSE }),
            Constant::Int(value) => {
                self.code.push(v::INT);
                wire::signed(&mut self.code, *value);
            }
            Constant::Double(value) => {
                self.code.push(v::DOUBLE);
                wire::number(&mut self.code, number_literal(value));
            }
            Constant::Text(text) => {
                self.code.push(v::TEXT);
                import(&mut self.code, &mut self.strings, text);
            }
            Constant::Thickness(ns) | Constant::CornerRadius(ns) => {
                self.code.push(if matches!(constant, Constant::Thickness(_)) { v::THICKNESS } else { v::CORNER });
                for n in ns {
                    wire::number(&mut self.code, number_literal(n));
                }
            }
        }
    }

    fn value(&mut self, function: &str, values: &mut Vec<String>, value: &Value) {
        match value {
            Value::Dynamic(key) => {
                self.code.push(v::DYNAMIC);
                import(&mut self.code, &mut self.strings, key);
            }
            Value::Static { key, position } => {
                self.code.push(v::STATIC);
                import(&mut self.code, &mut self.strings, key);
                self.position(*position);
            }
            Value::Template(template) => {
                self.code.push(v::TEMPLATE);
                self.code.push(u8::from(template.parent) | (u8::from(template.target_type.is_some()) << 1));
                if let Some(target) = &template.target_type {
                    import(&mut self.code, &mut self.types, target);
                }
                import(&mut self.code, &mut self.types, &template.content_type);
                import(&mut self.code, &mut self.templates, &template.build);
                self.position(template.position);
            }
            Value::Constant(constant) => self.constant(constant),
            Value::Expression(_) => {
                self.code.push(v::EXPRESSION);
                import(&mut self.code, &mut self.expressions, &native_value(function, values, value));
            }
            Value::Computed { .. } => {
                self.code.push(v::COMPUTED);
                import(&mut self.code, &mut self.computed, &native_value(function, values, value));
            }
            Value::Absent => self.code.push(v::ABSENT),
        }
    }

    fn members(&mut self, pool: &mut StylePool, function: &str, values: &mut Vec<String>, members: &[Member]) {
        for member in members {
            match member {
                Member::TargetType(t) => {
                    self.code.push(m::TARGET);
                    import(&mut self.code, &mut self.types, t);
                }
                Member::BasedOn { key, position } => {
                    match key {
                        Key::Text(text) => {
                            self.code.push(m::BASE_TEXT);
                            import(&mut self.code, &mut self.strings, text);
                        }
                        Key::Type(t) => {
                            self.code.push(m::BASE_TYPE);
                            import(&mut self.code, &mut self.types, t);
                        }
                    }
                    self.position(*position);
                }
                Member::Setter { property, form, value } => {
                    self.code.push(match form {
                        Form::Plain => m::SET,
                        Form::Created => m::SET_CREATED,
                        Form::Parented => m::SET_PARENTED,
                    });
                    import(&mut self.code, &mut self.properties, &pool.property(property));
                    self.value(function, values, value);
                }
                Member::Style(style) => {
                    self.code.push(if style.parent { m::STYLE_PARENTED } else { m::STYLE });
                    self.code.push(u8::from(style.selector.is_some()));
                    if let Some(steps) = &style.selector {
                        self.selector(pool, steps);
                        self.code.push(s::END);
                    }
                    self.members(pool, function, values, &style.members);
                    self.code.push(m::END_STYLE);
                    self.position(style.position);
                }
            }
        }
    }
}

pub(super) fn standalone_selector(pool: &mut StylePool, name: &str, steps: &[Step], image: &super::super::BytecodeImage) {
    let mut program = Program::default();
    program.code.push(op::VERSION);
    program.selector(pool, steps);
    program.code.push(s::END);
    let bytes = image.program(&program.code);
    let mut item = format!("static {name}: rt::TypedSelectorProgram = rt::TypedSelectorProgram::new({bytes}, rt::SelectorImports {{\n");
    for (field, imports) in [
        ("strings", program.strings), ("identities", program.identities),
        ("properties", program.properties), ("values", program.selector_values),
    ] {
        if !imports.is_empty() {
            writeln!(item, "    {field}: &[{}],", imports.join(", ")).unwrap();
        }
    }
    item.push_str("    ..rt::SelectorImports::EMPTY\n});\n");
    pool.items.push(item);
}

pub(in super::super) fn control_theme_program(pool: &mut StylePool, function: &str, theme: &Theme, image: &super::super::BytecodeImage) -> String {
    let table = format!("{}_THEME", function.to_uppercase());
    let mut program = Program::default();
    program.code.extend([op::VERSION, u8::from(theme.parent)]);
    let mut values = Vec::new();
    program.members(pool, function, &mut values, &theme.members);
    let bytes = image.program(&program.code);
    let mut item = format!("static {table}: rt::TypedStyleProgram = rt::TypedStyleProgram::new({bytes}, rt::TypedStyleImports {{\n");
    for (name, items) in [
        ("strings", program.strings),
        ("types", program.types),
        ("identities", program.identities),
        ("properties", program.properties),
        ("selector_values", program.selector_values),
        ("templates", program.templates),
        ("expressions", program.expressions),
        ("computed", program.computed),
    ] {
        if !items.is_empty() {
            writeln!(item, "    {name}: &[{}],", items.join(", ")).unwrap();
        }
    }
    item.push_str("    ..rt::TypedStyleImports::EMPTY\n});\n");
    pool.items.push(item);
    pool.items.extend(values);
    format!("{table}.build(&context)")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_operands_preserve_numeric_bits_and_share_text_imports() {
        let mut p = Program::default();
        p.constant(&Constant::Null);
        p.constant(&Constant::Bool(false));
        p.constant(&Constant::Bool(true));
        p.constant(&Constant::Int(-129));
        p.constant(&Constant::Double("-0.0_f64".into()));
        p.constant(&Constant::Text("\"shared\"".into()));
        p.constant(&Constant::Text("\"shared\"".into()));
        p.constant(&Constant::Thickness(["-32_f64", "0_f64", "1_f64", "222_f64"].map(str::to_owned)));
        p.constant(&Constant::CornerRadius(["0_f64", "2_f64", "3_f64", "4_f64"].map(str::to_owned)));
        let mut expected = vec![v::NULL, v::FALSE, v::TRUE, v::INT, 129, 2, v::DOUBLE, 255];
        expected.extend((-0.0_f64).to_le_bytes());
        expected.extend([v::TEXT, 0, v::TEXT, 0, v::THICKNESS, 0, 32, 33, 254, v::CORNER, 32, 34, 35, 36]);
        assert_eq!(p.code, expected);
        assert_eq!(p.strings, ["\"shared\""]);
    }
}
