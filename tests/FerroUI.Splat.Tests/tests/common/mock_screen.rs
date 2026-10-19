//! `MockScreen`.

use super::IScreen;

/// A screen of the tests.
#[derive(Debug, Default)]
pub struct MockScreen;

impl IScreen for MockScreen {}
