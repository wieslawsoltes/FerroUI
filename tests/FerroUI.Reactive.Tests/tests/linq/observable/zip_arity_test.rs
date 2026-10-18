//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/ZipTest.cs` of Rx.NET: the tests of
//! the forms of two to sixteen sources (the regions `Never/Never`, `Empty/Empty`, `SymmetricReturn`,
//! `SelectorThrows` and `AllCompleted`), in the order of the upstream file; the other tests are in
//! `zip_test.rs`.
//! Adapted: `Observable.Zip` of `N` sources is `zip` (two) or `zipN`; `ObservableEx.Zip` (the tuple
//! forms) is `zip_tuple` or `zipN_tuple`; a selector that throws is the selector of `try_zip` or
//! `try_zipN` returning the error. Rust implements `PartialEq` and `Debug` for tuples of at most
//! twelve elements, so the tuples of thirteen to sixteen values are compared as vectors (a `select`
//! of each tuple into a `Vec`).

use ferroui_reactive::ObservableExt;
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

#[test]
fn zip_never2() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1) = (e0.as_observable(), e1.as_observable());
    let res = scheduler.start(move || s0.zip(s1, |_, _| 42));

    assert_messages!(res.messages());

    for e in [&e0, &e1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never2_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1) = (e0.as_observable(), e1.as_observable());
    let res = scheduler.start(move || s0.zip_tuple(s1));

    assert_messages!(res.messages());

    for e in [&e0, &e1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never3() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2) = (e0.as_observable(), e1.as_observable(), e2.as_observable());
    let res = scheduler.start(move || s0.zip3(s1, s2, |_, _, _| 42));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never3_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2) = (e0.as_observable(), e1.as_observable(), e2.as_observable());
    let res = scheduler.start(move || s0.zip3_tuple(s1, s2));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never4() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable());
    let res = scheduler.start(move || s0.zip4(s1, s2, s3, |_, _, _, _| 42));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never4_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable());
    let res = scheduler.start(move || s0.zip4_tuple(s1, s2, s3));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never5() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable());
    let res = scheduler.start(move || s0.zip5(s1, s2, s3, s4, |_, _, _, _, _| 42));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never5_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable());
    let res = scheduler.start(move || s0.zip5_tuple(s1, s2, s3, s4));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never6() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4, s5) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable());
    let res = scheduler.start(move || s0.zip6(s1, s2, s3, s4, s5, |_, _, _, _, _, _| 42));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never6_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4, s5) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable());
    let res = scheduler.start(move || s0.zip6_tuple(s1, s2, s3, s4, s5));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never7() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4, s5, s6) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable());
    let res = scheduler.start(move || s0.zip7(s1, s2, s3, s4, s5, s6, |_, _, _, _, _, _, _| 42));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never7_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4, s5, s6) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable());
    let res = scheduler.start(move || s0.zip7_tuple(s1, s2, s3, s4, s5, s6));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never8() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable());
    let res = scheduler.start(move || s0.zip8(s1, s2, s3, s4, s5, s6, s7, |_, _, _, _, _, _, _, _| 42));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never8_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable());
    let res = scheduler.start(move || s0.zip8_tuple(s1, s2, s3, s4, s5, s6, s7));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never9() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable());
    let res = scheduler.start(move || s0.zip9(s1, s2, s3, s4, s5, s6, s7, s8, |_, _, _, _, _, _, _, _, _| 42));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never9_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable());
    let res = scheduler.start(move || s0.zip9_tuple(s1, s2, s3, s4, s5, s6, s7, s8));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never10() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable());
    let res = scheduler.start(move || s0.zip10(s1, s2, s3, s4, s5, s6, s7, s8, s9, |_, _, _, _, _, _, _, _, _, _| 42));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never10_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable());
    let res = scheduler.start(move || s0.zip10_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never11() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable());
    let res = scheduler.start(move || s0.zip11(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, |_, _, _, _, _, _, _, _, _, _, _| 42));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never11_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable());
    let res = scheduler.start(move || s0.zip11_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never12() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable());
    let res = scheduler.start(move || s0.zip12(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, |_, _, _, _, _, _, _, _, _, _, _, _| 42));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never12_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable());
    let res = scheduler.start(move || s0.zip12_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never13() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable());
    let res = scheduler.start(move || s0.zip13(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, |_, _, _, _, _, _, _, _, _, _, _, _, _| 42));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never13_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable());
    let res = scheduler.start(move || s0.zip13_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12).select(|(x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12)| vec![x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12]));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never14() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable());
    let res = scheduler.start(move || s0.zip14(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, |_, _, _, _, _, _, _, _, _, _, _, _, _, _| 42));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never14_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable());
    let res = scheduler.start(move || s0.zip14_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13).select(|(x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13)| vec![x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13]));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never15() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable(), e14.as_observable());
    let res = scheduler.start(move || s0.zip15(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, |_, _, _, _, _, _, _, _, _, _, _, _, _, _, _| 42));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never15_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable(), e14.as_observable());
    let res = scheduler.start(move || s0.zip15_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14).select(|(x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14)| vec![x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14]));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never16() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e15 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable(), e14.as_observable(), e15.as_observable());
    let res = scheduler.start(move || s0.zip16(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15, |_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _| 42));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14, &e15] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_never16_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e15 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable(), e14.as_observable(), e15.as_observable());
    let res = scheduler.start(move || s0.zip16_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15).select(|(x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14, x15)| vec![x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14, x15]));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14, &e15] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn zip_empty2() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);

    let (s0, s1) = (e0.as_observable(), e1.as_observable());
    let res = scheduler.start(move || s0.zip(s1, |_, _| 42));

    assert_messages!(res.messages(), RT::on_completed(220));

    let mut i = 0;
    for e in [&e0, &e1] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty2_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);

    let (s0, s1) = (e0.as_observable(), e1.as_observable());
    let res = scheduler.start(move || s0.zip_tuple(s1));

    assert_messages!(res.messages(), RT::on_completed(220));

    let mut i = 0;
    for e in [&e0, &e1] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty3() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);

    let (s0, s1, s2) = (e0.as_observable(), e1.as_observable(), e2.as_observable());
    let res = scheduler.start(move || s0.zip3(s1, s2, |_, _, _| 42));

    assert_messages!(res.messages(), RT::on_completed(230));

    let mut i = 0;
    for e in [&e0, &e1, &e2] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty3_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);

    let (s0, s1, s2) = (e0.as_observable(), e1.as_observable(), e2.as_observable());
    let res = scheduler.start(move || s0.zip3_tuple(s1, s2));

    assert_messages!(res.messages(), RT::on_completed(230));

    let mut i = 0;
    for e in [&e0, &e1, &e2] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty4() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);

    let (s0, s1, s2, s3) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable());
    let res = scheduler.start(move || s0.zip4(s1, s2, s3, |_, _, _, _| 42));

    assert_messages!(res.messages(), RT::on_completed(240));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty4_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);

    let (s0, s1, s2, s3) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable());
    let res = scheduler.start(move || s0.zip4_tuple(s1, s2, s3));

    assert_messages!(res.messages(), RT::on_completed(240));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty5() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);

    let (s0, s1, s2, s3, s4) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable());
    let res = scheduler.start(move || s0.zip5(s1, s2, s3, s4, |_, _, _, _, _| 42));

    assert_messages!(res.messages(), RT::on_completed(250));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty5_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);

    let (s0, s1, s2, s3, s4) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable());
    let res = scheduler.start(move || s0.zip5_tuple(s1, s2, s3, s4));

    assert_messages!(res.messages(), RT::on_completed(250));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty6() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);

    let (s0, s1, s2, s3, s4, s5) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable());
    let res = scheduler.start(move || s0.zip6(s1, s2, s3, s4, s5, |_, _, _, _, _, _| 42));

    assert_messages!(res.messages(), RT::on_completed(260));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty6_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);

    let (s0, s1, s2, s3, s4, s5) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable());
    let res = scheduler.start(move || s0.zip6_tuple(s1, s2, s3, s4, s5));

    assert_messages!(res.messages(), RT::on_completed(260));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty7() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);

    let (s0, s1, s2, s3, s4, s5, s6) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable());
    let res = scheduler.start(move || s0.zip7(s1, s2, s3, s4, s5, s6, |_, _, _, _, _, _, _| 42));

    assert_messages!(res.messages(), RT::on_completed(270));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty7_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);

    let (s0, s1, s2, s3, s4, s5, s6) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable());
    let res = scheduler.start(move || s0.zip7_tuple(s1, s2, s3, s4, s5, s6));

    assert_messages!(res.messages(), RT::on_completed(270));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty8() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable());
    let res = scheduler.start(move || s0.zip8(s1, s2, s3, s4, s5, s6, s7, |_, _, _, _, _, _, _, _| 42));

    assert_messages!(res.messages(), RT::on_completed(280));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty8_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable());
    let res = scheduler.start(move || s0.zip8_tuple(s1, s2, s3, s4, s5, s6, s7));

    assert_messages!(res.messages(), RT::on_completed(280));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty9() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable());
    let res = scheduler.start(move || s0.zip9(s1, s2, s3, s4, s5, s6, s7, s8, |_, _, _, _, _, _, _, _, _| 42));

    assert_messages!(res.messages(), RT::on_completed(290));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty9_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable());
    let res = scheduler.start(move || s0.zip9_tuple(s1, s2, s3, s4, s5, s6, s7, s8));

    assert_messages!(res.messages(), RT::on_completed(290));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty10() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable());
    let res = scheduler.start(move || s0.zip10(s1, s2, s3, s4, s5, s6, s7, s8, s9, |_, _, _, _, _, _, _, _, _, _| 42));

    assert_messages!(res.messages(), RT::on_completed(300));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty10_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable());
    let res = scheduler.start(move || s0.zip10_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9));

    assert_messages!(res.messages(), RT::on_completed(300));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty11() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable());
    let res = scheduler.start(move || s0.zip11(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, |_, _, _, _, _, _, _, _, _, _, _| 42));

    assert_messages!(res.messages(), RT::on_completed(310));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty11_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable());
    let res = scheduler.start(move || s0.zip11_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10));

    assert_messages!(res.messages(), RT::on_completed(310));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty12() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable());
    let res = scheduler.start(move || s0.zip12(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, |_, _, _, _, _, _, _, _, _, _, _, _| 42));

    assert_messages!(res.messages(), RT::on_completed(320));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty12_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable());
    let res = scheduler.start(move || s0.zip12_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11));

    assert_messages!(res.messages(), RT::on_completed(320));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty13() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(330)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable());
    let res = scheduler.start(move || s0.zip13(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, |_, _, _, _, _, _, _, _, _, _, _, _, _| 42));

    assert_messages!(res.messages(), RT::on_completed(330));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty13_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(330)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable());
    let res = scheduler.start(move || s0.zip13_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12).select(|(x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12)| vec![x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12]));

    assert_messages!(res.messages(), RT::on_completed(330));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty14() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(330)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(340)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable());
    let res = scheduler.start(move || s0.zip14(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, |_, _, _, _, _, _, _, _, _, _, _, _, _, _| 42));

    assert_messages!(res.messages(), RT::on_completed(340));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty14_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(330)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(340)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable());
    let res = scheduler.start(move || s0.zip14_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13).select(|(x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13)| vec![x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13]));

    assert_messages!(res.messages(), RT::on_completed(340));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty15() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(330)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(340)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(350)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable(), e14.as_observable());
    let res = scheduler.start(move || s0.zip15(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, |_, _, _, _, _, _, _, _, _, _, _, _, _, _, _| 42));

    assert_messages!(res.messages(), RT::on_completed(350));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty15_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(330)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(340)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(350)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable(), e14.as_observable());
    let res = scheduler.start(move || s0.zip15_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14).select(|(x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14)| vec![x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14]));

    assert_messages!(res.messages(), RT::on_completed(350));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty16() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(330)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(340)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(350)]);
    let e15 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(360)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable(), e14.as_observable(), e15.as_observable());
    let res = scheduler.start(move || s0.zip16(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15, |_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _| 42));

    assert_messages!(res.messages(), RT::on_completed(360));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14, &e15] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_empty16_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(330)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(340)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(350)]);
    let e15 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(360)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable(), e14.as_observable(), e15.as_observable());
    let res = scheduler.start(move || s0.zip16_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15).select(|(x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14, x15)| vec![x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14, x15]));

    assert_messages!(res.messages(), RT::on_completed(360));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14, &e15] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn zip_symmetric_return2() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);

    let (s0, s1) = (e0.as_observable(), e1.as_observable());
    let res = scheduler.start(move || s0.zip(s1, |x0, x1| x0 + x1));

    assert_messages!(res.messages(), RT::on_next(220, 3), RT::on_completed(400));

    for e in [&e0, &e1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return2_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);

    let (s0, s1) = (e0.as_observable(), e1.as_observable());
    let res = scheduler.start(move || s0.zip_tuple(s1));

    assert_messages!(res.messages(), RT::on_next(220, (1, 2)), RT::on_completed(400));

    for e in [&e0, &e1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return3() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);

    let (s0, s1, s2) = (e0.as_observable(), e1.as_observable(), e2.as_observable());
    let res = scheduler.start(move || s0.zip3(s1, s2, |x0, x1, x2| x0 + x1 + x2));

    assert_messages!(res.messages(), RT::on_next(230, 6), RT::on_completed(400));

    for e in [&e0, &e1, &e2] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return3_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);

    let (s0, s1, s2) = (e0.as_observable(), e1.as_observable(), e2.as_observable());
    let res = scheduler.start(move || s0.zip3_tuple(s1, s2));

    assert_messages!(res.messages(), RT::on_next(230, (1, 2, 3)), RT::on_completed(400));

    for e in [&e0, &e1, &e2] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return4() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);

    let (s0, s1, s2, s3) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable());
    let res = scheduler.start(move || s0.zip4(s1, s2, s3, |x0, x1, x2, x3| x0 + x1 + x2 + x3));

    assert_messages!(res.messages(), RT::on_next(240, 10), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return4_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);

    let (s0, s1, s2, s3) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable());
    let res = scheduler.start(move || s0.zip4_tuple(s1, s2, s3));

    assert_messages!(res.messages(), RT::on_next(240, (1, 2, 3, 4)), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return5() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable());
    let res = scheduler.start(move || s0.zip5(s1, s2, s3, s4, |x0, x1, x2, x3, x4| x0 + x1 + x2 + x3 + x4));

    assert_messages!(res.messages(), RT::on_next(250, 15), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return5_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable());
    let res = scheduler.start(move || s0.zip5_tuple(s1, s2, s3, s4));

    assert_messages!(res.messages(), RT::on_next(250, (1, 2, 3, 4, 5)), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return6() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4, s5) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable());
    let res = scheduler.start(move || s0.zip6(s1, s2, s3, s4, s5, |x0, x1, x2, x3, x4, x5| x0 + x1 + x2 + x3 + x4 + x5));

    assert_messages!(res.messages(), RT::on_next(260, 21), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4, &e5] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return6_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4, s5) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable());
    let res = scheduler.start(move || s0.zip6_tuple(s1, s2, s3, s4, s5));

    assert_messages!(res.messages(), RT::on_next(260, (1, 2, 3, 4, 5, 6)), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4, &e5] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return7() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4, s5, s6) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable());
    let res = scheduler.start(move || s0.zip7(s1, s2, s3, s4, s5, s6, |x0, x1, x2, x3, x4, x5, x6| x0 + x1 + x2 + x3 + x4 + x5 + x6));

    assert_messages!(res.messages(), RT::on_next(270, 28), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return7_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4, s5, s6) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable());
    let res = scheduler.start(move || s0.zip7_tuple(s1, s2, s3, s4, s5, s6));

    assert_messages!(res.messages(), RT::on_next(270, (1, 2, 3, 4, 5, 6, 7)), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return8() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable());
    let res = scheduler.start(move || s0.zip8(s1, s2, s3, s4, s5, s6, s7, |x0, x1, x2, x3, x4, x5, x6, x7| x0 + x1 + x2 + x3 + x4 + x5 + x6 + x7));

    assert_messages!(res.messages(), RT::on_next(280, 36), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return8_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable());
    let res = scheduler.start(move || s0.zip8_tuple(s1, s2, s3, s4, s5, s6, s7));

    assert_messages!(res.messages(), RT::on_next(280, (1, 2, 3, 4, 5, 6, 7, 8)), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return9() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable());
    let res = scheduler.start(move || s0.zip9(s1, s2, s3, s4, s5, s6, s7, s8, |x0, x1, x2, x3, x4, x5, x6, x7, x8| x0 + x1 + x2 + x3 + x4 + x5 + x6 + x7 + x8));

    assert_messages!(res.messages(), RT::on_next(290, 45), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return9_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable());
    let res = scheduler.start(move || s0.zip9_tuple(s1, s2, s3, s4, s5, s6, s7, s8));

    assert_messages!(res.messages(), RT::on_next(290, (1, 2, 3, 4, 5, 6, 7, 8, 9)), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return10() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable());
    let res = scheduler.start(move || s0.zip10(s1, s2, s3, s4, s5, s6, s7, s8, s9, |x0, x1, x2, x3, x4, x5, x6, x7, x8, x9| x0 + x1 + x2 + x3 + x4 + x5 + x6 + x7 + x8 + x9));

    assert_messages!(res.messages(), RT::on_next(300, 55), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return10_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable());
    let res = scheduler.start(move || s0.zip10_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9));

    assert_messages!(res.messages(), RT::on_next(300, (1, 2, 3, 4, 5, 6, 7, 8, 9, 10)), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return11() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_completed(400)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable());
    let res = scheduler.start(move || s0.zip11(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, |x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10| x0 + x1 + x2 + x3 + x4 + x5 + x6 + x7 + x8 + x9 + x10));

    assert_messages!(res.messages(), RT::on_next(310, 66), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return11_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_completed(400)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable());
    let res = scheduler.start(move || s0.zip11_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10));

    assert_messages!(res.messages(), RT::on_next(310, (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11)), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return12() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_completed(400)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_completed(400)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable());
    let res = scheduler.start(move || s0.zip12(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, |x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11| x0 + x1 + x2 + x3 + x4 + x5 + x6 + x7 + x8 + x9 + x10 + x11));

    assert_messages!(res.messages(), RT::on_next(320, 78), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return12_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_completed(400)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_completed(400)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable());
    let res = scheduler.start(move || s0.zip12_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11));

    assert_messages!(res.messages(), RT::on_next(320, (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12)), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return13() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_completed(400)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_completed(400)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_completed(400)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(330, 13), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable());
    let res = scheduler.start(move || s0.zip13(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, |x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12| x0 + x1 + x2 + x3 + x4 + x5 + x6 + x7 + x8 + x9 + x10 + x11 + x12));

    assert_messages!(res.messages(), RT::on_next(330, 91), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return13_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_completed(400)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_completed(400)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_completed(400)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(330, 13), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable());
    let res = scheduler.start(move || s0.zip13_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12).select(|(x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12)| vec![x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12]));

    assert_messages!(res.messages(), RT::on_next(330, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return14() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_completed(400)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_completed(400)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_completed(400)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(330, 13), RT::on_completed(400)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(340, 14), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable());
    let res = scheduler.start(move || s0.zip14(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, |x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13| x0 + x1 + x2 + x3 + x4 + x5 + x6 + x7 + x8 + x9 + x10 + x11 + x12 + x13));

    assert_messages!(res.messages(), RT::on_next(340, 105), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return14_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_completed(400)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_completed(400)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_completed(400)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(330, 13), RT::on_completed(400)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(340, 14), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable());
    let res = scheduler.start(move || s0.zip14_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13).select(|(x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13)| vec![x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13]));

    assert_messages!(res.messages(), RT::on_next(340, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return15() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_completed(400)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_completed(400)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_completed(400)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(330, 13), RT::on_completed(400)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(340, 14), RT::on_completed(400)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(350, 15), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable(), e14.as_observable());
    let res = scheduler.start(move || s0.zip15(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, |x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14| x0 + x1 + x2 + x3 + x4 + x5 + x6 + x7 + x8 + x9 + x10 + x11 + x12 + x13 + x14));

    assert_messages!(res.messages(), RT::on_next(350, 120), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return15_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_completed(400)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_completed(400)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_completed(400)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(330, 13), RT::on_completed(400)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(340, 14), RT::on_completed(400)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(350, 15), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable(), e14.as_observable());
    let res = scheduler.start(move || s0.zip15_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14).select(|(x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14)| vec![x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14]));

    assert_messages!(res.messages(), RT::on_next(350, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return16() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_completed(400)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_completed(400)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_completed(400)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(330, 13), RT::on_completed(400)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(340, 14), RT::on_completed(400)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(350, 15), RT::on_completed(400)]);
    let e15 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(360, 16), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable(), e14.as_observable(), e15.as_observable());
    let res = scheduler.start(move || s0.zip16(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15, |x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14, x15| x0 + x1 + x2 + x3 + x4 + x5 + x6 + x7 + x8 + x9 + x10 + x11 + x12 + x13 + x14 + x15));

    assert_messages!(res.messages(), RT::on_next(360, 136), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14, &e15] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_symmetric_return16_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_completed(400)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_completed(400)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_completed(400)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(330, 13), RT::on_completed(400)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(340, 14), RT::on_completed(400)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(350, 15), RT::on_completed(400)]);
    let e15 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(360, 16), RT::on_completed(400)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable(), e14.as_observable(), e15.as_observable());
    let res = scheduler.start(move || s0.zip16_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15).select(|(x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14, x15)| vec![x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14, x15]));

    assert_messages!(res.messages(), RT::on_next(360, vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]), RT::on_completed(400));

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14, &e15] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 400));
    }
}

#[test]
fn zip_selector_throws2() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);

    let ex = new_error("ex");

    let (s0, s1) = (e0.as_observable(), e1.as_observable());
    let ex1 = ex.clone();
    let res = scheduler.start(move || s0.try_zip(s1, move |_, _| Err::<i32, _>(ex1.clone())));

    assert_messages!(res.messages(), RT::on_error(220, ex.clone()));

    let es = [&e0, &e1];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + es.len() as i64 * 10));
    }
}

#[test]
fn zip_selector_throws3() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);

    let ex = new_error("ex");

    let (s0, s1, s2) = (e0.as_observable(), e1.as_observable(), e2.as_observable());
    let ex1 = ex.clone();
    let res = scheduler.start(move || s0.try_zip3(s1, s2, move |_, _, _| Err::<i32, _>(ex1.clone())));

    assert_messages!(res.messages(), RT::on_error(230, ex.clone()));

    let es = [&e0, &e1, &e2];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + es.len() as i64 * 10));
    }
}

#[test]
fn zip_selector_throws4() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);

    let ex = new_error("ex");

    let (s0, s1, s2, s3) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable());
    let ex1 = ex.clone();
    let res = scheduler.start(move || s0.try_zip4(s1, s2, s3, move |_, _, _, _| Err::<i32, _>(ex1.clone())));

    assert_messages!(res.messages(), RT::on_error(240, ex.clone()));

    let es = [&e0, &e1, &e2, &e3];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + es.len() as i64 * 10));
    }
}

#[test]
fn zip_selector_throws5() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);

    let ex = new_error("ex");

    let (s0, s1, s2, s3, s4) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable());
    let ex1 = ex.clone();
    let res = scheduler.start(move || s0.try_zip5(s1, s2, s3, s4, move |_, _, _, _, _| Err::<i32, _>(ex1.clone())));

    assert_messages!(res.messages(), RT::on_error(250, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + es.len() as i64 * 10));
    }
}

#[test]
fn zip_selector_throws6() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);

    let ex = new_error("ex");

    let (s0, s1, s2, s3, s4, s5) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable());
    let ex1 = ex.clone();
    let res = scheduler.start(move || s0.try_zip6(s1, s2, s3, s4, s5, move |_, _, _, _, _, _| Err::<i32, _>(ex1.clone())));

    assert_messages!(res.messages(), RT::on_error(260, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + es.len() as i64 * 10));
    }
}

#[test]
fn zip_selector_throws7() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);

    let ex = new_error("ex");

    let (s0, s1, s2, s3, s4, s5, s6) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable());
    let ex1 = ex.clone();
    let res = scheduler.start(move || s0.try_zip7(s1, s2, s3, s4, s5, s6, move |_, _, _, _, _, _, _| Err::<i32, _>(ex1.clone())));

    assert_messages!(res.messages(), RT::on_error(270, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + es.len() as i64 * 10));
    }
}

#[test]
fn zip_selector_throws8() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);

    let ex = new_error("ex");

    let (s0, s1, s2, s3, s4, s5, s6, s7) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable());
    let ex1 = ex.clone();
    let res = scheduler.start(move || s0.try_zip8(s1, s2, s3, s4, s5, s6, s7, move |_, _, _, _, _, _, _, _| Err::<i32, _>(ex1.clone())));

    assert_messages!(res.messages(), RT::on_error(280, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + es.len() as i64 * 10));
    }
}

#[test]
fn zip_selector_throws9() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);

    let ex = new_error("ex");

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable());
    let ex1 = ex.clone();
    let res = scheduler.start(move || s0.try_zip9(s1, s2, s3, s4, s5, s6, s7, s8, move |_, _, _, _, _, _, _, _, _| Err::<i32, _>(ex1.clone())));

    assert_messages!(res.messages(), RT::on_error(290, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + es.len() as i64 * 10));
    }
}

#[test]
fn zip_selector_throws10() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_completed(400)]);

    let ex = new_error("ex");

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable());
    let ex1 = ex.clone();
    let res = scheduler.start(move || s0.try_zip10(s1, s2, s3, s4, s5, s6, s7, s8, s9, move |_, _, _, _, _, _, _, _, _, _| Err::<i32, _>(ex1.clone())));

    assert_messages!(res.messages(), RT::on_error(300, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + es.len() as i64 * 10));
    }
}

#[test]
fn zip_selector_throws11() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_completed(400)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_completed(400)]);

    let ex = new_error("ex");

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable());
    let ex1 = ex.clone();
    let res = scheduler.start(move || s0.try_zip11(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, move |_, _, _, _, _, _, _, _, _, _, _| Err::<i32, _>(ex1.clone())));

    assert_messages!(res.messages(), RT::on_error(310, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + es.len() as i64 * 10));
    }
}

#[test]
fn zip_selector_throws12() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_completed(400)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_completed(400)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_completed(400)]);

    let ex = new_error("ex");

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable());
    let ex1 = ex.clone();
    let res = scheduler.start(move || s0.try_zip12(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, move |_, _, _, _, _, _, _, _, _, _, _, _| Err::<i32, _>(ex1.clone())));

    assert_messages!(res.messages(), RT::on_error(320, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + es.len() as i64 * 10));
    }
}

#[test]
fn zip_selector_throws13() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_completed(400)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_completed(400)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_completed(400)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(330, 13), RT::on_completed(400)]);

    let ex = new_error("ex");

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable());
    let ex1 = ex.clone();
    let res = scheduler.start(move || s0.try_zip13(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, move |_, _, _, _, _, _, _, _, _, _, _, _, _| Err::<i32, _>(ex1.clone())));

    assert_messages!(res.messages(), RT::on_error(330, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + es.len() as i64 * 10));
    }
}

#[test]
fn zip_selector_throws14() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_completed(400)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_completed(400)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_completed(400)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(330, 13), RT::on_completed(400)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(340, 14), RT::on_completed(400)]);

    let ex = new_error("ex");

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable());
    let ex1 = ex.clone();
    let res = scheduler.start(move || s0.try_zip14(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, move |_, _, _, _, _, _, _, _, _, _, _, _, _, _| Err::<i32, _>(ex1.clone())));

    assert_messages!(res.messages(), RT::on_error(340, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + es.len() as i64 * 10));
    }
}

#[test]
fn zip_selector_throws15() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_completed(400)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_completed(400)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_completed(400)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(330, 13), RT::on_completed(400)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(340, 14), RT::on_completed(400)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(350, 15), RT::on_completed(400)]);

    let ex = new_error("ex");

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable(), e14.as_observable());
    let ex1 = ex.clone();
    let res = scheduler.start(move || s0.try_zip15(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, move |_, _, _, _, _, _, _, _, _, _, _, _, _, _, _| Err::<i32, _>(ex1.clone())));

    assert_messages!(res.messages(), RT::on_error(350, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + es.len() as i64 * 10));
    }
}

#[test]
fn zip_selector_throws16() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_completed(400)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_completed(400)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_completed(400)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_completed(400)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_completed(400)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(330, 13), RT::on_completed(400)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(340, 14), RT::on_completed(400)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(350, 15), RT::on_completed(400)]);
    let e15 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(360, 16), RT::on_completed(400)]);

    let ex = new_error("ex");

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable(), e14.as_observable(), e15.as_observable());
    let ex1 = ex.clone();
    let res = scheduler.start(move || s0.try_zip16(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15, move |_, _, _, _, _, _, _, _, _, _, _, _, _, _, _, _| Err::<i32, _>(ex1.clone())));

    assert_messages!(res.messages(), RT::on_error(360, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14, &e15];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + es.len() as i64 * 10));
    }
}

#[test]
fn zip_all_completed2() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);

    let (s0, s1) = (e0.as_observable(), e1.as_observable());
    let res = scheduler.start(move || s0.zip(s1, |x0, x1| x0 + x1));

    assert_messages!(res.messages(), RT::on_next(210, 10), RT::on_completed(220));

    let es = [&e0, &e1];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed2_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);

    let (s0, s1) = (e0.as_observable(), e1.as_observable());
    let res = scheduler.start(move || s0.zip_tuple(s1));

    assert_messages!(res.messages(), RT::on_next(210, (5, 5)), RT::on_completed(220));

    let es = [&e0, &e1];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed3() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);

    let (s0, s1, s2) = (e0.as_observable(), e1.as_observable(), e2.as_observable());
    let res = scheduler.start(move || s0.zip3(s1, s2, |x0, x1, x2| x0 + x1 + x2));

    assert_messages!(res.messages(), RT::on_next(210, 15), RT::on_completed(230));

    let es = [&e0, &e1, &e2];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed3_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);

    let (s0, s1, s2) = (e0.as_observable(), e1.as_observable(), e2.as_observable());
    let res = scheduler.start(move || s0.zip3_tuple(s1, s2));

    assert_messages!(res.messages(), RT::on_next(210, (5, 5, 5)), RT::on_completed(230));

    let es = [&e0, &e1, &e2];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed4() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);

    let (s0, s1, s2, s3) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable());
    let res = scheduler.start(move || s0.zip4(s1, s2, s3, |x0, x1, x2, x3| x0 + x1 + x2 + x3));

    assert_messages!(res.messages(), RT::on_next(210, 20), RT::on_completed(240));

    let es = [&e0, &e1, &e2, &e3];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed4_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);

    let (s0, s1, s2, s3) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable());
    let res = scheduler.start(move || s0.zip4_tuple(s1, s2, s3));

    assert_messages!(res.messages(), RT::on_next(210, (5, 5, 5, 5)), RT::on_completed(240));

    let es = [&e0, &e1, &e2, &e3];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed5() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);

    let (s0, s1, s2, s3, s4) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable());
    let res = scheduler.start(move || s0.zip5(s1, s2, s3, s4, |x0, x1, x2, x3, x4| x0 + x1 + x2 + x3 + x4));

    assert_messages!(res.messages(), RT::on_next(210, 25), RT::on_completed(250));

    let es = [&e0, &e1, &e2, &e3, &e4];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed5_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);

    let (s0, s1, s2, s3, s4) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable());
    let res = scheduler.start(move || s0.zip5_tuple(s1, s2, s3, s4));

    assert_messages!(res.messages(), RT::on_next(210, (5, 5, 5, 5, 5)), RT::on_completed(250));

    let es = [&e0, &e1, &e2, &e3, &e4];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed6() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_completed(270)]);

    let (s0, s1, s2, s3, s4, s5) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable());
    let res = scheduler.start(move || s0.zip6(s1, s2, s3, s4, s5, |x0, x1, x2, x3, x4, x5| x0 + x1 + x2 + x3 + x4 + x5));

    assert_messages!(res.messages(), RT::on_next(210, 30), RT::on_completed(260));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed6_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_completed(270)]);

    let (s0, s1, s2, s3, s4, s5) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable());
    let res = scheduler.start(move || s0.zip6_tuple(s1, s2, s3, s4, s5));

    assert_messages!(res.messages(), RT::on_next(210, (5, 5, 5, 5, 5, 5)), RT::on_completed(260));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed7() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_completed(270)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_completed(280)]);

    let (s0, s1, s2, s3, s4, s5, s6) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable());
    let res = scheduler.start(move || s0.zip7(s1, s2, s3, s4, s5, s6, |x0, x1, x2, x3, x4, x5, x6| x0 + x1 + x2 + x3 + x4 + x5 + x6));

    assert_messages!(res.messages(), RT::on_next(210, 35), RT::on_completed(270));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed7_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_completed(270)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_completed(280)]);

    let (s0, s1, s2, s3, s4, s5, s6) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable());
    let res = scheduler.start(move || s0.zip7_tuple(s1, s2, s3, s4, s5, s6));

    assert_messages!(res.messages(), RT::on_next(210, (5, 5, 5, 5, 5, 5, 5)), RT::on_completed(270));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed8() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_completed(270)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_completed(280)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_completed(290)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable());
    let res = scheduler.start(move || s0.zip8(s1, s2, s3, s4, s5, s6, s7, |x0, x1, x2, x3, x4, x5, x6, x7| x0 + x1 + x2 + x3 + x4 + x5 + x6 + x7));

    assert_messages!(res.messages(), RT::on_next(210, 40), RT::on_completed(280));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed8_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_completed(270)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_completed(280)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_completed(290)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable());
    let res = scheduler.start(move || s0.zip8_tuple(s1, s2, s3, s4, s5, s6, s7));

    assert_messages!(res.messages(), RT::on_next(210, (5, 5, 5, 5, 5, 5, 5, 5)), RT::on_completed(280));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed9() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_completed(270)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_completed(280)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_completed(290)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_completed(300)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable());
    let res = scheduler.start(move || s0.zip9(s1, s2, s3, s4, s5, s6, s7, s8, |x0, x1, x2, x3, x4, x5, x6, x7, x8| x0 + x1 + x2 + x3 + x4 + x5 + x6 + x7 + x8));

    assert_messages!(res.messages(), RT::on_next(210, 45), RT::on_completed(290));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed9_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_completed(270)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_completed(280)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_completed(290)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_completed(300)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable());
    let res = scheduler.start(move || s0.zip9_tuple(s1, s2, s3, s4, s5, s6, s7, s8));

    assert_messages!(res.messages(), RT::on_next(210, (5, 5, 5, 5, 5, 5, 5, 5, 5)), RT::on_completed(290));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed10() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_completed(270)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_completed(280)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_completed(290)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_completed(300)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_completed(310)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable());
    let res = scheduler.start(move || s0.zip10(s1, s2, s3, s4, s5, s6, s7, s8, s9, |x0, x1, x2, x3, x4, x5, x6, x7, x8, x9| x0 + x1 + x2 + x3 + x4 + x5 + x6 + x7 + x8 + x9));

    assert_messages!(res.messages(), RT::on_next(210, 50), RT::on_completed(300));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed10_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_completed(270)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_completed(280)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_completed(290)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_completed(300)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_completed(310)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable());
    let res = scheduler.start(move || s0.zip10_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9));

    assert_messages!(res.messages(), RT::on_next(210, (5, 5, 5, 5, 5, 5, 5, 5, 5, 5)), RT::on_completed(300));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed11() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_completed(270)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_completed(280)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_completed(290)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_completed(300)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_completed(310)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_completed(320)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable());
    let res = scheduler.start(move || s0.zip11(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, |x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10| x0 + x1 + x2 + x3 + x4 + x5 + x6 + x7 + x8 + x9 + x10));

    assert_messages!(res.messages(), RT::on_next(210, 55), RT::on_completed(310));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed11_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_completed(270)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_completed(280)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_completed(290)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_completed(300)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_completed(310)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_completed(320)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable());
    let res = scheduler.start(move || s0.zip11_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10));

    assert_messages!(res.messages(), RT::on_next(210, (5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5)), RT::on_completed(310));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed12() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_completed(270)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_completed(280)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_completed(290)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_completed(300)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_completed(310)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_completed(320)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_completed(330)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable());
    let res = scheduler.start(move || s0.zip12(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, |x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11| x0 + x1 + x2 + x3 + x4 + x5 + x6 + x7 + x8 + x9 + x10 + x11));

    assert_messages!(res.messages(), RT::on_next(210, 60), RT::on_completed(320));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed12_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_completed(270)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_completed(280)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_completed(290)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_completed(300)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_completed(310)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_completed(320)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_completed(330)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable());
    let res = scheduler.start(move || s0.zip12_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11));

    assert_messages!(res.messages(), RT::on_next(210, (5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5)), RT::on_completed(320));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed13() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_completed(270)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_completed(280)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_completed(290)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_completed(300)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_completed(310)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_completed(320)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_completed(330)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_next(330, 17), RT::on_completed(340)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable());
    let res = scheduler.start(move || s0.zip13(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, |x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12| x0 + x1 + x2 + x3 + x4 + x5 + x6 + x7 + x8 + x9 + x10 + x11 + x12));

    assert_messages!(res.messages(), RT::on_next(210, 65), RT::on_completed(330));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed13_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_completed(270)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_completed(280)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_completed(290)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_completed(300)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_completed(310)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_completed(320)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_completed(330)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_next(330, 17), RT::on_completed(340)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable());
    let res = scheduler.start(move || s0.zip13_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12).select(|(x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12)| vec![x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12]));

    assert_messages!(res.messages(), RT::on_next(210, vec![5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5]), RT::on_completed(330));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed14() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_completed(270)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_completed(280)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_completed(290)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_completed(300)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_completed(310)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_completed(320)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_completed(330)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_next(330, 17), RT::on_completed(340)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_next(330, 17), RT::on_next(340, 18), RT::on_completed(350)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable());
    let res = scheduler.start(move || s0.zip14(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, |x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13| x0 + x1 + x2 + x3 + x4 + x5 + x6 + x7 + x8 + x9 + x10 + x11 + x12 + x13));

    assert_messages!(res.messages(), RT::on_next(210, 70), RT::on_completed(340));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed14_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_completed(270)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_completed(280)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_completed(290)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_completed(300)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_completed(310)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_completed(320)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_completed(330)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_next(330, 17), RT::on_completed(340)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_next(330, 17), RT::on_next(340, 18), RT::on_completed(350)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable());
    let res = scheduler.start(move || s0.zip14_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13).select(|(x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13)| vec![x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13]));

    assert_messages!(res.messages(), RT::on_next(210, vec![5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5]), RT::on_completed(340));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed15() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_completed(270)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_completed(280)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_completed(290)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_completed(300)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_completed(310)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_completed(320)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_completed(330)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_next(330, 17), RT::on_completed(340)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_next(330, 17), RT::on_next(340, 18), RT::on_completed(350)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_next(330, 17), RT::on_next(340, 18), RT::on_next(350, 19), RT::on_completed(360)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable(), e14.as_observable());
    let res = scheduler.start(move || s0.zip15(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, |x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14| x0 + x1 + x2 + x3 + x4 + x5 + x6 + x7 + x8 + x9 + x10 + x11 + x12 + x13 + x14));

    assert_messages!(res.messages(), RT::on_next(210, 75), RT::on_completed(350));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed15_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_completed(270)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_completed(280)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_completed(290)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_completed(300)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_completed(310)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_completed(320)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_completed(330)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_next(330, 17), RT::on_completed(340)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_next(330, 17), RT::on_next(340, 18), RT::on_completed(350)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_next(330, 17), RT::on_next(340, 18), RT::on_next(350, 19), RT::on_completed(360)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable(), e14.as_observable());
    let res = scheduler.start(move || s0.zip15_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14).select(|(x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14)| vec![x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14]));

    assert_messages!(res.messages(), RT::on_next(210, vec![5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5]), RT::on_completed(350));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed16() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_completed(270)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_completed(280)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_completed(290)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_completed(300)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_completed(310)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_completed(320)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_completed(330)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_next(330, 17), RT::on_completed(340)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_next(330, 17), RT::on_next(340, 18), RT::on_completed(350)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_next(330, 17), RT::on_next(340, 18), RT::on_next(350, 19), RT::on_completed(360)]);
    let e15 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_next(330, 17), RT::on_next(340, 18), RT::on_next(350, 19), RT::on_next(360, 20), RT::on_completed(370)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable(), e14.as_observable(), e15.as_observable());
    let res = scheduler.start(move || s0.zip16(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15, |x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14, x15| x0 + x1 + x2 + x3 + x4 + x5 + x6 + x7 + x8 + x9 + x10 + x11 + x12 + x13 + x14 + x15));

    assert_messages!(res.messages(), RT::on_next(210, 80), RT::on_completed(360));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14, &e15];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}

#[test]
fn zip_all_completed16_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_completed(220)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_completed(230)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_completed(240)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_completed(250)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_completed(260)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_completed(270)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_completed(280)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_completed(290)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_completed(300)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_completed(310)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_completed(320)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_completed(330)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_next(330, 17), RT::on_completed(340)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_next(330, 17), RT::on_next(340, 18), RT::on_completed(350)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_next(330, 17), RT::on_next(340, 18), RT::on_next(350, 19), RT::on_completed(360)]);
    let e15 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 5), RT::on_next(220, 6), RT::on_next(230, 7), RT::on_next(240, 8), RT::on_next(250, 9), RT::on_next(260, 10), RT::on_next(270, 11), RT::on_next(280, 12), RT::on_next(290, 13), RT::on_next(300, 14), RT::on_next(310, 15), RT::on_next(320, 16), RT::on_next(330, 17), RT::on_next(340, 18), RT::on_next(350, 19), RT::on_next(360, 20), RT::on_completed(370)]);

    let (s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15) = (e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable(), e6.as_observable(), e7.as_observable(), e8.as_observable(), e9.as_observable(), e10.as_observable(), e11.as_observable(), e12.as_observable(), e13.as_observable(), e14.as_observable(), e15.as_observable());
    let res = scheduler.start(move || s0.zip16_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15).select(|(x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14, x15)| vec![x0, x1, x2, x3, x4, x5, x6, x7, x8, x9, x10, x11, x12, x13, x14, x15]));

    assert_messages!(res.messages(), RT::on_next(210, vec![5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5]), RT::on_completed(360));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14, &e15];

    let mut i = 0;
    for e in &es[..es.len() - 1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 220 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es[es.len() - 1].subscriptions(), RT::subscribe(200, 220 + (i - 1) * 10));
}
