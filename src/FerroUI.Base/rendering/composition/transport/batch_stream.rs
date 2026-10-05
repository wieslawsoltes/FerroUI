//! The serialized form of composition changes.
//!
//! As upstream, a batch carries two independent streams: plain values
//! ("structs") and object references. Values are encoded explicitly
//! (little-endian) through [`BatchValue`] instead of being blitted, so no
//! `unsafe` is involved; object slots are a closed enum, [`BatchObject`],
//! which is where the thread-crossing contract of the transport is visible.

use crate::rendering::composition::server::{ServerCompositor, ServerObjectId};
use crate::rendering::composition::server::IServerObject;
use crate::media::imaging::{BitmapBlendingMode, BitmapInterpolationMode};
use crate::media::{
    AlignmentX, AlignmentY, BaselinePixelAlignment, Color, EdgeMode, GradientSpreadMethod, PenLineCap, PenLineJoin,
    RenderOptions, Stretch, TextHintingMode, TextOptions, TextRenderingMode, TileMode,
};
use crate::numerics::{Matrix3x2, Matrix4x4, Quaternion, Vector2, Vector3, Vector4};
use crate::rendering::composition::{
    CompositionBlendMode, CompositionGradientExtendMode, CompositionStretch, CompositionTileMode,
    CompositionTransparencyLevel,
};
use crate::rendering::{LayoutPassTiming, RendererDebugOverlays};
use crate::{
    CornerRadius, Matrix, Point, Rect, RelativePoint, RelativeRect, RelativeScalar, RelativeUnit, RoundedRect, Size,
    Vector, Vector3D,
};
use std::any::Any;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;

/// A job executed by the server compositor on the render thread.
pub type ServerJob = Box<dyn FnOnce(&ServerCompositor)>;

/// A job executed by the server compositor on the render thread with a
/// server object, which is resolved when the batch is read: upstream the
/// job holds the server object itself, so it reaches an object the same
/// batch disposes (the dispose list of a batch precedes its jobs).
pub type ServerObjectJob = Box<dyn FnOnce(&ServerCompositor, Option<Rc<dyn IServerObject>>)>;

/// Creates the server-side counterpart of a composition object on the
/// render thread.
pub type ServerObjectFactory = Box<dyn FnOnce(&Rc<ServerCompositor>, ServerObjectId) -> Rc<dyn IServerObject>>;

/// Markers that structure the object stream of a batch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatchMarker {
    /// Followed by a count (value stream) and that many `ServerObject`,
    /// `Create` pairs.
    CreateStart,
    /// Followed by a count (value stream) and that many `ServerObject`s to
    /// dispose.
    RenderThreadDisposeStart,
    RenderThreadJobsStart,
    RenderThreadJobsEnd,
    RenderThreadPostTargetJobsStart,
    RenderThreadPostTargetJobsEnd,
    /// Debug aid: written after each object's changes when serialization
    /// checks are enabled.
    ObjectEnd,
}

/// One slot of the object stream.
///
/// Everything a UI-thread object hands to the server goes through one of
/// these variants. `Job`, `Create` and `Value` payloads are not `Send`
/// today because the platform resource handles they carry (`Rc<dyn
/// IGeometryImpl>`, bitmaps, glyph runs, immutable brushes) are `Rc`-based;
/// see the threading notes in `rendering/composition/mod.rs`.
pub enum BatchObject {
    Null,
    /// A reference to a server-side object.
    ServerObject(ServerObjectId),
    Marker(BatchMarker),
    Job(ServerJob),
    /// Preceded by the `ServerObject` the job works with.
    ObjectJob(ServerObjectJob),
    Create(ServerObjectFactory),
    /// Any other reference-typed payload (render data resources, platform
    /// handles, lists).
    Value(Box<dyn Any>),
}

impl BatchObject {
    /// Wraps a payload value.
    pub fn value<T: Any>(value: T) -> BatchObject {
        BatchObject::Value(Box::new(value))
    }
}

/// A reference-typed property value that is either sent as it is (an
/// immutable brush, an immutable transform) or names a server object (the
/// server-side counterpart of a mutable brush): what `GetServer` yields for
/// a compositor.
pub enum BatchResource<T: ?Sized> {
    Value(Rc<T>),
    Server(ServerObjectId),
}

impl<T: ?Sized> Clone for BatchResource<T> {
    fn clone(&self) -> Self {
        match self {
            BatchResource::Value(value) => BatchResource::Value(value.clone()),
            BatchResource::Server(id) => BatchResource::Server(*id),
        }
    }
}

/// The two streams of a batch.
#[derive(Default)]
pub struct BatchStreamData {
    pub(crate) objects: VecDeque<BatchObject>,
    pub(crate) structs: Vec<u8>,
    pub(crate) struct_read_offset: usize,
}

impl BatchStreamData {
    pub fn new() -> Self {
        Self::default()
    }

    /// Empties both streams, keeping their capacity for reuse.
    pub(crate) fn reset(&mut self) {
        self.objects.clear();
        self.structs.clear();
        self.struct_read_offset = 0;
    }
}

/// A plain value that can be written to the value stream of a batch.
pub trait BatchValue: Sized {
    fn write_to(&self, out: &mut Vec<u8>);
    fn read_from(input: &mut BatchValueReader<'_>) -> Self;
}

/// A cursor over the value stream.
pub struct BatchValueReader<'a> {
    data: &'a [u8],
    offset: &'a mut usize,
}

impl BatchValueReader<'_> {
    /// Takes the next `N` bytes. Panics at the end of the stream.
    pub fn take<const N: usize>(&mut self) -> [u8; N] {
        let end = *self.offset + N;
        if end > self.data.len() {
            panic!("attempted to read past the end of the batch value stream");
        }
        let mut bytes = [0u8; N];
        bytes.copy_from_slice(&self.data[*self.offset..end]);
        *self.offset = end;
        bytes
    }
}

macro_rules! impl_batch_value_for_number {
    ($($ty:ty),*) => {$(
        impl BatchValue for $ty {
            #[inline]
            fn write_to(&self, out: &mut Vec<u8>) {
                out.extend_from_slice(&self.to_le_bytes());
            }
            #[inline]
            fn read_from(input: &mut BatchValueReader<'_>) -> Self {
                <$ty>::from_le_bytes(input.take())
            }
        }
    )*};
}

impl_batch_value_for_number!(u8, i8, u16, i16, u32, i32, u64, i64, u128, f32, f64);

impl BatchValue for bool {
    fn write_to(&self, out: &mut Vec<u8>) {
        out.push(u8::from(*self));
    }
    fn read_from(input: &mut BatchValueReader<'_>) -> Self {
        input.take::<1>()[0] != 0
    }
}

impl BatchValue for Duration {
    fn write_to(&self, out: &mut Vec<u8>) {
        self.as_nanos().write_to(out);
    }
    fn read_from(input: &mut BatchValueReader<'_>) -> Self {
        let nanos = u128::read_from(input);
        Duration::new((nanos / 1_000_000_000) as u64, (nanos % 1_000_000_000) as u32)
    }
}

impl<T: BatchValue> BatchValue for Option<T> {
    fn write_to(&self, out: &mut Vec<u8>) {
        match self {
            Some(value) => {
                true.write_to(out);
                value.write_to(out);
            }
            None => false.write_to(out),
        }
    }
    fn read_from(input: &mut BatchValueReader<'_>) -> Self {
        if bool::read_from(input) {
            Some(T::read_from(input))
        } else {
            None
        }
    }
}

macro_rules! impl_batch_value_for_fields {
    ($ty:ty { $($field:ident : $fty:ty),* } => $ctor:expr) => {
        impl BatchValue for $ty {
            fn write_to(&self, out: &mut Vec<u8>) {
                $(self.$field.write_to(out);)*
            }
            fn read_from(input: &mut BatchValueReader<'_>) -> Self {
                $(let $field = <$fty>::read_from(input);)*
                $ctor
            }
        }
    };
}

impl_batch_value_for_fields!(Point { x: f64, y: f64 } => Point::new(x, y));
impl_batch_value_for_fields!(Vector { x: f64, y: f64 } => Vector::new(x, y));
impl_batch_value_for_fields!(Size { width: f64, height: f64 } => Size::new(width, height));
impl_batch_value_for_fields!(Rect { x: f64, y: f64, width: f64, height: f64 } => Rect::new(x, y, width, height));
impl_batch_value_for_fields!(Vector3D { x: f64, y: f64, z: f64 } => Vector3D::new(x, y, z));
impl_batch_value_for_fields!(RoundedRect {
    rect: Rect, radii_top_left: Vector, radii_top_right: Vector, radii_bottom_left: Vector, radii_bottom_right: Vector
} => RoundedRect { rect, radii_top_left, radii_top_right, radii_bottom_left, radii_bottom_right });

impl_batch_value_for_fields!(Vector2 { x: f32, y: f32 } => Vector2::new(x, y));
impl_batch_value_for_fields!(Vector3 { x: f32, y: f32, z: f32 } => Vector3::new(x, y, z));
impl_batch_value_for_fields!(Vector4 { x: f32, y: f32, z: f32, w: f32 } => Vector4::new(x, y, z, w));
impl_batch_value_for_fields!(Quaternion { x: f32, y: f32, z: f32, w: f32 } => Quaternion::new(x, y, z, w));
impl_batch_value_for_fields!(Matrix3x2 { m11: f32, m12: f32, m21: f32, m22: f32, m31: f32, m32: f32 }
    => Matrix3x2::new(m11, m12, m21, m22, m31, m32));
impl_batch_value_for_fields!(Matrix4x4 {
    m11: f32, m12: f32, m13: f32, m14: f32, m21: f32, m22: f32, m23: f32, m24: f32,
    m31: f32, m32: f32, m33: f32, m34: f32, m41: f32, m42: f32, m43: f32, m44: f32
} => Matrix4x4 { m11, m12, m13, m14, m21, m22, m23, m24, m31, m32, m33, m34, m41, m42, m43, m44 });
impl_batch_value_for_fields!(CornerRadius { top_left: f64, top_right: f64, bottom_right: f64, bottom_left: f64 }
    => CornerRadius::new(top_left, top_right, bottom_right, bottom_left));
impl_batch_value_for_fields!(RelativePoint { point: Point, unit: RelativeUnit }
    => RelativePoint::from_point(point, unit));
impl_batch_value_for_fields!(RelativeScalar { scalar: f64, unit: RelativeUnit } => RelativeScalar::new(scalar, unit));
impl_batch_value_for_fields!(RelativeRect { rect: Rect, unit: RelativeUnit } => RelativeRect::from_rect(rect, unit));
impl_batch_value_for_fields!(LayoutPassTiming { pass_counter: i32, elapsed: Duration }
    => LayoutPassTiming::new(pass_counter, elapsed));
impl_batch_value_for_fields!(RenderOptions {
    text_rendering_mode: TextRenderingMode,
    bitmap_interpolation_mode: BitmapInterpolationMode,
    edge_mode: EdgeMode,
    bitmap_blending_mode: BitmapBlendingMode,
    requires_full_opacity_handling: Option<bool>
} => RenderOptions {
    text_rendering_mode,
    bitmap_interpolation_mode,
    edge_mode,
    bitmap_blending_mode,
    requires_full_opacity_handling,
});
impl_batch_value_for_fields!(TextOptions {
    text_rendering_mode: TextRenderingMode,
    text_hinting_mode: TextHintingMode,
    baseline_pixel_alignment: BaselinePixelAlignment
} => TextOptions { text_rendering_mode, text_hinting_mode, baseline_pixel_alignment });

/// Enumerations cross as their numeric value. A value that names no member
/// cannot have been written by the other side: reading one panics.
macro_rules! impl_batch_value_for_enum {
    ($ty:ty as $repr:ty { $($variant:ident),* $(,)? }) => {
        impl BatchValue for $ty {
            #[inline]
            fn write_to(&self, out: &mut Vec<u8>) {
                (*self as $repr).write_to(out);
            }
            fn read_from(input: &mut BatchValueReader<'_>) -> Self {
                let value = <$repr>::read_from(input);
                $(if value == <$ty>::$variant as $repr {
                    return <$ty>::$variant;
                })*
                panic!("the batch value stream holds {value}, which is not a {}", stringify!($ty));
            }
        }
    };
}

impl_batch_value_for_enum!(RelativeUnit as u8 { Relative, Absolute });
impl_batch_value_for_enum!(PenLineCap as i32 { Flat, Round, Square });
impl_batch_value_for_enum!(PenLineJoin as i32 { Bevel, Miter, Round });
impl_batch_value_for_enum!(AlignmentX as i32 { Left, Center, Right });
impl_batch_value_for_enum!(AlignmentY as i32 { Top, Center, Bottom });
impl_batch_value_for_enum!(Stretch as i32 { None, Fill, Uniform, UniformToFill });
impl_batch_value_for_enum!(TileMode as i32 { None, FlipX, FlipY, FlipXY, Tile });
impl_batch_value_for_enum!(GradientSpreadMethod as i32 { Pad, Reflect, Repeat });
impl_batch_value_for_enum!(TextRenderingMode as u8 { Unspecified, SubpixelAntialias, Antialias, Alias });
impl_batch_value_for_enum!(TextHintingMode as u8 { Unspecified, None, Light, Strong });
impl_batch_value_for_enum!(BaselinePixelAlignment as u8 { Unspecified, Aligned, Unaligned });
impl_batch_value_for_enum!(EdgeMode as u8 { Unspecified, Antialias, Aliased });
impl_batch_value_for_enum!(BitmapInterpolationMode as u8 {
    Unspecified, None, LowQuality, MediumQuality, HighQuality
});
impl_batch_value_for_enum!(BitmapBlendingMode as u8 {
    Unspecified, SourceOver, Source, Destination, DestinationOver, SourceIn, DestinationIn, SourceOut,
    DestinationOut, SourceAtop, DestinationAtop, Xor, Plus, Screen, Overlay, Darken, Lighten, ColorDodge,
    ColorBurn, HardLight, SoftLight, Difference, Exclusion, Multiply, Hue, Saturation, Color, Luminosity
});
impl_batch_value_for_enum!(CompositionTransparencyLevel as u8 { None, Transparent, Blur, AcrylicBlur, Mica });
impl_batch_value_for_enum!(CompositionBlendMode as i32 {
    Clear, Src, Dst, SrcOver, DstOver, SrcIn, DstIn, SrcOut, DstOut, SrcATop, DstATop, Xor, Plus, Modulate,
    Screen, Overlay, Darken, Lighten, ColorDodge, ColorBurn, HardLight, SoftLight, Difference, Exclusion,
    Multiply, Hue, Saturation, Color, Luminosity
});
impl_batch_value_for_enum!(CompositionGradientExtendMode as i32 { Clamp, Wrap, Mirror });
impl_batch_value_for_enum!(CompositionStretch as i32 { None, Fill });

impl BatchValue for RendererDebugOverlays {
    fn write_to(&self, out: &mut Vec<u8>) {
        self.bits().write_to(out);
    }
    fn read_from(input: &mut BatchValueReader<'_>) -> Self {
        RendererDebugOverlays::from_bits_retain(i32::read_from(input))
    }
}

impl BatchValue for CompositionTileMode {
    fn write_to(&self, out: &mut Vec<u8>) {
        self.bits().write_to(out);
    }
    fn read_from(input: &mut BatchValueReader<'_>) -> Self {
        CompositionTileMode::from_bits_retain(i32::read_from(input))
    }
}

impl BatchValue for Color {
    fn write_to(&self, out: &mut Vec<u8>) {
        self.to_uint32().write_to(out);
    }
    fn read_from(input: &mut BatchValueReader<'_>) -> Self {
        Color::from_uint32(u32::read_from(input))
    }
}

impl BatchValue for Matrix {
    fn write_to(&self, out: &mut Vec<u8>) {
        for value in [self.m11, self.m12, self.m13, self.m21, self.m22, self.m23, self.m31, self.m32, self.m33] {
            value.write_to(out);
        }
    }
    fn read_from(input: &mut BatchValueReader<'_>) -> Self {
        let mut v = [0.0f64; 9];
        for value in &mut v {
            *value = f64::read_from(input);
        }
        Matrix::new_3x3(v[0], v[1], v[2], v[3], v[4], v[5], v[6], v[7], v[8])
    }
}

/// Writes changes into the streams of a batch.
pub struct BatchStreamWriter<'a> {
    output: &'a mut BatchStreamData,
}

impl<'a> BatchStreamWriter<'a> {
    pub fn new(output: &'a mut BatchStreamData) -> Self {
        Self { output }
    }

    /// Writes a plain value to the value stream.
    #[inline]
    pub fn write<T: BatchValue>(&mut self, item: T) {
        item.write_to(&mut self.output.structs);
    }

    /// Writes an object slot to the object stream.
    #[inline]
    pub fn write_object(&mut self, item: BatchObject) {
        self.output.objects.push_back(item);
    }

    /// Writes a reference to a server object (or null).
    #[inline]
    pub fn write_server_object(&mut self, id: Option<ServerObjectId>) {
        self.write_object(match id {
            Some(id) => BatchObject::ServerObject(id),
            None => BatchObject::Null,
        });
    }

    /// Writes a payload value (or null) to the object stream.
    #[inline]
    pub fn write_value<T: Any>(&mut self, item: Option<T>) {
        self.write_object(match item {
            Some(item) => BatchObject::value(item),
            None => BatchObject::Null,
        });
    }

    /// Writes a resource reference (or null) to the object stream.
    #[inline]
    pub fn write_resource<T: ?Sized + 'static>(&mut self, item: Option<BatchResource<T>>) {
        self.write_object(match item {
            Some(BatchResource::Value(value)) => BatchObject::value(value),
            Some(BatchResource::Server(id)) => BatchObject::ServerObject(id),
            None => BatchObject::Null,
        });
    }

    /// Writes a marker to the object stream.
    #[inline]
    pub fn write_marker(&mut self, marker: BatchMarker) {
        self.write_object(BatchObject::Marker(marker));
    }
}

/// Reads changes from the streams of a batch.
pub struct BatchStreamReader<'a> {
    input: &'a mut BatchStreamData,
}

impl<'a> BatchStreamReader<'a> {
    pub fn new(input: &'a mut BatchStreamData) -> Self {
        Self { input }
    }

    /// Reads a plain value from the value stream. Panics at the end of the
    /// stream.
    #[inline]
    pub fn read<T: BatchValue>(&mut self) -> T {
        let mut reader =
            BatchValueReader { data: &self.input.structs, offset: &mut self.input.struct_read_offset };
        T::read_from(&mut reader)
    }

    /// Reads the next object slot. Panics at the end of the stream.
    #[inline]
    pub fn read_object(&mut self) -> BatchObject {
        match self.input.objects.pop_front() {
            Some(object) => object,
            None => panic!("attempted to read past the end of the batch object stream"),
        }
    }

    /// Reads a reference to a server object (or null).
    pub fn read_server_object(&mut self) -> Option<ServerObjectId> {
        match self.read_object() {
            BatchObject::Null => None,
            BatchObject::ServerObject(id) => Some(id),
            _ => panic!("the batch object stream does not hold a server object reference here"),
        }
    }

    /// Reads a payload value of type `T` (or null).
    pub fn read_value<T: Any>(&mut self) -> Option<T> {
        match self.read_object() {
            BatchObject::Null => None,
            BatchObject::Value(value) => match value.downcast::<T>() {
                Ok(value) => Some(*value),
                Err(_) => panic!("the batch object stream holds a value of another type here"),
            },
            _ => panic!("the batch object stream does not hold a value here"),
        }
    }

    /// Reads a resource reference (or null): a payload value `Rc<T>` or
    /// the id of a server object.
    pub fn read_resource<T: ?Sized + 'static>(&mut self) -> Option<BatchResource<T>> {
        match self.read_object() {
            BatchObject::Null => None,
            BatchObject::ServerObject(id) => Some(BatchResource::Server(id)),
            BatchObject::Value(value) => match value.downcast::<Rc<T>>() {
                Ok(value) => Some(BatchResource::Value(*value)),
                Err(_) => panic!("the batch object stream holds a value of another type here"),
            },
            _ => panic!("the batch object stream does not hold a resource reference here"),
        }
    }

    /// Whether the object stream is exhausted.
    pub fn is_object_eof(&self) -> bool {
        self.input.objects.is_empty()
    }

    /// Whether the value stream is exhausted.
    pub fn is_struct_eof(&self) -> bool {
        self.input.struct_read_offset >= self.input.structs.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_stream_correctly_writes_and_reads_data() {
        let mut data = BatchStreamData::new();
        let values: Vec<u128> = (0..453u128).map(|c| c.wrapping_mul(0x9E37_79B9_7F4A_7C15_F39C_C060_5CED_C835)).collect();
        let objects: Vec<Rc<()>> = (0..453).map(|_| Rc::new(())).collect();

        {
            let mut writer = BatchStreamWriter::new(&mut data);
            for value in &values {
                writer.write(*value);
            }
            for object in &objects {
                writer.write_object(BatchObject::value(object.clone()));
            }
        }

        let mut reader = BatchStreamReader::new(&mut data);
        for value in &values {
            assert_eq!(*value, reader.read::<u128>());
        }
        for object in &objects {
            assert!(Rc::ptr_eq(object, &reader.read_value::<Rc<()>>().unwrap()));
        }
        assert!(reader.is_object_eof());
        assert!(reader.is_struct_eof());
    }

    #[test]
    fn values_round_trip() {
        let mut data = BatchStreamData::new();
        {
            let mut writer = BatchStreamWriter::new(&mut data);
            writer.write(true);
            writer.write(-7i32);
            writer.write(1.5f32);
            writer.write(Some(Rect::new(1.0, 2.0, 3.0, 4.0)));
            writer.write(None::<f64>);
            writer.write(Matrix::create_translation(3.0, 4.0));
            writer.write(Duration::from_millis(1234));
            writer.write(Color::from_uint32(0x80112233));
            writer.write_server_object(None);
            writer.write_marker(BatchMarker::RenderThreadJobsEnd);
        }
        let mut reader = BatchStreamReader::new(&mut data);
        assert!(reader.read::<bool>());
        assert_eq!(reader.read::<i32>(), -7);
        assert_eq!(reader.read::<f32>(), 1.5);
        assert_eq!(reader.read::<Option<Rect>>(), Some(Rect::new(1.0, 2.0, 3.0, 4.0)));
        assert_eq!(reader.read::<Option<f64>>(), None);
        assert_eq!(reader.read::<Matrix>(), Matrix::create_translation(3.0, 4.0));
        assert_eq!(reader.read::<Duration>(), Duration::from_millis(1234));
        assert_eq!(reader.read::<Color>(), Color::from_uint32(0x80112233));
        assert_eq!(reader.read_server_object(), None);
        assert!(matches!(reader.read_object(), BatchObject::Marker(BatchMarker::RenderThreadJobsEnd)));
    }

    #[test]
    #[should_panic(expected = "past the end")]
    fn reading_past_the_end_panics() {
        let mut data = BatchStreamData::new();
        let mut reader = BatchStreamReader::new(&mut data);
        let _ = reader.read::<i32>();
    }
}
