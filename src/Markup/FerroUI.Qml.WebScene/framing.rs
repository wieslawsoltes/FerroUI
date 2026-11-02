//! Bounded, length-delimited, UTF-8 JSON messages over private process pipes.
use serde_json::Value;
use std::io::{self, Read, Write};
pub const LIMIT: usize = 32 * 1024 * 1024;
pub fn send(out: &mut impl Write, value: &Value) -> io::Result<()> {
    let bytes = serde_json::to_vec(value)?;
    if bytes.len() > LIMIT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "WebScene message too large",
        ));
    }
    out.write_all(&(bytes.len() as u32).to_le_bytes())?;
    out.write_all(&bytes)?;
    out.flush()
}
pub fn receive(input: &mut impl Read) -> io::Result<Value> {
    let mut length = [0; 4];
    input.read_exact(&mut length)?;
    let size = u32::from_le_bytes(length) as usize;
    if size > LIMIT {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "WebScene message too large",
        ));
    }
    let mut data = vec![0; size];
    input.read_exact(&mut data)?;
    Ok(serde_json::from_slice(&data)?)
}
