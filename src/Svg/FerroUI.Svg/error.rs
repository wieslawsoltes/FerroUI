use std::fmt;

/// An export failure. A failed strict-vector export never returns a partial SVG.
#[derive(Debug)]
pub enum SvgError {
    InvalidOptions(String),
    InvalidLayer(String),
    Recording {
        layer: String,
        reasons: Vec<String>,
    },
    VectorFidelity {
        layer: String,
        reasons: Vec<String>,
    },
    RasterLimit {
        width: u64,
        height: u64,
        max_pixels: u64,
    },
    RasterAllocation,
    ImageEncoding,
    Xml(String),
    Io(std::io::Error),
}

impl fmt::Display for SvgError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOptions(message) => write!(f, "Invalid SVG export options: {message}"),
            Self::InvalidLayer(message) => write!(f, "Invalid SVG layer: {message}"),
            Self::Recording { layer, reasons } => write!(f, "Layer '{layer}' could not be recorded: {}", reasons.join("; ")),
            Self::VectorFidelity { layer, reasons } => write!(f, "Layer '{layer}' requires raster fallback: {}", reasons.join("; ")),
            Self::RasterLimit { width, height, max_pixels } => write!(f, "Raster fallback {width} × {height} exceeds the export limit ({max_pixels} pixels or the maximum dimension)"),
            Self::RasterAllocation => f.write_str("Skia could not allocate the raster export surface"),
            Self::ImageEncoding => f.write_str("Skia could not encode the raster export as PNG"),
            Self::Xml(message) => write!(f, "Invalid SVG XML: {message}"),
            Self::Io(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for SvgError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(error) => Some(error),
            _ => None,
        }
    }
}

impl From<std::io::Error> for SvgError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
