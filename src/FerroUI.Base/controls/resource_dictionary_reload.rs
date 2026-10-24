//! Development ownership of initially populated dictionary contributions.
use super::*;
use crate::WeakRef;
use std::rc::Weak;

type Items = HashMap<ResourceKey, ResourceItem, ResourceKeyBuildHasher>;
type Versions = HashMap<ResourceKey, u64, ResourceKeyBuildHasher>;
type Themes = Vec<(ThemeVariant, Rc<dyn IThemeVariantProvider>)>;

#[derive(Clone, Default)]
struct Owned {
    keys: Versions,
    merged: Vec<Weak<dyn IResourceProvider>>,
    themes: Vec<(ThemeVariant, Weak<dyn IThemeVariantProvider>)>,
    index: usize,
}
/// Claims existing entries immediately after XAML population. Retains no resource
/// values or owners. Later application entries survive replacement; writes to an
/// owned key (including assigning the same value) relinquish ownership and reject
/// the reload. Deferred realization itself is not an application write.
#[derive(Clone)]
pub struct ResourceDictionaryOwnership {
    dictionary: WeakRef<ResourceDictionary>,
    owned: Rc<RefCell<Owned>>,
}
impl ResourceDictionaryOwnership {
    pub fn capture(dictionary: &Ref<ResourceDictionary>) -> Self {
        Self {
            dictionary: dictionary.downgrade(),
            owned: Rc::new(RefCell::new(Owned {
                keys: dictionary.writes.borrow().clone(),
                merged: dictionary
                    .merged_dictionaries()
                    .to_vec()
                    .iter()
                    .map(Rc::downgrade)
                    .collect(),
                themes: dictionary
                    .theme_dictionaries_snapshot()
                    .into_iter()
                    .map(|(key, provider)| (key, Rc::downgrade(&provider)))
                    .collect(),
                index: 0,
            })),
        }
    }
    pub fn owns_container(&self, dictionary: &Ref<ResourceDictionary>) -> bool {
        self.dictionary
            .upgrade()
            .is_some_and(|original| original.ptr_eq(dictionary))
    }
    fn validate(
        &self,
        dictionary: &ResourceDictionary,
    ) -> Result<(usize, Vec<Rc<dyn IResourceProvider>>, Themes), String> {
        let owned = self.owned.borrow();
        if owned
            .keys
            .iter()
            .any(|(key, version)| dictionary.writes.borrow().get(key) != Some(version))
        {
            return Err("Application changed an owned resource key".into());
        }
        let merged = owned
            .merged
            .iter()
            .map(|weak| {
                weak.upgrade()
                    .ok_or("Owned merged resource was removed".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let current = dictionary.merged_dictionaries().to_vec();
        if merged.iter().any(|owned| {
            current
                .iter()
                .filter(|entry| resource_provider_ptr_eq(entry, owned))
                .count()
                != 1
        }) {
            return Err("Ambiguous merged-resource ownership".into());
        }
        let index = merged
            .first()
            .and_then(|first| {
                current
                    .iter()
                    .position(|entry| resource_provider_ptr_eq(entry, first))
            })
            .unwrap_or(owned.index);
        if current
            .get(index..index + merged.len())
            .is_none_or(|range| {
                range
                    .iter()
                    .zip(&merged)
                    .any(|(a, b)| !resource_provider_ptr_eq(a, b))
            })
        {
            return Err("Owned merged resources are no longer contiguous".into());
        }
        let themes = owned
            .themes
            .iter()
            .map(|(key, weak)| {
                let provider = weak.upgrade().ok_or("Owned theme resource was removed")?;
                if dictionary
                    .theme_dictionary(key)
                    .is_none_or(|current| current.as_ref() != provider.as_ref())
                {
                    return Err("Application changed an owned theme resource".to_string());
                }
                if dictionary
                    .theme_dictionaries_snapshot()
                    .iter()
                    .filter(|(_, current)| current.as_ref() == provider.as_ref())
                    .count()
                    != 1
                {
                    return Err("Ambiguous theme-resource ownership".to_string());
                }
                Ok((key.clone(), provider))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok((index, merged, themes))
    }
    /// Looks up only application contributions for a detached candidate's ambient
    /// context. Does not attach an owner or realize a copy of an application value.
    pub fn application_resource(
        &self,
        key: &ResourceKey,
        theme: Option<&ThemeVariant>,
    ) -> Option<ResourceValue> {
        let dictionary = self.dictionary.upgrade()?;
        let owned = self.owned.borrow();
        if !owned.keys.contains_key(key) {
            if let Some(value) = dictionary.try_get_value(key) {
                return Some(value);
            }
        }
        let mut variants = Vec::new();
        if let Some(theme) = theme {
            let mut current = Some(theme.clone());
            while let Some(variant) = current {
                current = variant.inherit_variant();
                variants.push(variant);
            }
        }
        variants.push(ThemeVariant::default());
        for variant in variants {
            if !owned.themes.iter().any(|(key, _)| *key == variant) {
                if let Some(provider) = dictionary.theme_dictionary(&variant) {
                    if let Some(value) =
                        crate::controls::resource_preview::lookup(&*provider, key, theme)
                    {
                        return Some(value);
                    }
                }
            }
        }
        for provider in dictionary.merged_dictionaries().to_vec().iter().rev() {
            if !owned
                .merged
                .iter()
                .filter_map(Weak::upgrade)
                .any(|old| resource_provider_ptr_eq(&old, provider))
            {
                if let Some(value) =
                    crate::controls::resource_preview::lookup(&**provider, key, theme)
                {
                    return Some(value);
                }
            }
        }
        None
    }
    /// Resolves the composed dictionary without transferring owners. Application
    /// entries remain in place, replacement themes keep per-variant precedence,
    /// and replacement merged providers occupy the original owned range.
    /// Call this throughout candidate population, including deferred factories.
    pub fn preview_resource(
        &self,
        candidate: &ResourceDictionary,
        key: &ResourceKey,
        theme: Option<&ThemeVariant>,
    ) -> Result<Option<ResourceValue>, String> {
        let dictionary = self.dictionary.upgrade().ok_or("Resource owner closed")?;
        let (index, old_merged, _) = self.validate(&dictionary)?;
        let owned = self.owned.borrow().clone();
        if let Some(value) = candidate.try_get_value(key) {
            return Ok(Some(value));
        }
        if !owned.keys.contains_key(key) {
            if let Some(value) = dictionary.try_get_value(key) {
                return Ok(Some(value));
            }
        }
        let mut variants = Vec::new();
        let mut current = theme.cloned().filter(|variant| *variant != ThemeVariant::default());
        while let Some(variant) = current {
            current = variant.inherit_variant();
            variants.push(variant);
        }
        if !variants.contains(&ThemeVariant::default()) {
            variants.push(ThemeVariant::default());
        }
        for variant in variants {
            let provider = candidate.theme_dictionary(&variant).or_else(|| {
                if owned.themes.iter().any(|(key, _)| *key == variant) { None }
                else { dictionary.theme_dictionary(&variant) }
            });
            if let Some(provider) = provider {
                if let Some(value) = crate::controls::resource_preview::lookup(&*provider, key, theme) {
                    return Ok(Some(value));
                }
            }
        }
        let current = dictionary.merged_dictionaries().to_vec();
        let new = candidate.merged_dictionaries().to_vec();
        for provider in current[index + old_merged.len()..].iter().rev()
            .chain(new.iter().rev()).chain(current[..index].iter().rev()) {
            if let Some(value) = crate::controls::resource_preview::lookup(&**provider, key, theme) {
                return Ok(Some(value));
            }
        }
        Ok(None)
    }
    /// Prepares replacement when every candidate lookup uses `preview_resource`.
    /// Callers without a composed preview must use `prepare`, which conservatively
    /// rejects provider combinations whose precedence it cannot validate.
    pub fn prepare_with_preview(
        &self,
        candidate: &Ref<ResourceDictionary>,
    ) -> Result<ResourceDictionaryChange, String> {
        self.prepare_impl(candidate, true)
    }
    /// Candidate must be detached from its temporary owner before commit.
    /// Commit/rollback operate on framework collections; custom notification
    /// observers must not panic, mutate those collections or perform irreversible work.
    pub fn prepare(
        &self,
        candidate: &Ref<ResourceDictionary>,
    ) -> Result<ResourceDictionaryChange, String> {
        self.prepare_impl(candidate, false)
    }
    fn prepare_impl(
        &self,
        candidate: &Ref<ResourceDictionary>,
        composed_preview: bool,
    ) -> Result<ResourceDictionaryChange, String> {
        let dictionary = self.dictionary.upgrade().ok_or("Resource owner closed")?;
        if !std::ptr::eq(dictionary.get_type(), ResourceDictionary::TYPE)
            || !std::ptr::eq(candidate.get_type(), ResourceDictionary::TYPE)
        {
            return Err(
                "Custom resource dictionaries require an explicit ownership adapter".into(),
            );
        }
        let (index, merged, themes) = self.validate(&dictionary)?;
        let owned = self.owned.borrow().clone();
        let candidate_versions = candidate.writes.borrow().clone();
        let new = candidate.inner.borrow().clone();
        let new_merged = candidate.merged_dictionaries().to_vec();
        let new_themes = candidate.theme_dictionaries_snapshot();
        for key in new.keys() {
            if dictionary.contains_key(key) && !owned.keys.contains_key(key) {
                return Err("New XAML resource collides with an application key".into());
            }
        }
        if new_themes.iter().any(|(key, _)| {
            dictionary.theme_dictionary(key).is_some()
                && !owned.themes.iter().any(|(old, _)| old == key)
        }) {
            return Err("New XAML theme collides with an application theme".into());
        }
        if !composed_preview {
            // Candidate resource lookup must have the same precedence as commit.
            // Mixed application provider trees need an explicit composed preview.
            let app_merged = dictionary.merged_dictionaries().count() != merged.len();
            let app_themes = dictionary.theme_dictionaries_snapshot().len() != themes.len();
            if (app_merged && !new_merged.is_empty())
                || (app_themes && (!new_merged.is_empty() || !new_themes.is_empty()))
            {
                return Err(
                    "Mixed application/XAML provider trees require a resource preview adapter".into(),
                );
            }
            let mut variants: Vec<_> = new_themes
                .iter()
                .map(|(variant, _)| Some(variant.clone()))
                .collect();
            variants.push(None);
            for provider in [&dictionary, candidate] {
                variants.push(provider.owner().and_then(|owner| {
                    owner
                        .as_theme_variant_host()
                        .and_then(|host| host.actual_theme_variant())
                }));
            }
            for key in dictionary
                .keys()
                .iter()
                .filter(|key| !owned.keys.contains_key(*key))
            {
                if variants
                    .iter()
                    .any(|variant| candidate.try_get_resource(key, variant.as_ref()).is_some())
                {
                    return Err("XAML resource lookup conflicts with application precedence".into());
                }
            }
        }
        if *candidate.writes.borrow() != candidate_versions {
            return Err("Candidate resource factory changed dictionary entries".into());
        }
        let old = dictionary
            .inner
            .borrow()
            .iter()
            .filter(|(key, _)| owned.keys.contains_key(*key))
            .map(|(key, item)| (key.clone(), item.clone()))
            .collect();
        let live_versions = dictionary.writes.borrow().clone();
        Ok(ResourceDictionaryChange {
            live_versions,
            candidate_versions,
            live_merged: dictionary.merged_dictionaries().to_vec(),
            live_themes: dictionary.theme_dictionaries_snapshot(),
            ownership: self.clone(),
            dictionary,
            candidate: candidate.clone(),
            old,
            new,
            old_versions: owned.keys,
            merged,
            new_merged,
            themes,
            new_themes,
            index,
            started: false,
        })
    }
}

/// Reversible replacement, retaining deferred factories without evaluating them.
pub struct ResourceDictionaryChange {
    ownership: ResourceDictionaryOwnership,
    dictionary: Ref<ResourceDictionary>,
    candidate: Ref<ResourceDictionary>,
    old: Items,
    new: Items,
    old_versions: Versions,
    live_versions: Versions,
    candidate_versions: Versions,
    live_merged: Vec<Rc<dyn IResourceProvider>>,
    live_themes: Themes,
    merged: Vec<Rc<dyn IResourceProvider>>,
    new_merged: Vec<Rc<dyn IResourceProvider>>,
    themes: Themes,
    new_themes: Themes,
    index: usize,
    started: bool,
}
impl ResourceDictionaryChange {
    pub fn commit(&mut self) -> Result<(), String> {
        let (index, _, _) = self.ownership.validate(&self.dictionary)?;
        if index != self.index
            || *self.dictionary.writes.borrow() != self.live_versions
            || self.dictionary.merged_dictionaries().to_vec() != self.live_merged
            || self.dictionary.theme_dictionaries_snapshot() != self.live_themes
            || *self.candidate.writes.borrow() != self.candidate_versions
            || self.candidate.merged_dictionaries().to_vec() != self.new_merged
            || self.candidate.theme_dictionaries_snapshot() != self.new_themes
        {
            return Err("Resource contributions changed after preparation".into());
        }
        if self.candidate.owner().is_some() {
            return Err("Candidate resources still have an owner".into());
        }
        // Include resources realized by later candidate preparation, preserving
        // the identity already captured by its StaticResource consumers.
        self.new = self.candidate.inner.borrow().clone();
        if self.old.is_empty()
            && self.new.is_empty()
            && self.merged.is_empty()
            && self.new_merged.is_empty()
            && self.themes.is_empty()
            && self.new_themes.is_empty()
        {
            return Ok(());
        }
        self.started = true;
        {
            let mut items = self.dictionary.inner.borrow_mut();
            for key in self.old.keys() {
                items.remove(key);
                self.dictionary.writes.borrow_mut().remove(key);
            }
            for (key, item) in &self.new {
                self.dictionary.record_write(key);
                items.insert(key.clone(), item.clone());
            }
        }
        let merged = self.dictionary.merged_dictionaries();
        merged.remove_range(self.index, self.merged.len());
        merged.insert_range(self.index, self.new_merged.clone());
        for (key, _) in &self.themes {
            self.dictionary.remove_theme_dictionary(key);
        }
        for (key, value) in &self.new_themes {
            self.dictionary
                .set_theme_dictionary(key.clone(), value.clone());
        }
        self.dictionary.raise_resources_changed();
        Ok(())
    }
    pub fn rollback(&mut self) {
        if !self.started {
            return;
        }
        {
            let mut items = self.dictionary.inner.borrow_mut();
            for key in self.new.keys() {
                items.remove(key);
                self.dictionary.writes.borrow_mut().remove(key);
            }
            for (key, item) in &self.old {
                items.insert(key.clone(), item.clone());
            }
            self.dictionary
                .writes
                .borrow_mut()
                .extend(self.old_versions.clone());
        }
        let merged = self.dictionary.merged_dictionaries();
        for provider in &self.new_merged {
            if let Some(index) = merged
                .to_vec()
                .iter()
                .position(|entry| resource_provider_ptr_eq(entry, provider))
            {
                merged.remove_at(index);
            }
        }
        for (offset, provider) in self.merged.iter().enumerate() {
            if !merged
                .to_vec()
                .iter()
                .any(|entry| resource_provider_ptr_eq(entry, provider))
            {
                merged.insert(self.index + offset, provider.clone());
            }
        }
        for (key, _) in &self.new_themes {
            self.dictionary.remove_theme_dictionary(key);
        }
        for (key, value) in &self.themes {
            self.dictionary
                .set_theme_dictionary(key.clone(), value.clone());
        }
        self.dictionary.raise_resources_changed();
        self.started = false;
    }
    pub fn finish(&mut self) {
        *self.ownership.owned.borrow_mut() = Owned {
            keys: self
                .new
                .keys()
                .map(|key| (key.clone(), self.dictionary.writes.borrow()[key]))
                .collect(),
            merged: self.new_merged.iter().map(Rc::downgrade).collect(),
            themes: self
                .new_themes
                .iter()
                .map(|(key, provider)| (key.clone(), Rc::downgrade(provider)))
                .collect(),
            index: self.index,
        };
    }
}
