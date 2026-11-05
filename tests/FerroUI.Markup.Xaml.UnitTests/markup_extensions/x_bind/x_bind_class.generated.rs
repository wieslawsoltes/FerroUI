// Placeholder of step X7 of #525 (x_bind_class.xaml), written by hand so that the tests compile
// before the fixture is regenerated; the regeneration replaces this file with the emitter's
// output (`rust_emitter::generate_class_file_with`): `populate`, the generated `bindings()`
// accessor of the class and the loader table.

#![allow(dead_code, unused_imports)]

use ::ferroui_markup_xaml::xaml_il::runtime::compiled as rt;

pub fn populate(
    _service_provider: ::core::option::Option<::std::rc::Rc<dyn ::ferroui_base::metadata::IServiceProvider>>,
    _root: &::ferroui_base::Ref<crate::support::x_bind::XBindPage>,
) -> ::core::result::Result<(), ::ferroui_markup_xaml::XamlLoadError> {
    ::core::result::Result::Err(::ferroui_markup_xaml::XamlLoadError::with_message("regenerate the x:Bind class fixture"))
}

impl crate::support::x_bind::XBindPage {
    /// The `{x:Bind}` bindings of the root of the document of this class (WinUI's
    /// `this.Bindings`): `initialize`, `update` and `stop_tracking`. `None` until the document
    /// has populated the instance, or when it has no `x:Bind` of its root.
    pub fn bindings(&self) -> ::core::option::Option<::std::rc::Rc<::ferroui_markup_xaml::markup_extensions::XBindings>> {
        ::ferroui_markup_xaml::markup_extensions::XBindings::get_for_root(self)
    }
}
