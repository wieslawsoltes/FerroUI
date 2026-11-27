//! Public Rust construction and native property transfer, not a CLR object shim.
use super::*;
use ferroribbon::mvvm::model_values;
use ferroribbon_core::RibbonString;
use std::panic::{AssertUnwindSafe, catch_unwind};
#[path = "domain_values.rs"]
mod domain;
use domain::DomainValue;

struct Slot {
    field: &'static str,
    node: Rc<RibbonButtonModel>,
    backstage: Rc<RibbonBackstageItemModel>,
}
impl Slot {
    fn new(field: &'static str, prefix: &'static str, trace: Trace) -> Self {
        let this = Self {
            field,
            node: RibbonButtonModel::new(Some("id".into()), Some("Label".into())),
            backstage: RibbonBackstageItemModel::new(Some("info".into()), Some("Label".into())),
        };
        let callback =
            Rc::new(move |name: &str| trace.borrow_mut().push(format!("{prefix}:{name}")));
        if field.starts_with("Backstage") {
            this.backstage.property_changed().add(callback);
        } else {
            this.node.property_changed().add(callback);
        }
        this
    }
    fn core_set(&self, value: Option<RibbonObject>) {
        match self.field {
            "Tag" => self.node.set_tag(value),
            "CommandParameter" => self.node.set_command_parameter(value),
            "BackstageContent" => self.backstage.set_content(value),
            "BackstageCommandParameter" => self.backstage.set_command_parameter(value),
            _ => unreachable!(),
        }
    }
    fn core_get(&self) -> Option<RibbonObject> {
        match self.field {
            "Tag" => self.node.tag(),
            "CommandParameter" => self.node.command_parameter(),
            "BackstageContent" => self.backstage.content(),
            "BackstageCommandParameter" => self.backstage.command_parameter(),
            _ => unreachable!(),
        }
    }
    fn native(&self) -> (BoxedValue, &'static MarkupType, &'static str) {
        match self.field {
            "Tag" => (
                owner(self.node.clone()),
                RibbonButtonModelMetadata::MARKUP,
                "Tag",
            ),
            "CommandParameter" => (
                owner(self.node.clone()),
                RibbonButtonModelMetadata::MARKUP,
                "CommandParameter",
            ),
            "BackstageContent" => (
                owner(self.backstage.clone()),
                RibbonBackstageItemModelMetadata::MARKUP,
                "Content",
            ),
            "BackstageCommandParameter" => (
                owner(self.backstage.clone()),
                RibbonBackstageItemModelMetadata::MARKUP,
                "CommandParameter",
            ),
            _ => unreachable!(),
        }
    }
    fn label(&self) -> Option<RibbonString> {
        if self.field.starts_with("Backstage") {
            self.backstage.label()
        } else {
            self.node.label()
        }
    }
    fn set_label(&self, value: &str) {
        if self.field.starts_with("Backstage") {
            self.backstage.set_label(Some(value.into()));
        } else {
            self.node.set_label(Some(value.into()));
        }
    }
}

#[test]
fn public_native_value_conversion_preserves_96_source_field_transfers_and_comparison_traces() {
    crate::tests::run(|| {
        let mut actual = Vec::new();
        for source_field in [
            "Tag",
            "CommandParameter",
            "BackstageContent",
            "BackstageCommandParameter",
        ] {
            for target_field in [
                "Tag",
                "CommandParameter",
                "BackstageContent",
                "BackstageCommandParameter",
            ] {
                for mode in [
                    "identity",
                    "value",
                    "cross",
                    "false_self",
                    "throw",
                    "reenter",
                ] {
                    let trace: Trace = Rc::default();
                    let source = Slot::new(source_field, "source", trace.clone());
                    let target = Rc::new(Slot::new(target_field, "target", trace.clone()));
                    let policy = match mode {
                        "false_self" => "false",
                        "throw" | "reenter" => "value",
                        value => value,
                    };
                    let a = DomainValue::new("a", policy, trace.clone());
                    // This unregistered, non-Clone value carries its own native
                    // equality; ordinary Rust application construction imports it.
                    assert!(ValueTypes::box_erased(a.clone()).is_err());
                    source.core_set(model_values::from_native(Some(a.clone())));
                    let native = model_values::to_native(source.core_get()).unwrap().unwrap();
                    let (target_owner, metadata, property) = target.native();
                    set(metadata, property, &target_owner, Some(native.clone()));
                    source.core_set(None);
                    drop(source);
                    a.state().set(9);
                    let first = get(metadata, property, &target_owner).unwrap();
                    let same = Rc::ptr_eq(&first, &native);
                    let shared = first.downcast_ref::<DomainValue>().unwrap().state().get();
                    let replacement: BoxedValue = match mode {
                        "false_self" => a.clone(),
                        "cross" => DomainValue::other(trace.clone()),
                        _ => DomainValue::new("b", "identity", trace.clone()),
                    };
                    if mode == "throw" {
                        a.set_hook(Rc::new(|| panic!("equals")));
                    }
                    if mode == "reenter" {
                        let target = Rc::downgrade(&target);
                        a.set_hook(Rc::new(move || {
                            target.upgrade().unwrap().set_label("reentered")
                        }));
                    }
                    let threw = catch_unwind(AssertUnwindSafe(|| {
                        set(metadata, property, &target_owner, Some(replacement))
                    }))
                    .is_err();
                    let final_value = get(metadata, property, &target_owner).unwrap();
                    actual.push(json!({"sourceField":source_field,"targetField":target_field,"mode":mode,"same":same,"shared":shared,"retained":Rc::ptr_eq(&final_value,&native),"threw":threw,"final":final_value.downcast_ref::<DomainValue>().unwrap().name(),"label":target.label().map(|v|v.to_utf8().unwrap()),"trace":*trace.borrow()}));
                }
            }
        }
        let expected: Value =
            serde_json::from_str(include_str!("../fixtures/model-value-transfer.json")).unwrap();
        assert_eq!(Value::Array(actual), expected["corpus"]["cases"]);
    });
}

#[test]
fn public_native_value_boundary_has_typed_errors_and_releases_the_last_owner() {
    crate::tests::run(|| {
        assert!(model_values::from_native(None).is_none());
        assert!(model_values::to_native(None).unwrap().is_none());
        struct CoreOnly;
        let error =
            model_values::to_native(Some(RibbonObject::reference(Rc::new(CoreOnly)))).unwrap_err();
        let as_error: &dyn std::error::Error = &error;
        assert!(
            as_error
                .to_string()
                .contains("Unregistered native model payload")
        );
        let payload = DomainValue::new("a", "false", Rc::default());
        let weak = Rc::downgrade(&payload);
        let source = RibbonButtonModel::new(None, None);
        source.set_tag(model_values::from_native(Some(payload.clone())));
        let reader = model_values::to_native(source.tag()).unwrap();
        let target = RibbonBackstageItemModel::new(None, None);
        target.set_content(model_values::from_native(reader.clone()));
        source.set_tag(None);
        drop(source);
        drop(payload);
        assert!(weak.upgrade().is_some());
        target.set_content(None);
        drop(target);
        assert!(weak.upgrade().is_some());
        drop(reader);
        assert!(weak.upgrade().is_none());
    });
}
