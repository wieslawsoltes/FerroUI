use crate::DesignTimeEdit;
use ferroui_base::Ref;
use ferroui_controls::{
    ComboBox, Control, StackPanel, TextBlock, TextBox, automation::AutomationProperties,
};
use ferroui_designer_core::NodeId;
use ferroui_designer_core::design_fixtures::{DesignFixtures, FIXTURES_ATTRIBUTE};
use std::rc::Rc;

pub struct DesignFixtureEditorModel {
    pub root: NodeId,
    pub source: Option<String>,
    pub selected: Option<String>,
}

pub(crate) struct DesignFixtureEditor {
    root: NodeId,
    original: String,
    names: Vec<String>,
    picker: Ref<ComboBox>,
    source: Ref<TextBox>,
}

impl DesignFixtureEditor {
    pub fn append(host: &Ref<StackPanel>, model: DesignFixtureEditorModel, writable: bool) -> Self {
        let original = model.source.unwrap_or_default();
        let manifest = DesignFixtures::parse(&original).ok();
        let mut names = manifest
            .as_ref()
            .into_iter()
            .flat_map(|set| &set.fixtures)
            .map(|fixture| fixture.name.clone())
            .collect::<Vec<_>>();
        let picker = ComboBox::new();
        picker.set_name(Some("DesignFixturePicker".into()));
        AutomationProperties::set_name(&picker, Some("Preview design fixture"));
        picker.items().add(Some(Rc::new("View default".to_owned())));
        if let Some(name) = model.selected.as_ref().filter(|name| !names.contains(name)) {
            names.push(name.clone());
        }
        for name in &names {
            picker.items().add(Some(Rc::new(name.clone())));
        }
        let selected = model
            .selected
            .and_then(|name| names.iter().position(|candidate| *candidate == name));
        picker.set_selected_index(selected.map_or(0, |index| 1 + index as i32));
        picker.set_is_enabled(!names.is_empty());

        let label = TextBlock::new();
        label.set_text(Some("Named design fixtures"));
        host.children().add(label.upcast::<Control>());
        host.children().add(picker.clone().upcast::<Control>());
        let source = TextBox::new();
        source.set_name(Some("DesignFixtureJson".into()));
        AutomationProperties::set_name(&source, Some("Fixtures JSON"));
        source.classes().add("designer-property-field");
        source.set_accepts_return(true);
        source.set_min_height(85.0);
        source.set_is_read_only(!writable);
        source.set_placeholder_text(Some(
            r#"{"default":"Content","fixtures":[{"name":"Content","data":{"Status":"Ready"}}]}"#,
        ));
        let original = manifest
            .and_then(|set| serde_json::to_string_pretty(&set).ok())
            .unwrap_or(original);
        source.set_text(Some(&original));
        host.children().add(source.clone().upcast::<Control>());
        Self {
            root: model.root,
            original,
            names,
            picker,
            source,
        }
    }

    pub fn selected(&self) -> Option<String> {
        usize::try_from(self.picker.selected_index())
            .ok()
            .and_then(|index| index.checked_sub(1))
            .and_then(|index| self.names.get(index).cloned())
    }

    pub fn edit(&self) -> Option<DesignTimeEdit> {
        let text = self.source.text().unwrap_or_default();
        (text != self.original).then(|| DesignTimeEdit::SetProperty {
            node: self.root,
            property: FIXTURES_ATTRIBUTE.into(),
            value: (!text.trim().is_empty()).then(|| text.trim().to_owned()),
        })
    }
}
