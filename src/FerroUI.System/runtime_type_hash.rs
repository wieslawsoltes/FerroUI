//! Native runtime type identity hashing, shared by Core records and native type descriptors.
//!
//! Runtime type hash values belong to one process. This bridge uses the intrinsic TypeId,
//! never the display name, a temporary facade allocation, or a value's user callbacks.
use std::{
    any::TypeId,
    collections::hash_map::RandomState,
    hash::{BuildHasher, Hash},
    sync::LazyLock,
};

/// The process-local Int32 hash of the actual represented native type identity.
/// Equal identities always receive the same hash, including across OS threads. Values
/// are not portable across processes/runtimes and must not be serialized as stable IDs.
pub fn runtime_type_hash(type_id: TypeId) -> i32 {
    runtime_identity_hash(type_id)
}

/// Hash native allocation identity with the same process-local intrinsic state. Callers
/// must supply actual reference identity (for example Ref::hash or a canonical Rc data
/// address), not a value's virtual/field hash or a temporary facade allocation.
/// No reference or address is retained by this operation.
pub fn runtime_identity_hash(identity: impl Hash) -> i32 {
    static STATE: LazyLock<RandomState> = LazyLock::new(RandomState::new);
    STATE.hash_one(identity) as u32 as i32
}

/// The identity hash of `T`, without constructing or retaining an instance of it.
pub fn runtime_type_hash_of<T: 'static>() -> i32 {
    runtime_type_hash(TypeId::of::<T>())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn generic_and_erased_identities_agree_without_constructing_values() {
        struct Unconstructible {
            _private: std::convert::Infallible,
        }
        assert_eq!(
            runtime_type_hash_of::<Unconstructible>(),
            runtime_type_hash(TypeId::of::<Unconstructible>())
        );
        type Alias = Unconstructible;
        assert_eq!(
            runtime_type_hash_of::<Unconstructible>(),
            runtime_type_hash_of::<Alias>()
        );
        for _ in 0..32 {
            assert_eq!(
                runtime_type_hash_of::<i32>(),
                runtime_type_hash(TypeId::of::<i32>())
            );
        }
    }
    #[test]
    fn initial_and_repeated_calls_share_one_process_state_across_fresh_threads() {
        struct Concurrent;
        let workers: Vec<_> = (0..16)
            .map(|_| std::thread::spawn(runtime_type_hash_of::<Concurrent>))
            .collect();
        let values: Vec<_> = workers.into_iter().map(|v| v.join().unwrap()).collect();
        assert!(values
            .iter()
            .all(|v| *v == runtime_type_hash_of::<Concurrent>()));
    }
}
