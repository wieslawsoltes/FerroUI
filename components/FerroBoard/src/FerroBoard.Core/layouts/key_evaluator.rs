//! Native evaluator values and shared UI handles from KeyData.cs.
use super::IKeyEvaluator;
use crate::{keys::*, text::TextCulture};
use std::{
    cell::RefCell,
    rc::{Rc, Weak},
    sync::{
        Arc, LazyLock,
        atomic::{AtomicBool, AtomicI32, Ordering},
    },
};
#[derive(Clone, Debug)]
pub struct KeyEvaluatorValues {
    pub mode: KeyboardMode,
    pub shift_state: ShiftState,
    pub variation: KeyVariation,
    pub direction: LayoutDirection,
    pub is_char_half_width: bool,
    pub is_kana_kata: bool,
    pub culture_override: Option<TextCulture>,
}
impl Default for KeyEvaluatorValues {
    fn default() -> Self {
        Self {
            mode: KeyboardMode::Characters,
            shift_state: ShiftState::Unshifted,
            variation: KeyVariation::Normal,
            direction: LayoutDirection::Ltr,
            is_char_half_width: false,
            is_kana_kata: false,
            culture_override: None,
        }
    }
}
/// Shared default configuration. Native locale values are safe to share across threads.
/// The source object identity is this process-wide Arc, never a per-thread singleton.
#[doc(hidden)]
pub struct SharedKeyEvaluator {
    mode: AtomicI32,
    shift_state: AtomicI32,
    variation: AtomicI32,
    direction: AtomicI32,
    is_char_half_width: AtomicBool,
    is_kana_kata: AtomicBool,
    culture_override: parking_lot::RwLock<Option<TextCulture>>,
}
impl SharedKeyEvaluator {
    pub fn as_evaluator(self: &Arc<Self>) -> KeyEvaluator {
        KeyEvaluator::from_shared(self.clone())
    }

    pub fn values(&self) -> KeyEvaluatorValues {
        KeyEvaluatorValues {
            mode: self.mode(),
            shift_state: self.shift_state(),
            variation: self.variation(),
            direction: self.direction(),
            is_char_half_width: self.is_char_half_width(),
            is_kana_kata: self.is_kana_kata(),
            culture_override: self.culture_override(),
        }
    }
    fn set_mode(&self, value: KeyboardMode) {
        self.mode.store(value.0, Ordering::Relaxed);
    }
    fn set_shift_state(&self, value: ShiftState) {
        self.shift_state.store(value.0, Ordering::Relaxed);
    }
    fn set_variation(&self, value: KeyVariation) {
        self.variation.store(value.0, Ordering::Relaxed);
    }
    fn set_direction(&self, value: LayoutDirection) {
        self.direction.store(value.0, Ordering::Relaxed);
    }
    fn set_is_char_half_width(&self, value: bool) {
        self.is_char_half_width.store(value, Ordering::Relaxed);
    }
    fn set_is_kana_kata(&self, value: bool) {
        self.is_kana_kata.store(value, Ordering::Relaxed);
    }
    pub fn culture_override(&self) -> Option<TextCulture> {
        *self.culture_override.read()
    }
}
impl PartialEq for SharedKeyEvaluator {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
            || self.mode() == other.mode()
                && self.shift_state() == other.shift_state()
                && self.variation() == other.variation()
                && self.direction() == other.direction()
                && self.is_char_half_width() == other.is_char_half_width()
                && self.is_kana_kata() == other.is_kana_kata()
                && self.culture_override() == other.culture_override()
    }
}

impl std::fmt::Debug for SharedKeyEvaluator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("KeyEvaluator(Default)")
    }
}
impl IKeyEvaluator for SharedKeyEvaluator {
    fn mode(&self) -> KeyboardMode {
        KeyboardMode(self.mode.load(Ordering::Relaxed))
    }
    fn shift_state(&self) -> ShiftState {
        ShiftState(self.shift_state.load(Ordering::Relaxed))
    }
    fn variation(&self) -> KeyVariation {
        KeyVariation(self.variation.load(Ordering::Relaxed))
    }
    fn direction(&self) -> LayoutDirection {
        LayoutDirection(self.direction.load(Ordering::Relaxed))
    }
    fn is_char_half_width(&self) -> bool {
        self.is_char_half_width.load(Ordering::Relaxed)
    }
    fn is_kana_kata(&self) -> bool {
        self.is_kana_kata.load(Ordering::Relaxed)
    }
    fn culture(&self) -> Option<TextCulture> {
        Some(
            self.culture_override()
                .unwrap_or_else(TextCulture::invariant),
        )
    }
}
enum Storage {
    Local(RefCell<KeyEvaluatorValues>),
    Shared(Arc<SharedKeyEvaluator>),
}
#[doc(hidden)]
pub struct KeyEvaluatorObject {
    self_weak: Weak<KeyEvaluatorObject>,
    storage: Storage,
}
#[derive(Clone)]
pub struct KeyEvaluator {
    inner: Rc<KeyEvaluatorObject>,
}
pub struct WeakKeyEvaluator {
    inner: WeakEvaluator,
}
enum WeakEvaluator {
    Local(Weak<KeyEvaluatorObject>),
    Shared(std::sync::Weak<SharedKeyEvaluator>),
}
impl WeakKeyEvaluator {
    pub fn ptr_eq(&self, other: &Self) -> bool {
        match (&self.inner, &other.inner) {
            (WeakEvaluator::Local(a), WeakEvaluator::Local(b)) => a.ptr_eq(b),
            (WeakEvaluator::Shared(a), WeakEvaluator::Shared(b)) => a.ptr_eq(b),
            _ => false,
        }
    }
    pub fn upgrade(&self) -> Option<KeyEvaluator> {
        match &self.inner {
            WeakEvaluator::Local(v) => v.upgrade().map(|inner| KeyEvaluator { inner }),
            WeakEvaluator::Shared(v) => v.upgrade().map(KeyEvaluator::from_shared),
        }
    }
}
ferroui_system::ferro_thread_local! {static DEFAULT_VIEW:RefCell<Weak<KeyEvaluatorObject>>=const{RefCell::new(Weak::new())};}
impl Default for KeyEvaluator {
    fn default() -> Self {
        Self::from_values(KeyEvaluatorValues::default())
    }
}
impl std::fmt::Debug for KeyEvaluator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("KeyEvaluator")
            .field(&Rc::as_ptr(&self.inner))
            .finish()
    }
}
impl KeyEvaluator {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        mode: KeyboardMode,
        shift_state: ShiftState,
        variation: KeyVariation,
        direction: LayoutDirection,
        is_char_half_width: bool,
        is_kana_kata: bool,
        culture_override: Option<TextCulture>,
    ) -> Self {
        Self::from_values(KeyEvaluatorValues {
            mode,
            shift_state,
            variation,
            direction,
            is_char_half_width,
            is_kana_kata,
            culture_override,
        })
    }
    pub fn from_values(values: KeyEvaluatorValues) -> Self {
        Self {
            inner: Rc::new_cyclic(|self_weak| KeyEvaluatorObject {
                self_weak: self_weak.clone(),
                storage: Storage::Local(RefCell::new(values)),
            }),
        }
    }
    pub fn from_original(inner: Rc<KeyEvaluatorObject>) -> Self {
        Self { inner }
    }
    pub fn original_object(&self) -> Rc<KeyEvaluatorObject> {
        self.inner.clone()
    }
    pub fn shared_default() -> Arc<SharedKeyEvaluator> {
        static VALUE: LazyLock<Arc<SharedKeyEvaluator>> = LazyLock::new(|| {
            Arc::new(SharedKeyEvaluator {
                mode: AtomicI32::new(KeyboardMode::Characters.0),
                shift_state: AtomicI32::new(ShiftState::Unshifted.0),
                variation: AtomicI32::new(KeyVariation::Normal.0),
                direction: AtomicI32::new(LayoutDirection::Ltr.0),
                is_char_half_width: AtomicBool::new(false),
                is_kana_kata: AtomicBool::new(false),
                culture_override: parking_lot::RwLock::new(None),
            })
        });
        VALUE.clone()
    }
    pub fn default_reference() -> Self {
        Self::from_shared(Self::shared_default())
    }
    fn from_shared(value: Arc<SharedKeyEvaluator>) -> Self {
        DEFAULT_VIEW.with_borrow_mut(|view| {
            if let Some(inner) = view.upgrade() {
                return Self { inner };
            }
            let inner = Rc::new_cyclic(|self_weak| KeyEvaluatorObject {
                self_weak: self_weak.clone(),
                storage: Storage::Shared(value),
            });
            *view = Rc::downgrade(&inner);
            Self { inner }
        })
    }
    pub fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
            || match (&self.inner.storage, &other.inner.storage) {
                (Storage::Shared(a), Storage::Shared(b)) => Arc::ptr_eq(a, b),
                _ => false,
            }
    }
    pub fn downgrade(&self) -> WeakKeyEvaluator {
        WeakKeyEvaluator {
            inner: match &self.inner.storage {
                Storage::Local(_) => WeakEvaluator::Local(Rc::downgrade(&self.inner)),
                Storage::Shared(v) => WeakEvaluator::Shared(Arc::downgrade(v)),
            },
        }
    }
    pub fn values(&self) -> KeyEvaluatorValues {
        self.inner.values()
    }

    pub fn culture_override(&self) -> Option<TextCulture> {
        self.inner.culture_override()
    }
    pub fn set_culture_override(&self, value: Option<TextCulture>) {
        match &self.inner.storage {
            Storage::Local(v) => v.borrow_mut().culture_override = value,
            Storage::Shared(v) => *v.culture_override.write() = value,
        }
    }
}
macro_rules! fields {
 ($($name:ident,$init:ident:$t:ty;)*)=>{$(
 impl KeyEvaluator {pub fn $name(&self)->$t{self.inner.$name()}pub fn $init(&self,value:$t){match &self.inner.storage{Storage::Local(v)=>{v.borrow_mut().$name=value;},Storage::Shared(v)=>{v.$init(value);}}}}
 )*};
}
fields! {mode,set_mode:KeyboardMode;shift_state,set_shift_state:ShiftState;variation,set_variation:KeyVariation;direction,set_direction:LayoutDirection;is_char_half_width,set_is_char_half_width:bool;is_kana_kata,set_is_kana_kata:bool;}
impl KeyEvaluatorObject {
    pub fn downgrade(&self) -> WeakKeyEvaluator {
        WeakKeyEvaluator {
            inner: match &self.storage {
                Storage::Local(_) => WeakEvaluator::Local(self.self_weak.clone()),
                Storage::Shared(v) => WeakEvaluator::Shared(Arc::downgrade(v)),
            },
        }
    }

    fn values(&self) -> KeyEvaluatorValues {
        match &self.storage {
            Storage::Local(v) => v.borrow().clone(),
            Storage::Shared(v) => v.values(),
        }
    }
    fn culture_override(&self) -> Option<TextCulture> {
        match &self.storage {
            Storage::Local(v) => v.borrow().culture_override,
            Storage::Shared(v) => v.culture_override(),
        }
    }
}
impl PartialEq for KeyEvaluatorObject {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
            || self.mode() == other.mode()
                && self.shift_state() == other.shift_state()
                && self.variation() == other.variation()
                && self.direction() == other.direction()
                && self.is_char_half_width() == other.is_char_half_width()
                && self.is_kana_kata() == other.is_kana_kata()
                && self.culture_override() == other.culture_override()
    }
}
impl PartialEq for KeyEvaluator {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other) || self.inner.as_ref() == other.inner.as_ref()
    }
}

impl PartialEq<KeyEvaluator> for SharedKeyEvaluator {
    fn eq(&self, other: &KeyEvaluator) -> bool {
        KeyEvaluator::default_reference() == *other
    }
}
impl PartialEq<SharedKeyEvaluator> for KeyEvaluator {
    fn eq(&self, _: &SharedKeyEvaluator) -> bool {
        *self == KeyEvaluator::default_reference()
    }
}

impl IKeyEvaluator for KeyEvaluatorObject {
    fn mode(&self) -> KeyboardMode {
        match &self.storage {
            Storage::Local(v) => v.borrow().mode,
            Storage::Shared(v) => v.mode(),
        }
    }
    fn shift_state(&self) -> ShiftState {
        match &self.storage {
            Storage::Local(v) => v.borrow().shift_state,
            Storage::Shared(v) => v.shift_state(),
        }
    }
    fn variation(&self) -> KeyVariation {
        match &self.storage {
            Storage::Local(v) => v.borrow().variation,
            Storage::Shared(v) => v.variation(),
        }
    }
    fn direction(&self) -> LayoutDirection {
        match &self.storage {
            Storage::Local(v) => v.borrow().direction,
            Storage::Shared(v) => v.direction(),
        }
    }
    fn is_char_half_width(&self) -> bool {
        match &self.storage {
            Storage::Local(v) => v.borrow().is_char_half_width,
            Storage::Shared(v) => v.is_char_half_width(),
        }
    }
    fn is_kana_kata(&self) -> bool {
        match &self.storage {
            Storage::Local(v) => v.borrow().is_kana_kata,
            Storage::Shared(v) => v.is_kana_kata(),
        }
    }
    fn culture(&self) -> Option<TextCulture> {
        Some(
            self.culture_override()
                .unwrap_or_else(TextCulture::invariant),
        )
    }
}
impl IKeyEvaluator for KeyEvaluator {
    fn mode(&self) -> KeyboardMode {
        self.inner.mode()
    }
    fn shift_state(&self) -> ShiftState {
        self.inner.shift_state()
    }
    fn variation(&self) -> KeyVariation {
        self.inner.variation()
    }
    fn direction(&self) -> LayoutDirection {
        self.inner.direction()
    }
    fn is_char_half_width(&self) -> bool {
        self.inner.is_char_half_width()
    }
    fn is_kana_kata(&self) -> bool {
        self.inner.is_kana_kata()
    }
    fn culture(&self) -> Option<TextCulture> {
        self.inner.culture()
    }
}
