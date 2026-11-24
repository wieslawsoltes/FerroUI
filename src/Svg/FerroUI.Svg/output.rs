use crate::SvgError;
use flate2::{Compression, GzBuilder};
use std::io::Write;
use std::path::Path;

/// An actionable export diagnostic. A layer ID is present for layer-specific fallbacks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SvgDiagnostic {
    pub layer_id: Option<String>,
    pub message: String,
}

/// Measured output structure; raster fallback is never hidden from callers.
#[derive(Clone, Debug, Default)]
pub struct SvgExportReport {
    pub layers: usize,
    pub vector_layers: usize,
    pub raster_layers: usize,
    pub draw_calls: u64,
    pub embedded_images: u64,
    /// Vector layers reused without replaying the native SVG writer, including
    /// layers whose prepared fragment was reused as well.
    pub vector_cache_hits: usize,
    /// Vector layers whose namespaced, inspected fragment was reused. These
    /// hits are a subset of `vector_cache_hits`; final document processing still runs.
    pub prepared_vector_cache_hits: usize,
    pub raster_cache_hits: usize,
    pub unoptimized_bytes: usize,
    pub output_bytes: usize,
    pub definitions_deduplicated: usize,
    pub diagnostics: Vec<SvgDiagnostic>,
}

/// A completed, validated SVG and the decisions used to produce it.
pub struct SvgOutput {
    pub svg: String,
    pub report: SvgExportReport,
}

impl SvgOutput {
    pub fn as_str(&self) -> &str {
        &self.svg
    }
    pub fn as_bytes(&self) -> &[u8] {
        self.svg.as_bytes()
    }
    pub fn write_to(&self, writer: &mut impl Write) -> Result<(), SvgError> {
        writer.write_all(self.as_bytes())?;
        Ok(())
    }
    pub fn save(&self, path: impl AsRef<Path>) -> Result<(), SvgError> {
        let mut writer = std::io::BufWriter::new(std::fs::File::create(path)?);
        self.write_to(&mut writer)?;
        writer.flush()?;
        Ok(())
    }
    /// A reproducible SVGZ: fixed mtime/OS header and no filename or comment.
    pub fn write_svgz(&self, writer: &mut impl Write, level: u32) -> Result<(), SvgError> {
        if level > 9 {
            return Err(SvgError::InvalidOptions(
                "gzip level must be in 0..=9".into(),
            ));
        }
        let mut encoder = GzBuilder::new()
            .mtime(0)
            .operating_system(255)
            .write(writer, Compression::new(level));
        encoder.write_all(self.as_bytes())?;
        encoder.finish()?;
        Ok(())
    }
    pub fn save_svgz(&self, path: impl AsRef<Path>) -> Result<(), SvgError> {
        let mut writer = std::io::BufWriter::new(std::fs::File::create(path)?);
        self.write_svgz(&mut writer, 6)?;
        writer.flush()?;
        Ok(())
    }
}
