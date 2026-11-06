//! Engine/LanguageModels.cs: synchronized native registries with lazy dictionary initialization.
use crate::{
    events::SharedEvent,
    text::{DictionaryError, OrdinalKey, TextCulture, UserDictionary, Utf16Text, WordDictionary},
};
use parking_lot::{Mutex, ReentrantMutex};
use std::{any::Any, cell::RefCell, collections::HashMap, sync::Arc};
/// Preserves a Rust panic payload when a lazy factory fails. It is not silently converted to a value.
pub struct FactoryPanic {
    payload: Mutex<Box<dyn Any + Send>>,
}
impl FactoryPanic {
    pub fn inspect<R>(&self, inspect: impl FnOnce(&(dyn Any + Send)) -> R) -> R {
        inspect(&**self.payload.lock())
    }
}
impl std::fmt::Debug for FactoryPanic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FactoryPanic { payload: retained }")
    }
}
#[derive(Debug)]
pub enum LanguageModelError {
    NullReference,
    NullFactory,
    RecursiveFactory,
    Dictionary(DictionaryError),
    Factory(String),
    Panic(FactoryPanic),
}
impl std::fmt::Display for LanguageModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NullReference => f.write_str("required language-model data is missing"),
            _ => write!(f, "{self:?}"),
        }
    }
}
impl std::error::Error for LanguageModelError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Dictionary(error) => Some(error),
            _ => None,
        }
    }
}
pub type LanguageModelResult<T> = Result<T, Arc<LanguageModelError>>;
pub type DictionaryFactory =
    Arc<dyn Fn() -> LanguageModelResult<Option<Arc<WordDictionary>>> + Send + Sync>;
enum LazyState {
    Pending(DictionaryFactory),
    Running,
    Complete(LanguageModelResult<Option<Arc<WordDictionary>>>),
}
struct LazyDictionary {
    initializer: ReentrantMutex<()>,
    state: Mutex<LazyState>,
}
impl LazyDictionary {
    fn new(factory: DictionaryFactory) -> Self {
        Self {
            initializer: ReentrantMutex::new(()),
            state: Mutex::new(LazyState::Pending(factory)),
        }
    }
    fn is_created(&self) -> bool {
        matches!(&*self.state.lock(), LazyState::Complete(Ok(_)))
    }
    fn value(&self) -> LanguageModelResult<Option<Arc<WordDictionary>>> {
        // Completed failures can be observed while an outer factory still holds its initializer
        // lock after catching a recursive-access exception. This fast path is intentional.
        if let LazyState::Complete(result) = &*self.state.lock() {
            return result.clone();
        }
        let _initializer = self.initializer.lock();
        let factory = {
            let mut state = self.state.lock();
            match &*state {
                LazyState::Complete(result) => return result.clone(),
                LazyState::Running => {
                    let error = Arc::new(LanguageModelError::RecursiveFactory);
                    *state = LazyState::Complete(Err(error.clone()));
                    return Err(error);
                }
                LazyState::Pending(factory) => {
                    let factory = factory.clone();
                    *state = LazyState::Running;
                    factory
                }
            }
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| factory()))
            .unwrap_or_else(|payload| {
                Err(Arc::new(LanguageModelError::Panic(FactoryPanic {
                    payload: Mutex::new(payload),
                })))
            });
        *self.state.lock() = LazyState::Complete(result.clone());
        result
    }
}
#[derive(Default)]
struct State {
    // Hash indices give native O(1) lookup; dense entries preserve first registration order.
    // These registries never remove entries, so replacement keeps both key spelling and position.
    dictionary_indices: HashMap<OrdinalKey, usize>,
    dictionaries: Vec<(OrdinalKey, Arc<LazyDictionary>)>,
    user_indices: HashMap<OrdinalKey, usize>,
    users: Vec<(OrdinalKey, UserDictionary)>,
}
pub struct LanguageUserDictionaryChanged {
    pub sender: Arc<LanguageModels>,
    pub language: Utf16Text,
}
pub struct UserDictionarySnapshot {
    indices: HashMap<OrdinalKey, usize>,
    values: Vec<(OrdinalKey, UserDictionary)>,
}
impl UserDictionarySnapshot {
    pub fn len(&self) -> usize {
        self.values.len()
    }
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
    pub fn get(&self, language: &Utf16Text) -> Option<&UserDictionary> {
        self.indices
            .get(&OrdinalKey(language.clone()))
            .map(|&index| &self.values[index].1)
    }
    pub fn iter(&self) -> impl Iterator<Item = (&Utf16Text, &UserDictionary)> {
        self.values.iter().map(|(k, v)| (&k.0, v))
    }
}
pub struct LanguageModels {
    gate: ReentrantMutex<RefCell<State>>,
    user_dictionary_changed: SharedEvent<LanguageUserDictionaryChanged>,
}
impl PartialEq for LanguageModels {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}
impl Eq for LanguageModels {}
impl LanguageModels {
    pub fn new() -> Arc<Self> {
        let models = Arc::new(Self {
            gate: ReentrantMutex::new(RefCell::new(State::default())),
            user_dictionary_changed: SharedEvent::default(),
        });
        models
            .register_factory(
                &"en".into(),
                Some(Arc::new(|| {
                    WordDictionary::load_bundled_english()
                        .map(|d| Some(Arc::new(d)))
                        .map_err(|e| Arc::new(LanguageModelError::Dictionary(e)))
                })),
            )
            .expect("bundled factory is present");
        models
    }
    pub fn user_dictionary_changed(&self) -> &SharedEvent<LanguageUserDictionaryChanged> {
        &self.user_dictionary_changed
    }
    pub fn register_factory(
        &self,
        language: &Utf16Text,
        factory: Option<DictionaryFactory>,
    ) -> LanguageModelResult<()> {
        let guard = self.gate.lock();
        let key = OrdinalKey(normalize(language));
        let factory = factory.ok_or_else(|| Arc::new(LanguageModelError::NullFactory))?;
        let previous = {
            let mut state = guard.borrow_mut();
            let dictionary = Arc::new(LazyDictionary::new(factory));
            if let Some(&index) = state.dictionary_indices.get(&key) {
                Some(std::mem::replace(
                    &mut state.dictionaries[index].1,
                    dictionary,
                ))
            } else {
                let index = state.dictionaries.len();
                state.dictionary_indices.insert(key.clone(), index);
                state.dictionaries.push((key, dictionary));
                None
            }
        };
        drop(guard);
        drop(previous);
        Ok(())
    }
    pub fn register_dictionary(
        &self,
        language: &Utf16Text,
        dictionary: Option<Arc<WordDictionary>>,
    ) {
        self.register_factory(language, Some(Arc::new(move || Ok(dictionary.clone()))))
            .expect("instance factory is present");
    }
    pub fn languages(&self) -> Vec<Utf16Text> {
        self.gate
            .lock()
            .borrow()
            .dictionaries
            .iter()
            .map(|(key, _)| key.0.clone())
            .collect()
    }
    pub fn has_dictionary(&self, language_tag: &Utf16Text) -> bool {
        self.has_dictionary_nullable(Some(language_tag))
            .expect("nonnull language tag")
    }
    pub fn has_dictionary_nullable(
        &self,
        language_tag: Option<&Utf16Text>,
    ) -> LanguageModelResult<bool> {
        let guard = self.gate.lock();
        let language_tag =
            language_tag.ok_or_else(|| Arc::new(LanguageModelError::NullReference))?;
        Ok(guard
            .borrow()
            .dictionary_indices
            .contains_key(&OrdinalKey(normalize(language_tag))))
    }
    pub fn get_dictionary(
        &self,
        language_tag: &Utf16Text,
    ) -> LanguageModelResult<Option<Arc<WordDictionary>>> {
        self.get_dictionary_nullable(Some(language_tag))
    }
    pub fn get_dictionary_nullable(
        &self,
        language_tag: Option<&Utf16Text>,
    ) -> LanguageModelResult<Option<Arc<WordDictionary>>> {
        let lazy = {
            let guard = self.gate.lock();
            let language_tag =
                language_tag.ok_or_else(|| Arc::new(LanguageModelError::NullReference))?;
            let state = guard.borrow();
            state
                .dictionary_indices
                .get(&OrdinalKey(normalize(language_tag)))
                .map(|&index| state.dictionaries[index].1.clone())
        };
        match lazy {
            Some(lazy) => lazy.value(),
            None => Ok(None),
        }
    }
    pub fn is_loaded(&self, language_tag: &Utf16Text) -> bool {
        self.is_loaded_nullable(Some(language_tag))
            .expect("nonnull language tag")
    }
    pub fn is_loaded_nullable(
        &self,
        language_tag: Option<&Utf16Text>,
    ) -> LanguageModelResult<bool> {
        let guard = self.gate.lock();
        let language_tag =
            language_tag.ok_or_else(|| Arc::new(LanguageModelError::NullReference))?;
        let state = guard.borrow();
        Ok(state
            .dictionary_indices
            .get(&OrdinalKey(normalize(language_tag)))
            .is_some_and(|&index| state.dictionaries[index].1.is_created()))
    }
    pub fn get_user_dictionary(self: &Arc<Self>, language_tag: &Utf16Text) -> UserDictionary {
        self.get_user_dictionary_nullable(Some(language_tag))
            .expect("nonnull language tag")
    }
    pub fn get_user_dictionary_nullable(
        self: &Arc<Self>,
        language_tag: Option<&Utf16Text>,
    ) -> LanguageModelResult<UserDictionary> {
        let language_tag =
            language_tag.ok_or_else(|| Arc::new(LanguageModelError::NullReference))?;
        let language = normalize(language_tag);
        let key = OrdinalKey(language.clone());
        let guard = self.gate.lock();
        {
            let state = guard.borrow();
            if let Some(&index) = state.user_indices.get(&key) {
                return Ok(state.users[index].1.retaining(self.clone()));
            }
        }
        let user = UserDictionary::default();
        let owner = Arc::downgrade(self);
        user.changed().add(Arc::new(move |_| {
            // Public user handles retain the owner. The cache uses an ownerless handle to avoid
            // a strong cycle; this weak edge cannot fail during a public user operation.
            let owner = owner
                .upgrade()
                .expect("language user handle retains its registry");
            owner
                .user_dictionary_changed
                .raise(&LanguageUserDictionaryChanged {
                    sender: owner.clone(),
                    language: language.clone(),
                });
        }));
        {
            let mut state = guard.borrow_mut();
            let index = state.users.len();
            state.user_indices.insert(key.clone(), index);
            state.users.push((key, user.clone()));
        }
        Ok(user.retaining(self.clone()))
    }
    pub fn user_dictionaries(self: &Arc<Self>) -> UserDictionarySnapshot {
        let values: Vec<_> = self
            .gate
            .lock()
            .borrow()
            .users
            .iter()
            .map(|(key, user)| (key.clone(), user.retaining(self.clone())))
            .collect();
        let indices = values
            .iter()
            .enumerate()
            .map(|(index, (key, _))| (key.clone(), index))
            .collect();
        UserDictionarySnapshot { indices, values }
    }
}
fn normalize(tag: &Utf16Text) -> Utf16Text {
    let prefix = tag
        .units()
        .split(|&c| c == b'-' as u16 || c == b'_' as u16)
        .next()
        .expect("split always has a first segment");
    TextCulture::to_lower_invariant(&Utf16Text::from_units(prefix.to_vec()))
}
