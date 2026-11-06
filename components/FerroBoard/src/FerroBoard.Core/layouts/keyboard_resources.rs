// Derived from Layouts/KeyboardResources.cs; see ../NOTICE.md.
//! Mutable registries have owner-thread Rc ownership; independent registries can be constructed
//! on any thread. Native engine defaults own a bundled registry. Share one explicitly through Rc
//! when engines should observe the same custom layouts, caches and resource-change notifications.
use super::*;
use crate::{
    resources::EmbeddedResources,
    text::{AppenderComposer, HangulComposer, IComposer, KanaComposer, RulesComposer, Utf16Text},
};
use ferroui_system::events::Event;
use std::{cell::RefCell, collections::HashMap, hash::Hash, rc::Rc, sync::Arc};

#[derive(Debug)]
pub enum ResourceError {
    NullReference,
    KeyboardMode(UnsupportedKeyboardMode),
    LayoutType(UnsupportedLayoutType),
    Collection(ResourceTextListError),
    Json(LayoutJsonError),
    Collation(crate::text::locale_text::CollationError),
    Io(std::io::Error),
    /// The current JSON/identity bridge cannot represent this source value yet.
    UnsupportedRepresentation(&'static str),
}
impl std::fmt::Display for ResourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NullReference => f.write_str("required keyboard resource is missing"),
            Self::KeyboardMode(error) => std::fmt::Display::fmt(error, f),
            Self::LayoutType(error) => std::fmt::Display::fmt(error, f),
            Self::Collection(error) => std::fmt::Display::fmt(error, f),
            Self::Json(error) => f.write_str(&error.0),
            Self::Collation(error) => std::fmt::Display::fmt(error, f),
            Self::Io(error) => std::fmt::Display::fmt(error, f),
            Self::UnsupportedRepresentation(detail) => {
                write!(f, "Unsupported representation: {detail}")
            }
        }
    }
}
impl std::error::Error for ResourceError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::KeyboardMode(error) => Some(error),
            Self::LayoutType(error) => Some(error),
            Self::Collection(error) => Some(error),
            Self::Json(error) => Some(error),
            Self::Collation(error) => Some(error),
            Self::Io(error) => Some(error),
            Self::NullReference | Self::UnsupportedRepresentation(_) => None,
        }
    }
}
impl From<UnsupportedKeyboardMode> for ResourceError {
    fn from(value: UnsupportedKeyboardMode) -> Self {
        Self::KeyboardMode(value)
    }
}
impl From<UnsupportedLayoutType> for ResourceError {
    fn from(value: UnsupportedLayoutType) -> Self {
        Self::LayoutType(value)
    }
}
impl From<ResourceTextListError> for ResourceError {
    fn from(value: ResourceTextListError) -> Self {
        Self::Collection(value)
    }
}
impl From<LayoutJsonError> for ResourceError {
    fn from(value: LayoutJsonError) -> Self {
        Self::Json(value)
    }
}
impl From<std::io::Error> for ResourceError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
impl From<crate::text::locale_text::CollationError> for ResourceError {
    fn from(value: crate::text::locale_text::CollationError) -> Self {
        Self::Collation(value)
    }
}
pub type ResourceResult<T> = Result<T, ResourceError>;
pub type ResourceFileProvider = Rc<dyn Fn(&Utf16Text) -> ResourceResult<Option<Utf16Text>>>;
type ArrangementLoader = Rc<dyn Fn() -> ResourceResult<Option<Rc<LayoutArrangement>>>>;
type PopupLoader = Rc<dyn Fn() -> ResourceResult<Rc<PopupMapping>>>;

// These dictionaries never remove keys. Replacing a value preserves its enumeration slot.
// Keeping the order explicitly avoids HashMap's randomized iteration affecting the API.
struct RegistryMap<K, V>(Vec<(K, V)>, HashMap<K, usize>);
impl<K, V> Default for RegistryMap<K, V> {
    fn default() -> Self {
        Self(Vec::new(), HashMap::new())
    }
}
impl<K: Eq + Hash + Clone, V> RegistryMap<K, V> {
    fn get(&self, key: &K) -> Option<&V> {
        self.1.get(key).map(|i| &self.0[*i].1)
    }
    fn insert(&mut self, key: K, value: V) -> Option<V> {
        if let Some(i) = self.1.get(&key) {
            Some(std::mem::replace(&mut self.0[*i].1, value))
        } else {
            self.1.insert(key.clone(), self.0.len());
            self.0.push((key, value));
            None
        }
    }
}
struct LayoutEntry {
    metadata: Arc<LayoutMetadata>,
    extension_id: Option<Utf16Text>,
    loader: ArrangementLoader,
    cached: RefCell<Option<Rc<KeyboardLayout>>>,
}
#[derive(Default)]
struct State {
    layouts: RegistryMap<(LayoutType, ComponentName), Rc<LayoutEntry>>,
    popup_loaders: RegistryMap<ComponentName, PopupLoader>,
    popup_mappings: HashMap<ComponentName, Rc<PopupMapping>>,
    currency_sets: RegistryMap<ComponentName, Rc<CurrencySet>>,
    composers: RegistryMap<ComponentName, Arc<dyn IComposer>>,
    punctuation_rules: RegistryMap<ComponentName, Arc<PunctuationRule>>,
    subtypes: Vec<Arc<Subtype>>,
    extensions: Vec<Rc<KeyboardExtensionManifest>>,
}
pub struct KeyboardResources {
    state: RefCell<State>,
    changed: Event<KeyboardResources>,
}
fn utf8(text: &Utf16Text) -> ResourceResult<String> {
    text.to_utf8()
        .map_err(|_| ResourceError::UnsupportedRepresentation("JSON/path with unpaired surrogate"))
}
fn required(text: &Option<Utf16Text>, field: &'static str) -> ResourceResult<Utf16Text> {
    text.clone()
        .ok_or(ResourceError::UnsupportedRepresentation(field))
}
fn path(parts: &[&Utf16Text]) -> Utf16Text {
    Utf16Text::from_units(
        parts
            .iter()
            .flat_map(|p| p.units().iter().copied())
            .collect::<Vec<_>>(),
    )
}
impl PartialEq for KeyboardResources {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for KeyboardResources {}
impl KeyboardResources {
    pub const CORE_LAYOUTS_EXTENSION: &'static str = "org.florisboard.layouts";
    pub const CUSTOM_EXTENSION: &'static str = "custom";
    pub fn create_empty() -> Self {
        let mut state = State::default();
        state.composers.insert(
            ComponentName::new(Subtype::CORE_COMPOSERS, "appender"),
            AppenderComposer::instance(),
        );
        state.punctuation_rules.insert(
            ComponentName::new(Subtype::CORE_LOCALIZATION, "default"),
            PunctuationRule::shared_default(),
        );
        Self {
            state: RefCell::new(state),
            changed: Event::new(),
        }
    }
    pub fn create_default() -> ResourceResult<Self> {
        let resources = Self::create_empty();
        for extension in [
            Self::CORE_LAYOUTS_EXTENSION,
            Subtype::CORE_LOCALIZATION,
            Subtype::CORE_COMPOSERS,
            Subtype::CORE_CURRENCY_SETS,
        ] {
            let root = format!("keyboard/{extension}");
            let Some(json) = EmbeddedResources::read_text(&format!("{root}/extension.json"))?
            else {
                continue;
            };
            let manifest = Rc::new(LayoutJson::read_extension_manifest(&utf8(&json)?)?);
            resources.add_extension(
                manifest,
                Rc::new(move |relative| {
                    Ok(EmbeddedResources::read_text(&format!(
                        "{root}/{}",
                        utf8(relative)?
                    ))?)
                }),
            )?;
        }
        Ok(resources)
    }
    pub fn changed(&self) -> &Event<Self> {
        &self.changed
    }
    pub fn extensions(&self) -> Vec<Rc<KeyboardExtensionManifest>> {
        self.state.borrow().extensions.clone()
    }
    pub fn subtype_presets(&self) -> Vec<Arc<Subtype>> {
        self.state.borrow().subtypes.clone()
    }
    /// Exact ordinal-insensitive tag first, then the first matching language prefix.
    pub fn find_subtype_preset(&self, language_tag: &Utf16Text) -> Option<Arc<Subtype>> {
        use crate::text::ordinal_ignore_case_equal;
        fn language(tag: &Utf16Text) -> Utf16Text {
            Utf16Text::from_units(
                tag.units()
                    .split(|&c| c == b'-' as u16)
                    .next()
                    .unwrap()
                    .to_vec(),
            )
        }
        let state = self.state.borrow();
        if let Some(exact) = state.subtypes.iter().find(|s| {
            s.language_tag()
                .as_ref()
                .is_some_and(|tag| ordinal_ignore_case_equal(tag, language_tag))
        }) {
            return Some(exact.clone());
        }
        let lang = language(language_tag);
        state
            .subtypes
            .iter()
            .find(|s| {
                ordinal_ignore_case_equal(
                    &language(
                        &s.language_tag()
                            .unwrap_or_else(|| std::panic::panic_any(ResourceError::NullReference)),
                    ),
                    &lang,
                )
            })
            .cloned()
    }
    pub fn currency_sets(&self) -> Vec<Rc<CurrencySet>> {
        self.state
            .borrow()
            .currency_sets
            .0
            .iter()
            .map(|(_, v)| v.clone())
            .collect()
    }
    pub fn composers(&self) -> Vec<Arc<dyn IComposer>> {
        let mut values = Vec::new();
        for (_, composer) in &self.state.borrow().composers.0 {
            if !values.iter().any(|v| Arc::ptr_eq(v, composer)) {
                values.push(composer.clone());
            }
        }
        values
    }
    pub fn popup_mapping_names(&self) -> Vec<ComponentName> {
        self.state
            .borrow()
            .popup_loaders
            .0
            .iter()
            .map(|(k, _)| k.clone())
            .collect()
    }
    pub fn add_extension(
        &self,
        manifest: Rc<KeyboardExtensionManifest>,
        file_provider: ResourceFileProvider,
    ) -> ResourceResult<()> {
        // Validate current bridge limits before mutation. CLR null component names remain an
        // explicitly unsupported case, rather than silently converting null to an empty name.
        for meta in manifest.layouts.borrow().iter() {
            required(&meta.id, "null layout id")?;
        }
        for (id, _) in manifest.popup_mappings.borrow().iter() {
            required(id, "null popup id")?;
        }
        // Retired providers and composers can run user Drop code. Keep their
        // owners until the resource mutation (including partial failure) ends.
        let mut retired_layouts = Vec::new();
        let mut retired_popups = Vec::new();
        let mut retired_composers = Vec::new();
        {
            let mut state = self.state.borrow_mut();
            state.extensions.retain(|e| e.id != manifest.id);
            state.extensions.push(manifest.clone());
            for meta in manifest.layouts.borrow().iter() {
                let id = required(&meta.id, "null layout id")?;
                let relative = path(&[
                    &format!("layouts/{}/", meta.layout_type.to_id()?)
                        .as_str()
                        .into(),
                    &id,
                    &".json".into(),
                ]);
                let provider = file_provider.clone();
                let loader = Rc::new(move || match provider(&relative)? {
                    Some(json) => Ok(Some(LayoutJson::read_arrangement(&utf8(&json)?)?)),
                    None => Ok(Some(LayoutArrangement::empty())),
                });
                if let Some(previous) = state.layouts.insert(
                    (
                        meta.layout_type,
                        ComponentName::new(manifest.id.clone(), id),
                    ),
                    Rc::new(LayoutEntry {
                        metadata: meta.clone(),
                        extension_id: Some(manifest.id.clone()),
                        loader,
                        cached: RefCell::new(None),
                    }),
                ) {
                    retired_layouts.push(previous);
                }
            }
            for (id, authors) in manifest.popup_mappings.borrow().iter() {
                let id = required(id, "null popup id")?;
                let name = ComponentName::new(manifest.id.clone(), id.clone());
                let extension_id = manifest.id.clone();
                let authors = authors.clone();
                let provider = file_provider.clone();
                let relative = path(&[&"popupMappings/".into(), &id, &".json".into()]);
                state.popup_mappings.remove(&name);
                if let Some(previous) = state.popup_loaders.insert(
                    name,
                    Rc::new(move || {
                        let json = provider(&relative)?.unwrap_or_else(|| "{}".into());
                        Ok(Rc::new(LayoutJson::read_popup_mapping(
                            &utf8(&json)?,
                            id.clone(),
                            extension_id.clone(),
                            Some(authors.clone()),
                        )?))
                    }),
                ) {
                    retired_popups.push(previous);
                }
            }
            for set in manifest.currency_sets.borrow().iter() {
                state.currency_sets.insert(
                    ComponentName::new_nullable(Some(manifest.id.clone()), set.id()),
                    set.clone(),
                );
            }
            for composer in manifest.composers.borrow().iter() {
                if let Some(previous) = state.composers.insert(
                    ComponentName::new(manifest.id.clone(), composer.id.clone()),
                    Self::create_composer(composer),
                ) {
                    retired_composers.push(previous);
                }
            }
            for rule in manifest.punctuation_rules.borrow().iter() {
                state.punctuation_rules.insert(
                    ComponentName::new_nullable(Some(manifest.id.clone()), rule.id()),
                    rule.clone(),
                );
            }
            for subtype in manifest.subtype_presets.borrow().iter() {
                state.subtypes.retain(|s| s.id() != subtype.id());
                state.subtypes.push(subtype.clone());
            }
        }
        drop((retired_layouts, retired_popups, retired_composers));
        self.changed.raise(self);
        Ok(())
    }
    pub fn add_layout(&self, layout: Rc<KeyboardLayout>) {
        let metadata = Arc::new(LayoutMetadata {
            layout_type: layout.layout_type(),
            id: layout.id().clone(),
            label: layout.label().clone(),
            authors: layout.authors().clone(),
            direction: layout.direction(),
            modifier: layout.modifier().clone(),
        });
        let captured = layout.clone();
        let previous = self.state.borrow_mut().layouts.insert(
            (layout.layout_type(), layout.name()),
            Rc::new(LayoutEntry {
                metadata,
                extension_id: layout.extension_id().clone(),
                loader: Rc::new(move || Ok(captured.arrangement().clone())),
                cached: RefCell::new(Some(layout)),
            }),
        );
        drop(previous);
        self.changed.raise(self);
    }
    /// Explicit null extension argument, distinct from the omitted source default argument.
    pub fn add_layout_json_utf16(
        &self,
        json: Option<&Utf16Text>,
        extension_id: Option<Utf16Text>,
    ) -> ResourceResult<Rc<KeyboardLayout>> {
        let layout = Rc::new(LayoutJson::read_layout_document_utf16(json, extension_id)?);
        self.add_layout(layout.clone());
        Ok(layout)
    }
    /// Explicit null extension argument, distinct from the omitted source default argument.
    pub fn add_layout_json_nullable(
        &self,
        json: &str,
        extension_id: Option<Utf16Text>,
    ) -> ResourceResult<Rc<KeyboardLayout>> {
        let layout = Rc::new(LayoutJson::read_layout_document_nullable(
            json,
            extension_id,
        )?);
        self.add_layout(layout.clone());
        Ok(layout)
    }
    pub fn add_layout_json(
        &self,
        json: &str,
        extension_id: Option<Utf16Text>,
    ) -> ResourceResult<Rc<KeyboardLayout>> {
        let layout = Rc::new(LayoutJson::read_layout_document(
            json,
            extension_id.unwrap_or_else(|| Self::CUSTOM_EXTENSION.into()),
        )?);
        self.add_layout(layout.clone());
        Ok(layout)
    }
    pub fn add_popup_mapping(&self, mapping: Rc<PopupMapping>) {
        let name = ComponentName::new_nullable(mapping.extension_id(), mapping.id());
        let previous = {
            let mut state = self.state.borrow_mut();
            state.popup_mappings.insert(name.clone(), mapping.clone());
            state
                .popup_loaders
                .insert(name, Rc::new(move || Ok(mapping.clone())))
        };
        drop(previous);
        self.changed.raise(self);
    }
    pub fn add_subtype_preset(&self, subtype: Arc<Subtype>) {
        {
            let mut state = self.state.borrow_mut();
            state.subtypes.retain(|s| s.id() != subtype.id());
            state.subtypes.push(subtype);
        }
        self.changed.raise(self);
    }
    pub fn add_currency_set(&self, set: Rc<CurrencySet>) -> ResourceResult<()> {
        self.state.borrow_mut().currency_sets.insert(
            ComponentName::new_nullable(set.extension_id(), set.id()),
            set,
        );
        self.changed.raise(self);
        Ok(())
    }
    pub fn add_composer(&self, composer: Arc<dyn IComposer>, extension_id: Option<Utf16Text>) {
        // Obtain the virtual id without a RefCell borrow: the getter may reenter.
        let name = ComponentName::new(
            extension_id.unwrap_or_else(|| Self::CUSTOM_EXTENSION.into()),
            composer.id(),
        );
        let previous = self.state.borrow_mut().composers.insert(name, composer);
        // A custom composer's Drop may inspect or update these resources.
        // Its replacement is visible and the mutable borrow is released first.
        drop(previous);
        self.changed.raise(self);
    }
    pub fn add_punctuation_rule(&self, rule: Arc<PunctuationRule>) {
        self.add_punctuation_rule_nullable(Some(rule))
            .expect("nonnull punctuation rule");
    }
    pub fn add_punctuation_rule_nullable(
        &self,
        rule: Option<Arc<PunctuationRule>>,
    ) -> ResourceResult<()> {
        let mut state = self.state.borrow_mut();
        let rule = rule.ok_or(ResourceError::NullReference)?;
        state.punctuation_rules.insert(
            ComponentName::new_nullable(rule.extension_id(), rule.id()),
            rule,
        );
        drop(state);
        self.changed.raise(self);
        Ok(())
    }
    /// Source GetLayoutMetadata: a new stable sorted snapshot of the ORIGINAL metadata refs.
    /// Captures the caller's current culture once and does not invoke arrangement loaders.
    pub fn get_layout_metadata(
        &self,
        layout_type: LayoutType,
    ) -> ResourceResult<Vec<Arc<LayoutMetadata>>> {
        let name = crate::text::locale_text::current_locale_name()?;
        self.get_layout_metadata_for_culture(layout_type, &name)
    }
    /// Additional explicit-culture entry point for deterministic Core consumers/tests. The source
    /// public overload without a culture uses the host ambient resolver above.
    pub fn get_layout_metadata_for_culture(
        &self,
        layout_type: LayoutType,
        culture: &str,
    ) -> ResourceResult<Vec<Arc<LayoutMetadata>>> {
        use crate::text::locale_text::LocaleText;
        let compare = LocaleText::new(culture)?;
        let mut metadata: Vec<_> = self
            .state
            .borrow()
            .layouts
            .0
            .iter()
            .filter(|((kind, _), _)| *kind == layout_type)
            .map(|(_, entry)| entry.metadata.clone())
            .collect();
        // A fallible stable merge avoids swallowing comparator failures or returning an ordering
        // fallback after an error. It only rearranges this new snapshot, never registry entries.
        let count = metadata.len();
        let mut scratch = metadata.clone();
        let mut width = 1usize;
        while width < count {
            for start in (0..count).step_by(width.saturating_mul(2)) {
                let middle = (start + width).min(count);
                let end = (middle + width).min(count);
                let (mut left, mut right) = (start, middle);
                for slot in &mut scratch[start..end] {
                    let take_left = if left == middle {
                        false
                    } else if right == end {
                        true
                    } else {
                        match (&metadata[left].label, &metadata[right].label) {
                            (None, _) => true,
                            (Some(_), None) => false,
                            (Some(a), Some(b)) => {
                                compare.compare(a, b, true)? != std::cmp::Ordering::Greater
                            }
                        }
                    };
                    *slot = metadata[if take_left {
                        let i = left;
                        left += 1;
                        i
                    } else {
                        let i = right;
                        right += 1;
                        i
                    }]
                    .clone();
                }
            }
            std::mem::swap(&mut metadata, &mut scratch);
            width = width.saturating_mul(2);
        }
        Ok(metadata)
    }
    pub fn get_layout_names(&self, layout_type: LayoutType) -> Vec<ComponentName> {
        self.state
            .borrow()
            .layouts
            .0
            .iter()
            .filter(|((t, _), _)| *t == layout_type)
            .map(|((_, name), _)| name.clone())
            .collect()
    }
    pub fn get_layout(
        &self,
        layout_type: LayoutType,
        name: &ComponentName,
    ) -> ResourceResult<Option<Rc<KeyboardLayout>>> {
        let Some(entry) = self
            .state
            .borrow()
            .layouts
            .get(&(layout_type, name.clone()))
            .cloned()
        else {
            return Ok(None);
        };
        if let Some(cached) = entry.cached.borrow().clone() {
            return Ok(Some(cached));
        }
        let arrangement = (entry.loader)()?;
        let layout = Rc::new(KeyboardLayout::new_nullable(
            layout_type,
            entry.extension_id.clone(),
            entry.metadata.id.clone(),
            arrangement,
        ));
        layout.set_label(entry.metadata.label.clone());
        layout.set_authors(entry.metadata.authors.clone());
        layout.set_direction(entry.metadata.direction);
        layout.set_modifier(entry.metadata.modifier.clone());
        let cached = entry.cached.borrow_mut().get_or_insert(layout).clone();
        Ok(Some(cached))
    }
    pub fn get_popup_mapping(
        &self,
        name: &ComponentName,
    ) -> ResourceResult<Option<Rc<PopupMapping>>> {
        let loader = {
            let state = self.state.borrow();
            if let Some(cached) = state.popup_mappings.get(name) {
                return Ok(Some(cached.clone()));
            }
            state.popup_loaders.get(name).cloned()
        };
        let Some(loader) = loader else {
            return Ok(None);
        };
        let mapping = loader()?;
        self.state
            .borrow_mut()
            .popup_mappings
            .insert(name.clone(), mapping.clone());
        Ok(Some(mapping))
    }
    pub fn get_currency_set(&self, name: &ComponentName) -> Option<Rc<CurrencySet>> {
        self.state.borrow().currency_sets.get(name).cloned()
    }
    pub fn get_composer(&self, name: &ComponentName) -> Arc<dyn IComposer> {
        self.state
            .borrow()
            .composers
            .get(name)
            .cloned()
            .unwrap_or_else(|| AppenderComposer::instance())
    }
    pub fn get_punctuation_rule(&self, name: &ComponentName) -> Arc<PunctuationRule> {
        self.state
            .borrow()
            .punctuation_rules
            .get(name)
            .cloned()
            .unwrap_or_else(PunctuationRule::shared_default)
    }
    // Arc retains the existing process-shared Appender identity. It does NOT promise
    // Send/Sync for this erased interface: custom IComposer implementations remain unrestricted.
    // Builtin concrete composers and their retained ComposerRules now support shared ownership.
    pub fn create_composer(definition: &ComposerDefinition) -> Arc<dyn IComposer> {
        if definition.kind == "hangul-unicode".into() {
            Arc::new(HangulComposer)
        } else if definition.kind == "kana-unicode".into() {
            Arc::new(KanaComposer)
        } else if definition.kind == "with-rules".into() {
            Arc::new(RulesComposer::new(
                definition.id.clone(),
                definition.label.clone(),
                definition.rules.clone(),
            ))
        } else {
            AppenderComposer::instance()
        }
    }
}
