//! Conservative state transfer for named, unambiguous controls in one reload boundary.
use ferroui_base::{BoxedValue, StyledElement, Vector};
#[cfg(test)]
use ferroui_base::Ref;
use ferroui_controls::{primitives::SelectingItemsControl, SelectionMode};
use ferroui_controls::{ScrollViewer, TextBox};
#[cfg(test)]
use ferroui_markup_xaml::FromXamlObject;
use std::collections::BTreeMap;
#[derive(Default)]
pub(crate) struct ViewState(BTreeMap<Vec<String>, Option<Entry>>, Option<String>);
struct Entry {
    source: std::rc::Rc<str>,
    original: ferroui_base::WeakRef<StyledElement>,
    name: String,
    kind: &'static ferroui_base::TypeInfo,
    focused: bool,
    text: Option<TextState>,
    scroll: Option<Vector>,
    selection: Option<SelectionState>,
    custom: Vec<Box<dyn crate::CapturedControlState>>,
}
struct TextState {
    value: Option<String>,
    start: i32,
    end: i32,
    caret: i32,
}
struct SelectionState {
    items: Vec<Option<BoxedValue>>,
    anchor: Option<Option<BoxedValue>>,
    mode: SelectionMode,
}
fn unique_index(items: &[Option<BoxedValue>], value: &Option<BoxedValue>) -> Option<i32> {
    let mut matches = items.iter().enumerate().filter(|(_, item)| *item == value);
    let index = matches.next()?.0;
    if matches.next().is_some() {
        None
    } else {
        i32::try_from(index).ok()
    }
}
impl SelectionState {
    fn capture(control: &SelectingItemsControl) -> Option<Self> {
        let model = control.selection();
        // A caller-owned model is durable application state, not UI state to copy.
        model.as_internal_selection_model()?;
        let items: Vec<_> = control.items_view().iter().collect();
        let selected = model
            .selected_items()
            .iter()
            .filter(|item| unique_index(&items, item).is_some())
            .collect();
        let anchor = usize::try_from(model.anchor_index())
            .ok()
            .and_then(|index| items.get(index))
            .filter(|item| unique_index(&items, item).is_some())
            .cloned();
        Some(Self {
            items: selected,
            anchor,
            mode: control.selection_mode(),
        })
    }
    fn restore(&self, control: &SelectingItemsControl, before: &str, after: &str, old_name: &str, name: &str) {
        let properties = [
            SelectingItemsControl::selected_index_property().as_property(),
            SelectingItemsControl::selected_item_property().as_property(),
            SelectingItemsControl::selected_items_property().as_property(),
            SelectingItemsControl::selection_property().as_property(),
            SelectingItemsControl::selected_value_property().as_property(),
            SelectingItemsControl::selected_value_binding_property().as_property(),
        ];
        for property in properties {
            if control.has_local_value_binding(property)
                || control.is_set(property)
                || authored(after, name, property.name()).is_some()
                || !unchanged_unbound(before, after, old_name, name, property.name())
            {
                return;
            }
        }
        let model = control.selection();
        if model.as_internal_selection_model().is_none()
            || model.count() != 0
            || self.mode != control.selection_mode()
        {
            return;
        }
        let items: Vec<_> = control.items_view().iter().collect();
        let indexes: Vec<_> = self
            .items
            .iter()
            .filter_map(|item| unique_index(&items, item))
            .collect();
        if indexes.is_empty() {
            return;
        }
        struct EndBatch(std::rc::Rc<dyn ferroui_controls::selection::ISelectionModel>);
        impl Drop for EndBatch {
            fn drop(&mut self) {
                self.0.end_batch_update();
            }
        }
        model.begin_batch_update();
        let _batch = EndBatch(model.clone());
        for index in indexes {
            model.select(index);
        }
        if let Some(index) = self
            .anchor
            .as_ref()
            .and_then(|anchor| unique_index(&items, anchor))
        {
            model.set_anchor_index(index);
        }
    }
}
use crate::identity::authored;
pub(crate) fn unchanged_unbound(before: &str, after: &str, old_name: &str, name: &str, property: &str) -> bool {
    let old = authored(before, old_name, property);
    let new = authored(after, name, property);
    old == new && !new.as_deref().is_some_and(|value| value.starts_with('{'))
}
impl ViewState {
    #[cfg(test)]
    pub(crate) fn capture(root: Option<&BoxedValue>, source: &str) -> Self {
        Self::capture_with(root, source, &[])
    }
    #[cfg(test)]
    pub(crate) fn capture_with(
        root: Option<&BoxedValue>,
        source: &str,
        adapters: &[std::rc::Rc<dyn crate::ControlStateAdapter>],
    ) -> Self {
        Self::capture_document(root, None, source, adapters)
    }
    pub(crate) fn capture_document(
        root: Option<&BoxedValue>, uri: Option<&str>, source: &str,
        adapters: &[std::rc::Rc<dyn crate::ControlStateAdapter>],
    ) -> Self {
        let mut state = Self(BTreeMap::new(), uri.map(str::to_owned));
        for crate::state_scope::Control { identity, name, element, source } in root.map(|root| crate::state_scope::controls(root, uri, source)).unwrap_or_default() {
            let entry = Entry {
                source,
                original: element.downgrade(),
                name,
                kind: element.get_type(),
                focused: element
                    .cast::<ferroui_base::input::InputElement>()
                    .is_some_and(|input| input.is_focused()),
                text: element
                    .cast::<TextBox>()
                    .map(|text| TextState {
                        value: text.text(),
                        start: text.selection_start(),
                        end: text.selection_end(),
                        caret: text.caret_index(),
                    }),
                scroll: element.cast::<ScrollViewer>().map(|scroll| scroll.offset()),
                selection: element
                    .cast::<SelectingItemsControl>()
                    .and_then(|control| SelectionState::capture(&control)),
                custom: adapters.iter().filter_map(|adapter| adapter.capture(&element)).collect(),
            };
            state
                .0
                .entry(identity)
                .and_modify(|entry| *entry = None)
                .or_insert(Some(entry));
        }
        state
    }
    pub(crate) fn restore(&self, root: &BoxedValue, _before: &str, after: &str) {
        let elements = crate::state_scope::controls(root, self.1.as_deref(), after);
        let has_new_focus = crate::state_scope::has_focus(root);
        let mut counts = BTreeMap::new();
        for record in &elements {
            *counts.entry(record.identity.clone()).or_insert(0) += 1;
        }
        for crate::state_scope::Control { identity, name, element, source } in elements {
            if counts.get(&identity) != Some(&1) {
                continue;
            }
            let Some(Some(entry)) = self.0.get(&identity) else {
                continue;
            };
            if entry.original.upgrade().is_some_and(|original| original.ptr_eq(&element)) { continue; }
            let before = &*entry.source;
            let after = &*source;
            if !std::ptr::eq(entry.kind, element.get_type()) {
                continue;
            }
            if let (Some(text), Some(saved)) =
                (element.cast::<TextBox>(), &entry.text)
            {
                if unchanged_unbound(before, after, &entry.name, &name, "Text")
                    && !text.is_set(TextBox::text_property())
                    && !text.has_local_value_binding(TextBox::text_property())
                {
                    text.set_text(saved.value.as_deref());
                }
                // Caret and selection setters update each other. Admit the entire group
                // before writing any member, so neither authored changes (including
                // removal) nor a two-way binding on another member can be overwritten.
                let cursor_properties = [
                    TextBox::caret_index_property(),
                    TextBox::selection_start_property(),
                    TextBox::selection_end_property(),
                ];
                if text.text() == saved.value
                    && cursor_properties.iter().all(|property| {
                        !text.is_set(*property)
                            && !text.has_local_value_binding(property)
                            && unchanged_unbound(before, after, &entry.name, &name, property.name())
                    })
                {
                    // Setting the caret collapses selection. Set it first, then restore
                    // the endpoints in their original order, including backwards ranges.
                    text.set_caret_index(saved.caret);
                    text.set_selection_start(saved.start);
                    text.set_selection_end(saved.end);
                }
            }
            if let (Some(scroll), Some(offset)) = (element.cast::<ScrollViewer>(), entry.scroll) {
                if !scroll.is_set(ScrollViewer::offset_property())
                    && !scroll.has_local_value_binding(ScrollViewer::offset_property())
                    && unchanged_unbound(before, after, &entry.name, &name, "Offset")
                {
                    scroll.set_offset(offset);
                }
            }
            if entry.focused && !has_new_focus {
                if let Some(input) = element.cast::<ferroui_base::input::InputElement>() {
                    input.focus();
                }
            }
            if let (Some(control), Some(selection)) =
                (element.cast::<SelectingItemsControl>(), &entry.selection)
            {
                selection.restore(&control, before, after, &entry.name, &name);
            }
            let context = crate::ControlStateRestoreContext {
                control: &element,
                identity: &identity,
                before,
                after,
                before_name: &entry.name,
                name: &name,
            };
            for custom in &entry.custom {
                custom.restore(&element, &context);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ferroui_base::threading::Dispatcher;
    use ferroui_markup_xaml::RuntimeXamlLoaderDocument;
    const LIST_SOURCE: &str = r#"<ListBox xmlns="https://github.com/ferroui" Name="choices"/>"#;
    fn list(items: &[&str], multiple: bool) -> (BoxedValue, Ref<ferroui_controls::ListBox>) {
        ferroui_controls::register_types();
        let control = ferroui_controls::ListBox::new();
        control.set_name(Some("choices".into()));
        if multiple {
            control.set_selection_mode(SelectionMode::MULTIPLE);
        }
        for item in items {
            control
                .items()
                .add(Some(std::rc::Rc::new(item.to_string())));
        }
        (std::rc::Rc::new(control.clone()), control)
    }
    #[test]
    fn selection_follows_unique_items_across_reordering() {
        let _scope = Dispatcher::unit_test_scope();
        let (before, old) = list(&["a", "b", "c"], true);
        old.selection().select(0);
        old.selection().select(2);
        old.selection().set_anchor_index(2);
        let state = ViewState::capture(Some(&before), LIST_SOURCE);
        let (after, new) = list(&["c", "b", "a"], true);
        state.restore(&after, LIST_SOURCE, LIST_SOURCE);
        assert_eq!(new.selection().selected_indexes().to_vec(), vec![0, 2]);
        assert_eq!(new.selection().anchor_index(), 0);
    }
    #[test]
    fn ambiguous_or_removed_selected_items_are_not_restored_by_index() {
        let _scope = Dispatcher::unit_test_scope();
        let (before, old) = list(&["a", "b"], false);
        old.set_selected_index(1);
        let state = ViewState::capture(Some(&before), LIST_SOURCE);
        for items in [&["b", "b"][..], &["a", "c"][..]] {
            let (after, new) = list(items, false);
            state.restore(&after, LIST_SOURCE, LIST_SOURCE);
            assert_eq!(new.selected_index(), -1);
        }
    }
    #[test]
    fn authored_selection_and_removal_override_restored_selection() {
        let _scope = Dispatcher::unit_test_scope();
        let authored = LIST_SOURCE.replace("/>", " SelectedIndex=\"-1\"/>");
        for (before_source, after_source) in [
            (LIST_SOURCE, authored.as_str()),
            (authored.as_str(), LIST_SOURCE),
        ] {
            let (before, old) = list(&["a", "b"], false);
            old.set_selected_index(1);
            let state = ViewState::capture(Some(&before), before_source);
            let (after, new) = list(&["a", "b"], false);
            state.restore(&after, before_source, after_source);
            assert_eq!(new.selected_index(), -1);
        }
    }
    #[test]
    fn direct_two_way_selection_binding_is_never_written_by_restoration() {
        use ferroui_base::data::{BindingMode, IndexerBinding};
        let _scope = Dispatcher::unit_test_scope();
        let (before, old) = list(&["a", "b"], false);
        old.set_selected_index(1);
        let state = ViewState::capture(Some(&before), LIST_SOURCE);
        let (after, new) = list(&["a", "b"], false);
        let (_, model) = list(&["a", "b"], false);
        let property = SelectingItemsControl::selected_index_property().as_property();
        let binding = IndexerBinding::new(model.clone().upcast(), property, BindingMode::TwoWay);
        let expression = new.bind_binding(property, &binding);
        assert!(new.has_local_value_binding(property));
        state.restore(&after, LIST_SOURCE, LIST_SOURCE);
        assert_eq!(new.selected_index(), -1);
        assert_eq!(model.selected_index(), -1);
        new.set_selected_index(1);
        assert_eq!(model.selected_index(), 1);
        expression.dispose();
        assert!(!new.has_local_value_binding(property));
    }
    fn source(attributes: &str) -> String {
        format!(
            r#"<TextBox xmlns="https://github.com/ferroui"
          xmlns:hr="https://github.com/ferroui/hot-reload"
          xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006"
          mc:Ignorable="hr" hr:Key="draft" {attributes}/ >"#
        )
        .replace("/ >", "/>")
    }
    fn load(source: &str) -> (BoxedValue, Ref<TextBox>) {
        ferroui_controls::register_types();
        ferroui_markup_xaml::register_types();
        let root = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
            RuntimeXamlLoaderDocument::with_base_uri(None, source),
            None,
        )
        .unwrap();
        let control = Ref::<TextBox>::from_xaml_object(&root).unwrap();
        (root, control)
    }
    #[test]
    fn initial_development_keys_require_literals() {
        let _scope = Dispatcher::unit_test_scope();
        ferroui_controls::register_types(); ferroui_markup_xaml::register_types();
        for key in ["", "{Binding Text}"] {
            let source = source("").replace("hr:Key=\"draft\"", &format!("hr:Key=\"{key}\""));
            let result = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
                RuntimeXamlLoaderDocument::new(&source), None);
            assert!(result.is_err(), "key {key:?} must fail before population");
        }
    }
    #[test]
    fn duplicate_unnamed_keys_are_ambiguous_without_duplicate_public_names() {
        let _scope = Dispatcher::unit_test_scope();
        ferroui_controls::register_types(); ferroui_markup_xaml::register_types();
        let source = r#"<StackPanel xmlns="https://github.com/ferroui" xmlns:hr="https://github.com/ferroui/hot-reload"><TextBox hr:Key="same"/><TextBox hr:Key="same"/></StackPanel>"#;
        let load = || {
            let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(RuntimeXamlLoaderDocument::new(source), None).unwrap();
            let panel = Ref::<ferroui_controls::StackPanel>::from_xaml_object(&value).unwrap();
            (value, panel.children().to_vec().iter().map(|control| control.cast::<TextBox>().unwrap()).collect::<Vec<_>>())
        };
        let (old, controls) = load();
        controls[0].set_text(Some("left")); controls[1].set_text(Some("right"));
        let state = ViewState::capture(Some(&old), source);
        let (new, controls) = load(); state.restore(&new, source, source);
        assert!(controls.iter().all(|control| control.name().is_none() && control.text().is_none()));
    }
    #[test]
    fn development_key_precedes_an_application_assigned_name() {
        let _scope = Dispatcher::unit_test_scope();
        let source = source("").replace("hr:Key=\"draft\"", "hr:Key=\"{}draft\"");
        let (before, old) = load(&source);
        old.set_name(Some("application-old".into())); old.set_text(Some("draft"));
        let state = ViewState::capture(Some(&before), &source);
        let (after, new) = load(&source);
        new.set_name(Some("application-new".into()));
        state.restore(&after, &source, &source);
        assert_eq!(new.text().as_deref(), Some("draft"));
        assert_eq!(new.name().as_deref(), Some("application-new"));
    }
    #[test]
    fn unnamed_development_key_restores_text_and_selection() {
        let _scope = Dispatcher::unit_test_scope();
        let source = source("");
        let (before, old) = load(&source);
        assert!(old.name().is_none(), "development keys must not become public names");
        old.set_text(Some("draft text"));
        old.set_selection_start(2);
        old.set_selection_end(5);
        let state = ViewState::capture(Some(&before), &source);
        let (after, new) = load(&source);
        state.restore(&after, &source, &source);
        assert_eq!(new.text().as_deref(), Some("draft text"));
        assert_eq!((new.selection_start(), new.selection_end()), (2, 5));
    }
    #[test]
    fn selection_restores_its_active_caret_end() {
        let _scope = Dispatcher::unit_test_scope();
        let source = source("");
        for (start, end, caret) in [(2, 5, 5), (5, 2, 2), (2, 5, 2), (4, 4, 4)] {
            let (before, old) = load(&source);
            old.set_text(Some("draft text"));
            old.set_caret_index(caret);
            old.set_selection_start(start);
            old.set_selection_end(end);
            let state = ViewState::capture(Some(&before), &source);
            let (after, new) = load(&source);
            state.restore(&after, &source, &source);
            assert_eq!(
                (new.selection_start(), new.selection_end(), new.caret_index()),
                (start, end, caret),
            );
        }
    }
    #[test]
    fn removed_authored_caret_does_not_restore_old_selection() {
        let _scope = Dispatcher::unit_test_scope();
        let before_source = source(r#"CaretIndex="4""#);
        let after_source = source("");
        let (before, old) = load(&before_source);
        old.set_text(Some("draft text"));
        old.set_caret_index(5);
        old.set_selection_start(2);
        old.set_selection_end(5);
        let state = ViewState::capture(Some(&before), &before_source);
        let (after, new) = load(&after_source);
        state.restore(&after, &before_source, &after_source);
        assert_eq!(new.text().as_deref(), Some("draft text"));
        assert_eq!(
            (new.selection_start(), new.selection_end(), new.caret_index()),
            (0, 0, 0),
        );
    }
    #[test]
    fn cursor_restoration_never_writes_through_any_two_way_cursor_binding() {
        use ferroui_base::data::{BindingMode, IndexerBinding};
        let _scope = Dispatcher::unit_test_scope();
        let source = source("");
        let (before, old) = load(&source);
        old.set_text(Some("draft text"));
        old.set_caret_index(5);
        old.set_selection_start(2);
        old.set_selection_end(5);
        let state = ViewState::capture(Some(&before), &source);
        for property in [
            TextBox::caret_index_property(),
            TextBox::selection_start_property(),
            TextBox::selection_end_property(),
        ] {
            let (after, new) = load(&source);
            new.set_text(Some("draft text"));
            let model = TextBox::new();
            model.set_text(Some("draft text"));
            model.set_caret_index(1);
            let binding = IndexerBinding::new(
                model.clone().upcast(),
                TextBox::caret_index_property().as_property(),
                BindingMode::TwoWay,
            );
            let expression = new.bind_binding(property.as_property(), &binding);
            let expected = (new.caret_index(), new.selection_start(), new.selection_end());
            state.restore(&after, &source, &source);
            assert_eq!(
                (new.caret_index(), new.selection_start(), new.selection_end()),
                expected,
                "restoration must leave the whole bound cursor group intact",
            );
            assert_eq!(model.caret_index(), 1, "restoration wrote into the model");
            new.set_value(property, 3);
            assert_eq!(model.caret_index(), 3, "the original binding must remain active");
            expression.dispose();
        }
    }
    #[test]
    fn authored_changes_and_removals_take_precedence() {
        let _scope = Dispatcher::unit_test_scope();
        let old_source = source(r#"Text="initial""#);
        let (before, old) = load(&old_source);
        old.set_text(Some("user edit"));
        let state = ViewState::capture(Some(&before), &old_source);
        for attributes in [r#"Text="new markup""#, ""] {
            let new_source = source(attributes);
            let (after, new) = load(&new_source);
            let authored = new.text();
            state.restore(&after, &old_source, &new_source);
            assert_eq!(new.text(), authored);
        }
    }
    #[test]
    fn authored_values_follow_development_keys_when_names_are_exchanged() {
        use ferroui_controls::StackPanel;
        let _scope = Dispatcher::unit_test_scope();
        ferroui_controls::register_types();
        ferroui_markup_xaml::register_types();
        let before_source = r#"<StackPanel xmlns="https://github.com/ferroui"
            xmlns:hr="https://github.com/ferroui/hot-reload"
            xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006" mc:Ignorable="hr">
            <TextBox Name="a" hr:Key="left" Text="initial"/>
            <TextBox Name="b" hr:Key="right"/>
        </StackPanel>"#;
        let after_source = before_source
            .replace(r#"Name="a" hr:Key="left" Text="initial""#, r#"Name="b" hr:Key="left""#)
            .replace(r#"Name="b" hr:Key="right""#, r#"Name="a" hr:Key="right""#);
        assert_eq!(
            crate::DocumentContract::parse(before_source).unwrap(),
            crate::DocumentContract::parse(&after_source).unwrap(),
        );
        let load_pair = |source: &str| {
            let prepared = crate::identity::runtime_source(source).unwrap();
            let value = ferroui_markup_xaml_loader::FerroRuntimeXamlLoader::load_document(
                RuntimeXamlLoaderDocument::new(&prepared), None,
            ).unwrap();
            let panel = Ref::<StackPanel>::from_xaml_object(&value).unwrap();
            let first = panel.children().to_vec()[0].cast::<TextBox>().unwrap();
            (value, first)
        };
        let (before, old) = load_pair(before_source);
        old.set_text(Some("stale input"));
        let state = ViewState::capture(Some(&before), before_source);
        let (after, new) = load_pair(&after_source);
        state.restore(&after, before_source, &after_source);
        assert_eq!(new.text(), None, "removed authored Text belongs to the stable key, not the old name");
    }
    #[test]
    fn restoration_does_not_write_through_two_way_binding() {
        let _scope = Dispatcher::unit_test_scope();
        let old_source = source("");
        let (before, old) = load(&old_source);
        old.set_text(Some("stale UI"));
        let state = ViewState::capture(Some(&before), &old_source);
        let new_source = source(r#"Text="{Binding Text, Mode=TwoWay}""#);
        let (after, new) = load(&new_source);
        let model = TextBox::new();
        model.set_text(Some("model"));
        new.set_data_context(Some(std::rc::Rc::new(model.clone())));
        state.restore(&after, &old_source, &new_source);
        assert_eq!(model.text().as_deref(), Some("model"));
        assert_eq!(new.text().as_deref(), Some("model"));
        new.set_text(Some("fresh edit"));
        assert_eq!(model.text().as_deref(), Some("fresh edit"));
    }
    #[test]
    fn identical_names_in_distinct_named_scopes_do_not_cross() {
        use ferroui_base::controls::{NameScope, NameScopeRef};
        use ferroui_controls::StackPanel;
        let _scope = Dispatcher::unit_test_scope();
        fn tree(reverse: bool) -> (BoxedValue, Ref<TextBox>, Ref<TextBox>) {
            let root = StackPanel::new();
            let mut texts = Vec::new();
            let mut panels = Vec::new();
            for name in ["left", "right"] {
                let panel = StackPanel::new();
                panel.set_name(Some(name.into()));
                NameScope::set_name_scope(
                    &panel,
                    Some(NameScopeRef(std::rc::Rc::new(NameScope::new()))),
                );
                let text = TextBox::new();
                text.set_name(Some("edit".into()));
                panel
                    .children()
                    .add(text.clone().upcast::<ferroui_controls::Control>());
                texts.push(text);
                panels.push(panel);
            }
            if reverse {
                panels.reverse();
            }
            for panel in panels {
                root.children()
                    .add(panel.upcast::<ferroui_controls::Control>());
            }
            (std::rc::Rc::new(root), texts[0].clone(), texts[1].clone())
        }
        let source = r#"<StackPanel><StackPanel Name="left"><TextBox Name="edit"/></StackPanel><StackPanel Name="right"><TextBox Name="edit"/></StackPanel></StackPanel>"#;
        let (before, left, right) = tree(false);
        left.set_text(Some("left draft"));
        right.set_text(Some("right draft"));
        let state = ViewState::capture(Some(&before), source);
        let (after, left, right) = tree(true);
        state.restore(&after, source, source);
        assert_eq!(left.text().as_deref(), Some("left draft"));
        assert_eq!(right.text().as_deref(), Some("right draft"));
    }
}
