//! Actual compiled dependency consumer for explicit open integer-backed enums.
#[allow(clippy::clone_on_copy)]
mod generated {
    ferroui_markup_xaml::include_compiled_xaml!();
}
pub use generated::{compiled_markup, compiled_xaml};
#[cfg(test)]
mod tests {
    use ferroui_base::{controls::ResourceDictionary, data::core::ValueTypes};
    use open_enum_values::{Choice, EnumBag};
    use std::rc::Rc;
    #[test]
    fn independent_compiled_runtime_unknown_nullable_and_boundaries() {
        ferroui_controls::register_types();
        open_enum_values::register_types();
        super::compiled_markup::register();
        let compiled =
            super::compiled_xaml::try_load(None, "ferres://open_enum_fixture/Unknown.xaml")
                .unwrap()
                .unwrap();
        let runtime = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
            include_str!("Unknown.xaml"),
            None,
        )
        .unwrap();
        for root in [compiled, runtime] {
            let dict = ValueTypes::as_object(&*root)
                .unwrap()
                .cast::<ResourceDictionary>()
                .unwrap();
            for (key, value, optional) in [
                ("known", 1, None),
                ("unknown", 99, Some(-99)),
                ("limits", i32::MIN, Some(i32::MAX)),
                ("static", 1, Some(0)),
                ("narrowed", i32::MIN, Some(i32::MAX)),
            ] {
                let boxed = dict.try_get_resource(&key.into(), None).flatten().unwrap();
                let bag = boxed.downcast_ref::<Rc<EnumBag>>().unwrap();
                if key == "known" {
                    assert_eq!(bag.closed.get(), open_enum_values::Closed::First);
                    assert_eq!(bag.bits.get(), open_enum_values::Bits(3));
                }
                assert_eq!(bag.value.get(), Choice(value));
                assert_eq!(bag.optional.get(), optional.map(Choice));
            }
        }
    }
    #[test]
    fn runtime_emitter_uses_explicit_factory_and_retains_closed_enum_and_flags_errors() {
        ferroui_controls::register_types();
        open_enum_values::register_types();
        use ferroui_markup_xaml_loader::rust_emitter::compile_documents;
        let config = ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::new();
        let docs = compile_documents(
            &[("Unknown.xaml", include_str!("Unknown.xaml"))],
            None,
            &config,
            &[],
        );
        let source = docs[0].source.as_ref().unwrap();
        assert!(source.contains("ChoiceMetadata::from_i32(99_i32)"));
        assert!(source.contains("ChoiceMetadata::from_i32(-2147483648_i32)"));
        assert!(!source.contains("must_not_be_called"));
        // XamlX narrows numeric enum text before it reaches either back end.
        // This retained framework behavior is NOT source CLR/WinUI overflow
        // parity; the separate reference/reproducer records that limitation.
        for attr in ["Closed=\"99\"", "Bits=\"4\""] {
            let xaml = format!("<e:EnumBag xmlns:e=\"urn:open-enums\" {attr}/>");
            assert!(
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(&xaml, None).is_err(),
                "{xaml}"
            );
            assert!(
                compile_documents(&[("Invalid.xaml", &xaml)], None, &config, &[])[0]
                    .source
                    .is_err(),
                "{xaml}"
            );
        }
    }

    #[test]
    fn model_and_runtime_emitters_agree_and_plain_enum_opt_in_is_required() {
        ferroui_controls::register_types();
        open_enum_values::register_types();
        use ferroui_build::{
            model_set::ModelSet,
            type_system::{ModelEmitTypes, ModelTypeSystem},
        };
        use ferroui_markup_xaml_loader::rust_emitter::{
            EmitterHost, TransformOptions, compile_documents, compile_documents_with,
        };
        let (mut models, _) =
            ModelSet::read(&[std::path::PathBuf::from(env!("FERROUI_XAMLMETA"))]).unwrap();
        let configuration = ferroui_markup_xaml::RuntimeXamlLoaderConfiguration::new();
        let system = ModelTypeSystem::new(models.clone());
        let types = ModelEmitTypes::new(system.clone(), Vec::new());
        let host = EmitterHost {
            type_system: system.as_type_system(),
            types: &types,
            parsers: Vec::new(),
        };
        let docs = [("Unknown.xaml", include_str!("Unknown.xaml"))];
        let actual =
            compile_documents_with(&host, &docs, None, &TransformOptions::of(&configuration));
        let expected = compile_documents(&docs, None, &configuration, &[]);
        assert_eq!(
            actual[0].source.as_ref().unwrap(),
            expected[0].source.as_ref().unwrap()
        );
        for attr in ["Closed=\"99\"", "Bits=\"4\""] {
            let xaml = format!("<e:EnumBag xmlns:e=\"urn:open-enums\" {attr}/>");
            assert!(
                compile_documents_with(
                    &host,
                    &[("Invalid.xaml", &xaml)],
                    None,
                    &TransformOptions::of(&configuration)
                )[0]
                .source
                .is_err()
            );
        }
        let choice = models
            .iter_mut()
            .flat_map(|m| m.types.iter_mut())
            .find(|t| t.name == "Choice" && t.kind == ferroui_build::model::TypeKind::Enum)
            .unwrap();
        choice.enum_i32_factory = None;
        let system = ModelTypeSystem::new(models);
        let types = ModelEmitTypes::new(system.clone(), Vec::new());
        let host = EmitterHost {
            type_system: system.as_type_system(),
            types: &types,
            parsers: Vec::new(),
        };
        let failed =
            compile_documents_with(&host, &docs, None, &TransformOptions::of(&configuration));
        assert!(
            failed[0]
                .source
                .as_ref()
                .unwrap_err()
                .contains("99 is not a member")
        );
    }
}
