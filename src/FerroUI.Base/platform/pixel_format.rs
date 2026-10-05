use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum PixelFormatEnum {
    Rgb565,
    Rgba8888,
    Bgra8888,
    BlackWhite,
    Gray2,
    Gray4,
    Gray8,
    Gray16,
    Gray32Float,
    Rgba64,
    Rgb24,
    Rgb32,
    Bgr24,
    Bgr32,
    Bgr555,
    Bgr565,
}

/// The format of the pixels of a bitmap or framebuffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PixelFormat {
    pub(crate) format: PixelFormatEnum,
}

impl PixelFormat {
    pub const RGB565: PixelFormat = PixelFormats::RGB565;
    pub const RGBA8888: PixelFormat = PixelFormats::RGBA8888;
    pub const RGB32: PixelFormat = PixelFormats::RGB32;
    pub const BGRA8888: PixelFormat = PixelFormats::BGRA8888;

    const fn new(format: PixelFormatEnum) -> Self {
        Self { format }
    }

    /// The number of bits one pixel occupies.
    pub fn bits_per_pixel(&self) -> u32 {
        use PixelFormatEnum::*;
        match self.format {
            BlackWhite => 1,
            Gray2 => 2,
            Gray4 => 4,
            Gray8 => 8,
            Rgb565 | Bgr555 | Bgr565 | Gray16 => 16,
            Bgr24 | Rgb24 => 24,
            Rgba64 => 64,
            Rgba8888 | Bgra8888 | Gray32Float | Rgb32 | Bgr32 => 32,
        }
    }

    /// Whether the format carries an alpha channel.
    pub fn has_alpha(&self) -> bool {
        matches!(self.format, PixelFormatEnum::Rgba8888 | PixelFormatEnum::Bgra8888 | PixelFormatEnum::Rgba64)
    }
}

impl fmt::Display for PixelFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.format, f)
    }
}

/// The well-known pixel formats.
pub struct PixelFormats;

impl PixelFormats {
    pub const RGB565: PixelFormat = PixelFormat::new(PixelFormatEnum::Rgb565);
    pub const RGBA8888: PixelFormat = PixelFormat::new(PixelFormatEnum::Rgba8888);
    pub const RGBA64: PixelFormat = PixelFormat::new(PixelFormatEnum::Rgba64);
    pub const BGRA8888: PixelFormat = PixelFormat::new(PixelFormatEnum::Bgra8888);
    pub const BLACK_WHITE: PixelFormat = PixelFormat::new(PixelFormatEnum::BlackWhite);
    pub const GRAY2: PixelFormat = PixelFormat::new(PixelFormatEnum::Gray2);
    pub const GRAY4: PixelFormat = PixelFormat::new(PixelFormatEnum::Gray4);
    pub const GRAY8: PixelFormat = PixelFormat::new(PixelFormatEnum::Gray8);
    pub const GRAY16: PixelFormat = PixelFormat::new(PixelFormatEnum::Gray16);
    pub const GRAY32_FLOAT: PixelFormat = PixelFormat::new(PixelFormatEnum::Gray32Float);
    pub const RGB24: PixelFormat = PixelFormat::new(PixelFormatEnum::Rgb24);
    pub const RGB32: PixelFormat = PixelFormat::new(PixelFormatEnum::Rgb32);
    pub const BGR24: PixelFormat = PixelFormat::new(PixelFormatEnum::Bgr24);
    pub const BGR32: PixelFormat = PixelFormat::new(PixelFormatEnum::Bgr32);
    pub const BGR555: PixelFormat = PixelFormat::new(PixelFormatEnum::Bgr555);
    pub const BGR565: PixelFormat = PixelFormat::new(PixelFormatEnum::Bgr565);
}
