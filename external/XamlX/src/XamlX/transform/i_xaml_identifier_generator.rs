//! Port of `Transform/IXamlIdentifierGenerator.cs`.

use std::collections::hash_map::RandomState;
use std::hash::{BuildHasher, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};

pub trait IXamlIdentifierGenerator {
    fn generate_identifier_part(&self) -> String;
}

/// Generates 32 hexadecimal digits that are unique within the process and random across
/// processes (the upstream implementation formats a new GUID).
#[derive(Default)]
pub struct GuidIdentifierGenerator;

impl IXamlIdentifierGenerator for GuidIdentifierGenerator {
    fn generate_identifier_part(&self) -> String {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut high = RandomState::new().build_hasher();
        high.write_u64(counter);
        let mut low = RandomState::new().build_hasher();
        low.write_u64(counter.wrapping_add(0x9E37_79B9_7F4A_7C15));
        format!("{:016x}{:016x}", high.finish(), low.finish())
    }
}
