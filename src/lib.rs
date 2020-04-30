//! This crate provides heapless slab allocator related collections.
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
//! assert_eq!(k3, Err(8));
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
//! assert_eq!(None, slots.try_read(idx, |&e| e*2)); // reading from a freed slot fails by returning None
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

use core::marker::PhantomData;
#[cfg(feature = "verify_owner")]
use core::sync::atomic::{AtomicUsize, Ordering};
use core::mem::replace;
use generic_array::{GenericArray, ArrayLength, sequence::GenericSequence};

mod private;
use private::Entry;

pub use generic_array::typenum::consts;

#[derive(Debug, PartialEq)]
pub struct Key<IT, N> {
    #[cfg(feature = "verify_owner")]
    owner_id: usize,
    index: usize,
    _item_marker: PhantomData<IT>,
    _size_marker: PhantomData<N>
}

pub trait Size<I>: ArrayLength<Entry<I>> {}
impl<T, I> Size<I> for T where T: ArrayLength<Entry<I>> {}

impl<IT, N> Key<IT, N> {
    fn new(owner: &Slots<IT, N>, idx: usize) -> Self where N: Size<IT> {
        Self {
            #[cfg(feature = "verify_owner")]
            owner_id: owner.id,
            index: idx,
            _item_marker: PhantomData,
            _size_marker: PhantomData
        }
    }

    pub fn index(&self) -> usize {
        self.index
    }
}

// Data type that stores values and returns a key that can be used to manipulate
// the stored values.
// Values can be read by anyone but can only be modified using the key.
pub struct Slots<IT, N>
    where N: Size<IT> {
    #[cfg(feature = "verify_owner")]
    id: usize,
    items: GenericArray<Entry<IT>, N>,
    next_free: usize,
    count: usize
}

#[cfg(feature = "verify_owner")]
fn new_instance_id() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    COUNTER.fetch_add(1, Ordering::Relaxed)
}

impl<IT, N> Slots<IT, N>
    where N: Size<IT> {
    pub fn new() -> Self {
        let size = N::to_usize();

        Self {
            #[cfg(feature = "verify_owner")]
            id: new_instance_id(),
            items: GenericArray::generate(|i| i.checked_sub(1).map(Entry::EmptyNext).unwrap_or(Entry::EmptyLast)),
            next_free: size.saturating_sub(1), // edge case: N == 0
            count: 0
        }
    }

    #[cfg(feature = "verify_owner")]
    fn verify_key(&self, key: &Key<IT, N>) {
        assert!(key.owner_id == self.id, "Key used in wrong instance");
    }

    #[cfg(not(feature = "verify_owner"))]
    fn verify_key(&self, _key: &Key<IT, N>) {
    }

    pub fn capacity(&self) -> usize {
        N::to_usize()
    }

    pub fn count(&self) -> usize {
        self.count
    }

    fn full(&self) -> bool {
        self.count == self.capacity()
    }

    fn free(&mut self, idx: usize) {
        debug_assert!(self.count != 0, "Free called on an empty collection");

        if self.full() {
            self.items[idx] = Entry::EmptyLast;
        } else {
            self.items[idx] = Entry::EmptyNext(self.next_free);
        }

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
                Entry::EmptyLast => 0, // replace last element with anything
                _ => unreachable!("Non-empty item in entry behind free chain"),
            };
            self.count += 1;
            Some(index)
        }
    }

    pub fn store(&mut self, item: IT) -> Result<Key<IT, N>, IT> {
        match self.alloc() {
            Some(i) => {
                self.items[i] = Entry::Used(item);
                Ok(Key::new(self, i))
            }
            None => Err(item)
        }
    }

    pub fn take(&mut self, key: Key<IT, N>) -> IT {
        self.verify_key(&key);

        if let Entry::Used(item) = replace(&mut self.items[key.index], Entry::EmptyLast) {
            self.free(key.index);
            item
        } else {
            unreachable!("Invalid key");
        }
    }

    pub fn read<T, F>(&self, key: &Key<IT, N>, function: F) -> T where F: FnOnce(&IT) -> T {
        self.verify_key(&key);

        match self.try_read(key.index, function) {
            Some(t) => t,
            None => unreachable!("Invalid key")
        }
    }

    pub fn try_read<T, F>(&self, key: usize, function: F) -> Option<T> where F: FnOnce(&IT) -> T {
        if key >= self.capacity() {
            None
        } else {
            match &self.items[key] {
                Entry::Used(item) => Some(function(&item)),
                _ => None
            }
        }
    }

    pub fn modify<T, F>(&mut self, key: &Key<IT, N>, function: F) -> T where F: FnOnce(&mut IT) -> T {
        self.verify_key(&key);

        match self.items[key.index] {
            Entry::Used(ref mut item) => function(item),
            _ => unreachable!("Invalid key")
        }
    }
}
