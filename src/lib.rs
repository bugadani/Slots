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
//! use slots::slots::Slots;
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
//! # use slots::slots::Slots;
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
//! # use slots::slots::Slots;
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
//! # use slots::slots::Slots;
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
//! # use slots::slots::Slots;
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
//! use slots::slots::{Slots, Size, Key};
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

pub mod iterator;
mod private;
pub mod slots;
pub mod unrestricted;

pub use generic_array::typenum::consts;
