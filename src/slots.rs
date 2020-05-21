use core::marker::PhantomData;

use crate::iterator::Iter;
use crate::unrestricted::UnrestrictedSlots;

pub use crate::unrestricted::Size;

/// The key used to access stored elements.
///
/// **Important:** It should only be used to access the same collection that returned it.
/// When the `verify_owner` feature is disabled, extra care must be taken to ensure this constraint.
#[derive(Debug)]
pub struct Key<IT, N> {
    #[cfg(feature = "verify_owner")]
    owner_id: usize,
    index: usize,
    _item_marker: PhantomData<IT>,
    _size_marker: PhantomData<N>,
}

impl<IT, N> Key<IT, N> {
    fn new(owner: &Slots<IT, N>, idx: usize) -> Self
    where
        N: Size<IT>,
    {
        Self {
            #[cfg(feature = "verify_owner")]
            owner_id: owner.id,
            index: idx,
            _item_marker: PhantomData,
            _size_marker: PhantomData,
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

/// Data type that stores values and returns a key that can be used to manipulate
/// the stored values.
/// Values can be read by anyone but can only be modified using the key.
#[derive(Default)]
pub struct Slots<IT, N>
where
    N: Size<IT>,
{
    #[cfg(feature = "verify_owner")]
    id: usize,
    inner: UnrestrictedSlots<IT, N>,
}

#[cfg(feature = "verify_owner")]
fn new_instance_id() -> usize {
    use core::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    COUNTER.fetch_add(1, Ordering::Relaxed)
}

impl<IT, N> Slots<IT, N>
where
    N: Size<IT>,
{
    /// Creates a new, empty Slots object.
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "verify_owner")]
            id: new_instance_id(),
            inner: UnrestrictedSlots::new(),
        }
    }

    /// Returns a read-only iterator.
    /// The iterator can be used to read data from all occupied slots.
    ///
    /// **Note:** Do not rely on the order in which the elements are returned.
    pub fn iter(&self) -> Iter<IT> {
        self.inner.iter()
    }

    #[cfg(feature = "verify_owner")]
    fn verify_key(&self, key: &Key<IT, N>) {
        assert_eq!(key.owner_id, self.id, "Key used in wrong instance");
    }

    #[cfg(not(feature = "verify_owner"))]
    fn verify_key(&self, _key: &Key<IT, N>) {}

    /// Returns the number of slots
    pub fn capacity(&self) -> usize {
        N::to_usize()
    }

    /// Returns the number of occupied slots
    pub fn count(&self) -> usize {
        self.inner.count()
    }

    /// Store an element in a free slot and return the key to access it.
    pub fn store(&mut self, item: IT) -> Result<Key<IT, N>, IT> {
        self.inner.store(item).map(|idx| Key::new(self, idx))
    }

    /// Remove and return the element that belongs to the key.
    pub fn take(&mut self, key: Key<IT, N>) -> IT {
        self.verify_key(&key);

        self.inner.take(key.index).expect("Invalid key")
    }

    /// Read the element that belongs to the key.
    ///
    /// This operation does not move ownership so the `function` callback must be used
    /// to access the stored element. The callback may return arbitrary derivative of the element.
    pub fn read<T>(&self, key: &Key<IT, N>, function: impl FnOnce(&IT) -> T) -> T {
        self.verify_key(&key);

        self.inner.read(key.index, function).expect("Invalid key")
    }

    /// Read the element that belongs to a particular index. Since the index may point to
    /// a free slot or outside the collection, this operation may return None without invoking the callback.
    ///
    /// This operation does not move ownership so the `function` callback must be used
    /// to access the stored element. The callback may return arbitrary derivative of the element.
    pub fn try_read<T>(&self, key: usize, function: impl FnOnce(&IT) -> T) -> Option<T> {
        self.inner.read(key, function)
    }

    /// Access the element that belongs to the key for modification.
    ///
    /// This operation does not move ownership so the `function` callback must be used
    /// to access the stored element. The callback may return arbitrary derivative of the element.
    pub fn modify<T>(&mut self, key: &Key<IT, N>, function: impl FnOnce(&mut IT) -> T) -> T {
        self.verify_key(&key);

        self.inner.modify(key.index, function).expect("Invalid key")
    }
}
