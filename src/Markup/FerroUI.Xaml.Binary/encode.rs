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

/// Encode a homogeneous run without changing insertion or error order.
pub fn encode(kind: Kind, rows: &[Resource<'_>]) -> Result<Vec<u8>, Error> {
    if rows.len() > MAX_ITEMS {
        return Err(Error("too many FXB records"));
    }
    let mut strings = Vec::with_capacity(rows.len());
    for row in rows {
        strings.push(row.key);
        match (kind, row.value) {
            (Kind::LiteralBrush, Value::Argb(_)) | (Kind::DeferredFactory, Value::Import(_)) => {}
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
    let mut out = HEADER.to_vec();
    out.push(kind as u8);
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
    for row in rows {
        uint(&mut out, index(row.key));
        match row.value {
            Value::Argb(value) => out.extend_from_slice(&value.to_le_bytes()),
            Value::Text(s) => uint(&mut out, index(s)),
            Value::Import(slot) => uint(&mut out, slot),
        }
        uint(&mut out, row.line);
        uint(&mut out, row.column);
        uint(&mut out, zigzag(row.line, row.value_line));
        uint(&mut out, zigzag(row.column, row.value_column));
    }
    if out.len() > MAX_BLOCK_BYTES {
        return Err(Error("FXB block too large"));
    }
    Ok(out)
}
