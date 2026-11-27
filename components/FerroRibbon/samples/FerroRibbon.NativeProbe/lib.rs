extern crate self as ferroribbon_native_probe;
pub mod layout_preview;
mod metadata;
pub mod view_model;
use ferroui_base::{
    Ref,
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_controls::Window;
use std::rc::Rc;
// The pinned markup emitter clones literal bools; scope the lint exemption to its output.
#[allow(clippy::clone_on_copy)]
mod generated {
    ferroui_markup_xaml::include_compiled_xaml!();
}
pub fn register_types() {
    ferroribbon::register_types();
    ferroui_markup_xaml::register_types();
    ferroui_themes_simple::register_types();
    MarkupType::register_all(&[<view_model::ProbeViewModel as MarkupTyped>::MARKUP]);
    generated::compiled_markup::register();
}
pub fn create_window(model: Rc<view_model::ProbeViewModel>) -> Ref<Window> {
    let value =
        generated::compiled_xaml::try_load(None, "ferres://ferroribbon_native_probe/Main.xaml")
            .expect("compiled window registration")
            .expect("valid compiled window");
    let window = value
        .downcast_ref::<Ref<Window>>()
        .expect("Window root")
        .clone();
    window.set_data_context(Some(model));
    window
}
#[cfg(test)]
mod tests;
