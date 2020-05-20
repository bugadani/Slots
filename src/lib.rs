//! This crate provides a heapless slab allocator with strict access control.
//!
//! Slots implements a "heapless", fixed size, unordered data structure,
//! inspired by SlotMap.
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
//! use slots::Slots;
//! use slots::consts::U2;
//!
//! let mut slots: Slots<_, U2> = Slots::new(); // Capacity of 2 elements
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
//! # Remove data
//!
//! Removing data from a Slots collection invalidates its key. Because of this,
//! the [`take`] method consumes the key.
//!
//! ```rust
//! # use slots::Slots;
//! # use slots::consts::U2;
//! #
//! # let mut slots: Slots<_, U2> = Slots::new();
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
//! # use slots::Slots;
//! # use slots::consts::U1;
//! #
//! # let mut slots: Slots<_, U1> = Slots::new();
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
//! # use slots::Slots;
//! # use slots::consts::U2;
//! #
//! # let mut slots: Slots<_, U2> = Slots::new();
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
//! # use slots::Slots;
//! # use slots::consts::U2;
//! #
//! # let mut slots: Slots<_, U2> = Slots::new();
//! let k1 = slots.store(2).unwrap();
//! let idx = k1.index();
//! slots.take(k1); // idx no longer points to valid data
//!
//! assert_eq!(None, slots.try_read(idx, |&e| e*2)); // reading from a freed slot fails
//! ```
//!
//! # Passing around Slots
//!
//! When you need to work with arbitrarily sized Slots objects,
//! you need to specify that the [`Size`] trait is implemented for
//! the parameter N.
//! ```
//! use slots::{Slots, Size, Key};
//!
//! fn examine<IT, N>(slots: &Slots<IT, N>, keys: &[Key<IT, N>])
//!     where N: Size<IT>,
//! {
//!     unimplemented!();
//! }
//! ```
//!
//! [`Key`]: ./struct.Key.html
//! [`Size`]: ./trait.Size.html
//! [`index`]: ./struct.Key.html#method.index
//! [`take`]: ./struct.Slots.html#method.take
//! [`read`]: ./struct.Slots.html#method.read
//! [`modify`]: ./struct.Slots.html#method.modify

#![cfg_attr(not(test), no_std)]

mod private;

use core::marker::PhantomData;
use core::mem::replace;
use generic_array::{sequence::GenericSequence, ArrayLength, GenericArray};

use private::Entry;

pub use generic_array::typenum::consts;

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

/// Alias of [`ArrayLength`](../generic_array/trait.ArrayLength.html)
pub trait Size<I>: ArrayLength<Entry<I>> {}
impl<T, I> Size<I> for T where T: ArrayLength<Entry<I>> {}

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

pub struct UnrestrictedSlots<IT, N>
where
    N: Size<IT>,
{
    items: GenericArray<Entry<IT>, N>,
    next_free: usize,
    count: usize,
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

/// Read-only iterator to access all occupied slots.
pub struct Iter<'a, IT> {
    inner: core::slice::Iter<'a, private::Entry<IT>>,
}

/// Read-write iterator to access all occupied slots.
pub struct IterMut<'a, IT> {
    inner: core::slice::IterMut<'a, private::Entry<IT>>,
}

impl<'a, IT> Iterator for Iter<'a, IT> {
    type Item = &'a IT;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(slot) = self.inner.next() {
            if let Entry::Used(ref item) = slot {
                return Some(item);
            }
        }
        None
    }
}

impl<'a, IT> Iterator for IterMut<'a, IT> {
    type Item = &'a mut IT;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(slot) = self.inner.next() {
            if let Entry::Used(ref mut item) = slot {
                return Some(item);
            }
        }
        None
    }
}

#[cfg(test)]
mod iter_test {
    use super::{consts::U3, Slots, UnrestrictedSlots};

    #[test]
    fn sanity_check() {
        let mut slots: Slots<_, U3> = Slots::new();

        let _k1 = slots.store(1).unwrap();
        let k2 = slots.store(2).unwrap();
        let _k3 = slots.store(3).unwrap();

        slots.take(k2);

        let mut iter = slots.iter();
        // iterator does not return elements in order of store
        assert_eq!(Some(&3), iter.next());
        assert_eq!(Some(&1), iter.next());
        assert_eq!(None, iter.next());

        for &_ in slots.iter() {}
    }

    #[test]
    fn test_mut() {
        let mut slots: UnrestrictedSlots<_, U3> = UnrestrictedSlots::new();

        let _k1 = slots.store(1).unwrap();
        let k2 = slots.store(2).unwrap();
        let _k3 = slots.store(3).unwrap();

        for k in slots.iter_mut() {
            *k *= 2;
        }

        assert_eq!(Some(4), slots.take(k2));
    }
}

#[cfg(feature = "verify_owner")]
fn new_instance_id() -> usize {
    use core::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    COUNTER.fetch_add(1, Ordering::Relaxed)
}

impl<IT, N> Default for UnrestrictedSlots<IT, N>
where
    N: Size<IT>,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<IT, N> UnrestrictedSlots<IT, N>
where
    N: Size<IT>,
{
    /// Creates a new, empty Slots object.
    pub fn new() -> Self {
        Self {
            items: GenericArray::generate(|i| {
                i.checked_sub(1)
                    .map(Entry::EmptyNext)
                    .unwrap_or(Entry::EmptyLast)
            }),
            next_free: N::USIZE.saturating_sub(1), // edge case: N == 0
            count: 0,
        }
    }

    /// Returns a read-only iterator.
    /// The iterator can be used to read data from all occupied slots.
    ///
    /// **Note:** Do not rely on the order in which the elements are returned.
    pub fn iter(&self) -> Iter<IT> {
        Iter {
            inner: self.items.iter(),
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<IT> {
        IterMut {
            inner: self.items.iter_mut(),
        }
    }

    /// Returns the number of slots
    pub fn capacity(&self) -> usize {
        N::USIZE
    }

    /// Returns the number of occupied slots
    pub fn count(&self) -> usize {
        self.count
    }

    fn full(&self) -> bool {
        self.count == self.capacity()
    }

    fn free(&mut self, idx: usize) {
        debug_assert!(self.count != 0, "Free called on an empty collection");

        self.items[idx] = if self.full() {
            Entry::EmptyLast
        } else {
            Entry::EmptyNext(self.next_free)
        };

        self.next_free = idx; // the freed element will always be the top of the free stack
        self.count -= 1;
    }

    fn alloc(&mut self) -> Option<usize> {
        if self.full() {
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
    pub fn read<T>(&self, key: usize, function: impl FnOnce(&IT) -> T) -> Option<T> {
        if key >= self.capacity() {
            None
        } else {
            match &self.items[key] {
                Entry::Used(item) => Some(function(&item)),
                _ => None,
            }
        }
    }

    /// Access the element that belongs to the key for modification.
    ///
    /// This operation does not move ownership so the `function` callback must be used
    /// to access the stored element. The callback may return arbitrary derivative of the element.
    pub fn modify<T>(&mut self, key: usize, function: impl FnOnce(&mut IT) -> T) -> Option<T> {
        match self.items[key] {
            Entry::Used(ref mut item) => Some(function(item)),
            _ => None,
        }
    }
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
        self.inner.count
    }

    fn full(&self) -> bool {
        self.inner.full()
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
