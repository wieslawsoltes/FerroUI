//! Tests of template content and of the templates of markup.

use super::*;
use crate::test_support::{boxed, TestServiceProvider};
use crate::xaml_il::runtime::{DeferredContentBuilder, XamlIlRuntimeHelpers};
use crate::{register_types, ServiceProviderExtensions};
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingBase;
use ferroui_base::metadata::{attributes, from_markup_value, into_markup_value, MarkupAttributeValue, MarkupTyped};
use ferroui_base::styling::ITemplate;
use ferroui_base::{BoxedValue, FerroProperty, Ref, StaticType};
use ferroui_controls::chrome::{IWindowDrawnDecorationsTemplate, WindowDrawnDecorationsContent};
use ferroui_controls::primitives::TemplatedControl;
use ferroui_controls::templates::{IControlTemplate, IDataTemplate, ITemplateOf};
use ferroui_controls::{Border, Button, Control, Panel, StackPanel, TextBlock};
use std::cell::Cell;
use std::rc::Rc;

/// Deferred content as the runtime helpers create it for a template body.
fn content<T: 'static>(build: impl Fn() -> Option<BoxedValue> + 'static) -> Option<BoxedValue> {
    let root = Border::new();
    let provider = TestServiceProvider::new().with_parents(vec![boxed(root.clone())]).with_root(Some(boxed(root))).sp();
    let builder = DeferredContentBuilder::new(move |_| build());
    let deferred: BoxedValue = XamlIlRuntimeHelpers::deferred_transformation_factory_v3::<T>(builder, &provider);
    Some(deferred)
}

fn named_text_block() -> Option<BoxedValue> {
    let root = Border::new();
    let provider = TestServiceProvider::new().with_parents(vec![]).with_root(Some(boxed(root))).sp();
    let builder = DeferredContentBuilder::new(|sp| {
        let text = TextBlock::new();
        sp.get_name_scope().unwrap().register("PART_Text", text.clone().upcast());
        Some(boxed(text))
    });
    let deferred: BoxedValue = XamlIlRuntimeHelpers::deferred_transformation_factory_v3::<Ref<Control>>(builder, &provider);
    Some(deferred)
}

#[test]
fn loading_no_content_gives_nothing() {
    assert!(TemplateContent::load(None).is_none());
    assert!(TemplateContent::load_as::<Ref<Panel>>(None).is_none());
    // Content that builds nothing gives nothing.
    assert!(TemplateContent::load(content::<Ref<Control>>(|| None).as_ref()).is_none());
}

#[test]
fn loading_content_gives_the_result_and_its_name_scope() {
    let deferred = named_text_block();
    let first = TemplateContent::load(deferred.as_ref()).unwrap();
    let second = TemplateContent::load(deferred.as_ref()).unwrap();

    assert!(first.result().is::<TextBlock>());
    assert!(first.result() != second.result());
    assert!(first.name_scope().find("PART_Text").unwrap().ptr_eq(first.result()));
    assert!(second.name_scope().find("PART_Text").unwrap().ptr_eq(second.result()));
    assert!(first.name_scope().is_completed());
}

#[test]
fn loading_content_as_a_more_specific_type() {
    let deferred = content::<Ref<Panel>>(|| Some(boxed(StackPanel::new())));
    let result = TemplateContent::load_as::<Ref<Panel>>(deferred.as_ref()).unwrap();
    assert!(result.result().is::<StackPanel>());
    // The handle of the object itself is accepted as content too.
    let deferred = crate::object_casts::rc_of::<crate::xaml_il::runtime::DeferredContent>(deferred.as_ref().unwrap()).unwrap();
    assert!(TemplateContent::load(Some(&boxed(deferred))).is_some());
}

#[test]
#[should_panic(expected = "Unexpected content")]
fn loading_anything_else_panics() {
    let _ = TemplateContent::load(Some(&boxed("text".to_string())));
}

#[test]
#[should_panic(expected = "Unable to cast object of type")]
fn loading_content_of_another_type_panics() {
    let deferred = content::<Ref<Control>>(|| Some(boxed(Button::new())));
    let _ = TemplateContent::load_as::<Ref<Panel>>(deferred.as_ref());
}

#[test]
fn control_template_builds_its_content_for_a_control() {
    register_types();
    let template = ControlTemplate::new();
    let control = TemplatedControl::new();
    assert!(template.build(&control).is_none());
    assert!(template.target_type().is_none());

    template.set_content(named_text_block());
    template.set_target_type(Some(<Button as StaticType>::TYPE));
    assert!(std::ptr::eq(template.target_type().unwrap(), <Button as StaticType>::TYPE));

    let result = template.build(&control).unwrap();
    assert!(result.result().is::<TextBlock>());
    assert!(result.name_scope().find("PART_Text").is_some());

    // Through the contract, and applied to a control.
    let contract: Rc<dyn IControlTemplate> = template.as_control_template();
    assert!(contract.build(&control).is_some());
    control.set_template(Some(contract));
    control.apply_template();
    assert_eq!(control.visual_children_count(), 1);
}

#[test]
fn control_template_metadata_builds_through_untyped_calls() {
    register_types();
    let markup = <ControlTemplate as MarkupTyped>::MARKUP;
    assert_eq!(markup.namespace(), "FerroUI.Markup.Xaml.Templates");
    assert_eq!(markup.content_property, Some("Content"));
    let template = (markup.constructors[0].invoke)(&[]).unwrap();

    let content_property = markup.find_property("Content").unwrap();
    assert_eq!(content_property.attributes[0].name, attributes::TEMPLATE_CONTENT);
    (content_property.set.unwrap())(&[template.clone(), named_text_block()]).unwrap();
    let target_type = markup.find_property("TargetType").unwrap();
    (target_type.set.unwrap())(&[template.clone(), into_markup_value(<Button as StaticType>::TYPE)]).unwrap();

    let build = markup.find_methods("Build").next().unwrap();
    let built = (build.invoke)(&[template.clone(), into_markup_value(TemplatedControl::new())]).unwrap();
    assert!(from_markup_value::<Ref<Control>>(&built).unwrap().is::<TextBlock>());

    // The template is assignable to a property typed with the contract.
    assert!(from_markup_value::<Rc<dyn IControlTemplate>>(&template).is_some());
    assert!(from_markup_value::<Option<Rc<dyn IControlTemplate>>>(&template).unwrap().is_some());
}

#[test]
fn data_template_matches_by_data_type_and_builds() {
    let template = DataTemplate::new();
    // Without a data type everything matches, null included.
    assert!(template.match_(None));
    assert!(template.match_(Some(&boxed(1i32))));

    template.set_data_type(Some(ValueType::of::<String>()));
    assert!(template.match_(Some(&boxed("text".to_string()))));
    assert!(!template.match_(Some(&boxed(1i32))));
    assert!(!template.match_(None));

    // A class matches the handles of derived classes.
    template.set_data_type(Some(ValueType::of::<Ref<Control>>()));
    assert!(template.match_(Some(&boxed(Button::new()))));
    assert!(!template.match_(Some(&boxed("text".to_string()))));

    assert!(template.build(Some(&boxed(1i32))).is_none());
    let built = Rc::new(Cell::new(0));
    let counter = built.clone();
    template.set_content(content::<Ref<Control>>(move || {
        counter.set(counter.get() + 1);
        Some(boxed(TextBlock::new()))
    }));
    assert!(template.build(Some(&boxed(1i32))).unwrap().is::<TextBlock>());
    assert_eq!(built.get(), 1);

    // An existing control is reused.
    let existing: Ref<Control> = Button::new().upcast();
    let reused = template.build_with_existing(Some(&boxed(1i32)), Some(existing.clone())).unwrap();
    assert!(reused == existing);
    assert_eq!(built.get(), 1);
}

#[test]
fn data_template_implements_the_data_template_contracts() {
    register_types();
    let template = DataTemplate::new();
    template.set_data_type(Some(ValueType::of::<i32>()));
    template.set_content(content::<Ref<Control>>(|| Some(boxed(TextBlock::new()))));

    let contract: Rc<dyn IDataTemplate> = template.as_data_template();
    assert!(contract.match_(Some(&boxed(1i32))));
    assert!(contract.build(&Some(boxed(1i32))).is_some());
    assert_eq!(contract.as_typed_data_template().unwrap().data_type(), Some(std::any::TypeId::of::<i32>()));
    let recycling = contract.as_recycling_data_template().unwrap();
    let existing: Ref<Control> = Border::new().upcast();
    assert!(recycling.build_with_existing(None, Some(existing.clone())).unwrap() == existing);
    assert!(contract.as_tree_data_template().is_none());

    let markup = <DataTemplate as MarkupTyped>::MARKUP;
    assert_eq!(markup.find_property("DataType").unwrap().attributes[0].name, attributes::DATA_TYPE);
    let untyped = (markup.constructors[0].invoke)(&[]).unwrap();
    let data_type = markup.find_property("DataType").unwrap();
    (data_type.set.unwrap())(&[untyped.clone(), into_markup_value(ValueType::of::<String>())]).unwrap();
    let matches = markup.find_methods("Match").next().unwrap();
    let result = (matches.invoke)(&[untyped.clone(), into_markup_value("x".to_string())]).unwrap().unwrap();
    assert_eq!(result.downcast_ref::<bool>(), Some(&true));
    assert!(from_markup_value::<Rc<dyn IDataTemplate>>(&untyped).is_some());
    assert_eq!(markup.find_methods("Build").count(), 2);
}

#[test]
fn tree_data_template_builds_with_the_data_as_data_context_and_binds_children() {
    register_types();
    let template = TreeDataTemplate::new();
    assert!(template.match_(None));
    template.set_data_type(Some(ValueType::of::<String>()));
    assert!(!template.match_(Some(&boxed(1i32))));
    assert!(template.build(Some(&boxed("node".to_string()))).is_none());

    template.set_content(content::<Ref<Control>>(|| Some(boxed(TextBlock::new()))));
    let item = boxed("node".to_string());
    let built = template.build(Some(&item)).unwrap();
    assert_eq!(built.data_context().unwrap().downcast_ref::<String>().unwrap(), "node");

    // Without an items source nothing is bound.
    let target = Border::new();
    let property: &'static FerroProperty = Control::tag_property();
    template.bind_children(&target, property, &item).dispose();
    assert!(target.tag().is_none());

    // The items source binding is instantiated on the target.
    target.resources().add("children", Some(boxed("the children".to_string())));
    let source = crate::markup_extensions::DynamicResourceExtension::with_resource_key(Some(boxed("children".to_string())));
    let source: Rc<dyn BindingBase> = source;
    template.set_items_source(Some(source));
    let subscription = template.bind_children(&target, property, &item);
    assert_eq!(target.tag().unwrap().downcast_ref::<String>().unwrap(), "the children");
    subscription.dispose();

    let contract: Rc<dyn IDataTemplate> = template.as_data_template();
    assert!(contract.as_tree_data_template().is_some());
    assert!(contract.as_typed_data_template().is_some());
    assert!(contract.as_recycling_data_template().is_none());

    let markup = <TreeDataTemplate as MarkupTyped>::MARKUP;
    let items_source = markup.find_property("ItemsSource").unwrap();
    assert_eq!(items_source.attributes[0].name, attributes::ASSIGN_BINDING);
    let untyped = (markup.constructors[0].invoke)(&[]).unwrap();
    assert_eq!((items_source.get.unwrap())(&[untyped.clone()]).unwrap(), None);
    // A binding is assigned as its concrete object.
    let binding: BoxedValue = crate::markup_extensions::DynamicResourceExtension::new();
    (items_source.set.unwrap())(&[untyped.clone(), Some(binding)]).unwrap();
    assert!((items_source.get.unwrap())(&[untyped.clone()]).unwrap().is_some());
    let binding = crate::markup_extensions::ReflectionBindingExtension::with_path("Children")
        .provide_value(&TestServiceProvider::new().sp());
    (items_source.set.unwrap())(&[untyped.clone(), into_markup_value(binding.clone())]).unwrap();
    let read = (items_source.get.unwrap())(&[untyped.clone()]).unwrap();
    let expected: Rc<dyn BindingBase> = binding;
    assert!(from_markup_value::<Rc<dyn BindingBase>>(&read).unwrap() == expected);
    (items_source.set.unwrap())(&[untyped.clone(), None]).unwrap();
    assert_eq!((items_source.get.unwrap())(&[untyped]), Ok(None));
}

#[test]
fn items_panel_template_builds_a_panel() {
    register_types();
    let template = ItemsPanelTemplate::new();
    assert!(template.build().is_none());
    template.set_content(content::<Ref<Control>>(|| Some(boxed(StackPanel::new()))));
    assert!(template.build().unwrap().is::<StackPanel>());

    let typed: Rc<dyn ITemplateOf<Option<Ref<Panel>>>> = template.as_template();
    assert!(typed.build_typed().is_some());
    // The untyped build holds the value type of the panel template properties.
    assert!(ITemplate::build(&*template).is::<Option<Ref<Panel>>>());

    let markup = <ItemsPanelTemplate as MarkupTyped>::MARKUP;
    assert!(markup.find_attribute(attributes::CONTROL_TEMPLATE_SCOPE).is_some());
    let untyped: BoxedValue = template;
    assert!(from_markup_value::<Rc<dyn ITemplateOf<Option<Ref<Panel>>>>>(&Some(untyped.clone())).is_some());
    let build = markup.find_methods("Build").next().unwrap();
    assert!((build.invoke)(&[Some(untyped)]).unwrap().is_some());
}

#[test]
#[should_panic(expected = "to type 'Panel'")]
fn items_panel_template_rejects_content_that_is_not_a_panel() {
    let template = ItemsPanelTemplate::new();
    template.set_content(content::<Ref<Control>>(|| Some(boxed(Button::new()))));
    let _ = template.build();
}

#[test]
fn template_and_focus_adorner_template_build_a_control() {
    register_types();
    let template = Template::new();
    assert!(template.build().is_none());
    template.set_content(content::<Ref<Control>>(|| Some(boxed(Border::new()))));
    assert!(template.build().unwrap().is::<Border>());
    assert!(template.as_template().build_typed().is_some());
    assert!(ITemplate::build(&*template).is::<Option<Ref<Control>>>());

    let adorner = FocusAdornerTemplate::new();
    assert!(adorner.as_template().build_typed().is_none());
    // The members of the base class.
    adorner.set_content(content::<Ref<Control>>(|| Some(boxed(Border::new()))));
    assert!(adorner.content().is_some());
    assert!(adorner.as_template().build_typed().unwrap().is::<Border>());
    assert!(ITemplate::build(&*adorner).is::<Option<Ref<Control>>>());

    // As the value of the focus adorner property of a control.
    let untyped: BoxedValue = adorner;
    let value = from_markup_value::<Option<Rc<dyn ITemplateOf<Option<Ref<Control>>>>>>(&Some(untyped.clone())).unwrap();
    let control = Button::new();
    control.set_focus_adorner(value);
    assert!(control.focus_adorner().unwrap().build_typed().is_some());

    // The inherited content property is reached through the base class.
    let base = <Template as MarkupTyped>::MARKUP;
    let markup = <FocusAdornerTemplate as MarkupTyped>::MARKUP;
    assert_eq!((markup.base.unwrap())(), ValueType::of::<Rc<Template>>());
    let content_property = base.find_property("Content").unwrap();
    assert!((content_property.get.unwrap())(&[Some(untyped)]).unwrap().is_some());
    assert!((markup.constructors[0].invoke)(&[]).unwrap().is_some());
}

#[test]
fn window_drawn_decorations_template_builds_its_content() {
    register_types();
    let template = WindowDrawnDecorationsTemplate::new();
    template.set_content(content::<Ref<WindowDrawnDecorationsContent>>(|| Some(boxed(WindowDrawnDecorationsContent::new()))));

    let first = template.build();
    let second = template.build();
    assert!(first.result() != second.result());
    let contract: Rc<dyn IWindowDrawnDecorationsTemplate> = template.as_window_drawn_decorations_template();
    assert!(contract.build_typed().name_scope().is_completed());
    assert!(ITemplate::build(&*template).is::<Ref<WindowDrawnDecorationsContent>>());

    let markup = <WindowDrawnDecorationsTemplate as MarkupTyped>::MARKUP;
    assert!(markup.find_attribute(attributes::CONTROL_TEMPLATE_SCOPE).is_some());
    let attribute = &markup.find_property("Content").unwrap().attributes[0];
    assert_eq!(attribute.name, attributes::TEMPLATE_CONTENT);
    assert_eq!(
        attribute.property("TemplateResultType"),
        Some(MarkupAttributeValue::Type(|| ValueType::of::<Ref<WindowDrawnDecorationsContent>>()))
    );
    let untyped: BoxedValue = template;
    assert!(from_markup_value::<Rc<dyn IWindowDrawnDecorationsTemplate>>(&Some(untyped.clone())).is_some());
    let build = markup.find_methods("Build").next().unwrap();
    assert!((build.invoke)(&[Some(untyped)]).unwrap().unwrap().is::<Ref<WindowDrawnDecorationsContent>>());
}

#[test]
#[should_panic(expected = "Operation is not valid")]
fn window_drawn_decorations_template_requires_content() {
    let _ = WindowDrawnDecorationsTemplate::new().build();
}

#[test]
fn template_content_is_loadable_through_its_metadata() {
    register_types();
    let markup = <TemplateContent as MarkupTyped>::MARKUP;
    let load = markup.find_methods("Load").next().unwrap();
    assert!(load.is_static);
    let built = (load.invoke)(&[named_text_block()]).unwrap();
    assert!(from_markup_value::<Ref<Control>>(&built).unwrap().is::<TextBlock>());
    assert_eq!((load.invoke)(&[None]), Ok(None));
}

#[test]
fn data_templates_are_reached_from_their_contract_handle() {
    register_types();
    let template = DataTemplate::new();
    let handle: Rc<dyn IDataTemplate> = template.as_data_template();
    assert!(handle.as_any().unwrap().is::<DataTemplate>());
    assert!(Rc::ptr_eq(&DataTemplate::from_data_template(&handle).unwrap(), &template));
    assert!(TreeDataTemplate::from_data_template(&handle).is_none());

    let tree = TreeDataTemplate::new();
    let handle: Rc<dyn IDataTemplate> = tree.as_data_template();
    assert!(handle.as_any().unwrap().is::<TreeDataTemplate>());
    assert!(Rc::ptr_eq(&TreeDataTemplate::from_data_template(&handle).unwrap(), &tree));
    assert!(DataTemplate::from_data_template(&handle).is_none());

    // Also from the handle a metadata cast produced.
    let untyped: BoxedValue = template.clone();
    let handle = from_markup_value::<Rc<dyn IDataTemplate>>(&Some(untyped)).unwrap();
    assert!(Rc::ptr_eq(&DataTemplate::from_data_template(&handle).unwrap(), &template));
}

// Plain reference types (view models) with a base class and a contract, as
// their markup metadata states them.

trait INamed {}

impl PartialEq for dyn INamed {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}

#[derive(PartialEq)]
struct Person(u8);
#[derive(PartialEq)]
struct Employee(u8);
#[derive(PartialEq)]
struct Manager(u8);
#[derive(PartialEq)]
struct Stranger(u8);

impl INamed for Person {}

ferroui_base::ferro_markup_type!(interface dyn INamed as "INamed" {
    handles: [Rc<dyn INamed>, Option<Rc<dyn INamed>>],
    namespace: "Tests.DataTemplates",
});
ferroui_base::ferro_markup_type!(class Person {
    handles: [Person, Rc<Person>, Option<Rc<Person>>],
    namespace: "Tests.DataTemplates",
    interfaces: [Rc<dyn INamed>],
});
ferroui_base::ferro_markup_type!(class Employee {
    handles: [Employee, Rc<Employee>, Option<Rc<Employee>>],
    namespace: "Tests.DataTemplates",
    base: Rc<Person>,
});
ferroui_base::ferro_markup_type!(class Manager {
    handles: [Manager, Rc<Manager>, Option<Rc<Manager>>],
    namespace: "Tests.DataTemplates",
    base: Rc<Employee>,
});
ferroui_base::ferro_markup_type!(class Stranger {
    handles: [Stranger, Rc<Stranger>, Option<Rc<Stranger>>],
    namespace: "Tests.DataTemplates",
});

#[test]
fn data_template_matches_plain_reference_types_and_types_derived_from_them() {
    use ferroui_base::metadata::MarkupType;

    register_types();
    MarkupType::register_all(&[
        <dyn INamed as MarkupTyped>::MARKUP,
        <Person as MarkupTyped>::MARKUP,
        <Employee as MarkupTyped>::MARKUP,
        <Manager as MarkupTyped>::MARKUP,
        <Stranger as MarkupTyped>::MARKUP,
    ]);

    // Data in both boxing forms: the object itself and its shared handle.
    fn forms<T: PartialEq + 'static>(value: T) -> [BoxedValue; 2] {
        let shared = Rc::new(value);
        let object: BoxedValue = shared.clone();
        let handle: BoxedValue = Rc::new(shared);
        [object, handle]
    }

    let template = DataTemplate::new();
    let tree = TreeDataTemplate::new();
    // The data type named by either handle of the type.
    for data_type in [ValueType::of::<Person>(), ValueType::of::<Rc<Person>>()] {
        template.set_data_type(Some(data_type));
        tree.set_data_type(Some(data_type));
        for data in forms(Person(1)).iter().chain(&forms(Employee(2))).chain(&forms(Manager(3))) {
            assert!(template.match_(Some(data)), "{} for {data_type}", (**data).type_name());
            assert!(tree.match_(Some(data)), "{} for {data_type}", (**data).type_name());
        }
        for data in forms(Stranger(4)).iter().chain(&[boxed(5i32), boxed("text".to_string())]) {
            assert!(!template.match_(Some(data)), "{} for {data_type}", (**data).type_name());
        }
        assert!(!template.match_(None));
    }

    // A derived data type does not match its base.
    template.set_data_type(Some(ValueType::of::<Employee>()));
    assert!(template.match_(Some(&forms(Employee(1))[0])));
    assert!(template.match_(Some(&forms(Manager(1))[1])));
    assert!(!template.match_(Some(&forms(Person(1))[0])));

    // A contract matches the types that implement it, and the ones derived from them.
    template.set_data_type(Some(ValueType::of::<Rc<dyn INamed>>()));
    assert!(template.match_(Some(&forms(Person(1))[0])));
    assert!(template.match_(Some(&forms(Manager(1))[1])));
    assert!(!template.match_(Some(&forms(Stranger(1))[0])));
}

// --- Templates as the values of setters, assigned through the metadata of `Setter` in
// --- every form a template value arrives in, and applied by a style.

#[test]
fn a_control_template_is_the_value_of_a_setter_of_the_template_property() {
    use ferroui_base::styling::testing::try_attach;
    use ferroui_base::styling::{Selectors, Setter, Style};

    register_types();
    let template = ControlTemplate::new();
    template.set_content(content::<Ref<Control>>(|| Some(boxed(Border::new()))));
    let contract: Rc<dyn IControlTemplate> = template.as_control_template();
    let object: BoxedValue = template.clone();
    let forms: Vec<(&str, Option<BoxedValue>)> = vec![
        ("handle", Some(boxed(template.clone()))),
        ("object", Some(object)),
        ("contract", Some(boxed(contract.clone()))),
        ("nullable contract", Some(boxed(Some(contract.clone())))),
    ];
    let value = <Setter as MarkupTyped>::MARKUP.find_property("Value").unwrap();
    for (form, template_value) in forms {
        let setter = Setter::empty();
        setter.set_property(Some(TemplatedControl::template_property().as_property()));
        (value.set.unwrap())(&[into_markup_value(setter.clone()), template_value]).unwrap();

        let style = Style::with_selector(Selectors::of_type::<Button>());
        style.setters().add(setter);
        let button = Button::new();
        try_attach(&style, &button, None);
        let applied = button.template().unwrap_or_else(|| panic!("{form}: the template is not set"));
        assert!(*applied == *contract, "{form}");
    }
}

#[test]
fn a_template_builds_the_value_of_a_setter_of_another_property() {
    use ferroui_base::styling::testing::try_attach;
    use ferroui_base::styling::{Selectors, Setter, Style};
    use ferroui_controls::ContentControl;

    register_types();
    let template = Template::new();
    template.set_content(content::<Ref<Control>>(|| Some(boxed(TextBlock::new()))));
    let value = <Setter as MarkupTyped>::MARKUP.find_property("Value").unwrap();
    let any: Option<BoxedValue> = Some(Rc::new(template.clone()));
    for (form, template_value) in [("handle", boxed(template.clone())), ("any value", Rc::new(any) as BoxedValue)] {
        let setter = Setter::empty();
        setter.set_property(Some(ContentControl::content_property().as_property()));
        (value.set.unwrap())(&[into_markup_value(setter.clone()), Some(template_value)]).unwrap();

        let style = Style::with_selector(Selectors::of_type::<ContentControl>());
        style.setters().add(setter);
        let control = ContentControl::new();
        try_attach(&style, &control, None);
        // The content is what the template built, not the template.
        let content = control.content().unwrap_or_else(|| panic!("{form}: no content"));
        assert!(Control::from_boxed(&content).is_some_and(|built| built.is::<TextBlock>()), "{form}");
    }
}

#[test]
fn a_control_template_is_reached_from_its_contract_handle() {
    let template = ControlTemplate::new();
    let contract: Rc<dyn IControlTemplate> = template.as_control_template();
    let concrete = ControlTemplate::from_control_template(&contract).expect("the concrete template");
    assert!(Rc::ptr_eq(&concrete, &template));
    // A template of another class is not one.
    let other: Rc<dyn IControlTemplate> =
        ferroui_controls::templates::FuncControlTemplate::new(|_, _| Border::new().upcast());
    assert!(ControlTemplate::from_control_template(&other).is_none());
    assert!(other.as_any().is_some_and(|any| any.is::<ferroui_controls::templates::FuncControlTemplate>()));
}

#[test]
fn a_template_is_the_value_of_a_setter_of_a_control_template_property() {
    use ferroui_base::styling::testing::try_attach;
    use ferroui_base::styling::{Selectors, Setter, Style};
    use ferroui_controls::GridSplitter;

    register_types();
    let template = Template::new();
    template.set_content(content::<Ref<Control>>(|| Some(boxed(Border::new()))));
    let value = <Setter as MarkupTyped>::MARKUP.find_property("Value").unwrap();
    let any: Option<BoxedValue> = Some(Rc::new(template.clone()));
    let object: BoxedValue = template.clone();
    for (form, template_value) in
        [("handle", boxed(template.clone())), ("object", object), ("any value", Rc::new(any) as BoxedValue)]
    {
        let setter = Setter::empty();
        setter.set_property(Some(GridSplitter::preview_content_property().as_property()));
        (value.set.unwrap())(&[into_markup_value(setter.clone()), Some(template_value)]).unwrap();

        let style = Style::with_selector(Selectors::of_type::<GridSplitter>());
        style.setters().add(setter);
        let splitter = GridSplitter::new();
        try_attach(&style, &splitter, None);
        // The property holds templates: its value is the template, which builds the control.
        let preview = splitter.preview_content().unwrap_or_else(|| panic!("{form}: no preview content"));
        assert!(preview.build_typed().is::<Border>(), "{form}");
    }
}

