use std::cell::RefCell;

/// A block of memory with a stable address that is released explicitly.
///
/// The address stays the same from construction until [`dispose`](Self::dispose),
/// so platform code may hand it to a native or script-side consumer.
pub struct UnmanagedBlob {
    memory: RefCell<Option<Box<[u8]>>>,
}

impl UnmanagedBlob {
    /// Allocates `size` zeroed bytes.
    ///
    /// # Panics
    /// Panics when `size` is not positive.
    pub fn new(size: i32) -> Self {
        if size <= 0 {
            panic!("Positive number required (Parameter 'size')");
        }
        Self { memory: RefCell::new(Some(vec![0u8; size as usize].into_boxed_slice())) }
    }

    /// The address of the first byte.
    ///
    /// # Panics
    /// Panics when the blob has been disposed.
    pub fn address(&self) -> *mut u8 {
        match self.memory.borrow_mut().as_mut() {
            Some(memory) => memory.as_mut_ptr(),
            None => panic!("Cannot access a disposed object: UnmanagedBlob"),
        }
    }

    /// The number of bytes, zero once disposed.
    #[allow(dead_code)]
    pub fn size(&self) -> i32 {
        self.memory.borrow().as_ref().map_or(0, |memory| memory.len() as i32)
    }

    /// Whether the memory has been released.
    #[allow(dead_code)]
    pub fn is_disposed(&self) -> bool {
        self.memory.borrow().is_none()
    }

    /// Gives `access` the bytes of the blob. Returns `false` without calling
    /// it when the blob has been disposed.
    ///
    /// The blob must not be accessed or disposed from within `access`.
    pub fn with_data(&self, access: &mut dyn FnMut(&mut [u8])) -> bool {
        match self.memory.borrow_mut().as_mut() {
            Some(memory) => {
                access(memory);
                true
            }
            None => false,
        }
    }

    /// Releases the memory. Further calls do nothing.
    pub fn dispose(&self) {
        self.memory.borrow_mut().take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocates_the_requested_size_with_a_stable_address() {
        let blob = UnmanagedBlob::new(64);

        assert_eq!(64, blob.size());
        assert!(!blob.is_disposed());
        assert!(!blob.address().is_null());
        assert_eq!(blob.address(), blob.address());
    }

    #[test]
    fn data_written_through_the_slice_is_kept() {
        let blob = UnmanagedBlob::new(4);

        assert!(blob.with_data(&mut |data| data.copy_from_slice(&[1, 2, 3, 4])));

        let mut read = Vec::new();
        blob.with_data(&mut |data| read = data.to_vec());
        assert_eq!(vec![1, 2, 3, 4], read);
    }

    #[test]
    fn dispose_releases_the_memory_once() {
        let blob = UnmanagedBlob::new(8);

        blob.dispose();
        blob.dispose();

        assert!(blob.is_disposed());
        assert_eq!(0, blob.size());
        assert!(!blob.with_data(&mut |_| panic!("no data after dispose")));
    }

    #[test]
    #[should_panic(expected = "Positive number required")]
    fn a_size_of_zero_is_rejected() {
        UnmanagedBlob::new(0);
    }

    #[test]
    #[should_panic(expected = "Cannot access a disposed object")]
    fn the_address_of_a_disposed_blob_is_not_available() {
        let blob = UnmanagedBlob::new(8);
        blob.dispose();
        blob.address();
    }
}
