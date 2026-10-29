//! The public Rust paths of the registered types of this crate, by the shortest path another crate
//! names it by; the emitter of Rust source of the markup compiler reads them.

ferroui_base::ferro_rust_paths! {
    classes: [
        crate::AdaptiveBehavior,
        crate::AdaptiveClassSetter,
        crate::AspectRatioBehavior,
        crate::AspectRatioClassSetter,
    ],
    types: [
        crate::AdaptiveClassSetters,
        crate::AspectRatioClassSetters,
    ],
    contracts: [
    ],
    generics: [
    ],
    generic_contracts: [
    ],
}
