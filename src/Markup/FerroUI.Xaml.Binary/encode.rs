use super::*;

fn uint(out: &mut Vec<u8>, mut value: u32) {
    while value >= 128 {
        out.push(value as u8 | 128);
        value >>= 7;
    }
    out.push(value as u8);
}

fn zigzag(base: u32, next: u32) -> u32 {
    let delta = (next as i32) - (base as i32);
    ((delta << 1) ^ (delta >> 31)) as u32
}

fn location(row: &Resource<'_>) -> [u32; 4] {
    [row.line, row.column, zigzag(row.line, row.value_line), zigzag(row.column, row.value_column)]
}

// Select lanes only when every row satisfies the prediction. Singletons and
// irregular blocks keep their v1 encoding, without an extra mode byte.
fn location_flags(rows: &[Resource<'_>]) -> u8 {
    let Some(first) = rows.first() else { return 0 };
    let base = location(first);
    let mut same = REPEAT;
    let mut steps = true;
    for (i, row) in rows.iter().enumerate().skip(1) {
        let next = location(row);
        for field in 0..4 {
            if next[field] != base[field] {
                same &= !(1 << field);
            }
        }
        steps &= next[0] == base[0] + i as u32;
    }
    if steps {
        same |= STEP_LINE | 1;
    }
    if (rows.len() - 1) * (same & REPEAT).count_ones() as usize > 1 { same } else { 0 }
}

/// Encode a homogeneous run without changing insertion or error order.
pub fn encode(kind: Kind, rows: &[Resource<'_>]) -> Result<Vec<u8>, Error> {
    if rows.len() > MAX_ITEMS {
        return Err(Error("too many FXB records"));
    }
    let mut strings = Vec::with_capacity(rows.len());
    for row in rows {
        strings.push(row.key);
        match (kind, row.value) {
            (Kind::LiteralBrush | Kind::Color, Value::Argb(_))
            | (Kind::Double, Value::Double(_))
            | (Kind::Thickness | Kind::CornerRadius, Value::Quad(_))
            | (Kind::DeferredFactory, Value::Import(_)) => {}
            (Kind::Alias | Kind::DynamicBrush | Kind::StaticBrush, Value::Text(value)) => {
                strings.push(value)
            }
            _ => return Err(Error("FXB value does not match the factory kind")),
        }
        if [row.line, row.column, row.value_line, row.value_column]
            .iter()
            .any(|&p| p > i32::MAX as u32)
        {
            return Err(Error("invalid FXB source position"));
        }
    }
    strings.sort_unstable();
    strings.dedup();
    if strings.len() > MAX_ITEMS
        || strings
            .iter()
            .try_fold(0usize, |n, s| n.checked_add(s.len()))
            .is_none_or(|n| n > MAX_BLOCK_BYTES)
    {
        return Err(Error("FXB string table too large"));
    }
    let repeat = location_flags(rows);
    let mut out = MAGIC.to_vec();
    out.push(if repeat == 0 { 1 } else { 2 });
    out.push(kind as u8);
    if repeat != 0 {
        out.push(repeat);
    }
    uint(&mut out, strings.len() as u32);
    uint(&mut out, rows.len() as u32);
    let mut previous = "";
    for &s in &strings {
        let mut prefix = s
            .bytes()
            .zip(previous.bytes())
            .take_while(|(a, b)| a == b)
            .count();
        while !s.is_char_boundary(prefix) {
            prefix -= 1;
        }
        uint(&mut out, prefix as u32);
        uint(&mut out, (s.len() - prefix) as u32);
        out.extend_from_slice(&s.as_bytes()[prefix..]);
        previous = s;
    }
    let index = |s: &str| strings.binary_search(&s).expect("interned resource string") as u32;
    for (i, row) in rows.iter().enumerate() {
        uint(&mut out, index(row.key));
        match row.value {
            Value::Argb(value) => out.extend_from_slice(&value.to_le_bytes()),
            Value::Text(s) => uint(&mut out, index(s)),
            Value::Import(slot) => uint(&mut out, slot),
            Value::Double(bits) => number(&mut out, bits),
            Value::Quad(components) => {
                for bits in components {
                    number(&mut out, bits);
                }
            }
        }
        for (field, value) in location(row).into_iter().enumerate() {
            if i == 0 || repeat & (1 << field) == 0 {
                uint(&mut out, value);
            }
        }
    }
    if out.len() > MAX_BLOCK_BYTES {
        return Err(Error("FXB block too large"));
    }
    Ok(out)
}

/// Integers -32..=222 occupy one byte. The escape preserves all IEEE-754 bits,
/// including negative zero, fractional values, infinities and arbitrary NaN payloads.
fn number(out: &mut Vec<u8>, bits: u64) {
    let tag = (f64::from_bits(bits) + 32.0) as u8;
    if tag != 255 && (f64::from(tag) - 32.0).to_bits() == bits {
        out.push(tag);
    } else {
        out.push(255);
        out.extend_from_slice(&bits.to_le_bytes());
    }
}
