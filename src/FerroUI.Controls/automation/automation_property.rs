/// Identifies a property of an automation peer or of one of its providers.
///
/// Identifiers are compared by identity: two identifiers are equal only if
/// they are the same identifier.
pub struct AutomationProperty {
    // Not zero sized, so that every identifier has an address of its own.
    _identity: u8,
}

impl AutomationProperty {
    pub(crate) const fn new() -> Self {
        Self { _identity: 0 }
    }
}

impl PartialEq for AutomationProperty {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for AutomationProperty {}

impl std::hash::Hash for AutomationProperty {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(self, state)
    }
}

impl std::fmt::Debug for AutomationProperty {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AutomationProperty({:p})", self)
    }
}
