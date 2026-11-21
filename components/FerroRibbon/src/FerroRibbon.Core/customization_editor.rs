//! State-editing algorithms from RibbonCustomizeDialog.cs. Copyright Wiesław Šoltés, MIT.
//! Native UI assembly stays in the controls crate; this editor has no UI dependency.
use crate::collections::RibbonItems;
use crate::{
    RibbonQuickAccessPosition, RibbonString,
    state::{
        RibbonCustomGroup, RibbonCustomTab, RibbonCustomization, RibbonState, RibbonStateGroups,
        RibbonStateStrings,
    },
};
use std::{collections::HashSet, rc::Rc};

#[derive(Clone)]
pub struct RibbonCustomizationCommand {
    pub id: RibbonString,
    pub label: Option<RibbonString>,
    pub path: Option<RibbonString>,
    pub icon: Option<crate::model::RibbonObject>,
}
#[derive(Clone)]
pub struct RibbonCustomizationGroup {
    pub id: Option<RibbonString>,
    pub header: Option<RibbonString>,
    pub original_header: Option<RibbonString>,
}
#[derive(Clone)]
pub struct RibbonCustomizationTab {
    pub id: Option<RibbonString>,
    pub header: Option<RibbonString>,
    pub original_header: Option<RibbonString>,
    pub is_custom: bool,
    pub is_contextual: bool,
    pub groups: Vec<RibbonCustomizationGroup>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RibbonCustomizationRowKind {
    Tab,
    Group,
    Command,
}
impl RibbonCustomizationRowKind {
    pub fn level(self) -> usize {
        match self {
            Self::Tab => 0,
            Self::Group => 1,
            Self::Command => 2,
        }
    }
}
#[derive(Clone, Debug)]
pub struct RibbonCustomizationRow {
    pub id: Option<RibbonString>,
    pub label: Option<RibbonString>,
    pub kind: RibbonCustomizationRowKind,
    pub hidden: bool,
    pub is_custom: bool,
    pub parent_id: Option<RibbonString>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RibbonCustomizationEditError {
    MissingField(&'static str),
    MissingBuiltInTab,
    EmptyMove,
    DuplicateCommandId,
    MissingRowPrefix,
}
impl std::fmt::Display for RibbonCustomizationEditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField(name) => write!(f, "Customization state is missing {name}"),
            Self::MissingBuiltInTab => {
                f.write_str("Customization row has no matching built-in tab")
            }
            Self::DuplicateCommandId => f.write_str("Command catalog contains duplicate IDs"),
            Self::EmptyMove => f.write_str("Cannot compute a move target in an empty list"),
            Self::MissingRowPrefix => f.write_str("Command row has no valid group prefix"),
        }
    }
}
impl std::error::Error for RibbonCustomizationEditError {}
type EditResult<T> = Result<T, RibbonCustomizationEditError>;
fn required<T>(value: Option<T>, field: &'static str) -> EditResult<T> {
    value.ok_or(RibbonCustomizationEditError::MissingField(field))
}
fn joined(parts: &[Option<RibbonString>]) -> RibbonString {
    RibbonString::from_utf16(
        parts
            .iter()
            .flatten()
            .flat_map(|s| s.as_utf16().iter().copied())
            .collect::<Vec<_>>(),
    )
}

#[derive(Clone)]
pub struct RibbonCustomizationListRow {
    pub id: Option<RibbonString>,
    pub label: Option<RibbonString>,
    pub detail: Option<RibbonString>,
    pub icon: Option<crate::model::RibbonObject>,
}

/// Working state is the same shared object exposed by the dialog. Rebuilding rows
/// creates fresh row identities and restores selection by the first matching ID.
pub struct RibbonCustomizationEditor {
    working: Rc<RibbonState>,
    tabs: Vec<RibbonCustomizationTab>,
    commands: Vec<RibbonCustomizationCommand>,
    suffix: Option<RibbonString>,
    rows: Vec<Rc<RibbonCustomizationRow>>,
    selected: Option<Rc<RibbonCustomizationRow>>,
}
impl RibbonCustomizationEditor {
    pub fn new(
        working: Rc<RibbonState>,
        tabs: Vec<RibbonCustomizationTab>,
        commands: Vec<RibbonCustomizationCommand>,
        custom_suffix: Option<RibbonString>,
    ) -> EditResult<Self> {
        if working.quick_access_item_ids().is_none() {
            working.set_quick_access_item_ids(Some(RibbonItems::new()));
        }
        let mut editor = Self {
            working,
            tabs,
            commands,
            suffix: custom_suffix,
            rows: Vec::new(),
            selected: None,
        };
        editor.refresh()?;
        Ok(editor)
    }
    /// The control refreshes these source UI snapshots before editing actions.
    pub fn set_tabs(&mut self, tabs: Vec<RibbonCustomizationTab>) {
        self.tabs = tabs;
    }
    pub fn set_custom_suffix(&mut self, suffix: Option<RibbonString>) {
        self.suffix = suffix;
    }
    pub fn quick_access_rows(
        &self,
        own: &[RibbonCustomizationCommand],
    ) -> EditResult<Vec<RibbonCustomizationListRow>> {
        let ids = required(self.working.quick_access_item_ids(), "QuickAccessItemIds")?;
        let mut lookup = std::collections::HashMap::new();
        for command in &self.commands {
            if lookup.insert(&command.id, command).is_some() {
                return Err(RibbonCustomizationEditError::DuplicateCommandId);
            }
        }
        Ok(ids
            .iter()
            .map(|id| {
                if let Some(command) = id.as_ref().and_then(|id| lookup.get(id)) {
                    RibbonCustomizationListRow {
                        id,
                        label: command.label.clone(),
                        detail: command.path.clone(),
                        icon: command.icon.clone(),
                    }
                } else {
                    let item = own.iter().find(|item| Some(&item.id) == id.as_ref());
                    RibbonCustomizationListRow {
                        label: item
                            .and_then(|item| item.label.clone())
                            .or_else(|| id.clone()),
                        icon: item.and_then(|item| item.icon.clone()),
                        id,
                        detail: Some("".into()),
                    }
                }
            })
            .collect())
    }
    /// Source import replaces only customization/QAT IDs/position. Other working
    /// state remains unchanged, including the QAT visibility flag.
    pub fn import_fields(&self, imported: &RibbonState) {
        self.working.set_customization(imported.customization());
        if let Some(ids) = imported.quick_access_item_ids() {
            self.working.set_quick_access_item_ids(Some(ids));
        }
        self.working
            .set_quick_access_position(imported.quick_access_position());
    }
    pub fn working_state(&self) -> Rc<RibbonState> {
        self.working.clone()
    }
    pub fn commands(&self) -> &[RibbonCustomizationCommand] {
        &self.commands
    }
    pub fn rows(&self) -> &[Rc<RibbonCustomizationRow>] {
        &self.rows
    }
    pub fn selected(&self) -> Option<Rc<RibbonCustomizationRow>> {
        self.selected.clone()
    }
    pub fn select(&mut self, id: Option<&RibbonString>) {
        self.selected = self.rows.iter().find(|r| r.id.as_ref() == id).cloned();
    }
    fn customization(&self) -> EditResult<Rc<RibbonCustomization>> {
        required(self.working.customization(), "Customization")
    }
    fn groups(&self) -> EditResult<RibbonStateGroups> {
        required(self.customization()?.custom_groups(), "CustomGroups")
    }
    fn custom_tabs(&self) -> EditResult<crate::state::RibbonStateTabs> {
        required(self.customization()?.custom_tabs(), "CustomTabs")
    }
    pub fn refresh(&mut self) -> EditResult<()> {
        let selected = self.selected.as_ref().and_then(|r| r.id.clone());
        self.rows.clear();
        self.selected = None;
        let c = self.customization()?;
        let custom_tabs = required(c.custom_tabs(), "CustomTabs")?;
        let custom_groups = required(c.custom_groups(), "CustomGroups")?;
        let hidden_tabs = required(c.hidden_tab_ids(), "HiddenTabIds")?;
        let hidden_groups = required(c.hidden_group_ids(), "HiddenGroupIds")?;
        let labels = required(c.labels(), "Labels")?;
        let order = required(c.tab_order(), "TabOrder")?;
        let group_order = required(c.group_order(), "GroupOrder")?;
        let mut custom_group_ids = HashSet::new();
        for group in custom_groups.iter() {
            custom_group_ids.insert(required(group, "CustomGroup")?.id());
        }
        for tab in custom_tabs.iter() {
            for group in required(required(tab, "CustomTab")?.groups(), "CustomTab.Groups")?.iter()
            {
                custom_group_ids.insert(required(group, "CustomTab.Group")?.id());
            }
        }
        let tabs = self
            .tabs
            .iter()
            .filter(|t| !t.is_custom && !t.is_contextual && t.id.is_some())
            .cloned()
            .collect::<Vec<_>>();
        let mut ids = tabs
            .iter()
            .map(|t| t.id.clone())
            .chain(custom_tabs.iter().map(|t| t.and_then(|t| t.id())))
            .collect::<Vec<_>>();
        if !order.is_empty() {
            ids.sort_by_key(|id| {
                order
                    .iter()
                    .position(|other| &other == id)
                    .unwrap_or(usize::MAX)
            });
        }
        for id in ids {
            let custom = custom_tabs.iter().find_map(|t| t.filter(|t| t.id() == id));
            if let Some(tab) = custom {
                self.rows.push(Rc::new(RibbonCustomizationRow {
                    id: tab.id(),
                    label: Some(joined(&[
                        tab.label(),
                        Some(" ".into()),
                        self.suffix.clone(),
                    ])),
                    kind: RibbonCustomizationRowKind::Tab,
                    hidden: hidden_tabs.iter().any(|x| x == tab.id()),
                    is_custom: true,
                    parent_id: None,
                }));
                for group in required(tab.groups(), "CustomTab.Groups")?.iter() {
                    let group = required(group, "CustomTab.Group")?;
                    self.push_custom_group(&group, tab.id(), false, &hidden_groups)?;
                }
                continue;
            }
            let tab = tabs
                .iter()
                .find(|t| t.id == id)
                .ok_or(RibbonCustomizationEditError::MissingBuiltInTab)?;
            let key = required(id.clone(), "BuiltInTab.Id")?;
            self.rows.push(Rc::new(RibbonCustomizationRow {
                id: id.clone(),
                label: labels.get(&key).unwrap_or_else(|| {
                    tab.original_header
                        .clone()
                        .or_else(|| tab.header.clone())
                        .or_else(|| id.clone())
                }),
                kind: RibbonCustomizationRowKind::Tab,
                hidden: hidden_tabs.iter().any(|x| x == id),
                is_custom: false,
                parent_id: None,
            }));
            let mut groups = tab
                .groups
                .iter()
                .filter(|g| g.id.is_some() && !custom_group_ids.contains(&g.id))
                .cloned()
                .collect::<Vec<_>>();
            if let Some(group_ids) = group_order.get(&key) {
                let group_ids = required(group_ids, "GroupOrder.Value")?;
                if !group_ids.is_empty() {
                    groups.sort_by_key(|g| {
                        group_ids
                            .iter()
                            .position(|id| id == g.id)
                            .unwrap_or(usize::MAX)
                    });
                }
            }
            for group in groups {
                let key = required(group.id.clone(), "BuiltInGroup.Id")?;
                self.rows.push(Rc::new(RibbonCustomizationRow {
                    id: group.id.clone(),
                    label: labels.get(&key).unwrap_or_else(|| {
                        group
                            .original_header
                            .clone()
                            .or_else(|| group.header.clone())
                            .or_else(|| group.id.clone())
                    }),
                    kind: RibbonCustomizationRowKind::Group,
                    hidden: hidden_groups.iter().any(|x| x == group.id),
                    is_custom: false,
                    parent_id: id.clone(),
                }));
            }
            for group in custom_groups.iter() {
                let group = required(group, "CustomGroup")?;
                if group.tab_id() == id {
                    self.push_custom_group(&group, id.clone(), true, &hidden_groups)?;
                }
            }
        }
        if let Some(id) = selected {
            self.select(Some(&id));
        }
        Ok(())
    }
    fn push_custom_group(
        &mut self,
        group: &RibbonCustomGroup,
        tab_id: Option<RibbonString>,
        suffix: bool,
        hidden: &RibbonStateStrings,
    ) -> EditResult<()> {
        let id = group.id();
        let label = if suffix {
            Some(joined(&[
                group.label(),
                Some(" ".into()),
                self.suffix.clone(),
            ]))
        } else {
            group.label()
        };
        self.rows.push(Rc::new(RibbonCustomizationRow {
            id: id.clone(),
            label,
            kind: RibbonCustomizationRowKind::Group,
            hidden: hidden.iter().any(|x| x == id),
            is_custom: true,
            parent_id: tab_id,
        }));
        for item in required(group.item_ids(), "CustomGroup.ItemIds")?.iter() {
            let label = self
                .commands
                .iter()
                .find(|c| Some(&c.id) == item.as_ref())
                .and_then(|c| c.label.clone())
                .or_else(|| item.clone());
            self.rows.push(Rc::new(RibbonCustomizationRow {
                id: Some(joined(&[id.clone(), Some("/".into()), item])),
                label,
                kind: RibbonCustomizationRowKind::Command,
                hidden: false,
                is_custom: true,
                parent_id: id.clone(),
            }));
        }
        Ok(())
    }
    pub fn set_visible(&mut self, id: Option<&RibbonString>, visible: bool) -> EditResult<()> {
        let Some(row) = self.rows.iter().find(|row| row.id.as_ref() == id).cloned() else {
            return Ok(());
        };
        if row.kind == RibbonCustomizationRowKind::Command {
            return Ok(());
        }
        let c = self.customization()?;
        let list = if row.kind == RibbonCustomizationRowKind::Tab {
            required(c.hidden_tab_ids(), "HiddenTabIds")?
        } else {
            required(c.hidden_group_ids(), "HiddenGroupIds")?
        };
        list.remove(&row.id);
        if !visible {
            list.add(row.id.clone());
        }
        Ok(())
    }
    fn find_group(&self, id: Option<&RibbonString>) -> EditResult<Option<Rc<RibbonCustomGroup>>> {
        for group in self.groups()?.iter() {
            let group = required(group, "CustomGroup")?;
            if group.id().as_ref() == id {
                return Ok(Some(group));
            }
        }
        for tab in self.custom_tabs()?.iter() {
            for group in required(required(tab, "CustomTab")?.groups(), "CustomTab.Groups")?.iter()
            {
                let group = required(group, "CustomTab.Group")?;
                if group.id().as_ref() == id {
                    return Ok(Some(group));
                }
            }
        }
        Ok(None)
    }
    fn target_group(&self) -> EditResult<Option<Rc<RibbonCustomGroup>>> {
        match self.selected.as_deref() {
            Some(row) if row.kind == RibbonCustomizationRowKind::Group && row.is_custom => {
                self.find_group(row.id.as_ref())
            }
            Some(row) if row.kind == RibbonCustomizationRowKind::Command => {
                self.find_group(row.parent_id.as_ref())
            }
            _ => Ok(None),
        }
    }
    pub fn add_command(
        &mut self,
        id: &RibbonString,
        new_group_id: RibbonString,
        new_group_label: Option<RibbonString>,
        selected_tab: Option<RibbonString>,
    ) -> EditResult<()> {
        let mut group = self.target_group()?;
        if group.is_none() {
            self.new_group(new_group_id, new_group_label, selected_tab)?;
            group = self.target_group()?;
        }
        if let Some(group) = group {
            let items = required(group.item_ids(), "CustomGroup.ItemIds")?;
            if !items.iter().any(|item| item.as_ref() == Some(id)) {
                items.add(Some(id.clone()));
                self.refresh()?;
            }
        }
        Ok(())
    }
    pub fn remove_selected(&mut self) -> EditResult<()> {
        let Some(row) = self.selected.clone() else {
            return Ok(());
        };
        if row.kind == RibbonCustomizationRowKind::Command {
            if let Some(group) = self.find_group(row.parent_id.as_ref())? {
                required(group.item_ids(), "CustomGroup.ItemIds")?.remove(&Some(command_id(&row)?));
            }
        } else if row.kind == RibbonCustomizationRowKind::Group && row.is_custom {
            self.groups()?
                .remove_all_state(|g| g.as_ref().is_some_and(|g| g.id() == row.id));
            for tab in self.custom_tabs()?.iter() {
                required(required(tab, "CustomTab")?.groups(), "CustomTab.Groups")?
                    .remove_all_state(|g| g.as_ref().is_some_and(|g| g.id() == row.id));
            }
        } else if row.kind == RibbonCustomizationRowKind::Tab && row.is_custom {
            self.custom_tabs()?
                .remove_all_state(|t| t.as_ref().is_some_and(|t| t.id() == row.id));
            required(self.customization()?.tab_order(), "TabOrder")?.remove(&row.id);
        }
        self.refresh()
    }
    /// IDs are supplied by the native dialog's UUID generator; fixtures can use
    /// deterministic IDs without replacing any source algorithm.
    pub fn new_tab(
        &mut self,
        id: RibbonString,
        label: Option<RibbonString>,
        group_id: RibbonString,
        group_label: Option<RibbonString>,
    ) -> EditResult<()> {
        let tab = RibbonCustomTab::new();
        tab.set_id(Some(id));
        tab.set_label(label);
        let group = RibbonCustomGroup::new();
        group.set_id(Some(group_id.clone()));
        group.set_label(group_label);
        required(tab.groups(), "CustomTab.Groups")?.add(Some(group));
        self.custom_tabs()?.add(Some(tab));
        self.refresh()?;
        self.select(Some(&group_id));
        Ok(())
    }
    pub fn new_group(
        &mut self,
        id: RibbonString,
        label: Option<RibbonString>,
        selected_tab: Option<RibbonString>,
    ) -> EditResult<()> {
        let tab_id = match self.selected.as_deref() {
            None => selected_tab,
            Some(row) if row.kind == RibbonCustomizationRowKind::Tab => row.id.clone(),
            Some(row) if row.kind == RibbonCustomizationRowKind::Group => row.parent_id.clone(),
            Some(row) => self
                .rows
                .iter()
                .find(|r| r.id == row.parent_id)
                .and_then(|r| r.parent_id.clone()),
        };
        let Some(tab_id) = tab_id else {
            return Ok(());
        };
        let group = RibbonCustomGroup::new();
        group.set_id(Some(id.clone()));
        group.set_label(label);
        group.set_tab_id(Some(tab_id.clone()));
        let tab = self
            .custom_tabs()?
            .iter()
            .find_map(|t| t.filter(|t| t.id().as_ref() == Some(&tab_id)));
        if let Some(tab) = tab {
            group.set_tab_id(None);
            required(tab.groups(), "CustomTab.Groups")?.add(Some(group));
        } else {
            self.groups()?.add(Some(group));
        }
        self.refresh()?;
        self.select(Some(&id));
        Ok(())
    }
    pub fn rename_selected(&mut self, label: RibbonString) -> EditResult<()> {
        let Some(row) = self.selected.clone() else {
            return Ok(());
        };
        self.rename_row(&row, label)
    }
    /// Rename retains the row captured before a nested dialog, even if the
    /// current selection changes while the native parent dialog is hidden.
    pub fn rename_row(
        &mut self,
        row: &RibbonCustomizationRow,
        label: RibbonString,
    ) -> EditResult<()> {
        if row.kind == RibbonCustomizationRowKind::Command {
            return Ok(());
        }
        if label.as_utf16().iter().all(|c| crate::text::whitespace(*c)) {
            return Ok(());
        }
        if row.is_custom {
            if let Some(tab) = self
                .custom_tabs()?
                .iter()
                .find_map(|t| t.filter(|t| t.id() == row.id))
            {
                tab.set_label(Some(label));
            } else if let Some(group) = self.find_group(row.id.as_ref())? {
                group.set_label(Some(label));
            }
        } else {
            required(self.customization()?.labels(), "Labels")?
                .insert(required(row.id.clone(), "Row.Id")?, Some(label));
        }
        self.refresh()
    }
    pub fn move_selected(&mut self, delta: i32) -> EditResult<()> {
        let Some(row) = self.selected.clone() else {
            return Ok(());
        };
        match row.kind {
            RibbonCustomizationRowKind::Tab => {
                let ids = RibbonItems::from_values(
                    self.rows
                        .iter()
                        .filter(|r| r.kind == RibbonCustomizationRowKind::Tab)
                        .map(|r| r.id.clone())
                        .collect(),
                );
                if move_ids(&ids, &row.id, delta)? {
                    self.customization()?.set_tab_order(Some(ids));
                    self.refresh()?;
                    self.select(row.id.as_ref());
                }
                return Ok(());
            }
            RibbonCustomizationRowKind::Command => {
                if let Some(group) = self.find_group(row.parent_id.as_ref())?
                    && move_ids(
                        &required(group.item_ids(), "CustomGroup.ItemIds")?,
                        &Some(command_id(&row)?),
                        delta,
                    )?
                {
                    self.refresh()?;
                    self.select(row.id.as_ref());
                }
                return Ok(());
            }
            RibbonCustomizationRowKind::Group => {}
        }
        if row.is_custom {
            self.move_custom_group(&row, delta)?;
        } else if let Some(tab_id) = row.parent_id.clone() {
            let ids = RibbonItems::from_values(
                self.rows
                    .iter()
                    .filter(|r| {
                        r.kind == RibbonCustomizationRowKind::Group
                            && !r.is_custom
                            && r.parent_id.as_ref() == Some(&tab_id)
                    })
                    .map(|r| r.id.clone())
                    .collect(),
            );
            if move_ids(&ids, &row.id, delta)? {
                required(self.customization()?.group_order(), "GroupOrder")?
                    .insert(tab_id, Some(ids));
                self.refresh()?;
                self.select(row.id.as_ref());
            }
        }
        Ok(())
    }
    fn move_custom_group(&mut self, row: &RibbonCustomizationRow, delta: i32) -> EditResult<()> {
        let mut containing_tab = None;
        for tab in self.custom_tabs()?.iter() {
            let tab = required(tab, "CustomTab")?;
            let mut contains = false;
            for group in required(tab.groups(), "CustomTab.Groups")?.iter() {
                if required(group, "CustomTab.Group")?.id() == row.id {
                    contains = true;
                    break;
                }
            }
            if contains {
                containing_tab = Some(tab);
                break;
            }
        }
        if let Some(tab) = containing_tab {
            let groups = required(tab.groups(), "CustomTab.Groups")?;
            let mut ids = Vec::new();
            for group in groups.iter() {
                ids.push(required(group, "CustomTab.Group")?.id());
            }
            let ids = RibbonItems::from_values(ids);
            if move_ids(&ids, &row.id, delta)? {
                let mut reordered = Vec::new();
                for id in ids.iter() {
                    let mut found = None;
                    for group in groups.iter() {
                        let group = required(group, "CustomTab.Group")?;
                        if group.id() == id {
                            found = Some(group);
                            break;
                        }
                    }
                    reordered.push(found);
                }
                tab.set_groups(Some(RibbonItems::from_values(reordered)));
            }
        } else {
            let mut groups = Vec::new();
            for group in self.groups()?.iter() {
                groups.push(required(group, "CustomGroup")?);
            }
            let siblings = groups
                .iter()
                .filter(|g| g.tab_id() == row.parent_id)
                .cloned()
                .collect::<Vec<_>>();
            let ids = RibbonItems::from_values(siblings.iter().map(|g| g.id()).collect());
            if !move_ids(&ids, &row.id, delta)? {
                return Ok(());
            }
            let others = groups
                .iter()
                .filter(|g| g.tab_id() != row.parent_id)
                .cloned()
                .map(Some);
            let reordered = ids
                .iter()
                .map(|id| siblings.iter().find(|g| g.id() == id).cloned());
            self.customization()?
                .set_custom_groups(Some(RibbonItems::from_values(
                    others.chain(reordered).collect(),
                )));
        }
        self.refresh()?;
        self.select(row.id.as_ref());
        Ok(())
    }
    pub fn reset_customization(&mut self) -> EditResult<()> {
        self.working
            .set_customization(Some(RibbonCustomization::new()));
        self.refresh()
    }
    pub fn move_quick_access(&self, id: Option<RibbonString>, delta: i32) -> EditResult<bool> {
        move_ids(
            &required(self.working.quick_access_item_ids(), "QuickAccessItemIds")?,
            &id,
            delta,
        )
    }
    pub fn add_quick_access(&self, id: RibbonString) -> EditResult<bool> {
        let ids = required(self.working.quick_access_item_ids(), "QuickAccessItemIds")?;
        if ids.iter().any(|i| i.as_ref() == Some(&id)) {
            return Ok(false);
        }
        ids.add(Some(id));
        Ok(true)
    }
    pub fn remove_quick_access(&self, id: Option<RibbonString>) -> EditResult<bool> {
        Ok(required(self.working.quick_access_item_ids(), "QuickAccessItemIds")?.remove(&id))
    }
    pub fn reset_quick_access(
        &self,
        defaults: impl IntoIterator<Item = Option<RibbonString>>,
        position: RibbonQuickAccessPosition,
    ) {
        self.working
            .set_quick_access_item_ids(Some(RibbonItems::from_values(
                defaults.into_iter().collect(),
            )));
        self.working.set_quick_access_position(position);
        self.working.set_is_quick_access_visible(true);
    }
}
fn command_id(row: &RibbonCustomizationRow) -> EditResult<RibbonString> {
    let parent = required(row.parent_id.clone(), "Row.GroupId")?;
    let id = required(row.id.clone(), "Row.Id")?;
    let offset = parent.len() + 1;
    let units = id
        .as_utf16()
        .get(offset..)
        .ok_or(RibbonCustomizationEditError::MissingRowPrefix)?;
    Ok(RibbonString::from_utf16(units))
}
fn move_ids(ids: &RibbonStateStrings, id: &Option<RibbonString>, delta: i32) -> EditResult<bool> {
    let index = ids.iter().position(|x| &x == id).map_or(-1, |i| i as i32);
    let max = i32::try_from(ids.len())
        .map_err(|_| RibbonCustomizationEditError::EmptyMove)?
        .wrapping_sub(1);
    if max < 0 {
        return Err(RibbonCustomizationEditError::EmptyMove);
    }
    let target = index.wrapping_add(delta).clamp(0, max);
    if index < 0 || target == index {
        return Ok(false);
    }
    ids.remove_at(index as usize);
    ids.insert(target as usize, id.clone());
    Ok(true)
}
