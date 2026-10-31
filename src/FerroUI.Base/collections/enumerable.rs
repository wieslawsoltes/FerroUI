// SPDX-License-Identifier: MIT
//! Shared nongeneric collection contracts. Projections retain the original object and
//! dispatch through its real collection/cursor implementation; they do not copy items.
use crate::{
    data::core::ValueTypes,
    metadata::{MarkupInvokeError, MarkupType, MarkupTyped},
    BoxedValue,
};
use std::{any::Any, rc::Rc};
type Result<T> = std::result::Result<T, MarkupInvokeError>;

/// Operations of an original enumerable, projected by its declaring crate.
pub trait EnumerableDispatch: Any {
    fn get_enumerator(&self) -> Result<Option<Enumerator>>;
    /// Explicit actual IList capability. IReadOnlyList alone does not imply this facet.
    fn indexed(&self) -> Option<&dyn IndexedEnumerableDispatch> {
        None
    }
}
/// Indexed view of an object known to implement the nongeneric IList contract.
/// This is an access facet, not a declaration of the complete CLR IList metadata graph.
pub trait IndexedEnumerableDispatch {
    fn count(&self) -> Result<usize>;
    fn get_at(&self, index: usize) -> Result<Option<BoxedValue>>;
}
/// Operations of an original cursor. Disposal is an independent runtime capability.
pub trait EnumeratorDispatch: Any {
    fn current(&self) -> Result<Option<BoxedValue>>;
    fn move_next(&self) -> Result<bool>;
    fn reset(&self) -> Result<()>;
    /// `None` means the cursor does not implement IDisposable. No disposal is inferred
    /// from its IEnumerator declaration.
    fn dispose(&self) -> Option<Result<()>> {
        None
    }
}
#[derive(Clone)]
pub struct Enumerable {
    object: BoxedValue,
    dispatch: Rc<dyn EnumerableDispatch>,
}
impl Enumerable {
    /// The dispatch view must operate on `object`. It may retain a typed view of that
    /// object; boxing this projection always returns the supplied original allocation.
    pub fn new(object: BoxedValue, dispatch: impl EnumerableDispatch) -> Self {
        Self {
            object,
            dispatch: Rc::new(dispatch),
        }
    }
    pub fn object(&self) -> BoxedValue {
        self.object.clone()
    }
    pub fn get_enumerator(&self) -> Result<Option<Enumerator>> {
        self.dispatch.get_enumerator()
    }
    pub fn indexed(&self) -> Option<&dyn IndexedEnumerableDispatch> {
        self.dispatch.indexed()
    }
    pub fn dispatch_ref<T: Any>(&self) -> Option<&T> {
        (self.dispatch.as_ref() as &dyn Any).downcast_ref()
    }
}
impl PartialEq for Enumerable {
    fn eq(&self, other: &Self) -> bool {
        *self.object == *other.object
    }
}
#[derive(Clone)]
pub struct Enumerator {
    object: BoxedValue,
    dispatch: Rc<dyn EnumeratorDispatch>,
}
impl Enumerator {
    pub fn new(object: BoxedValue, dispatch: impl EnumeratorDispatch) -> Self {
        Self {
            object,
            dispatch: Rc::new(dispatch),
        }
    }
    pub fn object(&self) -> BoxedValue {
        self.object.clone()
    }
    pub fn current(&self) -> Result<Option<BoxedValue>> {
        self.dispatch.current()
    }
    pub fn move_next(&self) -> Result<bool> {
        self.dispatch.move_next()
    }
    pub fn reset(&self) -> Result<()> {
        self.dispatch.reset()
    }
    pub fn dispose(&self) -> Option<Result<()>> {
        self.dispatch.dispose()
    }
    pub fn dispatch_ref<T: Any>(&self) -> Option<&T> {
        (self.dispatch.as_ref() as &dyn Any).downcast_ref()
    }
}
impl PartialEq for Enumerator {
    fn eq(&self, other: &Self) -> bool {
        *self.object == *other.object
    }
}
#[doc(hidden)]
pub struct EnumerableMetadata;
#[doc(hidden)]
pub struct EnumeratorMetadata;
crate::ferro_markup_type!(interface EnumerableMetadata as "IEnumerable" {
 namespace:"System.Collections",this:Enumerable,handles:[Enumerable,Option<Enumerable>],
 methods:[try fn GetEnumerator()->Option<Enumerator> =>Enumerable::get_enumerator[Abstract,Virtual]]
});
crate::ferro_markup_type!(interface EnumeratorMetadata as "IEnumerator" {
 namespace:"System.Collections",this:Enumerator,handles:[Enumerator,Option<Enumerator>],
 properties:[Current:Option<BoxedValue>{try_get:Enumerator::current}],
 methods:[try fn MoveNext()->bool=>Enumerator::move_next[Abstract,Virtual],try fn Reset()=>Enumerator::reset[Abstract,Virtual]]
});
pub(crate) fn register() {
    MarkupType::register_all(&[EnumerableMetadata::MARKUP, EnumeratorMetadata::MARKUP]);
    ValueTypes::register_global(register_values);
}
fn register_values() {
    ValueTypes::register_nullable::<Enumerable>();
    ValueTypes::register_nullable::<Enumerator>();
    ValueTypes::register_boxed_cast::<Enumerable, Option<BoxedValue>>(|value| {
        Some(Some(value.downcast_ref::<Enumerable>()?.object()))
    });
    ValueTypes::register_boxed_cast::<Enumerator, Option<BoxedValue>>(|value| {
        Some(Some(value.downcast_ref::<Enumerator>()?.object()))
    });
    crate::__ferro_compiler_metadata! {crate::metadata::register_type_rust_paths(&[
    (||std::any::TypeId::of::<Enumerable>(),"::ferroui_base::collections::Enumerable"),
    (||std::any::TypeId::of::<Enumerator>(),"::ferroui_base::collections::Enumerator")]);}
}

/// Cursor over a framework collection's existing copy-on-write snapshot. Values are
/// boxed only when Current is read; the collection decides when to take the snapshot.
struct SnapshotCursor<T> {
    items: Rc<Vec<T>>,
    index: std::cell::Cell<usize>,
    valid: std::cell::Cell<bool>,
    box_item: fn(&T) -> Option<BoxedValue>,
}
impl<T> PartialEq for SnapshotCursor<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl<T: crate::PropertyValue> EnumeratorDispatch for Rc<SnapshotCursor<T>> {
    fn current(&self) -> Result<Option<BoxedValue>> {
        if !self.valid.get() {
            return Err(MarkupInvokeError::Failed(
                "Enumeration has not started or has already finished.".into(),
            ));
        }
        Ok((self.box_item)(&self.items[self.index.get() - 1]))
    }
    fn move_next(&self) -> Result<bool> {
        let index = self.index.get();
        let valid = index < self.items.len();
        if valid {
            self.index.set(index + 1);
        }
        self.valid.set(valid);
        Ok(valid)
    }
    fn reset(&self) -> Result<()> {
        self.index.set(0);
        self.valid.set(false);
        Ok(())
    }
    fn dispose(&self) -> Option<Result<()>> {
        Some(Ok(()))
    }
}
impl Enumerator {
    pub fn from_snapshot<T: crate::PropertyValue>(
        items: Rc<Vec<T>>,
        box_item: fn(&T) -> Option<BoxedValue>,
    ) -> Self {
        ValueTypes::register_reference::<SnapshotCursor<T>>();
        ValueTypes::register_boxed_cast_from_id::<Enumerator>(std::any::TypeId::of::<SnapshotCursor<T>>(), |object| {
            let cursor =
                crate::metadata::from_markup_value::<Rc<SnapshotCursor<T>>>(&Some(object.clone()))?;
            Some(Enumerator::new(object.clone(), cursor))
        });
        let cursor = Rc::new(SnapshotCursor {
            items,
            index: std::cell::Cell::new(0),
            valid: std::cell::Cell::new(false),
            box_item,
        });
        Self::new(cursor.clone(), cursor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        data::core::WeakValue,
        metadata::{from_markup_value, into_markup_value},
    };
    use std::cell::Cell;
    #[derive(Clone, PartialEq)]
    struct Observed {
        value: i32,
        boxes: Rc<Cell<usize>>,
    }
    fn box_observed(v: &Observed) -> Option<BoxedValue> {
        v.boxes.set(v.boxes.get() + 1);
        Some(Rc::new(v.value))
    }
    #[test]
    fn snapshot_cursor_boxes_current_lazily_and_shares_its_actual_reference_object() {
        crate::register_types();
        let boxes = Rc::new(Cell::new(0));
        let cursor = Enumerator::from_snapshot(
            Rc::new(vec![Observed {
                value: 7,
                boxes: boxes.clone(),
            }]),
            box_observed,
        );
        assert_eq!(boxes.get(), 0);
        assert!(cursor.current().is_err());
        let object = cursor.object();
        let weak = WeakValue::new(&object);
        let projected = from_markup_value::<Enumerator>(&Some(object.clone())).unwrap();
        assert!(Rc::ptr_eq(&projected.object(), &object));
        assert!(Rc::ptr_eq(
            into_markup_value(projected.clone()).as_ref().unwrap(),
            &object
        ));
        assert!(projected.move_next().unwrap());
        assert_eq!(
            from_markup_value::<i32>(&cursor.current().unwrap()),
            Some(7)
        );
        assert_eq!(boxes.get(), 1);
        assert_eq!(
            from_markup_value::<i32>(&projected.current().unwrap()),
            Some(7)
        );
        assert_eq!(boxes.get(), 2);
        assert!(!cursor.move_next().unwrap());
        assert!(projected.current().is_err());
        projected.reset().unwrap();
        assert!(cursor.move_next().unwrap());
        cursor.dispose().unwrap().unwrap();
        drop(object);
        drop(cursor);
        assert!(weak.upgrade().is_some());
        drop(projected);
        assert!(weak.upgrade().is_none());
    }
}
