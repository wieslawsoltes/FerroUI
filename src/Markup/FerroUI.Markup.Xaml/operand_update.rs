//! FXU is an execution delta against one installed Rust/FXB program. Its ABI
//! schema is a trusted build artifact. Neither artifact requires an interpreter.
use super::Provider;
use crate::xaml_il::runtime::compiled::binary_resources::{
    Batch, Kind, SharedBatch, MAX_BLOCK_BYTES,
};
use crate::{CompiledArchiveRoot, RuntimeXamlLoaderConfiguration, XamlLoadError};
use ferroui_base::{utilities::Uri, BoxedValue};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(super) type Blocks = BTreeMap<String, SharedBatch>;

fn invalid(msg: &str) -> XamlLoadError {
    XamlLoadError::with_message(format!("Invalid FXU update: {msg}"))
}

fn take<'a>(input: &mut &'a [u8], n: usize) -> Result<&'a [u8], XamlLoadError> {
    let (head, tail) = input
        .split_at_checked(n)
        .ok_or_else(|| invalid("truncated field"))?;
    *input = tail;
    Ok(head)
}

fn uint(input: &mut &[u8]) -> Result<usize, XamlLoadError> {
    Ok(u32::from_le_bytes(take(input, 4)?.try_into().unwrap()) as usize)
}

fn field<'a>(input: &mut &'a [u8]) -> Result<&'a [u8], XamlLoadError> {
    let n = uint(input)?;
    take(input, n)
}

fn stage(mut schema: &[u8], mut update: &[u8]) -> Result<Blocks, XamlLoadError> {
    if update.len() < 72 || update.len() > 64 << 20 || schema.len() > 2 << 20 {
        return Err(invalid("invalid envelope length"));
    }
    let (body, checksum) = update.split_at(update.len() - 32);
    if Sha256::digest(body).as_slice() != checksum {
        return Err(invalid("package checksum mismatch"));
    }
    update = body;
    let expected = Sha256::digest(schema);
    if take(&mut schema, 4)? != b"FXS\x01" || take(&mut update, 4)? != b"FXU\x01" {
        return Err(invalid("signature or version"));
    }
    take(&mut schema, 32)?; // the source, options, call layout and original operand digest
    if take(&mut update, 32)? != expected.as_slice() {
        return Err(invalid("wrong installed build"));
    }
    let text = |s: &[u8]| {
        std::str::from_utf8(s)
            .map(str::to_owned)
            .map_err(|_| invalid("schema name encoding"))
    };
    let module = text(field(&mut schema)?)?;
    let prefix = text(field(&mut schema)?)?;
    let leaves = uint(&mut schema)?;
    let count = uint(&mut update)?;
    if leaves > 65536 || count > leaves || schema.len() != leaves * 15 {
        return Err(invalid("invalid layout"));
    }
    let mut patches = Blocks::new();
    let mut last = None;
    let mut allocated = 0usize;
    for _ in 0..count {
        let i = uint(&mut update)?;
        if i >= leaves || last.is_some_and(|prev| i <= prev) {
            return Err(invalid("unknown, duplicate or unsorted block"));
        }
        last = Some(i);
        let row = &schema[i * 15..(i + 1) * 15];
        let hash = u64::from_le_bytes(row[..8].try_into().unwrap());
        let collision = u16::from_le_bytes(row[8..10].try_into().unwrap());
        let expected =
            Kind::from_wire(row[10]).ok_or_else(|| invalid("invalid linked block kind"))?;
        let imports = u32::from_le_bytes(row[11..].try_into().unwrap()) as usize;
        let bytes = field(&mut update)?;
        if bytes.len() > MAX_BLOCK_BYTES {
            return Err(invalid("oversized block"));
        }
        let batch = Batch::decode(bytes).map_err(|e| invalid(&e.to_string()))?;
        allocated = allocated
            .checked_add(batch.allocated_bytes())
            .filter(|bytes| *bytes <= 128 << 20)
            .ok_or_else(|| invalid("expanded operand data exceeds the update budget"))?;
        if batch.kind() != expected {
            return Err(invalid("block kind differs from native caller"));
        }
        if expected == Kind::DeferredFactory {
            batch
                .validate_imports(imports)
                .map_err(|e| invalid(&e.to_string()))?;
        }
        let id = if collision == 0 {
            format!("{module}/{prefix}-{hash:016x}.fxb")
        } else {
            format!("{module}/{prefix}-{hash:016x}-{collision}.fxb")
        };
        patches.insert(id, SharedBatch::new(batch));
    }
    if !update.is_empty() {
        return Err(invalid("trailing bytes"));
    }
    Ok(patches)
}

/// Run a builder-produced FXU revision. The schema must come from the installed
/// build. Each decoded block can select only its generated call site's imports.
pub fn load_operand_update(
    root: &CompiledArchiveRoot,
    schema: &[u8],
    bytes: &[u8],
    uri: &Uri,
    config: &RuntimeXamlLoaderConfiguration,
    parent: Option<&Provider>,
) -> Result<Option<BoxedValue>, XamlLoadError> {
    if !root.accepts(uri, config) {
        return Ok(None);
    }
    let blocks = stage(schema, bytes)?;
    crate::xaml_error_scope::with_xaml_errors(|| {
        super::scoped::with_blocks(root.class, blocks, || (root.load)(parent, root.uri))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const BLUE: &[u8] = b"FXB\0\x01\x02\x01\x01\x00\x01A\x00\xff\x00\x00\xff\x01\x01\x00\x00";
    const FACTORY: &[u8] = b"FXB\0\x01\x05\x01\x01\x00\x01A\x00\x01\x01\x01\x00\x00";

    fn field(out: &mut Vec<u8>, bytes: &[u8]) {
        out.extend((bytes.len() as u32).to_le_bytes());
        out.extend(bytes);
    }
    fn schema() -> Vec<u8> {
        let mut bytes = b"FXS\x01".to_vec();
        bytes.extend([0x15; 32]);
        field(&mut bytes, b"fixture::compiled");
        field(&mut bytes, b"compiled");
        bytes.extend(2u32.to_le_bytes());
        for (hash, kind, imports) in [
            (17u64, Kind::LiteralBrush, 0u32),
            (42, Kind::DeferredFactory, 1),
        ] {
            bytes.extend(hash.to_le_bytes());
            bytes.extend(0u16.to_le_bytes());
            bytes.push(kind as u8);
            bytes.extend(imports.to_le_bytes());
        }
        bytes
    }
    fn update(schema: &[u8], changes: &[(u32, &[u8])]) -> Vec<u8> {
        let mut bytes = b"FXU\x01".to_vec();
        bytes.extend(Sha256::digest(schema));
        bytes.extend((changes.len() as u32).to_le_bytes());
        for (slot, leaf) in changes {
            bytes.extend(slot.to_le_bytes());
            field(&mut bytes, leaf);
        }
        bytes.extend(Sha256::digest(&bytes));
        bytes
    }

    #[test]
    fn verify_whole_revision_and_native_slots_before_using_any_block() {
        let schema = schema();
        let good = update(&schema, &[(0, BLUE)]);
        assert_eq!(stage(&schema, &good).unwrap().len(), 1);
        assert!(stage(&schema, &update(&schema, &[])).unwrap().is_empty());
        for i in 0..good.len() {
            assert!(stage(&schema, &good[..i]).is_err());
        }
        for i in [0, 4, 36, 40, good.len() - 1] {
            let mut broken = good.clone();
            broken[i] ^= 1;
            assert!(stage(&schema, &broken).is_err());
        }
        let mut other = schema.clone();
        other[17] ^= 1;
        assert!(stage(&other, &good).is_err());
        let mut tail = good.clone();
        tail.push(0);
        assert!(stage(&schema, &tail).is_err());
        for rows in [
            vec![(0, BLUE), (0, BLUE)],
            vec![(2, BLUE)],
            vec![(1, BLUE)],
            vec![(0, BLUE), (1, FACTORY)],
        ] {
            assert!(stage(&schema, &update(&schema, &rows)).is_err());
        }
        assert_eq!(stage(&schema, &good).unwrap().len(), 1);
    }
}
