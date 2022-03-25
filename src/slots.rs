//! Slots object that provides strict access control for the stored data.
//!
//! Data type that stores values and returns a key that can be used to manipulate
//! the stored values.
//! Values can be read by anyone but can only be modified using the key.
//!
//! # Store data
//!
//! When a piece of data is stored in the collection, a [`Key`] object is
//! returned. This key represents the owner of the stored data:
//! it is required to modify or remove (take out) the stored data.
//!
//! To ensure the stored data is always valid as long as the key exists,
//! the key can't be cloned.
//!
//! ```rust
//! use slots::slots::Slots;
//!
//! let mut slots: Slots<_, 2> = Slots::new(); // Capacity of 2 elements
//!
//! // Store elements
//! let k1 = slots.store(2).unwrap();
//! let k2 = slots.store(4).unwrap();
//!
//! assert_eq!(true, slots.is_full());
//!
//! // Now that the collection is full, the next store will fail and
//! // return an Err object that holds the original value we wanted to store.
//! let k3 = slots.store(8);
//! assert_eq!(k3.err(), Some(8));
//!
//! // Storage statistics
//! assert_eq!(2, slots.capacity()); // this instance can hold at most 2 elements
//! assert_eq!(2, slots.count()); // there are currently 2 elements stored
//! ```
//!
//! # Remove data
//!
//! Removing data from a Slots collection invalidates its key. Because of this,
//! the [`take`] method consumes the key.
//!
//! ```rust
//! # use slots::slots::Slots;
//! #
//! # let mut slots: Slots<_, 2> = Slots::new();
//! #
//! # let k = slots.store(2).unwrap();
//! # let _ = slots.store(4).unwrap();
//! #
//! // initially we have 2 elements in the collection
//! assert_eq!(2, slots.count());
//!
//! // remove an element
//! slots.take(k);
//!
//! // removing also decreases the count of stored elements
//! assert_eq!(1, slots.count());
//! ```
//!
//! ```rust{compile_fail}
//! # use slots::slots::Slots;
//! #
//! # let mut slots: Slots<_, 1> = Slots::new();
//! #
//! let k1 = slots.store(2).unwrap();
//!
//! slots.take(k1); // k1 is consumed and can no longer be used
//! slots.take(k1); // trying to use it again will cause a compile error
//! ```
//!
//! # Access stored data
//!
//! The key can be used to read or modify the stored data. This is done by passing a `FnOnce` closure
//! to the [`read`] and [`modify`] methods. Whatever the closures return, will be returned by the methods.
//!
//! ```rust
//! # use slots::slots::Slots;
//! #
//! # let mut slots: Slots<_, 2> = Slots::new();
//! #
//! # let k1 = slots.store(2).unwrap();
//! let k2 = slots.store(4).unwrap();
//! // Read element without modification
//! // closure can be used to transform element
//! assert_eq!(3, slots.read(&k2, |&e| e - 1));
//!
//! // Modify a stored element and return a derivative
//! assert_eq!(3, slots.modify(&k2, |e| {
//!     *e = 2 + *e;
//!     3
//! }));
//! // The value behind k2 has changed
//! assert_eq!(6, slots.read(&k2, |&e| e));
//! ```
//!
//! # Read using a numerical index
//!
//! It's possible to extract the index of the allocated slot from the [`Key`] object, using the [`index`] method.
//! Because this returns a number with the `usize` type, it is not guaranteed to refer to valid data.
//!
//! ```rust
//! # use slots::slots::Slots;
//! #
//! # let mut slots: Slots<_, 2> = Slots::new();
//! let k1 = slots.store(2).unwrap();
//! let idx = k1.index();
//! slots.take(k1); // idx no longer points to valid data
//!
//! assert_eq!(None, slots.try_read(idx, |&e| e*2)); // reading from a freed slot fails
//! ```
//!
//! [`Key`]: crate::slots::Key
//! [`index`]: crate::slots::Key::index
//! [`take`]: crate::slots::Slots::take
//! [`read`]: crate::slots::Slots::read
//! [`modify`]: crate::slots::Slots::modify
use core::marker::PhantomData;

use crate::iterator::Iter;
use crate::unrestricted::UnrestrictedSlots;

/// The key used to access stored elements.
///
/// **Important:** It should only be used to access the same collection that returned it.
/// When the `runtime_checks` feature is disabled, extra care must be taken to ensure this constraint.
#[derive(Debug)]
pub struct Key<IT, const N: usize> {
    #[cfg(feature = "runtime_checks")]
    owner_id: usize,
    index: usize,
    _item_marker: PhantomData<IT>,
}

impl<IT, const N: usize> Key<IT, N> {
    fn new(owner: &Slots<IT, N>, idx: usize) -> Self {
        Self {
            #[cfg(feature = "runtime_checks")]
            owner_id: owner.id,
            index: idx,
            _item_marker: PhantomData,
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

/// Slots object that provides strict access control for the stored data.
///
/// The struct has two type parameters:
///  - `IT` is the type of the stored data
///  - `N` is the number of slots.
///
/// For more information, see the [module level documentation](./index.html)
#[derive(Default)]
pub struct Slots<IT, const N: usize> {
    #[cfg(feature = "runtime_checks")]
    id: usize,
    inner: UnrestrictedSlots<IT, N>,
}

#[cfg(feature = "runtime_checks")]
fn new_instance_id() -> usize {
    use core::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    COUNTER.fetch_add(1, Ordering::Relaxed)
}

impl<IT, const N: usize> Slots<IT, N> {
    /// Creates a new, empty Slots object.
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "runtime_checks")]
            id: new_instance_id(),
            inner: UnrestrictedSlots::new(),
        }
    }

    /// Returns a read-only iterator.
    /// The iterator can be used to read data from all occupied slots.
    ///
    /// **Note:** Do not rely on the order in which the elements are returned.
    ///
    /// ```
    /// # use slots::slots::Slots;
    /// # let mut slots: Slots<_, 4> = Slots::new();
    /// slots.store(2).unwrap();
    /// slots.store(4).unwrap();
    /// slots.store(6).unwrap();
    ///
    /// assert_eq!(true, slots.iter().any(|&x| x < 3));
    /// ```
    pub fn iter(&self) -> Iter<IT> {
        self.inner.iter()
    }

    #[cfg(feature = "runtime_checks")]
    fn verify_key(&self, key: &Key<IT, N>) {
        assert_eq!(key.owner_id, self.id, "Key used in wrong instance");
    }

    #[cfg(not(feature = "runtime_checks"))]
    fn verify_key(&self, _key: &Key<IT, N>) {}

    /// Returns the number of slots
    ///
    /// ```
    /// # use slots::slots::Slots;
    /// let slots: Slots<f32, 4> = Slots::new();
    ///
    /// assert_eq!(4, slots.capacity());
    /// ```
    pub fn capacity(&self) -> usize {
        N
    }

    /// Returns the number of occupied slots
    ///
    /// ```
    /// # use slots::slots::Slots;
    /// let mut slots: Slots<_, 4> = Slots::new();
    ///
    /// assert_eq!(0, slots.count());
    ///
    /// slots.store(3).unwrap();
    /// slots.store(6).unwrap();
    ///
    /// assert_eq!(2, slots.count());
    /// ```
    pub fn count(&self) -> usize {
        self.inner.count()
    }

    /// Returns whether all the slots are occupied and the next [`store()`](#method.store) will fail.
    ///
    /// ```
    /// # use slots::slots::Slots;
    /// let mut slots: Slots<_, 4> = Slots::new();
    ///
    /// slots.store(3).unwrap();
    /// slots.store(4).unwrap();
    /// slots.store(5).unwrap();
    ///
    /// assert_eq!(false, slots.is_full());
    ///
    /// slots.store(6).unwrap();
    ///
    /// assert_eq!(true, slots.is_full());
    /// ```
    pub fn is_full(&self) -> bool {
        self.inner.is_full()
    }

    /// Store an element in a free slot and return the key to access it.
    ///
    /// Storing a variable takes ownership over it. If the storage is full,
    /// the inserted data is returned in the return value.
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
    ///
    /// ```
    /// # use slots::slots::Slots;
    /// # let mut slots: Slots<_, 4> = Slots::new();
    ///
    /// let k = slots.store(3).unwrap();
    ///
    /// assert_eq!(4, slots.read(&k, |elem| {
    ///     elem + 1
    /// }));
    /// ```
    pub fn read<T>(&self, key: &Key<IT, N>, function: impl FnOnce(&IT) -> T) -> T {
        self.verify_key(&key);

        self.inner.read(key.index, function).expect("Invalid key")
    }

    /// Read the element that belongs to a particular index. Since the index may point to
    /// a free slot or outside the collection, this operation may return None without invoking the callback.
    ///
    /// This operation does not move ownership so the `function` callback must be used
    /// to access the stored element. The callback may return arbitrary derivative of the element.
    ///
    /// This operation is fallible. If `index` addresses a free slot, `None` is returned.
    ///
    /// ```
    /// # use slots::slots::Slots;
    /// # let mut slots: Slots<_, 4> = Slots::new();
    ///
    /// let k = slots.store(3).unwrap();
    /// let idx = k.index();
    ///
    /// assert_eq!(Some(4), slots.try_read(idx, |elem| {
    ///     elem + 1
    /// }));
    ///
    /// slots.take(k);
    ///
    /// assert_eq!(None, slots.try_read(idx, |elem| {
    ///     elem + 1
    /// }));
    /// ```
    pub fn try_read<T>(&self, index: usize, function: impl FnOnce(&IT) -> T) -> Option<T> {
        self.inner.read(index, function)
    }

    /// Access the element that belongs to the key for modification.
    ///
    /// This operation does not move ownership so the `function` callback must be used
    /// to access the stored element. The callback may return arbitrary derivative of the element.
    ///
    /// ```
    /// # use slots::slots::Slots;
    /// # let mut slots: Slots<_, 4> = Slots::new();
    ///
    /// let k = slots.store(3).unwrap();
    ///
    /// assert_eq!("found", slots.modify(&k, |elem| {
    ///     *elem = *elem + 1;
    ///
    ///     "found"
    /// }));
    ///
    /// // Assert that the stored data was modified
    /// assert_eq!(4, slots.take(k));
    /// ```
    pub fn modify<T>(&mut self, key: &Key<IT, N>, function: impl FnOnce(&mut IT) -> T) -> T {
        self.verify_key(&key);

        self.inner.modify(key.index, function).expect("Invalid key")
    }
}
