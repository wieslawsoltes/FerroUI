//! Tests specific to this port: the tables of virtual members.
//!
//! A call of a virtual member reaches the override of the most derived class
//! that has one, and an override reaches the one below it with
//! `parent_<member>`, whatever the classes in between are written as: a plain
//! `impl` (their slot is a function that forwards to the base class), or
//! `ferro_impl_classes!` and `ferro_overrides!`, which state what the class
//! overrides (the slot of a member that is not overridden is then the slot of
//! the base class itself).
//!
//! The hierarchy below is six classes deep under the class that declares the
//! members and mixes the three forms. The class names are unique in the
//! crate: the table of known types is process-wide.

use super::*;
use crate::*;
use std::cell::RefCell;

thread_local! {
    /// The overrides that ran on the thread, in order.
    static LOG: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

fn log(entry: &'static str) {
    LOG.with(|log| log.borrow_mut().push(entry));
}

fn take_log() -> Vec<&'static str> {
    LOG.with(|log| std::mem::take(&mut *log.borrow_mut()))
}

#[repr(C)]
pub struct DispatchRoot {
    base: FerroObject,
}

ferro_class! {
    DispatchRoot: FerroObject, virtuals DispatchRootImpl: FerroObjectImpl {
        /// Logs the classes whose override runs and counts them.
        fn visit(this, count: i32) -> i32;
        fn name(this) -> String;
    }
}

// Written as a plain `impl`: states nothing.
impl FerroObjectImpl for DispatchRoot {
    fn on_property_changed_core(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        log("root.core>");
        Self::parent_on_property_changed_core(this, change);
        log("root.core<");
    }

    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        log("root.changed");
        Self::parent_on_property_changed(this, change);
    }
}

impl DispatchRootImpl for DispatchRoot {
    fn visit(_this: &Self, count: i32) -> i32 {
        log("root");
        count + 1
    }

    fn name(_this: &Self) -> String {
        s("root")
    }
}

impl DispatchRoot {
    ferro_property!(pub fn foo_property() -> StyledProperty<String> {
        FerroProperty::register::<DispatchRoot, _>("Foo", s("foodefault"))
    });

    fn construct() -> Self {
        Self { base: FerroObject::construct() }
    }

    pub fn new() -> Ref<Self> {
        instantiate(Self::construct())
    }
}

/// Declares a class of the hierarchy, without its implementations.
macro_rules! dispatch_class {
    ($name:ident : $base:ident) => {
        #[repr(C)]
        pub struct $name {
            base: $base,
        }

        ferro_class!($name: $base);

        impl $name {
            fn construct() -> Self {
                Self { base: $base::construct() }
            }

            pub fn new() -> Ref<Self> {
                instantiate(Self::construct())
            }
        }
    };
}

// Level 1: overrides `visit` in a plain `impl`, the base first.
dispatch_class!(DispatchLevel1: DispatchRoot);
ferro_impl_classes!(DispatchLevel1: FerroObjectImpl);

impl DispatchRootImpl for DispatchLevel1 {
    fn visit(this: &Self, count: i32) -> i32 {
        let count = Self::parent_visit(this, count);
        log("l1");
        count + 1
    }
}

// Level 2: overrides nothing and states it.
dispatch_class!(DispatchLevel2: DispatchLevel1);
ferro_impl_classes!(DispatchLevel2: FerroObjectImpl, DispatchRootImpl);

// Level 3: overrides `name` and `on_property_changed` and states them.
dispatch_class!(DispatchLevel3: DispatchLevel2);

ferro_overrides! { impl FerroObjectImpl for DispatchLevel3 {
    fn on_property_changed(this: &Self, change: &FerroPropertyChangedEventArgs<'_>) {
        log("l3.changed>");
        Self::parent_on_property_changed(this, change);
        log("l3.changed<");
    }
} }

ferro_overrides! { impl DispatchRootImpl for DispatchLevel3 {
    /// The name of the class before the name of its base.
    fn name(this: &Self) -> String {
        format!("l3:{}", Self::parent_name(this))
    }
} }

// Level 4: overrides nothing, in plain `impl`s.
dispatch_class!(DispatchLevel4: DispatchLevel3);
impl FerroObjectImpl for DispatchLevel4 {}
impl DispatchRootImpl for DispatchLevel4 {}

// Level 5: overrides nothing and states it.
dispatch_class!(DispatchLevel5: DispatchLevel4);
ferro_impl_classes!(DispatchLevel5: FerroObjectImpl, DispatchRootImpl);

// Level 6: overrides `constructed` and `visit` (the base last) and states them.
dispatch_class!(DispatchLevel6: DispatchLevel5);

ferro_overrides! { impl FerroObjectImpl for DispatchLevel6 {
    fn constructed(this: &Self) {
        Self::parent_constructed(this);
        log("l6.constructed");
    }
} }

ferro_overrides! { impl DispatchRootImpl for DispatchLevel6 {
    fn visit(this: &Self, count: i32) -> i32 {
        log("l6");
        Self::parent_visit(this, count) + 1
    }
} }

/// The six slots of a table of the hierarchy: `visit`, `name`, and the four
/// members of the root class (`constructed`, `on_property_changed_core`,
/// `on_property_changed`, `update_data_validation`).
fn slots<T: ObjectType<VTable = DispatchRootVTable>>() -> [usize; 6] {
    let table = T::vtable();
    [
        table.visit as usize,
        table.name as usize,
        table.base.constructed as usize,
        table.base.on_property_changed_core as usize,
        table.base.on_property_changed as usize,
        table.base.update_data_validation as usize,
    ]
}

const VISIT: usize = 0;
const NAME: usize = 1;
const CONSTRUCTED: usize = 2;
const ON_PROPERTY_CHANGED_CORE: usize = 3;
const ON_PROPERTY_CHANGED: usize = 4;
const UPDATE_DATA_VALIDATION: usize = 5;

#[test]
fn call_reaches_the_most_derived_override_and_each_base_once() {
    let root = DispatchRoot::new();
    assert_eq!(1, root.visit(0));
    assert_eq!(vec!["root"], take_log());

    // The override of level 1 calls its base before its own work.
    let level1 = DispatchLevel1::new();
    assert_eq!(2, level1.visit(0));
    assert_eq!(vec!["root", "l1"], take_log());

    // The levels without an override reach the one of level 1, whether they
    // state that they have none (2, 3, 5) or not (4).
    assert_eq!(2, DispatchLevel2::new().visit(0));
    assert_eq!(vec!["root", "l1"], take_log());
    assert_eq!(2, DispatchLevel3::new().visit(0));
    assert_eq!(vec!["root", "l1"], take_log());
    assert_eq!(2, DispatchLevel4::new().visit(0));
    assert_eq!(vec!["root", "l1"], take_log());
    assert_eq!(2, DispatchLevel5::new().visit(0));
    assert_eq!(vec!["root", "l1"], take_log());

    // The override of level 6 calls its base after its own work: every
    // override is entered once, in the order the overrides call each other.
    let level6 = DispatchLevel6::new();
    assert_eq!(vec!["l6.constructed"], take_log());
    assert_eq!(3, level6.visit(0));
    assert_eq!(vec!["l6", "root", "l1"], take_log());
}

#[test]
fn call_through_a_base_class_reaches_the_same_override() {
    let level6 = DispatchLevel6::new();
    take_log();

    let as_root: Ref<DispatchRoot> = level6.clone().upcast();
    assert_eq!(3, as_root.visit(0));
    assert_eq!(vec!["l6", "root", "l1"], take_log());

    let as_level3: Ref<DispatchLevel3> = level6.clone().upcast();
    assert_eq!(3, as_level3.visit(0));
    assert_eq!(vec!["l6", "root", "l1"], take_log());
    assert_eq!("l3:root", as_level3.name());
}

#[test]
fn override_in_the_middle_is_inherited_by_the_classes_below_it() {
    assert_eq!("root", DispatchRoot::new().name());
    assert_eq!("root", DispatchLevel1::new().name());
    assert_eq!("root", DispatchLevel2::new().name());
    assert_eq!("l3:root", DispatchLevel3::new().name());
    assert_eq!("l3:root", DispatchLevel4::new().name());
    assert_eq!("l3:root", DispatchLevel5::new().name());
    assert_eq!("l3:root", DispatchLevel6::new().name());
}

#[test]
fn constructed_runs_the_override_of_the_class_once() {
    DispatchLevel5::new();
    assert!(take_log().is_empty());

    DispatchLevel6::new();
    assert_eq!(vec!["l6.constructed"], take_log());
}

#[test]
fn property_change_reaches_the_overrides_through_every_form_of_class() {
    // Levels 1 and 2 state that they override neither member.
    let level2 = DispatchLevel2::new();
    level2.set_value(DispatchRoot::foo_property(), s("a"));
    assert_eq!(vec!["root.core>", "root.changed", "root.core<"], take_log());

    // Level 3 overrides `on_property_changed`; the levels below it do not,
    // stated (5, 6) or not (4).
    for target in [
        DispatchLevel3::new().upcast::<DispatchRoot>(),
        DispatchLevel4::new().upcast::<DispatchRoot>(),
        DispatchLevel5::new().upcast::<DispatchRoot>(),
        DispatchLevel6::new().upcast::<DispatchRoot>(),
    ] {
        take_log();
        target.set_value(DispatchRoot::foo_property(), s("a"));
        assert_eq!(vec!["root.core>", "l3.changed>", "root.changed", "l3.changed<", "root.core<"], take_log());
    }
}

#[test]
fn implementation_states_the_members_it_overrides() {
    fn names(overrides: Option<&'static [&'static str]>) -> Option<Vec<&'static str>> {
        overrides.map(|names| names.to_vec())
    }

    // A plain `impl` states nothing.
    assert_eq!(None, names(<DispatchRoot as FerroObjectImpl>::__OVERRIDES));
    assert_eq!(None, names(<DispatchLevel1 as DispatchRootImpl>::__OVERRIDES));
    assert_eq!(None, names(<DispatchLevel4 as FerroObjectImpl>::__OVERRIDES));
    assert_eq!(None, names(<DispatchLevel4 as DispatchRootImpl>::__OVERRIDES));

    // `ferro_impl_classes!` states that there are none.
    assert_eq!(Some(vec![]), names(<DispatchLevel1 as FerroObjectImpl>::__OVERRIDES));
    assert_eq!(Some(vec![]), names(<DispatchLevel2 as DispatchRootImpl>::__OVERRIDES));
    assert_eq!(Some(vec![]), names(<DispatchLevel5 as FerroObjectImpl>::__OVERRIDES));

    // `ferro_overrides!` states the functions of its block.
    assert_eq!(Some(vec!["on_property_changed"]), names(<DispatchLevel3 as FerroObjectImpl>::__OVERRIDES));
    assert_eq!(Some(vec!["name"]), names(<DispatchLevel3 as DispatchRootImpl>::__OVERRIDES));
    assert_eq!(Some(vec!["constructed"]), names(<DispatchLevel6 as FerroObjectImpl>::__OVERRIDES));
    assert_eq!(Some(vec!["visit"]), names(<DispatchLevel6 as DispatchRootImpl>::__OVERRIDES));
}

#[test]
fn slot_of_a_member_stated_as_not_overridden_is_the_slot_of_the_base_class() {
    let root = slots::<DispatchRoot>();
    let level1 = slots::<DispatchLevel1>();
    let level2 = slots::<DispatchLevel2>();
    let level3 = slots::<DispatchLevel3>();
    let level4 = slots::<DispatchLevel4>();
    let level5 = slots::<DispatchLevel5>();
    let level6 = slots::<DispatchLevel6>();

    // Level 1 states that it overrides no member of the root class.
    assert_eq!(root[CONSTRUCTED..], level1[CONSTRUCTED..]);

    // Level 2 states that it overrides nothing.
    assert_eq!(level1, level2);

    // Level 3 overrides `name` and `on_property_changed`.
    for member in [VISIT, CONSTRUCTED, ON_PROPERTY_CHANGED_CORE, UPDATE_DATA_VALIDATION] {
        assert_eq!(level2[member], level3[member]);
    }
    assert_ne!(level2[NAME], level3[NAME]);
    assert_ne!(level2[ON_PROPERTY_CHANGED], level3[ON_PROPERTY_CHANGED]);

    // Level 5 states that it overrides nothing: its slots are the ones of
    // level 4, which are functions of level 4.
    assert_eq!(level4, level5);

    // Level 6 overrides `constructed` and `visit`.
    for member in [NAME, ON_PROPERTY_CHANGED_CORE, ON_PROPERTY_CHANGED, UPDATE_DATA_VALIDATION] {
        assert_eq!(level5[member], level6[member]);
    }
    assert_ne!(level5[VISIT], level6[VISIT]);
    assert_ne!(level5[CONSTRUCTED], level6[CONSTRUCTED]);
}

#[test]
fn slot_is_not_taken_from_a_base_class_that_has_no_such_member() {
    // Nothing stated: the slot is a function of the class.
    assert!(!__forwards_to_parent::<DispatchLevel1, DispatchRootVTable>(None, "visit"));

    // Stated: the slot of the base class, for the members that are not named.
    assert!(__forwards_to_parent::<DispatchLevel1, DispatchRootVTable>(Some(&[]), "visit"));
    assert!(!__forwards_to_parent::<DispatchLevel1, DispatchRootVTable>(Some(&["visit"]), "visit"));
    assert!(__forwards_to_parent::<DispatchLevel1, DispatchRootVTable>(Some(&["visit"]), "name"));

    // The class that declares a member has no base class with it, whatever
    // its implementation states; it has one with the members of the root.
    assert!(!__forwards_to_parent::<DispatchRoot, DispatchRootVTable>(Some(&[]), "visit"));
    assert!(__forwards_to_parent::<DispatchRoot, FerroObjectVTable>(Some(&[]), "constructed"));
}
