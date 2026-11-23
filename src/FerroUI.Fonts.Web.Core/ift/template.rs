//! The URL template of a patch map (section 5.3.4 of the standard).

use crate::FontDecodeError;

const MALFORMED: FontDecodeError = FontDecodeError::Malformed("URL template");
const BASE32_HEX: &[u8; 32] = b"0123456789ABCDEFGHIJKLMNOPQRSTUV";
const BASE64_URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

/// The id of a patch: a number, or the bytes of a string id.
#[derive(Clone, Copy, Debug)]
pub(super) enum EntryId<'a> {
    Number(u32),
    Bytes(&'a [u8]),
}

/// Checks that a template can be expanded: its opcodes and that its literals are UTF-8.
pub(super) fn validate_template(template: &[u8]) -> Result<(), FontDecodeError> {
    walk(template, |_| Ok(()), |_| Ok(()))
}

fn walk(
    template: &[u8],
    mut literal: impl FnMut(&str) -> Result<(), FontDecodeError>,
    mut variable: impl FnMut(u8) -> Result<(), FontDecodeError>,
) -> Result<(), FontDecodeError> {
    let mut position = 0;
    while let Some(&opcode) = template.get(position) {
        position += 1;
        match opcode {
            0 => return Err(MALFORMED),
            1..=127 => {
                let end = position + opcode as usize;
                let bytes = template.get(position..end).ok_or(MALFORMED)?;
                literal(std::str::from_utf8(bytes).map_err(|_| MALFORMED)?)?;
                position = end;
            }
            128..=133 => variable(opcode)?,
            _ => return Err(MALFORMED),
        }
    }
    Ok(())
}

/// The URL of a patch.
pub(super) fn expand_template(template: &[u8], id: EntryId<'_>) -> Result<String, FontDecodeError> {
    let number;
    let bytes: &[u8] = match id {
        EntryId::Bytes(bytes) => bytes,
        EntryId::Number(value) => {
            number = value.to_be_bytes();
            // Without its leading zero bytes; zero is one zero byte.
            let skip = number.iter().take(3).take_while(|byte| **byte == 0).count();
            &number[skip..]
        }
    };
    let id32 = base32_hex(bytes);
    let output = std::cell::RefCell::new(String::with_capacity(template.len() + id32.len()));
    walk(
        template,
        |literal| {
            output.borrow_mut().push_str(literal);
            Ok(())
        },
        |opcode| {
            let mut output = output.borrow_mut();
            match opcode {
                128 => output.push_str(&id32),
                129..=132 => {
                    let back = usize::from(opcode - 128);
                    let digit = id32.len().checked_sub(back).map(|index| id32.as_bytes()[index]);
                    output.push(digit.map_or('_', char::from));
                }
                _ => output.push_str(&base64_url(bytes)),
            }
            Ok(())
        },
    )?;
    Ok(output.into_inner())
}

/// Base32hex without padding (RFC 4648, section 7).
fn base32_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 8 / 5 + 1);
    let (mut buffer, mut bits) = (0u32, 0u32);
    for byte in bytes {
        buffer = (buffer << 8) | u32::from(*byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            output.push(char::from(BASE32_HEX[((buffer >> bits) & 31) as usize]));
        }
        buffer &= (1 << bits) - 1;
    }
    if bits > 0 {
        output.push(char::from(BASE32_HEX[((buffer << (5 - bits)) & 31) as usize]));
    }
    output
}

/// Base64url with its padding, each `=` written as `%3D` as the standard requires.
fn base64_url(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 6);
    for chunk in bytes.chunks(3) {
        let value = chunk.iter().fold(0u32, |value, byte| (value << 8) | u32::from(*byte)) << (8 * (3 - chunk.len()));
        for index in 0..4 {
            if index <= chunk.len() {
                output.push(char::from(BASE64_URL[((value >> (18 - 6 * index)) & 63) as usize]));
            } else {
                output.push_str("%3D");
            }
        }
    }
    output
}
