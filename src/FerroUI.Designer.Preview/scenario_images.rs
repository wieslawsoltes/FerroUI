//! Compare decoded native output and keep the visual evidence.
use ferroui_designer_tools::scenario::{BaselineComparison, ComparisonStatus, ImageSize};
use png::{BitDepth, ColorType, Decoder, Encoder, Transformations};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};

pub(super) struct ComparedImage {
    pub(super) summary: BaselineComparison,
    pub(super) difference: Option<Vec<u8>>,
}

impl ComparedImage {
    pub(super) fn message(&self, baseline: &Path) -> String {
        let comparison = self.summary;
        match comparison.status {
            ComparisonStatus::Missing => {
                format!("Baseline does not exist at {}", baseline.display())
            }
            ComparisonStatus::Resized => {
                let expected = comparison.expected.expect("resized baseline");
                format!(
                    "Expected {} × {}, rendered {} × {}",
                    expected.width,
                    expected.height,
                    comparison.actual.width,
                    comparison.actual.height
                )
            }
            _ => format!(
                "{} pixels changed, with {} allowed",
                comparison.changed_pixels.unwrap_or_default(),
                comparison.max_changed_pixels
            ),
        }
    }
}

pub(super) fn compare(
    actual: &[u8],
    expected: Option<&[u8]>,
    tolerance: u8,
    maximum: u32,
) -> Result<ComparedImage, String> {
    let current = Pixels::decode(actual)?;
    let mut summary = BaselineComparison {
        status: ComparisonStatus::Missing,
        actual: current.size(),
        expected: None,
        changed_pixels: None,
        max_changed_pixels: maximum,
    };
    let mut difference = None;
    if let Some(expected) = expected {
        let reference = Pixels::decode(expected)?;
        summary.expected = Some(reference.size());
        if current.size() != reference.size() {
            summary.status = ComparisonStatus::Resized;
        } else if actual == expected {
            summary.status = ComparisonStatus::Match;
            summary.changed_pixels = Some(0);
        } else {
            let (changed, diff) = current.diff(&reference, tolerance);
            summary.changed_pixels = Some(changed);
            summary.status = if changed > maximum {
                ComparisonStatus::Changed
            } else {
                ComparisonStatus::Match
            };
            if changed > 0 {
                difference = Some(diff.encode()?);
            }
        }
    }
    Ok(ComparedImage {
        summary,
        difference,
    })
}

pub(super) fn record(
    directory: &Path,
    step: usize,
    kind: &str,
    bytes: &[u8],
) -> Result<PathBuf, String> {
    fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let path = directory.join(format!("{}-{step:03}-{kind}.png", std::process::id()));
    fs::write(&path, bytes).map_err(|error| error.to_string())?;
    Ok(path)
}

struct Pixels {
    width: u32,
    height: u32,
    rgba: Vec<u8>,
}

impl Pixels {
    fn size(&self) -> ImageSize {
        ImageSize {
            width: self.width,
            height: self.height,
        }
    }

    fn decode(bytes: &[u8]) -> Result<Self, String> {
        let mut decoder = Decoder::new(Cursor::new(bytes));
        decoder.set_transformations(Transformations::EXPAND | Transformations::STRIP_16);
        let mut reader = decoder.read_info().map_err(|error| error.to_string())?;
        let info = reader.info();
        if info.width > 1536 || info.height > 1536 {
            return Err("Scenario images must fit a 1536 × 1536 native preview".into());
        }
        let mut buffer = vec![0; reader.output_buffer_size().ok_or("Invalid PNG size")?];
        let info = reader
            .next_frame(&mut buffer)
            .map_err(|error| error.to_string())?;
        let buffer = &buffer[..info.buffer_size()];
        let rgba = match info.color_type {
            ColorType::Rgba => buffer.to_vec(),
            ColorType::Rgb => buffer
                .chunks_exact(3)
                .flat_map(|pixel| [pixel[0], pixel[1], pixel[2], 255])
                .collect(),
            ColorType::GrayscaleAlpha => buffer
                .chunks_exact(2)
                .flat_map(|pixel| [pixel[0], pixel[0], pixel[0], pixel[1]])
                .collect(),
            ColorType::Grayscale => buffer
                .iter()
                .flat_map(|&gray| [gray, gray, gray, 255])
                .collect(),
            ColorType::Indexed => return Err("The PNG palette could not be expanded".into()),
        };
        Ok(Self {
            width: info.width,
            height: info.height,
            rgba,
        })
    }

    fn diff(&self, expected: &Self, tolerance: u8) -> (u32, Self) {
        let mut changed = 0;
        let rgba = self
            .rgba
            .chunks_exact(4)
            .zip(expected.rgba.chunks_exact(4))
            .flat_map(|(pixel, baseline)| {
                if pixel
                    .iter()
                    .zip(baseline)
                    .any(|(actual, expected)| actual.abs_diff(*expected) > tolerance)
                {
                    changed += 1;
                    [255, 48, 94, 255]
                } else {
                    let gray =
                        (u16::from(pixel[0]) + u16::from(pixel[1]) + u16::from(pixel[2])) / 6 + 112;
                    [gray as u8, gray as u8, gray as u8, 255]
                }
            })
            .collect();
        (
            changed,
            Self {
                width: self.width,
                height: self.height,
                rgba,
            },
        )
    }

    fn encode(&self) -> Result<Vec<u8>, String> {
        let mut bytes = Vec::new();
        let mut encoder = Encoder::new(&mut bytes, self.width, self.height);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);
        encoder
            .write_header()
            .and_then(|mut writer| writer.write_image_data(&self.rgba))
            .map_err(|error| error.to_string())?;
        Ok(bytes)
    }
}
