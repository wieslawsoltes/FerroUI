//! Ribbon.Keyboard.cs shortcut cache/reachability, including actual QAT views.
use super::super::ribbon_item_helper::IRibbonItemInternal;
use super::super::{IRibbonItem, RibbonGroup, RibbonItemHelper};
use super::*;
use ferroribbon_core::{
    commands::{RibbonKeyGesture, RibbonModifierKeys},
    text::{RibbonOrdinalIgnoreCaseKey, trim},
};
use ferroui_base::{
    Visual,
    data::core::{ValueType, ValueTypes},
    input::{Key, KeyEventArgs, KeyModifiers},
};
use ferroui_controls::{AutoSuggestBox, RichEditBox, TextBox, TopLevel};
use std::collections::HashMap;
#[path = "ribbon_shortcut_key_names.rs"]
mod names;
type Entries = Vec<(RibbonKeyGesture, Ref<Control>)>;
type Cache = Vec<(RibbonString, Entries)>;
pub(super) struct ShortcutState {
    cache: RefCell<Option<Rc<Cache>>>,
    version: Cell<i32>,
}
impl Default for ShortcutState {
    fn default() -> Self {
        Self {
            cache: RefCell::default(),
            version: Cell::new(-1),
        }
    }
}
fn cast<T: ferroui_base::PropertyValue>(item: &Control) -> Option<T> {
    ValueTypes::try_cast(&Control::boxed(item.to_ref()), ValueType::of::<T>())
        .and_then(|v| v.downcast_ref::<T>().cloned())
}
impl Ribbon {
    pub fn is_key_tips_enabled(&self) -> bool {
        self.get_value(Self::is_key_tips_enabled_property())
    }
    pub fn set_is_key_tips_enabled(&self, value: bool) {
        self.set_value(Self::is_key_tips_enabled_property(), value)
    }
    pub fn is_shortcut_routing_enabled(&self) -> bool {
        self.get_value(Self::is_shortcut_routing_enabled_property())
    }
    pub fn set_is_shortcut_routing_enabled(&self, value: bool) {
        self.set_value(Self::is_shortcut_routing_enabled_property(), value)
    }
    pub fn invalidate_shortcuts(&self) {
        let old = self.shortcuts.cache.take();
        drop(old);
    }
    fn build_shortcut_cache(&self) -> Cache {
        let mut result: Cache = Vec::new();
        let mut lookup = HashMap::new();
        // Source sequence is GetAllItems followed by actual QAT presentations.
        for item in self.command_presentations().iter() {
            let Some(shortcut) = cast::<Rc<dyn IRibbonItemInternal>>(&item)
                .and_then(|i| i.shortcut())
                .filter(|s| !s.is_empty())
            else {
                continue;
            };
            for units in shortcut
                .as_utf16()
                .split(|&c| c == b',' as u16)
                .map(trim)
                .filter(|s| !s.is_empty())
            {
                let text = RibbonString::from(units);
                let mut gesture = RibbonKeyGesture::default();
                if !RibbonKeyGesture::try_parse(Some(&text), &mut gesture) {
                    continue;
                }
                let source =
                    RibbonItemHelper::get_source_item(&item).unwrap_or_else(|| item.clone());
                let key = gesture.to_ribbon_string();
                let index = *lookup
                    .entry(RibbonOrdinalIgnoreCaseKey(key.clone()))
                    .or_insert_with(|| {
                        let index = result.len();
                        result.push((key, Vec::new()));
                        index
                    });
                if !result[index].1.iter().any(|(_, old)| old.ptr_eq(&source)) {
                    result[index].1.push((gesture, source));
                }
            }
        }
        result
    }
    pub(super) fn try_route_shortcut(&self, key: Key, modifiers: RibbonModifierKeys) -> bool {
        if modifiers == RibbonModifierKeys::NONE && !(Key::F1..=Key::F24).contains(&key) {
            return false;
        }
        if (modifiers == RibbonModifierKeys::SHIFT || modifiers == RibbonModifierKeys::NONE)
            && TopLevel::get_top_level(Some(self))
                .and_then(|w| w.focus_manager().get_focused_element())
                .is_some_and(|e| {
                    e.is::<TextBox>() || e.is::<RichEditBox>() || e.is::<AutoSuggestBox>()
                })
        {
            return false;
        }
        // Native TextBox covers password editing too.
        let rebuild = self.shortcuts.cache.borrow().is_none()
            || self.shortcuts.version.get() != RibbonItemHelper::items_version();
        if rebuild {
            let cache = Rc::new(self.build_shortcut_cache());
            let old = self.shortcuts.cache.replace(Some(cache));
            drop(old);
            self.shortcuts
                .version
                .set(RibbonItemHelper::items_version());
        }
        let Some(name) = names::key_name(key) else {
            return false;
        };
        let name = RibbonString::from(name);
        // Keep the original enumerated dictionary alive when invocation reenters
        // invalidation. No RefCell borrow crosses an item getter or invocation.
        let cache = self
            .shortcuts
            .cache
            .borrow()
            .clone()
            .expect("shortcut cache publication");
        for (_, entries) in cache.iter() {
            for (gesture, item) in entries {
                if gesture.matches(Some(&name), modifiers)
                    && let Some(ribbon_item) = cast::<Rc<dyn IRibbonItem>>(item)
                    && self.is_shortcut_reachable(item)
                {
                    return ribbon_item.invoke();
                }
            }
        }
        false
    }
    fn is_shortcut_reachable(&self, item: &Control) -> bool {
        // Uno's coerce callback includes inherited disabling in IsEnabled.
        // FerroUI exposes that result separately from the local property.
        if !item.is_effectively_enabled() {
            return false;
        }
        let (tab, group) = self.find_location(item);
        if tab.as_ref().is_some_and(|t| !t.is_available())
            || group.as_ref().is_some_and(|g| !g.is_shown())
        {
            return false;
        }
        let mut current = Some(item.to_ref().upcast::<FerroObject>());
        while let Some(node) = current {
            if node.is::<RibbonGroup>() || node.is::<RibbonTab>() || node.is::<Ribbon>() {
                break;
            }
            if node
                .downcast_ref::<Visual>()
                .is_some_and(|v| !v.is_visible())
            {
                return false;
            }
            // A WinUI Window is outside the source FrameworkElement tree.
            // Native TopLevel is a Control with a logical TopLevelHost whose
            // visual parent is this same window; stop at the XamlRoot boundary.
            if node.is::<TopLevel>() {
                break;
            }
            current = node
                .downcast_ref::<Visual>()
                .and_then(|v| v.visual_parent())
                .map(|p| p.upcast::<FerroObject>())
                .or_else(|| {
                    node.downcast_ref::<Control>()
                        .and_then(|c| c.get_logical_parent())
                        .map(|p| p.upcast())
                });
        }
        tab.is_some()
            || group.is_some()
            || self.tab_strip_items.contains(&Some(item.to_ref()))
            || effectively_visible(item)
    }
    pub(super) fn on_shortcut_key_down(&self, e: &KeyEventArgs) {
        if matches!(e.key, Key::LeftAlt | Key::RightAlt) {
            return;
        }
        if e.handled() || !effectively_visible(self) {
            return;
        }

        if self.is_shortcut_routing_enabled()
            && self.try_route_shortcut(e.key, modifiers(e.key_modifiers))
        {
            e.set_handled(true);
        }
    }
}
fn effectively_visible(item: &Control) -> bool {
    if TopLevel::get_top_level(Some(item)).is_none() {
        return false;
    }
    let mut current = Some(item.to_ref().upcast::<Visual>());
    while let Some(node) = current {
        if !node.is_visible() {
            return false;
        }
        current = node.visual_parent();
    }
    true
}
fn modifiers(value: KeyModifiers) -> RibbonModifierKeys {
    let mut result = RibbonModifierKeys::NONE;
    for (from, to) in [
        (KeyModifiers::CONTROL, RibbonModifierKeys::CONTROL),
        (KeyModifiers::SHIFT, RibbonModifierKeys::SHIFT),
        (KeyModifiers::ALT, RibbonModifierKeys::ALT),
        (KeyModifiers::META, RibbonModifierKeys::META),
    ] {
        if value.contains(from) {
            result |= to;
        }
    }
    result
}

#[cfg(test)]
#[path = "../shortcut_tests.rs"]
mod tests;
