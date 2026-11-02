//! The binary leaves of one generated module. Filenames and reload identifiers are
//! deterministic and independent of unrelated runs inserted earlier in the document.

use ferroui_xaml_binary::{encode, Kind, Resource};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

/// A compiled leaf and the number of native builders its call site exposes.
pub struct BinaryLeaf {
    pub bytes: Vec<u8>,
    pub factory_imports: Option<usize>,
}

#[derive(Clone)]
pub struct BinaryResourceFiles {
    module: String,
    files: Rc<RefCell<BTreeMap<String, BinaryLeaf>>>,
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
        self.take_leaves().into_iter().map(|(name, leaf)| (name, leaf.bytes)).collect()
    }

    /// Take the bytes and call-site ABI for a reloadable module.
    pub fn take_leaves(&self) -> BTreeMap<String, BinaryLeaf> {
        self.files.take()
    }

    /// Expose the compiled call-site manifest only to development hosts. A release
    /// build never expands the include or retains the host's absolute OUT_DIR path.
    pub(super) fn declare_manifest(&self, source: &mut String) {
        let name = super::rust_string_literal(&format!("{}.fxb.json", self.module));
        source.push_str(&format!(
            "\n/// Original build path and compiled baseline of the FXB hot-reload manifest.\n#[cfg(debug_assertions)]\npub const FXB_MANIFEST: (&str, &str) = (concat!(env!(\"OUT_DIR\"), \"/xaml/\", {name}), include_str!({name}));\n"
        ));
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
        let factory_imports = (kind == Kind::DeferredFactory).then_some(rows.len());
        files.insert(name.clone(), BinaryLeaf { bytes, factory_imports });
        let name = super::rust_string_literal(&name);
        Some(format!("rt::binary_resources::Block::new(concat!(module_path!(), \"/\", {name}), include_bytes!({name}))"))
    }
}
