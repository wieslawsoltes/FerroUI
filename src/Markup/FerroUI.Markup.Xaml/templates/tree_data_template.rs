//! Port of `Templates/TreeDataTemplate.cs`.

use super::data_template::is_instance_of_type;
use super::TemplateContent;
use ferroui_base::data::core::ValueType;
use ferroui_base::data::BindingBase;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{ferro_markup_type, BoxedValue, FerroObject, FerroProperty, Ref};
use ferroui_controls::templates::{IDataTemplate, ITemplateWithParam, ITreeDataTemplate, ITypedDataTemplate};
use ferroui_controls::Control;
use std::any::TypeId;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

/// A data template for hierarchical data, as markup declares it: besides
/// the control of an item it describes where the children of the item come
/// from.
pub struct TreeDataTemplate {
    this: Weak<TreeDataTemplate>,
    data_type: RefCell<Option<ValueType>>,
    content: RefCell<Option<BoxedValue>>,
    items_source: RefCell<Option<Rc<dyn BindingBase>>>,
}

impl TreeDataTemplate {
    pub fn new() -> Rc<Self> {
        Rc::new_cyclic(|this| Self {
            this: this.clone(),
            data_type: RefCell::new(None),
            content: RefCell::new(None),
            items_source: RefCell::new(None),
        })
    }

    /// The type of data the template is for. `None` matches all data.
    pub fn data_type(&self) -> Option<ValueType> {
        *self.data_type.borrow()
    }

    pub fn set_data_type(&self, value: Option<ValueType>) {
        *self.data_type.borrow_mut() = value;
    }

    /// The template content: deferred content built for each item.
    pub fn content(&self) -> Option<BoxedValue> {
        self.content.borrow().clone()
    }

    pub fn set_content(&self, value: Option<BoxedValue>) {
        *self.content.borrow_mut() = value;
    }

    /// The binding that selects the children of an item. It is assigned,
    /// not applied: [`bind_children`](Self::bind_children) instantiates it
    /// on the control that shows the children.
    pub fn items_source(&self) -> Option<Rc<dyn BindingBase>> {
        self.items_source.borrow().clone()
    }

    pub fn set_items_source(&self, value: Option<Rc<dyn BindingBase>>) {
        *self.items_source.borrow_mut() = value;
    }

    /// Whether the template is for `data`.
    pub fn match_(&self, data: Option<&BoxedValue>) -> bool {
        match self.data_type() {
            None => true,
            Some(data_type) => is_instance_of_type(data_type, data),
        }
    }

    /// Binds the children of `item` to `target_property` of `target`.
    pub fn bind_children(
        &self,
        target: &FerroObject,
        target_property: &'static FerroProperty,
        _item: &BoxedValue,
    ) -> Rc<dyn IDisposable> {
        match self.items_source() {
            Some(items_source) => target.bind_binding(target_property, &*items_source),
            None => Disposable::empty(),
        }
    }

    /// Builds the control for `data` and gives it the data as its data
    /// context.
    pub fn build(&self, data: Option<&BoxedValue>) -> Option<Ref<Control>> {
        let visual_tree_for_item =
            TemplateContent::load(self.content().as_ref()).map(|result| result.result().clone());
        if let Some(visual_tree_for_item) = &visual_tree_for_item {
            visual_tree_for_item.set_data_context(data.cloned());
        }

        visual_tree_for_item
    }

    /// The template behind a data template handle, if it is one of this
    /// class (the cast of the contract back to the class).
    pub fn from_data_template(template: &Rc<dyn IDataTemplate>) -> Option<Rc<TreeDataTemplate>> {
        template.as_any()?.downcast_ref::<TreeDataTemplate>()?.this.upgrade()
    }

    /// The template as the data template contract.
    pub fn as_data_template(&self) -> Rc<dyn IDataTemplate> {
        self.this.upgrade().expect("the template is alive while it is used")
    }
}

impl ITemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>> for TreeDataTemplate {
    fn build(&self, param: &Option<BoxedValue>) -> Option<Ref<Control>> {
        TreeDataTemplate::build(self, param.as_ref())
    }
}

impl IDataTemplate for TreeDataTemplate {
    fn match_(&self, data: Option<&BoxedValue>) -> bool {
        TreeDataTemplate::match_(self, data)
    }

    fn as_typed_data_template(&self) -> Option<&dyn ITypedDataTemplate> {
        Some(self)
    }

    fn as_tree_data_template(&self) -> Option<&dyn ITreeDataTemplate> {
        Some(self)
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

impl ITreeDataTemplate for TreeDataTemplate {
    fn bind_children(
        &self,
        target: &FerroObject,
        target_property: &'static FerroProperty,
        item: &BoxedValue,
    ) -> Rc<dyn IDisposable> {
        TreeDataTemplate::bind_children(self, target, target_property, item)
    }
}

impl ITypedDataTemplate for TreeDataTemplate {
    fn data_type(&self) -> Option<TypeId> {
        TreeDataTemplate::data_type(self).map(|data_type| data_type.id())
    }
}

crate::identity_eq!(TreeDataTemplate);

ferro_markup_type!(class TreeDataTemplate {
    this: Rc<TreeDataTemplate>,
    handles: [TreeDataTemplate, Rc<TreeDataTemplate>, Option<Rc<TreeDataTemplate>>],
    interfaces: [Rc<dyn ITreeDataTemplate>, Rc<dyn ITypedDataTemplate>, Rc<dyn IDataTemplate>],
    constructors: [() => TreeDataTemplate::new],
    content: Content,
    properties: [
        DataType: Option<ValueType> {
            get: |this: &Rc<TreeDataTemplate>| TreeDataTemplate::data_type(this),
            set: TreeDataTemplate::set_data_type
        } [DataType],
        Content: Option<BoxedValue> { get: TreeDataTemplate::content, set: TreeDataTemplate::set_content }
            [TemplateContent],
        ItemsSource: Option<Rc<dyn BindingBase>> {
            get: TreeDataTemplate::items_source,
            set: TreeDataTemplate::set_items_source
        } [AssignBinding],
    ],
    methods: [
        fn Match(Option<BoxedValue>) -> bool => |this: &Rc<TreeDataTemplate>, data: Option<BoxedValue>| {
            TreeDataTemplate::match_(this, data.as_ref())
        },
        try fn Build(Option<BoxedValue>) -> Option<Ref<Control>> =>
            |this: &Rc<TreeDataTemplate>, data: Option<BoxedValue>| {
                let built = TemplateContent::try_load_as::<Ref<Control>>(this.content().as_ref()).map(|result| result.map(|result| result.result().clone()))?;
                if let Some(built) = &built {
                    built.set_data_context(data);
                }
                Ok::<_, crate::XamlLoadException>(built)
            },
    ],
});
