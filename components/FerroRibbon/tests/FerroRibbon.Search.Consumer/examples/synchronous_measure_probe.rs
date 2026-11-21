//! expected to fail the WinUI-style assertion.
//! No FerroRibbon control or helper participates in this native nested layout.
use ferroui_base::Size;
use ferroui_controls::{AppBuilder, Application, Border};
use ferroui_headless::{FerroHeadlessPlatformExtensions, HeadlessUnitTestSession};
fn main() {
    HeadlessUnitTestSession::start_new(|| {
        AppBuilder::configure::<Application>().use_headless(Default::default())
    })
    .dispatch(
        || {
            let leaf = Border::new();
            leaf.set_width(400.);
            leaf.set_height(20.);
            let middle = Border::new();
            middle.set_child(leaf.clone());
            let root = Border::new();
            root.set_child(middle.clone());
            root.measure(Size::INFINITY);
            let before = root.desired_size().width;
            leaf.set_width(80.);
            root.invalidate_measure();
            root.measure(Size::INFINITY);
            let direct = root.desired_size().width;
            middle.invalidate_measure();
            root.invalidate_measure();
            root.measure(Size::INFINITY);
            let reconciled = root.desired_size().width;
            eprintln!("before={before}, direct={direct}, reconciled={reconciled}");
            assert_eq!(before, 400.);
            assert_eq!(reconciled, 80.);
            assert_eq!(direct, 80., "WinUI-style synchronous nested measurement");
        },
        ferroui_base::threading::CancellationToken::none(),
    )
    .wait()
    .unwrap();
}
