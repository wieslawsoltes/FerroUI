//! Wire primitives shared by the build-time encoders and the compiled XAML runtime.
pub type ReadResult<T> = Result<T, &'static str>;

pub fn unsigned(bytes: &mut Vec<u8>, mut value: u32) {
    while value >= 128 {
        bytes.push((value as u8) | 128);
        value >>= 7;
    }
    bytes.push(value as u8);
}

pub fn signed(bytes: &mut Vec<u8>, value: i32) {
    unsigned(bytes, (value as u32) << 1 ^ (value >> 31) as u32);
}

pub fn number(bytes: &mut Vec<u8>, value: f64) {
    let small = value as i16;
    if (-32..=222).contains(&small) && f64::from(small).to_bits() == value.to_bits() {
        bytes.push((small + 32) as u8);
    } else {
        bytes.push(255);
        bytes.extend(value.to_le_bytes());
    }
}

pub fn take<const N: usize>(input: &mut &[u8]) -> ReadResult<[u8; N]> {
    let (bytes, rest) = input.split_first_chunk::<N>().ok_or("truncated operand")?;
    *input = rest;
    Ok(*bytes)
}

pub fn read_unsigned(input: &mut &[u8]) -> ReadResult<u32> {
    let mut value = 0;
    for shift in (0..32).step_by(7) {
        let [byte] = take(input)?;
        if shift == 28 && byte > 15 {
            return Err("integer overflow");
        }
        value |= u32::from(byte & 127) << shift;
        if byte < 128 {
            if shift > 0 && byte == 0 {
                return Err("noncanonical integer");
            }
            return Ok(value);
        }
    }
    Err("unterminated integer")
}

pub fn read_signed(input: &mut &[u8]) -> ReadResult<i32> {
    let bits = read_unsigned(input)?;
    Ok((bits >> 1) as i32 ^ -((bits & 1) as i32))
}

pub fn read_number(input: &mut &[u8]) -> ReadResult<f64> {
    match take(input)? {
        [255] => Ok(f64::from_le_bytes(take(input)?)),
        [small] => Ok(f64::from(small) - 32.0),
    }
}
