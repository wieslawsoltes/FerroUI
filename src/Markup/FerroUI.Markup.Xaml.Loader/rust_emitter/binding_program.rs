//! Lower the members of a resolved XamlX binding path. The native import for a custom
//! operation retains its typed accessor or delegate and may borrow previously built paths.
use super::{identifiers, rust_string_literal, XamlIlBindingPathElementNode as Element};
use ferroui_markup::{binding_opcodes as op, compiled_operands as wire};

#[derive(Default)]
pub(super) struct Program {
    code: Vec<u8>,
    imports: Vec<String>,
    paths: Vec<String>,
    named: bool,
}

impl Program {
    fn import(&mut self, value: String) {
        let index = self
            .imports
            .iter()
            .position(|old| old == &value)
            .unwrap_or_else(|| {
                self.imports.push(value);
                self.imports.len() - 1
            });
        wire::unsigned(
            &mut self.code,
            index.try_into().expect("binding path import"),
        );
    }

    pub(super) fn step(&mut self, element: &Element, call: &str) {
        match element {
            Element::Not(_) => self.code.push(op::NOT),
            Element::StreamObservable(_) => self.code.push(op::OBSERVABLE),
            Element::StreamTask(_) => self.code.push(op::TASK),
            Element::SelfElement(_) => self.code.push(op::SELF),
            Element::TemplatedParent(_) => self.code.push(op::TEMPLATE_PARENT),
            Element::ElementName(e) => {
                self.named = true;
                self.code.push(op::NAMED);
                self.import(format!("Text({})", rust_string_literal(&e.name)));
            }
            Element::FerroProperty(e) => {
                let definition = if e.accepts_null {
                    call.strip_prefix("builder.ferro_property_with(")
                        .and_then(|s| s.strip_suffix(", true)"))
                } else {
                    call.strip_prefix("builder.ferro_property(")
                        .and_then(|s| s.strip_suffix(')'))
                }
                .expect("resolved registered property");
                self.code.push(if e.accepts_null {
                    op::NULLABLE_PROPERTY
                } else {
                    op::PROPERTY
                });
                self.import(format!("Property(|| {definition})"));
            }
            Element::ArrayIndexer(e) => {
                self.code.push(op::ARRAY);
                wire::unsigned(
                    &mut self.code,
                    e.values.len().try_into().expect("binding path indices"),
                );
                for value in &e.values {
                    wire::signed(&mut self.code, *value);
                }
            }
            Element::FindAncestor(_) | Element::FindVisualAncestor(_) => {
                let visual = matches!(element, Element::FindVisualAncestor(_));
                let (ty, level) = call
                    .split_once("::core::option::Option::Some(")
                    .unwrap()
                    .1
                    .rsplit_once("), ")
                    .unwrap();
                self.code.push(if visual {
                    op::VISUAL_ANCESTOR
                } else {
                    op::ANCESTOR
                });
                self.import(format!("Type({ty})"));
                wire::unsigned(
                    &mut self.code,
                    level.strip_suffix(')').unwrap().parse().unwrap(),
                );
            }
            _ => {
                self.code.push(op::IMPORTED_STEP);
                let mut captures = Vec::new();
                for name in identifiers(call) {
                    if !name
                        .strip_prefix("path_")
                        .is_some_and(|n| n.parse::<usize>().is_ok())
                        || captures.contains(&name)
                    {
                        continue;
                    }
                    captures.push(name);
                }
                let mut body = String::new();
                for name in &captures {
                    let i = self
                        .paths
                        .iter()
                        .position(|old| old == name)
                        .unwrap_or_else(|| {
                            self.paths.push(name.clone());
                            self.paths.len() - 1
                        });
                    body.push_str(&format!("let {name} = *paths.get({i})?; "));
                }
                let paths = if captures.is_empty() { "_" } else { "paths" };
                self.import(format!("Step(|builder, {paths}| {{ {body}Some({call}) }})"));
            }
        }
    }

    pub(super) fn program(&self, image: &super::BytecodeImage) -> String {
        let bytes = image.program(&self.code);
        let imports = self
            .imports
            .iter()
            .map(|i| format!("rt::BindingPathImport::{i}"))
            .collect::<Vec<_>>()
            .join(", ");
        format!("rt::BindingPathProgram {{ code: {bytes}, imports: &[{imports}] }}")
    }

    pub(super) fn build(
        &self,
        program: &str,
        line: i32,
        column: i32,
    ) -> (String, super::construction::ResolvedCall) {
        let paths = self
            .paths
            .iter()
            .map(|p| format!("&{p}"))
            .collect::<Vec<_>>()
            .join(", ");
        let scope = if self.named {
            "name_scope.as_ref()"
        } else {
            "None"
        };
        let call = format!("{program}.build({scope}, &[{paths}], {line}, {column})?");
        let operation = super::construction::ResolvedCall {
            kind: super::construction::CallKind::Path { named: self.named },
            inputs: self
                .paths
                .iter()
                .map(|name| super::construction::CallInput::Register(name.clone(), false))
                .collect(),
            import: format!("Path(&{program})"),
            result: Some((
                "::ferroui_base::data::CompiledBindingPath".to_string(),
                false,
            )),
            line,
            column,
        };
        (call, operation)
    }
}
