//! Port of `MarkupExtensions/On.cs`.

use ferroui_base::metadata::IAddChild;
use ferroui_base::{ferro_markup_type, BoxedValue};
use std::cell::RefCell;
use std::rc::Rc;

/// One branch of an option markup extension in element syntax:
///
/// ```xml
/// <OnPlatform>
///   <On Options="Windows, Linux">..</On>
/// </OnPlatform>
/// ```
///
/// `TReturn` is the type of the value; the non-generic form of the managed
/// original is `On` with its default type argument (any value).
pub struct On<TReturn = BoxedValue> {
    options: RefCell<Vec<String>>,
    content: RefCell<Option<TReturn>>,
}

impl<TReturn: Clone> On<TReturn> {
    pub fn new() -> Rc<Self> {
        Rc::new(Self { options: RefCell::new(Vec::new()), content: RefCell::new(None) })
    }

    /// The options this branch applies to.
    pub fn options(&self) -> Vec<String> {
        self.options.borrow().clone()
    }

    /// Adds an option (the list of the managed original is filled by the
    /// markup compiler).
    pub fn add_option(&self, option: impl Into<String>) {
        self.options.borrow_mut().push(option.into());
    }

    /// The value of the branch.
    pub fn content(&self) -> Option<TReturn> {
        self.content.borrow().clone()
    }

    pub fn set_content(&self, value: Option<TReturn>) {
        *self.content.borrow_mut() = value;
    }
}

impl<TReturn> PartialEq for On<TReturn> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

ferro_markup_type!(class On as "On" {
    this: Rc<On>,
    handles: [On, Rc<On>, Option<Rc<On>>],
    constructors: [() => On::<BoxedValue>::new],
    content: Content,
    properties: [
        Options: Vec<String> { get: |on: &Rc<On>| on.options() },
        Content: Option<BoxedValue> {
            get: |on: &Rc<On>| on.content(),
            set: |on: &Rc<On>, value: Option<BoxedValue>| on.set_content(value)
        },
    ],
    methods: [
        fn AddOption(String) => |on: &Rc<On>, option: String| on.add_option(option),
    ],
});

/// Carries the metadata of `IAddChild<On>`, the contract of the option markup
/// extensions (`OnPlatform`, `OnFormFactor`): with it markup finds how the
/// `On` children of an extension written as an element are added, as it
/// does for a type without a content property.
#[doc(hidden)]
pub struct AddChildOfOn;

ferro_markup_type!(interface AddChildOfOn as "IAddChild`1" {
    namespace: "FerroUI.Metadata",
    handles: [Rc<dyn IAddChild<Rc<On>>>, Option<Rc<dyn IAddChild<Rc<On>>>>],
    this: Rc<dyn IAddChild<Rc<On>>>,
    generic: "IAddChild`1" [Rc<On>],
    methods: [
        fn AddChild(Rc<On>) => |this: &Rc<dyn IAddChild<Rc<On>>>, child: Rc<On>| this.add_child(child),
    ],
});
