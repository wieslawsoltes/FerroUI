//! The animated picture of the patchwork's chat: the GIF decoder of the benchmark's shared C#
//! files, bit for bit. The .NET app decodes the picture's frames with it, as neither its runtime
//! nor its UI framework decodes the frames of a GIF; this port does the same.
//!
//! A malformed file ends the decoding with an error of the kind `InvalidData` where the C# code
//! throws `InvalidDataException`; reads past the end of a truncated file panic, where the C# code
//! throws `IndexOutOfRangeException`.

use std::io;

/// A decoded GIF: its width and height in pixels and its frames.
pub struct Gif {
    pub width: usize,
    pub height: usize,
    /// The frames as RGBA pixels, each the whole picture after the frame was drawn over the one
    /// before.
    pub frames: Vec<Vec<u8>>,
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message.to_string())
}

/// The frames of a GIF as RGBA pixels, each the whole picture after the frame was drawn over the
/// one before.
pub fn decode(gif: &[u8]) -> io::Result<Gif> {
    if gif.len() < 13 || gif[0] != b'G' || gif[1] != b'I' || gif[2] != b'F' {
        return Err(invalid("not a GIF"));
    }
    let width = usize::from(gif[6]) | usize::from(gif[7]) << 8;
    let height = usize::from(gif[8]) | usize::from(gif[9]) << 8;
    let mut at = 13;
    let global = if gif[10] & 0x80 != 0 { Some(read_table(gif, &mut at, 2 << (gif[10] & 7))) } else { None };
    let mut canvas = vec![0u8; width * height * 4];
    let mut frames = Vec::new();
    let mut transparent: i32 = -1;
    while at < gif.len() && gif[at] != 0x3B {
        let block = gif[at];
        at += 1;
        if block == 0x21 {
            let label = gif[at];
            at += 1;
            if label == 0xF9 {
                transparent = if gif[at + 1] & 1 != 0 { i32::from(gif[at + 4]) } else { -1 };
            }
            skip_blocks(gif, &mut at);
        } else if block == 0x2C {
            let left = usize::from(gif[at]) | usize::from(gif[at + 1]) << 8;
            let top = usize::from(gif[at + 2]) | usize::from(gif[at + 3]) << 8;
            let frame_width = usize::from(gif[at + 4]) | usize::from(gif[at + 5]) << 8;
            let frame_height = usize::from(gif[at + 6]) | usize::from(gif[at + 7]) << 8;
            let flags = gif[at + 8];
            at += 9;
            let table = if flags & 0x80 != 0 {
                read_table(gif, &mut at, 2 << (flags & 7))
            } else {
                global.ok_or_else(|| invalid("no color table"))?
            };
            let indices = lzw(gif, &mut at, frame_width * frame_height);
            for (pixel, &index) in indices.iter().enumerate() {
                if i32::from(index) == transparent {
                    continue;
                }
                let to = ((top + pixel / frame_width) * width + left + pixel % frame_width) * 4;
                let index = usize::from(index);
                canvas[to] = table[index * 3];
                canvas[to + 1] = table[index * 3 + 1];
                canvas[to + 2] = table[index * 3 + 2];
                canvas[to + 3] = 0xFF;
            }
            frames.push(canvas.clone());
        } else {
            return Err(invalid("unknown GIF block"));
        }
    }
    Ok(Gif { width, height, frames })
}

fn read_table<'a>(gif: &'a [u8], at: &mut usize, colors: usize) -> &'a [u8] {
    let table = &gif[*at..*at + colors * 3];
    *at += colors * 3;
    table
}

fn skip_blocks(gif: &[u8], at: &mut usize) {
    while gif[*at] != 0 {
        *at += usize::from(gif[*at]) + 1;
    }
    *at += 1;
}

/// The color indices of one image's data, `count` of them.
fn lzw(gif: &[u8], at: &mut usize, count: usize) -> Vec<u8> {
    let minimum = u32::from(gif[*at]);
    *at += 1;
    let mut data = Vec::new();
    while gif[*at] != 0 {
        let length = usize::from(gif[*at]);
        data.extend_from_slice(&gif[*at + 1..*at + 1 + length]);
        *at += length + 1;
    }
    *at += 1;
    let clear: i32 = 1 << minimum;
    let end = clear + 1;
    let mut prefix = vec![0i32; 4096];
    let mut suffix = vec![0u8; 4096];
    let mut stack = vec![0u8; 4097];
    let mut output = vec![0u8; count];
    let mut written = 0;
    let mut size = minimum + 1;
    let mut next = end + 1;
    let mut previous: i32 = -1;
    let mut first: u8 = 0;
    let mut bits: u32 = 0;
    let mut have: u32 = 0;
    let mut cursor = 0;
    while written < count {
        while have < size {
            if cursor >= data.len() {
                return output;
            }
            bits |= u32::from(data[cursor]) << have;
            cursor += 1;
            have += 8;
        }
        let code = (bits & ((1 << size) - 1)) as i32;
        bits >>= size;
        have -= size;
        if code == clear {
            size = minimum + 1;
            next = end + 1;
            previous = -1;
            continue;
        }
        if code == end {
            break;
        }
        let mut top = 0;
        let mut current = code;
        if previous < 0 {
            output[written] = code as u8;
            written += 1;
            previous = code;
            first = code as u8;
            continue;
        }
        if code >= next {
            stack[top] = first;
            top += 1;
            current = previous;
        }
        while current >= clear {
            stack[top] = suffix[current as usize];
            top += 1;
            current = prefix[current as usize];
        }
        first = current as u8;
        stack[top] = first;
        top += 1;
        if next < 4096 {
            prefix[next as usize] = previous;
            suffix[next as usize] = first;
            next += 1;
            if next == 1 << size && size < 12 {
                size += 1;
            }
        }
        previous = code;
        while top > 0 && written < count {
            top -= 1;
            output[written] = stack[top];
            written += 1;
        }
    }
    output
}
