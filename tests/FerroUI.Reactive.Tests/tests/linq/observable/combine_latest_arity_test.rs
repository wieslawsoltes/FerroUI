//! Ported from `Tests.System.Reactive/Tests/Linq/Observable/CombineLatestTest.cs` of Rx.NET: the tests
//! of each number of sources (regions Never, Empty, SelectorThrows, AllEmptyButOne and Typical); the
//! other tests, and the list of the tests not ported, are in `combine_latest_test.rs`.
//! Adapted: `Observable.CombineLatest(e0, ..., eN, selector)` is `combine_latestN` on the first source
//! (`combine_latest` for two sources), the tuple forms of `ObservableEx` are `combine_latest_tuple`
//! and `combine_latestN_tuple`, and the list form with a selector (the `N` tests) is
//! `Observable::combine_latest_all_with` (a slice for the selector's `IList`). A selector that throws
//! is the selector of the `try_` form returning the error. Tuples of more than 12 values have no
//! `PartialEq` or `Debug` in Rust, so the tuple tests of 13 to 16 sources compare the tuples as arrays
//! (`select(|t| [t.0, ..])`).

use ferroui_reactive::{Observable, ObservableError, ObservableExt};
use ferroui_reactive_testing::{assert_messages, assert_subscriptions, new_error, ReactiveTest as RT, TestScheduler};

#[test]
fn combine_latest_never_n() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable()];

    let res = scheduler.start(move || Observable::combine_latest_all_with(sources, |xs: &[i32]| xs.iter().sum::<i32>()));

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never2() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let sources = [e0.as_observable(), e1.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1] = sources;
        s0.combine_latest(s1, |_0, _1| 42)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never2_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let sources = [e0.as_observable(), e1.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1] = sources;
        s0.combine_latest_tuple(s1)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never3() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2] = sources;
        s0.combine_latest3(s1, s2, |_0, _1, _2| 42)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never3_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2] = sources;
        s0.combine_latest3_tuple(s1, s2)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never4() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3] = sources;
        s0.combine_latest4(s1, s2, s3, |_0, _1, _2, _3| 42)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never4_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3] = sources;
        s0.combine_latest4_tuple(s1, s2, s3)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never5() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4] = sources;
        s0.combine_latest5(s1, s2, s3, s4, |_0, _1, _2, _3, _4| 42)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never5_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4] = sources;
        s0.combine_latest5_tuple(s1, s2, s3, s4)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never6() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5] = sources;
        s0.combine_latest6(s1, s2, s3, s4, s5, |_0, _1, _2, _3, _4, _5| 42)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never6_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5] = sources;
        s0.combine_latest6_tuple(s1, s2, s3, s4, s5)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never7() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6] = sources;
        s0.combine_latest7(s1, s2, s3, s4, s5, s6, |_0, _1, _2, _3, _4, _5, _6| 42)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never7_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6] = sources;
        s0.combine_latest7_tuple(s1, s2, s3, s4, s5, s6)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never8() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7] = sources;
        s0.combine_latest8(s1, s2, s3, s4, s5, s6, s7, |_0, _1, _2, _3, _4, _5, _6, _7| 42)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never8_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7] = sources;
        s0.combine_latest8_tuple(s1, s2, s3, s4, s5, s6, s7)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never9() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8] = sources;
        s0.combine_latest9(s1, s2, s3, s4, s5, s6, s7, s8, |_0, _1, _2, _3, _4, _5, _6, _7, _8| 42)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never9_tuple() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8] = sources;
        s0.combine_latest9_tuple(s1, s2, s3, s4, s5, s6, s7, s8)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never10() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9] = sources;
        s0.combine_latest10(s1, s2, s3, s4, s5, s6, s7, s8, s9, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9| 42)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never10_tuple() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9] = sources;
        s0.combine_latest10_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never11() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10] = sources;
        s0.combine_latest11(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10| 42)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never11_tuple() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10] = sources;
        s0.combine_latest11_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never12() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11] = sources;
        s0.combine_latest12(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11| 42)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never12_tuple() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11] = sources;
        s0.combine_latest12_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never13() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12] = sources;
        s0.combine_latest13(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12| 42)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never13_tuple() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12] = sources;
        s0.combine_latest13_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12).select(|t| [t.0, t.1, t.2, t.3, t.4, t.5, t.6, t.7, t.8, t.9, t.10, t.11, t.12])
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never14() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13] = sources;
        s0.combine_latest14(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13| 42)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never14_tuple() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13] = sources;
        s0.combine_latest14_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13).select(|t| [t.0, t.1, t.2, t.3, t.4, t.5, t.6, t.7, t.8, t.9, t.10, t.11, t.12, t.13])
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never15() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
        e14.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14] = sources;
        s0.combine_latest15(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13, _14| 42)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never15_tuple() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
        e14.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14] = sources;
        s0.combine_latest15_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14).select(|t| [t.0, t.1, t.2, t.3, t.4, t.5, t.6, t.7, t.8, t.9, t.10, t.11, t.12, t.13, t.14])
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never16() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
        e14.as_observable(),
        e15.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15] = sources;
        s0.combine_latest16(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13, _14, _15| 42)
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14, &e15] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_never16_tuple() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
        e14.as_observable(),
        e15.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15] = sources;
        s0.combine_latest16_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15).select(|t| [t.0, t.1, t.2, t.3, t.4, t.5, t.6, t.7, t.8, t.9, t.10, t.11, t.12, t.13, t.14, t.15])
    });

    assert_messages!(res.messages());

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14, &e15] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 1000));
    }
}

#[test]
fn combine_latest_empty_n() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable()];

    let res = scheduler.start(move || Observable::combine_latest_all_with(sources, |xs: &[i32]| xs.iter().sum::<i32>()));

    assert_messages!(res.messages(), RT::on_completed(230));

    let mut i = 0;
    for e in [&e0, &e1, &e2] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty2() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);

    let sources = [e0.as_observable(), e1.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1] = sources;
        s0.combine_latest(s1, |_0, _1| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(220));

    let mut i = 0;
    for e in [&e0, &e1] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty2_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);

    let sources = [e0.as_observable(), e1.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1] = sources;
        s0.combine_latest_tuple(s1)
    });

    assert_messages!(res.messages(), RT::on_completed(220));

    let mut i = 0;
    for e in [&e0, &e1] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty3() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2] = sources;
        s0.combine_latest3(s1, s2, |_0, _1, _2| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(230));

    let mut i = 0;
    for e in [&e0, &e1, &e2] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty3_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2] = sources;
        s0.combine_latest3_tuple(s1, s2)
    });

    assert_messages!(res.messages(), RT::on_completed(230));

    let mut i = 0;
    for e in [&e0, &e1, &e2] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty4() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3] = sources;
        s0.combine_latest4(s1, s2, s3, |_0, _1, _2, _3| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(240));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty4_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3] = sources;
        s0.combine_latest4_tuple(s1, s2, s3)
    });

    assert_messages!(res.messages(), RT::on_completed(240));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty5() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4] = sources;
        s0.combine_latest5(s1, s2, s3, s4, |_0, _1, _2, _3, _4| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(250));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty5_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4] = sources;
        s0.combine_latest5_tuple(s1, s2, s3, s4)
    });

    assert_messages!(res.messages(), RT::on_completed(250));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty6() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5] = sources;
        s0.combine_latest6(s1, s2, s3, s4, s5, |_0, _1, _2, _3, _4, _5| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(260));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty6_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5] = sources;
        s0.combine_latest6_tuple(s1, s2, s3, s4, s5)
    });

    assert_messages!(res.messages(), RT::on_completed(260));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty7() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6] = sources;
        s0.combine_latest7(s1, s2, s3, s4, s5, s6, |_0, _1, _2, _3, _4, _5, _6| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(270));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty7_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6] = sources;
        s0.combine_latest7_tuple(s1, s2, s3, s4, s5, s6)
    });

    assert_messages!(res.messages(), RT::on_completed(270));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty8() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7] = sources;
        s0.combine_latest8(s1, s2, s3, s4, s5, s6, s7, |_0, _1, _2, _3, _4, _5, _6, _7| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(280));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty8_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(210)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(220)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(230)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(240)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7] = sources;
        s0.combine_latest8_tuple(s1, s2, s3, s4, s5, s6, s7)
    });

    assert_messages!(res.messages(), RT::on_completed(280));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty9() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8] = sources;
        s0.combine_latest9(s1, s2, s3, s4, s5, s6, s7, s8, |_0, _1, _2, _3, _4, _5, _6, _7, _8| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(290));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty9_tuple() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8] = sources;
        s0.combine_latest9_tuple(s1, s2, s3, s4, s5, s6, s7, s8)
    });

    assert_messages!(res.messages(), RT::on_completed(290));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty10() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9] = sources;
        s0.combine_latest10(s1, s2, s3, s4, s5, s6, s7, s8, s9, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(300));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty10_tuple() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9] = sources;
        s0.combine_latest10_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9)
    });

    assert_messages!(res.messages(), RT::on_completed(300));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty11() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10] = sources;
        s0.combine_latest11(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(310));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty11_tuple() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10] = sources;
        s0.combine_latest11_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10)
    });

    assert_messages!(res.messages(), RT::on_completed(310));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty12() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11] = sources;
        s0.combine_latest12(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(320));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty12_tuple() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11] = sources;
        s0.combine_latest12_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11)
    });

    assert_messages!(res.messages(), RT::on_completed(320));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty13() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12] = sources;
        s0.combine_latest13(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(330));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty13_tuple() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12] = sources;
        s0.combine_latest13_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12).select(|t| [t.0, t.1, t.2, t.3, t.4, t.5, t.6, t.7, t.8, t.9, t.10, t.11, t.12])
    });

    assert_messages!(res.messages(), RT::on_completed(330));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty14() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13] = sources;
        s0.combine_latest14(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(340));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty14_tuple() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13] = sources;
        s0.combine_latest14_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13).select(|t| [t.0, t.1, t.2, t.3, t.4, t.5, t.6, t.7, t.8, t.9, t.10, t.11, t.12, t.13])
    });

    assert_messages!(res.messages(), RT::on_completed(340));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty15() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
        e14.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14] = sources;
        s0.combine_latest15(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13, _14| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(350));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty15_tuple() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
        e14.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14] = sources;
        s0.combine_latest15_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14).select(|t| [t.0, t.1, t.2, t.3, t.4, t.5, t.6, t.7, t.8, t.9, t.10, t.11, t.12, t.13, t.14])
    });

    assert_messages!(res.messages(), RT::on_completed(350));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty16() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
        e14.as_observable(),
        e15.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15] = sources;
        s0.combine_latest16(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13, _14, _15| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(360));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14, &e15] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_empty16_tuple() {
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

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
        e14.as_observable(),
        e15.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15] = sources;
        s0.combine_latest16_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15).select(|t| [t.0, t.1, t.2, t.3, t.4, t.5, t.6, t.7, t.8, t.9, t.10, t.11, t.12, t.13, t.14, t.15])
    });

    assert_messages!(res.messages(), RT::on_completed(360));

    let mut i = 0;
    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14, &e15] {
        i += 1;
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + i * 10));
    }
}

#[test]
fn combine_latest_selector_throws_n() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);

    let ex = new_error("ex");
    let f = {
        let ex = ex.clone();
        move |_xs: &[i32]| -> Result<i32, ObservableError> { Err(ex.clone()) }
    };

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable()];

    let res = scheduler.start(move || Observable::try_combine_latest_all_with(sources, f));

    assert_messages!(res.messages(), RT::on_error(230, ex.clone()));

    let es = [&e0, &e1, &e2];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 10 * es.len() as i64));
    }
}

#[test]
fn combine_latest_selector_throws2() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);

    let ex = new_error("ex");
    let f = {
        let ex = ex.clone();
        move || -> Result<i32, ObservableError> { Err(ex.clone()) }
    };

    let sources = [e0.as_observable(), e1.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1] = sources;
        s0.try_combine_latest(s1, move |_0, _1| f())
    });

    assert_messages!(res.messages(), RT::on_error(220, ex.clone()));

    let es = [&e0, &e1];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 10 * es.len() as i64));
    }
}

#[test]
fn combine_latest_selector_throws3() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);

    let ex = new_error("ex");
    let f = {
        let ex = ex.clone();
        move || -> Result<i32, ObservableError> { Err(ex.clone()) }
    };

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2] = sources;
        s0.try_combine_latest3(s1, s2, move |_0, _1, _2| f())
    });

    assert_messages!(res.messages(), RT::on_error(230, ex.clone()));

    let es = [&e0, &e1, &e2];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 10 * es.len() as i64));
    }
}

#[test]
fn combine_latest_selector_throws4() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);

    let ex = new_error("ex");
    let f = {
        let ex = ex.clone();
        move || -> Result<i32, ObservableError> { Err(ex.clone()) }
    };

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3] = sources;
        s0.try_combine_latest4(s1, s2, s3, move |_0, _1, _2, _3| f())
    });

    assert_messages!(res.messages(), RT::on_error(240, ex.clone()));

    let es = [&e0, &e1, &e2, &e3];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 10 * es.len() as i64));
    }
}

#[test]
fn combine_latest_selector_throws5() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);

    let ex = new_error("ex");
    let f = {
        let ex = ex.clone();
        move || -> Result<i32, ObservableError> { Err(ex.clone()) }
    };

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4] = sources;
        s0.try_combine_latest5(s1, s2, s3, s4, move |_0, _1, _2, _3, _4| f())
    });

    assert_messages!(res.messages(), RT::on_error(250, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 10 * es.len() as i64));
    }
}

#[test]
fn combine_latest_selector_throws6() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);

    let ex = new_error("ex");
    let f = {
        let ex = ex.clone();
        move || -> Result<i32, ObservableError> { Err(ex.clone()) }
    };

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5] = sources;
        s0.try_combine_latest6(s1, s2, s3, s4, s5, move |_0, _1, _2, _3, _4, _5| f())
    });

    assert_messages!(res.messages(), RT::on_error(260, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 10 * es.len() as i64));
    }
}

#[test]
fn combine_latest_selector_throws7() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_completed(400)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_completed(400)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_completed(400)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_completed(400)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_completed(400)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_completed(400)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_completed(400)]);

    let ex = new_error("ex");
    let f = {
        let ex = ex.clone();
        move || -> Result<i32, ObservableError> { Err(ex.clone()) }
    };

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6] = sources;
        s0.try_combine_latest7(s1, s2, s3, s4, s5, s6, move |_0, _1, _2, _3, _4, _5, _6| f())
    });

    assert_messages!(res.messages(), RT::on_error(270, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 10 * es.len() as i64));
    }
}

#[test]
fn combine_latest_selector_throws8() {
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
    let f = {
        let ex = ex.clone();
        move || -> Result<i32, ObservableError> { Err(ex.clone()) }
    };

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7] = sources;
        s0.try_combine_latest8(s1, s2, s3, s4, s5, s6, s7, move |_0, _1, _2, _3, _4, _5, _6, _7| f())
    });

    assert_messages!(res.messages(), RT::on_error(280, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 10 * es.len() as i64));
    }
}

#[test]
fn combine_latest_selector_throws9() {
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
    let f = {
        let ex = ex.clone();
        move || -> Result<i32, ObservableError> { Err(ex.clone()) }
    };

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8] = sources;
        s0.try_combine_latest9(s1, s2, s3, s4, s5, s6, s7, s8, move |_0, _1, _2, _3, _4, _5, _6, _7, _8| f())
    });

    assert_messages!(res.messages(), RT::on_error(290, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 10 * es.len() as i64));
    }
}

#[test]
fn combine_latest_selector_throws10() {
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
    let f = {
        let ex = ex.clone();
        move || -> Result<i32, ObservableError> { Err(ex.clone()) }
    };

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9] = sources;
        s0.try_combine_latest10(s1, s2, s3, s4, s5, s6, s7, s8, s9, move |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9| f())
    });

    assert_messages!(res.messages(), RT::on_error(300, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 10 * es.len() as i64));
    }
}

#[test]
fn combine_latest_selector_throws11() {
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
    let f = {
        let ex = ex.clone();
        move || -> Result<i32, ObservableError> { Err(ex.clone()) }
    };

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10] = sources;
        s0.try_combine_latest11(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, move |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10| f())
    });

    assert_messages!(res.messages(), RT::on_error(310, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 10 * es.len() as i64));
    }
}

#[test]
fn combine_latest_selector_throws12() {
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
    let f = {
        let ex = ex.clone();
        move || -> Result<i32, ObservableError> { Err(ex.clone()) }
    };

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11] = sources;
        s0.try_combine_latest12(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, move |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11| f())
    });

    assert_messages!(res.messages(), RT::on_error(320, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 10 * es.len() as i64));
    }
}

#[test]
fn combine_latest_selector_throws13() {
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
    let f = {
        let ex = ex.clone();
        move || -> Result<i32, ObservableError> { Err(ex.clone()) }
    };

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12] = sources;
        s0.try_combine_latest13(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, move |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12| f())
    });

    assert_messages!(res.messages(), RT::on_error(330, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 10 * es.len() as i64));
    }
}

#[test]
fn combine_latest_selector_throws14() {
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
    let f = {
        let ex = ex.clone();
        move || -> Result<i32, ObservableError> { Err(ex.clone()) }
    };

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13] = sources;
        s0.try_combine_latest14(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, move |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13| f())
    });

    assert_messages!(res.messages(), RT::on_error(340, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 10 * es.len() as i64));
    }
}

#[test]
fn combine_latest_selector_throws15() {
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
    let f = {
        let ex = ex.clone();
        move || -> Result<i32, ObservableError> { Err(ex.clone()) }
    };

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
        e14.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14] = sources;
        s0.try_combine_latest15(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, move |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13, _14| f())
    });

    assert_messages!(res.messages(), RT::on_error(350, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 10 * es.len() as i64));
    }
}

#[test]
fn combine_latest_selector_throws16() {
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
    let f = {
        let ex = ex.clone();
        move || -> Result<i32, ObservableError> { Err(ex.clone()) }
    };

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
        e14.as_observable(),
        e15.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15] = sources;
        s0.try_combine_latest16(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15, move |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13, _14, _15| f())
    });

    assert_messages!(res.messages(), RT::on_error(360, ex.clone()));

    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14, &e15];
    for e in es {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 10 * es.len() as i64));
    }
}

#[test]
fn combine_latest_will_never_be_able_to_combine_n() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable()];

    let res = scheduler.start(move || Observable::combine_latest_all_with(sources, |_xs: &[i32]| 42));

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine2() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [e0.as_observable(), e1.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1] = sources;
        s0.combine_latest(s1, |_0, _1| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine2_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [e0.as_observable(), e1.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1] = sources;
        s0.combine_latest_tuple(s1)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine3() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2] = sources;
        s0.combine_latest3(s1, s2, |_0, _1, _2| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine3_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2] = sources;
        s0.combine_latest3_tuple(s1, s2)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine4() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3] = sources;
        s0.combine_latest4(s1, s2, s3, |_0, _1, _2, _3| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine4_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3] = sources;
        s0.combine_latest4_tuple(s1, s2, s3)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine5() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4] = sources;
        s0.combine_latest5(s1, s2, s3, s4, |_0, _1, _2, _3, _4| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine5_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4] = sources;
        s0.combine_latest5_tuple(s1, s2, s3, s4)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine6() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5] = sources;
        s0.combine_latest6(s1, s2, s3, s4, s5, |_0, _1, _2, _3, _4, _5| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4, &e5];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine6_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5] = sources;
        s0.combine_latest6_tuple(s1, s2, s3, s4, s5)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4, &e5];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine7() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6] = sources;
        s0.combine_latest7(s1, s2, s3, s4, s5, s6, |_0, _1, _2, _3, _4, _5, _6| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine7_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6] = sources;
        s0.combine_latest7_tuple(s1, s2, s3, s4, s5, s6)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine8() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7] = sources;
        s0.combine_latest8(s1, s2, s3, s4, s5, s6, s7, |_0, _1, _2, _3, _4, _5, _6, _7| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine8_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7] = sources;
        s0.combine_latest8_tuple(s1, s2, s3, s4, s5, s6, s7)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine9() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8] = sources;
        s0.combine_latest9(s1, s2, s3, s4, s5, s6, s7, s8, |_0, _1, _2, _3, _4, _5, _6, _7, _8| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine9_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8] = sources;
        s0.combine_latest9_tuple(s1, s2, s3, s4, s5, s6, s7, s8)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine10() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(330)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9] = sources;
        s0.combine_latest10(s1, s2, s3, s4, s5, s6, s7, s8, s9, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine10_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(330)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9] = sources;
        s0.combine_latest10_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine11() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(330)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(340)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10] = sources;
        s0.combine_latest11(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine11_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(330)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(340)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10] = sources;
        s0.combine_latest11_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine12() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(330)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(340)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(350)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11] = sources;
        s0.combine_latest12(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine12_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(330)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(340)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(350)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11] = sources;
        s0.combine_latest12_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine13() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(330)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(340)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(350)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(360)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12] = sources;
        s0.combine_latest13(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine13_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(330)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(340)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(350)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(360)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12] = sources;
        s0.combine_latest13_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12).select(|t| [t.0, t.1, t.2, t.3, t.4, t.5, t.6, t.7, t.8, t.9, t.10, t.11, t.12])
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine14() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(330)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(340)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(350)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(360)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(370)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13] = sources;
        s0.combine_latest14(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine14_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(330)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(340)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(350)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(360)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(370)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13] = sources;
        s0.combine_latest14_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13).select(|t| [t.0, t.1, t.2, t.3, t.4, t.5, t.6, t.7, t.8, t.9, t.10, t.11, t.12, t.13])
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine15() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(330)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(340)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(350)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(360)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(370)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(380)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
        e14.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14] = sources;
        s0.combine_latest15(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13, _14| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine15_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(330)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(340)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(350)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(360)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(370)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(380)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
        e14.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14] = sources;
        s0.combine_latest15_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14).select(|t| [t.0, t.1, t.2, t.3, t.4, t.5, t.6, t.7, t.8, t.9, t.10, t.11, t.12, t.13, t.14])
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine16() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(330)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(340)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(350)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(360)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(370)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(380)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(390)]);
    let e15 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
        e14.as_observable(),
        e15.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15] = sources;
        s0.combine_latest16(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13, _14, _15| 42)
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14, &e15];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_will_never_be_able_to_combine16_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(250)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(260)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(270)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(280)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(290)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(300)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(310)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(320)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(330)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(340)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(350)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(360)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(370)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(380)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_completed(390)]);
    let e15 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(500, 2), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
        e14.as_observable(),
        e15.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15] = sources;
        s0.combine_latest16_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15).select(|t| [t.0, t.1, t.2, t.3, t.4, t.5, t.6, t.7, t.8, t.9, t.10, t.11, t.12, t.13, t.14, t.15])
    });

    assert_messages!(res.messages(), RT::on_completed(500));

    let mut i = 0;
    let es = [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14, &e15];
    for e in es.iter().take(es.len() - 1) {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 200 + 50 + i * 10));
        i += 1;
    }

    assert_subscriptions!(es.last().expect("the sources").subscriptions(), RT::subscribe(200, 500));
}

#[test]
fn combine_latest_typical_n() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 4), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 5), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 6), RT::on_completed(800)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable()];

    let res = scheduler.start(move || Observable::combine_latest_all_with(sources, |xs: &[i32]| xs.iter().sum::<i32>()));

    assert_messages!(
        res.messages(),
        RT::on_next(230, 6),
        RT::on_next(410, 9),
        RT::on_next(420, 12),
        RT::on_next(430, 15),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical2() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 3), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 4), RT::on_completed(800)]);

    let sources = [e0.as_observable(), e1.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1] = sources;
        s0.combine_latest(s1, |_0, _1| _0 + _1)
    });

    assert_messages!(res.messages(), RT::on_next(220, 3), RT::on_next(410, 5), RT::on_next(420, 7), RT::on_completed(800));

    for e in [&e0, &e1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical2_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 3), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 4), RT::on_completed(800)]);

    let sources = [e0.as_observable(), e1.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1] = sources;
        s0.combine_latest_tuple(s1)
    });

    assert_messages!(res.messages(), RT::on_next(220, (1, 2)), RT::on_next(410, (3, 2)), RT::on_next(420, (3, 4)), RT::on_completed(800));

    for e in [&e0, &e1] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical3() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 4), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 5), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 6), RT::on_completed(800)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2] = sources;
        s0.combine_latest3(s1, s2, |_0, _1, _2| _0 + _1 + _2)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(230, 6),
        RT::on_next(410, 9),
        RT::on_next(420, 12),
        RT::on_next(430, 15),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical3_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 4), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 5), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 6), RT::on_completed(800)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2] = sources;
        s0.combine_latest3_tuple(s1, s2)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(230, (1, 2, 3)),
        RT::on_next(410, (4, 2, 3)),
        RT::on_next(420, (4, 5, 3)),
        RT::on_next(430, (4, 5, 6)),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical4() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 5), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 6), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 7), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 8), RT::on_completed(800)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3] = sources;
        s0.combine_latest4(s1, s2, s3, |_0, _1, _2, _3| _0 + _1 + _2 + _3)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, 10),
        RT::on_next(410, 14),
        RT::on_next(420, 18),
        RT::on_next(430, 22),
        RT::on_next(440, 26),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical4_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 5), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 6), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 7), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 8), RT::on_completed(800)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3] = sources;
        s0.combine_latest4_tuple(s1, s2, s3)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(240, (1, 2, 3, 4)),
        RT::on_next(410, (5, 2, 3, 4)),
        RT::on_next(420, (5, 6, 3, 4)),
        RT::on_next(430, (5, 6, 7, 4)),
        RT::on_next(440, (5, 6, 7, 8)),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical5() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 6), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 7), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 8), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 9), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 10), RT::on_completed(800)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4] = sources;
        s0.combine_latest5(s1, s2, s3, s4, |_0, _1, _2, _3, _4| _0 + _1 + _2 + _3 + _4)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, 15),
        RT::on_next(410, 20),
        RT::on_next(420, 25),
        RT::on_next(430, 30),
        RT::on_next(440, 35),
        RT::on_next(450, 40),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical5_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 6), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 7), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 8), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 9), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 10), RT::on_completed(800)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4] = sources;
        s0.combine_latest5_tuple(s1, s2, s3, s4)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(250, (1, 2, 3, 4, 5)),
        RT::on_next(410, (6, 2, 3, 4, 5)),
        RT::on_next(420, (6, 7, 3, 4, 5)),
        RT::on_next(430, (6, 7, 8, 4, 5)),
        RT::on_next(440, (6, 7, 8, 9, 5)),
        RT::on_next(450, (6, 7, 8, 9, 10)),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical6() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 7), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 8), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 9), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 10), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 11), RT::on_completed(800)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_next(460, 12), RT::on_completed(800)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5] = sources;
        s0.combine_latest6(s1, s2, s3, s4, s5, |_0, _1, _2, _3, _4, _5| _0 + _1 + _2 + _3 + _4 + _5)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(260, 21),
        RT::on_next(410, 27),
        RT::on_next(420, 33),
        RT::on_next(430, 39),
        RT::on_next(440, 45),
        RT::on_next(450, 51),
        RT::on_next(460, 57),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4, &e5] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical6_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 7), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 8), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 9), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 10), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 11), RT::on_completed(800)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_next(460, 12), RT::on_completed(800)]);

    let sources = [e0.as_observable(), e1.as_observable(), e2.as_observable(), e3.as_observable(), e4.as_observable(), e5.as_observable()];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5] = sources;
        s0.combine_latest6_tuple(s1, s2, s3, s4, s5)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(260, (1, 2, 3, 4, 5, 6)),
        RT::on_next(410, (7, 2, 3, 4, 5, 6)),
        RT::on_next(420, (7, 8, 3, 4, 5, 6)),
        RT::on_next(430, (7, 8, 9, 4, 5, 6)),
        RT::on_next(440, (7, 8, 9, 10, 5, 6)),
        RT::on_next(450, (7, 8, 9, 10, 11, 6)),
        RT::on_next(460, (7, 8, 9, 10, 11, 12)),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4, &e5] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical7() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 8), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 9), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 10), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 11), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 12), RT::on_completed(800)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_next(460, 13), RT::on_completed(800)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_next(470, 14), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6] = sources;
        s0.combine_latest7(s1, s2, s3, s4, s5, s6, |_0, _1, _2, _3, _4, _5, _6| _0 + _1 + _2 + _3 + _4 + _5 + _6)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(270, 28),
        RT::on_next(410, 35),
        RT::on_next(420, 42),
        RT::on_next(430, 49),
        RT::on_next(440, 56),
        RT::on_next(450, 63),
        RT::on_next(460, 70),
        RT::on_next(470, 77),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical7_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 8), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 9), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 10), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 11), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 12), RT::on_completed(800)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_next(460, 13), RT::on_completed(800)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_next(470, 14), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6] = sources;
        s0.combine_latest7_tuple(s1, s2, s3, s4, s5, s6)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(270, (1, 2, 3, 4, 5, 6, 7)),
        RT::on_next(410, (8, 2, 3, 4, 5, 6, 7)),
        RT::on_next(420, (8, 9, 3, 4, 5, 6, 7)),
        RT::on_next(430, (8, 9, 10, 4, 5, 6, 7)),
        RT::on_next(440, (8, 9, 10, 11, 5, 6, 7)),
        RT::on_next(450, (8, 9, 10, 11, 12, 6, 7)),
        RT::on_next(460, (8, 9, 10, 11, 12, 13, 7)),
        RT::on_next(470, (8, 9, 10, 11, 12, 13, 14)),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical8() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 9), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 10), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 11), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 12), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 13), RT::on_completed(800)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_next(460, 14), RT::on_completed(800)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_next(470, 15), RT::on_completed(800)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_next(480, 16), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7] = sources;
        s0.combine_latest8(s1, s2, s3, s4, s5, s6, s7, |_0, _1, _2, _3, _4, _5, _6, _7| _0 + _1 + _2 + _3 + _4 + _5 + _6 + _7)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(280, 36),
        RT::on_next(410, 44),
        RT::on_next(420, 52),
        RT::on_next(430, 60),
        RT::on_next(440, 68),
        RT::on_next(450, 76),
        RT::on_next(460, 84),
        RT::on_next(470, 92),
        RT::on_next(480, 100),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical8_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 9), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 10), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 11), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 12), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 13), RT::on_completed(800)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_next(460, 14), RT::on_completed(800)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_next(470, 15), RT::on_completed(800)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_next(480, 16), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7] = sources;
        s0.combine_latest8_tuple(s1, s2, s3, s4, s5, s6, s7)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(280, (1, 2, 3, 4, 5, 6, 7, 8)),
        RT::on_next(410, (9, 2, 3, 4, 5, 6, 7, 8)),
        RT::on_next(420, (9, 10, 3, 4, 5, 6, 7, 8)),
        RT::on_next(430, (9, 10, 11, 4, 5, 6, 7, 8)),
        RT::on_next(440, (9, 10, 11, 12, 5, 6, 7, 8)),
        RT::on_next(450, (9, 10, 11, 12, 13, 6, 7, 8)),
        RT::on_next(460, (9, 10, 11, 12, 13, 14, 7, 8)),
        RT::on_next(470, (9, 10, 11, 12, 13, 14, 15, 8)),
        RT::on_next(480, (9, 10, 11, 12, 13, 14, 15, 16)),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical9() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 10), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 11), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 12), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 13), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 14), RT::on_completed(800)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_next(460, 15), RT::on_completed(800)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_next(470, 16), RT::on_completed(800)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_next(480, 17), RT::on_completed(800)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_next(490, 18), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8] = sources;
        s0.combine_latest9(s1, s2, s3, s4, s5, s6, s7, s8, |_0, _1, _2, _3, _4, _5, _6, _7, _8| _0 + _1 + _2 + _3 + _4 + _5 + _6 + _7 + _8)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(290, 45),
        RT::on_next(410, 54),
        RT::on_next(420, 63),
        RT::on_next(430, 72),
        RT::on_next(440, 81),
        RT::on_next(450, 90),
        RT::on_next(460, 99),
        RT::on_next(470, 108),
        RT::on_next(480, 117),
        RT::on_next(490, 126),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical9_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 10), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 11), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 12), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 13), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 14), RT::on_completed(800)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_next(460, 15), RT::on_completed(800)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_next(470, 16), RT::on_completed(800)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_next(480, 17), RT::on_completed(800)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_next(490, 18), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8] = sources;
        s0.combine_latest9_tuple(s1, s2, s3, s4, s5, s6, s7, s8)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(290, (1, 2, 3, 4, 5, 6, 7, 8, 9)),
        RT::on_next(410, (10, 2, 3, 4, 5, 6, 7, 8, 9)),
        RT::on_next(420, (10, 11, 3, 4, 5, 6, 7, 8, 9)),
        RT::on_next(430, (10, 11, 12, 4, 5, 6, 7, 8, 9)),
        RT::on_next(440, (10, 11, 12, 13, 5, 6, 7, 8, 9)),
        RT::on_next(450, (10, 11, 12, 13, 14, 6, 7, 8, 9)),
        RT::on_next(460, (10, 11, 12, 13, 14, 15, 7, 8, 9)),
        RT::on_next(470, (10, 11, 12, 13, 14, 15, 16, 8, 9)),
        RT::on_next(480, (10, 11, 12, 13, 14, 15, 16, 17, 9)),
        RT::on_next(490, (10, 11, 12, 13, 14, 15, 16, 17, 18)),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical10() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 11), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 12), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 13), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 14), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 15), RT::on_completed(800)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_next(460, 16), RT::on_completed(800)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_next(470, 17), RT::on_completed(800)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_next(480, 18), RT::on_completed(800)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_next(490, 19), RT::on_completed(800)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_next(500, 20), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9] = sources;
        s0.combine_latest10(s1, s2, s3, s4, s5, s6, s7, s8, s9, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9| _0 + _1 + _2 + _3 + _4 + _5 + _6 + _7 + _8 + _9)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(300, 55),
        RT::on_next(410, 65),
        RT::on_next(420, 75),
        RT::on_next(430, 85),
        RT::on_next(440, 95),
        RT::on_next(450, 105),
        RT::on_next(460, 115),
        RT::on_next(470, 125),
        RT::on_next(480, 135),
        RT::on_next(490, 145),
        RT::on_next(500, 155),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical10_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 11), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 12), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 13), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 14), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 15), RT::on_completed(800)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_next(460, 16), RT::on_completed(800)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_next(470, 17), RT::on_completed(800)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_next(480, 18), RT::on_completed(800)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_next(490, 19), RT::on_completed(800)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_next(500, 20), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9] = sources;
        s0.combine_latest10_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(300, (1, 2, 3, 4, 5, 6, 7, 8, 9, 10)),
        RT::on_next(410, (11, 2, 3, 4, 5, 6, 7, 8, 9, 10)),
        RT::on_next(420, (11, 12, 3, 4, 5, 6, 7, 8, 9, 10)),
        RT::on_next(430, (11, 12, 13, 4, 5, 6, 7, 8, 9, 10)),
        RT::on_next(440, (11, 12, 13, 14, 5, 6, 7, 8, 9, 10)),
        RT::on_next(450, (11, 12, 13, 14, 15, 6, 7, 8, 9, 10)),
        RT::on_next(460, (11, 12, 13, 14, 15, 16, 7, 8, 9, 10)),
        RT::on_next(470, (11, 12, 13, 14, 15, 16, 17, 8, 9, 10)),
        RT::on_next(480, (11, 12, 13, 14, 15, 16, 17, 18, 9, 10)),
        RT::on_next(490, (11, 12, 13, 14, 15, 16, 17, 18, 19, 10)),
        RT::on_next(500, (11, 12, 13, 14, 15, 16, 17, 18, 19, 20)),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical11() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 12), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 13), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 14), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 15), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 16), RT::on_completed(800)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_next(460, 17), RT::on_completed(800)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_next(470, 18), RT::on_completed(800)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_next(480, 19), RT::on_completed(800)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_next(490, 20), RT::on_completed(800)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_next(500, 21), RT::on_completed(800)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_next(510, 22), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10] = sources;
        s0.combine_latest11(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10| _0 + _1 + _2 + _3 + _4 + _5 + _6 + _7 + _8 + _9 + _10)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(310, 66),
        RT::on_next(410, 77),
        RT::on_next(420, 88),
        RT::on_next(430, 99),
        RT::on_next(440, 110),
        RT::on_next(450, 121),
        RT::on_next(460, 132),
        RT::on_next(470, 143),
        RT::on_next(480, 154),
        RT::on_next(490, 165),
        RT::on_next(500, 176),
        RT::on_next(510, 187),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical11_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 12), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 13), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 14), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 15), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 16), RT::on_completed(800)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_next(460, 17), RT::on_completed(800)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_next(470, 18), RT::on_completed(800)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_next(480, 19), RT::on_completed(800)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_next(490, 20), RT::on_completed(800)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_next(500, 21), RT::on_completed(800)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_next(510, 22), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10] = sources;
        s0.combine_latest11_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(310, (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11)),
        RT::on_next(410, (12, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11)),
        RT::on_next(420, (12, 13, 3, 4, 5, 6, 7, 8, 9, 10, 11)),
        RT::on_next(430, (12, 13, 14, 4, 5, 6, 7, 8, 9, 10, 11)),
        RT::on_next(440, (12, 13, 14, 15, 5, 6, 7, 8, 9, 10, 11)),
        RT::on_next(450, (12, 13, 14, 15, 16, 6, 7, 8, 9, 10, 11)),
        RT::on_next(460, (12, 13, 14, 15, 16, 17, 7, 8, 9, 10, 11)),
        RT::on_next(470, (12, 13, 14, 15, 16, 17, 18, 8, 9, 10, 11)),
        RT::on_next(480, (12, 13, 14, 15, 16, 17, 18, 19, 9, 10, 11)),
        RT::on_next(490, (12, 13, 14, 15, 16, 17, 18, 19, 20, 10, 11)),
        RT::on_next(500, (12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 11)),
        RT::on_next(510, (12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22)),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical12() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 13), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 14), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 15), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 16), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 17), RT::on_completed(800)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_next(460, 18), RT::on_completed(800)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_next(470, 19), RT::on_completed(800)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_next(480, 20), RT::on_completed(800)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_next(490, 21), RT::on_completed(800)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_next(500, 22), RT::on_completed(800)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_next(510, 23), RT::on_completed(800)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_next(520, 24), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11] = sources;
        s0.combine_latest12(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11| _0 + _1 + _2 + _3 + _4 + _5 + _6 + _7 + _8 + _9 + _10 + _11)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(320, 78),
        RT::on_next(410, 90),
        RT::on_next(420, 102),
        RT::on_next(430, 114),
        RT::on_next(440, 126),
        RT::on_next(450, 138),
        RT::on_next(460, 150),
        RT::on_next(470, 162),
        RT::on_next(480, 174),
        RT::on_next(490, 186),
        RT::on_next(500, 198),
        RT::on_next(510, 210),
        RT::on_next(520, 222),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical12_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 13), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 14), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 15), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 16), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 17), RT::on_completed(800)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_next(460, 18), RT::on_completed(800)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_next(470, 19), RT::on_completed(800)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_next(480, 20), RT::on_completed(800)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_next(490, 21), RT::on_completed(800)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_next(500, 22), RT::on_completed(800)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_next(510, 23), RT::on_completed(800)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_next(520, 24), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11] = sources;
        s0.combine_latest12_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(320, (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12)),
        RT::on_next(410, (13, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12)),
        RT::on_next(420, (13, 14, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12)),
        RT::on_next(430, (13, 14, 15, 4, 5, 6, 7, 8, 9, 10, 11, 12)),
        RT::on_next(440, (13, 14, 15, 16, 5, 6, 7, 8, 9, 10, 11, 12)),
        RT::on_next(450, (13, 14, 15, 16, 17, 6, 7, 8, 9, 10, 11, 12)),
        RT::on_next(460, (13, 14, 15, 16, 17, 18, 7, 8, 9, 10, 11, 12)),
        RT::on_next(470, (13, 14, 15, 16, 17, 18, 19, 8, 9, 10, 11, 12)),
        RT::on_next(480, (13, 14, 15, 16, 17, 18, 19, 20, 9, 10, 11, 12)),
        RT::on_next(490, (13, 14, 15, 16, 17, 18, 19, 20, 21, 10, 11, 12)),
        RT::on_next(500, (13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 11, 12)),
        RT::on_next(510, (13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 12)),
        RT::on_next(520, (13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24)),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical13() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 14), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 15), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 16), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 17), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 18), RT::on_completed(800)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_next(460, 19), RT::on_completed(800)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_next(470, 20), RT::on_completed(800)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_next(480, 21), RT::on_completed(800)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_next(490, 22), RT::on_completed(800)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_next(500, 23), RT::on_completed(800)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_next(510, 24), RT::on_completed(800)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_next(520, 25), RT::on_completed(800)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(330, 13), RT::on_next(530, 26), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12] = sources;
        s0.combine_latest13(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12| _0 + _1 + _2 + _3 + _4 + _5 + _6 + _7 + _8 + _9 + _10 + _11 + _12)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(330, 91),
        RT::on_next(410, 104),
        RT::on_next(420, 117),
        RT::on_next(430, 130),
        RT::on_next(440, 143),
        RT::on_next(450, 156),
        RT::on_next(460, 169),
        RT::on_next(470, 182),
        RT::on_next(480, 195),
        RT::on_next(490, 208),
        RT::on_next(500, 221),
        RT::on_next(510, 234),
        RT::on_next(520, 247),
        RT::on_next(530, 260),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical13_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 14), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 15), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 16), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 17), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 18), RT::on_completed(800)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_next(460, 19), RT::on_completed(800)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_next(470, 20), RT::on_completed(800)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_next(480, 21), RT::on_completed(800)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_next(490, 22), RT::on_completed(800)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_next(500, 23), RT::on_completed(800)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_next(510, 24), RT::on_completed(800)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_next(520, 25), RT::on_completed(800)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(330, 13), RT::on_next(530, 26), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12] = sources;
        s0.combine_latest13_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12).select(|t| [t.0, t.1, t.2, t.3, t.4, t.5, t.6, t.7, t.8, t.9, t.10, t.11, t.12])
    });

    assert_messages!(
        res.messages(),
        RT::on_next(330, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]),
        RT::on_next(410, [14, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]),
        RT::on_next(420, [14, 15, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]),
        RT::on_next(430, [14, 15, 16, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13]),
        RT::on_next(440, [14, 15, 16, 17, 5, 6, 7, 8, 9, 10, 11, 12, 13]),
        RT::on_next(450, [14, 15, 16, 17, 18, 6, 7, 8, 9, 10, 11, 12, 13]),
        RT::on_next(460, [14, 15, 16, 17, 18, 19, 7, 8, 9, 10, 11, 12, 13]),
        RT::on_next(470, [14, 15, 16, 17, 18, 19, 20, 8, 9, 10, 11, 12, 13]),
        RT::on_next(480, [14, 15, 16, 17, 18, 19, 20, 21, 9, 10, 11, 12, 13]),
        RT::on_next(490, [14, 15, 16, 17, 18, 19, 20, 21, 22, 10, 11, 12, 13]),
        RT::on_next(500, [14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 11, 12, 13]),
        RT::on_next(510, [14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 12, 13]),
        RT::on_next(520, [14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 13]),
        RT::on_next(530, [14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26]),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical14() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 15), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 16), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 17), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 18), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 19), RT::on_completed(800)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_next(460, 20), RT::on_completed(800)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_next(470, 21), RT::on_completed(800)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_next(480, 22), RT::on_completed(800)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_next(490, 23), RT::on_completed(800)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_next(500, 24), RT::on_completed(800)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_next(510, 25), RT::on_completed(800)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_next(520, 26), RT::on_completed(800)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(330, 13), RT::on_next(530, 27), RT::on_completed(800)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(340, 14), RT::on_next(540, 28), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13] = sources;
        s0.combine_latest14(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13| _0 + _1 + _2 + _3 + _4 + _5 + _6 + _7 + _8 + _9 + _10 + _11 + _12 + _13)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(340, 105),
        RT::on_next(410, 119),
        RT::on_next(420, 133),
        RT::on_next(430, 147),
        RT::on_next(440, 161),
        RT::on_next(450, 175),
        RT::on_next(460, 189),
        RT::on_next(470, 203),
        RT::on_next(480, 217),
        RT::on_next(490, 231),
        RT::on_next(500, 245),
        RT::on_next(510, 259),
        RT::on_next(520, 273),
        RT::on_next(530, 287),
        RT::on_next(540, 301),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical14_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 15), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 16), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 17), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 18), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 19), RT::on_completed(800)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_next(460, 20), RT::on_completed(800)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_next(470, 21), RT::on_completed(800)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_next(480, 22), RT::on_completed(800)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_next(490, 23), RT::on_completed(800)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_next(500, 24), RT::on_completed(800)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_next(510, 25), RT::on_completed(800)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_next(520, 26), RT::on_completed(800)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(330, 13), RT::on_next(530, 27), RT::on_completed(800)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(340, 14), RT::on_next(540, 28), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13] = sources;
        s0.combine_latest14_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13).select(|t| [t.0, t.1, t.2, t.3, t.4, t.5, t.6, t.7, t.8, t.9, t.10, t.11, t.12, t.13])
    });

    assert_messages!(
        res.messages(),
        RT::on_next(340, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]),
        RT::on_next(410, [15, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]),
        RT::on_next(420, [15, 16, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]),
        RT::on_next(430, [15, 16, 17, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]),
        RT::on_next(440, [15, 16, 17, 18, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]),
        RT::on_next(450, [15, 16, 17, 18, 19, 6, 7, 8, 9, 10, 11, 12, 13, 14]),
        RT::on_next(460, [15, 16, 17, 18, 19, 20, 7, 8, 9, 10, 11, 12, 13, 14]),
        RT::on_next(470, [15, 16, 17, 18, 19, 20, 21, 8, 9, 10, 11, 12, 13, 14]),
        RT::on_next(480, [15, 16, 17, 18, 19, 20, 21, 22, 9, 10, 11, 12, 13, 14]),
        RT::on_next(490, [15, 16, 17, 18, 19, 20, 21, 22, 23, 10, 11, 12, 13, 14]),
        RT::on_next(500, [15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 11, 12, 13, 14]),
        RT::on_next(510, [15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 12, 13, 14]),
        RT::on_next(520, [15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 13, 14]),
        RT::on_next(530, [15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 14]),
        RT::on_next(540, [15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28]),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical15() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 16), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 17), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 18), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 19), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 20), RT::on_completed(800)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_next(460, 21), RT::on_completed(800)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_next(470, 22), RT::on_completed(800)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_next(480, 23), RT::on_completed(800)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_next(490, 24), RT::on_completed(800)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_next(500, 25), RT::on_completed(800)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_next(510, 26), RT::on_completed(800)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_next(520, 27), RT::on_completed(800)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(330, 13), RT::on_next(530, 28), RT::on_completed(800)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(340, 14), RT::on_next(540, 29), RT::on_completed(800)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(350, 15), RT::on_next(550, 30), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
        e14.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14] = sources;
        s0.combine_latest15(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13, _14| _0 + _1 + _2 + _3 + _4 + _5 + _6 + _7 + _8 + _9 + _10 + _11 + _12 + _13 + _14)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(350, 120),
        RT::on_next(410, 135),
        RT::on_next(420, 150),
        RT::on_next(430, 165),
        RT::on_next(440, 180),
        RT::on_next(450, 195),
        RT::on_next(460, 210),
        RT::on_next(470, 225),
        RT::on_next(480, 240),
        RT::on_next(490, 255),
        RT::on_next(500, 270),
        RT::on_next(510, 285),
        RT::on_next(520, 300),
        RT::on_next(530, 315),
        RT::on_next(540, 330),
        RT::on_next(550, 345),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical15_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 16), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 17), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 18), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 19), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 20), RT::on_completed(800)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_next(460, 21), RT::on_completed(800)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_next(470, 22), RT::on_completed(800)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_next(480, 23), RT::on_completed(800)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_next(490, 24), RT::on_completed(800)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_next(500, 25), RT::on_completed(800)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_next(510, 26), RT::on_completed(800)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_next(520, 27), RT::on_completed(800)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(330, 13), RT::on_next(530, 28), RT::on_completed(800)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(340, 14), RT::on_next(540, 29), RT::on_completed(800)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(350, 15), RT::on_next(550, 30), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
        e14.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14] = sources;
        s0.combine_latest15_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14).select(|t| [t.0, t.1, t.2, t.3, t.4, t.5, t.6, t.7, t.8, t.9, t.10, t.11, t.12, t.13, t.14])
    });

    assert_messages!(
        res.messages(),
        RT::on_next(350, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
        RT::on_next(410, [16, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
        RT::on_next(420, [16, 17, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
        RT::on_next(430, [16, 17, 18, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
        RT::on_next(440, [16, 17, 18, 19, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
        RT::on_next(450, [16, 17, 18, 19, 20, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
        RT::on_next(460, [16, 17, 18, 19, 20, 21, 7, 8, 9, 10, 11, 12, 13, 14, 15]),
        RT::on_next(470, [16, 17, 18, 19, 20, 21, 22, 8, 9, 10, 11, 12, 13, 14, 15]),
        RT::on_next(480, [16, 17, 18, 19, 20, 21, 22, 23, 9, 10, 11, 12, 13, 14, 15]),
        RT::on_next(490, [16, 17, 18, 19, 20, 21, 22, 23, 24, 10, 11, 12, 13, 14, 15]),
        RT::on_next(500, [16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 11, 12, 13, 14, 15]),
        RT::on_next(510, [16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 12, 13, 14, 15]),
        RT::on_next(520, [16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 13, 14, 15]),
        RT::on_next(530, [16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 14, 15]),
        RT::on_next(540, [16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 15]),
        RT::on_next(550, [16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30]),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical16() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 17), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 18), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 19), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 20), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 21), RT::on_completed(800)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_next(460, 22), RT::on_completed(800)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_next(470, 23), RT::on_completed(800)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_next(480, 24), RT::on_completed(800)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_next(490, 25), RT::on_completed(800)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_next(500, 26), RT::on_completed(800)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_next(510, 27), RT::on_completed(800)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_next(520, 28), RT::on_completed(800)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(330, 13), RT::on_next(530, 29), RT::on_completed(800)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(340, 14), RT::on_next(540, 30), RT::on_completed(800)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(350, 15), RT::on_next(550, 31), RT::on_completed(800)]);
    let e15 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(360, 16), RT::on_next(560, 32), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
        e14.as_observable(),
        e15.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15] = sources;
        s0.combine_latest16(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15, |_0, _1, _2, _3, _4, _5, _6, _7, _8, _9, _10, _11, _12, _13, _14, _15| _0 + _1 + _2 + _3 + _4 + _5 + _6 + _7 + _8 + _9 + _10 + _11 + _12 + _13 + _14 + _15)
    });

    assert_messages!(
        res.messages(),
        RT::on_next(360, 136),
        RT::on_next(410, 152),
        RT::on_next(420, 168),
        RT::on_next(430, 184),
        RT::on_next(440, 200),
        RT::on_next(450, 216),
        RT::on_next(460, 232),
        RT::on_next(470, 248),
        RT::on_next(480, 264),
        RT::on_next(490, 280),
        RT::on_next(500, 296),
        RT::on_next(510, 312),
        RT::on_next(520, 328),
        RT::on_next(530, 344),
        RT::on_next(540, 360),
        RT::on_next(550, 376),
        RT::on_next(560, 392),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14, &e15] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}

#[test]
fn combine_latest_typical16_tuple() {
    let scheduler = TestScheduler::new();

    let e0 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(210, 1), RT::on_next(410, 17), RT::on_completed(800)]);
    let e1 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(220, 2), RT::on_next(420, 18), RT::on_completed(800)]);
    let e2 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(230, 3), RT::on_next(430, 19), RT::on_completed(800)]);
    let e3 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(240, 4), RT::on_next(440, 20), RT::on_completed(800)]);
    let e4 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(250, 5), RT::on_next(450, 21), RT::on_completed(800)]);
    let e5 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(260, 6), RT::on_next(460, 22), RT::on_completed(800)]);
    let e6 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(270, 7), RT::on_next(470, 23), RT::on_completed(800)]);
    let e7 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(280, 8), RT::on_next(480, 24), RT::on_completed(800)]);
    let e8 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(290, 9), RT::on_next(490, 25), RT::on_completed(800)]);
    let e9 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(300, 10), RT::on_next(500, 26), RT::on_completed(800)]);
    let e10 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(310, 11), RT::on_next(510, 27), RT::on_completed(800)]);
    let e11 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(320, 12), RT::on_next(520, 28), RT::on_completed(800)]);
    let e12 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(330, 13), RT::on_next(530, 29), RT::on_completed(800)]);
    let e13 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(340, 14), RT::on_next(540, 30), RT::on_completed(800)]);
    let e14 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(350, 15), RT::on_next(550, 31), RT::on_completed(800)]);
    let e15 = scheduler.create_hot_observable(vec![RT::on_next(150, 1), RT::on_next(360, 16), RT::on_next(560, 32), RT::on_completed(800)]);

    let sources = [
        e0.as_observable(),
        e1.as_observable(),
        e2.as_observable(),
        e3.as_observable(),
        e4.as_observable(),
        e5.as_observable(),
        e6.as_observable(),
        e7.as_observable(),
        e8.as_observable(),
        e9.as_observable(),
        e10.as_observable(),
        e11.as_observable(),
        e12.as_observable(),
        e13.as_observable(),
        e14.as_observable(),
        e15.as_observable(),
    ];

    let res = scheduler.start(move || {
        let [s0, s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15] = sources;
        s0.combine_latest16_tuple(s1, s2, s3, s4, s5, s6, s7, s8, s9, s10, s11, s12, s13, s14, s15).select(|t| [t.0, t.1, t.2, t.3, t.4, t.5, t.6, t.7, t.8, t.9, t.10, t.11, t.12, t.13, t.14, t.15])
    });

    assert_messages!(
        res.messages(),
        RT::on_next(360, [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
        RT::on_next(410, [17, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
        RT::on_next(420, [17, 18, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
        RT::on_next(430, [17, 18, 19, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
        RT::on_next(440, [17, 18, 19, 20, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
        RT::on_next(450, [17, 18, 19, 20, 21, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
        RT::on_next(460, [17, 18, 19, 20, 21, 22, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
        RT::on_next(470, [17, 18, 19, 20, 21, 22, 23, 8, 9, 10, 11, 12, 13, 14, 15, 16]),
        RT::on_next(480, [17, 18, 19, 20, 21, 22, 23, 24, 9, 10, 11, 12, 13, 14, 15, 16]),
        RT::on_next(490, [17, 18, 19, 20, 21, 22, 23, 24, 25, 10, 11, 12, 13, 14, 15, 16]),
        RT::on_next(500, [17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 11, 12, 13, 14, 15, 16]),
        RT::on_next(510, [17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 12, 13, 14, 15, 16]),
        RT::on_next(520, [17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 13, 14, 15, 16]),
        RT::on_next(530, [17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 14, 15, 16]),
        RT::on_next(540, [17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 15, 16]),
        RT::on_next(550, [17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 16]),
        RT::on_next(560, [17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32]),
        RT::on_completed(800)
    );

    for e in [&e0, &e1, &e2, &e3, &e4, &e5, &e6, &e7, &e8, &e9, &e10, &e11, &e12, &e13, &e14, &e15] {
        assert_subscriptions!(e.subscriptions(), RT::subscribe(200, 800));
    }
}
