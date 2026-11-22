use std::ops::Deref;
use std::sync::Arc;

/// Owned read-only UTF-16 memory, preserving unpaired surrogates. A range within
/// one rope leaf shares its allocation and remains valid after document edits.
#[derive(Clone, Debug, Default)]
pub struct Utf16Memory {
    data: Arc<[u16]>,
    start: usize,
    length: usize,
}

impl Utf16Memory {
    pub(crate) fn shared(data: Arc<[u16]>, start: usize, length: usize) -> Self {
        debug_assert!(start <= data.len() && length <= data.len() - start);
        Self {
            data,
            start,
            length,
        }
    }

    pub fn slice(&self, offset: usize, length: usize) -> Option<Self> {
        if offset > self.length || length > self.length - offset {
            None
        } else {
            Some(Self::shared(self.data.clone(), self.start + offset, length))
        }
    }

    /// Tests allocation identity, independently of the range within it.
    pub fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.data, &other.data)
    }
}

impl Deref for Utf16Memory {
    type Target = [u16];
    fn deref(&self) -> &Self::Target {
        &self.data[self.start..self.start + self.length]
    }
}

impl AsRef<[u16]> for Utf16Memory {
    fn as_ref(&self) -> &[u16] {
        self
    }
}

impl From<Vec<u16>> for Utf16Memory {
    fn from(value: Vec<u16>) -> Self {
        let length = value.len();
        Self::shared(value.into(), 0, length)
    }
}

impl PartialEq for Utf16Memory {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}
impl Eq for Utf16Memory {}
