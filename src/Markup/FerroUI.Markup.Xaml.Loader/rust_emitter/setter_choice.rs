//! The resolved overload plan of a runtime-valued property assignment.
use super::*;
use ferroui_markup::{compiled_operands::unsigned, construction_opcodes::choice};

pub(super) enum Predicate {
    Instance(String),
    NonNull,
    Null,
}

impl Predicate {
    pub(super) fn native(&self, value: &str) -> String {
        match self {
            Self::Instance(handle) => format!("rt::is_instance(&{value}, {handle})"),
            Self::NonNull => format!("{value}.is_some()"),
            Self::Null => format!("{value}.is_none()"),
        }
    }
}

fn index(code: &mut Vec<u8>, imports: &mut Vec<String>, item: String) {
    let i = imports
        .iter()
        .position(|entry| entry == &item)
        .unwrap_or_else(|| {
            imports.push(item);
            imports.len() - 1
        });
    unsigned(code, u32::try_from(i).expect("XAML setter choice imports"));
}

impl<'a> Emitter<'a> {
    pub(super) fn emit_setter_choice(
        &mut self,
        node: &Rc<dyn IXamlAstNode>,
        assignment: &XamlPropertyAssignmentNode,
        target: &Typed<'a>,
        value_node: &Rc<dyn IXamlAstNode>,
        value: &str,
        priority: Option<&str>,
        branches: &[(Option<Predicate>, Rc<dyn IXamlPropertySetter>)],
        nullable: Option<&Rc<dyn IXamlPropertySetter>>,
    ) -> EmitResult<bool> {
        let Some((ty, object)) = self.construction_type(target.kind) else {
            return Ok(false);
        };
        if !is_identifier(&target.expr) {
            return Ok(false);
        }
        let mut types = Vec::new();
        let mut actions = Vec::new();
        let mut code = Vec::new();
        let mut always = false;
        let null = Predicate::Null;
        let cases = branches
            .iter()
            .map(|(test, setter)| (test.as_ref(), setter))
            .chain(nullable.map(|setter| (Some(&null), setter)));
        for (test, setter) in cases {
            match test {
                None => {
                    code.push(choice::ALWAYS);
                    always = true;
                }
                Some(Predicate::NonNull) => code.push(choice::NON_NULL),
                Some(Predicate::Null) => code.push(choice::NULL),
                Some(Predicate::Instance(handle)) => {
                    code.push(choice::INSTANCE);
                    index(&mut code, &mut types, format!("|| {handle}"));
                }
            }
            let placeholder = "__ferroui_choice_receiver";
            let receiver = Typed {
                expr: placeholder.to_owned(),
                kind: target.kind,
            };
            let statement = self.setter_statement(
                node,
                assignment,
                setter,
                &receiver,
                &SetterValues::Untyped("value", value_node),
                priority.map(|_| "priority"),
            )?;
            // The setter emitter expects an owned local. Dereference its borrowed
            // slot, leaving the emitter's original borrowing and cloning intact.
            // A collision in a user-provided name keeps that statement native.
            if statement.matches(placeholder).count()
                != identifiers(&statement)
                    .iter()
                    .filter(|word| *word == placeholder)
                    .count()
            {
                return Ok(false);
            }
            let statement = statement.replace(placeholder, "(*receiver)");
            let receiver_ty = if object {
                format!("::ferroui_base::Ref<{ty}>")
            } else {
                ty.clone()
            };
            let used = |name| {
                if statement
                    .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                    .any(|word| word == name)
                {
                    name
                } else {
                    "_"
                }
            };
            let action = format!("|receiver, {}, {}| {{ let receiver = rt::choice_receiver::<{receiver_ty}>(receiver)?; {statement} Ok(()) }}", used("value"), used("priority"));
            index(&mut code, &mut actions, action);
            if always {
                break;
            }
        }
        let bytes = self.documents.bytecode_image.program(&code);
        let name = format!(
            "{}_SETTER_CHOICE_{}",
            self.function_name.to_uppercase(),
            self.theme_styles.items.len()
        );
        let property = rust_string_literal(&assignment.property.name());
        self.theme_styles.items.push(format!("static {name}: rt::TypedSetterChoice = rt::TypedSetterChoice {{ code: {bytes}, types: &[{}], actions: &[{}], property: {property} }};", types.join(", "), actions.join(", ")));
        let default = "::ferroui_base::data::BindingPriority::LocalValue";
        let call = format!(
            "{name}.apply(&{}, &{value}, {}, {}, {})?",
            target.expr,
            priority.unwrap_or(default),
            node.line(),
            node.position()
        );
        let read = if object {
            format!("&args.object::<{ty}>()?")
        } else {
            format!("args.value::<{ty}>()?")
        };
        let mut inputs = vec![
            construction::CallInput::Register(target.expr.clone(), object),
            construction::CallInput::Register(value.to_owned(), false),
        ];
        let read_priority = if let Some(priority) = priority {
            inputs.push(construction::CallInput::Register(
                priority.to_owned(),
                false,
            ));
            "*args.value::<::ferroui_base::data::BindingPriority>()?"
        } else {
            default
        };
        let import = format!("Call(|args| {{ {name}.apply({read}, args.value::<::ferroui_base::metadata::MarkupValue>()?, {read_priority}, args.line, args.column)?; Ok(None) }})");
        self.record_resolved_call(
            &call,
            construction::ResolvedCall {
                kind: construction::CallKind::Member,
                inputs,
                import,
                result: None,
                line: node.line(),
                column: node.position(),
            },
        );
        self.line(format!("{call};"));
        Ok(true)
    }
}
