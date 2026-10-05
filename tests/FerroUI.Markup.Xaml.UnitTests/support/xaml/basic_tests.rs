//! The test types declared by the upstream test file `Xaml/BasicTests.cs`.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use ferroui_base::data::core::ValueTypes;
use ferroui_base::data::model::{Event, INotifyPropertyChanged};
use ferroui_base::metadata::{IAddChild, MarkupTyped};
use ferroui_base::{
    ferro_markup_type, ferro_properties, ferro_static_type, AttachedProperty, BoxedValue, FerroObject, FerroProperty,
    Ref, StaticType,
};

use ferroui_controls::primitives::SelectedItemsList;
use ferroui_controls::ItemsSource;

use crate::support::TypeModule;

type Object = Option<BoxedValue>;

// --- ObjectWithAddChild ------------------------------------------------------

/// A plain object that takes its child through the untyped add-child
/// contract.
pub struct ObjectWithAddChild {
    this: Weak<ObjectWithAddChild>,
    child: RefCell<Object>,
}

crate::test_identity_eq!(ObjectWithAddChild);

impl ObjectWithAddChild {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), child: RefCell::new(None) })
    }

    pub fn child(&self) -> Object {
        self.child.borrow().clone()
    }

    pub fn set_child(&self, value: Object) {
        *self.child.borrow_mut() = value;
    }

    fn as_add_child(&self) -> Rc<dyn IAddChild<BoxedValue>> {
        self.this.upgrade().expect("the object is alive while it is used")
    }
}

impl IAddChild<BoxedValue> for ObjectWithAddChild {
    fn add_child(&self, child: BoxedValue) {
        self.set_child(Some(child));
    }
}

ferro_markup_type!(class ObjectWithAddChild {
    this: Rc<ObjectWithAddChild>,
    handles: [ObjectWithAddChild, Rc<ObjectWithAddChild>, Option<Rc<ObjectWithAddChild>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
    interfaces: [Rc<dyn IAddChild<BoxedValue>>],
    constructors: [() => ObjectWithAddChild::new],
    properties: [
        Child: Option<BoxedValue> {
            get: |this: &Rc<ObjectWithAddChild>| this.child(),
            set: |this: &Rc<ObjectWithAddChild>, value: Object| this.set_child(value)
        },
    ],
});

// --- ObjectWithoutPublicCtor -------------------------------------------------

/// A plain object markup cannot create: a document can only populate an
/// existing instance.
pub struct ObjectWithoutPublicCtor {
    test1: RefCell<Option<String>>,
    test2: RefCell<Option<String>>,
}

crate::test_identity_eq!(ObjectWithoutPublicCtor);

impl ObjectWithoutPublicCtor {
    pub fn new(param: &str) -> Rc<Self> {
        Rc::new(Self { test1: RefCell::new(Some(param.to_string())), test2: RefCell::new(None) })
    }

    pub fn test1(&self) -> Option<String> {
        self.test1.borrow().clone()
    }

    pub fn set_test1(&self, value: Option<String>) {
        *self.test1.borrow_mut() = value;
    }

    pub fn test2(&self) -> Option<String> {
        self.test2.borrow().clone()
    }

    pub fn set_test2(&self, value: Option<String>) {
        *self.test2.borrow_mut() = value;
    }
}

ferro_markup_type!(class ObjectWithoutPublicCtor {
    this: Rc<ObjectWithoutPublicCtor>,
    handles: [ObjectWithoutPublicCtor, Rc<ObjectWithoutPublicCtor>, Option<Rc<ObjectWithoutPublicCtor>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
    constructors: [(String) => |param: String| ObjectWithoutPublicCtor::new(&param)],
    properties: [
        Test1: Option<String> {
            get: |this: &Rc<ObjectWithoutPublicCtor>| this.test1(),
            set: |this: &Rc<ObjectWithoutPublicCtor>, value: Option<String>| this.set_test1(value)
        },
        Test2: Option<String> {
            get: |this: &Rc<ObjectWithoutPublicCtor>| this.test2(),
            set: |this: &Rc<ObjectWithoutPublicCtor>, value: Option<String>| this.set_test2(value)
        },
    ],
});

// --- ObjectWithAddChildOfT ---------------------------------------------------

/// A plain object with both the untyped add-child contract and the one for
/// text: text content must go through the typed one.
pub struct ObjectWithAddChildOfT {
    this: Weak<ObjectWithAddChildOfT>,
    text: RefCell<Option<String>>,
    child: RefCell<Object>,
}

crate::test_identity_eq!(ObjectWithAddChildOfT);

impl ObjectWithAddChildOfT {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self { this: this.clone(), text: RefCell::new(None), child: RefCell::new(None) })
    }

    pub fn text(&self) -> Option<String> {
        self.text.borrow().clone()
    }

    pub fn set_text(&self, value: Option<String>) {
        *self.text.borrow_mut() = value;
    }

    pub fn child(&self) -> Object {
        self.child.borrow().clone()
    }

    pub fn set_child(&self, value: Object) {
        *self.child.borrow_mut() = value;
    }

    fn as_add_child(&self) -> Rc<dyn IAddChild<BoxedValue>> {
        self.this.upgrade().expect("the object is alive while it is used")
    }

    fn as_add_child_of_string(&self) -> Rc<dyn IAddChild<String>> {
        self.this.upgrade().expect("the object is alive while it is used")
    }
}

impl IAddChild<BoxedValue> for ObjectWithAddChildOfT {
    fn add_child(&self, child: BoxedValue) {
        self.set_child(Some(child));
    }
}

impl IAddChild<String> for ObjectWithAddChildOfT {
    fn add_child(&self, child: String) {
        self.set_text(Some(child));
    }
}

ferro_markup_type!(class ObjectWithAddChildOfT {
    this: Rc<ObjectWithAddChildOfT>,
    handles: [ObjectWithAddChildOfT, Rc<ObjectWithAddChildOfT>, Option<Rc<ObjectWithAddChildOfT>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
    interfaces: [Rc<dyn IAddChild<BoxedValue>>, Rc<dyn IAddChild<String>>],
    constructors: [() => ObjectWithAddChildOfT::new],
    properties: [
        Text: Option<String> {
            get: |this: &Rc<ObjectWithAddChildOfT>| this.text(),
            set: |this: &Rc<ObjectWithAddChildOfT>, value: Option<String>| this.set_text(value)
        },
        Child: Option<BoxedValue> {
            get: |this: &Rc<ObjectWithAddChildOfT>| this.child(),
            set: |this: &Rc<ObjectWithAddChildOfT>, value: Object| this.set_child(value)
        },
    ],
});

// --- BasicTestsAttachedPropertyHolder ----------------------------------------

/// The owner of the attached property `Foo`, in the namespace of the tests.
pub struct BasicTestsAttachedPropertyHolder;

ferro_static_type!(BasicTestsAttachedPropertyHolder);

ferro_properties! {
    impl BasicTestsAttachedPropertyHolder {
        pub fn foo_property() -> AttachedProperty<Option<String>> {
            FerroProperty::register_attached::<BasicTestsAttachedPropertyHolder, FerroObject, _>("Foo", None)
        }
    }
}

impl BasicTestsAttachedPropertyHolder {
    pub fn set_foo(target: &FerroObject, value: Option<String>) {
        target.set_value(Self::foo_property(), value)
    }

    pub fn get_foo(target: &FerroObject) -> Option<String> {
        target.get_value(Self::foo_property())
    }
}

ferro_markup_type!(static BasicTestsAttachedPropertyHolder {
    type_info: BasicTestsAttachedPropertyHolder,
    namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
    methods: [
        static fn SetFoo(Ref<FerroObject>, Option<String>) =>
            |target: Ref<FerroObject>, value: Option<String>| BasicTestsAttachedPropertyHolder::set_foo(&target, value),
        static fn GetFoo(Ref<FerroObject>) -> Option<String> =>
            |target: Ref<FerroObject>| BasicTestsAttachedPropertyHolder::get_foo(&target),
    ],
});

// --- SelectedItemsViewModel --------------------------------------------------

/// The view model of the list box test: the items and the list the
/// selection is bound to.
pub struct SelectedItemsViewModel {
    items: RefCell<Option<ItemsSource>>,
    selected_items: RefCell<SelectedItemsList>,
    property_changed: Event<str>,
}

crate::test_identity_eq!(SelectedItemsViewModel);

impl INotifyPropertyChanged for SelectedItemsViewModel {
    fn property_changed(&self) -> &Event<str> {
        &self.property_changed
    }
}

impl SelectedItemsViewModel {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            items: RefCell::new(None),
            selected_items: RefCell::new(SelectedItemsList::new()),
            property_changed: Event::new(),
        })
    }

    pub fn items(&self) -> Option<ItemsSource> {
        self.items.borrow().clone()
    }

    pub fn set_items(&self, value: Option<ItemsSource>) {
        *self.items.borrow_mut() = value;
    }

    pub fn selected_items(&self) -> SelectedItemsList {
        self.selected_items.borrow().clone()
    }

    pub fn set_selected_items(&self, value: SelectedItemsList) {
        *self.selected_items.borrow_mut() = value;
        self.property_changed.raise("SelectedItems");
    }
}

ferro_markup_type!(class SelectedItemsViewModel {
    this: Rc<SelectedItemsViewModel>,
    handles: [SelectedItemsViewModel, Rc<SelectedItemsViewModel>, Option<Rc<SelectedItemsViewModel>>],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
    properties: [
        Items: Option<ItemsSource> {
            get: |this: &Rc<SelectedItemsViewModel>| this.items(),
            set: |this: &Rc<SelectedItemsViewModel>, value: Option<ItemsSource>| this.set_items(value)
        },
        SelectedItems: SelectedItemsList {
            get: |this: &Rc<SelectedItemsViewModel>| this.selected_items(),
            set: |this: &Rc<SelectedItemsViewModel>, value: SelectedItemsList| this.set_selected_items(value)
        },
    ],
    notify_property_changed: SelectedItemsViewModel,
});

pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[<BasicTestsAttachedPropertyHolder as StaticType>::TYPE],
    markup_types: &[
        <ObjectWithAddChild as MarkupTyped>::MARKUP,
        <ObjectWithoutPublicCtor as MarkupTyped>::MARKUP,
        <ObjectWithAddChildOfT as MarkupTyped>::MARKUP,
        <BasicTestsAttachedPropertyHolder as MarkupTyped>::MARKUP,
        <SelectedItemsViewModel as MarkupTyped>::MARKUP,
    ],
    value_types: || {
        ValueTypes::register_reference::<ObjectWithAddChild>();
        ValueTypes::register_reference::<ObjectWithoutPublicCtor>();
        ValueTypes::register_reference::<ObjectWithAddChildOfT>();
        ValueTypes::register_reference::<SelectedItemsViewModel>();
        ValueTypes::register_cast::<ObjectWithAddChild, Rc<dyn IAddChild<BoxedValue>>>(ObjectWithAddChild::as_add_child);
        ValueTypes::register_cast::<ObjectWithAddChildOfT, Rc<dyn IAddChild<BoxedValue>>>(
            ObjectWithAddChildOfT::as_add_child,
        );
        ValueTypes::register_cast::<ObjectWithAddChildOfT, Rc<dyn IAddChild<String>>>(
            ObjectWithAddChildOfT::as_add_child_of_string,
        );
    },
};
