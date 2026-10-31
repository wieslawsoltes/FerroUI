// SPDX-License-Identifier: MIT
//! Array constants shared by the runtime loader and generated markup.
use ferroui_base::{
    data::{
        core::{ValueType, ValueTypes},
        model::{bindable_array_element, BindableArray},
    },
    metadata::{from_markup_value, into_markup_value, MarkupValue},
    BoxedValue, PropertyValue,
};
use std::{any::Any, any::TypeId, cell::RefCell, collections::HashMap, fmt, rc::Rc};

pub trait RuntimeArrayElement {
    fn full_name(&self) -> String;
    fn value_type(&self) -> Option<ValueType>;
    fn as_any(&self) -> &dyn Any;
}
struct NamedElement {
    name: &'static str,
    type_: ValueType,
}
impl RuntimeArrayElement for NamedElement {
    fn full_name(&self) -> String {
        self.name.into()
    }
    fn value_type(&self) -> Option<ValueType> {
        Some(self.type_)
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}
trait NativeArray {
    fn boxed(&self) -> BoxedValue;
    fn items(&self) -> Vec<MarkupValue>;
    fn len(&self) -> usize;
}
struct TypedArray<T: PropertyValue>(Rc<BindableArray<T>>);
impl<T: PropertyValue> NativeArray for TypedArray<T> {
    fn boxed(&self) -> BoxedValue {
        self.0.clone()
    }
    fn items(&self) -> Vec<MarkupValue> {
        self.0.to_vec().into_iter().map(into_markup_value).collect()
    }
    fn len(&self) -> usize {
        self.0.len()
    }
}
enum Storage {
    Untyped {
        items: Vec<MarkupValue>,
        materializing: bool,
    },
    Native(Rc<dyn NativeArray>),
}
#[derive(Clone)]
pub struct RuntimeArray {
    element_type: Rc<dyn RuntimeArrayElement>,
    storage: Rc<RefCell<Storage>>,
}
type VectorForm = fn(&[MarkupValue], bool) -> Option<BoxedValue>;
type NativeForm = fn(&[MarkupValue]) -> Option<Rc<dyn NativeArray>>;
ferroui_base::ferro_thread_local! {
 static VECTOR_FORMS:RefCell<HashMap<TypeId,VectorForm>>=RefCell::new(HashMap::new());
 static NATIVE_FORMS:RefCell<HashMap<TypeId,NativeForm>>=RefCell::new(HashMap::new());
}
// Reset even when a conversion callback fails or unwinds. No callback runs under a state borrow.
struct MaterializationGuard(Rc<RefCell<Storage>>);
impl Drop for MaterializationGuard {
    fn drop(&mut self) {
        if let Storage::Untyped { materializing, .. } = &mut *self.0.borrow_mut() {
            *materializing = false;
        }
    }
}
impl RuntimeArray {
    pub fn new(element_type: Rc<dyn RuntimeArrayElement>, items: Vec<MarkupValue>) -> Self {
        Self {
            element_type,
            storage: Rc::new(RefCell::new(Storage::Untyped {
                items,
                materializing: false,
            })),
        }
    }
    /// Generated code allocates its actual typed array once, before it is handed on.
    pub fn typed<T: PropertyValue>(name: &'static str, items: Vec<T>) -> Self {
        Self::register_element::<T>();
        Self {
            element_type: Rc::new(NamedElement {
                name,
                type_: ValueType::of::<T>(),
            }),
            storage: Rc::new(RefCell::new(Storage::Native(Rc::new(TypedArray(
                BindableArray::new(items),
            ))))),
        }
    }
    pub fn element_type(&self) -> &Rc<dyn RuntimeArrayElement> {
        &self.element_type
    }
    /// Current elements. Once materialized, the typed array is the only element store.
    pub fn items(&self) -> Vec<MarkupValue> {
        let native = match &*self.storage.borrow() {
            Storage::Untyped { items, .. } => return items.clone(),
            Storage::Native(array) => array.clone(),
        };
        native.items()
    }
    pub fn len(&self) -> usize {
        match &*self.storage.borrow() {
            Storage::Untyped { items, .. } => items.len(),
            Storage::Native(array) => array.len(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn register_element<T: PropertyValue>() {
        fn vector<T: PropertyValue>(items: &[MarkupValue], optional: bool) -> Option<BoxedValue> {
            let typed = items
                .iter()
                .map(from_markup_value::<T>)
                .collect::<Option<Vec<T>>>()?;
            Some(if optional {
                Rc::new(Some(typed))
            } else {
                Rc::new(typed)
            })
        }
        fn native<T: PropertyValue>(items: &[MarkupValue]) -> Option<Rc<dyn NativeArray>> {
            let typed = items
                .iter()
                .map(from_markup_value::<T>)
                .collect::<Option<Vec<T>>>()?;
            Some(Rc::new(TypedArray(BindableArray::new(typed))))
        }
        ValueTypes::register_nullable::<Vec<T>>();
        ferroui_base::data::model::register_bindable_array::<T>();
        VECTOR_FORMS.with_borrow_mut(|forms| {
            forms.insert(TypeId::of::<Vec<T>>(), vector::<T>);
            forms.insert(TypeId::of::<Option<Vec<T>>>(), vector::<T>);
        });
        NATIVE_FORMS.with_borrow_mut(|forms| {
            forms.insert(TypeId::of::<T>(), native::<T>);
        });
    }
    pub fn to_declared(&self, target: ValueType) -> Result<BoxedValue, String> {
        if target.is_object() || target.is::<RuntimeArray>() {
            return Ok(Rc::new(self.clone()));
        }
        if target.is::<Option<RuntimeArray>>() {
            return Ok(Rc::new(Some(self.clone())));
        }
        if let Some(element) = bindable_array_element(target.id()) {
            if self.element_type.value_type() != Some(element) {
                return Err(self.conversion_error(target));
            }
            let current = match &*self.storage.borrow() {
                Storage::Native(array) => Some(array.clone()),
                _ => None,
            };
            let array = match current {
                Some(array) => array,
                None => {
                    let form = NATIVE_FORMS
                        .with_borrow(|forms| forms.get(&element.id()).copied())
                        .ok_or_else(|| self.conversion_error(target))?;
                    let items = {
                        let mut storage = self.storage.borrow_mut();
                        match &mut *storage {
                            Storage::Untyped {
                                items,
                                materializing,
                            } => {
                                if *materializing {
                                    return Err("Reentrant array materialization".into());
                                }
                                *materializing = true;
                                items.clone()
                            }
                            Storage::Native(_) => unreachable!("no callbacks between state reads"),
                        }
                    };
                    let guard = MaterializationGuard(self.storage.clone());
                    let array = form(&items).ok_or_else(|| self.conversion_error(target))?;
                    let old = self.storage.replace(Storage::Native(array.clone()));
                    drop(guard);
                    drop(old); // Destruction may reenter the now-published array.
                    array
                }
            };
            return ValueTypes::try_cast(&array.boxed(), target)
                .ok_or_else(|| self.conversion_error(target));
        }
        let form = VECTOR_FORMS.with_borrow(|forms| forms.get(&target.id()).copied());
        let optional = target.name().starts_with("core::option::Option<");
        form.and_then(|form| form(&self.items(), optional))
            .ok_or_else(|| self.conversion_error(target))
    }
    fn conversion_error(&self, target: ValueType) -> String {
        format!(
            "An array of {} cannot be passed as a {}: the element type has no matching registered array form",
            self.element_type.full_name(),
            target.name()
        )
    }
}
impl PartialEq for RuntimeArray {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.storage, &other.storage)
    }
}
impl fmt::Debug for RuntimeArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[{}]", self.element_type.full_name(), self.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        cell::Cell,
        panic::{catch_unwind, AssertUnwindSafe},
    };
    fn raw(items: Vec<MarkupValue>) -> RuntimeArray {
        RuntimeArray::register_element::<i32>();
        RuntimeArray::new(
            Rc::new(NamedElement {
                name: "System.Int32",
                type_: ValueType::of::<i32>(),
            }),
            items,
        )
    }
    fn receiver() -> ValueType {
        ValueType::of::<Rc<BindableArray<i32>>>()
    }
    fn native(array: &RuntimeArray) -> Rc<BindableArray<i32>> {
        from_markup_value(&Some(array.to_declared(receiver()).unwrap())).unwrap()
    }
    #[test]
    fn one_materialization_preserves_identity_and_reads_mutated_values_in_all_forms() {
        let array = raw(vec![into_markup_value(97i32)]);
        let alias = array.clone();
        let a = native(&array);
        let b = native(&alias);
        assert!(Rc::ptr_eq(&a, &b));
        a.set(&[0], 98);
        assert_eq!(from_markup_value::<i32>(&array.items()[0]), Some(98));
        assert_eq!(
            array
                .to_declared(ValueType::of::<Vec<i32>>())
                .unwrap()
                .downcast_ref::<Vec<i32>>(),
            Some(&vec![98])
        );
        assert_eq!(array, alias);
        assert_eq!(array.len(), 1);
    }
    #[test]
    fn reentrant_failed_conversion_releases_guard_and_keeps_unmodified_original_values() {
        #[derive(Clone, PartialEq)]
        struct Probe;
        let array = Rc::new(raw(vec![into_markup_value(Probe)]));
        let weak = Rc::downgrade(&array);
        let fail = Rc::new(Cell::new(true));
        let failed = fail.clone();
        ValueTypes::register_boxed_cast::<Probe, i32>(move |_| {
            let array = weak.upgrade().unwrap();
            assert!(array
                .to_declared(receiver())
                .unwrap_err()
                .contains("Reentrant"));
            if failed.replace(false) {
                None
            } else {
                Some(97)
            }
        });
        assert!(array.to_declared(receiver()).is_err());
        assert!(from_markup_value::<Probe>(&array.items()[0]).is_some());
        assert!(!fail.get());
        assert_eq!(native(&array).get(&[0]), 97);
    }
    #[test]
    fn unwind_during_element_conversion_does_not_leave_materialization_locked() {
        #[derive(Clone, PartialEq)]
        struct Probe;
        let array = raw(vec![into_markup_value(Probe)]);
        let first = Cell::new(true);
        ValueTypes::register_boxed_cast::<Probe, i32>(move |_| {
            assert!(!first.replace(false), "conversion callback failure");
            Some(99)
        });
        assert!(catch_unwind(AssertUnwindSafe(|| array.to_declared(receiver()))).is_err());
        assert_eq!(native(&array).get(&[0]), 99);
    }
}
