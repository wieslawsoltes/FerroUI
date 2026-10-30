//! Metadata of the existing native RenderOptions type used by the source window.
use ferroui_base::{
    media::{
        imaging::{BitmapBlendingMode, BitmapInterpolationMode},
        EdgeMode, RenderOptions, TextRenderingMode,
    },
    Ref, Visual,
};
/// Metadata carrier for the native value type; owns no rendering state or behavior.
#[doc(hidden)]
pub struct RenderOptionsMetadata;
ferroui_base::ferro_markup_type!(struct RenderOptionsMetadata as "RenderOptions" {
    namespace:"FerroUI.Media",this:RenderOptions,handles:[RenderOptions],
    methods:[
        static fn GetBitmapInterpolationMode(Ref<Visual>)->BitmapInterpolationMode => (|visual:Ref<Visual>|RenderOptions::get_bitmap_interpolation_mode(&visual)),
        static fn SetBitmapInterpolationMode(Ref<Visual>,BitmapInterpolationMode) => (|visual:Ref<Visual>,value|RenderOptions::set_bitmap_interpolation_mode(&visual,value)),
        static fn GetBitmapBlendingMode(Ref<Visual>)->BitmapBlendingMode => (|visual:Ref<Visual>|RenderOptions::get_bitmap_blending_mode(&visual)),
        static fn SetBitmapBlendingMode(Ref<Visual>,BitmapBlendingMode) => (|visual:Ref<Visual>,value|RenderOptions::set_bitmap_blending_mode(&visual,value)),
        static fn GetEdgeMode(Ref<Visual>)->EdgeMode => (|visual:Ref<Visual>|RenderOptions::get_edge_mode(&visual)),
        static fn SetEdgeMode(Ref<Visual>,EdgeMode) => (|visual:Ref<Visual>,value|RenderOptions::set_edge_mode(&visual,value)),
        static fn GetTextRenderingMode(Ref<Visual>)->TextRenderingMode => (|visual:Ref<Visual>|RenderOptions::get_text_rendering_mode(&visual)),
        static fn SetTextRenderingMode(Ref<Visual>,TextRenderingMode) => (|visual:Ref<Visual>,value|RenderOptions::set_text_rendering_mode(&visual,value)),
        static fn GetRequiresFullOpacityHandling(Ref<Visual>)->Option<bool> => (|visual:Ref<Visual>|RenderOptions::get_requires_full_opacity_handling(&visual)),
        static fn SetRequiresFullOpacityHandling(Ref<Visual>,Option<bool>) => (|visual:Ref<Visual>,value|RenderOptions::set_requires_full_opacity_handling(&visual,value)),
    ],
});
