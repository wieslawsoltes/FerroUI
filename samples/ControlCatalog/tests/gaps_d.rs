//! Minimal reproductions of gaps of the framework found while porting the
//! navigation page samples (see `gaps.rs`).

use super::support::*;
use ferroui_base::media::ImageBrush;
use ferroui_base::metadata::from_markup_value;
use ferroui_base::Ref;
use ferroui_controls::Border;

const XMLNS: &str = "xmlns='https://github.com/ferroui' xmlns:x='http://schemas.microsoft.com/winfx/2006/xaml'";

#[test]
#[ignore = "gap C400: the text of ImageBrush.Source is converted to an IImage, which is not accepted as an IImageBrushSource"]
fn gap_c400_image_brush_source_from_text() {
    let _app = start_catalog_application();
    let border = from_markup_value::<Ref<Border>>(&Some(load_text(&format!(
        "<Border {XMLNS}>\
           <Border.Background>\
             <ImageBrush Source='ferres://ControlCatalog/Assets/CurvedHeader/featured.jpg' Stretch='UniformToFill'/>\
           </Border.Background>\
         </Border>"
    ))))
    .expect("a border");
    let brush = border
        .background()
        .and_then(|brush| brush.as_object().and_then(|object| object.to_ref().cast::<ImageBrush>()))
        .expect("an image brush");
    assert!(brush.source().is_some());
}
