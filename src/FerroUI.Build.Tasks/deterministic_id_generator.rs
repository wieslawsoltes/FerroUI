//! Port of `DeterministicIdGenerator.cs`.

use std::cell::Cell;
use xamlx::transform::IXamlIdentifierGenerator;

/// Numbers the identifiers the compiler asks for, from 1: the same documents
/// compiled in the same order get the same identifiers, so the output of a
/// build does not change between runs.
pub struct DeterministicIdGenerator {
    next_id: Cell<i32>,
}

impl DeterministicIdGenerator {
    pub fn new() -> Self {
        Self { next_id: Cell::new(1) }
    }
}

impl Default for DeterministicIdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl IXamlIdentifierGenerator for DeterministicIdGenerator {
    fn generate_identifier_part(&self) -> String {
        let id = self.next_id.get();
        self.next_id.set(id.wrapping_add(1));
        id.to_string()
    }
}

#[cfg(test)]
mod tests {
    // Not from upstream, which has no tests of the class.
    use super::*;

    #[test]
    fn the_identifiers_count_from_one() {
        let generator = DeterministicIdGenerator::new();

        assert_eq!("1", generator.generate_identifier_part());
        assert_eq!("2", generator.generate_identifier_part());
        assert_eq!("3", generator.generate_identifier_part());
    }

    #[test]
    fn every_generator_starts_again() {
        let first = DeterministicIdGenerator::new();
        first.generate_identifier_part();
        first.generate_identifier_part();

        let second = DeterministicIdGenerator::default();

        assert_eq!("1", second.generate_identifier_part());
        assert_eq!("3", first.generate_identifier_part());
    }
}
