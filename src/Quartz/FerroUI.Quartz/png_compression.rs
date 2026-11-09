//! ImageIO supplies PNG encoding and metadata. Recompress its lossless IDAT stream
//! through macOS's system zlib to honor Ferro's explicit compression-level contract.
use ferroui_base::media::imaging::CompressionLevel;
use std::{
    ffi::{c_int, c_uint, c_ulong},
    io,
};
#[link(name = "z")]
unsafe extern "C" {
    fn uncompress(
        dest: *mut u8,
        length: *mut c_ulong,
        source: *const u8,
        source_length: c_ulong,
    ) -> c_int;
    fn compressBound(length: c_ulong) -> c_ulong;
    fn compress2(
        dest: *mut u8,
        length: *mut c_ulong,
        source: *const u8,
        source_length: c_ulong,
        level: c_int,
    ) -> c_int;
    fn crc32(crc: c_ulong, bytes: *const u8, length: c_uint) -> c_ulong;
}
fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "Invalid ImageIO PNG stream")
}

pub fn recompress(png: &[u8], level: CompressionLevel) -> io::Result<Vec<u8>> {
    if !png.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err(invalid());
    }
    let mut chunks = Vec::new();
    let mut compressed = Vec::new();
    let mut position = 8;
    let mut dimensions = None;
    let mut ended = false;
    while position < png.len() {
        let header = png.get(position..position + 8).ok_or_else(invalid)?;
        let length = u32::from_be_bytes(header[..4].try_into().unwrap()) as usize;
        let end = position
            .checked_add(12)
            .and_then(|n| n.checked_add(length))
            .ok_or_else(invalid)?;
        let chunk = png.get(position..end).ok_or_else(invalid)?;
        let data = &chunk[8..8 + length];
        let tag = &chunk[4..8];
        if tag == b"IHDR" {
            if data.len() != 13 {
                return Err(invalid());
            }
            dimensions = Some((
                u32::from_be_bytes(data[..4].try_into().unwrap()) as usize,
                u32::from_be_bytes(data[4..8].try_into().unwrap()) as usize,
            ));
        }
        if tag == b"IDAT" {
            compressed.extend_from_slice(data);
        }
        if tag == b"IEND" {
            ended = true;
        }
        chunks.push((tag, chunk));
        position = end;
    }
    let (width, height) = dimensions.ok_or_else(invalid)?;
    if width == 0 || height == 0 || compressed.is_empty() || !ended {
        return Err(invalid());
    }
    // A generous bound for RGBA16 plus all scanline/filter bytes, including
    // Adam7 passes. This is our own ImageIO output, not an arbitrary PNG decoder.
    let capacity = width
        .checked_mul(height)
        .and_then(|n| n.checked_mul(8))
        .and_then(|n| height.checked_mul(64).and_then(|rows| n.checked_add(rows)))
        .and_then(|n| n.checked_add(1024))
        .ok_or_else(invalid)?;
    let mut raw = vec![0; capacity];
    let mut raw_length = capacity as c_ulong;
    // SAFETY: all zlib lengths match live, owned slice allocations. The output
    // lengths are initialized to their capacities; no pointer escapes the call.
    if unsafe {
        uncompress(
            raw.as_mut_ptr(),
            &mut raw_length,
            compressed.as_ptr(),
            compressed.len() as c_ulong,
        )
    } != 0
    {
        return Err(io::Error::other("System zlib could not unpack ImageIO PNG"));
    }
    raw.truncate(raw_length as usize);
    let zlevel = match level {
        CompressionLevel::Optimal => 6,
        CompressionLevel::Fastest => 1,
        CompressionLevel::NoCompression => 0,
        CompressionLevel::SmallestSize => 9,
    };
    let bound = unsafe { compressBound(raw.len() as c_ulong) };
    let mut result = vec![0; bound as usize];
    let mut length = bound;
    if unsafe {
        compress2(
            result.as_mut_ptr(),
            &mut length,
            raw.as_ptr(),
            raw.len() as c_ulong,
            zlevel,
        )
    } != 0
    {
        return Err(io::Error::other("System zlib could not compress PNG"));
    }
    result.truncate(length as usize);
    let length = u32::try_from(result.len()).map_err(|_| invalid())?;
    let mut output = Vec::with_capacity(png.len());
    output.extend_from_slice(&png[..8]);
    let mut written = false;
    for (tag, chunk) in chunks {
        if tag == b"IDAT" {
            if written {
                continue;
            }
            output.extend_from_slice(&length.to_be_bytes());
            output.extend_from_slice(b"IDAT");
            output.extend_from_slice(&result);
            let crc = unsafe { crc32(crc32(0, b"IDAT".as_ptr(), 4), result.as_ptr(), length) };
            output.extend_from_slice(&(crc as u32).to_be_bytes());
            written = true;
        } else {
            output.extend_from_slice(chunk);
        }
    }
    Ok(output)
}
