//! Metadata over actual source interfaces, with a recording application implementation.
use crate::*;
use ferroui_base::{
    metadata::{from_markup_value, MarkupAssembly, MarkupTyped},
    BoxedValue,
};
use std::{
    any::Any,
    cell::{Cell, RefCell},
    rc::Rc,
};
struct Provider(RefCell<Option<String>>);
impl IControlRecyclingIdProvider for Provider {
    fn get_control_recycling_id(&self) -> Option<String> {
        self.0.borrow().clone()
    }
}
#[test]
fn id_provider_metadata_owns_the_original_assembly_and_preserves_null_empty_and_shared_values() {
    register_types();
    let metadata = <dyn IControlRecyclingIdProviderMetadata>::MARKUP;
    assert_eq!(
        MarkupAssembly::of_module(metadata.module_path)
            .unwrap()
            .name,
        "FerroDock.Controls.Recycling.Model"
    );
    assert_eq!(
        metadata.full_name(),
        "FerroUI.Controls.Recycling.Model.IControlRecyclingIdProvider"
    );
    let provider = Rc::new(Provider(RefCell::new(None)));
    let model: Rc<dyn IControlRecyclingIdProvider> = provider.clone();
    let value: BoxedValue = Rc::new(model.clone());
    let method = metadata
        .find_methods("GetControlRecyclingId")
        .next()
        .unwrap();
    for expected in [None, Some(String::new()), Some("stable".into())] {
        *provider.0.borrow_mut() = expected.clone();
        let result = (method.invoke)(&[Some(value.clone())]).unwrap();
        assert_eq!(
            from_markup_value::<Option<String>>(&result).unwrap(),
            expected
        );
    }
    assert!(ferroui_base::data::core::ValueTypes::reference_equals(
        &value,
        &(Rc::new(model) as BoxedValue)
    ));
}
type SourceValue = Rc<dyn Any>;
type BuildArguments = (
    Option<SourceValue>,
    Option<SourceValue>,
    Option<SourceValue>,
);
struct Cache {
    enabled: Cell<bool>,
    added: RefCell<Option<(SourceValue, SourceValue)>>,
    built: RefCell<Option<BuildArguments>>,
    clears: Cell<usize>,
}
impl IControlRecycling for Cache {
    fn try_to_use_id_as_key(&self) -> bool {
        self.enabled.get()
    }
    fn set_try_to_use_id_as_key(&self, value: bool) {
        self.enabled.set(value);
    }
    fn try_get_value(&self, _data: Option<SourceValue>) -> Option<SourceValue> {
        None
    }
    fn add(&self, data: SourceValue, control: SourceValue) {
        *self.added.borrow_mut() = Some((data, control));
    }
    fn build(
        &self,
        data: Option<SourceValue>,
        existing: Option<SourceValue>,
        parent: Option<SourceValue>,
    ) -> Option<SourceValue> {
        *self.built.borrow_mut() = Some((data, existing.clone(), parent));
        existing
    }
    fn clear(&self) {
        self.clears.set(self.clears.get() + 1);
    }
}
#[test]
fn recycling_interface_metadata_forwards_properties_arguments_results_and_clear_without_a_cache_copy(
) {
    register_types();
    ferrodock_runtime::register_value_type::<String>();
    let metadata = <dyn IControlRecyclingMetadata>::MARKUP;
    assert_eq!(
        MarkupAssembly::of_module(metadata.module_path)
            .unwrap()
            .name,
        "FerroDock.Controls.Recycling.Model"
    );
    let cache = Rc::new(Cache {
        enabled: Cell::new(false),
        added: RefCell::new(None),
        built: RefCell::new(None),
        clears: Cell::new(0),
    });
    let model: Rc<dyn IControlRecycling> = cache.clone();
    let value: BoxedValue = Rc::new(model);
    (metadata
        .find_property("TryToUseIdAsKey")
        .unwrap()
        .set
        .unwrap())(&[Some(value.clone()), Some(Rc::new(true))])
    .unwrap();
    assert!(cache.enabled.get());
    let data: BoxedValue = Rc::new("data".to_owned());
    let control: BoxedValue = Rc::new("control".to_owned());
    let parent: BoxedValue = Rc::new("parent".to_owned());
    (metadata.find_methods("Add").next().unwrap().invoke)(&[
        Some(value.clone()),
        Some(data.clone()),
        Some(control.clone()),
    ])
    .unwrap();
    let added = cache.added.borrow();
    let (a, b) = added.as_ref().unwrap();
    assert!(Rc::ptr_eq(a, &(data.clone() as SourceValue)));
    assert!(Rc::ptr_eq(b, &(control.clone() as SourceValue)));
    drop(added);
    let result = (metadata.find_methods("Build").next().unwrap().invoke)(&[
        Some(value.clone()),
        Some(data.clone()),
        Some(control.clone()),
        Some(parent.clone()),
    ])
    .unwrap()
    .unwrap();
    assert!(Rc::ptr_eq(&result, &control));
    let built = cache.built.borrow();
    let (a, b, c) = built.as_ref().unwrap();
    assert!(Rc::ptr_eq(a.as_ref().unwrap(), &(data as SourceValue)));
    assert!(Rc::ptr_eq(b.as_ref().unwrap(), &(control as SourceValue)));
    assert!(Rc::ptr_eq(c.as_ref().unwrap(), &(parent as SourceValue)));
    drop(built);
    let result = (metadata.find_methods("Build").next().unwrap().invoke)(&[
        Some(value.clone()),
        None,
        None,
        None,
    ])
    .unwrap();
    assert!(result.is_none());
    (metadata.find_methods("Clear").next().unwrap().invoke)(&[Some(value)]).unwrap();
    assert_eq!(cache.clears.get(), 1);
    // Source TryGetValue has a by-reference out parameter, which is still unported.
    assert!(metadata.find_methods("TryGetValue").next().is_none());
}
