//! Native and bytecode lowering of resolved XamlX selector chains.
use super::*;
use theme_styles::Step;

impl Emitter<'_> {
    pub(super) fn selector(&mut self, node: &Rc<dyn IXamlAstNode>) -> EmitResult<Option<String>> {
        let start = self.lines.len();
        let mut steps = Vec::new();
        let mut captures = Vec::new();
        let local = self.visit_selector(node, &mut steps, &mut captures)?;
        if self.documents.backend.uses_construction_programs() {
            if let Some(local) = &local {
                let program = self.theme_styles.resolve_selector(steps);
                self.lines.truncate(start);
                let expression = if captures.is_empty() {
                    let expression = format!("{program}.build_selector()?");
                    self.record_resolved_call(
                        &expression,
                        construction::ResolvedCall {
                            kind: construction::CallKind::Selector,
                            inputs: Vec::new(),
                            import: format!("Selector(&{program})"),
                            result: Some(("::ferroui_base::styling::Selector".into(), false)),
                            line: 0,
                            column: 0,
                        },
                    );
                    expression
                } else {
                    // A conversion that reads a scoped local or can fail is evaluated at its
                    // equality opcode, with the original source location and property/value order.
                    let count = captures.len();
                    let arms = captures.join("\n");
                    format!("{program}.build_selector_with({count}, &mut |slot, property| match slot {{\n{arms}\n_ => unreachable!(),\n}})?")
                };
                self.line(format!("let {local} = {expression};"));
            }
        }
        Ok(local)
    }

    fn visit_selector(
        &mut self,
        node: &Rc<dyn IXamlAstNode>,
        steps: &mut Vec<Step>,
        captures: &mut Vec<String>,
    ) -> EmitResult<Option<String>> {
        const SELECTORS: &str = "::ferroui_base::styling::Selectors";
        if node.is::<XamlIlSelectorInitialNode>() {
            return Ok(None);
        }
        let mut previous = |emitter: &mut Self, previous: &Option<Rc<dyn XamlIlSelectorNode>>| -> EmitResult<String> {
            Ok(match previous {
                Some(previous) => match emitter.visit_selector(&previous.clone().as_node(), steps, captures)? {
                    Some(local) => format!("::core::option::Option::Some({local})"),
                    None => "::core::option::Option::None".to_string(),
                },
                None => "::core::option::Option::None".to_string(),
            })
        };
        let types = self.types;
        let class_of = |type_: &Rc<dyn IXamlType>| -> EmitResult<String> {
            let class = types.class_of(&**type_).ok_or_else(|| {
                unsupported(
                    node,
                    format!(
                        "{} is not a class of the object model: it cannot be used as a control type",
                        type_.get_full_name()
                    ),
                )
            })?;
            let path = class
                .rust_path()
                .ok_or_else(|| unsupported(node, format!("no public Rust path is recorded for {}", class.full_name())))?;
            // The identity of the class, not its type: a style for a class the application never
            // creates does not keep the class in the executable.
            Ok(format!("<{} as ::ferroui_base::ObjectType>::IDENTITY", absolute(path)))
        };
        let (call, step) = if let Some(n) = node.cast::<XamlIlTypeSelector>() {
            let previous = previous(self, &n.base.previous)?;
            let class = class_of(&n.target_type)?;
            match n.concrete {
                true => (
                    format!("{SELECTORS}::of_type_identity({previous}, {class})"),
                    Step::OfType(class),
                ),
                false => (format!("{SELECTORS}::is_type_identity({previous}, {class})"), Step::Is(class)),
            }
        } else if let Some(n) = node.cast::<XamlIlStringSelector>() {
            let previous = previous(self, &n.base.previous)?;
            let text = rust_string_literal(&n.string());
            match n.selector_type {
                XamlIlStringSelectorType::Class => (format!("{SELECTORS}::class({previous}, {text})"), Step::Class(text)),
                XamlIlStringSelectorType::Name => (format!("{SELECTORS}::name({previous}, {text})"), Step::Name(text)),
            }
        } else if let Some(n) = node.cast::<XamlIlCombinatorSelector>() {
            let previous = previous(self, &n.base.previous)?;
            match n.selector_type {
                CombinatorSelectorType::Child => (format!("{SELECTORS}::child({previous})"), Step::Child),
                CombinatorSelectorType::Descendant => (format!("{SELECTORS}::descendant({previous})"), Step::Descendant),
                CombinatorSelectorType::Template => (format!("{SELECTORS}::template({previous})"), Step::Template),
            }
        } else if let Some(n) = node.cast::<XamlIlNotSelector>() {
            let previous = previous(self, &n.base.previous)?;
            let mut argument_steps = Vec::new();
            let argument = self
                .visit_selector(&n.argument.clone().as_node(), &mut argument_steps, captures)?
                .ok_or_else(|| unsupported(node, "a not selector without an argument"))?;
            (format!("{SELECTORS}::not({previous}, {argument})"), Step::Not(argument_steps))
        } else if let Some(n) = node.cast::<XamlIlNthChildSelector>() {
            let previous = previous(self, &n.base.previous)?;
            match n.selector_type {
                XamlIlNthChildSelectorType::NthChild => (
                    format!("{SELECTORS}::nth_child({previous}, {}, {})", n.step, n.offset),
                    Step::NthChild(n.step, n.offset),
                ),
                XamlIlNthChildSelectorType::NthLastChild => (
                    format!("{SELECTORS}::nth_last_child({previous}, {}, {})", n.step, n.offset),
                    Step::NthLastChild(n.step, n.offset),
                ),
            }
        } else if let Some(n) = node.cast::<XamlIlPropertyEqualsSelector>() {
            let previous = previous(self, &n.base.previous)?;
            let field = n.resolve_ferro_property_field().map_err(|e| failed(node, e))?;
            self.equality_call(node, previous, &field, &n.value(), captures)?
        } else if let Some(n) = node.cast::<XamlIlAttachedPropertyEqualsSelector>() {
            let previous = previous(self, &n.base.previous)?;
            self.equality_call(node, previous, &n.property_filed(), &n.value(), captures)?
        } else if let Some(n) = node.cast::<XamlIlOrSelectorNode>() {
            let alternatives = n.selectors();
            match alternatives.as_slice() {
                [] => return Err(unsupported(node, "an or selector without alternatives")),
                [only] => return self.visit_selector(&only.clone().as_node(), steps, captures),
                _ => {
                    let mut list = Vec::with_capacity(alternatives.len());
                    let mut chains = Vec::with_capacity(alternatives.len());
                    for alternative in &alternatives {
                        let mut chain = Vec::new();
                        list.push(
                            self.visit_selector(&alternative.clone().as_node(), &mut chain, captures)?
                                .ok_or_else(|| unsupported(node, "an alternative of an or selector that is empty"))?,
                        );
                        chains.push(chain);
                    }
                    (format!("{SELECTORS}::or([{}])", list.join(", ")), Step::Or(chains))
                }
            }
        } else if let Some(n) = node.cast::<XamlIlNestingSelector>() {
            let previous = previous(self, &n.base.previous)?;
            (format!("{SELECTORS}::nesting({previous})"), Step::Nesting)
        } else {
            return Err(unsupported(node, "no emitter for this selector node"));
        };
        steps.push(step);
        let local = self.local_named("selector");
        self.line(format!("let {local} = {call};"));
        Ok(Some(local))
    }

    fn equality_call(
        &mut self,
        node: &Rc<dyn IXamlAstNode>,
        previous: String,
        field: &Rc<dyn IXamlField>,
        value: &Rc<dyn IXamlAstValueNode>,
        captures: &mut Vec<String>,
    ) -> EmitResult<(String, Step)> {
        let definition = self.registered_definition(node, &field.name(), field)?;
        let property = self
            .types
            .field(field.as_ref())
            .and_then(|field| field.ferro_property())
            .ok_or_else(|| unsupported(node, format!("{}: not a registered property", field.name())))?;
        let value_node = value.clone().as_node();
        let before = self.lines.len();
        let value = self.value(&value_node)?;
        let typed = self.coerce(&value, property.property_type()).ok_or_else(|| {
            unsupported(
                &value_node,
                format!(
                    "{}: the value cannot be stated as `{}`",
                    property.name(),
                    property.property_type_name()
                ),
            )
        })?;
        // A generic registered cast normally infers its destination from a typed setter.
        // Selector values are immediately erased into BoxedValue, so supply the registered
        // property's exact type before erasure. The accessor is a type witness and is not run.
        let typed = if typed.starts_with("rt::cast(") || typed == "::core::option::Option::None" {
            let accessor = definition.strip_suffix("()").expect("a resolved property accessor");
            format!("rt::selector_operand({accessor}, {typed})")
        } else {
            typed
        };
        let closed = before == self.lines.len()
            && !theme_styles::without_strings(&typed).contains('?')
            && identifiers(&typed).iter().all(|name| {
                !matches!(name.as_str(), "context" | "name_scope" | "service_provider" | "root")
                    && !name.rsplit_once('_').is_some_and(|(prefix, index)| {
                        index
                            .parse::<usize>()
                            .ok()
                            .is_some_and(|i| self.local_names.get(prefix).is_some_and(|count| i < *count))
                    })
            });
        let property = format!("rt::property({definition})");
        let call = format!(
            "::ferroui_base::styling::Selectors::property_equals_untyped({previous}, {property}, ::std::rc::Rc::new({typed}))"
        );
        let step = if closed {
            Step::PropertyEquals { property, value: typed }
        } else {
            let slot = captures.len();
            let setup = self.lines[before..].join("\n");
            captures.push(format!(
                "{slot} => {{\n{setup}\nOk((property(), ::std::rc::Rc::new({typed})))\n}},"
            ));
            Step::CapturedEquals { property, slot }
        };
        Ok((call, step))
    }
}

#[cfg(all(test, feature = "runtime"))]
mod tests {
    use crate::rust_emitter::{compile_documents_with, EmitterHost, TransformOptions, XamlBackend};

    #[test]
    fn fallible_equality_converters_stay_in_their_lexical_scope() {
        ferroui_markup_xaml::register_types();
        let host = EmitterHost::runtime(&[]).unwrap();
        let source = r#"<Border xmlns="https://github.com/ferroui">
          <Border.Styles><Style Selector="Border[RenderTransform=none]" /></Border.Styles>
        </Border>"#;
        let options = TransformOptions {
            backend: XamlBackend::Bytecode,
            ..Default::default()
        };
        let docs = compile_documents_with(&host, &[("transform.xaml", source)], Some("ferres://Example/"), &options);
        let generated = docs.into_iter().next().unwrap().source.unwrap();
        assert!(generated.contains(".build_selector_with(1,"), "{generated}");
        assert!(generated.contains("::__markup_parse("));
        assert!(generated.contains("rt::selector_operand(::ferroui_base::Visual::render_transform_property, rt::cast("));
        assert!(!generated.contains("::styling::Selectors::"));
        syn::parse_file(&generated).expect("syntactically valid scoped conversion");
    }

    #[test]
    fn page_and_deferred_template_use_resolved_selectors_with_exact_imports() {
        ferroui_markup_xaml::register_types();
        let host = EmitterHost::runtime(&[]).unwrap();
        let page = r#"<Border xmlns="https://github.com/ferroui">
          <Border.Styles><Style Selector="Button:not(.quiet), Border[(Grid.Row)=1]" /></Border.Styles>
        </Border>"#;
        let template = r#"<ControlTemplate xmlns="https://github.com/ferroui" TargetType="Button">
          <Border><Border.Styles><Style Selector="Button[IsEnabled=True]"><Style Selector="^:pointerover" /></Style></Border.Styles></Border>
        </ControlTemplate>"#;
        let options = TransformOptions {
            backend: XamlBackend::Bytecode,
            ..Default::default()
        };
        let docs = compile_documents_with(
            &host,
            &[("page.xaml", page), ("template.xaml", template)],
            Some("ferres://Example/"),
            &options,
        );
        for doc in docs {
            let source = doc.source.expect("a resolved standalone selector");
            assert!(source.contains("rt::ConstructionImport::Selector(&"), "{}", doc.name);
            assert!(!source.contains("::styling::Selectors::"), "{}", doc.name);
            assert!(source.contains("::ObjectType>::IDENTITY"), "{}", doc.name);
            if doc.name == "page.xaml" {
                assert!(source.contains("::Grid::row_property()"));
                assert_eq!(source.matches(": rt::TypedSelectorProgram =").count(), 1);
            } else {
                assert!(source.contains("_deferred_0("));
                assert_eq!(source.matches(": rt::TypedSelectorProgram =").count(), 2);
            }
        }
    }
}
