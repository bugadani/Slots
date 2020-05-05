//! Data structure with only constant time operations
//!
//! Slots implements a "heapless", fixed size, unordered data structure where elements
//! can only be modified using the key retrieved when storing them.
//! Slots behaves similarly to a stack, except the key can be used to retrieve (and delete)
//! elements without restriction.
//!
//! Example usage:
//!
//! ```
//! use slots::Slots;
//! use slots::consts::U6;
//!
//! let mut slots: Slots<_, U6> = Slots::new(); // Capacity of 6 elements
//!
//! // Store elements
//! let k1 = slots.store(2).unwrap(); // returns Err(2) if full
//! let k2 = slots.store(4).unwrap();
//!
//! // Remove first element
//! let first = slots.take(k1); // k1 is consumed and can no longer be used
//! assert_eq!(2, first);
//!
//! // Read element without modification
//! let three = slots.read(&k2, |&e| e-1); // closure can be used to transform element
//! assert_eq!(3, three);
//!
//! // Try to read from an index without the key:
//! let this_will_be_none = slots.try_read(5, |&e| e); // closure *is not* called because index is not used
//! assert_eq!(None, this_will_be_none);
//!
//! // Try to read from an index extracted from the key:
//! let index = k2.index(); // this will only allow us to read since there are no guarantees the item will be valid
//! let this_will_be_five = slots.try_read(index, |&e| e+1).unwrap(); //closure *is* called
//! assert_eq!(5, this_will_be_five);
//!
//! // Modify a stored element
//! let three = slots.modify(&k2, |e| {
//!     *e = 2 + *e;
//!     3
//! });
//! assert_eq!(3, three);
//!
//! // Information about the storage
//! assert_eq!(6, slots.capacity());
//! assert_eq!(1, slots.count());
//! ```
//!
//! When you need to work with arbitrarily sized Slots objects,
//! you need to specify that the slots::Size<IT> trait is implemented for
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

#![cfg_attr(not(test), no_std)]

use core::marker::PhantomData;
#[cfg(feature = "verify_owner")]
use core::sync::atomic::{AtomicUsize, Ordering};
use core::mem::replace;
use generic_array::{GenericArray, ArrayLength, sequence::GenericSequence};

mod private;
use private::Entry;

pub use generic_array::typenum::consts;

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
        match &self.items[key] {
            Entry::Used(item) => Some(function(&item)),
            _ => None
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
