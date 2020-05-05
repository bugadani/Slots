Slots [![crates.io](https://img.shields.io/crates/v/slots.svg)](https://crates.io/crates/slots) ![build status](https://github.com/bugadani/slots/workflows/Rust/badge.svg) [![codecov](https://codecov.io/gh/bugadani/Slots/branch/master/graph/badge.svg)](https://codecov.io/gh/bugadani/Slots)
=====

This crate provides a heapless slab allocator with strict access control.

Slots implements a static friendly, fixed size, unordered data structure inspired by SlotMap. All operations are constant time.

Features
========
 * Slots provide the `verify_owner` feature that can be used to disable key owner verification.
   By default the feature is on and it is recommended to leave it enabled for development builds and disabled for release builds.

   *Note: This feature requires atomic instructions, which are not generally available (for example, on ARM Cortex-M0 microcontrollers)*