//! This crate provides a heapless, fixed size, unordered data structure, inspired by SlotMap.
//!
//! The following basic operations (all of them `O(1)`) are defined for Slots:
//! - Insert: store data and retrieve a handle for later access
//! - Read, modify: use the given handle to access the data without removal
//! - Take: use the given handle to remove data
//!
//! There are two variations of this data structure:
//!  * [`Slots`](./slots/index.html), where elements can only be modified using a `Key` that can't be copied
//!  * [`UnrestrictedSlots`](./unrestricted/index.html), where elements are free to be modified by anyone

#![cfg_attr(not(test), no_std)]

pub mod iterator;
mod private;
pub mod slots;
pub mod unrestricted;

pub use generic_array::typenum::consts;
