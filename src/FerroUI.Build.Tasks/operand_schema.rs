//! Compact immutable ABI for an opt-in FXU consumer. The schema references
//! block ordinals and data, so retaining it cannot root a native factory.
use ferroui_markup_xaml_loader::rust_emitter::BinaryLeaf;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};

fn field(bytes: &[u8]) -> Result<[u8; 4], String> {
    Ok(u32::try_from(bytes.len())
        .map_err(|_| "FXU field exceeds u32")?
        .to_le_bytes())
}

pub(super) fn encode(
    file: &Path,
    module: &str,
    source: &str,
    options: [bool; 2],
    leaves: &BTreeMap<String, BinaryLeaf>,
) -> Result<Vec<u8>, String> {
    let prefix = file
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or("invalid FXU module filename")?;
    let module = module.trim_start_matches("::");
    let flags = format!("({}, {})\n", options[0], options[1]);
    let mut hash = Sha256::new();
    hash.update(b"FerroUI/FXU/1");
    for value in [module, prefix, source, &flags] {
        hash.update(field(value.as_bytes())?);
        hash.update(value.as_bytes());
    }
    if leaves.len() > 65_536 {
        return Err("too many FXU block slots".into());
    }
    let mut rows = Vec::with_capacity(leaves.len() * 15);
    for (name, leaf) in leaves {
        let suffix = name
            .strip_prefix(prefix)
            .and_then(|s| s.strip_prefix('-'))
            .and_then(|s| s.strip_suffix(".fxb"))
            .ok_or("invalid generated FXB filename")?;
        let (id, collision) = suffix.split_once('-').unwrap_or((suffix, "0"));
        let id = u64::from_str_radix(id, 16).map_err(|_| "invalid generated FXB ID")?;
        let collision: u16 = collision
            .parse()
            .map_err(|_| "FXB collision ordinal exceeds u16")?;
        let imports =
            u32::try_from(leaf.factory_imports.unwrap_or(0)).map_err(|_| "too many FXB imports")?;
        let start = rows.len();
        rows.extend(id.to_le_bytes());
        rows.extend(collision.to_le_bytes());
        rows.push(leaf.bytes[5]);
        rows.extend(imports.to_le_bytes());
        hash.update(&rows[start..]);
        hash.update(field(&leaf.bytes)?);
        hash.update(&leaf.bytes);
    }
    let mut schema = b"FXS\x01".to_vec();
    schema.extend(hash.finalize());
    for value in [module, prefix] {
        schema.extend(field(value.as_bytes())?);
        schema.extend(value.bytes());
    }
    schema.extend((leaves.len() as u32).to_le_bytes());
    schema.extend(rows);
    Ok(schema)
}
