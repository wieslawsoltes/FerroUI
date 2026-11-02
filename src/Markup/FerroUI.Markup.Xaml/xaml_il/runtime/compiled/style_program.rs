//! Repeated instructions in compiled nested styles. Each site names only the
//! property getters it invokes. Evaluating a getter can register metadata and must
//! stay ordered with setter construction and application.
use super::{
    add_dynamic_resource_setter_with_parent, cast, to_value, XamlIlContext, XamlLoadError,
};
use ferroui_base::styling::{Style, StyleBase};
use ferroui_base::{FerroProperty, Ref};
use std::rc::Rc;

#[inline(never)]
pub fn begin(context: &Rc<XamlIlContext>) -> Ref<Style> {
    let style = Style::new();
    context.push_parent(to_value(style.clone()));
    style
}

#[inline(never)]
pub fn attach(
    parent: &Ref<StyleBase>,
    child: &Ref<Style>,
    line: i32,
    column: i32,
) -> Result<(), XamlLoadError> {
    StyleBase::__markup_Add_1(parent, cast(child.clone(), line, column)?);
    Ok(())
}

pub type DynamicSetter = (fn() -> &'static FerroProperty, &'static str);

#[inline(never)]
pub fn finish_dynamic(
    context: &Rc<XamlIlContext>,
    parent: &Ref<StyleBase>,
    child: &Ref<Style>,
    setters: &[DynamicSetter],
    line: i32,
    column: i32,
) -> Result<(), XamlLoadError> {
    for (get_property, key) in setters {
        add_dynamic_resource_setter_with_parent(
            context,
            child.upcast_ref::<StyleBase>(),
            get_property(),
            key,
        );
    }
    context.pop_parent();
    attach(parent, child, line, column)
}
