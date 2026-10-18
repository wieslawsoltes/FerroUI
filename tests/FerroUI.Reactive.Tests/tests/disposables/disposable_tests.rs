//! Ported from `Tests.System.Reactive/Tests/Disposables/DisposableTests.cs` of Rx.NET.
//! Adapted: captured flags are `Rc<Cell<bool>>`; `Assert.Same` compares `Rc` addresses; the
//! enumeration of a `CompositeDisposable` (`ToArray`, `GetEnumerator`) is `to_vec`; the array of
//! `CopyTo` is a slice of `Option<Rc<dyn IDisposable>>` (`None` for `null`), and `CopyTo` panics
//! where upstream throws `ArgumentOutOfRangeException`; the `ICancelable`
//! side of `Disposable.Create` is checked on `AnonymousDisposable::new`, since `Disposable::create`
//! returns a plain `Rc<dyn IDisposable>`; null-argument lines inside ported tests are dropped.

use ferroui_reactive::concurrency::{IScheduler, Scheduler};
use ferroui_reactive::disposables::{
    AnonymousDisposable, BooleanDisposable, CancellationDisposable, CompositeDisposable, Disposable, DisposableExt,
    ICancelable, MultipleAssignmentDisposable, RefCountDisposable, ScheduledDisposable, SerialDisposable,
    SingleAssignmentDisposable, StableCompositeDisposable,
};
use ferroui_reactive::IDisposable;
use ferroui_reactive_testing::assert_throws;
use ferroui_system::threading::CancellationTokenSource;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

/// `Assert.Same` for reference-counted values of any (possibly unsized) type.
fn same<A: ?Sized, B: ?Sized>(a: &Rc<A>, b: &Rc<B>) -> bool {
    std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))
}

/// A flag and a disposable that sets it when disposed.
fn flagged() -> (Rc<Cell<bool>>, Rc<dyn IDisposable>) {
    let flag = Rc::new(Cell::new(false));
    let f = flag.clone();
    (flag, Disposable::create(move || f.set(true)))
}

/// A flag and a disposable that asserts it is disposed once and sets the flag.
fn flagged_once() -> (Rc<Cell<bool>>, Rc<dyn IDisposable>) {
    let flag = Rc::new(Cell::new(false));
    let f = flag.clone();
    (
        flag,
        Disposable::create(move || {
            assert!(!f.get());
            f.set(true);
        }),
    )
}

fn as_dyn<D: IDisposable + 'static>(d: &Rc<D>) -> Rc<dyn IDisposable> {
    d.clone()
}

#[test]
fn anonymous_disposable_create() {
    let _d: Rc<dyn IDisposable> = Disposable::create(|| {});
}

#[test]
fn anonymous_disposable_dispose() {
    let disposed = Rc::new(Cell::new(false));
    let f = disposed.clone();
    let d = AnonymousDisposable::new(move || f.set(true));
    assert!(!disposed.get());
    d.dispose();
    assert!(disposed.get());

    let c: Rc<dyn ICancelable> = d.clone();
    assert!(c.is_disposed());
}

#[test]
fn empty_disposable() {
    let d = Disposable::empty();
    d.dispose();
}

#[test]
fn boolean_disposable() {
    let d = BooleanDisposable::new();
    assert!(!d.is_disposed());
    d.dispose();
    assert!(d.is_disposed());
    d.dispose();
    assert!(d.is_disposed());
}

#[test]
fn single_assignment_disposable_set_null() {
    let d = SingleAssignmentDisposable::new();
    d.set_disposable(None);
}

#[test]
fn single_assignment_disposable_dispose_after_set() {
    let (disposed, dd) = flagged();

    let d = SingleAssignmentDisposable::new();
    d.set_disposable(Some(dd.clone()));

    assert!(same(&dd, &d.disposable().expect("the assigned disposable")));

    assert!(!disposed.get());
    d.dispose();
    assert!(disposed.get());
    d.dispose();
    assert!(disposed.get());

    assert!(d.is_disposed());
}

#[test]
fn single_assignment_disposable_dispose_before_set() {
    let (disposed, dd) = flagged();

    let d = SingleAssignmentDisposable::new();

    assert!(!disposed.get());
    d.dispose();
    assert!(!disposed.get());
    assert!(d.is_disposed());

    d.set_disposable(Some(dd));
    assert!(disposed.get());
    // Undefined upstream after disposal; disposing what is there must be a no-op.
    if let Some(inner) = d.disposable() {
        inner.dispose();
    }

    d.dispose();
    assert!(disposed.get());
}

#[test]
fn single_assignment_disposable_set_multiple_times() {
    let d = SingleAssignmentDisposable::new();
    d.set_disposable(Some(Disposable::empty()));

    assert_throws(|| d.set_disposable(Some(Disposable::empty())));
}

#[test]
fn composite_disposable_contains() {
    let d1 = Disposable::create(|| {});
    let d2 = Disposable::create(|| {});

    let g = CompositeDisposable::from_disposables([d1.clone(), d2.clone()]);
    assert_eq!(2, g.count());
    assert!(g.contains(&d1));
    assert!(g.contains(&d2));
}

#[test]
fn composite_disposable_is_read_only() {
    assert!(!CompositeDisposable::new().is_read_only());
}

#[test]
fn composite_disposable_copy_to_beyond_end() {
    assert_throws(|| CompositeDisposable::new().copy_to(&mut [None, None], 2));
}

#[test]
fn composite_disposable_copy_to() {
    let d1 = Disposable::create(|| {});
    let d2 = Disposable::create(|| {});
    let g = CompositeDisposable::from_disposables(vec![d1.clone(), d2.clone()]);

    let mut d: [Option<Rc<dyn IDisposable>>; 3] = [None, None, None];
    g.copy_to(&mut d, 1);
    assert!(same(d[1].as_ref().expect("the first disposable is copied"), &d1));
    assert!(same(d[2].as_ref().expect("the second disposable is copied"), &d2));
}

#[test]
fn composite_disposable_to_array() {
    let d1 = Disposable::create(|| {});
    let d2 = Disposable::create(|| {});
    let g = CompositeDisposable::from_disposables([d1.clone(), d2.clone()]);
    assert_eq!(2, g.count());
    let x = g.to_vec();
    assert_eq!(2, x.len());
    assert!(same(&x[0], &d1));
    assert!(same(&x[1], &d2));
}

#[test]
fn composite_disposable_get_enumerator() {
    let d1 = Disposable::create(|| {});
    let d2 = Disposable::create(|| {});
    let g = CompositeDisposable::from_disposables([d1.clone(), d2.clone()]);
    let mut lst = Vec::new();
    for x in g.to_vec() {
        lst.push(x);
    }

    assert_eq!(2, lst.len());
    assert!(same(&lst[0], &d1));
    assert!(same(&lst[1], &d2));
}

#[test]
fn composite_disposable_collection_initializer() {
    let d1 = Disposable::create(|| {});
    let d2 = Disposable::create(|| {});
    let g = CompositeDisposable::new();
    g.add(d1.clone());
    g.add(d2.clone());
    assert_eq!(2, g.count());
    assert!(g.contains(&d1));
    assert!(g.contains(&d2));
}

#[test]
fn composite_disposable_add() {
    let d1 = Disposable::create(|| {});
    let d2 = Disposable::create(|| {});
    let g = CompositeDisposable::from_disposables([d1.clone()]);
    assert_eq!(1, g.count());
    assert!(g.contains(&d1));
    g.add(d2.clone());
    assert_eq!(2, g.count());
    assert!(g.contains(&d2));
}

#[test]
fn composite_disposable_add_after_dispose() {
    let (disp1, d1) = flagged();
    let (disp2, d2) = flagged();
    let g = CompositeDisposable::from_disposables([d1]);
    assert_eq!(1, g.count());

    g.dispose();
    assert!(disp1.get());
    assert_eq!(0, g.count());

    g.add(d2);
    assert!(disp2.get());
    assert_eq!(0, g.count());

    assert!(g.is_disposed());
}

#[test]
fn composite_disposable_remove() {
    let (disp1, d1) = flagged();
    let (disp2, d2) = flagged();
    let g = CompositeDisposable::from_disposables([d1.clone(), d2.clone()]);

    assert_eq!(2, g.count());
    assert!(g.contains(&d1));
    assert!(g.contains(&d2));

    assert!(g.remove(&d1));
    assert_eq!(1, g.count());
    assert!(!g.contains(&d1));
    assert!(g.contains(&d2));
    assert!(disp1.get());

    assert!(g.remove(&d2));
    assert!(!g.contains(&d1));
    assert!(!g.contains(&d2));
    assert!(disp2.get());

    let (disp3, d3) = flagged();
    assert!(!g.remove(&d3));
    assert!(!disp3.get());
}

#[test]
fn composite_disposable_clear() {
    let (disp1, d1) = flagged();
    let (disp2, d2) = flagged();
    let g = CompositeDisposable::from_disposables([d1, d2]);
    assert_eq!(2, g.count());

    g.clear();
    assert!(disp1.get());
    assert!(disp2.get());
    assert_eq!(0, g.count());

    let (disp3, d3) = flagged();
    g.add(d3);
    assert!(!disp3.get());
    assert_eq!(1, g.count());
}

#[test]
fn composite_disposable_remove_optimization_behavior() {
    let g = CompositeDisposable::new();
    let mut m: HashMap<i32, Rc<dyn IDisposable>> = HashMap::new();
    let r = Rc::new(RefCell::new(Vec::<i32>::new()));

    let n = 100;

    for i in 0..n {
        let j = i;

        let r = r.clone();
        let d = Disposable::create(move || r.borrow_mut().push(j));
        m.insert(j, d.clone());
        g.add(d);
    }

    let d1: Vec<i32> = (0..n).filter(|i| i % 2 == 0).collect();
    for i in &d1 {
        g.remove(&m[i]);
    }

    assert_eq!(d1, *r.borrow());

    let d2: Vec<i32> = (0..n).filter(|i| i % 3 == 0).collect();
    for i in &d2 {
        g.remove(&m[i]);
    }

    let mut expected: Vec<i32> = d1.clone();
    expected.extend(d2.iter().copied().filter(|x| !d1.contains(x)));
    assert_eq!(expected, *r.borrow());

    let d3: Vec<i32> = (0..n).filter(|i| i % 5 == 0).collect();
    for i in &d3 {
        g.remove(&m[i]);
    }

    expected.extend(d3.iter().copied().filter(|x| !d1.contains(x) && !d2.contains(x)));
    assert_eq!(expected, *r.borrow());

    g.dispose();

    let mut z: Vec<i32> = Vec::new();
    for x in r.borrow().iter().copied() {
        if !d1.contains(&x) && !d2.contains(&x) && !d3.contains(&x) && !z.contains(&x) {
            z.push(x);
        }
    }
    let rest: Vec<i32> = (0..n).filter(|i| !(i % 2 == 0 || i % 3 == 0 || i % 5 == 0)).collect();
    assert_eq!(rest, z);
}

#[test]
fn composite_disposable_empty_get_enumerator() {
    let composite = CompositeDisposable::new();

    assert!(composite.to_vec().into_iter().next().is_none());
}

#[test]
fn composite_disposable_dispose_with() {
    let c = CompositeDisposable::new();
    let d = BooleanDisposable::new();
    d.clone().dispose_with(&c);
    assert!(c.contains(&as_dyn(&d)));

    c.dispose();
    assert!(d.is_disposed());
    assert!(c.is_disposed());
}

#[test]
fn composite_disposable_non_collection_enumerable_init() {
    let d = BooleanDisposable::new();

    let composite = CompositeDisposable::from_disposables(std::iter::once(as_dyn(&d)));

    composite.dispose();

    assert!(d.is_disposed());
}

#[test]
fn composite_disposable_disposed_is_no_op() {
    let d = BooleanDisposable::new();

    let composite = CompositeDisposable::from_disposables([as_dyn(&d)]);

    composite.dispose();

    composite.clear();

    assert!(!composite.contains(&as_dyn(&d)));

    let mut array: [Option<Rc<dyn IDisposable>>; 1] = [None];

    composite.copy_to(&mut array, 0);

    assert!(array[0].is_none());
}

#[test]
fn composite_disposable_copy_to_index_out_of_range() {
    let d1 = BooleanDisposable::new();
    let d2 = BooleanDisposable::new();

    let composite = CompositeDisposable::from_disposables([as_dyn(&d1), as_dyn(&d2)]);

    let mut array: [Option<Rc<dyn IDisposable>>; 2] = [None, None];

    // Upstream throws `ArgumentOutOfRangeException`; the port panics.
    assert_throws(|| composite.copy_to(&mut array, 1));
}

#[test]
fn composite_disposable_get_enumerator_disposed_entries() {
    let d1 = BooleanDisposable::new();
    let d2 = BooleanDisposable::new();
    let d3 = BooleanDisposable::new();

    let composite = CompositeDisposable::from_disposables([as_dyn(&d1), as_dyn(&d2), as_dyn(&d3)]);

    composite.remove(&as_dyn(&d2));

    let items = composite.to_vec();
    let mut enumerator = items.iter();

    assert!(same(enumerator.next().expect("the first entry"), &d1));

    assert!(same(enumerator.next().expect("the second entry"), &d3));

    assert!(enumerator.next().is_none());
}

#[test]
fn cancellation_disposable_default_ctor() {
    let c = CancellationDisposable::new();
    assert!(!c.token().is_cancellation_requested());
    assert!(c.token().can_be_canceled());
    c.dispose();
    assert!(c.is_disposed());
    assert!(c.token().is_cancellation_requested());
}

#[test]
fn cancellation_disposable_token_ctor() {
    let t = CancellationTokenSource::new();
    let c = CancellationDisposable::with_source(t.clone());
    // `t.Token == c.Token`: tokens have no equality in the port; that both observe the same source
    // is checked by the cancellation of `t` below.
    assert!(!t.token().is_cancellation_requested());
    assert!(!c.token().is_cancellation_requested());
    assert!(c.token().can_be_canceled());
    c.dispose();
    assert!(c.is_disposed());
    assert!(c.token().is_cancellation_requested());
    assert!(t.token().is_cancellation_requested());
}

#[test]
fn serial_disposable_ctor_prop() {
    let m = SerialDisposable::new();
    assert!(m.disposable().is_none());
}

#[test]
fn serial_disposable_replace_before_dispose() {
    let (disp1, d1) = flagged();
    let (disp2, d2) = flagged();

    let m = SerialDisposable::new();
    m.set_disposable(Some(d1.clone()));
    assert!(same(&d1, &m.disposable().expect("d1")));
    assert!(!disp1.get());

    m.set_disposable(Some(d2.clone()));
    assert!(same(&d2, &m.disposable().expect("d2")));
    assert!(disp1.get());
    assert!(!disp2.get());
}

#[test]
fn serial_disposable_replace_after_dispose() {
    let (disp1, d1) = flagged();
    let (disp2, d2) = flagged();

    let m = SerialDisposable::new();
    m.dispose();
    assert!(m.is_disposed());

    m.set_disposable(Some(d1));
    assert!(m.disposable().is_none());
    assert!(disp1.get());

    m.set_disposable(Some(d2));
    assert!(m.disposable().is_none());
    assert!(disp2.get());
}

#[test]
fn serial_disposable_dispose() {
    let (disp, d) = flagged();

    let m = SerialDisposable::new();
    m.set_disposable(Some(d.clone()));
    assert!(same(&d, &m.disposable().expect("d")));
    assert!(!disp.get());

    m.dispose();
    assert!(m.is_disposed());
    assert!(disp.get());
}

#[test]
fn ref_count_disposable_single_reference() {
    let d = BooleanDisposable::new();
    let r = RefCountDisposable::new(as_dyn(&d));
    assert!(!d.is_disposed());
    r.dispose();
    assert!(d.is_disposed());
    r.dispose();
    assert!(d.is_disposed());
}

#[test]
fn ref_count_disposable_ref_counting() {
    let d = BooleanDisposable::new();
    let r = RefCountDisposable::new(as_dyn(&d));
    assert!(!d.is_disposed());

    let d1 = r.get_disposable();
    let d2 = r.get_disposable();
    assert!(!d.is_disposed());

    d1.dispose();
    assert!(!d.is_disposed());

    d2.dispose();
    assert!(!d.is_disposed());

    r.dispose();
    assert!(d.is_disposed());
    assert!(r.is_disposed());

    let d3 = r.get_disposable();
    d3.dispose();
}

#[test]
fn ref_count_disposable_primary_disposes_first() {
    let d = BooleanDisposable::new();
    let r = RefCountDisposable::new(as_dyn(&d));
    assert!(!d.is_disposed());

    let d1 = r.get_disposable();
    let d2 = r.get_disposable();
    assert!(!d.is_disposed());

    d1.dispose();
    assert!(!d.is_disposed());

    r.dispose();
    assert!(!d.is_disposed());

    d2.dispose();
    assert!(d.is_disposed());
}

#[test]
fn ref_count_disposable_throw_if_disposed() {
    let d = BooleanDisposable::new();
    let r = RefCountDisposable::with_throw_when_disposed(as_dyn(&d), true);
    r.dispose();

    assert!(d.is_disposed());

    assert_throws(|| {
        r.get_disposable();
    });
}

#[test]
fn scheduled_disposable() {
    let d = BooleanDisposable::new();
    let s = ScheduledDisposable::new(Scheduler::immediate(), as_dyn(&d));

    assert!(!d.is_disposed());

    let immediate: Rc<dyn IScheduler> = Scheduler::immediate();
    assert!(same(&immediate, s.scheduler()));
    assert!(same(&d, &s.disposable()));

    s.dispose();

    assert!(d.is_disposed());
    assert!(s.is_disposed());

    assert!(same(&immediate, s.scheduler()));
    // Undefined upstream after disposal; disposing what is there must be a no-op.
    s.disposable().dispose();
}

#[test]
fn multiple_assignment_disposable() {
    let m = MultipleAssignmentDisposable::new();

    let (disp1, d1) = flagged();
    m.set_disposable(Some(d1.clone()));
    assert!(same(&d1, &m.disposable().expect("d1")));
    assert!(!m.is_disposed());

    let (disp2, d2) = flagged();
    m.set_disposable(Some(d2.clone()));
    assert!(same(&d2, &m.disposable().expect("d2")));
    assert!(!m.is_disposed());
    assert!(!disp1.get());

    m.dispose();
    assert!(disp2.get());
    assert!(m.is_disposed());
    // Undefined upstream after disposal; disposing what is there must be a no-op.
    if let Some(inner) = m.disposable() {
        inner.dispose();
    }

    let (disp3, d3) = flagged();
    m.set_disposable(Some(d3));
    assert!(disp3.get());
    if let Some(inner) = m.disposable() {
        inner.dispose();
    }
    assert!(m.is_disposed());
}

#[test]
fn stable_composite_disposable_binary() {
    let (disp1, d1) = flagged_once();
    let (disp2, d2) = flagged_once();

    let d = StableCompositeDisposable::create(d1, d2);

    assert!(!disp1.get());
    assert!(!disp2.get());
    assert!(!d.is_disposed());

    d.dispose();

    assert!(disp1.get());
    assert!(disp2.get());
    assert!(d.is_disposed());

    d.dispose();

    assert!(disp1.get());
    assert!(disp2.get());
    assert!(d.is_disposed());
}

#[test]
fn stable_composite_disposable_nary1() {
    let (disp1, d1) = flagged_once();
    let (disp2, d2) = flagged_once();
    let (disp3, d3) = flagged_once();

    let d = StableCompositeDisposable::create_from([d1, d2, d3]);

    assert!(!disp1.get());
    assert!(!disp2.get());
    assert!(!disp3.get());
    assert!(!d.is_disposed());

    d.dispose();

    assert!(disp1.get());
    assert!(disp2.get());
    assert!(disp3.get());
    assert!(d.is_disposed());

    d.dispose();

    assert!(disp1.get());
    assert!(disp2.get());
    assert!(disp3.get());
    assert!(d.is_disposed());
}

#[test]
fn stable_composite_disposable_nary2() {
    let (disp1, d1) = flagged_once();
    let (disp2, d2) = flagged_once();
    let (disp3, d3) = flagged_once();

    let d = StableCompositeDisposable::create_from(vec![d1, d2, d3]);

    assert!(!disp1.get());
    assert!(!disp2.get());
    assert!(!disp3.get());
    assert!(!d.is_disposed());

    d.dispose();

    assert!(disp1.get());
    assert!(disp2.get());
    assert!(disp3.get());
    assert!(d.is_disposed());

    d.dispose();

    assert!(disp1.get());
    assert!(disp2.get());
    assert!(disp3.get());
    assert!(d.is_disposed());
}

// Not ported: AnonymousDisposable_CreateNull - argument validation
// Not ported: CompositeDisposable_ArgumentChecking - argument validation (null arrays; a negative capacity cannot be a usize)
// Not ported: CompositeDisposable_CopyTo_Null - argument validation
// Not ported: CompositeDisposable_CopyTo_Negative - argument validation (a negative index cannot be a usize)
// Not ported: CompositeDisposable_GetEnumeratorNonGeneric - Rust has no non-generic enumeration (same as CompositeDisposable_GetEnumerator)
// Not ported: CompositeDisposable_AddNull_via_params_ctor - argument validation
// Not ported: CompositeDisposable_AddNull_via_IEnum_ctor - argument validation
// Not ported: CompositeDisposable_AddNull - argument validation
// Not ported: CompositeDisposable_RemoveNull - argument validation
// Not ported: CompositeDisposable_GetEnumerator_Reset - IEnumerator.Reset has no counterpart on Rust iterators
// Not ported: CancellationDisposable_Ctor_Null - argument validation
// Not ported: ContextDisposable_CreateNullContext - argument validation; ContextDisposable (SynchronizationContext) is not in the port
// Not ported: ContextDisposable_CreateNullDisposable - argument validation; ContextDisposable (SynchronizationContext) is not in the port
// Not ported: ContextDisposable - ContextDisposable and SynchronizationContext are not in the port
// Not ported: RefCountDisposable_Ctor_Null - argument validation
// Not ported: ScheduledDisposable_Null - argument validation
// Not ported: StableCompositeDisposable_ArgumentChecking - argument validation

/// Many members are removed in linear time overall (dotnet/reactive issue 2005): above the
/// threshold of a linear search, a removal finds its member through an index; the members keep the
/// order they were added in.
#[test]
fn composite_disposable_large_number_of_items_add_remove() {
    use std::cell::Cell;
    let disposed = Rc::new(Cell::new(0usize));
    let group = CompositeDisposable::new();
    let items: Vec<Rc<dyn IDisposable>> = (0..5000)
        .map(|_| {
            let disposed = disposed.clone();
            Disposable::create(move || disposed.set(disposed.get() + 1))
        })
        .collect();
    for item in &items {
        group.add(item.clone());
    }
    assert_eq!(5000, group.count());
    for item in items.iter().step_by(2) {
        assert!(group.remove(item));
    }
    assert_eq!(2500, group.count());
    assert_eq!(2500, disposed.get());
    assert!(group.contains(&items[1]));
    assert!(!group.contains(&items[0]));
    let remaining = group.to_vec();
    assert!(remaining.iter().zip(items.iter().skip(1).step_by(2)).all(|(a, b)| std::ptr::addr_eq(Rc::as_ptr(a), Rc::as_ptr(b))));
    group.dispose();
    assert_eq!(5000, disposed.get());
}
