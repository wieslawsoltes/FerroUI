//! Shared Ribbon.Search.cs. Copyright Wiesław Šoltés, MIT.
use ferroribbon_core::collections::RibbonFixedItems;
use ferroribbon_core::collections::RibbonItems;
use ferroui_base::{
    AttachedProperty, FerroObject, FerroProperty, Ref, ferro_properties, ferro_static_type,
};
pub struct RibbonSearch;
ferro_static_type!(RibbonSearch);
ferro_properties! {impl RibbonSearch{
 pub fn keywords_property()->AttachedProperty<Option<String>>{FerroProperty::register_attached::<Self,FerroObject,_>("Keywords",None)}
 pub fn is_searchable_property()->AttachedProperty<bool>{FerroProperty::register_attached::<Self,FerroObject,_>("IsSearchable",true)}
}}
impl RibbonSearch {
    pub fn get_keywords(element: &FerroObject) -> Option<String> {
        element.get_value(Self::keywords_property())
    }
    pub fn set_keywords(element: &FerroObject, value: Option<String>) {
        element.set_value(Self::keywords_property(), value)
    }
    pub fn get_is_searchable(element: &FerroObject) -> bool {
        element.get_value(Self::is_searchable_property())
    }
    pub fn set_is_searchable(element: &FerroObject, value: bool) {
        element.set_value(Self::is_searchable_property(), value)
    }
}

ferroui_base::ferro_markup_type!(static RibbonSearch{
 type_info:RibbonSearch,namespace:"FerroRibbon.Controls",methods:[
 static fn GetKeywords(Ref<FerroObject>)->Option<String> =>|e:Ref<FerroObject>|RibbonSearch::get_keywords(&e),
 static fn SetKeywords(Ref<FerroObject>,Option<String>)=>|e:Ref<FerroObject>,v|RibbonSearch::set_keywords(&e,v),
 static fn GetIsSearchable(Ref<FerroObject>)->bool=>|e:Ref<FerroObject>|RibbonSearch::get_is_searchable(&e),
 static fn SetIsSearchable(Ref<FerroObject>,bool)=>|e:Ref<FerroObject>,v|RibbonSearch::set_is_searchable(&e,v)
 ]
});

use super::ribbon_item_helper::IRibbonItemInternal;
use super::{
    IRibbonItem, Ribbon, RibbonBackstageItem, RibbonButton, RibbonDropDownButton, RibbonGallery,
    RibbonInputBase, RibbonScreenTip,
};
use ferroribbon_core::{
    RibbonGroupState, RibbonString, RibbonVisibilityMode,
    collections::RibbonReadOnlyList,
    commands::RibbonCommandDescriptor,
    model::RibbonItemsSource,
    search::{RibbonSearchEntry, RibbonSearchEntryOptions, RibbonSearchResults},
    text::{RibbonOrdinalIgnoreCaseKey, trim},
};
use ferroui_base::{
    BoxedValue, PropertyValue, WeakRef,
    data::core::{ValueType, ValueTypes},
    metadata::{MarkupDelegate, SYSTEM_ACTION_DELEGATE},
    threading::{Dispatcher, DispatcherPriority},
};
use ferroui_controls::{Control, TextBox, primitives::TemplatedControl};
use ferroui_system::collections::object_model::ObservableCollection;
use std::{collections::HashSet, rc::Rc};

pub type RibbonAdditionalSearchEntries = ObservableCollection<Option<Rc<RibbonSearchEntry>>>;
pub type RibbonSearchEntries = RibbonReadOnlyList<Option<Rc<RibbonSearchEntry>>>;

fn cast<T: PropertyValue>(value: &BoxedValue) -> Option<T> {
    ValueTypes::try_cast(value, ValueType::of::<T>()).and_then(|v| v.downcast_ref::<T>().cloned())
}
fn native_target(entry: &RibbonSearchEntry) -> Option<BoxedValue> {
    crate::metadata::model_objects::to_native(entry.target()).expect("search target native value")
}
fn concatenate(parts: &[Option<RibbonString>]) -> RibbonString {
    RibbonString::from_utf16(
        parts
            .iter()
            .filter_map(Option::as_ref)
            .flat_map(|s| s.as_utf16().iter().copied())
            .collect::<Vec<_>>(),
    )
}
fn description(tip: Option<BoxedValue>) -> Option<RibbonString> {
    let tip = tip?;
    if let Some(text) = tip.downcast_ref::<String>() {
        return Some(text.as_str().into());
    }
    if let Some(tip) = ValueTypes::as_object(&*tip).and_then(|v| v.cast::<RibbonScreenTip>()) {
        return tip.description().map(Into::into);
    }
    cast::<Rc<ferroribbon_core::model::RibbonScreenTip>>(&tip).and_then(|tip| tip.description())
}
fn keywords(value: Option<String>) -> Option<ferroribbon_core::search::RibbonSearchStrings> {
    value.map(|value| {
        let value = RibbonString::from(value);
        let values = value
            .as_utf16()
            .split(|&c| c == u16::from(b','))
            .filter_map(|s| {
                let s = trim(s);
                (!s.is_empty()).then(|| Some(RibbonString::from(s)))
            })
            .collect();
        RibbonFixedItems::new(values) as RibbonReadOnlyList<_>
    })
}
impl Ribbon {
    pub fn additional_search_entries(&self) -> RibbonAdditionalSearchEntries {
        self.additional_search_entries.clone()
    }
    pub fn build_search_entries(&self) -> RibbonSearchEntries {
        let entries = RibbonItems::new();
        let mut seen = HashSet::new();
        for tab in self.tabs().iter() {
            let tab = tab.expect("NullReference: tab.IsAvailable");
            if !tab.is_available() {
                continue;
            }
            for group in tab.groups().iter() {
                let group = group.expect("NullReference: group.IsShown");
                if !group.is_shown() {
                    continue;
                }
                for element in group.get_all_items().iter() {
                    let Some(item) =
                        cast::<Rc<dyn IRibbonItemInternal>>(&Control::boxed(element.clone()))
                    else {
                        continue;
                    };
                    if item
                        .label()
                        .as_ref()
                        .is_none_or(|s| trim(s.as_utf16()).is_empty())
                        || !RibbonSearch::get_is_searchable(&element)
                        || !element.is_visible()
                    {
                        continue;
                    }
                    let id = item.id().unwrap_or_else(|| {
                        concatenate(&[
                            tab.effective_id().map(Into::into),
                            Some("/".into()),
                            group.effective_id().map(Into::into),
                            Some("/".into()),
                            item.label(),
                        ])
                    });
                    if !seen.insert(Some(id.clone())) {
                        continue;
                    }
                    let description = description(item.screen_tip());
                    let label = item.label().expect("NullReference: Label.Replace");
                    let label = RibbonString::from_utf16(
                        label
                            .as_utf16()
                            .iter()
                            .map(|&u| if u == 10 { 32 } else { u })
                            .collect::<Vec<_>>(),
                    );
                    let path = concatenate(&[
                        tab.header().map(Into::into),
                        Some(" › ".into()),
                        group.header().map(Into::into),
                    ]);
                    entries.add(Some(RibbonSearchEntry::with_options(
                        Some(id),
                        Some(label),
                        RibbonSearchEntryOptions {
                            path: Some(path),
                            description,
                            keywords: keywords(RibbonSearch::get_keywords(&element)),
                            shortcut: item.shortcut(),
                            target: crate::metadata::model_objects::from_native(Some(
                                Control::boxed(element.clone()),
                            )),
                            is_enabled: element
                                .cast::<TemplatedControl>()
                                .is_none_or(|c| c.is_enabled() && c.is_effectively_enabled()),
                        },
                    )));
                }
            }
        }
        if let Some(backstage) = self.backstage() {
            for item in backstage.items().iter() {
                let item = item.expect("NullReference: backstage item.IsVisible");
                if !item.is_visible() || item.header().as_ref().is_none_or(String::is_empty) {
                    continue;
                }
                let id = concatenate(&[
                    Some("backstage/".into()),
                    item.id().or_else(|| item.header()).map(Into::into),
                ]);
                if seen.insert(Some(id.clone())) {
                    entries.add(Some(RibbonSearchEntry::with_options(
                        Some(id),
                        item.header().map(Into::into),
                        RibbonSearchEntryOptions {
                            path: self.application_button_label().map(Into::into).or_else(|| {
                                ferroribbon_core::localization::RibbonStrings::current().file()
                            }),
                            target: crate::metadata::model_objects::from_native(Some(Rc::new(
                                item.clone(),
                            ))),
                            is_enabled: item.is_enabled(),
                            ..Default::default()
                        },
                    )));
                }
            }
        }
        if let Some(catalog) = self.command_catalog() {
            for command in catalog.commands() {
                let represented = entries.iter().any(|entry| {
                    let target = native_target(&entry.expect("NullReference: search entry.Target"));
                    target
                        .as_ref()
                        .and_then(cast::<Rc<dyn IRibbonItem>>)
                        .and_then(|i| i.command_id())
                        .is_some_and(|id| {
                            RibbonOrdinalIgnoreCaseKey(id)
                                == RibbonOrdinalIgnoreCaseKey(command.id())
                        })
                });
                // Source short-circuits: represented commands never reserve this id.
                if represented {
                    continue;
                }
                let id = concatenate(&[Some("command/".into()), Some(command.id())]);
                if !seen.insert(Some(id.clone())) {
                    continue;
                }
                entries.add(Some(RibbonSearchEntry::with_options(
                    Some(id),
                    command.label(),
                    RibbonSearchEntryOptions {
                        path: command.category(),
                        description: command.description(),
                        keywords: Some(RibbonFixedItems::new(command.keywords().borrow().clone())
                            as RibbonReadOnlyList<_>),
                        shortcut: command.shortcut(),
                        target: crate::metadata::model_objects::from_native(Some(Rc::new(
                            command.clone(),
                        ))),
                        is_enabled: command.is_enabled(),
                    },
                )));
            }
        }
        for extra in self.additional_search_entries().iter() {
            let extra = extra.expect("NullReference: extra.Id");
            if seen.insert(extra.id()) {
                entries.add(Some(extra));
            }
        }
        entries as RibbonReadOnlyList<_>
    }
    /// Source overload default is twelve; the explicit limit remains signed Int32.
    pub fn search_default(&self, query: Option<&RibbonString>) -> RibbonSearchResults {
        self.search(query, 12)
    }
    pub fn search(&self, query: Option<&RibbonString>, max_results: i32) -> RibbonSearchResults {
        let entries = self.build_search_entries();
        self.search_engine()
            .set_entries(Some(RibbonItemsSource::new(move || {
                Box::new(entries.iter())
            })));
        self.search_engine().search(query, max_results)
    }
    pub fn execute_search_entry(&self, entry: Option<Rc<RibbonSearchEntry>>) -> bool {
        self.execute_search_entry_from(entry, None)
    }
    pub(super) fn execute_search_entry_from(
        &self,
        entry: Option<Rc<RibbonSearchEntry>>,
        search_box: Option<WeakRef<TextBox>>,
    ) -> bool {
        let entry = entry.expect("NullReference: entry.Id");
        self.search_engine().mark_used(entry.id());
        let Some(target) = native_target(&entry) else {
            return false;
        };
        if let Some(item) =
            ValueTypes::as_object(&*target).and_then(|v| v.cast::<RibbonBackstageItem>())
            && let Some(backstage) = self.backstage()
        {
            self.set_is_backstage_open(true);
            backstage.invoke(Some(item));
            return true;
        }
        if let Some(descriptor) = cast::<Rc<RibbonCommandDescriptor>>(&target) {
            return self
                .command_catalog()
                .is_some_and(|catalog| catalog.execute(Some(&descriptor.id()), None));
        }
        if let Some(action) = cast::<MarkupDelegate>(&target) {
            // Source tests the nominal System.Action, not any callable or an
            // equivalent method shape. Untyped callbacks carry no such proof.
            if !action.is_of_type(&SYSTEM_ACTION_DELEGATE) {
                return false;
            }
            action.try_invoke(&[]).expect("search Action invocation");
            return true;
        }
        if let Some(element) = ValueTypes::as_object(&*target).and_then(|v| v.cast::<Control>())
            && let Some(item) = cast::<Rc<dyn IRibbonItem>>(&target)
        {
            let button = element.cast::<RibbonButton>();
            let needs_ui = element.cast::<RibbonInputBase>().is_some()
                || element.cast::<RibbonGallery>().is_some()
                || element.cast::<RibbonDropDownButton>().is_some()
                || button
                    .as_ref()
                    .is_some_and(|button| button.flyout().is_some());
            if needs_ui {
                let (tab, group) = self.find_location(&element);
                if let Some(tab) = tab {
                    self.set_selected_tab(Some(tab));
                    if self.visibility_mode() == RibbonVisibilityMode::TabsOnly {
                        self.open_minimized_popup();
                    }
                    if let Some(group) = group
                        && group.state() == RibbonGroupState::Collapsed
                        && !group.is_simplified()
                    {
                        group.open_popup();
                    }
                }
                Dispatcher::ui_thread().post_local(
                    move || {
                        // The flyout computes its position when opened. Flush any
                        // pending layout for the tab and popup that expose its command.
                        element.update_layout();
                        // In the browser the restored query focus reopens
                        // results and consumes the next pointer press. Focus
                        // the command before opening while the query still
                        // owns focus.
                        if cfg!(target_os = "emscripten")
                            && search_box
                                .as_ref()
                                .and_then(WeakRef::upgrade)
                                .is_some_and(|search| search.is_focused())
                            && button.is_some_and(|button| button.flyout().is_some())
                        {
                            element.focus();
                        }
                        item.on_key_tip();
                    },
                    DispatcherPriority::LOADED,
                );
                return true;
            }
            return item.invoke();
        }
        false
    }
}

#[cfg(test)]
#[path = "../root_search_tests.rs"]
mod tests;
