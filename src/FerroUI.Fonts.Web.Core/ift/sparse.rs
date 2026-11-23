//! The sparse bit set of a patch map entry (section 5.3.3 of the standard).

use crate::FontDecodeError;
use std::collections::VecDeque;

const MAX_CODE_POINT: u64 = 0x10_FFFF;
/// The most nodes waiting at once: enough for a set spread over all of Unicode, and a limit on
/// the memory a hostile set can ask for.
const MAX_QUEUE: usize = 1 << 19;

/// Decodes a set, returning sorted, merged, inclusive ranges and the bytes it took. `nodes` is the
/// budget of nodes left for the whole table.
pub(super) fn decode_sparse_bit_set(
    data: &[u8],
    bias: u32,
    nodes: &mut usize,
) -> Result<(Vec<(u32, u32)>, usize), FontDecodeError> {
    const MALFORMED: FontDecodeError = FontDecodeError::Malformed("sparse bit set");
    let header = *data.first().ok_or(FontDecodeError::Truncated)?;
    if header & 0x80 != 0 {
        return Err(MALFORMED);
    }
    let (branch, max_height): (u64, u32) = match header & 3 {
        0 => (2, 31),
        1 => (4, 16),
        2 => (8, 11),
        _ => (32, 7),
    };
    let height = u32::from(header >> 2) & 0x1F;
    if height > max_height {
        return Err(MALFORMED);
    }
    if height == 0 {
        return Ok((Vec::new(), 1));
    }
    let bits = &data[1..];
    let total_bits = bits.len().saturating_mul(8);
    let mut position = 0usize;
    let mut ranges: Vec<(u32, u32)> = Vec::new();
    let mut queue: VecDeque<(u64, u32)> = VecDeque::new();
    queue.push_back((0, 1));
    let bias = u64::from(bias);
    while let Some((start, depth)) = queue.pop_front() {
        *nodes = nodes.checked_sub(1).ok_or(FontDecodeError::LimitExceeded("patch map nodes"))?;
        if position + branch as usize > total_bits {
            return Err(FontDecodeError::Truncated);
        }
        let mut value = 0u32;
        for index in 0..branch as usize {
            let at = position + index;
            value |= u32::from((bits[at / 8] >> (at % 8)) & 1) << index;
        }
        position += branch as usize;
        if value == 0 {
            let length = branch.pow(height - depth + 1);
            add_range(&mut ranges, start + bias, start + bias + length - 1);
            continue;
        }
        for index in 0..branch {
            if value & (1 << index) == 0 {
                continue;
            }
            if depth == height {
                add_range(&mut ranges, start + bias + index, start + bias + index);
            } else {
                if queue.len() >= MAX_QUEUE {
                    return Err(FontDecodeError::LimitExceeded("sparse bit set"));
                }
                queue.push_back((start + index * branch.pow(height - depth), depth + 1));
            }
        }
    }
    ranges.sort_unstable();
    let mut merged: Vec<(u32, u32)> = Vec::with_capacity(ranges.len().min(1024));
    for (first, last) in ranges {
        match merged.last_mut() {
            Some(previous) if first <= previous.1.saturating_add(1) => previous.1 = previous.1.max(last),
            _ => merged.push((first, last)),
        }
    }
    Ok((merged, 1 + position.div_ceil(8)))
}

/// Adds a range, leaving out what is above the last code point.
fn add_range(ranges: &mut Vec<(u32, u32)>, first: u64, last: u64) {
    if first > MAX_CODE_POINT {
        return;
    }
    let last = last.min(MAX_CODE_POINT) as u32;
    let first = first as u32;
    // The tree is walked by level, so runs of neighbours arrive together most of the time.
    match ranges.last_mut() {
        Some(previous) if previous.1.checked_add(1) == Some(first) => previous.1 = last,
        _ => ranges.push((first, last)),
    }
}
