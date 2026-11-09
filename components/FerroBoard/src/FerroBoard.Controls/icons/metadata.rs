//! Static declarations over the original icon cache and native vector factory.
use super::IconFactory;
use ferroboard_core::{keys::KeyIcon, text::Utf16Text};
use ferroui_base::{
    Ref,
    media::{Geometry, IBrush},
    metadata::{MarkupType, MarkupTyped},
};
use ferroui_controls::shapes::Path;
use std::rc::Rc;
ferroui_base::ferro_markup_type!(static IconFactory{
 namespace:"FerroBoard.Controls.Icons",
 methods:[
 static fn Register(KeyIcon,Option<String>)=>(|icon,path:Option<String>|IconFactory::register_nullable(icon,path.map(|v|Utf16Text::from(v.as_str())))),
 static fn HasIcon(KeyIcon)->bool=>IconFactory::has_icon,
 static try fn Create(KeyIcon,f64,Option<Rc<dyn IBrush>>)->Ref<Path> =>IconFactory::create,
 static try fn CreateGeometry(KeyIcon,f64)->Ref<Geometry> =>IconFactory::create_geometry
 ]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::icons::IconFactory],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register_all(&[IconFactory::MARKUP]);
}
