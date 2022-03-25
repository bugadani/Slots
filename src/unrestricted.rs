//! Slots object that provides an unrestricted access control for the stored data.
//!
//! Data type that stores values and returns an index that can be used to manipulate
//! the stored values.
//!
//! Unlike [`Slots`], it's not guaranteed that the accessed slot has valid data.
//! For this reason, the data access methods are always fallible, meaning they return
//! None when a free slot is addressed.
//!
//! This structure is also susceptible to the [ABA problem][aba-problem].
//!
//! # Store data
//!
//! When a piece of data is stored in the collection, a handle is returned. This handle
//! identifies the slot and can be used to access the data. Unlike with [`Slots`], this
//! handle is a `usize` which can be freely copied and shared.
//!
//! There should be no assumptions made on the value of the handle, except that it is `0 <= handle < N`
//! where N is the capacity.
//!
//! ```rust
//! use slots::unrestricted::UnrestrictedSlots;
//!
//! let mut slots: UnrestrictedSlots<_, 2> = UnrestrictedSlots::new(); // Capacity of 2 elements
//!
//! // Store elements
//! let k1 = slots.store(2).unwrap();
//! let k2 = slots.store(4).unwrap();
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
//! [`Slots`]: crate::slots
//! [aba-problem]: https://en.wikipedia.org/wiki/ABA_problem

use core::mem::replace;

use crate::iterator::*;
use crate::private::Entry;

/// Slots object that provides an unrestricted access control for the stored data.
///
/// The struct has two type parameters:
///  - `IT` is the type of the stored data
///  - `N` is the number of slots.
///
/// For more information, see the [module level documentation](crate::unrestricted)
pub struct UnrestrictedSlots<IT, const N: usize> {
    items: [Entry<IT>; N],
    next_free: usize,
    count: usize,
}

impl<IT, const N: usize> Default for UnrestrictedSlots<IT, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<IT, const N: usize> UnrestrictedSlots<IT, N> {
    /// Creates a new, empty UnrestrictedSlots object.
    pub fn new() -> Self {
        Self {
            items: array_init::array_init(|i| {
                i.checked_sub(1)
                    .map(Entry::EmptyNext)
                    .unwrap_or(Entry::EmptyLast)
            }),
            next_free: N.saturating_sub(1), // edge case: N == 0
            count: 0,
        }
    }

    /// Returns a read-only iterator.
    /// The iterator can be used to read data from all occupied slots.
    ///
    /// **Note:** Do not rely on the order in which the elements are returned.
    ///
    /// ```
    /// # use slots::unrestricted::UnrestrictedSlots;
    /// # let mut slots: UnrestrictedSlots<_, 4> = UnrestrictedSlots::new();
    /// slots.store(2).unwrap();
    /// slots.store(4).unwrap();
    /// slots.store(6).unwrap();
    ///
    /// assert_eq!(true, slots.iter().any(|&x| x < 3));
    /// ```
    pub fn iter(&self) -> Iter<IT> {
        Iter::from_entry_slice(self.items.as_slice())
    }

    /// Returns a read-write iterator.
    /// The iterator can be used to read and modify data from all occupied slots, but it can't remove data.
    ///
    /// **Note:** Do not rely on the order in which the elements are returned.
    ///
    /// ```
    /// # use slots::unrestricted::UnrestrictedSlots;
    /// # let mut slots: UnrestrictedSlots<_, 4> = UnrestrictedSlots::new();
    /// let k = slots.store(2).unwrap();
    /// slots.store(4).unwrap();
    /// slots.store(6).unwrap();
    ///
    /// for mut x in slots.iter_mut() {
    ///     *x *= 2;
    /// }
    ///
    /// assert_eq!(4, slots.take(k).unwrap());
    /// ```
    pub fn iter_mut(&mut self) -> IterMut<IT> {
        IterMut::from_entry_slice(self.items.as_mut_slice())
    }

    /// Returns the number of slots
    ///
    /// ```
    /// # use slots::unrestricted::UnrestrictedSlots;
    /// let slots: UnrestrictedSlots<f32, 4> = UnrestrictedSlots::new();
    ///
    /// assert_eq!(4, slots.capacity());
    /// ```
    pub fn capacity(&self) -> usize {
        N
    }

    /// Returns the number of occupied slots
    ///
    /// ```
    /// # use slots::unrestricted::UnrestrictedSlots;
    /// let mut slots: UnrestrictedSlots<_, 4> = UnrestrictedSlots::new();
    ///
    /// assert_eq!(0, slots.count());
    ///
    /// slots.store(3).unwrap();
    /// slots.store(6).unwrap();
    ///
    /// assert_eq!(2, slots.count());
    /// ```
    pub fn count(&self) -> usize {
        self.count
    }

    /// Returns whether all the slots are occupied and the next [`store()`](UnrestrictedSlots::store) will fail.
    ///
    /// ```
    /// # use slots::unrestricted::UnrestrictedSlots;
    /// let mut slots: UnrestrictedSlots<_, 4> = UnrestrictedSlots::new();
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
        self.count == self.capacity()
    }

    fn free(&mut self, idx: usize) {
        debug_assert!(self.count != 0, "Free called on an empty collection");

        self.items[idx] = if self.is_full() {
            Entry::EmptyLast
        } else {
            Entry::EmptyNext(self.next_free)
        };

        self.next_free = idx; // the freed element will always be the top of the free stack
        self.count -= 1;
    }

    fn alloc(&mut self) -> Option<usize> {
        if self.is_full() {
            // no free slot
            None
        } else {
            // next_free points to the top of the free stack
            let index = self.next_free;

            self.next_free = match self.items[index] {
                Entry::EmptyNext(n) => n, // pop the stack
                Entry::EmptyLast => 0,    // replace last element with anything
                _ => unreachable!("Non-empty item in entry behind free chain"),
            };
            self.count += 1;
            Some(index)
        }
    }

    /// Store an element in a free slot and return the key to access it.
    ///
    /// Storing a variable takes ownership over it. If the storage is full,
    /// the inserted data is returned in the return value.
    pub fn store(&mut self, item: IT) -> Result<usize, IT> {
        match self.alloc() {
            Some(i) => {
                self.items[i] = Entry::Used(item);
                Ok(i)
            }
            None => Err(item),
        }
    }

    /// Remove and return the element that belongs to the key.
    ///
    /// This operation is fallible. If `key` addresses a free slot, `None` is returned.
    pub fn take(&mut self, key: usize) -> Option<IT> {
        if let Entry::Used(item) = replace(&mut self.items[key], Entry::EmptyLast) {
            self.free(key);
            Some(item)
        } else {
            None
        }
    }

    /// Read the element that belongs to a particular index. Since the index may point to
    /// a free slot or outside the collection, this operation may return None without invoking the callback.
    ///
    /// This operation does not move ownership so the `function` callback must be used
    /// to access the stored element. The callback may return arbitrary derivative of the element.
    ///
    /// This operation is fallible. If `key` addresses a free slot, `None` is returned.
    ///
    /// ```
    /// # use slots::unrestricted::UnrestrictedSlots;
    /// # let mut slots: UnrestrictedSlots<_, 4> = UnrestrictedSlots::new();
    ///
    /// let k = slots.store(3).unwrap();
    ///
    /// assert_eq!(Some(4), slots.read(k, |elem| {
    ///     elem + 1
    /// }));
    ///
    /// slots.take(k);
    ///
    /// assert_eq!(None, slots.read(k, |elem| {
    ///     elem + 1
    /// }));
    /// ```
    pub fn read<T>(&self, key: usize, function: impl FnOnce(&IT) -> T) -> Option<T> {
        if key >= self.capacity() {
            None
        } else {
            match self.items[key] {
                Entry::Used(ref item) => Some(function(item)),
                _ => None,
            }
        }
    }

    /// Access the element that belongs to the key for modification.
    ///
    /// This operation does not move ownership so the `function` callback must be used
    /// to access the stored element. The callback may return arbitrary derivative of the element.
    ///
    /// This operation is fallible. If `key` addresses a free slot, `None` is returned.
    ///
    /// ```
    /// # use slots::unrestricted::UnrestrictedSlots;
    /// # let mut slots: UnrestrictedSlots<_, 4> = UnrestrictedSlots::new();
    ///
    /// let k = slots.store(3).unwrap();
    ///
    /// assert_eq!(Some("found"), slots.modify(k, |elem| {
    ///     *elem = *elem + 1;
    ///
    ///     "found"
    /// }));
    ///
    /// // Assert that the stored data was modified
    /// assert_eq!(Some(4), slots.take(k));
    ///
    /// assert_eq!(None, slots.modify(k, |elem| {
    ///     *elem = *elem + 1;
    ///
    ///     "found"
    /// }));
    /// ```
    pub fn modify<T>(&mut self, key: usize, function: impl FnOnce(&mut IT) -> T) -> Option<T> {
        match self.items[key] {
            Entry::Used(ref mut item) => Some(function(item)),
            _ => None,
        }
    }
}
