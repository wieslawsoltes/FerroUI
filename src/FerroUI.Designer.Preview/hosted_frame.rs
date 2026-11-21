use crate::protocol::Image;
use base64::{Engine, engine::general_purpose::STANDARD};
use ferroui_remote_protocol::viewport::{FrameMessage, PixelFormat};
use serde::Deserialize;

#[derive(Clone, Copy, Deserialize)]
pub(crate) struct FrameIdentity {
    pub(crate) revision: i64,
    pub(crate) sequence: i64,
}

pub(crate) fn png(frame: &FrameMessage) -> Result<Image, String> {
    let (width, height, stride) = (frame.width, frame.height, frame.stride);
    if !(1..=4096).contains(&width)
        || !(1..=4096).contains(&height)
        || stride < width * 4
        || stride > 4096 * 4
        || width * height > 4_194_304
        || frame.format != PixelFormat::Rgba8888
    {
        return Err("The hosted frame has unsupported dimensions or pixel format".into());
    }
    let data = frame
        .data
        .as_deref()
        .filter(|data| data.len() == stride as usize * height as usize)
        .ok_or("The application sent an incomplete preview frame")?;
    let pixels: Vec<u8> = data
        .chunks_exact(stride as usize)
        .flat_map(|row| {
            row[..width as usize * 4].chunks_exact(4).flat_map(|pixel| {
                let alpha = u16::from(pixel[3]);
                let color = |c: u8| {
                    if alpha == 0 {
                        0
                    } else {
                        ((u16::from(c) * 255 + alpha / 2) / alpha).min(255) as u8
                    }
                };
                [color(pixel[0]), color(pixel[1]), color(pixel[2]), pixel[3]]
            })
        })
        .collect();
    let (width, height) = (width as u32, height as u32);
    let mut bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut bytes, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|error| error.to_string())?;
    writer
        .write_image_data(&pixels)
        .map_err(|error| error.to_string())?;
    writer.finish().map_err(|error| error.to_string())?;
    Ok(Image {
        png: STANDARD.encode(bytes),
        width,
        height,
    })
}
