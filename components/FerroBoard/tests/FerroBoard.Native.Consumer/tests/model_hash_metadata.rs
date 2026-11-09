//! CLR hash methods must not reappear in downstream model metadata.
use ferroboard_core_ferro::{
    KeyComputeContextMetadata, KeyEvaluatorMetadata, SubtypeLayoutMapMetadata, SubtypeMetadata,
};
use ferroui_base::metadata::MarkupTyped;
#[test]
fn reusable_model_metadata_does_not_export_clr_hash_contracts() {
    ferroboard_core_ferro::register_types();
    for metadata in [
        KeyComputeContextMetadata::MARKUP,
        KeyEvaluatorMetadata::MARKUP,
        SubtypeMetadata::MARKUP,
        SubtypeLayoutMapMetadata::MARKUP,
    ] {
        assert!(
            metadata.find_methods("GetHashCode").next().is_none(),
            "{}",
            metadata.name
        );
        assert!(
            !metadata.constructors.is_empty(),
            "{} keeps native construction",
            metadata.name
        );
        assert!(
            !metadata.properties.is_empty(),
            "{} keeps keyboard configuration",
            metadata.name
        );
    }
}
