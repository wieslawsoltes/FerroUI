// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use crate::{
    error, IJsonTypeInfoResolver, JsonErrorKind, JsonListType, JsonReadContext,
    JsonSerializerOptions, JsonValue, JsonWriteContext,
};
use ferrodock_model::{
    core::{DockSerializationError, DockStream, IDockSerializer, SerializationResult},
    io::StreamIo,
};
use std::{
    io::{Read, Write},
    panic::{catch_unwind, resume_unwind, AssertUnwindSafe},
    rc::Rc,
};
/// Original generic serializer using a supplied generated metadata resolver.
/// Default model discovery remains unexposed until its compiled catalog is ported.
pub struct DockSerializer {
    resolver: Rc<dyn IJsonTypeInfoResolver>,
    options: JsonSerializerOptions,
}
impl DockSerializer {
    /// Creates the original resolver-based overload; null resolver is rejected immediately.
    pub fn with_resolver(
        resolver: Option<Rc<dyn IJsonTypeInfoResolver>>,
    ) -> SerializationResult<Rc<Self>> {
        Ok(Rc::new(Self {
            resolver: resolver
                .ok_or_else(|| error(JsonErrorKind::ArgumentNull, "typeInfoResolver"))?,
            options: JsonSerializerOptions::default(),
        }))
    }
    /// Creates the original list-type and resolver overload with source guard order.
    pub fn with_list_type_and_resolver(
        list_type: Option<JsonListType>,
        resolver: Option<Rc<dyn IJsonTypeInfoResolver>>,
    ) -> SerializationResult<Rc<Self>> {
        let list_type = list_type.ok_or_else(|| error(JsonErrorKind::ArgumentNull, "listType"))?;
        let resolver =
            resolver.ok_or_else(|| error(JsonErrorKind::ArgumentNull, "typeInfoResolver"))?;
        Ok(Rc::new(Self {
            resolver,
            options: JsonSerializerOptions::with_list_type(list_type),
        }))
    }
    /// Gets the same options instance passed to supplied metadata resolvers.
    pub fn options(&self) -> &JsonSerializerOptions {
        &self.options
    }
}
impl IDockSerializer for DockSerializer {
    fn serialize<T: 'static>(&self, value: T) -> SerializationResult<String> {
        JsonWriteContext::new(self.resolver.as_ref(), &self.options)
            .value(&value)?
            .to_json()
    }
    fn deserialize<T: 'static>(&self, text: Option<&str>) -> SerializationResult<Option<T>> {
        let text = text.ok_or_else(|| error(JsonErrorKind::ArgumentNull, "json"))?;
        let mut context = JsonReadContext::new(self.resolver.as_ref(), &self.options);
        context.info::<T>()?;
        context.value(&JsonValue::parse(text)?)
    }
    fn load<T: 'static>(
        &self,
        stream: Option<Rc<dyn DockStream>>,
    ) -> SerializationResult<Option<T>> {
        let stream = stream.ok_or_else(|| error(JsonErrorKind::ArgumentNull, "stream"))?;
        if !stream.can_read() {
            return Err(error(JsonErrorKind::Argument, "Stream was not readable."));
        }
        using_stream(stream.clone(), || {
            let mut bytes = Vec::new();
            StreamIo::new(stream)
                .read_to_end(&mut bytes)
                .map_err(DockSerializationError::new)?;
            let text = decode_text(&bytes);
            self.deserialize(Some(&text))
        })
    }
    fn save<T: 'static>(
        &self,
        stream: Option<Rc<dyn DockStream>>,
        value: T,
    ) -> SerializationResult<()> {
        let text = self.serialize(value)?;
        if text.chars().all(char::is_whitespace) {
            return Ok(());
        }
        let stream = stream.ok_or_else(|| error(JsonErrorKind::ArgumentNull, "stream"))?;
        if !stream.can_write() {
            return Err(error(JsonErrorKind::Argument, "Stream was not writable."));
        }
        let preamble =
            !stream.can_seek() || stream.position().map_err(DockSerializationError::new)? == 0;
        using_stream(stream.clone(), || {
            let mut io = StreamIo::new(stream);
            if preamble {
                io.write_all(&[0xEF, 0xBB, 0xBF])
                    .map_err(DockSerializationError::new)?;
            }
            io.write_all(text.as_bytes())
                .map_err(DockSerializationError::new)?;
            io.flush().map_err(DockSerializationError::new)
        })
    }
}
fn using_stream<T>(
    stream: Rc<dyn DockStream>,
    action: impl FnOnce() -> SerializationResult<T>,
) -> SerializationResult<T> {
    let result = catch_unwind(AssertUnwindSafe(action));
    stream.close().map_err(DockSerializationError::new)?;
    match result {
        Ok(result) => result,
        Err(error) => resume_unwind(error),
    }
}
fn decode_text(bytes: &[u8]) -> String {
    if let Some(bytes) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    let (bytes, width, little) = if let Some(b) = bytes.strip_prefix(&[0xFF, 0xFE, 0, 0]) {
        (b, 4, true)
    } else if let Some(b) = bytes.strip_prefix(&[0, 0, 0xFE, 0xFF]) {
        (b, 4, false)
    } else if let Some(b) = bytes.strip_prefix(&[0xFF, 0xFE]) {
        (b, 2, true)
    } else if let Some(b) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        (b, 2, false)
    } else {
        return String::from_utf8_lossy(bytes).into_owned();
    };
    if width == 2 {
        let units = bytes.as_chunks::<2>().0.iter().map(|b| {
            if little {
                u16::from_le_bytes([b[0], b[1]])
            } else {
                u16::from_be_bytes([b[0], b[1]])
            }
        });
        let mut text = String::from_utf16_lossy(&units.collect::<Vec<_>>());
        if bytes.len() % 2 != 0 {
            text.push(char::REPLACEMENT_CHARACTER);
        }
        text
    } else {
        let mut text = String::new();
        for b in bytes.as_chunks::<4>().0 {
            let n = if little {
                u32::from_le_bytes([b[0], b[1], b[2], b[3]])
            } else {
                u32::from_be_bytes([b[0], b[1], b[2], b[3]])
            };
            text.push(char::from_u32(n).unwrap_or(char::REPLACEMENT_CHARACTER));
        }
        if bytes.len() % 4 != 0 {
            text.push(char::REPLACEMENT_CHARACTER);
        }
        text
    }
}
