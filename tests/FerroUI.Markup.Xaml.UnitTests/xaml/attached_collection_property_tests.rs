//! Tests this port adds: upstream has none for these forms.
//!
//! An attached property whose static getter creates its collection on demand, the pattern
//! of `Interaction.Behaviors` of the behaviors library (`GetBehaviors` creates and sets the
//! collection when the property has no value):
//!
//! - property-element syntax without a collection element
//!   (`<Border><Owner.Prop><Item/><Item/></Owner.Prop></Border>`) adds the items to the
//!   collection the getter returns (the `AdderSetter` of the compiler: getter, then `Add`);
//! - a style setter whose value is a template that builds a collection gives each styled
//!   element a collection of its own (`PropertySetterTemplateInstance`).

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::styling::ITemplate;
use ferroui_base::{
    ferro_markup_type, ferro_properties, ferro_static_type, AttachedProperty, BoxedValue, FerroObject, FerroProperty,
    Ref, StaticType,
};
use ferroui_controls::{Border, Window};
use ferroui_markup_xaml::templates::TemplateContent;

use crate::support::app::{styled_window_application, xaml_test_base};
use crate::support::loader::load_as;
use crate::support::TypeModule;

// --- test types -------------------------------------------------------------

/// An item of the collection (a behavior of the behaviors library).
pub struct TestBehavior {
    label: RefCell<Option<String>>,
}

crate::test_identity_eq!(TestBehavior);

impl TestBehavior {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { label: RefCell::new(None) })
    }

    pub fn label(&self) -> Option<String> {
        self.label.borrow().clone()
    }

    pub fn set_label(&self, value: Option<String>) {
        *self.label.borrow_mut() = value;
    }
}

ferro_markup_type!(class TestBehavior {
    this: Rc<TestBehavior>,
    handles: [TestBehavior, Rc<TestBehavior>, Option<Rc<TestBehavior>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
    constructors: [() => TestBehavior::new],
    properties: [
        Label: Option<String> {
            get: |this: &Rc<TestBehavior>| this.label(),
            set: |this: &Rc<TestBehavior>, value: Option<String>| this.set_label(value)
        },
    ],
});

/// The collection the attached property holds: a class with an `Add` method, which is what
/// the compiler looks for to add the items of a property element.
pub struct TestBehaviorCollection {
    items: RefCell<Vec<Rc<TestBehavior>>>,
}

crate::test_identity_eq!(TestBehaviorCollection);

impl TestBehaviorCollection {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { items: RefCell::new(Vec::new()) })
    }

    pub fn add(&self, item: Rc<TestBehavior>) {
        self.items.borrow_mut().push(item);
    }

    pub fn count(&self) -> usize {
        self.items.borrow().len()
    }

    /// `this[index]`. Panics if out of range.
    pub fn get(&self, index: usize) -> Rc<TestBehavior> {
        self.items.borrow()[index].clone()
    }

    /// The labels of the items, in order.
    pub fn labels(&self) -> Vec<Option<String>> {
        self.items.borrow().iter().map(|item| item.label()).collect()
    }
}

ferro_markup_type!(class TestBehaviorCollection {
    this: Rc<TestBehaviorCollection>,
    handles: [TestBehaviorCollection, Rc<TestBehaviorCollection>, Option<Rc<TestBehaviorCollection>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
    constructors: [() => TestBehaviorCollection::new],
    properties: [
        Count: i32 { get: |this: &Rc<TestBehaviorCollection>| this.count() as i32 },
    ],
    methods: [
        fn Add(Rc<TestBehavior>) => |this: &Rc<TestBehaviorCollection>, item: Rc<TestBehavior>| this.add(item),
    ],
});

/// A template that builds a collection (`BehaviorCollectionTemplate` of the behaviors
/// library): its content is template content whose result is a collection. The general
/// `Template` builds controls only.
pub struct TestBehaviorCollectionTemplate {
    this: Weak<TestBehaviorCollectionTemplate>,
    content: RefCell<Option<BoxedValue>>,
}

crate::test_identity_eq!(TestBehaviorCollectionTemplate);

impl TestBehaviorCollectionTemplate {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), content: RefCell::new(None) })
    }

    /// The template content: deferred content built on each [`build`](Self::build).
    pub fn content(&self) -> Option<BoxedValue> {
        self.content.borrow().clone()
    }

    pub fn set_content(&self, value: Option<BoxedValue>) {
        *self.content.borrow_mut() = value;
    }

    /// Builds a new collection from the content.
    pub fn build(&self) -> Option<Rc<TestBehaviorCollection>> {
        TemplateContent::load_as::<Rc<TestBehaviorCollection>>(self.content().as_ref())
            .map(|result| result.result().clone())
    }

    fn as_template(&self) -> Rc<dyn ITemplate> {
        self.this.upgrade().expect("the template is alive while it is used")
    }
}

/// The value a setter sets is what the template builds: a value of exactly the type of the
/// property (`Option<Rc<TestBehaviorCollection>>`).
impl ITemplate for TestBehaviorCollectionTemplate {
    fn build(&self) -> BoxedValue {
        Rc::new(TestBehaviorCollectionTemplate::build(self))
    }
}

ferro_markup_type!(class TestBehaviorCollectionTemplate {
    this: Rc<TestBehaviorCollectionTemplate>,
    handles: [
        TestBehaviorCollectionTemplate,
        Rc<TestBehaviorCollectionTemplate>,
        Option<Rc<TestBehaviorCollectionTemplate>>,
    ],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
    interfaces: [Rc<dyn ITemplate>],
    constructors: [() => TestBehaviorCollectionTemplate::new],
    content: Content,
    properties: [
        Content: Option<BoxedValue> {
            get: |this: &Rc<TestBehaviorCollectionTemplate>| this.content(),
            set: |this: &Rc<TestBehaviorCollectionTemplate>, value: Option<BoxedValue>| this.set_content(value)
        } [TemplateContent(TemplateResultType = type(Rc<TestBehaviorCollection>))],
    ],
});

/// The owner of the attached property `Behaviors` (`Interaction` of the behaviors library).
pub struct TestInteraction;

ferro_static_type!(TestInteraction);

ferro_properties! {
    impl TestInteraction {
        pub fn behaviors_property() -> AttachedProperty<Option<Rc<TestBehaviorCollection>>> {
            FerroProperty::register_attached::<TestInteraction, FerroObject, _>("Behaviors", None)
        }
    }
}

impl TestInteraction {
    /// The collection of `target`, created and set when the property has no value.
    pub fn get_behaviors(target: &FerroObject) -> Rc<TestBehaviorCollection> {
        if let Some(behaviors) = target.get_value(Self::behaviors_property()) {
            return behaviors;
        }
        let behaviors = TestBehaviorCollection::new();
        target.set_value(Self::behaviors_property(), Some(behaviors.clone()));
        behaviors
    }

    pub fn set_behaviors(target: &FerroObject, value: Option<Rc<TestBehaviorCollection>>) {
        target.set_value(Self::behaviors_property(), value)
    }
}

ferro_markup_type!(static TestInteraction {
    type_info: TestInteraction,
    namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
    methods: [
        static fn GetBehaviors(Ref<FerroObject>) -> Rc<TestBehaviorCollection> =>
            |target: Ref<FerroObject>| TestInteraction::get_behaviors(&target),
        static fn SetBehaviors(Ref<FerroObject>, Option<Rc<TestBehaviorCollection>>) =>
            |target: Ref<FerroObject>, value: Option<Rc<TestBehaviorCollection>>| {
                TestInteraction::set_behaviors(&target, value)
            },
    ],
});

/// The test types of this file.
pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[<TestInteraction as StaticType>::TYPE],
    markup_types: &[
        <TestBehavior as MarkupTyped>::MARKUP,
        <TestBehaviorCollection as MarkupTyped>::MARKUP,
        <TestBehaviorCollectionTemplate as MarkupTyped>::MARKUP,
        <TestInteraction as MarkupTyped>::MARKUP,
    ],
    value_types: || {
        ValueTypes::register_reference::<TestBehavior>();
        ValueTypes::register_reference::<TestBehaviorCollection>();
        ValueTypes::register_reference::<TestBehaviorCollectionTemplate>();
        // The template is a template (`Setter.Value` builds the value with it), whichever
        // form its value is held in: the object itself or its handle.
        ValueTypes::register_cast::<TestBehaviorCollectionTemplate, Rc<dyn ITemplate>>(
            TestBehaviorCollectionTemplate::as_template,
        );
        ValueTypes::register_cast::<Rc<TestBehaviorCollectionTemplate>, Rc<dyn ITemplate>>(
            |template: &Rc<TestBehaviorCollectionTemplate>| -> Rc<dyn ITemplate> { template.clone() },
        );
    },
};

// --- tests ------------------------------------------------------------------

/// Without a collection element the items are added to the collection the getter returns:
/// the getter creates it and sets it as the value of the property.
#[test]
fn property_element_adds_items_to_the_collection_the_getter_creates() {
    let _base = xaml_test_base();
    let xaml = "
<Border xmlns='https://github.com/ferroui'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <local:TestInteraction.Behaviors>
        <local:TestBehavior Label='first'/>
        <local:TestBehavior Label='second'/>
    </local:TestInteraction.Behaviors>
</Border>";
    let border = load_as::<Ref<Border>>(xaml);

    let behaviors =
        border.get_value(TestInteraction::behaviors_property()).expect("the getter created and set the collection");
    assert_eq!(behaviors.labels(), [Some("first".to_string()), Some("second".to_string())]);
    assert!(Rc::ptr_eq(&behaviors, &TestInteraction::get_behaviors(&border)));
}

/// A single item is added too, rather than assigned to the property (it is not a
/// collection).
#[test]
fn property_element_with_one_item_adds_it_to_the_collection_the_getter_creates() {
    let _base = xaml_test_base();
    let xaml = "
<Border xmlns='https://github.com/ferroui'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <local:TestInteraction.Behaviors>
        <local:TestBehavior Label='only'/>
    </local:TestInteraction.Behaviors>
</Border>";
    let border = load_as::<Ref<Border>>(xaml);

    let behaviors =
        border.get_value(TestInteraction::behaviors_property()).expect("the getter created and set the collection");
    assert_eq!(behaviors.labels(), [Some("only".to_string())]);
}

/// With a collection element the collection is the value of the property.
#[test]
fn property_element_with_a_collection_element_sets_the_property() {
    let _base = xaml_test_base();
    let xaml = "
<Border xmlns='https://github.com/ferroui'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <local:TestInteraction.Behaviors>
        <local:TestBehaviorCollection>
            <local:TestBehavior Label='first'/>
            <local:TestBehavior Label='second'/>
        </local:TestBehaviorCollection>
    </local:TestInteraction.Behaviors>
</Border>";
    let border = load_as::<Ref<Border>>(xaml);

    let behaviors = border.get_value(TestInteraction::behaviors_property()).expect("the collection is set");
    assert_eq!(behaviors.labels(), [Some("first".to_string()), Some("second".to_string())]);
}

/// A style setter whose value is a template sets the property of every element it applies
/// to to a collection the template builds for that element: the elements share neither the
/// collection nor its items.
#[test]
fn style_setter_with_a_template_gives_each_element_its_own_collection() {
    let _app = styled_window_application();
    let xaml = "
<Window xmlns='https://github.com/ferroui'
        xmlns:local='clr-namespace:FerroUI.Markup.Xaml.UnitTests.Xaml;assembly=FerroUI.Markup.Xaml.UnitTests'>
    <Window.Styles>
        <Style Selector='Border.behaviors'>
            <Setter Property='local:TestInteraction.Behaviors'>
                <local:TestBehaviorCollectionTemplate>
                    <local:TestBehaviorCollection>
                        <local:TestBehavior Label='styled'/>
                    </local:TestBehaviorCollection>
                </local:TestBehaviorCollectionTemplate>
            </Setter>
        </Style>
    </Window.Styles>
    <StackPanel>
        <Border Name='first' Classes='behaviors'/>
        <Border Name='second' Classes='behaviors'/>
    </StackPanel>
</Window>";
    let window = load_as::<Ref<Window>>(xaml);
    let first = window.get_control::<Border>("first");
    let second = window.get_control::<Border>("second");

    let first_behaviors =
        first.get_value(TestInteraction::behaviors_property()).expect("the style set the collection of 'first'");
    let second_behaviors =
        second.get_value(TestInteraction::behaviors_property()).expect("the style set the collection of 'second'");

    assert_eq!(first_behaviors.labels(), [Some("styled".to_string())]);
    assert_eq!(second_behaviors.labels(), [Some("styled".to_string())]);
    assert!(!Rc::ptr_eq(&first_behaviors, &second_behaviors));
    assert!(!Rc::ptr_eq(&first_behaviors.get(0), &second_behaviors.get(0)));

    // The value is built once per element, and the getter returns it rather than creating
    // a collection.
    let again = first.get_value(TestInteraction::behaviors_property()).expect("the collection of 'first'");
    assert!(Rc::ptr_eq(&first_behaviors, &again));
    assert!(Rc::ptr_eq(&first_behaviors, &TestInteraction::get_behaviors(&first)));
}
