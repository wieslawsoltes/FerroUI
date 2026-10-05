// RichTextKit
// Copyright © 2019 Topten Software. All Rights Reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may
// not use this product except in compliance with the License. You may obtain
// a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
// WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
// License for the specific language governing permissions and limitations
// under the License.
// Copied from: https://github.com/toptensoftware/RichTextKit

use std::io::{self, Read, Write};

/// Big endian reads for binary readers.
#[allow(dead_code)] // upstream helpers without users in the runtime library
pub(crate) trait BinaryReaderExtensions: Read {
    fn read_int32_be(&mut self) -> io::Result<i32> {
        let mut bytes = [0u8; 4];
        self.read_exact(&mut bytes)?;
        Ok(i32::from_be_bytes(bytes))
    }

    fn read_uint32_be(&mut self) -> io::Result<u32> {
        let mut bytes = [0u8; 4];
        self.read_exact(&mut bytes)?;
        Ok(u32::from_be_bytes(bytes))
    }
}

impl<R: Read + ?Sized> BinaryReaderExtensions for R {}

/// Big endian writes for binary writers (the `WriteBE` overloads upstream).
#[allow(dead_code)] // upstream helpers without users in the runtime library
pub(crate) trait BinaryWriterExtensions: Write {
    fn write_be_i32(&mut self, value: i32) -> io::Result<()> {
        self.write_all(&value.to_be_bytes())
    }

    fn write_be_u32(&mut self, value: u32) -> io::Result<()> {
        self.write_all(&value.to_be_bytes())
    }
}

impl<W: Write + ?Sized> BinaryWriterExtensions for W {}
