//! This crate provides a heapless slab allocator.
//!
//! Slots implements a "heapless", fixed size, unordered data structure,
//! inspired by SlotMap.
//!
//! There are two variations of this data structure:
//!  * [`Slots`](./slots/index.html), where elements can only be modified using a `Key` that can't be copied
//!  * [`UnrestrictedSlots`](../unrestricted/index.html), where elements are free to be modified by anyone

#![cfg_attr(not(test), no_std)]

pub mod iterator;
mod private;
pub mod slots;
pub mod unrestricted;

pub use generic_array::typenum::consts;
