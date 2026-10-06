use super::RenderDataOpcode;
use crate::media::{RenderOptions, TextOptions};
use crate::rendering::composition::transport::{BatchValue, BatchValueReader};
use crate::{Matrix, Point, Rect, RoundedRect};

/// The fixed-size part of an operation of a render data stream, written
/// after its opcode.
pub trait IRenderDataPayload: BatchValue + Copy {
    /// The opcode written before the payload.
    const OPCODE: RenderDataOpcode;
}

/// Implements a payload: its opcode, and its encoding (the fields in
/// declaration order, as upstream blits the struct).
macro_rules! render_data_payload {
    ($name:ident => $opcode:ident { $($field:ident : $fty:ty),* $(,)? }) => {
        impl IRenderDataPayload for $name {
            const OPCODE: RenderDataOpcode = RenderDataOpcode::$opcode;
        }

        impl BatchValue for $name {
            #[inline]
            fn write_to(&self, out: &mut Vec<u8>) {
                $(self.$field.write_to(out);)*
            }

            #[inline]
            fn read_from(input: &mut BatchValueReader<'_>) -> Self {
                $(let $field = <$fty>::read_from(input);)*
                Self { $($field,)* }
            }
        }
    };
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DrawLinePayload {
    pub server_pen: i32,
    pub client_pen: i32,
    pub p1: Point,
    pub p2: Point,
}

render_data_payload!(DrawLinePayload => DrawLine { server_pen: i32, client_pen: i32, p1: Point, p2: Point });

/// Followed by `box_shadow_count` box shadows.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DrawRectanglePayload {
    pub server_brush: i32,
    pub server_pen: i32,
    pub client_pen: i32,
    pub rect: RoundedRect,
    pub box_shadow_count: i32,
}

render_data_payload!(DrawRectanglePayload => DrawRectangle {
    server_brush: i32,
    server_pen: i32,
    client_pen: i32,
    rect: RoundedRect,
    box_shadow_count: i32,
});

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DrawEllipsePayload {
    pub server_brush: i32,
    pub server_pen: i32,
    pub client_pen: i32,
    pub rect: Rect,
}

render_data_payload!(DrawEllipsePayload => DrawEllipse {
    server_brush: i32,
    server_pen: i32,
    client_pen: i32,
    rect: Rect,
});

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DrawGeometryPayload {
    pub server_brush: i32,
    pub server_pen: i32,
    pub client_pen: i32,
    pub geometry: i32,
}

render_data_payload!(DrawGeometryPayload => DrawGeometry {
    server_brush: i32,
    server_pen: i32,
    client_pen: i32,
    geometry: i32,
});

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DrawGlyphRunPayload {
    pub server_brush: i32,
    pub glyph_run: i32,
}

render_data_payload!(DrawGlyphRunPayload => DrawGlyphRun { server_brush: i32, glyph_run: i32 });

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DrawBitmapPayload {
    pub bitmap: i32,
    pub opacity: f64,
    pub source_rect: Rect,
    pub dest_rect: Rect,
}

render_data_payload!(DrawBitmapPayload => DrawBitmap { bitmap: i32, opacity: f64, source_rect: Rect, dest_rect: Rect });

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DrawCustomPayload {
    pub operation: i32,
}

render_data_payload!(DrawCustomPayload => DrawCustom { operation: i32 });

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PushClipPayload {
    pub clip: RoundedRect,
}

render_data_payload!(PushClipPayload => PushClip { clip: RoundedRect });

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PushGeometryClipPayload {
    pub geometry: i32,
}

render_data_payload!(PushGeometryClipPayload => PushGeometryClip { geometry: i32 });

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PushOpacityPayload {
    pub opacity: f64,
}

render_data_payload!(PushOpacityPayload => PushOpacity { opacity: f64 });

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PushOpacityMaskPayload {
    pub brush: i32,
    pub bounds: Rect,
}

render_data_payload!(PushOpacityMaskPayload => PushOpacityMask { brush: i32, bounds: Rect });

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PushTransformPayload {
    pub matrix: Matrix,
}

render_data_payload!(PushTransformPayload => PushTransform { matrix: Matrix });

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PushRenderOptionsPayload {
    pub options: RenderOptions,
}

render_data_payload!(PushRenderOptionsPayload => PushRenderOptions { options: RenderOptions });

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PushTextOptionsPayload {
    pub options: TextOptions,
}

render_data_payload!(PushTextOptionsPayload => PushTextOptions { options: TextOptions });

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PushEffectPayload {
    pub effect: i32,
    pub bounds: Rect,
}

render_data_payload!(PushEffectPayload => PushEffect { effect: i32, bounds: Rect });
