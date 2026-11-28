//! Builds the same reactive path used by native compiled bindings. Only path construction
//! reads bytecode. Notifications and updates use the ordinary binding engine.
use crate::XamlLoadError;
use ferroui_base::{
    controls::INameScope,
    data::{CompiledBindingPath, CompiledBindingPathBuilder},
    TypeInfo,
};
use ferroui_markup::{binding_opcodes as op, compiled_operands as wire};
use std::rc::Rc;

pub enum BindingPathImport {
    Property(super::compiled::PropertyFn),
    Text(&'static str),
    Type(&'static TypeInfo),
    Step(
        fn(
            &CompiledBindingPathBuilder,
            &[&CompiledBindingPath],
        ) -> Option<CompiledBindingPathBuilder>,
    ),
}

pub struct BindingPathProgram {
    pub code: &'static [u8],
    pub imports: &'static [BindingPathImport],
}

impl BindingPathProgram {
    #[inline(never)]
    pub fn build(
        &self,
        scope: Option<&Rc<dyn INameScope>>,
        paths: &[&CompiledBindingPath],
        line: i32,
        column: i32,
    ) -> Result<CompiledBindingPath, XamlLoadError> {
        fn invalid() -> XamlLoadError {
            XamlLoadError::with_message("Invalid compiled XAML binding path program")
        }
        let unsigned = |code: &mut &[u8]| wire::read_unsigned(code).map_err(|_| invalid());
        let import = |code: &mut &[u8]| {
            self.imports
                .get(unsigned(code)? as usize)
                .ok_or_else(invalid)
        };
        let mut builder = CompiledBindingPathBuilder::new();
        let mut code = self.code;
        while let Some((&tag, rest)) = code.split_first() {
            code = rest;
            builder = match tag {
                op::NOT => builder.not(),
                op::OBSERVABLE => builder.stream_observable(),
                op::TASK => builder.stream_task(),
                op::SELF => builder.self_(),
                op::TEMPLATE_PARENT => builder.templated_parent(),
                op::NAMED => {
                    let BindingPathImport::Text(name) = import(&mut code)? else {
                        return Err(invalid());
                    };
                    builder
                        .element_name(super::compiled::path_name_scope(scope, line, column)?, name)
                }
                op::PROPERTY | op::NULLABLE_PROPERTY => {
                    let BindingPathImport::Property(property) = import(&mut code)? else {
                        return Err(invalid());
                    };
                    builder.ferro_property_with(property(), tag == op::NULLABLE_PROPERTY)
                }
                op::ANCESTOR | op::VISUAL_ANCESTOR => {
                    let BindingPathImport::Type(ty) = import(&mut code)? else {
                        return Err(invalid());
                    };
                    let level = unsigned(&mut code)? as usize;
                    if tag == op::ANCESTOR {
                        builder.ancestor(Some(ty), level)
                    } else {
                        builder.visual_ancestor(Some(ty), level)
                    }
                }
                op::ARRAY => {
                    let count = unsigned(&mut code)? as usize;
                    if count > code.len() {
                        return Err(invalid());
                    }
                    let indices: Result<Vec<_>, _> =
                        (0..count).map(|_| wire::read_signed(&mut code)).collect();
                    builder.array_element(&indices.map_err(|_| invalid())?)
                }
                op::IMPORTED_STEP => {
                    let BindingPathImport::Step(step) = import(&mut code)? else {
                        return Err(invalid());
                    };
                    step(&builder, paths).ok_or_else(invalid)?
                }
                _ => return Err(invalid()),
            };
        }
        Ok(builder.build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::xaml_il::runtime::compiled as rt;
    use ferroui_base::{controls::NameScope, input::InputElement, FerroObject};
    use ferroui_controls::Button;

    #[test]
    fn named_compiled_binding_tracks_the_live_source_through_construction_program() {
        use ferroui_markup::construction_opcodes as c;
        crate::register_types();
        static PATH: BindingPathProgram = BindingPathProgram {
            code: &[op::NAMED, 0, op::PROPERTY, 1],
            imports: &[
                BindingPathImport::Text("source"),
                BindingPathImport::Property(|| InputElement::is_enabled_property()),
            ],
        };
        static BIND: rt::ConstructionProgram = rt::ConstructionProgram {
            code: &[
                c::BUILD_PATH,
                0,
                21,
                8,
                0,
                c::NEW_BINDING,
                1,
                c::PROVIDE_BINDING,
                0,
                3,
                1,
                21,
                8,
            ],
            imports: &rt::ConstructionImports { strings: &[], operations: &[
                rt::ConstructionImport::Path(&PATH),
                rt::ConstructionImport::Property(|| InputElement::is_enabled_property()),
            ] },
        };
        let source = Button::new();
        let target = Button::new();
        let scope: Rc<dyn INameScope> = Rc::new(NameScope::new());
        scope
            .try_register("source", source.clone().upcast())
            .unwrap();
        scope.complete();
        let context = rt::create_context(None, None, &[]);
        let inputs = [rt::ConstructionInput::Object(
            target.upcast_ref::<FerroObject>(),
        )];
        let error = BIND.apply(&inputs, None, Some(&context)).unwrap_err();
        assert!(error.message().contains("line 21 position 8"), "{error}");
        BIND.apply(&inputs, Some(&scope), Some(&context)).unwrap();
        for value in [false, true, false] {
            source.set_is_enabled(value);
            assert_eq!(target.is_enabled(), value);
        }
    }

    #[test]
    fn rejects_invalid_binding_programs_and_preserves_path_operands() {
        let path = BindingPathProgram {
            code: &[op::SELF, op::NULLABLE_PROPERTY, 0, op::ARRAY, 2, 5, 14],
            imports: &[BindingPathImport::Property(|| {
                InputElement::is_enabled_property()
            })],
        }
        .build(None, &[], 1, 1)
        .unwrap();
        let steps: Vec<_> = path.elements().collect();
        assert_eq!(steps[1].name(), Some("IsEnabled"));
        assert!(steps[1].accepts_null());
        assert_eq!(steps[2].array_indices(), Some(&[-3, 7][..]));
        for code in [
            &[op::PROPERTY, 0][..],
            &[op::ARRAY, 127],
            &[op::ANCESTOR],
            &[250],
        ] {
            assert!(BindingPathProgram { code, imports: &[] }
                .build(None, &[], 1, 1)
                .is_err());
        }
    }
}
