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

#![cfg_attr(not(test), no_std)]

use core::marker::PhantomData;
#[cfg(feature = "verify_owner")]
use core::sync::atomic::{AtomicUsize, Ordering};
use generic_array::{GenericArray, sequence::GenericSequence};

pub use generic_array::typenum::consts;
pub use generic_array::ArrayLength;

mod slotmap;
pub use slotmap::SlotMap;

use generic_array::typenum::Unsigned;

/// Access key to a Slots instance
///
/// Keys must only be used with the Slots they were generated from; using them for any other access
/// is a logic error and results in immediate or later panics from the Slot's functions. Erroneous
/// use is largely caught by type constraints; additional runtime constraints can be enabled using
/// the `verify_owner` feature.
///
/// Unless `verify_owner` is used, a Key is pointer-sized.
///
/// Keys can only be created by a Slot's [`store`](struct.Slots.html#method.store) method.
pub struct Key<IT, N> {
    #[cfg(feature = "verify_owner")]
    owner_id: usize,
    index: usize,
    _item_marker: PhantomData<IT>,
    _size_marker: PhantomData<N>
}

impl<IT, N> Key<IT, N>
    where N: ArrayLength<Entry<IT>> + Unsigned {
    fn new(owner: &Slots<IT, N>, idx: usize) -> Self {
        Self {
            #[cfg(feature = "verify_owner")]
            owner_id: owner.id,
            index: idx,
            _item_marker: PhantomData,
            _size_marker: PhantomData
        }
    }

    /// The underlying index of the key
    ///
    /// A key's index can be used in fallible access using the
    /// [`try_read()`](struct.Slots.html#method.try_read) method.
    pub fn index(&self) -> usize {
        self.index
    }
}

/// Internal element of `RawSlots` instances
///
/// This struct is not expected to be used by code outside the slots crate, except to define
/// suitable array sizes that are `ArrayLength<Entry<IT>>` as required by the
/// [`RawSlots`](struct.RawSlots.html) and [`Slots`](struct.Slots.html) generics
/// for usages that are generic over the length of the used slots:
///
/// ```
/// # use slots::*;
/// fn examine<IT, N>(slots: &Slots<IT, N>, keys: &[Key<IT, N>])
/// where
///     N: slots::ArrayLength<slots::Entry<IT>>,
/// {
///    unimplemented!();
/// }
/// ```
pub struct Entry<IT>(EntryInner<IT>);

enum EntryInner<IT> {
    Used(IT),
    EmptyNext(usize),
    EmptyLast
}

/// An unchecked slab allocator with predetermined size
///
/// The allocator deals out usize handles that can be used to access the data later through a
/// (shared or mutable) reference to the `RawSlots`. All access is fallible, as the handles can be
/// arbitrarily created.
///
/// It is up to slots' users to ensure that the item they intend to access is still identified by
/// that handle, especially as it can have been removed in the meantime, and the handle replaced by
/// a different object.
pub struct RawSlots<IT, N>
    where N: ArrayLength<Entry<IT>> + Unsigned {
    items: GenericArray<Entry<IT>, N>,
    // Could be optimized by making it just usize and relying on free_count to determine its
    // validity
    next_free: Option<usize>,
    free_count: usize
}

/// A type-checked slab allocator with predetermined size
///
/// The allocator deals out [`Key`](struct.Key.html) objects that can be used to access the data
/// later through a (shared or mutable) reference to the Slots. By the interface's design, access
/// is guaranteed to succeed, as the keys are unclonable and consumed to remove an item from the
/// Slots instance.
pub struct Slots<IT, N>
where N: ArrayLength<Entry<IT>> + Unsigned {
    #[cfg(feature = "verify_owner")]
    id: usize,
    raw: RawSlots<IT, N>
}

#[cfg(feature = "verify_owner")]
fn new_instance_id() -> usize {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    COUNTER.fetch_add(1, Ordering::Relaxed)
}

impl<IT, N> RawSlots<IT, N>
    where N: ArrayLength<Entry<IT>> + Unsigned {
    /// Create an empty raw slot allocator of size `N`.
    pub fn new() -> Self {
        let size = N::to_usize();

        Self {
            items: GenericArray::generate(|i| Entry(i.checked_sub(1).map(EntryInner::EmptyNext).unwrap_or(EntryInner::EmptyLast))),
            next_free: size.checked_sub(1),
            free_count: size
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

    /// Put an item into a free slot
    ///
    /// This returns an access index for the stored data in the success case, or hands the unstored
    /// item back in case of an error.
    pub fn store(&mut self, item: IT) -> Result<usize, IT> {
        match self.alloc() {
            Some(i) => {
                self.items[i] = Entry(EntryInner::Used(item));
                Ok(i)
            }
            None => Err(item)
        }
    }

    /// Move an item out of a slot, if it exists
    pub fn take(&mut self, index: usize) -> Option<IT> {
        let taken = core::mem::replace(&mut self.items[index], Entry(EntryInner::EmptyLast));
        self.free(index);
        match taken.0 {
            EntryInner::Used(item) => Some(item),
            _ => None
        }
    }

    /// Provide immutable access to an item
    ///
    /// The callback is only run if the given index is currently valid.
    pub fn read<T, F>(&self, index: usize, function: F) -> Option<T> where F: FnOnce(&IT) -> T {
        match &self.items[index].0 {
            EntryInner::Used(item) => Some(function(&item)),
            _ => None
        }
    }

    /// Provide mutable access to an item
    //
    /// The callback is only run if the given index is currently valid.
    pub fn modify<T, F>(&mut self, index: usize, function: F) -> Option<T> where F: FnOnce(&mut IT) -> T {
        match self.items[index].0 {
            EntryInner::Used(ref mut item) => Some(function(item)),
            _ => None
        }
    }
}

impl<IT, N> Slots<IT, N>
    where N: ArrayLength<Entry<IT>> + Unsigned {

    /// Create an empty slot allocator of size `N`.
    ///
    /// If the `verify_owner` feature is enabled, it will carry a new unique (except for
    /// wraparounds) ID which it shares with its keys.
    pub fn new() -> Self {
        Self {
            #[cfg(feature = "verify_owner")]
            id: new_instance_id(),
            raw: RawSlots::new(),
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
        self.raw.capacity()
    }

    pub fn count(&self) -> usize {
        self.raw.count()
    }

    /// Put an item into a free slot
    ///
    /// This returns an access index for the stored data in the success case, or hands the unstored
    /// item back in case of an error.
    pub fn store(&mut self, item: IT) -> Result<Key<IT, N>, IT> {
        self.raw.store(item).map(|id| Key::new(self, id))
    }

    /// Move an item out of its slot
    pub fn take(&mut self, key: Key<IT, N>) -> IT {
        self.verify_key(&key);
        self.raw.take(key.index()).expect("Invalid key")
    }

    /// Provide immutable access to an item
    pub fn read<T, F>(&self, key: &Key<IT, N>, function: F) -> T where F: FnOnce(&IT) -> T {
        self.verify_key(&key);
        self.raw.read(key.index(), function).expect("Invalid key")
    }

    /// Opportunistic immutable access to an item by its index
    ///
    /// A suitable index can be generated from a [`Key`](struct.Key.html) through its
    /// [`index()`](struct.Key.html#method.index) method; unlike the regular access, this can fail if
    /// the element has been removed (in which case the function is not run at all), or might have
    /// been replaced by a completely unrelated element inbetween.
    pub fn try_read<T, F>(&self, index: usize, function: F) -> Option<T> where F: FnOnce(&IT) -> T {
        self.raw.read(index, function)
    }

    /// Provide mutable access to an item
    pub fn modify<T, F>(&mut self, key: &Key<IT, N>, function: F) -> T where F: FnOnce(&mut IT) -> T {
        self.verify_key(&key);
        self.raw.modify(key.index(), function).expect("Invalid key")
    }
}
