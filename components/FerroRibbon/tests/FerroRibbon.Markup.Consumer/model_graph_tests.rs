//! Actual nested model XAML and compound bindings through native reusable metadata.
use ferroribbon::{
    controls::{Ribbon, RibbonButton, RibbonItemHelper, RibbonToggleButton},
    metadata::models::*,
};
use ferroribbon_core::{commands::RibbonRelayCommand, model::*};
use ferroui_base::{
    Rect, Size,
    data::core::{ValueType, ValueTypes},
    metadata::{MarkupType, MarkupTyped},
    threading::Dispatcher,
};
use ferroui_controls::{Control, StackPanel, TextBlock, Window};
use serde_json::Value;
use std::{cell::Cell, rc::Rc};
fn drain() {
    Dispatcher::ui_thread().run_jobs(None);
}
#[test]
fn source_model_metadata_surfaces_and_optional_constructor_arities_are_explicit() {
    crate::tests::run(|| {
        let source: Value =
            serde_json::from_str(include_str!("../fixtures/model-metadata.json")).unwrap();
        let metadata: &[&MarkupType] = &[
            RibbonNodeModelMetadata::MARKUP,
            RibbonItemModelMetadata::MARKUP,
            RibbonButtonModelMetadata::MARKUP,
            RibbonToggleButtonModelMetadata::MARKUP,
            RibbonDropDownButtonModelMetadata::MARKUP,
            RibbonSplitButtonModelMetadata::MARKUP,
            RibbonCheckBoxModelMetadata::MARKUP,
            RibbonComboBoxModelMetadata::MARKUP,
            RibbonSpinnerModelMetadata::MARKUP,
            RibbonTextBoxModelMetadata::MARKUP,
            RibbonSliderModelMetadata::MARKUP,
            RibbonGridPickerModelMetadata::MARKUP,
            RibbonColorPickerModelMetadata::MARKUP,
            RibbonSegmentModelMetadata::MARKUP,
            RibbonSegmentedModelMetadata::MARKUP,
            RibbonMenuItemModelMetadata::MARKUP,
            RibbonMenuSeparatorModelMetadata::MARKUP,
            RibbonMenuHeaderModelMetadata::MARKUP,
            RibbonGroupModelMetadata::MARKUP,
            RibbonTabModelMetadata::MARKUP,
            RibbonContextualGroupModelMetadata::MARKUP,
            RibbonBackstageItemModelMetadata::MARKUP,
            RibbonBackstageModelMetadata::MARKUP,
            RibbonModelMetadata::MARKUP,
        ];
        for class in source["corpus"]["classes"].as_array().unwrap() {
            let name = class["name"].as_str().unwrap();
            let m = metadata.iter().find(|m| m.name == name).unwrap();
            assert_eq!(
                m.base_type().unwrap().name,
                class["baseName"].as_str().unwrap()
            );
            let mut expected = Vec::new();
            for p in class["properties"].as_array().unwrap() {
                let property = p["name"].as_str().unwrap();
                expected.push(property);
                let actual = m
                    .properties
                    .iter()
                    .find(|p| p.name == property)
                    .unwrap_or_else(|| panic!("missing {name}.{property}"));
                assert!(actual.get.is_some());
                assert_eq!(
                    actual.set.is_some(),
                    p["canWrite"].as_bool().unwrap(),
                    "{name}.{property}"
                );
            }
            let mut names = m.properties.iter().map(|p| p.name).collect::<Vec<_>>();
            names.sort_unstable();
            expected.sort_unstable();
            assert_eq!(names, expected, "{name}");
            let actual = m
                .constructors
                .iter()
                .map(|c| c.parameters.len())
                .collect::<Vec<_>>();
            let expected = class["arities"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| a.as_u64().unwrap() as usize)
                .collect::<Vec<_>>();
            assert_eq!(
                actual, expected,
                "{name}: all accepted optional constructor arities"
            );
        }
    });
}
#[test]
fn model_contextual_activation_metadata_uses_source_array_semantics() {
    use ferroui_base::metadata::into_markup_value;

    crate::tests::run(|| {
        let model: Rc<dyn RibbonModelImpl> = RibbonModel::new();
        let first = RibbonContextualGroupModel::new(Some("same".into()), None);
        let duplicate = RibbonContextualGroupModel::new(Some("same".into()), None);
        let unset = RibbonContextualGroupModel::new(None, None);
        unset.set_id(None);
        let empty = RibbonContextualGroupModel::new(Some("".into()), None);
        for group in [&first, &duplicate, &unset, &empty] {
            model.contextual_groups().add(Some(group.clone()));
        }

        let method = RibbonModelMetadata::MARKUP
            .find_methods("SetActiveContextualGroups")
            .next()
            .unwrap();
        let set_active = |ids: Option<Vec<Option<String>>>| {
            (method.invoke)(&[into_markup_value(model.clone()), into_markup_value(ids)]).unwrap();
        };
        set_active(Some(vec![Some("same".into()), None]));
        assert!(first.is_visible() && duplicate.is_visible() && unset.is_visible());
        assert!(!empty.is_visible());
        set_active(Some(vec![Some("SAME".into()), Some(String::new())]));
        assert!(!first.is_visible() && !duplicate.is_visible() && !unset.is_visible());
        assert!(empty.is_visible());
        set_active(None);
        assert!(!empty.is_visible());
    });
}
#[test]
fn independent_nested_models_compiled_runtime_bindings_selection_identity_and_real_controls() {
    crate::tests::run(|| {
        let _theme = crate::tests::NativeThemeScope::new();
        for runtime in [false, true] {
            let value = if runtime {
                ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                    include_str!("ModelGraph.xaml"),
                    Some(&crate::CONSUMER_ASSEMBLY),
                )
                .unwrap()
            } else {
                crate::compiled_xaml::try_load(
                    None,
                    "ferres://ferroribbon_markup_consumer/ModelGraph.xaml",
                )
                .unwrap()
                .unwrap()
            };
            let host = ValueTypes::as_object(&*value)
                .unwrap()
                .cast::<StackPanel>()
                .unwrap();
            let r = host.children().get(0).cast::<Ribbon>().unwrap();
            let graph = host
                .resources()
                .try_get_resource(&"Graph".into(), None)
                .flatten()
                .unwrap();
            let graph =
                ValueTypes::try_cast(&graph, ValueType::of::<Rc<dyn RibbonModelImpl>>()).unwrap();
            let graph = graph
                .downcast_ref::<Rc<dyn RibbonModelImpl>>()
                .unwrap()
                .clone();
            assert!(Rc::ptr_eq(&r.model().unwrap(), &graph));
            let text = |i| host.children().get(i).cast::<TextBlock>().unwrap().text();
            assert_eq!(text(1).as_deref(), Some("Model graph"));
            assert_eq!(text(2).as_deref(), Some("1"));
            assert_eq!(text(3).as_deref(), Some("Home"));
            assert_eq!(r.tabs().count(), 2);
            // The Model attribute is assigned before the subsequent declared XAML tab.
            let generated = r.tabs().get(0).unwrap();
            let declared = r.tabs().get(1).unwrap();
            assert_eq!(generated.id().as_deref(), Some("home"));
            assert_eq!(declared.id().as_deref(), Some("declared"));
            let save = graph.find_item(Some("save".into())).unwrap();
            let native = r.find_item(Some("save".into())).unwrap();
            let associated = Ribbon::get_model(&native).unwrap();
            assert!(std::ptr::addr_eq(
                Rc::as_ptr(&associated),
                Rc::as_ptr(&save)
            ));
            let calls = Rc::new(Cell::new(0));
            let out = calls.clone();
            save.set_command(Some(RibbonRelayCommand::new(move || {
                out.set(out.get() + 1)
            })));
            graph.quick_access_items().add(Some(save.clone()));
            let copy = r.quick_access_tool_bar().unwrap().items().get(0).unwrap();
            assert_eq!(
                RibbonItemHelper::get_source_item(&copy),
                Some(native.clone())
            );
            copy.cast::<RibbonButton>().unwrap().invoke_core();
            assert_eq!(calls.get(), 1);
            let w = Window::new();
            w.set_width(1000.);
            w.set_height(550.);
            w.set_content(Some(Control::boxed(host.clone())));
            w.show();
            w.measure(Size::new(1000., 550.));
            w.arrange(Rect::new(0., 0., 1000., 550.));
            drain();
            r.set_selected_tab(Some(generated.clone()));
            assert_eq!(graph.selected_tab_id(), Some("home".into()));
            assert_eq!(text(4).as_deref(), Some("Home"));
            graph.set_title(Some("Changed".into()));
            assert_eq!(text(1).as_deref(), Some("Changed"));
            graph
                .tabs()
                .get(0)
                .unwrap()
                .set_label(Some("E\u{301} 😀 שלום".into()));
            assert_eq!(generated.header().as_deref(), Some("E\u{301} 😀 שלום"));
            assert_eq!(text(3).as_deref(), Some("E\u{301} 😀 שלום"));
            assert_eq!(text(4).as_deref(), Some("E\u{301} 😀 שלום"));
            let next = RibbonTabModel::new(Some("next".into()), Some("Next".into()));
            graph.tabs().add(Some(next.clone()));
            assert_eq!(text(2).as_deref(), Some("2"));
            graph.set_selected_tab_id(Some("next".into()));
            assert_eq!(r.selected_tab().unwrap().id().as_deref(), Some("next"));
            assert_eq!(text(4).as_deref(), Some("Next"));
            graph.tabs().move_item(1, 0);
            assert_eq!(text(3).as_deref(), Some("Next"));
            // ArrangeContiguous anchors at the earliest existing generated element.
            // Adding the second model tab places it before the later declared tab.
            assert_eq!(r.tabs().get(1), Some(generated));
            assert_eq!(r.tabs().get(2), Some(declared));
            let bold = try_cast_node::<dyn RibbonToggleButtonModelImpl>(
                graph.find_node(Some("bold".into())),
            )
            .unwrap()
            .unwrap();
            let button = r
                .find_item(Some("bold".into()))
                .unwrap()
                .cast::<RibbonToggleButton>()
                .unwrap();
            button.set_is_checked(Some(true));
            assert!(bold.is_checked());
            r.set_model(None);
            graph
                .tabs()
                .get(1)
                .unwrap()
                .set_label(Some("Detached".into()));
            assert_eq!(r.tabs().count(), 1);
            w.close();
            w.set_content(None);
            drain();
        }
    });
}

#[test]
fn downstream_model_subclass_inherits_metadata_dispatches_and_releases_binding_sources() {
    crate::tests::run(|| {
        use crate::model_consumer::ConsumerButtonModel;
        use ferroui_system::component_model::INotifyPropertyChanged;
        for runtime in [false, true] {
            let retained = {
                let value = if runtime {
                    ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::parse(
                        include_str!("DerivedModel.xaml"),
                        Some(&crate::CONSUMER_ASSEMBLY),
                    )
                    .unwrap()
                } else {
                    crate::compiled_xaml::try_load(
                        None,
                        "ferres://ferroribbon_markup_consumer/DerivedModel.xaml",
                    )
                    .unwrap()
                    .unwrap()
                };
                let host = ValueTypes::as_object(&*value)
                    .unwrap()
                    .cast::<StackPanel>()
                    .unwrap();
                let boxed = host
                    .resources()
                    .try_get_resource(&"Model".into(), None)
                    .flatten()
                    .unwrap();
                assert!(
                    boxed.is::<ConsumerButtonModel>(),
                    "resources must hold the actual concrete model allocation"
                );
                let raw: Rc<dyn std::any::Any> = boxed.clone();
                let model = raw.downcast::<ConsumerButtonModel>().unwrap();
                let node = try_cast_model::<dyn RibbonNodeModelImpl>(Some(model.clone()))
                    .unwrap()
                    .unwrap();
                let node_value = ValueTypes::reference_object(
                    &(Rc::new(node.clone()) as ferroui_base::BoxedValue),
                )
                .unwrap();
                let node_any: Rc<dyn std::any::Any> = node_value.clone();
                assert!(Rc::ptr_eq(
                    &model,
                    &node_any.downcast::<ConsumerButtonModel>().unwrap()
                ));
                let first = host.children().get(0).cast::<TextBlock>().unwrap();
                let second = host.children().get(1).cast::<TextBlock>().unwrap();
                assert_eq!(first.text().as_deref(), Some("Before"));
                assert_eq!(second.text().as_deref(), Some("Before"));
                let count = model.notifications();
                let handlers = model.property_changed().handler_count();
                assert!(
                    handlers >= 2,
                    "both paths must subscribe to actual concrete model notifications"
                );
                node.set_label(Some("Changed".into()));
                assert_eq!(model.notifications(), count + 1);
                assert_eq!(first.text().as_deref(), Some("Changed"));
                assert_eq!(second.text().as_deref(), Some("Changed"));
                first.set_text(Some("From view"));
                assert_eq!(node.label(), Some("From view".into()));
                assert_eq!(second.text().as_deref(), Some("From view"));
                let method = RibbonNodeModelMetadata::MARKUP
                    .methods
                    .iter()
                    .find(|m| m.name == "ToString")
                    .unwrap();
                let output = (method.invoke)(&[Some(node_value.clone())])
                    .unwrap()
                    .unwrap();
                assert_eq!(
                    output.downcast_ref::<String>().unwrap(),
                    "consumer:From view"
                );
                // Removing each data context disposes the corresponding source subscription.
                first.set_data_context(None);
                assert_eq!(model.property_changed().handler_count(), handlers - 1);
                second.set_data_context(None);
                assert_eq!(model.property_changed().handler_count(), handlers - 2);
                let retained = Rc::downgrade(&model);
                drop(value);
                drop(host);
                retained
            };
            drain();
            assert!(
                retained.upgrade().is_none(),
                "metadata registration/binding accessors must not retain the model allocation"
            );
        }
    });
}

#[test]
fn original_optional_constructor_defaults_and_supplied_arguments_match_native_invokers() {
    crate::tests::run(|| {
        use ferroribbon_core::RibbonItemSize;
        use serde_json::json;
        let source: Value =
            serde_json::from_str(include_str!("../fixtures/model-metadata.json")).unwrap();
        let arguments: [Option<ferroui_base::BoxedValue>; 5] = [
            Some(Rc::new("id".to_owned())),
            Some(Rc::new("Label".to_owned())),
            None,
            None,
            Some(Rc::new(RibbonItemSize::Large)),
        ];
        let mut actual = Vec::new();
        for m in [
            RibbonButtonModelMetadata::MARKUP,
            RibbonToggleButtonModelMetadata::MARKUP,
        ] {
            for ctor in m.constructors {
                let arity = ctor.parameters.len();
                let value = (ctor.invoke)(&arguments[..arity]).unwrap().unwrap();
                let item =
                    ValueTypes::try_cast(&value, ValueType::of::<Rc<dyn RibbonItemModelImpl>>())
                        .unwrap();
                let item = item.downcast_ref::<Rc<dyn RibbonItemModelImpl>>().unwrap();
                let checked = try_cast_node::<dyn RibbonToggleButtonModelImpl>(Some(item.clone()))
                    .unwrap()
                    .map(|v| v.is_checked());
                let id = item.id().map(|s| s.to_utf8().unwrap());
                let generated = arity == 0
                    && id.as_ref().is_some_and(|s| {
                        s.len() == 32
                            && s.bytes()
                                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                    });
                actual.push(json!({"type":m.name,"arity":arity,"id":if generated{Some("<generated-guid>".to_owned())}else{id},"label":item.label().map(|s|s.to_utf8().unwrap()),"size":item.size().raw(),"visible":item.is_visible(),"enabled":item.is_enabled(),"showLabel":item.show_label(),"canAdd":item.can_add_to_quick_access(),"isChecked":checked}));
            }
        }
        assert_eq!(Value::Array(actual), source["corpus"]["constructors"]);
    });
}

#[test]
fn unpaired_utf16_model_text_is_retained_by_core_and_rejected_explicitly_at_native_string_boundary()
{
    crate::tests::run(|| {
        let original = ferroribbon_core::RibbonString::from_utf16(vec![0xD800]);
        let node: Rc<dyn RibbonNodeModelImpl> =
            RibbonTabModel::new(Some("utf16".into()), Some(original.clone()));
        let value = Rc::new(node.clone()) as ferroui_base::BoxedValue;
        let get = RibbonNodeModelMetadata::MARKUP
            .properties
            .iter()
            .find(|p| p.name == "Label")
            .unwrap()
            .get
            .unwrap();
        let error = get(&[Some(value)])
            .expect_err("native String must not silently substitute replacement glyphs");
        assert!(matches!(
            error,
            ferroui_base::metadata::MarkupInvokeError::Failed(_)
        ));
        assert_eq!(node.label().unwrap().as_utf16(), &[0xD800]);
    });
}
