use crate::primitives::TemplatedControl;
use crate::templates::{
    DataTemplates, FuncDataTemplate, FuncTemplate, IDataTemplate, ITemplateOf, ITemplateWithParam,
    ITypedDataTemplate,
};
use crate::test_support::boxed_str;
use crate::{Border, Canvas, Control, Decorator};
use ferroui_base::styling::ITemplate;
use ferroui_base::{AnyValue, BoxedValue, FerroObject, Ref};
use std::any::TypeId;
use std::rc::Rc;

/// Ported from the template extensions test of the reference.
#[test]
fn get_template_children_should_not_return_nested_template_controls() {
    let target = TemplatedControl::new();
    let target_object: Ref<FerroObject> = target.clone().upcast();
    let named = |name: &str| {
        let border = Border::new();
        border.set_name(Some(name.to_string()));
        border
    };
    let border1 = named("border1");
    border1.set_templated_parent(&target_object);
    let inner = TemplatedControl::new();
    inner.set_name(Some("inner".to_string()));
    inner.set_templated_parent(&target_object);
    let inner_object: Ref<FerroObject> = inner.clone().upcast();
    let border2 = named("border2");
    border2.set_templated_parent(&inner_object);
    let border3 = named("border3");
    border3.set_templated_parent(&inner_object);
    let border4 = named("border4");
    border4.set_templated_parent(&target_object);
    let border5 = named("border5");

    target.visual_children().add(border1.clone().upcast());
    border1.set_child(&inner);
    inner.visual_children().add(border2.upcast());
    inner.visual_children().add(border3.clone().upcast());
    border3.set_child(&border4);
    border4.set_child(&border5);

    let result: Vec<String> = target.get_template_descendants().iter().map(|x| x.name().unwrap()).collect();

    assert_eq!(result, vec!["border1", "inner", "border4"]);
}

#[test]
fn func_template_builds_typed_and_untyped_values() {
    let template = FuncTemplate::new(|| -> Option<Ref<Control>> { Some(Decorator::new().upcast()) });

    assert!(template.build_typed().unwrap().is::<Decorator>());

    let boxed = template.build();
    let boxed: &dyn AnyValue = &*boxed;
    assert!(boxed.downcast_ref::<Option<Ref<Control>>>().unwrap().as_ref().unwrap().is::<Decorator>());
}

#[test]
fn typed_func_data_template_matches_only_its_type() {
    let template = FuncDataTemplate::for_type::<String>(|_, _| Some(Border::new().upcast()), false);
    let number: BoxedValue = Rc::new(1_i32);

    assert!(template.match_(boxed_str("foo").as_ref()));
    assert!(!template.match_(Some(&number)));
    assert!(!template.match_(None));
    assert!(template.build(&boxed_str("foo")).unwrap().is::<Border>());
}

/// Not from upstream: `TypeUtilities.CanCast<T>` accepts a value of the type
/// a nullable `T` holds (an `int` for `int?`), null for a nullable `T`, and a
/// handle whose object is of class `T`.
#[test]
fn typed_func_data_template_matches_as_type_utilities_can_cast() {
    let nullable = FuncDataTemplate::for_type::<Option<i32>>(
        |value, _| {
            let border = Border::new();
            border.set_tag(Some(Rc::new(value.unwrap_or(-1)) as BoxedValue));
            Some(border.upcast())
        },
        false,
    );
    let number: BoxedValue = Rc::new(42_i32);

    assert!(nullable.match_(Some(&number)));
    assert!(nullable.match_(None));
    assert!(!nullable.match_(boxed_str("foo").as_ref()));
    let built = nullable.build(&Some(number)).unwrap();
    let tag = built.tag().unwrap();
    let tag: &dyn AnyValue = &*tag;
    assert_eq!(tag.downcast_ref::<i32>(), Some(&42));

    let borders = FuncDataTemplate::for_type::<Ref<Border>>(|_, _| Some(Decorator::new().upcast()), false);
    let border = Control::boxed(Border::new());
    let canvas = Control::boxed(Canvas::new());

    assert!(borders.match_(Some(&border)));
    assert!(!borders.match_(Some(&canvas)));
    assert!(!borders.match_(None));
    assert!(borders.build(&Some(border)).unwrap().is::<Decorator>());
}

#[test]
fn default_data_template_builds_nothing_for_null() {
    let template = FuncDataTemplate::default_template();

    assert!(template.match_(None));
    assert!(template.build(&None).is_none());
}

struct UntypedTemplate;

impl ITemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>> for UntypedTemplate {
    fn build(&self, _param: &Option<BoxedValue>) -> Option<Ref<Control>> {
        None
    }
}

impl IDataTemplate for UntypedTemplate {
    fn match_(&self, _data: Option<&BoxedValue>) -> bool {
        true
    }

    fn as_typed_data_template(&self) -> Option<&dyn ITypedDataTemplate> {
        Some(self)
    }
}

impl ITypedDataTemplate for UntypedTemplate {
    fn data_type(&self) -> Option<TypeId> {
        None
    }
}

#[test]
#[should_panic(expected = "must have a DataType set")]
fn data_templates_rejects_typed_template_without_data_type() {
    let templates = DataTemplates::new();
    templates.add(Rc::new(UntypedTemplate));
}
