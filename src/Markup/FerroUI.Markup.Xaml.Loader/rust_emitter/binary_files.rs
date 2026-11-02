//! The binary leaves of one generated module. Filenames and reload identifiers are
//! deterministic and independent of unrelated runs inserted earlier in the document.

use ferroui_xaml_binary::{encode, Kind, Resource};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

#[derive(Clone)]
pub struct BinaryResourceFiles {
    module: String,
    files: Rc<RefCell<BTreeMap<String, Vec<u8>>>>,
}

impl BinaryResourceFiles {
    pub fn new(module: &str) -> Self {
        Self {
            module: module.to_owned(),
            files: Rc::new(RefCell::new(BTreeMap::new())),
        }
    }

    /// Take the artifacts after compiling a group, to write adjacent to its generated Rust.
    pub fn take(&self) -> BTreeMap<String, Vec<u8>> {
        self.files.take()
    }

    pub(super) fn add(&self, function: &str, kind: Kind, rows: &[Resource<'_>]) -> Option<String> {
        // Oversized runs still have the identical typed Rust representation.
        let bytes = encode(kind, rows).ok()?;
        let mut hash = 0xcbf29ce484222325u64;
        for byte in function
            .bytes()
            .chain([0, kind as u8])
            .chain(rows.first()?.key.bytes())
        {
            hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
        }
        let mut files = self.files.borrow_mut();
        let base = format!("{}-{hash:016x}", self.module);
        let mut name = format!("{base}.fxb");
        for ordinal in 1.. {
            if !files.contains_key(&name) {
                break;
            }
            name = format!("{base}-{ordinal}.fxb");
        }
        files.insert(name.clone(), bytes);
        let name = super::rust_string_literal(&name);
        Some(format!("rt::binary_resources::Block::new(concat!(module_path!(), \"/\", {name}), include_bytes!({name}))"))
    }
}
