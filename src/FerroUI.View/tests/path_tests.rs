//! `path!` and the path builder: the same calls on a geometry context as the path parser.

use crate::generated::builders::*;
use crate::reactive::Owner;
use crate::{path, path_data, IntoElement, PathData};
use ferroui_base::media::text_formatting::testing::TextTestScope;
use ferroui_base::media::{FillRule, PathCommand, SweepDirection};
use ferroui_base::platform::IPlatformRenderInterface;
use ferroui_base::{FerroLocator, LocatorExtensions};
use ferroui_controls::shapes::Path;
use ferroui_controls::testing::{TestServices, UnitTestApplication, UnitTestApplicationScope};

macro_rules! same_as_the_parser {
    ($($text:literal),* $(,)?) => {$(
        assert_eq!(path!($text).commands(), PathCommand::record($text).expect($text).as_slice(), "{:?}", $text);
    )*};
}

/// The data of the tests of the path parser that it accepts.
#[test]
fn the_macro_makes_the_calls_of_the_parser() {
    same_as_the_parser![
        "M10 10",
        "M0 0L10 10",
        "M0 0L10 10z",
        "F 1M0,0",
        "M0 0 10 10 20 20",
        "M0,0 10,10 20,20",
        "M0,0,10,10,20,20",
        "m0 0 10 10 20 20",
        "m0,0 10,10 20,20",
        "m0,0,10,10,20,20",
        "M -1.01725E-005 -1.01725e-005",
        "M5.5.5 5.5.5 5.5.5",
        "F1M9.0771,11C9.1161,10.701,9.1801,10.352,9.3031,10L9.0001,10 9.0001,6.166 3.0001,9.767 3\
         .0001,10 9.99999999997669E-05,10 9.99999999997669E-05,0 3.0001,0 3.0001,0.234 9.0001,3.8\
         34 9.0001,0 12.0001,0 12.0001,8.062C12.1861,8.043 12.3821,8.031 12.5941,8.031 15.3481,8.\
         031 15.7961,9.826 15.9201,11L16.0001,16 9.0001,16 9.0001,12.562 9.0001,11z",
        "         M0 0",
        "F1 M24,14 A2,2,0,1,1,20,14 A2,2,0,1,1,24,14 z",
        "M50 50 L100 100 L150 50",
        "M50 50L100 100L150 50",
        "M50,50 L100,100 L150,50",
        "M50 50 L-10 -10 L10 50",
        "M50 50L-10-10L10 50",
        "M50 50 L100 100 L150 50zM50 50 L70 70 L120 50z",
        "M 50 50 L 100 100 L 150 50",
        "M50 50 L100 100 L150 50 H200 V100Z",
        "M 80 200 A 100 50 45 1 0 100 50",
        "F1 M 16.6309 18.6563C 17.1309 8.15625 29.8809 14.1563 29.8809 14.1563C 30.8809 11.1563 3\
         4.1308 11.4063 34.1308 11.4063C 33.5 12 34.6309 13.1563 34.6309 13.1563C 32.1309 13.1562 \
         31.1309 14.9062 31.1309 14.9062C 41.1309 23.9062 32.6309 27.9063 32.6309 27.9062C 24.630\
         9 24.9063 21.1309 22.1562 16.6309 18.6563 Z M 16.6309 19.9063C 21.6309 24.1563 25.1309 2\
         6.1562 31.6309 28.6562C 31.6309 28.6562 26.3809 39.1562 18.3809 36.1563C 18.3809 36.1563 \
         18 38 16.3809 36.9063C 15 36 16.3809 34.9063 16.3809 34.9063C 16.3809 34.9063 10.1309 30\
         .9062 16.6309 19.9063 Z ",
        "F1M16,12C16,14.209 14.209,16 12,16 9.791,16 8,14.209 8,12 8,11.817 8.03,11.644 8.054,11.\
         467L6.585,10 4,10 4,6.414 2.5,7.914 0,5.414 0,3.586 3.586,0 4.414,0 7.414,3 7.586,3 9,1.\
         586 11.914,4.5 10.414,6 12.461,8.046C14.45,8.278,16,9.949,16,12",
        "M0 0L10 10 \n ",
        "M0 0L10 10z \n ",
        "M0 0L10 10 ",
        "M0 0L10 10z ",
        "M 5.5, 5 L 5.5, 5 L 5.5, 5",
        "F1 M 9.0771, 11 C 9.1161, 10.701 9.1801, 10.352 9.3031, 10 L 9.0001, 10 L 9.0001, 6.166 \
         L 3.0001, 9.767 L 3.0001, 10 L 9.99999999997669E-05, 10 L 9.99999999997669E-05, 0 L 3.00\
         01, 0 L 3.0001, 0.234 L 9.0001, 3.834 L 9.0001, 0 L 12.0001, 0 L 12.0001, 8.062 C 12.186\
         1, 8.043 12.3821, 8.031 12.5941, 8.031 C 15.3481, 8.031 15.7961, 9.826 15.9201, 11 L 16.\
         0001, 16 L 9.0001, 16 L 9.0001, 12.562 L 9.0001, 11Z",
        "F1 M 24, 14 A 2, 2 0 1 1 20, 14 A 2, 2 0 1 1 24, 14Z",
        "M 0, 0 L 10, 10Z",
        "M 50, 50 L 100, 100 L 150, 50",
        "M 50, 50 L -10, -10 L 10, 50",
        "M 50, 50 L 100, 100 L 150, 50Z M 50, 50 L 70, 70 L 120, 50Z",
        "M 80, 200 A 100, 50 45 1 0 100, 50",
        "F1 M 16, 12 C 16, 14.209 14.209, 16 12, 16 C 9.791, 16 8, 14.209 8, 12 C 8, 11.817 8.03, \
         11.644 8.054, 11.467 L 6.585, 10 L 4, 10 L 4, 6.414 L 2.5, 7.914 L 0, 5.414 L 0, 3.586 L \
         3.586, 0 L 4.414, 0 L 7.414, 3 L 7.586, 3 L 9, 1.586 L 11.914, 4.5 L 10.414, 6 L 12.461, \
         8.046 C 14.45, 8.278 16, 9.949 16, 12",
        "M 5.5, 0.5 L 5.5, 0.5 L 5.5, 0.5",
        "M10,10L100,100Z m10,10",
        "M50,50z l -5,-5",
        "M0,0 C0,10 10,10 10,0 S20,-10 20,0 Q25,5 30,0 T40,0",
    ];
}

#[test]
fn the_builder_makes_the_calls_of_the_parser() {
    let cases = [
        ("M0 0L10 10", path_data().move_to(0.0, 0.0).line_to(10.0, 10.0)),
        ("M0 0L10 10z", path_data().move_to(0.0, 0.0).line_to(10.0, 10.0).close()),
        (
            "F1 M24,14 A2,2,0,1,1,20,14 A2,2,0,1,1,24,14 z",
            path_data()
                .fill_rule(FillRule::NonZero)
                .move_to(24.0, 14.0)
                .arc_to(2.0, 2.0, 0.0, true, SweepDirection::Clockwise, 20.0, 14.0)
                .arc_to(2.0, 2.0, 0.0, true, SweepDirection::Clockwise, 24.0, 14.0)
                .close(),
        ),
        (
            "M 80 200 A 100 50 45 1 0 100 50",
            path_data().move_to(80.0, 200.0).arc_to(100.0, 50.0, 45.0, true, SweepDirection::CounterClockwise, 100.0, 50.0),
        ),
        (
            "M0,0 C0,10 10,10 10,0 Q25,5 30,0",
            path_data().move_to(0.0, 0.0).cubic_to(0.0, 10.0, 10.0, 10.0, 10.0, 0.0).quad_to(25.0, 5.0, 30.0, 0.0),
        ),
        (
            "M50 50 L100 100 L150 50zM50 50 L70 70 L120 50z",
            path_data()
                .move_to(50.0, 50.0)
                .line_to(100.0, 100.0)
                .line_to(150.0, 50.0)
                .close()
                .move_to(50.0, 50.0)
                .line_to(70.0, 70.0)
                .line_to(120.0, 50.0)
                .close(),
        ),
        ("M0 0 L1 1 M2 2 L3 3", path_data().move_to(0.0, 0.0).line_to(1.0, 1.0).move_to(2.0, 2.0).line_to(3.0, 3.0)),
        // A segment after a close starts a figure at the start of the closed one.
        ("M50,50z l -5,-5", path_data().move_to(50.0, 50.0).close().line_to(45.0, 45.0)),
    ];
    for (text, built) in cases {
        assert_eq!(built.commands(), PathCommand::record(text).unwrap(), "{text}");
    }
}

struct App {
    _app: UnitTestApplicationScope,
    _text: TextTestScope,
}

fn app() -> App {
    let text = TextTestScope::new();
    let render_interface = FerroLocator::current()
        .get_service::<dyn IPlatformRenderInterface>()
        .expect("the text services have a render interface");
    let app = UnitTestApplication::start(TestServices::styled_window().with_render_interface(render_interface));
    App { _app: app, _text: text }
}

fn icon() -> PathData {
    path!("M8 3v5H3v2h5v5h2V10h5V8h-5V3z")
}

#[test]
fn the_shapes_of_one_path_hold_one_geometry() {
    let _app = app();
    let owner = Owner::new_root();
    let (first, second) = owner.with(|| {
        let shape = || path().data(icon()).into_element().mount().cast::<Path>().unwrap();
        (shape(), shape())
    });
    let (first, second) = (first.data().unwrap(), second.data().unwrap());
    assert!(first == second, "one table, one geometry");
    assert!(icon().to_geometry() != first, "to_geometry makes a geometry of its own");
    owner.dispose();
}

#[test]
fn rsx_takes_a_path_and_a_builder() {
    let _app = app();
    let owner = Owner::new_root();
    let (fixed, computed) = owner.with(|| {
        let fixed: crate::Element = crate::rsx! { Path { data: path!("M0 0L10 10z") } };
        let computed: crate::Element = crate::rsx! { Path { data: path_data().move_to(0.0, 0.0).line_to(5.0, 5.0) } };
        (fixed.mount().cast::<Path>().unwrap(), computed.mount().cast::<Path>().unwrap())
    });
    assert!(fixed.data().is_some());
    assert!(computed.data().is_some());
    owner.dispose();
}
