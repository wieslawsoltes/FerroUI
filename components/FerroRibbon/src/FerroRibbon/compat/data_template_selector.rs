//! Uno DataTemplateSelector selection overloads, using native IDataTemplate.
use ferroui_base::{
    BoxedValue, FerroObject, Ref,
    data::core::ValueTypes,
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_controls::templates::IDataTemplate;
use std::rc::Rc;

/// Plain reference class, as in the source: it is not a DependencyObject.
pub struct DataTemplateSelector;
pub trait DataTemplateSelectorImpl: 'static {
    fn select_template_core(&self, _item: Option<BoxedValue>) -> Option<Rc<dyn IDataTemplate>> {
        None
    }
    /// The two-argument source virtual does NOT fall back to the one-argument virtual.
    fn select_template_core_with_container(
        &self,
        _item: Option<BoxedValue>,
        container: Option<Ref<FerroObject>>,
    ) -> Option<Rc<dyn IDataTemplate>> {
        container.expect("ArgumentNullException: container");
        None
    }
}
pub trait DataTemplateSelectorExt: DataTemplateSelectorImpl {
    fn select_template(&self, item: Option<BoxedValue>) -> Option<Rc<dyn IDataTemplate>> {
        self.select_template_core(item)
    }
    fn select_template_with_container(
        &self,
        item: Option<BoxedValue>,
        container: Option<Ref<FerroObject>>,
    ) -> Option<Rc<dyn IDataTemplate>> {
        self.select_template_core_with_container(item, container)
    }
}
impl<T: DataTemplateSelectorImpl + ?Sized> DataTemplateSelectorExt for T {}
impl DataTemplateSelectorImpl for DataTemplateSelector {}
impl DataTemplateSelector {
    pub fn new() -> Rc<Self> {
        Rc::new(Self)
    }
}
impl PartialEq for DataTemplateSelector {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl PartialEq for dyn DataTemplateSelectorImpl {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
    }
}
ferroui_base::ferro_markup_type!(class DataTemplateSelector{
 namespace:"FerroRibbon.Compatibility",this:Rc<dyn DataTemplateSelectorImpl>,handles:[Rc<dyn DataTemplateSelectorImpl>,DataTemplateSelector,Rc<DataTemplateSelector>,Option<Rc<dyn DataTemplateSelectorImpl>>],
 constructors:[()=>DataTemplateSelector::new],
 methods:[fn SelectTemplate(Option<BoxedValue>)->Option<Rc<dyn IDataTemplate>> =>|s:&Rc<dyn DataTemplateSelectorImpl>,item|s.select_template(item),fn SelectTemplate(Option<BoxedValue>,Option<Ref<FerroObject>>)->Option<Rc<dyn IDataTemplate>> =>|s:&Rc<dyn DataTemplateSelectorImpl>,item,container|s.select_template_with_container(item,container)]
});
pub(crate) fn register() {
    ValueTypes::register_reference::<DataTemplateSelector>();
    ValueTypes::register_nullable::<Rc<dyn DataTemplateSelectorImpl>>();
    ValueTypes::register_cast::<Rc<DataTemplateSelector>, Rc<dyn DataTemplateSelectorImpl>>(|v| {
        v.clone()
    });
    ValueTypes::register_cast::<Rc<DataTemplateSelector>, Option<Rc<dyn DataTemplateSelectorImpl>>>(
        |v| Some(v.clone()),
    );
    MarkupType::register_all(&[DataTemplateSelector::MARKUP]);
}
