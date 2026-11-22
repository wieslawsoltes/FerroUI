use crate::library_scopes::LibraryScope;
use ferroui_base::{Ref, Thickness};
use ferroui_controls::{
    automation::AutomationProperties, ColumnDefinitions, ComboBox, Control, Grid, ToolTip,
};
use ferroui_designer_core::NodeId;
use ferroui_designer_language::LineIndex;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

pub(super) struct ScopeFilter {
    row: Ref<Grid>,
    picker: Ref<ComboBox>,
    model: RefCell<Model>,
}

#[derive(Default, PartialEq, Eq)]
struct Model {
    uri: String,
    owners: Vec<(NodeId, String, String)>,
    objects: BTreeMap<String, (NodeId, bool)>,
}

impl ScopeFilter {
    pub(super) fn new(category: &Ref<ComboBox>) -> Self {
        let row = Grid::new();
        row.set_margin(Thickness::new(4.0, 0.0, 4.0, 8.0));
        row.children().add(category.clone().upcast::<Control>());
        let picker = ComboBox::new();
        picker.set_name(Some("ComponentLibraryScope".into()));
        picker.set_margin(Thickness::new(4.0, 0.0, 0.0, 0.0));
        picker.set_is_visible(false);
        AutomationProperties::set_name(&picker, Some("Owning scope"));
        ToolTip::set_tip(
            &picker,
            Some(Rc::new("Browse objects by owning scope".to_owned())),
        );
        Grid::set_column(&picker, 1);
        row.children().add(picker.clone().upcast::<Control>());
        Self {
            row,
            picker,
            model: RefCell::default(),
        }
    }

    pub(super) fn control(&self) -> &Ref<Grid> {
        &self.row
    }
    pub(super) fn picker(&self) -> &Ref<ComboBox> {
        &self.picker
    }

    pub(super) fn replace(&self, uri: &str, source: &str, scopes: &[LibraryScope]) -> bool {
        let mut titles = BTreeMap::new();
        for scope in scopes {
            *titles.entry(scope.title.as_str()).or_insert(0usize) += 1;
        }
        let mut lines = None;
        let owners = scopes
            .iter()
            .map(|scope| {
                let duplicate = titles[scope.title.as_str()] > 1;
                let line = duplicate
                    .then_some(scope.owner_range)
                    .flatten()
                    .and_then(|range| {
                        lines
                            .get_or_insert_with(|| LineIndex::new(source))
                            .position(range.start)
                            .ok()
                    });
                let label = match line {
                    Some(line) => format!("{} · line {}", scope.title, line.line + 1),
                    None if duplicate => {
                        let parent = scope
                            .path
                            .strip_suffix(&scope.title)
                            .unwrap_or_default()
                            .trim_end_matches(" › ");
                        format!("{} · {parent}", scope.title)
                    }
                    None => scope.title.clone(),
                };
                (scope.owner, label, scope.path.clone())
            })
            .collect();
        let model = Model {
            uri: uri.to_owned(),
            owners,
            objects: scopes
                .iter()
                .flat_map(|scope| {
                    scope.objects.iter().map(|object| {
                        (
                            object.id.clone(),
                            (scope.owner, object.definition.is_none()),
                        )
                    })
                })
                .collect(),
        };
        if model == *self.model.borrow() {
            return false;
        }
        let selected = if model.uri == self.model.borrow().uri {
            self.selected_owner()
        } else {
            None
        };
        let selected_index =
            selected.and_then(|owner| model.owners.iter().position(|(id, _, _)| *id == owner));
        self.picker.items().clear();
        self.picker
            .items()
            .add(Some(Rc::new("Document and project".to_owned())));
        for (_, label, _) in &model.owners {
            self.picker.items().add(Some(Rc::new(label.clone())));
        }
        self.picker
            .set_selected_index(selected_index.map_or(0, |index| index as i32 + 1));
        let has_scopes = !model.owners.is_empty();
        self.row.set_column_definitions(
            ColumnDefinitions::parse(if has_scopes { "*,*" } else { "*,0" }).unwrap(),
        );
        self.picker.set_is_visible(has_scopes);
        self.model.replace(model);
        true
    }

    pub(super) fn selected_owner(&self) -> Option<NodeId> {
        let index = usize::try_from(self.picker.selected_index())
            .ok()?
            .checked_sub(1)?;
        self.model
            .borrow()
            .owners
            .get(index)
            .map(|(owner, _, _)| *owner)
    }

    pub(super) fn show_owner_path(&self, owner: Option<NodeId>) {
        let model = self.model.borrow();
        let label = owner
            .and_then(|owner| {
                model
                    .owners
                    .iter()
                    .find(|(id, _, _)| *id == owner)
                    .map(|(_, _, path)| path.as_str())
            })
            .unwrap_or("Browse objects by owning scope");
        ToolTip::set_tip(&self.picker, Some(Rc::new(label.to_owned())));
    }

    pub(super) fn contains(&self, id: &str, scope: Option<NodeId>) -> bool {
        if let Some(scope) = scope {
            self.model
                .borrow()
                .objects
                .get(id)
                .is_some_and(|&(owner, _)| owner == scope)
        } else {
            !self.is_source_only(id)
        }
    }

    pub(super) fn is_source_only(&self, id: &str) -> bool {
        self.model
            .borrow()
            .objects
            .get(id)
            .is_some_and(|&(_, source_only)| source_only)
    }
}
