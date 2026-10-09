use crate::{bson_class, bson_enum, ferro_remote_message_guid};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum PixelFormat {
    #[default]
    Rgb565 = 0,
    Rgba8888 = 1,
    Bgra8888 = 2,
}

#[allow(non_upper_case_globals)]
impl PixelFormat {
    pub const MaxValue: PixelFormat = PixelFormat::Bgra8888;

    /// The member with the given numeric value, if one is defined.
    pub fn from_value(value: i32) -> Option<PixelFormat> {
        match value {
            0 => Some(PixelFormat::Rgb565),
            1 => Some(PixelFormat::Rgba8888),
            2 => Some(PixelFormat::Bgra8888),
            _ => None,
        }
    }
}

bson_enum!(PixelFormat);

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeasureViewportMessage {
    pub width: f64,
    pub height: f64,
}

ferro_remote_message_guid!(MeasureViewportMessage, "6E3C5310-E2B1-4C3D-8688-01183AA48C5B");
bson_class!(MeasureViewportMessage as "MeasureViewportMessage" {
    "Width" => width: f64,
    "Height" => height: f64,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClientViewportAllocatedMessage {
    pub width: f64,
    pub height: f64,
    pub dpi_x: f64,
    pub dpi_y: f64,
}

ferro_remote_message_guid!(ClientViewportAllocatedMessage, "BD7A8DE6-3DB8-4A13-8583-D6D4AB189A31");
bson_class!(ClientViewportAllocatedMessage as "ClientViewportAllocatedMessage" {
    "Width" => width: f64,
    "Height" => height: f64,
    "DpiX" => dpi_x: f64,
    "DpiY" => dpi_y: f64,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RequestViewportResizeMessage {
    pub width: f64,
    pub height: f64,
}

ferro_remote_message_guid!(RequestViewportResizeMessage, "9B47B3D8-61DF-4C38-ACD4-8C1BB72554AC");
bson_class!(RequestViewportResizeMessage as "RequestViewportResizeMessage" {
    "Width" => width: f64,
    "Height" => height: f64,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClientSupportedPixelFormatsMessage {
    pub formats: Option<Vec<PixelFormat>>,
}

ferro_remote_message_guid!(ClientSupportedPixelFormatsMessage, "63481025-7016-43FE-BADC-F2FD0F88609E");
bson_class!(ClientSupportedPixelFormatsMessage as "ClientSupportedPixelFormatsMessage" {
    "Formats" => formats: Option<Vec<PixelFormat>>,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClientRenderInfoMessage {
    pub dpi_x: f64,
    pub dpi_y: f64,
}

ferro_remote_message_guid!(ClientRenderInfoMessage, "7A3c25d3-3652-438D-8EF1-86E942CC96C0");
bson_class!(ClientRenderInfoMessage as "ClientRenderInfoMessage" {
    "DpiX" => dpi_x: f64,
    "DpiY" => dpi_y: f64,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameReceivedMessage {
    pub sequence_id: i64,
}

ferro_remote_message_guid!(FrameReceivedMessage, "68014F8A-289D-4851-8D34-5367EDA7F827");
bson_class!(FrameReceivedMessage as "FrameReceivedMessage" {
    "SequenceId" => sequence_id: i64,
});

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameMessage {
    pub sequence_id: i64,
    pub format: PixelFormat,
    pub data: Option<Vec<u8>>,
    pub width: i32,
    pub height: i32,
    pub stride: i32,
    pub dpi_x: f64,
    pub dpi_y: f64,
}

ferro_remote_message_guid!(FrameMessage, "F58313EE-FE69-4536-819D-F52EDF201A0E");
bson_class!(FrameMessage as "FrameMessage" {
    "SequenceId" => sequence_id: i64,
    "Format" => format: PixelFormat,
    "Data" => data: Option<Vec<u8>>,
    "Width" => width: i32,
    "Height" => height: i32,
    "Stride" => stride: i32,
    "DpiX" => dpi_x: f64,
    "DpiY" => dpi_y: f64,
});
