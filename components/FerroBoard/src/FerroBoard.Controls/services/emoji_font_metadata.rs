//! Selected source static API; installation methods remain explicitly unavailable.
use super::EmojiFontFallback;
use ferroboard_core::text::Utf16Text;
use ferroui_base::metadata::{MarkupType, MarkupTyped};
ferroui_base::ferro_markup_type!(static EmojiFontFallback {
 namespace:"FerroBoard.Controls",
 fields:[
  FamilyName:Utf16Text => EmojiFontFallback::family_name,
  DefaultNotoColorEmojiUrl:Utf16Text => EmojiFontFallback::default_noto_color_emoji_url
 ],
 methods:[static fn IsEmojiCodePoint(i32)->bool => EmojiFontFallback::is_emoji_code_point]
});
ferroui_base::ferro_rust_paths! {classes:[],types:[crate::EmojiFontFallback],contracts:[],generics:[],generic_contracts:[]}
pub(crate) fn register() {
    register_rust_paths();
    MarkupType::register_all(&[EmojiFontFallback::MARKUP]);
}
