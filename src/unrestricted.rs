//! Slots object that provides an unrestricted access control for the stored data.
//!
//! Data type that stores values and returns an index that can be used to manipulate
//! the stored values.
//!
//! As opposed to [`Slots`](../slots/index.html), it's not guaranteed that the accessed slot has valid data.
//! For this reason, the data access methods are always fallible, meaning they return
//! None when a free slot is addressed.
//!
//! This structure is also susceptible to the [ABA problem](https://en.wikipedia.org/wiki/ABA_problem).

use core::mem::replace;
use generic_array::{sequence::GenericSequence, ArrayLength, GenericArray};

use crate::iterator::*;
use crate::private::Entry;

/// Alias of [`ArrayLength`](../generic_array/trait.ArrayLength.html)
pub trait Size<I>: ArrayLength<Entry<I>> {}
impl<T, I> Size<I> for T where T: ArrayLength<Entry<I>> {}

/// Slots object that provides an unrestricted access control for the stored data.
///
/// The struct has two type parameters:
///  - `IT` is the type of the stored data
///  - `N` is the number of slots, which is a type-level constant provided by the `typenum` crate.
///
/// For more information, see the [module level documentation](./index.html)
pub struct UnrestrictedSlots<IT, N>
where
    N: Size<IT>,
{
    items: GenericArray<Entry<IT>, N>,
    next_free: usize,
    count: usize,
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
    /// Creates a new, empty UnrestrictedSlots object.
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
        Iter::from_iter(self.items.as_slice())
    }

    /// Returns a read-write iterator.
    /// The iterator can be used to read and modify data from all occupied slots, but it can't remove data.
    ///
    /// **Note:** Do not rely on the order in which the elements are returned.
    pub fn iter_mut(&mut self) -> IterMut<IT> {
        IterMut::from_iter(self.items.as_mut_slice())
    }

    /// Returns the number of slots
    pub fn capacity(&self) -> usize {
        N::USIZE
    }

    /// Returns the number of occupied slots
    pub fn count(&self) -> usize {
        self.count
    }

    /// Returns whether all the slots are occupied and the next store() will fail.
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
    /// the ownership is returned in the return value.
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
