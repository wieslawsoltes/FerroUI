// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
//! Native views of the original search predicates and lazy query cursors.
use ferrodock_model::{
    controls::IRootDock,
    core::{IDockable, RootPredicate},
    DockableQuery, DockableQueryEnumerator,
};
use ferroui_base::{
    data::core::{ValueType, ValueTypes},
    metadata::{from_markup_value, into_markup_value, MarkupDelegate, MarkupType, MarkupTyped},
};
use ferroui_system::IDisposable;
use std::rc::Rc;

/// Source predicate used by Find and FindDockable.
pub use ferrodock_model::core::DockablePredicate;

/// Native view retaining the original dockable predicate.
#[derive(Clone)]
pub struct DockablePredicateReference(DockablePredicate);
impl DockablePredicateReference {
    /// Views an existing predicate without invoking it.
    pub fn new(model: DockablePredicate) -> Self {
        Self(model)
    }
    /// Gets the original predicate instance.
    pub fn model(&self) -> DockablePredicate {
        self.0.clone()
    }
    /// Invokes the original predicate with the original dockable.
    pub fn invoke(&self, value: Option<Rc<dyn IDockable>>) -> bool {
        (self.0)(value)
    }
    /// Checks a method signature and adapts it without evaluating the predicate.
    pub fn from_delegate(delegate: MarkupDelegate) -> Option<Self> {
        if !accepts(&delegate, ValueType::of::<Rc<dyn IDockable>>()) {
            return None;
        }
        Some(Self::new(Rc::new(move |value| {
            let result = delegate
                .try_invoke(&[into_markup_value(value)])
                .expect("the dockable predicate invocation succeeds");
            from_markup_value::<bool>(&result).expect("the dockable predicate returns Boolean")
        })))
    }
}
impl PartialEq for DockablePredicateReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
ferroui_base::ferro_markup_type!(class DockablePredicateReference as "Func" {
    namespace:"System",generic:"Func`2"[Rc<dyn IDockable>,bool],this:DockablePredicateReference,
    handles:[DockablePredicateReference,Option<DockablePredicateReference>],
    methods:[fn Invoke(Option<Rc<dyn IDockable>>)->bool => DockablePredicateReference::invoke],
});

/// Native view retaining the original optional FindRoot predicate.
#[derive(Clone)]
pub struct RootPredicateReference(RootPredicate);
impl RootPredicateReference {
    /// Views an existing predicate without invoking it.
    pub fn new(model: RootPredicate) -> Self {
        Self(model)
    }
    /// Gets the original predicate instance.
    pub fn model(&self) -> RootPredicate {
        self.0.clone()
    }
    /// Invokes the original predicate with the original root.
    pub fn invoke(&self, value: Rc<dyn IRootDock>) -> bool {
        (self.0)(value)
    }
    /// Checks a method signature and adapts it without evaluating the predicate.
    pub fn from_delegate(delegate: MarkupDelegate) -> Option<Self> {
        if !accepts(&delegate, ValueType::of::<Rc<dyn IRootDock>>()) {
            return None;
        }
        Some(Self::new(Rc::new(move |value| {
            let result = delegate
                .try_invoke(&[Some(Rc::new(value))])
                .expect("the root predicate invocation succeeds");
            from_markup_value::<bool>(&result).expect("the root predicate returns Boolean")
        })))
    }
}
impl PartialEq for RootPredicateReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
ferroui_base::ferro_markup_type!(class RootPredicateReference as "Func" {
    namespace:"System",generic:"Func`2"[Rc<dyn IRootDock>,bool],this:RootPredicateReference,
    handles:[RootPredicateReference,Option<RootPredicateReference>],
    methods:[fn Invoke(Rc<dyn IRootDock>)->bool => RootPredicateReference::invoke],
});

fn accepts(delegate: &MarkupDelegate, input: ValueType) -> bool {
    delegate.method().is_none_or(|method| {
        method.parameters.len() == 1
            && ValueTypes::is_assignable(input, (method.parameters[0])())
            && method
                .return_type
                .is_some_and(|value| value() == ValueType::of::<bool>())
    })
}

/// Native IEnumerable view retaining a source lazy query, without materializing results.
#[derive(Clone)]
pub struct DockableQueryReference(Rc<DockableQuery>);
impl DockableQueryReference {
    /// Retains the original query; no traversal or predicate runs here.
    pub fn new(model: DockableQuery) -> Self {
        Self(Rc::new(model))
    }
    /// Gets the original query.
    pub fn model(&self) -> Rc<DockableQuery> {
        self.0.clone()
    }
    /// Creates an independent cursor over the original query.
    pub fn get_enumerator(&self) -> DockableQueryEnumeratorReference {
        DockableQueryEnumeratorReference(self.0.get_enumerator())
    }
}
impl PartialEq for DockableQueryReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
ferroui_base::ferro_markup_type!(interface DockableQueryReference as "IEnumerable" {
    namespace:"System.Collections.Generic",generic:"IEnumerable`1"[Rc<dyn IDockable>],
    this:DockableQueryReference,handles:[DockableQueryReference],
    methods:[fn GetEnumerator()->DockableQueryEnumeratorReference => DockableQueryReference::get_enumerator],
});

/// Native cursor over a source query's own iterator, sharing Current and disposal state.
#[derive(Clone)]
pub struct DockableQueryEnumeratorReference(Rc<DockableQueryEnumerator>);
impl DockableQueryEnumeratorReference {
    /// Gets the source iterator's current dockable, or null before its first result.
    pub fn current(&self) -> Option<Rc<dyn IDockable>> {
        self.0.current()
    }
    /// Advances the existing source iterator; predicates run only during this call.
    pub fn move_next(&self) -> bool {
        self.0.move_next()
    }
    /// Disposes the source iterator without clearing its last Current value.
    pub fn dispose(&self) {
        self.0.dispose();
    }
    /// Iterator blocks in the original engine do not support Reset.
    pub fn reset(&self) -> Result<(), String> {
        self.0.reset()
    }
}
impl IDisposable for DockableQueryEnumeratorReference {
    fn dispose(&self) {
        DockableQueryEnumeratorReference::dispose(self);
    }
}
impl PartialEq for DockableQueryEnumeratorReference {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
ferroui_base::ferro_markup_type!(interface DockableQueryEnumeratorReference as "IEnumerator" {
    namespace:"System.Collections.Generic",generic:"IEnumerator`1"[Rc<dyn IDockable>],
    this:DockableQueryEnumeratorReference,handles:[DockableQueryEnumeratorReference],interfaces:[Rc<dyn IDisposable>],
    properties:[Current:Option<Rc<dyn IDockable>>{get:DockableQueryEnumeratorReference::current}],
    methods:[fn MoveNext()->bool => DockableQueryEnumeratorReference::move_next,
        fn Dispose() => DockableQueryEnumeratorReference::dispose,
        try fn Reset() => DockableQueryEnumeratorReference::reset],
});

pub(crate) fn register() {
    MarkupType::register_all(&[
        DockablePredicateReference::MARKUP,
        RootPredicateReference::MARKUP,
        DockableQueryReference::MARKUP,
        DockableQueryEnumeratorReference::MARKUP,
    ]);
    ValueTypes::register_cast::<DockableQueryEnumeratorReference, Rc<dyn IDisposable>>(|value| {
        Rc::new(value.clone())
    });
    ValueTypes::register_nullable::<DockablePredicateReference>();
    ValueTypes::register_nullable::<RootPredicateReference>();
    ValueTypes::register_reference_identity::<DockablePredicateReference>(|value| {
        Rc::as_ptr(&value.0).cast::<()>()
    });
    ValueTypes::register_reference_identity::<RootPredicateReference>(|value| {
        Rc::as_ptr(&value.0).cast::<()>()
    });
    ValueTypes::register_reference_identity::<DockableQueryReference>(|value| {
        Rc::as_ptr(&value.0).cast::<()>()
    });
    ValueTypes::register_reference_identity::<DockableQueryEnumeratorReference>(|value| {
        Rc::as_ptr(&value.0).cast::<()>()
    });
    ValueTypes::register_boxed_cast::<MarkupDelegate, DockablePredicateReference>(|value| {
        value
            .downcast_ref::<MarkupDelegate>()
            .cloned()
            .and_then(DockablePredicateReference::from_delegate)
    });
    ValueTypes::register_boxed_cast::<MarkupDelegate, RootPredicateReference>(|value| {
        value
            .downcast_ref::<MarkupDelegate>()
            .cloned()
            .and_then(RootPredicateReference::from_delegate)
    });
    ferroui_base::__ferro_compiler_metadata! {
        MarkupType::register_rust_paths(&[
            (DockablePredicateReference::MARKUP,"::ferrodock_model_ferro::DockablePredicateReference",false),
            (RootPredicateReference::MARKUP,"::ferrodock_model_ferro::RootPredicateReference",false),
            (DockableQueryReference::MARKUP,"::ferrodock_model_ferro::DockableQueryReference",false),
            (DockableQueryEnumeratorReference::MARKUP,"::ferrodock_model_ferro::DockableQueryEnumeratorReference",false),
        ]);
    }
}
