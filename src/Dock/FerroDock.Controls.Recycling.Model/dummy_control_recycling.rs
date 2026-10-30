use crate::IControlRecycling;
use std::any::Any;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

type CacheEntry = (Rc<dyn Any>, Rc<dyn Any>);

// Like the source fixture, this is test-only. Its keys are the opaque reference
// objects constructed by the contract tests, so equality is reference equality.
#[derive(Default)]
pub(crate) struct DummyControlRecycling {
    use_id: Cell<bool>,
    map: RefCell<Vec<CacheEntry>>,
}

impl IControlRecycling for DummyControlRecycling {
    fn try_to_use_id_as_key(&self) -> bool {
        self.use_id.get()
    }
    fn set_try_to_use_id_as_key(&self, value: bool) {
        self.use_id.set(value);
    }
    fn try_get_value(&self, data: Option<Rc<dyn Any>>) -> Option<Rc<dyn Any>> {
        let data = data?;
        self.map
            .borrow()
            .iter()
            .find(|(key, _)| Rc::ptr_eq(key, &data))
            .map(|(_, control)| control.clone())
    }
    fn add(&self, data: Rc<dyn Any>, control: Rc<dyn Any>) {
        let mut map = self.map.borrow_mut();
        if let Some((_, cached)) = map.iter_mut().find(|(key, _)| Rc::ptr_eq(key, &data)) {
            *cached = control;
        } else {
            map.push((data, control));
        }
    }
    fn build(
        &self,
        data: Option<Rc<dyn Any>>,
        existing: Option<Rc<dyn Any>>,
        _parent: Option<Rc<dyn Any>>,
    ) -> Option<Rc<dyn Any>> {
        if let Some(existing) = existing {
            self.add(
                data.expect("existing content has a data key in the source fixture"),
                existing.clone(),
            );
            return Some(existing);
        }
        let control: Rc<dyn Any> = Rc::new(());
        if let Some(data) = data {
            self.add(data, control.clone());
        }
        Some(control)
    }
    fn clear(&self) {
        self.map.borrow_mut().clear();
    }
}
