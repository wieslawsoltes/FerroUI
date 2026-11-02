//! Direct-mapped slots name compiled functions on the private worker protocol.
//! Full signatures resolve hash collisions. Nested invocations bypass these slots.
use std::hash::{DefaultHasher, Hash, Hasher};

const SLOTS: usize = 128;
const MAX_BYTES: usize = 64 * 1024;

#[derive(Hash, PartialEq)]
pub(super) struct Signature<'a> {
    pub source: &'a str,
    pub form: &'a str,
    pub origin: &'a str,
    pub hook: Option<&'a str>,
    pub parameters: &'a [String],
}

struct Program {
    source: String,
    form: String,
    origin: String,
    hook: Option<String>,
    parameters: Vec<String>,
}
impl Program {
    fn signature(&self) -> Signature<'_> {
        Signature {
            source: &self.source,
            form: &self.form,
            origin: &self.origin,
            hook: self.hook.as_deref(),
            parameters: &self.parameters,
        }
    }
}

#[derive(Default)]
pub(super) struct Programs(Vec<Option<Program>>);
impl Programs {
    pub fn register(&mut self, signature: Signature<'_>) -> Option<(usize, bool)> {
        let bytes = signature.source.len()
            + signature.origin.len()
            + signature.hook.map_or(0, str::len)
            + signature
                .parameters
                .iter()
                .map(|p| size_of::<String>() + p.len())
                .sum::<usize>();
        if bytes > MAX_BYTES {
            return None;
        }
        let mut hash = DefaultHasher::new();
        signature.hash(&mut hash);
        let index = hash.finish() as usize % SLOTS;
        if self.0.is_empty() {
            self.0.resize_with(SLOTS, || None);
        }
        let slot = &mut self.0[index];
        if slot.as_ref().is_some_and(|p| p.signature() == signature) {
            return Some((index, true));
        }
        *slot = Some(Program {
            source: signature.source.into(),
            form: signature.form.into(),
            origin: signature.origin.into(),
            hook: signature.hook.map(str::to_owned),
            parameters: signature.parameters.to_vec(),
        });
        Some((index, false))
    }
    pub fn forget(&mut self, index: usize) {
        if let Some(slot) = self.0.get_mut(index) {
            *slot = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_colliding_program_replaces_the_previous_signature() {
        let mut programs = Programs::default();
        let mut sources = std::collections::BTreeMap::new();
        let parameters = ["first".to_owned()];
        let signature = |source| Signature {
            source,
            form: "body",
            origin: "file:///Test.qml",
            hook: None,
            parameters: &parameters,
        };
        let bodies: Vec<_> = (0..=SLOTS).map(|i| format!("return first + {i}")).collect();
        for source in &bodies {
            let (index, hit) = programs.register(signature(source)).unwrap();
            assert!(!hit);
            assert_eq!(programs.register(signature(source)), Some((index, true)));
            if let Some(previous) = sources.insert(index, source) {
                assert_eq!(programs.register(signature(previous)), Some((index, false)));
                programs.forget(index);
                assert_eq!(programs.register(signature(previous)), Some((index, false)));
                return;
            }
        }
        panic!("a fixed number of slots must collide");
    }
}
