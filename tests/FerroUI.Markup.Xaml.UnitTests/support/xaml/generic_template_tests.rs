//! The test types declared by the upstream test file `Xaml/GenericTemplateTests.cs`.

use std::cell::RefCell;
use std::rc::Rc;

use ferroui_base::data::core::ValueTypes;
use ferroui_base::metadata::MarkupTyped;
use ferroui_base::{
    ferro_class, ferro_class_info, ferro_impl_classes, ferro_markup_type, instantiate, BoxedValue, FerroObjectImpl,
    Ref, StyledElement, StyledElementImpl,
};

use crate::support::TypeModule;

// --- List<SampleTemplatedObject> ---------------------------------------------

/// The list of the children of a [`SampleTemplatedObject`]: the
/// instantiation of the list of the runtime library for it.
pub struct SampleTemplatedObjectList {
    items: RefCell<Vec<Ref<SampleTemplatedObject>>>,
}

crate::test_identity_eq!(SampleTemplatedObjectList);

impl SampleTemplatedObjectList {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { items: RefCell::new(Vec::new()) })
    }

    pub fn add(&self, item: Ref<SampleTemplatedObject>) {
        self.items.borrow_mut().push(item);
    }

    pub fn count(&self) -> usize {
        self.items.borrow().len()
    }

    /// The item at `index`. Panics if out of range.
    pub fn get(&self, index: usize) -> Ref<SampleTemplatedObject> {
        self.items.borrow()[index].clone()
    }
}

ferro_markup_type!(class SampleTemplatedObjectList as "List`1" {
    this: Rc<SampleTemplatedObjectList>,
    handles: [SampleTemplatedObjectList, Rc<SampleTemplatedObjectList>, Option<Rc<SampleTemplatedObjectList>>],
    namespace: "System.Collections.Generic",
    generic: "List`1" [Ref<SampleTemplatedObject>],
    constructors: [() => SampleTemplatedObjectList::new],
    methods: [
        fn Add(Ref<SampleTemplatedObject>) =>
            |list: &Rc<SampleTemplatedObjectList>, item: Ref<SampleTemplatedObject>| list.add(item),
    ],
});

// --- SampleTemplatedObject ---------------------------------------------------

/// A styled element (not a control) that template content can build.
#[repr(C)]
pub struct SampleTemplatedObject {
    base: StyledElement,
    content: RefCell<Rc<SampleTemplatedObjectList>>,
    foo: RefCell<Option<String>>,
}

ferro_class!(SampleTemplatedObject: StyledElement);
ferro_impl_classes!(SampleTemplatedObject: FerroObjectImpl, StyledElementImpl);
ferro_class_info!(SampleTemplatedObject {
    new: SampleTemplatedObject::new,
    markup: {
        namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
        content: Content,
        properties: [
            Content: Rc<SampleTemplatedObjectList> {
                get: |this: &Ref<SampleTemplatedObject>| this.content(),
                set: |this: &Ref<SampleTemplatedObject>, value: Rc<SampleTemplatedObjectList>| this.set_content(value)
            },
            Foo: Option<String> {
                get: |this: &Ref<SampleTemplatedObject>| this.foo(),
                set: |this: &Ref<SampleTemplatedObject>, value: Option<String>| this.set_foo(value)
            },
        ],
    },
});

impl SampleTemplatedObject {
    pub fn construct() -> Self {
        Self {
            base: StyledElement::construct(),
            content: RefCell::new(SampleTemplatedObjectList::new()),
            foo: RefCell::new(None),
        }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }

    pub fn content(&self) -> Rc<SampleTemplatedObjectList> {
        self.content.borrow().clone()
    }

    pub fn set_content(&self, value: Rc<SampleTemplatedObjectList>) {
        *self.content.borrow_mut() = value;
    }

    pub fn foo(&self) -> Option<String> {
        self.foo.borrow().clone()
    }

    pub fn set_foo(&self, value: Option<String>) {
        *self.foo.borrow_mut() = value;
    }
}

// --- SampleTemplatedObjectTemplate -------------------------------------------

/// A template whose content builds a [`SampleTemplatedObject`].
pub struct SampleTemplatedObjectTemplate {
    content: RefCell<Option<BoxedValue>>,
}

crate::test_identity_eq!(SampleTemplatedObjectTemplate);

impl SampleTemplatedObjectTemplate {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { content: RefCell::new(None) })
    }

    pub fn content(&self) -> Option<BoxedValue> {
        self.content.borrow().clone()
    }

    pub fn set_content(&self, value: Option<BoxedValue>) {
        *self.content.borrow_mut() = value;
    }
}

ferro_markup_type!(class SampleTemplatedObjectTemplate {
    this: Rc<SampleTemplatedObjectTemplate>,
    handles: [
        SampleTemplatedObjectTemplate,
        Rc<SampleTemplatedObjectTemplate>,
        Option<Rc<SampleTemplatedObjectTemplate>>,
    ],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
    constructors: [() => SampleTemplatedObjectTemplate::new],
    content: Content,
    properties: [
        Content: Option<BoxedValue> {
            get: |this: &Rc<SampleTemplatedObjectTemplate>| this.content(),
            set: |this: &Rc<SampleTemplatedObjectTemplate>, value: Option<BoxedValue>| this.set_content(value)
        } [TemplateContent(TemplateResultType = type(Ref<SampleTemplatedObject>))],
    ],
});

// --- SampleTemplatedObjectContainer ------------------------------------------

/// The holder of a [`SampleTemplatedObjectTemplate`].
pub struct SampleTemplatedObjectContainer {
    template: RefCell<Option<Rc<SampleTemplatedObjectTemplate>>>,
}

crate::test_identity_eq!(SampleTemplatedObjectContainer);

impl SampleTemplatedObjectContainer {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { template: RefCell::new(None) })
    }

    pub fn template(&self) -> Option<Rc<SampleTemplatedObjectTemplate>> {
        self.template.borrow().clone()
    }

    pub fn set_template(&self, value: Option<Rc<SampleTemplatedObjectTemplate>>) {
        *self.template.borrow_mut() = value;
    }
}

ferro_markup_type!(class SampleTemplatedObjectContainer {
    this: Rc<SampleTemplatedObjectContainer>,
    handles: [
        SampleTemplatedObjectContainer,
        Rc<SampleTemplatedObjectContainer>,
        Option<Rc<SampleTemplatedObjectContainer>>,
    ],
    namespace: "FerroUI.Markup.Xaml.UnitTests.Xaml",
    constructors: [() => SampleTemplatedObjectContainer::new],
    properties: [
        Template: Option<Rc<SampleTemplatedObjectTemplate>> {
            get: |this: &Rc<SampleTemplatedObjectContainer>| this.template(),
            set: |this: &Rc<SampleTemplatedObjectContainer>, value: Option<Rc<SampleTemplatedObjectTemplate>>| {
                this.set_template(value)
            }
        },
    ],
});

pub(crate) const MODULE: TypeModule = TypeModule {
    types: &[SampleTemplatedObject::TYPE],
    markup_types: &[
        <SampleTemplatedObjectList as MarkupTyped>::MARKUP,
        <SampleTemplatedObjectTemplate as MarkupTyped>::MARKUP,
        <SampleTemplatedObjectContainer as MarkupTyped>::MARKUP,
    ],
    value_types: || {
        ValueTypes::register_reference::<SampleTemplatedObjectList>();
        ValueTypes::register_reference::<SampleTemplatedObjectTemplate>();
        ValueTypes::register_reference::<SampleTemplatedObjectContainer>();
    },
};
