//! The directory in a window of its own.

fn main() {
    view_interop::register_types();
    view_interop::register_components();
    seven_guis_model::run("view-interop", "Directory", 760.0, 480.0, || {
        ferroui_view::IntoElement::into_element(view_interop::directory())
    });
}
