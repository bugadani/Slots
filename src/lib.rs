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
//! use slots::{Slots, Strict};
//! use slots::consts::U6;
//!
//! let mut slots: Slots<_, U6, Strict> = Slots::new(); // Capacity of 6 elements
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

#![cfg_attr(not(test), no_std)]

use core::marker::PhantomData;
use generic_array::{GenericArray, sequence::GenericSequence};

pub use generic_array::typenum::consts;
pub use generic_array::ArrayLength;

use generic_array::typenum::Unsigned;

mod access_mode {
    pub struct Relaxed {}
    pub struct Strict {}

    pub trait AccessMode {
        type ObjectId;

        fn new_obj_id() -> Self::ObjectId;
    }

    impl AccessMode for Relaxed {
        type ObjectId = ();

        fn new_obj_id() -> Self::ObjectId {
            ()
        }
    }
    impl AccessMode for Strict {
        #[cfg(feature = "verify_owner")]
        type ObjectId = usize;

        #[cfg(not(feature = "verify_owner"))]
        type ObjectId = ();

        #[cfg(feature = "verify_owner")]
        fn new_obj_id() -> Self::ObjectId {
            use core::sync::atomic::{AtomicUsize, Ordering};
            static COUNTER: AtomicUsize = AtomicUsize::new(0);

            COUNTER.fetch_add(1, Ordering::Relaxed)
        }

        #[cfg(not(feature = "verify_owner"))]
        fn new_obj_id() -> Self::ObjectId {
            ()
        }
    }
}

pub use access_mode::{Strict, Relaxed};

pub struct Key<IT, N>
    where N: ArrayLength<Entry<IT>> + Unsigned {
    owner_id: <Strict as access_mode::AccessMode>::ObjectId,
    index: usize,
    _item_marker: PhantomData<IT>,
    _size_marker: PhantomData<N>
}

impl<IT, N> Key<IT, N>
    where N: ArrayLength<Entry<IT>> + Unsigned {
    fn new(owner: &Slots<IT, N, Strict>, idx: usize) -> Self {
        Self {
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

pub struct Entry<IT>(EntryInner<IT>);

enum EntryInner<IT> {
    Used(IT),
    EmptyNext(usize),
    EmptyLast
}

// Data type that stores values and returns a key that can be used to manipulate
// the stored values.
// Values can be read by anyone but can only be modified using the key.
pub struct Slots<IT, N, A>
    where N: ArrayLength<Entry<IT>> + Unsigned,
          A: access_mode::AccessMode {
    id: A::ObjectId,
    items: GenericArray<Entry<IT>, N>,
    // Could be optimized by making it just usize and relying on free_count to determine its
    // validity
    next_free: Option<usize>,
    free_count: usize,
    _mode_marker: PhantomData<A>
}

impl<IT, N, A> Slots<IT, N, A>
    where N: ArrayLength<Entry<IT>> + Unsigned,
          A: access_mode::AccessMode {
    pub fn new() -> Self {
        let size = N::to_usize();

        Self {
            id: A::new_obj_id(),
            items: GenericArray::generate(|i| Entry(i.checked_sub(1).map(EntryInner::EmptyNext).unwrap_or(EntryInner::EmptyLast))),
            next_free: size.checked_sub(1),
            free_count: size,
            _mode_marker: PhantomData
        }
    }

    pub fn capacity(&self) -> usize {
        N::to_usize()
    }

    pub fn count(&self) -> usize {
        self.capacity() - self.free_count
    }

    fn free(&mut self, idx: usize) {
        self.items[idx] = match self.next_free {
            Some(n) => Entry(EntryInner::EmptyNext(n)),
            None => Entry(EntryInner::EmptyLast),
        };
        self.next_free = Some(idx);
        self.free_count += 1;
    }

    fn alloc(&mut self) -> Option<usize> {
        let index = self.next_free?;
        self.next_free = match self.items[index].0 {
            EntryInner::EmptyNext(n) => Some(n),
            EntryInner::EmptyLast => None,
            _ => unreachable!("Non-empty item in entry behind free chain"),
        };
        self.free_count -= 1;
        Some(index)
    }
}

impl<IT, N> Slots<IT, N, Strict>
    where N: ArrayLength<Entry<IT>> + Unsigned {

    fn verify_key(&self, key: &Key<IT, N>) {
        assert!(key.owner_id == self.id, "Key used in wrong instance");
    }

    pub fn store(&mut self, item: IT) -> Result<Key<IT, N>, IT> {
        match self.alloc() {
            Some(i) => {
                self.items[i] = Entry(EntryInner::Used(item));
                Ok(Key::new(self, i))
            }
            None => Err(item)
        }
    }

    pub fn take(&mut self, key: Key<IT, N>) -> IT {
        self.verify_key(&key);

        let taken = core::mem::replace(&mut self.items[key.index], Entry(EntryInner::EmptyLast));
        match taken.0 {
            EntryInner::Used(item) => {
                self.free(key.index);
                item
            },
            _ => unreachable!("Invalid key")
        }
    }

    pub fn read<T, F>(&self, key: &Key<IT, N>, function: F) -> T where F: FnOnce(&IT) -> T {
        self.verify_key(&key);

        match self.try_read(key.index, function) {
            Some(t) => t,
            None => unreachable!("Invalid key")
        }
    }

    pub fn modify<T, F>(&mut self, key: &Key<IT, N>, function: F) -> T where F: FnOnce(&mut IT) -> T {
        self.verify_key(&key);

        match self.items[key.index].0 {
            EntryInner::Used(ref mut item) => function(item),
            _ => unreachable!("Invalid key")
        }
    }

    pub fn try_read<T, F>(&self, key: usize, function: F) -> Option<T> where F: FnOnce(&IT) -> T {
        match &self.items[key].0 {
            EntryInner::Used(item) => Some(function(&item)),
            _ => None
        }
    }
}

impl<IT, N> Slots<IT, N, Relaxed>
    where N: ArrayLength<Entry<IT>> + Unsigned {

    pub fn store(&mut self, item: IT) -> Result<usize, IT> {
        match self.alloc() {
            Some(i) => {
                self.items[i] = Entry(EntryInner::Used(item));
                Ok(i)
            }
            None => Err(item)
        }
    }

    pub fn take(&mut self, key: Key<IT, N>) -> Option<IT> {
        let taken = core::mem::replace(&mut self.items[key.index], Entry(EntryInner::EmptyLast));
        match taken.0 {
            EntryInner::Used(item) => {
                self.free(key.index);
                Some(item)
            },
            _ => None
        }
    }

    pub fn modify<T, F>(&mut self, key: &Key<IT, N>, function: F) -> Option<T> where F: FnOnce(&mut IT) -> T {
        match self.items[key.index].0 {
            EntryInner::Used(ref mut item) => Some(function(item)),
            _ => None
        }
    }

    pub fn read<T, F>(&self, key: usize, function: F) -> Option<T> where F: FnOnce(&IT) -> T {
        match &self.items[key].0 {
            EntryInner::Used(item) => Some(function(&item)),
            _ => None
        }
    }
}
