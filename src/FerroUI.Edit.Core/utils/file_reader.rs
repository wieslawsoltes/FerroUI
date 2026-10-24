use super::{FileReaderOptions, FileTextReader, TextEncoding};
use std::fs::File;
use std::io::{self, Read, Seek};
use std::path::Path;

/// Opens encoded text using AvaloniaEdit's bounded UTF-8 heuristic and Unicode
/// byte-order marks. Generic `Read` support accepts FerroUI storage streams
/// without adding a native-framework dependency to the document core.
pub struct FileReader;

impl FileReader {
    pub const DETECTION_LIMIT: usize = 500_000;

    /// Treats the stream's current position as the start of the input. Detection
    /// retains and replays at most `DETECTION_LIMIT` bytes, so seeking is not
    /// required. This is intended for finite file-like streams.
    pub fn open_stream<R: Read>(
        stream: R,
        default_encoding: TextEncoding,
    ) -> io::Result<FileTextReader<R>> {
        Self::open_stream_with_options(
            stream,
            FileReaderOptions {
                default_encoding,
                ..FileReaderOptions::default()
            },
        )
    }

    pub fn open_stream_with_options<R: Read>(
        mut stream: R,
        options: FileReaderOptions,
    ) -> io::Result<FileTextReader<R>> {
        let (prefix, encoding, bom_length) = detect(&mut stream, options.default_encoding)?;
        Ok(FileTextReader::with_prefix(
            stream,
            encoding,
            options.error_policy,
            prefix,
            bom_length,
        ))
    }

    /// Preserves the reference's explicit seekable-stream position precondition.
    /// Use `open_stream` for `IStorageFile::open_read_async`'s `Box<dyn Read>`.
    pub fn open_seekable_stream<R: Read + Seek>(
        mut stream: R,
        default_encoding: TextEncoding,
    ) -> io::Result<FileTextReader<R>> {
        if stream.stream_position()? != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "The stream is not positioned at its beginning",
            ));
        }
        Self::open_stream(stream, default_encoding)
    }

    pub fn open_file(
        path: impl AsRef<Path>,
        default_encoding: TextEncoding,
    ) -> io::Result<FileTextReader<File>> {
        Self::open_stream(File::open(path)?, default_encoding)
    }

    pub fn open_file_with_options(
        path: impl AsRef<Path>,
        options: FileReaderOptions,
    ) -> io::Result<FileTextReader<File>> {
        Self::open_stream_with_options(File::open(path)?, options)
    }

    pub fn read_stream_content<R: Read>(
        stream: R,
        default_encoding: TextEncoding,
    ) -> io::Result<String> {
        Self::open_stream(stream, default_encoding)?.read_to_end()
    }

    pub fn read_file_content(
        path: impl AsRef<Path>,
        default_encoding: TextEncoding,
    ) -> io::Result<String> {
        Self::open_file(path, default_encoding)?.read_to_end()
    }
}

#[derive(Default)]
struct Utf8Probe {
    saw_non_ascii: bool,
    remaining: u8,
}

impl Utf8Probe {
    // Deliberately retains the reference's structural heuristic. Scalar-value
    // validity and incomplete sequences are checked by the actual decoder.
    fn accept(&mut self, byte: u8) -> bool {
        match byte {
            0..=0x7f => self.remaining == 0,
            0x80..=0xbf if self.remaining != 0 => {
                self.remaining -= 1;
                true
            }
            0xc2..=0xf4 if self.remaining == 0 => {
                self.saw_non_ascii = true;
                self.remaining = if byte < 0xe0 {
                    1
                } else if byte < 0xf0 {
                    2
                } else {
                    3
                };
                true
            }
            _ => false,
        }
    }
}

fn read_retry(reader: &mut impl Read, buffer: &mut [u8]) -> io::Result<usize> {
    loop {
        match reader.read(buffer) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            result => return result,
        }
    }
}

fn detect(
    stream: &mut impl Read,
    default_encoding: TextEncoding,
) -> io::Result<(Vec<u8>, TextEncoding, usize)> {
    let mut first = [0; 4];
    let mut length = 0;
    while length != first.len() {
        let read = read_retry(stream, &mut first[length..])?;
        if read == 0 {
            break;
        }
        length += read;
    }
    let mut prefix = first[..length].to_vec();
    // UTF-32 LE must precede UTF-16 LE because their first two bytes agree.
    for encoding in [
        TextEncoding::Utf32Le,
        TextEncoding::Utf32Be,
        TextEncoding::Utf8,
        TextEncoding::Utf16Le,
        TextEncoding::Utf16Be,
    ] {
        let bom = encoding.byte_order_mark();
        if prefix.starts_with(bom) {
            return Ok((prefix, encoding, bom.len()));
        }
    }
    if length < 2 {
        return Ok((prefix, default_encoding, 0));
    }
    if matches!(
        &first[..2],
        [0, 0] | [0xff, 0xfe] | [0xfe, 0xff] | [0xef, 0xbb]
    ) {
        // StreamReader's BOM path defaults to UTF-8 even when the apparent BOM
        // prefix proves incomplete or is followed by other bytes.
        return Ok((prefix, TextEncoding::Utf8, 0));
    }
    let fallback = if default_encoding.is_unicode() {
        TextEncoding::Utf8
    } else {
        default_encoding
    };
    let mut probe = Utf8Probe::default();
    if !prefix.iter().all(|&byte| probe.accept(byte)) {
        return Ok((prefix, fallback, 0));
    }
    let mut buffer = [0; 4096];
    while prefix.len() < FileReader::DETECTION_LIMIT {
        let capacity = buffer.len().min(FileReader::DETECTION_LIMIT - prefix.len());
        let length = read_retry(stream, &mut buffer[..capacity])?;
        if length == 0 {
            break;
        }
        prefix.extend_from_slice(&buffer[..length]);
        if !buffer[..length].iter().all(|&byte| probe.accept(byte)) {
            return Ok((prefix, fallback, 0));
        }
    }
    let encoding = if !probe.saw_non_ascii && default_encoding.is_ascii_compatible() {
        default_encoding
    } else {
        TextEncoding::Utf8
    };
    Ok((prefix, encoding, 0))
}

#[cfg(test)]
#[path = "file_reader_tests.rs"]
mod tests;
