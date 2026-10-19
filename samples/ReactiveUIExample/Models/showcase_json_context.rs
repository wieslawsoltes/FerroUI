//! Port of `Models/ShowcaseJsonContext.cs`: the JSON of the persisted showcase settings.
//!
//! The reference generates the metadata of its serializer for [`ShowcaseState`]
//! (`[JsonSerializable(typeof(ShowcaseState))]`); here the state derives `serde`'s serialization,
//! and the context gives the two calls the session makes.

use super::ShowcaseState;

/// Reads and writes the JSON of the persisted showcase settings (`ShowcaseJsonContext`).
pub struct ShowcaseJsonContext;

impl ShowcaseJsonContext {
    /// `JsonSerializer.Serialize(state, ShowcaseJsonContext.Default.ShowcaseState)`.
    pub fn serialize_showcase_state(state: &ShowcaseState) -> Result<String, serde_json::Error> {
        serde_json::to_string(state)
    }

    /// `JsonSerializer.Deserialize(text, ShowcaseJsonContext.Default.ShowcaseState)`: the state the
    /// document `text` holds, `None` for the document `null`.
    pub fn deserialize_showcase_state(text: &str) -> Result<Option<ShowcaseState>, serde_json::Error> {
        serde_json::from_str(text)
    }
}
