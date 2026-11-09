//! CustomLayouts.cs. The explicit registry parameter replaces the unresolved Core Default access.
use ferroboard_core::layouts::{
    ComponentName, KeyboardResources, ResourceResult, Subtype, SubtypeLayoutMap,
};
use std::sync::Arc;
const ALPHABETICAL: &str = r#"
{
  "type": "characters",
  "id": "alphabetical",
  "label": "Alphabetical (ABC)",
  "authors": [ "ScreenKeyboard sample" ],
  "direction": "ltr",
  "arrangement": [
    [ "a", "b", "c", "d", "e", "f", "g", "h", "i", "j" ],
    [ "k", "l", "m", "n", "o", "p", "q", "r", "s" ],
    [ "t", "u", "v", "w", "x", "y", "z" ]
  ]
}
"#;
pub fn register(resources: &KeyboardResources) -> ResourceResult<()> {
    let layout = resources.add_layout_json(ALPHABETICAL, Some("sample".into()))?;
    resources.add_subtype_preset(Arc::new(Subtype::from_values(
        ferroboard_core::layouts::SubtypeValues {
            display_name: Some("English (ABC)".into()),
            popup_mapping: ComponentName::new(Subtype::CORE_LOCALIZATION, "en"),
            layouts: Some(Arc::new(SubtypeLayoutMap::from_values(
                ferroboard_core::layouts::SubtypeLayoutMapValues {
                    characters: layout.name(),
                    ..SubtypeLayoutMap::default().values()
                },
            ))),
            ..Subtype::new("en-US".into()).values()
        },
    )));
    Ok(())
}
