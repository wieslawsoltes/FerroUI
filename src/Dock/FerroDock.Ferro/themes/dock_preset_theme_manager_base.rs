// Copyright (c) Wiesław Šoltés. All rights reserved.
// Licensed under the MIT license. See LICENSE.TXT in the workspace root.
use super::IDockThemeManager;
use ferrodock_model::{
    collections::ReadOnlyListHandle,
    source_string::{is_white_space, ordinal_ignore_case_equal, ordinal_ignore_case_starts_with},
};
use ferroui_base::{
    controls::IResourceDictionary,
    styling::{IStyle, Styles, ThemeVariant},
    utilities::Uri,
};
use ferroui_controls::Application;
use ferroui_markup_xaml::styling::ResourceInclude;
use std::{fmt, rc::Rc};
/// The source abstract override, implemented by each concrete preset manager.
/// The composed base is the original base-class storage; this hook is required.
pub trait DockPresetThemeManagerHooks {
    /// Gets the enclosing manager's original base storage.
    fn base(&self) -> &DockPresetThemeManagerBase;
    /// Resolves the default dictionary which should also host a preset include.
    fn try_get_default_preset_owner(
        &self,
        application: &Application,
    ) -> Option<Rc<dyn IResourceDictionary>>;
}
/// A source constructor argument failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresetThemeManagerArgumentError {
    /// Source parameter name when the original exception supplies one.
    pub parameter: Option<&'static str>,
    /// Original constructor validation message.
    pub message: &'static str,
}
impl fmt::Display for PresetThemeManagerArgumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message)
    }
}
impl std::error::Error for PresetThemeManagerArgumentError {}
/// Original shared preset matching, traversal, removal and replacement algorithms.
/// Concrete managers compose this abstract base and implement its required hook.
pub struct DockPresetThemeManagerBase {
    preset_names: ReadOnlyListHandle<String>,
    preset_uris: ReadOnlyListHandle<Uri>,
    preset_uri_prefix: String,
}
impl DockPresetThemeManagerBase {
    /// Constructs the original abstract-base storage with its three source arguments.
    /// Invalid arguments return the corresponding source constructor failure.
    pub fn new(
        preset_names: Option<ReadOnlyListHandle<String>>,
        preset_uris: Option<ReadOnlyListHandle<Uri>>,
        preset_uri_prefix: Option<String>,
    ) -> Result<Self, PresetThemeManagerArgumentError> {
        let fail = |parameter, message| PresetThemeManagerArgumentError { parameter, message };
        let preset_names =
            preset_names.ok_or_else(|| fail(Some("presetNames"), "Value cannot be null."))?;
        let preset_uris =
            preset_uris.ok_or_else(|| fail(Some("presetUris"), "Value cannot be null."))?;
        let preset_uri_prefix = preset_uri_prefix
            .filter(|value| !is_white_space(value))
            .ok_or_else(|| {
                fail(
                    Some("presetUriPrefix"),
                    "Preset URI prefix cannot be empty.",
                )
            })?;
        if preset_names.count() == 0 {
            return Err(fail(
                Some("presetNames"),
                "At least one preset name is required.",
            ));
        }
        if preset_uris.count() == 0 {
            return Err(fail(
                Some("presetUris"),
                "At least one preset URI is required.",
            ));
        }
        if preset_names.count() != preset_uris.count() {
            return Err(fail(
                None,
                "Preset names and preset URIs must have the same count.",
            ));
        }
        Ok(Self {
            preset_names,
            preset_uris,
            preset_uri_prefix,
        })
    }
    /// Gets the same supplied read-only preset-name list, without copying its storage.
    pub fn preset_names(&self) -> ReadOnlyListHandle<String> {
        self.preset_names.clone()
    }
    /// Gets the first matching source preset index, or zero when none is found.
    pub fn current_preset_index(&self) -> i32 {
        self.get_current_preset_index()
    }
    /// Selects the source light/dark theme variant, leaving invalid indices unchanged.
    pub fn switch(&self, index: i32) {
        let Some(application) = Application::current() else {
            return;
        };
        let variant = match index {
            0 => Some(ThemeVariant::light()),
            1 => Some(ThemeVariant::dark()),
            _ => application.requested_theme_variant(),
        };
        application.set_requested_theme_variant(variant);
    }
    fn switch_preset(&self, index: i32, hooks: &impl DockPresetThemeManagerHooks) {
        if index < 0 || index as usize >= self.preset_uris.count() {
            return;
        }
        let Some(application) = Application::current() else {
            return;
        };
        let selected = self.preset_uris.get(index as usize);
        if self
            .try_find_application_preset_uri(&application)
            .is_some_and(|current| Self::are_equal_uris(&current, &selected))
        {
            return;
        }
        // Visit roots lazily in source order: application first, then each style tree.
        Self::visit_root_dictionaries(&application, &mut |dictionary| {
            self.remove_preset_includes(dictionary);
            false
        });
        let resources = application.resources();
        resources
            .merged_dictionaries()
            .add(Self::create_preset_include(&selected).as_resource_provider());
        if let Some(owner) = hooks.try_get_default_preset_owner(&application) {
            let root: Rc<dyn IResourceDictionary> = resources.into();
            if *owner != *root {
                owner
                    .merged_dictionaries()
                    .add(Self::create_preset_include(&selected).as_resource_provider());
            }
        }
    }
    fn get_current_preset_index(&self) -> i32 {
        let Some(application) = Application::current() else {
            return 0;
        };
        let Some(current) = self.try_find_application_preset_uri(&application) else {
            return 0;
        };
        for index in 0..self.preset_uris.count() {
            if Self::are_equal_uris(&current, &self.preset_uris.get(index)) {
                return index as i32;
            }
        }
        0
    }
    fn try_find_application_preset_uri(&self, application: &Application) -> Option<Uri> {
        let mut found = None;
        Self::visit_root_dictionaries(application, &mut |dictionary| {
            found = self.try_find_preset_uri(dictionary);
            found.is_some()
        });
        found
    }
    fn visit_root_dictionaries(
        application: &Application,
        visit: &mut impl FnMut(&dyn IResourceDictionary) -> bool,
    ) -> bool {
        let root: Rc<dyn IResourceDictionary> = application.resources().into();
        if visit(&*root) {
            return true;
        }
        for style in application.styles().snapshot().iter() {
            if Self::visit_style_resource_dictionaries(&**style, visit) {
                return true;
            }
        }
        false
    }
    fn visit_style_resource_dictionaries(
        style: &dyn IStyle,
        visit: &mut impl FnMut(&dyn IResourceDictionary) -> bool,
    ) -> bool {
        let Some(styles) = style
            .as_object()
            .and_then(|object| object.downcast_ref::<Styles>())
        else {
            return false;
        };
        let resources: Rc<dyn IResourceDictionary> = styles.resources().into();
        if visit(&*resources) {
            return true;
        }
        for child in styles.snapshot().iter() {
            if Self::visit_style_resource_dictionaries(&**child, visit) {
                return true;
            }
        }
        false
    }
    fn try_find_preset_uri(&self, dictionary: &dyn IResourceDictionary) -> Option<Uri> {
        for provider in dictionary.merged_dictionaries().snapshot().iter() {
            if let Some(include) = provider.as_any().and_then(|provider| provider.downcast_ref::<ResourceInclude>()) {
                if let Some(source) = include.source().filter(|source| self.is_preset_uri(source)) {
                    return Some(source);
                }
            }
            if let Some(dictionary) = provider.as_resource_dictionary() {
                if let Some(source) = self.try_find_preset_uri(dictionary) {
                    return Some(source);
                }
            }
        }
        None
    }
    fn remove_preset_includes(&self, dictionary: &dyn IResourceDictionary) {
        let merged = dictionary.merged_dictionaries();
        for index in (0..merged.count()).rev() {
            let provider = merged.get(index);
            if let Some(include) = provider.as_any().and_then(|provider| provider.downcast_ref::<ResourceInclude>()) {
                if include
                    .source()
                    .is_some_and(|source| self.is_preset_uri(&source))
                {
                    merged.remove_at(index);
                    continue;
                }
            }
            if let Some(dictionary) = provider.as_resource_dictionary() {
                self.remove_preset_includes(dictionary);
            }
        }
    }
    fn is_preset_uri(&self, source: &Uri) -> bool {
        ordinal_ignore_case_starts_with(&source.to_string(), &self.preset_uri_prefix)
    }
    fn are_equal_uris(left: &Uri, right: &Uri) -> bool {
        ordinal_ignore_case_equal(&left.to_string(), &right.to_string())
    }
    fn create_preset_include(preset_uri: &Uri) -> Rc<ResourceInclude> {
        let include = ResourceInclude::new(Some(preset_uri.clone()));
        include.set_source(Some(preset_uri.clone()));
        include
    }
}
impl<T: DockPresetThemeManagerHooks> IDockThemeManager for T {
    fn switch(&self, index: i32) {
        self.base().switch(index)
    }
    fn preset_names(&self) -> ReadOnlyListHandle<String> {
        self.base().preset_names()
    }
    fn current_preset_index(&self) -> i32 {
        self.base().current_preset_index()
    }
    fn switch_preset(&self, index: i32) {
        self.base().switch_preset(index, self)
    }
}
