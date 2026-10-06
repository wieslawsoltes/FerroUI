//! Port of `Rendering/SceneGraph/RenderDataWriterReaderTests.cs`.
//!
//! The `try`/`finally` blocks of upstream that dispose the writer are the
//! explicit `dispose` calls at the end of each test.

use crate::media::imaging::{BitmapBlendingMode, BitmapInterpolationMode};
use crate::media::{
    BaselinePixelAlignment, BoxShadow, Color, EdgeMode, RenderOptions, TextHintingMode, TextOptions, TextRenderingMode,
};
use crate::rendering::composition::drawing::{
    PushOpacityPayload, RenderDataOpcode, RenderDataReader, RenderDataWriter,
};
use crate::{Matrix, Point, Rect, RoundedRect, Vector};

#[test]
fn primitives_round_trip() {
    let mut writer = RenderDataWriter::new();
    writer.write::<u8>(200);
    writer.write_opcode(RenderDataOpcode::DrawGeometry);
    writer.write(-123456i32);
    writer.write(4000000000u32);
    writer.write(3.14159f64);
    writer.write(true);
    writer.write(false);

    let mut reader = RenderDataReader::new(writer.written());
    assert_eq!(200, reader.read::<u8>());
    assert_eq!(RenderDataOpcode::DrawGeometry, reader.read::<RenderDataOpcode>());
    assert_eq!(-123456, reader.read::<i32>());
    assert_eq!(4000000000u32, reader.read::<u32>());
    assert_eq!(3.14159, reader.read::<f64>());
    assert!(reader.read::<bool>());
    assert!(!reader.read::<bool>());
    assert!(reader.is_at_end());

    writer.dispose();
}

#[test]
fn geometric_structs_round_trip() {
    let point = Point::new(1.0, 2.0);
    let vector = Vector::new(3.0, 4.0);
    let rect = Rect::new(5.0, 6.0, 7.0, 8.0);
    let rounded_rect = RoundedRect::new(
        Rect::new(1.0, 2.0, 30.0, 40.0),
        Vector::new(1.0, 1.0),
        Vector::new(2.0, 2.0),
        Vector::new(3.0, 3.0),
        Vector::new(4.0, 4.0),
    );
    let matrix = Matrix::new_3x3(1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0);

    let mut writer = RenderDataWriter::new();
    writer.write(point);
    writer.write(vector);
    writer.write(rect);
    writer.write(rounded_rect);
    writer.write(matrix);

    let mut reader = RenderDataReader::new(writer.written());
    assert_eq!(point, reader.read::<Point>());
    assert_eq!(vector, reader.read::<Vector>());
    assert_eq!(rect, reader.read::<Rect>());

    let read_rounded = reader.read::<RoundedRect>();
    assert_eq!(rounded_rect.rect, read_rounded.rect);
    assert_eq!(rounded_rect.radii_top_left, read_rounded.radii_top_left);
    assert_eq!(rounded_rect.radii_top_right, read_rounded.radii_top_right);
    assert_eq!(rounded_rect.radii_bottom_right, read_rounded.radii_bottom_right);
    assert_eq!(rounded_rect.radii_bottom_left, read_rounded.radii_bottom_left);

    assert_eq!(matrix, reader.read::<Matrix>());
    assert!(reader.is_at_end());

    writer.dispose();
}

#[test]
fn color_and_box_shadow_round_trip() {
    let color = Color::from_argb(10, 20, 30, 40);
    let shadow = BoxShadow {
        offset_x: 1.5,
        offset_y: -2.5,
        blur: 3.0,
        spread: 4.0,
        color: Color::from_argb(255, 1, 2, 3),
        is_inset: true,
    };

    let mut writer = RenderDataWriter::new();
    writer.write(color);
    writer.write(shadow);

    let mut reader = RenderDataReader::new(writer.written());
    assert_eq!(color, reader.read::<Color>());

    let read_shadow = reader.read::<BoxShadow>();
    assert_eq!(shadow.offset_x, read_shadow.offset_x);
    assert_eq!(shadow.offset_y, read_shadow.offset_y);
    assert_eq!(shadow.blur, read_shadow.blur);
    assert_eq!(shadow.spread, read_shadow.spread);
    assert_eq!(shadow.color, read_shadow.color);
    assert_eq!(shadow.is_inset, read_shadow.is_inset);
    assert!(reader.is_at_end());

    writer.dispose();
}

#[test]
fn render_options_and_text_options_round_trip() {
    let render_options = RenderOptions {
        text_rendering_mode: TextRenderingMode::Antialias,
        bitmap_interpolation_mode: BitmapInterpolationMode::HighQuality,
        edge_mode: EdgeMode::Aliased,
        bitmap_blending_mode: BitmapBlendingMode::Plus,
        requires_full_opacity_handling: Some(true),
    };
    let text_options = TextOptions {
        text_rendering_mode: TextRenderingMode::SubpixelAntialias,
        text_hinting_mode: TextHintingMode::Light,
        baseline_pixel_alignment: BaselinePixelAlignment::Aligned,
    };

    let mut writer = RenderDataWriter::new();
    writer.write(render_options);
    writer.write(text_options);

    let mut reader = RenderDataReader::new(writer.written());
    assert_eq!(render_options, reader.read::<RenderOptions>());
    assert_eq!(text_options, reader.read::<TextOptions>());
    assert!(reader.is_at_end());

    writer.dispose();
}

fn nullable_boolean_round_trips_all_three_states(value: Option<bool>) {
    let mut writer = RenderDataWriter::new();
    writer.write(RenderOptions { requires_full_opacity_handling: value, ..Default::default() });

    let mut reader = RenderDataReader::new(writer.written());
    assert_eq!(value, reader.read::<RenderOptions>().requires_full_opacity_handling);

    writer.dispose();
}

#[test]
fn nullable_boolean_round_trips_all_three_states_1() {
    nullable_boolean_round_trips_all_three_states(None);
}

#[test]
fn nullable_boolean_round_trips_all_three_states_2() {
    nullable_boolean_round_trips_all_three_states(Some(true));
}

#[test]
fn nullable_boolean_round_trips_all_three_states_3() {
    nullable_boolean_round_trips_all_three_states(Some(false));
}

#[test]
fn length_tracks_written_bytes() {
    let mut writer = RenderDataWriter::new();
    assert_eq!(0, writer.length());
    writer.write::<u8>(1);
    assert_eq!(1, writer.length());
    writer.write(2i32);
    assert_eq!(5, writer.length());
    writer.write(3f64);
    assert_eq!(13, writer.length());

    writer.dispose();
}

#[test]
fn writer_grows_buffer_to_fit_large_payloads() {
    let mut writer = RenderDataWriter::new();
    for i in 0..1000i32 {
        writer.write(i);
    }

    assert_eq!(4000, writer.length());

    let mut reader = RenderDataReader::new(writer.written());
    for i in 0..1000i32 {
        assert_eq!(i, reader.read::<i32>());
    }
    assert!(reader.is_at_end());

    writer.dispose();
}

#[test]
fn payload_auto_prepends_opcode() {
    let mut writer = RenderDataWriter::new();
    writer.write_payload(PushOpacityPayload { opacity: 0.5 });

    let mut reader = RenderDataReader::new(writer.written());
    assert_eq!(RenderDataOpcode::PushOpacity, reader.read::<RenderDataOpcode>());
    let payload = reader.read::<PushOpacityPayload>();
    assert_eq!(0.5, payload.opacity);
    assert!(reader.is_at_end());

    writer.dispose();
}
