use crate::rendering::composition::transport::{BatchValue, BatchValueReader};

/// The opcode that starts every operation of a render data stream.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum RenderDataOpcode {
    #[default]
    Invalid = 0,
    DrawLine,
    DrawRectangle,
    DrawEllipse,
    DrawGeometry,
    DrawGlyphRun,
    DrawBitmap,
    DrawCustom,
    PushClip,
    PushGeometryClip,
    PushOpacity,
    PushOpacityMask,
    PushTransform,
    PushRenderOptions,
    PushTextOptions,
    PushEffect,
    Pop,
}

impl RenderDataOpcode {
    const ALL: [RenderDataOpcode; 17] = [
        Self::Invalid,
        Self::DrawLine,
        Self::DrawRectangle,
        Self::DrawEllipse,
        Self::DrawGeometry,
        Self::DrawGlyphRun,
        Self::DrawBitmap,
        Self::DrawCustom,
        Self::PushClip,
        Self::PushGeometryClip,
        Self::PushOpacity,
        Self::PushOpacityMask,
        Self::PushTransform,
        Self::PushRenderOptions,
        Self::PushTextOptions,
        Self::PushEffect,
        Self::Pop,
    ];
}

/// One byte, as the `byte`-backed enumeration of upstream. A byte that
/// names no opcode reads as [`RenderDataOpcode::Invalid`].
impl BatchValue for RenderDataOpcode {
    #[inline]
    fn write_to(&self, out: &mut Vec<u8>) {
        out.push(*self as u8);
    }

    #[inline]
    fn read_from(input: &mut BatchValueReader<'_>) -> Self {
        let value = input.take::<1>()[0];
        Self::ALL.get(value as usize).copied().unwrap_or(Self::Invalid)
    }
}
