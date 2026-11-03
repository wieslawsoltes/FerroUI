//! Test-only migration from former pane ids to source labels in the actual public panel.
//! No alias is registered in the runtime name scope and no presentation model is inspected.
use crate::composition::{MainViewSession, SampleSession};
use ferroui_base::{
    FerroObject, Ref,
    controls::{NameScope, NameScopeRef},
    metadata::from_markup_value,
};
use ferroui_controls::{Button, Control, Panel, ScrollViewer};
pub struct SampleUi {
    scope: NameScopeRef,
    panel: Ref<crate::SettingsPanel>,
}
pub fn ui(session: &SampleSession) -> SampleUi {
    SampleUi {
        scope: NameScope::get_name_scope(&session.window).unwrap(),
        panel: session.settings_panel.clone(),
    }
}
pub fn view_ui(session: &MainViewSession) -> SampleUi {
    SampleUi {
        scope: NameScope::get_name_scope(&session.view).unwrap(),
        panel: session.settings_panel.clone(),
    }
}
fn find(root: Ref<Control>, predicate: &dyn Fn(&Ref<Control>) -> bool) -> Option<Ref<Control>> {
    if predicate(&root) {
        return Some(root);
    }
    if let Some(stack) = root.cast::<Panel>() {
        for child in stack.children().to_vec() {
            if let Some(found) = find(child, predicate) {
                return Some(found);
            }
        }
    } else if let Some(scroll) = root.cast::<ScrollViewer>()
        && let Some(content) = from_markup_value::<Ref<Control>>(&scroll.content())
    {
        return find(content, predicate);
    }
    None
}
impl SampleUi {
    pub fn find(&self, name: &str) -> Option<Ref<FerroObject>> {
        if let Some(element) = self.scope.find(name) {
            return Some(element);
        }
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../../docs/sample-settings-controls.json")).unwrap();
        let scalar = manifest["controls"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["kind"] == "toggle" || v["kind"] == "slider")
            .chain(manifest["enum_controls"].as_array().unwrap().iter())
            .chain(manifest["theme_controls"].as_array().unwrap().iter())
            .find(|v| {
                let suffix = match v["kind"].as_str() {
                    Some("toggle") => "Toggle",
                    Some("slider") => "Slider",
                    _ => "Choice",
                };
                format!("{}{suffix}", v["property"].as_str().unwrap()) == name
            });
        let predicate = |control: &Ref<Control>| {
            if let Some(item) = scalar {
                return ferroui_controls::automation::AutomationProperties::get_name(control)
                    .as_deref()
                    == item["label"].as_str();
            }
            match name {
                "LanguageFilter" => control.name().as_deref() == Some("Filter"),
                "LanguageChoices" => control.name().as_deref() == Some("Choices"),
                "ResetButton" | "ClearLearnedButton" => {
                    control.cast::<Button>().is_some()
                        && from_markup_value::<String>(&control.cast::<Button>().unwrap().content())
                            .as_deref()
                            == Some(if name == "ResetButton" {
                                "Reset settings"
                            } else {
                                "Clear learned words"
                            })
                }
                _ => false,
            }
        };
        find(
            from_markup_value::<Ref<Control>>(&self.panel.content()).unwrap(),
            &predicate,
        )
        .map(Ref::upcast)
    }
}
