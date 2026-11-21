//! Compare the decoded native output. Keep the actual and the visual diff for review.
use png::{BitDepth, ColorType, Decoder, Encoder, Transformations};
use std::fs;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

pub(crate) struct ImageResult {
    pub(crate) message: String,
    pub(crate) failure: bool,
    pub(crate) artifacts: Vec<PathBuf>,
}

pub(crate) fn check(
    actual: &[u8],
    baseline: &Path,
    artifacts: &Path,
    step: usize,
    tolerance: u8,
    maximum: u32,
) -> Result<ImageResult, String> {
    let reference = match read_png(baseline) {
        Ok(reference) => reference,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let actual = record(artifacts, step, "actual", actual)?;
            return Ok(ImageResult {
                message: format!("Baseline does not exist at {}", baseline.display()),
                failure: true,
                artifacts: vec![actual],
            });
        }
        Err(error) => return Err(format!("{}: {error}", baseline.display())),
    };
    let expected = Pixels::decode(&reference)?;
    let current = Pixels::decode(actual)?;
    if (expected.width, expected.height) != (current.width, current.height) {
        return Ok(ImageResult {
            message: format!(
                "Expected {} × {}, rendered {} × {}",
                expected.width, expected.height, current.width, current.height
            ),
            failure: true,
            artifacts: vec![
                baseline.to_owned(),
                record(artifacts, step, "actual", actual)?,
            ],
        });
    }
    let (changed, diff) = current.diff(&expected, tolerance);
    let mut evidence = Vec::new();
    if changed > maximum {
        evidence.push(baseline.to_owned());
        evidence.push(record(artifacts, step, "actual", actual)?);
        evidence.push(record(artifacts, step, "diff", &diff.encode()?)?);
    }
    Ok(ImageResult {
        message: format!("{changed} pixels changed, with {maximum} allowed"),
        failure: changed > maximum,
        artifacts: evidence,
    })
}

fn read_png(path: &Path) -> std::io::Result<Vec<u8>> {
    use std::io::{Error, Read};
    let file = fs::File::open(path)?;
    let maximum = 16 * 1024 * 1024;
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > maximum {
        return Err(Error::other("Baseline PNG exceeds 16 MiB"));
    }
    Ok(bytes)
}

pub(crate) fn write_baseline(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let filename = path.file_name().ok_or(std::io::ErrorKind::InvalidInput)?;
    let parent = path.parent().ok_or(std::io::ErrorKind::InvalidInput)?;
    fs::create_dir_all(parent)?;
    let mut temporary_name = filename.to_os_string();
    temporary_name.push(format!(".{}.tmp", std::process::id()));
    let temporary = parent.join(temporary_name);
    let mut file = fs::File::options()
        .write(true)
        .create_new(true)
        .open(&temporary)?;
    let result = file.write_all(bytes);
    drop(file);
    let result = result.and_then(|()| fs::rename(&temporary, path));
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

fn record(directory: &Path, step: usize, kind: &str, bytes: &[u8]) -> Result<PathBuf, String> {
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
