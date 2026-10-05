use super::{FuncDataTemplate, IDataTemplate, IRecyclingDataTemplate, ITemplateWithParam, ITreeDataTemplate};
use crate::Control;
use ferroui_base::controls::NameScopeRef;
use ferroui_base::reactive::{Disposable, IDisposable};
use ferroui_base::{AnyValue, BoxedValue, FerroObject, FerroProperty, Ref};
use std::rc::Rc;

/// A template used to build hierarchical data.
///
/// `items_selector` returns the child items of an item as the untyped value
/// assigned to the target property, so it must hold exactly the value type
/// of that property.
pub struct FuncTreeDataTemplate {
    base: Rc<FuncDataTemplate>,
    items_selector: Box<dyn Fn(&BoxedValue) -> BoxedValue>,
}

impl FuncTreeDataTemplate {
    /// Creates a tree data template.
    pub fn new(
        match_: impl Fn(Option<&BoxedValue>) -> bool + 'static,
        build: impl Fn(&Option<BoxedValue>, &NameScopeRef) -> Option<Ref<Control>> + 'static,
        items_selector: impl Fn(&BoxedValue) -> BoxedValue + 'static,
    ) -> Rc<Self> {
        Rc::new(Self { base: FuncDataTemplate::new(match_, build, false), items_selector: Box::new(items_selector) })
    }

    /// Creates a tree data template that matches data of type `T`.
    pub fn for_type<T: 'static>(
        build: impl Fn(&T, &NameScopeRef) -> Option<Ref<Control>> + 'static,
        items_selector: impl Fn(&T) -> BoxedValue + 'static,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: FuncDataTemplate::for_type::<T>(build, false),
            items_selector: Self::cast_selector(items_selector),
        })
    }

    /// Creates a tree data template that matches data of type `T` accepted
    /// by `match_`.
    pub fn for_type_with_match<T: 'static>(
        match_: impl Fn(&T) -> bool + 'static,
        build: impl Fn(&T, &NameScopeRef) -> Option<Ref<Control>> + 'static,
        items_selector: impl Fn(&T) -> BoxedValue + 'static,
    ) -> Rc<Self> {
        Rc::new(Self {
            base: FuncDataTemplate::for_type_with_match::<T>(match_, build, false),
            items_selector: Self::cast_selector(items_selector),
        })
    }

    fn cast_selector<T: 'static>(
        items_selector: impl Fn(&T) -> BoxedValue + 'static,
    ) -> Box<dyn Fn(&BoxedValue) -> BoxedValue> {
        Box::new(move |item| {
            let value: &dyn AnyValue = &**item;
            match value.downcast_ref::<T>() {
                Some(item) => items_selector(item),
                None => panic!("The item passed to the data template is not of type {}.", std::any::type_name::<T>()),
            }
        })
    }
}

impl ITemplateWithParam<Option<BoxedValue>, Option<Ref<Control>>> for FuncTreeDataTemplate {
    fn build(&self, param: &Option<BoxedValue>) -> Option<Ref<Control>> {
        self.base.build(param)
    }
}

impl IDataTemplate for FuncTreeDataTemplate {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn match_(&self, data: Option<&BoxedValue>) -> bool {
        self.base.match_(data)
    }

    fn as_recycling_data_template(&self) -> Option<&dyn IRecyclingDataTemplate> {
        Some(self)
    }

    fn as_tree_data_template(&self) -> Option<&dyn ITreeDataTemplate> {
        Some(self)
    }
}

impl IRecyclingDataTemplate for FuncTreeDataTemplate {
    fn build_with_existing(&self, data: Option<&BoxedValue>, existing: Option<Ref<Control>>) -> Option<Ref<Control>> {
        self.base.build_with_existing(data, existing)
    }
}

impl ITreeDataTemplate for FuncTreeDataTemplate {
    fn bind_children(
        &self,
        target: &FerroObject,
        target_property: &'static FerroProperty,
        item: &BoxedValue,
    ) -> Rc<dyn IDisposable> {
        let items = (self.items_selector)(item);
        let items: &dyn AnyValue = &*items;
        target.set_current_value_untyped(target_property, items.as_any());
        Disposable::empty()
    }
}
