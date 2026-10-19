//! `ReactiveObjectObservationIntegrationTests` (`ReactiveUI.Tests/WhenAny/ReactiveObjectObservationIntegrationTests.cs`):
//! the observation of the properties of reactive objects. The tests of the partial file
//! `ReactiveObjectObservationIntegrationTests.WhenAnyValue.cs` are in
//! `reactive_object_observation_integration_tests_when_any_value.rs`.
//!
//! A nullable intermediate object (`x => x.Child!.IsNotNullString`) is a path with an optional
//! step (`then_optional_property`), whose value is `None` while there is no object; the tests set
//! the object before they observe it, as upstream's do, so the values are those of upstream
//! wrapped in `Some`.

use crate::mocks::{
    collect_items, AccountService, AccountUser, HostTestFixture, ObjChain1, ObjChain2, ObjChain3, OwnerClass, ProjectService, TestFixture,
    WhenAnyTestFixture,
};
use ferroui_reactive::concurrency::ImmediateScheduler;
use ferroui_reactive::ObservableExt;
use ferroui_reactiveui::*;
use ferroui_system::component_model::PropertyPath;
use ferroui_system::prop;
use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

/// The expected notification count after the second property change.
const EXPECTED_COUNT_AFTER_SECOND_CHANGE: usize = 2;
/// The expected notification count after the third property change.
const EXPECTED_COUNT_AFTER_THIRD_CHANGE: usize = 3;
/// Test value "Foo".
const FOO_TEXT: &str = "Foo";
/// Test value "Bar".
const BAR_TEXT: &str = "Bar";
/// Test value "Baz".
const BAZ_TEXT: &str = "Baz";
/// Test value "Bamf".
const BAMF_TEXT: &str = "Bamf";
/// Test value "abc".
const ABC_TEXT: &str = "abc";

fn text(value: &str) -> Option<String> {
    Some(value.to_owned())
}

/// `new TestFixture { IsNotNullString = "Foo", IsOnlyOneWord = "Baz", PocoProperty = "Bamf" }`.
fn foo_baz_bamf_fixture() -> Rc<TestFixture> {
    let fixture = TestFixture::new();
    fixture.set_is_not_null_string(text(FOO_TEXT));
    fixture.set_is_only_one_word(text(BAZ_TEXT));
    fixture.set_poco_property(text(BAMF_TEXT));
    fixture
}

/// The path `x => x.Child!.IsNotNullString`: `None` while there is no child.
fn child_is_not_null_string() -> PropertyPath<Rc<HostTestFixture>, Option<Option<String>>> {
    prop!(HostTestFixture => child).then_optional_property("IsNotNullString", |child: &Rc<TestFixture>| child.is_not_null_string())
}

/// The path `x => x.Model.Model.Model.SomeOtherParam`.
fn deep_some_other_param() -> PropertyPath<Rc<ObjChain1>, i32> {
    prop!(ObjChain1 => model.model.model.some_other_param)
}

/// Whether the user has a last name that is not blank (`!string.IsNullOrWhiteSpace(x.LastName)`).
fn has_last_name(user: &Rc<AccountUser>) -> bool {
    user.last_name().is_some_and(|name| !name.trim().is_empty())
}

#[test]
fn any_change_in_expression_list_triggers_update() {
    const FIRST_PARAM_VALUE: i32 = 42;
    const SECOND_PARAM_VALUE: i32 = 10;

    let obj = ObjChain1::new();

    let obs_updated = Rc::new(Cell::new(false));

    let updated = obs_updated.clone();
    let _ = obj.when_changed(deep_some_other_param()).subscribe_next(move |_| updated.set(true));

    obs_updated.set(false);

    obj.model().model().model().set_some_other_param(FIRST_PARAM_VALUE);

    assert!(obs_updated.get());

    obs_updated.set(false);

    obj.model().model().set_model(HostTestFixture::new());

    assert!(obs_updated.get());

    obs_updated.set(false);

    // new() { Model = new() { SomeOtherParam = 10 } }
    let host = HostTestFixture::new();
    host.set_some_other_param(SECOND_PARAM_VALUE);
    let chain3 = ObjChain3::new();
    chain3.set_model(host);
    obj.model().set_model(chain3);

    assert!(obs_updated.get());

    obs_updated.set(false);

    obj.set_model(ObjChain2::new());

    assert!(obs_updated.get());
}

#[test]
fn changed_should_have_valid_data() {
    let fixture = foo_baz_bamf_fixture();

    let sender: Rc<RefCell<Option<Rc<TestFixture>>>> = Rc::new(RefCell::new(None));

    let property_name: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let (sender_slot, name_slot) = (sender.clone(), property_name.clone());
    let _ = fixture.changed().observe_on(ImmediateScheduler::instance()).subscribe_next(move |x| {
        *sender_slot.borrow_mut() = Some(x.sender);

        *name_slot.borrow_mut() = Some(x.property_name);
    });

    fixture.set_uses_expr_raise_set(text(ABC_TEXT));

    assert!(sender.borrow().as_ref().is_some_and(|sender| Rc::ptr_eq(sender, &fixture)));

    assert_eq!(property_name.borrow().as_deref(), Some("UsesExprRaiseSet"));

    *sender.borrow_mut() = None;

    *property_name.borrow_mut() = None;

    fixture.set_poco_property(text(ABC_TEXT));

    assert!(sender.borrow().is_none());

    assert!(property_name.borrow().is_none());
}

#[test]
fn changing_should_have_valid_data() {
    let fixture = foo_baz_bamf_fixture();

    let sender: Rc<RefCell<Option<Rc<TestFixture>>>> = Rc::new(RefCell::new(None));

    let property_name: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let (sender_slot, name_slot) = (sender.clone(), property_name.clone());
    let _ = fixture.changing().observe_on(ImmediateScheduler::instance()).subscribe_next(move |x| {
        *sender_slot.borrow_mut() = Some(x.sender);

        *name_slot.borrow_mut() = Some(x.property_name);
    });

    fixture.set_uses_expr_raise_set(text(ABC_TEXT));

    assert!(sender.borrow().as_ref().is_some_and(|sender| Rc::ptr_eq(sender, &fixture)));

    assert_eq!(property_name.borrow().as_deref(), Some("UsesExprRaiseSet"));

    *sender.borrow_mut() = None;

    *property_name.borrow_mut() = None;

    fixture.set_poco_property(text(ABC_TEXT));

    assert!(sender.borrow().is_none());

    assert!(property_name.borrow().is_none());
}

// Not ported: MultiPropertyExpressionsShouldBeProperlyResolved - expression trees (it rewrites
// expressions with `Reflection.Rewrite` and reads the member names and types of their chains)

// `AccountUser` has no equality, so the dictionary of the users cannot be the value of a path
// (the value of an observed path is `PartialEq`). The path stops at `AccountService`, whose
// `AccountUsers` never changes, and the users are read from the service.
#[test]
fn non_nullable_types_test_shouldnt_need_decorators() {
    const EXPECTED_COUNT: usize = 3;

    let fixture = WhenAnyTestFixture::new();

    let result: Rc<RefCell<Option<Vec<Rc<AccountUser>>>>> = Rc::new(RefCell::new(None));

    let slot = result.clone();
    let _ = fixture
        .when_any_value(prop!(WhenAnyTestFixture => account_service))
        .select(|service: Rc<AccountService>| service.account_users().values().cloned().collect::<Vec<_>>())
        .where_(|users| !users.is_empty())
        .select(|users| users.into_iter().filter(has_last_name).collect::<Vec<_>>())
        .subscribe_next(move |users| *slot.borrow_mut() = Some(users));

    assert_eq!(result.borrow().as_ref().map(Vec::len), Some(EXPECTED_COUNT));
}

#[test]
fn non_nullable_types_test_shouldnt_need_decorators2() {
    const EXPECTED_COUNT: usize = 3;

    let fixture = WhenAnyTestFixture::new();

    let result: Rc<RefCell<Option<Vec<Rc<AccountUser>>>>> = Rc::new(RefCell::new(None));

    let slot = result.clone();
    let _ = fixture
        .when_any_value((prop!(WhenAnyTestFixture => project_service), prop!(WhenAnyTestFixture => account_service)))
        .where_(|(projects, users): &(Rc<ProjectService>, Rc<AccountService>)| !projects.projects().is_empty() && !users.account_users().is_empty())
        .select(|(_, users): (Rc<ProjectService>, Rc<AccountService>)| users.account_users().values().filter(|user| has_last_name(user)).cloned().collect::<Vec<_>>())
        .subscribe_next(move |users| *slot.borrow_mut() = Some(users));

    assert_eq!(result.borrow().as_ref().map(Vec::len), Some(EXPECTED_COUNT));
}

#[test]
fn nullable_types_test_shouldnt_need_decorators() {
    const EXPECTED_COUNT: usize = 3;

    let fixture = WhenAnyTestFixture::new();

    let result: Rc<RefCell<Option<Vec<Option<Rc<AccountUser>>>>>> = Rc::new(RefCell::new(None));

    let slot = result.clone();
    let _ = fixture
        .when_any_value(prop!(WhenAnyTestFixture => account_service))
        .select(|service: Rc<AccountService>| service.account_users_nullable().values().cloned().collect::<Vec<_>>())
        .where_(|users| !users.is_empty())
        .select(|users| users.into_iter().filter(|user| user.as_ref().is_some_and(has_last_name)).collect::<Vec<_>>())
        .subscribe_next(move |users| *slot.borrow_mut() = Some(users));

    assert_eq!(result.borrow().as_ref().map(Vec::len), Some(EXPECTED_COUNT));
}

#[test]
fn nullable_types_test_shouldnt_need_decorators2() {
    const EXPECTED_COUNT: usize = 3;

    let fixture = WhenAnyTestFixture::new();

    let result: Rc<RefCell<Option<Vec<Option<Rc<AccountUser>>>>>> = Rc::new(RefCell::new(None));

    let slot = result.clone();
    let _ = fixture
        .when_any_value((prop!(WhenAnyTestFixture => project_service), prop!(WhenAnyTestFixture => account_service)))
        .where_(|(projects, users): &(Rc<ProjectService>, Rc<AccountService>)| {
            !projects.projects_nullable().is_empty() && !users.account_users_nullable().is_empty()
        })
        .select(|(_, users): (Rc<ProjectService>, Rc<AccountService>)| {
            users.account_users_nullable().values().filter(|user| user.as_ref().is_some_and(has_last_name)).cloned().collect::<Vec<_>>()
        })
        .subscribe_next(move |users| *slot.borrow_mut() = Some(users));

    assert_eq!(result.borrow().as_ref().map(Vec::len), Some(EXPECTED_COUNT));
}

// Upstream checks that the garbage collector reclaims the replaced objects; here the replaced
// objects are freed when their last `Rc` goes, which the observation must not hold.
#[test]
fn object_should_be_garbage_collected_when_property_value_changes() {
    fn get_weak_reference1() -> (Rc<ObjChain1>, Weak<ObjChain2>) {
        let obj = ObjChain1::new();

        let weak_ref = Rc::downgrade(&obj.model());

        let _ = obj.when_changed(deep_some_other_param()).subscribe_next(|_| {});

        obj.set_model(ObjChain2::new());

        (obj, weak_ref)
    }

    fn get_weak_reference2() -> (Rc<ObjChain1>, Weak<ObjChain3>) {
        let obj = ObjChain1::new();

        let weak_ref = Rc::downgrade(&obj.model().model());

        let _ = obj.when_changed(deep_some_other_param()).subscribe_next(|_| {});

        obj.model().set_model(ObjChain3::new());

        (obj, weak_ref)
    }

    fn get_weak_reference3() -> (Rc<ObjChain1>, Weak<HostTestFixture>) {
        let obj = ObjChain1::new();

        let weak_ref = Rc::downgrade(&obj.model().model().model());

        let _ = obj.when_changed(deep_some_other_param()).subscribe_next(|_| {});

        obj.model().model().set_model(HostTestFixture::new());

        (obj, weak_ref)
    }

    let (obj1, weak_ref1) = get_weak_reference1();

    let (obj2, weak_ref2) = get_weak_reference2();

    let (obj3, weak_ref3) = get_weak_reference3();

    assert!(weak_ref1.upgrade().is_none());

    assert!(weak_ref2.upgrade().is_none());

    assert!(weak_ref3.upgrade().is_none());

    // Keep the objects alive until after the checks.
    drop((obj1, obj2, obj3));
}

#[test]
fn subscription_to_when_any_should_return_current_value() {
    const EXPECTED_VALUE: i32 = 42;

    let obj = HostTestFixture::new();

    let observed_value = Rc::new(Cell::new(1));

    let observed = observed_value.clone();
    let _ = obj.when_any_value(prop!(HostTestFixture => some_other_param)).subscribe_next(move |x| observed.set(x));

    obj.set_some_other_param(EXPECTED_VALUE);

    assert_eq!(observed_value.get(), obj.some_other_param());
}

#[test]
fn when_any_should_run_in_context() {
    let tid = std::thread::current().id();
    let when_any_tid = Rc::new(Cell::new(None));

    let fixture = foo_baz_bamf_fixture();

    let observed = when_any_tid.clone();
    let _ = fixture
        .when_any_value(prop!(TestFixture => is_not_null_string))
        .observe_on(ImmediateScheduler::instance())
        .subscribe_next(move |_| observed.set(Some(std::thread::current().id())));

    fixture.set_is_not_null_string(text(BAR_TEXT));

    assert_eq!(when_any_tid.get(), Some(tid));
}

#[test]
fn when_any_should_work_even_with_normal_properties() {
    let fixture = foo_baz_bamf_fixture();

    let output = collect_items(&fixture.when_any(prop!(TestFixture => poco_property), |x| x));

    let output2 = collect_items(&fixture.when_any_value(prop!(TestFixture => poco_property)));

    let output3 = collect_items(&fixture.when_any(prop!(TestFixture => nullable_int), |x| x));

    let output4 = collect_items(&fixture.when_any_value(prop!(TestFixture => nullable_int)));

    let output = output.borrow();
    let output2 = output2.borrow();
    let output3 = output3.borrow();
    let output4 = output4.borrow();

    assert_eq!(output.len(), 1);

    assert!(Rc::ptr_eq(&output[0].sender, &fixture));

    // Upstream's change has no expression (null); a change here carries the text of its path.
    assert_eq!(output[0].expression, "PocoProperty");

    assert_eq!(output[0].value.as_deref(), Some(BAMF_TEXT));

    assert_eq!(output2.len(), 1);

    assert_eq!(output2[0].as_deref(), Some(BAMF_TEXT));

    assert_eq!(output3.len(), 1);

    assert!(Rc::ptr_eq(&output3[0].sender, &fixture));

    assert_eq!(output3[0].expression, "NullableInt");

    assert!(output3[0].value.is_none());

    assert_eq!(output4.len(), 1);

    assert!(output4[0].is_none());
}

#[test]
fn when_any_smoke_test() {
    const INITIAL_PARAM: i32 = 5;
    const UPDATED_PARAM: i32 = 10;
    const THIRD_EMISSION_INDEX: usize = 2;

    let fixture = HostTestFixture::new();
    fixture.set_child(Some(TestFixture::new()));
    fixture.set_some_other_param(INITIAL_PARAM);

    let child = fixture.child().expect("the child is set");
    child.set_is_not_null_string(text(FOO_TEXT));

    // Upstream adds the two changes of each value to two lists; the pairs are kept in one here.
    let output = collect_items(&fixture.when_any((prop!(HostTestFixture => some_other_param), child_is_not_null_string()), |(sop, nns)| (sop, nns)));

    // ImmediateScheduler executes synchronously
    {
        let output = output.borrow();

        assert_eq!(output.len(), 1);

        assert!(Rc::ptr_eq(&output[0].0.sender, &fixture));

        assert!(Rc::ptr_eq(&output[0].1.sender, &fixture));

        assert_eq!(output[0].0.value, INITIAL_PARAM);

        assert_eq!(output[0].1.value, Some(text(FOO_TEXT)));
    }

    fixture.set_some_other_param(UPDATED_PARAM);

    // ImmediateScheduler executes synchronously
    {
        let output = output.borrow();

        assert_eq!(output.len(), EXPECTED_COUNT_AFTER_SECOND_CHANGE);

        assert!(Rc::ptr_eq(&output[1].0.sender, &fixture));

        assert!(Rc::ptr_eq(&output[1].1.sender, &fixture));

        assert_eq!(output[1].0.value, UPDATED_PARAM);

        assert_eq!(output[1].1.value, Some(text(FOO_TEXT)));
    }

    child.set_is_not_null_string(text(BAR_TEXT));

    // ImmediateScheduler executes synchronously
    {
        let output = output.borrow();

        assert_eq!(output.len(), EXPECTED_COUNT_AFTER_THIRD_CHANGE);

        assert!(Rc::ptr_eq(&output[THIRD_EMISSION_INDEX].0.sender, &fixture));

        assert!(Rc::ptr_eq(&output[THIRD_EMISSION_INDEX].1.sender, &fixture));

        assert_eq!(output[THIRD_EMISSION_INDEX].0.value, UPDATED_PARAM);

        assert_eq!(output[THIRD_EMISSION_INDEX].1.value, Some(text(BAR_TEXT)));
    }
}

#[test]
fn when_any_value_should_work_even_with_normal_properties() {
    const BAZ_LENGTH: usize = 3;

    let fixture = foo_baz_bamf_fixture();

    let output1 = collect_items(&fixture.when_any_value(prop!(TestFixture => poco_property)));

    let output2 = collect_items(&fixture.when_any_value_select(prop!(TestFixture => is_only_one_word), |x: Option<String>| x.map(|x| x.len())));

    assert_eq!(output1.borrow().len(), 1);

    assert_eq!(output1.borrow()[0].as_deref(), Some(BAMF_TEXT));

    assert_eq!(output2.borrow().len(), 1);

    assert_eq!(output2.borrow()[0], Some(BAZ_LENGTH));
}

#[test]
fn when_any_value_smoke_test() {
    const INITIAL_PARAM: i32 = 5;
    const UPDATED_PARAM: i32 = 10;
    const THIRD_EMISSION_INDEX: usize = 2;

    let fixture = HostTestFixture::new();
    fixture.set_child(Some(TestFixture::new()));
    fixture.set_some_other_param(INITIAL_PARAM);

    let child = fixture.child().expect("the child is set");
    child.set_is_not_null_string(text(FOO_TEXT));

    let output1 = Rc::new(RefCell::new(Vec::<i32>::new()));

    let output2 = Rc::new(RefCell::new(Vec::<Option<String>>::new()));

    let (sink1, sink2) = (output1.clone(), output2.clone());
    let _ = fixture
        .when_any_value_select((prop!(HostTestFixture => some_other_param), child_is_not_null_string()), |(sop, nns)| (sop, nns))
        .subscribe_next(move |(sop, nns)| {
            sink1.borrow_mut().push(sop);

            sink2.borrow_mut().push(nns.flatten());
        });

    // ImmediateScheduler executes synchronously
    assert_eq!(output1.borrow().len(), 1);

    assert_eq!(output2.borrow().len(), 1);

    assert_eq!(output1.borrow()[0], INITIAL_PARAM);

    assert_eq!(output2.borrow()[0].as_deref(), Some(FOO_TEXT));

    fixture.set_some_other_param(UPDATED_PARAM);

    // ImmediateScheduler executes synchronously
    assert_eq!(output1.borrow().len(), EXPECTED_COUNT_AFTER_SECOND_CHANGE);

    assert_eq!(output2.borrow().len(), EXPECTED_COUNT_AFTER_SECOND_CHANGE);

    assert_eq!(output1.borrow()[1], UPDATED_PARAM);

    assert_eq!(output2.borrow()[1].as_deref(), Some(FOO_TEXT));

    child.set_is_not_null_string(text(BAR_TEXT));

    // ImmediateScheduler executes synchronously
    assert_eq!(output1.borrow().len(), EXPECTED_COUNT_AFTER_THIRD_CHANGE);

    assert_eq!(output2.borrow().len(), EXPECTED_COUNT_AFTER_THIRD_CHANGE);

    assert_eq!(output1.borrow()[THIRD_EMISSION_INDEX], UPDATED_PARAM);

    assert_eq!(output2.borrow()[THIRD_EMISSION_INDEX].as_deref(), Some(BAR_TEXT));
}

#[test]
fn when_any_value_with_to_property() {
    let fixture = HostTestFixture::new();

    assert!(fixture.owner().is_none());

    assert!(fixture.owner_name().is_none());

    fixture.set_owner(Some(OwnerClass::with_name("Fred")));

    assert!(fixture.owner().is_some());

    assert_eq!(fixture.owner_name().as_deref(), Some("Fred"));

    let owner = fixture.owner().expect("the owner is set");

    owner.set_name(text("Wilma"));

    assert_eq!(fixture.owner_name().as_deref(), Some("Wilma"));

    owner.set_name(None);

    assert!(fixture.owner_name().is_none());

    owner.set_name(text("Barney"));

    assert_eq!(fixture.owner_name().as_deref(), Some("Barney"));

    owner.set_name(text("Betty"));

    assert_eq!(fixture.owner_name().as_deref(), Some("Betty"));
}
