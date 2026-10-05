//! Port of `Templates/DataTemplate.cs`.

use super::TemplateContent;
use ferroui_base::data::core::{ValueType, ValueTypes};
use ferroui_base::metadata::MarkupType;
use ferroui_base::{ferro_markup_type, BoxedValue, Ref};
use ferroui_controls::templates::{
    IDataTemplate, IRecyclingDataTemplate, ITemplateWithParam, ITypedDataTemplate,
};
use ferroui_controls::Control;
use std::any::TypeId;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A template that builds the control that presents a piece of data, as
/// markup declares it.
pub struct DataTemplate {
    this: Weak<DataTemplate>,
    data_type: RefCell<Option<ValueType>>,
    content: RefCell<Option<BoxedValue>>,
}

/// `dataType.IsInstanceOfType(data)`: whether the data is of the type or of
/// a type assignable to it. Null is an instance of no type.
pub(crate) fn is_instance_of_type(data_type: ValueType, data: Option<&BoxedValue>) -> bool {
    let Some(data) = data else { return false };
    let actual = ValueType::of_value(&**data);
    if ValueTypes::is_assignable(actual, data_type) {
        return true;
    }

    // Plain (non-class) types: the type of the data, the types it derives
    // from and the contracts they implement, as their markup metadata
    // states them. A type is named by any of its handles (the object or
    // its shared handle).
    let Some(target) = MarkupType::find_by_handle(data_type.id()) else { return false };
    let is_target = |handle: ValueType| {
        handle == data_type || MarkupType::find_by_handle(handle.id()).is_some_and(|m| std::ptr::eq(m, target))
    };
    let mut current = MarkupType::find_by_handle(actual.id());
    // Base chains are short; the bound only guards against a cyclic declaration.
    for _ in 0..64 {
        let Some(markup) = current else { break };
        if std::ptr::eq(markup, target) || markup.interfaces.iter().any(|interface| is_target(interface())) {
            return true;
        }
        current = markup.base.and_then(|base| MarkupType::find_by_handle(base().id()));
    }
    false
}

impl DataTemplate {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            data_type: RefCell::new(None),
            content: RefCell::new(None),
        })
    }

    /// The type of data the template is for: the handle type of a class
    /// (`Ref<T>`), the type of a model object or of a value. `None` matches
    /// all data.
    pub fn data_type(&self) -> Option<ValueType> {
        *self.data_type.borrow()
    }

    pub fn set_data_type(&self, value: Option<ValueType>) {
        *self.data_type.borrow_mut() = value;
    }

    /// The template content: deferred content built for each piece of data.
    pub fn content(&self) -> Option<BoxedValue> {
        self.content.borrow().clone()
    }

    pub fn set_content(&self, value: Option<BoxedValue>) {
        *self.content.borrow_mut() = value;
    }

    /// Whether the template is for `data`.
    pub fn match_(&self, data: Option<&BoxedValue>) -> bool {
        match self.data_type() {
            None => true,
            Some(data_type) => is_instance_of_type(data_type, data),
        }
    }

    /// Builds the control for `data`.
    pub fn build(&self, data: Option<&BoxedValue>) -> Option<Ref<Control>> {
        self.build_with_existing(data, None)
    }

    /// Builds the control for `data`, or reuses `existing`.
    pub fn build_with_existing(&self, _data: Option<&BoxedValue>, existing: Option<Ref<Control>>) -> Option<Ref<Control>> {
        existing.or_else(|| TemplateContent::load(self.content().as_ref()).map(|result| result.result().clone()))
    }

    /// The template behind a data template handle, if it is one of this
    /// class (the cast of the contract back to the class).
    pub fn from_data_template(template: &Rc<dyn IDataTemplate>) -> Option<Rc<DataTemplate>> {
        template.as_any()?.downcast_ref::<DataTemplate>()?.this.upgrade()
    }

    /// The template as the data template contract.
    pub fn as_data_template(&self) -> Rc<dyn IDataTemplate> {
        self.this.upgrade().expect("the template is alive while it is used")
    }
}

impl ITemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>> for DataTemplate {
    fn build(&self, param: &Option<BoxedValue>) -> Option<Ref<Control>> {
        DataTemplate::build(self, param.as_ref())
    }
}

impl IDataTemplate for DataTemplate {
    fn match_(&self, data: Option<&BoxedValue>) -> bool {
        DataTemplate::match_(self, data)
    }

    fn as_recycling_data_template(&self) -> Option<&dyn IRecyclingDataTemplate> {
        Some(self)
    }

    fn as_typed_data_template(&self) -> Option<&dyn ITypedDataTemplate> {
        Some(self)
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

impl IRecyclingDataTemplate for DataTemplate {
    fn build_with_existing(&self, data: Option<&BoxedValue>, existing: Option<Ref<Control>>) -> Option<Ref<Control>> {
        DataTemplate::build_with_existing(self, data, existing)
    }
}

impl ITypedDataTemplate for DataTemplate {
    fn data_type(&self) -> Option<TypeId> {
        DataTemplate::data_type(self).map(|data_type| data_type.id())
    }
}

crate::identity_eq!(DataTemplate);

ferro_markup_type!(class DataTemplate {
    this: Rc<DataTemplate>,
    handles: [DataTemplate, Rc<DataTemplate>, Option<Rc<DataTemplate>>],
    interfaces: [Rc<dyn IRecyclingDataTemplate>, Rc<dyn ITypedDataTemplate>, Rc<dyn IDataTemplate>],
    constructors: [() => DataTemplate::new],
    content: Content,
    properties: [
        DataType: Option<ValueType> {
            get: |this: &Rc<DataTemplate>| DataTemplate::data_type(this),
            set: DataTemplate::set_data_type
        } [DataType],
        Content: Option<BoxedValue> { get: DataTemplate::content, set: DataTemplate::set_content } [TemplateContent],
    ],
    methods: [
        fn Match(Option<BoxedValue>) -> bool => |this: &Rc<DataTemplate>, data: Option<BoxedValue>| {
            DataTemplate::match_(this, data.as_ref())
        },
        try fn Build(Option<BoxedValue>) -> Option<Ref<Control>> => |this: &Rc<DataTemplate>, _data: Option<BoxedValue>| {
            TemplateContent::try_load_as::<Ref<Control>>(this.content().as_ref()).map(|result| result.map(|result| result.result().clone()))
        },
        try fn Build(Option<BoxedValue>, Option<Ref<Control>>) -> Option<Ref<Control>> =>
            |this: &Rc<DataTemplate>, _data: Option<BoxedValue>, existing: Option<Ref<Control>>| {
                match existing {
                    Some(existing) => Ok(Some(existing)),
                    None => TemplateContent::try_load_as::<Ref<Control>>(this.content().as_ref()).map(|result| result.map(|result| result.result().clone())),
                }
            },
    ],
});
