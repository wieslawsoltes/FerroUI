//! Run with `cargo run -p qml-gallery`.
use ferroui_qml_controls::{ControlRegistry, QmlView};
use ferroui_view::prelude::*;

include!(concat!(env!("OUT_DIR"), "/main_qml.rs"));

fn main() {
    launch_themed(
        || ferroui_themes_winui::WinUITheme::new().as_style(),
        || {
            let registry = ControlRegistry::standard().expect("register native controls");
            let view = QmlView::new(&compiled_qml(), &registry, Default::default())
                .expect("instantiate Main.qml");
            let content = view.root();
            // The view owns the live binding graph and native event subscriptions.
            on_cleanup(move || drop(view));
            window()
                .title("FerroUI QML")
                .width(420.0)
                .height(260.0)
                .child(content)
                .into()
        },
        |_| {},
    );
}
