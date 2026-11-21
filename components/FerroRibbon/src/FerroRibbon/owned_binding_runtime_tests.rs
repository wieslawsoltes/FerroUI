//! Saved source-runtime helper behavior on real controls and native bindings.
use super::*;
use ferroui_base::data::{
    BindingBase, BindingMode, CompiledBinding, CompiledBindingPathBuilder, ReflectionBinding,
    RelativeSource, RelativeSourceMode,
};
use ferroui_controls::{TextBlock, automation::AutomationProperties};
use serde_json::{Value, json};

fn apply(item: &Control, value: Option<&str>) {
    let view = internal(item).unwrap();
    RibbonItemHelper::set_owned_value(
        item,
        view.owned_values(),
        AutomationProperties::name_property(),
        value.map(str::to_owned),
    )
    .unwrap();
}

fn has_owned(item: &Control) -> bool {
    internal(item)
        .unwrap()
        .owned_values()
        .entries
        .borrow()
        .contains_key(&AutomationProperties::name_property().id())
}

fn snapshot(
    step: &str,
    item: &Control,
    source: &TextBlock,
    baseline: usize,
    trace: &RefCell<Vec<Value>>,
) -> Value {
    let property = AutomationProperties::name_property();
    let local = item.get_local_value(property);
    let kind = match &local {
        None => "unset",
        Some(None) => "null",
        Some(Some(_)) => "value",
    };
    json!({"step":step,"value":item.get_value(property),"local":kind,
        "localValue":local.flatten(),"owned":has_owned(item),
        "bound":item.has_local_value_binding(property.as_property()),
        "subscriptions":source.property_changed_subscriber_count()-baseline,
        "changes":*trace.borrow()})
}

fn observe_case(case: &Value, family: usize, compiled: bool) -> (Value, usize) {
    let item: Ref<Control> = match family {
        0 => super::super::RibbonButton::new().upcast(),
        1 => super::super::RibbonDropDownButton::new().upcast(),
        2 => super::super::RibbonToggleButton::new().upcast(),
        3 => super::super::RibbonCheckBox::new().upcast(),
        _ => panic!("unknown native button family"),
    };
    let name = case["name"].as_str().unwrap();
    let source = TextBlock::new();
    source.set_text(match name {
        "null" => None,
        "different" => Some("application"),
        _ => Some("helper"),
    });
    let baseline = source.property_changed_subscriber_count();
    let parent = TextBlock::new();
    parent.set_text(Some("helper"));
    if name == "template-resolved" {
        item.set_templated_parent(parent.clone());
    }
    let property = AutomationProperties::name_property();
    let trace = Rc::new(RefCell::new(Vec::new()));
    let reenter = Rc::new(Cell::new(false));
    let initial = Rc::new(Cell::new(false));
    let weak = item.downgrade();
    let trace_token = item.property_changed({
        let trace = trace.clone();
        let reenter = reenter.clone();
        let initial = initial.clone();
        move |event| {
            if event.property().id() != property.id() {
                return;
            }
            let item = weak.upgrade().unwrap();
            let value = item.get_value(property);
            // Source expression lookup visibility during initial publication
            // differs from the native binding API. Compare Ribbon values/order;
            // binding presence is compared at every completed operation below.
            trace
                .borrow_mut()
                .push(json!({"value":value,"owned":has_owned(&item)}));
            if value.is_none() && reenter.replace(false) {
                apply(&item, Some("nested"));
            }
            if value.is_some() && initial.replace(false) {
                apply(&item, None);
            }
        }
    });
    let mut actual = Vec::new();
    apply(&item, Some("helper"));
    actual.push(snapshot("seed", &item, &source, baseline, &trace));
    let template = name.starts_with("template");
    let binding: Rc<dyn BindingBase> = if compiled {
        let path = CompiledBindingPathBuilder::new();
        let path = if template {
            path.templated_parent()
        } else {
            path
        };
        let binding = CompiledBinding::new(path.ferro_property(TextBlock::text_property()).build());
        binding.set_fallback_value(None);
        if !template && name != "pending" {
            binding.set_source(Some(Rc::new(source.clone())));
        }
        binding.set_mode(BindingMode::OneWay);
        binding
    } else {
        let binding = ReflectionBinding::new("Text");
        binding.set_fallback_value(None);
        if template {
            binding.set_relative_source(Some(RelativeSource::new(
                RelativeSourceMode::TemplatedParent,
            )));
        } else if name != "pending" {
            binding.set_source(Some(Rc::new(source.clone())));
        }
        binding.set_mode(BindingMode::OneWay);
        binding
    };
    let driver = Rc::new(Cell::new(None));
    if name == "clear-during-source-notification" {
        let weak_source = source.downgrade();
        let weak_item = item.downgrade();
        let token = driver.clone();
        driver.set(Some(source.add_property_changed_handler(Rc::new(
            move |event| {
                if event.property().id() == TextBlock::text_property().id()
                    && let Some(id) = token.take()
                {
                    weak_source
                        .upgrade()
                        .unwrap()
                        .remove_property_changed_handler(id);
                    apply(&weak_item.upgrade().unwrap(), None);
                }
            },
        ))));
    }
    if name == "initial-clear" {
        item.set_value(property, None);
        actual.push(snapshot("pre-bind-null", &item, &source, baseline, &trace));
        initial.set(true);
    }
    let handle = item.bind_binding(property, &binding);
    actual.push(snapshot("bound", &item, &source, baseline, &trace));
    reenter.set(name == "reentrant-clear");
    if name == "clear-during-source-notification" {
        source.set_text(Some("trigger"));
    } else {
        apply(&item, None);
    }
    actual.push(snapshot("cleared", &item, &source, baseline, &trace));
    source.set_text(Some("after"));
    if name == "template-resolved" {
        parent.set_text(Some("after"));
    }
    if name == "pending" {
        item.set_data_context(Some(Rc::new(source.clone())));
    }
    actual.push(snapshot("source-after", &item, &source, baseline, &trace));
    if name == "different" {
        source.set_text(Some("helper"));
        actual.push(snapshot("source-helper", &item, &source, baseline, &trace));
        apply(&item, None);
        actual.push(snapshot("cleared-again", &item, &source, baseline, &trace));
        source.set_text(Some("last"));
        actual.push(snapshot("source-last", &item, &source, baseline, &trace));
    }
    trace_token.dispose();
    handle.dispose();
    (
        json!(actual),
        source.property_changed_subscriber_count() - baseline,
    )
}

fn expected_case(case: &Value) -> Value {
    let mut expected = case["steps"].clone();
    for step in expected.as_array_mut().unwrap() {
        for change in step["changes"].as_array_mut().unwrap() {
            change.as_object_mut().unwrap().remove("bound");
        }
    }
    expected
}

#[test]
fn saved_owned_binding_runtime_cases_match_four_families_and_both_binding_paths() {
    super::owned_tests::run(|| {
        let corpus: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/owned-binding.json")).unwrap();
        for case in corpus["cases"].as_array().unwrap() {
            if case["name"] == "initial-clear" {
                continue;
            }
            for family in 0..4 {
                for compiled in [false, true] {
                    let (actual, remaining) = observe_case(case, family, compiled);
                    assert_eq!(
                        actual,
                        expected_case(case),
                        "{}, family={family}, compiled={compiled}",
                        case["name"]
                    );
                    assert_eq!(remaining, 0);
                }
            }
        }
    });
}

#[test]
fn initial_owned_clear_preserves_the_source_binding_after_its_initial_publication() {
    super::owned_tests::run(|| {
        let corpus: Value =
            serde_json::from_str(include_str!("../../tests/fixtures/owned-binding.json")).unwrap();
        let case = corpus["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == "initial-clear")
            .unwrap();
        let mut observations = Vec::new();
        for family in 0..4 {
            for compiled in [false, true] {
                let (actual, remaining) = observe_case(case, family, compiled);
                observations.push(json!({"family":family,"compiled":compiled,"steps":actual,"subscriptionsAfterDispose":remaining}));
            }
        }
        eprintln!("INITIAL_NATIVE_OBSERVATIONS={}", json!(observations));
        for observed in observations {
            assert_eq!(
                observed["steps"],
                expected_case(case),
                "family={}, compiled={}",
                observed["family"],
                observed["compiled"]
            );
            assert_eq!(observed["subscriptionsAfterDispose"], 0);
        }
    });
}

#[test]
fn model_and_linked_owned_binding_cleanup_and_replacement_use_the_same_controls() {
    super::owned_tests::run(|| {
        use crate::mvvm::{RibbonElementFactory, RibbonModelBindings};
        use ferroribbon_core::model::{RibbonButtonModel, RibbonNodeModelExt};
        let factory = RibbonElementFactory::new();
        let scope = RibbonModelBindings::new();
        let model = RibbonButtonModel::new(None, Some("Owned".into()));
        let item = factory
            .create_item(Some(model.clone()), scope.clone())
            .unwrap();
        let copy = internal(&item).unwrap().create_linked_copy().unwrap();
        let source = TextBlock::new();
        source.set_text(Some("Owned"));
        let before = source.property_changed_subscriber_count();
        let property = AutomationProperties::name_property();
        let binding = ReflectionBinding::new("Text");
        binding.set_source(Some(Rc::new(source.clone())));
        let first = item.bind_binding(property, &binding);
        let second = copy.bind_binding(property, &binding);
        assert_eq!(source.property_changed_subscriber_count(), before + 2);
        model.set_label(None);
        for control in [&item, &copy] {
            assert_eq!(control.get_value(property), None);
            assert!(!control.has_local_value_binding(property.as_property()));
        }
        assert_eq!(source.property_changed_subscriber_count(), before);
        source.set_text(Some("Stale"));
        assert_eq!(item.get_value(property), None);
        assert_eq!(copy.get_value(property), None);
        model.set_label(Some("Next".into()));
        assert_eq!(item.get_value(property).as_deref(), Some("Next"));
        assert_eq!(copy.get_value(property).as_deref(), Some("Next"));
        RibbonItemHelper::unlink(&copy);
        model.set_label(Some("Final".into()));
        assert_eq!(item.get_value(property).as_deref(), Some("Final"));
        assert_eq!(copy.get_value(property).as_deref(), Some("Next"));
        first.dispose();
        second.dispose();
        assert_eq!(item.get_value(property).as_deref(), Some("Final"));
        scope.dispose();
    });
}

#[test]
fn owned_binding_hidden_by_animation_clears_the_local_input_and_preserves_template_source() {
    super::owned_tests::run(|| {
        for template in [false, true] {
            let item = super::super::RibbonButton::new();
            item.set_label(Some("Owned".into()));
            let source = TextBlock::new();
            source.set_text(Some("Owned"));
            let before = source.property_changed_subscriber_count();
            let property = AutomationProperties::name_property();
            let binding = ReflectionBinding::new("Text");
            if template {
                item.set_templated_parent(source.clone());
                binding.set_relative_source(Some(RelativeSource::new(
                    RelativeSourceMode::TemplatedParent,
                )));
            } else {
                binding.set_source(Some(Rc::new(source.clone())));
            }
            let handle = item.bind_binding(property, &binding);
            let animation = item
                .set_value_with_priority(
                    property,
                    Some("Animated".into()),
                    BindingPriority::Animation,
                )
                .unwrap();
            item.set_label(None);
            assert_eq!(item.get_local_value(property), None);
            assert_eq!(item.get_value(property).as_deref(), Some("Animated"));
            assert_eq!(
                item.has_local_value_binding(property.as_property()),
                template
            );
            assert_eq!(
                source.property_changed_subscriber_count(),
                before + usize::from(template)
            );
            source.set_text(Some("After"));
            animation.dispose();
            assert_eq!(
                item.get_value(property).as_deref(),
                if template { Some("After") } else { None }
            );
            handle.dispose();
            assert_eq!(source.property_changed_subscriber_count(), before);
        }
    });
}

#[test]
fn owned_two_way_binding_stops_before_the_clear_can_write_back_to_the_model() {
    super::owned_tests::run(|| {
        let item = super::super::RibbonButton::new();
        item.set_label(Some("Owned".into()));
        let source = TextBlock::new();
        source.set_text(Some("Owned"));
        let before = source.property_changed_subscriber_count();
        let property = AutomationProperties::name_property();
        let binding = ReflectionBinding::new("Text");
        binding.set_source(Some(Rc::new(source.clone())));
        binding.set_mode(BindingMode::TwoWay);
        let handle = item.bind_binding(property, &binding);
        item.set_label(None);
        assert_eq!(item.get_value(property), None);
        assert_eq!(source.text().as_deref(), Some("Owned"));
        assert_eq!(source.property_changed_subscriber_count(), before);
        source.set_text(Some("After"));
        assert_eq!(item.get_value(property), None);
        handle.dispose();
    });
}
