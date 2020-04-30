use crate::{RawSlots, ArrayLength, Entry, Unsigned};

/// A SlotMap on a fixed size array with compact slot identifiers
///
/// A SlotMap is a pool allocator whose handles carry additional generation information.
///
/// This implementation inherits the properties of [`RawSlots`](struct.RawSlots.html) (in
/// particular, unlike other slot maps, it does not aim for contiguous allocation of items or
/// growability).
///
/// It stores the current generation as part of the slot map rather than the slot
/// (for simplicity of implementation; per-slot generations that persist data in the slot can not
/// be built on `RawSlots`), and combines index and generation into a single unsigned handler
/// created by concatenating the truncated generation counter by the width of the actuallly
/// required index.
///
/// It is a drop-in replacement for `RawSlots` with some performance penalty (storing an additional
/// usize per entry and map, and additional checks at runtime) with a largely reduced risk of an
/// old key accidentally matching a new object.
pub struct SlotMap<IT, N>
where
    N: ArrayLength<Entry<(usize, IT)>> + Unsigned
{
    raw: RawSlots<(usize, IT), N>,
    generation: usize,
}

impl<IT, N> SlotMap<IT, N>
where
    N: ArrayLength<Entry<(usize, IT)>> + Unsigned
{
    // All those pubs could be shared in an interface with RawSlots -- same API, just less likely
    // collisions

    /// Create an empty slot map of size `N`.
    pub fn new() -> Self {
        Self {
            raw: RawSlots::new(),
            generation: 0,
        }
    }

    fn pull_generation(&mut self) -> usize {
        self.generation = self.generation.wrapping_add(1);
        self.generation
    }

    /// Number of bits maximally required for an actual index
    // technically const
    fn shiftsize() -> u32 {
        core::mem::size_of::<usize>() as u32 * 8 - (N::to_usize().saturating_sub(1)).leading_zeros()
    }

    fn build_handle(&mut self, index: usize) -> usize {
        let genpart = self.pull_generation() << Self::shiftsize();
        index | genpart
    }

    fn extract_index(handle: usize) -> usize {
        // Bits that contain the index
        let mask = !(!0 << Self::shiftsize());
        handle & mask
    }

    /// Put an item into a free slot
    ///
    /// This returns an access handle for the stored data in the success case, or hands the unstored
    /// item back in case of an error.
    pub fn store(&mut self, item: IT) -> Result<usize, IT> {
        // If we only stored the trimmed generation in the checking field rather than the full
        // index, this could be a bit smoother, but the compiler should be able to optimize this
        // into a single write access. (And on the other hand, this eases access checks).
        match self.raw.store((0, item)) {
            Err((_, item)) => Err(item),
            Ok(i) => {
                let handle = self.build_handle(i);
                self.raw.modify(i, |item| item.0 = handle);
                Ok(handle)
            }
        }
    }

    /// Move an item out of its slot, if it exists
    pub fn take(&mut self, handle: usize) -> Option<IT> {
        let index = Self::extract_index(handle);
        if self.raw.read(index, |&(check, _)| check == handle) == Some(true) {
            Some(self.raw.take(index).expect("Item was checked to be present").1)
        } else {
            None
        }
    }

    /// Provide immutable access to an item
    ///
    /// The callback is only run if the given handle matches the currently stored item.
    pub fn read<T, F>(&self, handle: usize, function: F) -> Option<T> where F: FnOnce(&IT) -> T {
        self.raw.read(Self::extract_index(handle), |(check, data)|
            if check == &handle {
                Some(function(&data))
            } else {
                None
            }
        ).flatten()
    }

    /// Provide mutable access to an item
    //
    /// The callback is only run if the given handle matches the currently stored item.
    pub fn modify<T, F>(&mut self, handle: usize, function: F) -> Option<T> where F: FnOnce(&mut IT) -> T {
        self.raw.modify(Self::extract_index(handle), |(check, data)|
            if check == &handle {
                Some(function(data))
            } else {
                None
            }
        ).flatten()
    }
}
