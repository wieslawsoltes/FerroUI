//! Decode a binary leaf and dispatch to exactly the factory selected by the compiler.
//! The codec validates the entire block before the first dictionary insertion.

use super::{theme_resources, DocumentInfo, XamlIlContext, XamlLoadError};
use ferroui_base::{controls::ResourceDictionary, Ref};
use ferroui_xaml_binary::Payload;
pub use ferroui_xaml_binary::{fingerprint, Batch, Kind, MAX_BLOCK_BYTES};
use std::rc::Rc;

/// An individually linkable compiled block. The stable identifier is used by reload
/// clients; the module-qualified name disambiguates identical class names across crates.
pub struct Block {
    id: &'static str,
    bytes: &'static [u8],
}

impl Block {
    pub const fn new(id: &'static str, bytes: &'static [u8]) -> Self {
        Self { id, bytes }
    }

    fn decode(&self, expected: Kind) -> Result<Rc<Batch>, XamlLoadError> {
        #[cfg(all(feature = "hot-reload", debug_assertions))]
        if let Some(batch) = reload::get(self.id) {
            return checked(self.id, expected, batch);
        }
        let batch = Batch::decode(self.bytes).map_err(|e| error(self.id, e))?;
        checked(self.id, expected, Rc::new(batch))
    }
}

fn error(id: &str, message: impl std::fmt::Display) -> XamlLoadError {
    XamlLoadError::with_message(format!(
        "Invalid compiled XAML resource block '{id}': {message}"
    ))
}

fn checked(id: &str, expected: Kind, batch: Rc<Batch>) -> Result<Rc<Batch>, XamlLoadError> {
    if batch.kind() == expected {
        Ok(batch)
    } else {
        Err(error(id, "factory kind differs from compiled call site"))
    }
}

macro_rules! load {
    ($name:ident, $kind:ident, $payload:ident, $register:ident) => {
        #[inline(never)]
        pub fn $name(
            dictionary: &Ref<ResourceDictionary>,
            context: &Rc<XamlIlContext>,
            document: &'static DocumentInfo,
            block: Block,
        ) -> Result<(), XamlLoadError> {
            let batch = block.decode(Kind::$kind)?;
            for row in batch.records() {
                let Payload::$payload(value) = row.payload else {
                    unreachable!("validated homogeneous block")
                };
                theme_resources::$register(
                    dictionary,
                    context,
                    document,
                    row.key,
                    value,
                    row.line,
                    row.column,
                    row.value_line,
                    row.value_column,
                )?;
            }
            Ok(())
        }
    };
}

load!(aliases, Alias, Text, alias);
load!(static_brushes, StaticBrush, Text, static_brush);
load!(literal_brushes, LiteralBrush, Argb, literal_brush);
load!(dynamic_brushes, DynamicBrush, Text, dynamic_brush);

/// UI-thread candidate image for repopulating a theme. Validate edits before activation.
/// Deferred values created by an older population retain that generation's string pool.
/// A host may retry construction under the previous snapshot if the new theme fails.
#[cfg(all(feature = "hot-reload", debug_assertions))]
pub mod reload {
    use super::*;
    use std::{cell::RefCell, collections::BTreeMap};

    #[derive(Default, Clone)]
    pub struct Snapshot {
        blocks: BTreeMap<String, Rc<Batch>>,
    }

    ferroui_base::ferro_thread_local! {
        static ACTIVE: RefCell<Option<Rc<Snapshot>>> = const { RefCell::new(None) };
    }

    impl Snapshot {
        /// Atomically replace a validated block in this candidate generation. Changing its
        /// factory kind changes the generated Rust ABI and requires recompiling the caller.
        pub fn replace(
            &mut self,
            id: &str,
            expected: Kind,
            bytes: &[u8],
        ) -> Result<(), XamlLoadError> {
            self.replace_decoded(
                id,
                expected,
                Batch::decode(bytes).map_err(|e| error(id, e))?,
            )
        }

        /// Stage an image already decoded and validated on a worker thread.
        pub fn replace_decoded(
            &mut self,
            id: &str,
            expected: Kind,
            batch: Batch,
        ) -> Result<(), XamlLoadError> {
            self.blocks
                .insert(id.to_owned(), checked(id, expected, Rc::new(batch))?);
            Ok(())
        }

        /// Scope this candidate to one reconstruction; nesting and unwinding restore the
        /// previous snapshot without leaving a stale thread-local pointer.
        pub fn run<T>(self: &Rc<Self>, build: impl FnOnce() -> T) -> T {
            with_snapshot(Some(self.clone()), build)
        }
    }

    pub(crate) fn capture() -> Option<Rc<Snapshot>> {
        ACTIVE.with(|slot| slot.borrow().clone())
    }

    pub(crate) fn with_snapshot<T>(snapshot: Option<Rc<Snapshot>>, build: impl FnOnce() -> T) -> T {
        struct Restore(Option<Rc<Snapshot>>);
        impl Drop for Restore {
            fn drop(&mut self) {
                ACTIVE.with(|slot| {
                    slot.replace(self.0.take());
                });
            }
        }
        let _guard = Restore(ACTIVE.with(|slot| slot.replace(snapshot)));
        build()
    }

    pub(super) fn get(id: &str) -> Option<Rc<Batch>> {
        ACTIVE.with(|slot| slot.borrow().as_ref()?.blocks.get(id).cloned())
    }
}

#[cfg(all(test, feature = "hot-reload", debug_assertions))]
mod tests {
    use super::*;

    // One literal with key "A", two source positions (1,1) and zero deltas.
    const BLUE: &[u8] = b"FXB\0\x01\x02\x01\x01\x00\x01A\x00\xff\x00\x00\xff\x01\x01\x00\x00";
    const BLOCK: Block = Block::new("app/brushes", BLUE);

    fn color() -> u32 {
        let batch = BLOCK.decode(Kind::LiteralBrush).unwrap();
        let row = batch.records().next().unwrap();
        let Payload::Argb(color) = row.payload else {
            unreachable!()
        };
        color
    }

    #[test]
    fn candidate_validation_and_nested_reload_restore_the_previous_generation() {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        let mut candidate = reload::Snapshot::default();
        let mut green = BLUE.to_vec();
        green[12..16].copy_from_slice(&0xff00ff00u32.to_le_bytes());
        candidate
            .replace(BLOCK.id, Kind::LiteralBrush, &green)
            .unwrap();
        assert!(candidate
            .replace(BLOCK.id, Kind::LiteralBrush, &green[..16])
            .is_err());
        assert!(candidate.replace(BLOCK.id, Kind::Alias, BLUE).is_err());
        let candidate = Rc::new(candidate);
        // A deferred template can contain another compiled dictionary. Its body must
        // observe the declaration's candidate even when realized under a different one.
        use crate::test_support::TestServiceProvider;
        use crate::xaml_il::runtime::{DeferredContentBuilder, XamlIlRuntimeHelpers};
        use ferroui_base::data::core::ValueType;
        let declare = || {
            XamlIlRuntimeHelpers::deferred_transformation_factory_for(
                ValueType::object(),
                DeferredContentBuilder::new(|_| Some(Rc::new(color()))),
                &TestServiceProvider::new()
                    .with_parents(vec![])
                    .with_root(None)
                    .sp(),
            )
        };
        let original = declare();
        let edited = candidate.run(declare);
        let resolved = |value: &Rc<crate::xaml_il::runtime::DeferredContent>| {
            *value
                .build_with(None)
                .result
                .unwrap()
                .downcast_ref::<u32>()
                .unwrap()
        };
        assert_eq!(resolved(&edited), 0xff00ff00);
        candidate.run(|| assert_eq!(resolved(&original), 0xff0000ff));
        candidate.run(|| {
            assert_eq!(color(), 0xff00ff00);
            Rc::new(reload::Snapshot::default()).run(|| assert_eq!(color(), 0xff0000ff));
            assert_eq!(color(), 0xff00ff00);
        });
        assert!(catch_unwind(AssertUnwindSafe(|| {
            candidate.run(|| panic!("abort candidate construction"));
        }))
        .is_err());
        assert_eq!(color(), 0xff0000ff);
    }
}
