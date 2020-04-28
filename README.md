Slots [![crates.io](https://img.shields.io/crates/v/slots.svg)](https://crates.io/crates/slots) ![build status](https://github.com/bugadani/slots/workflows/Rust/badge.svg) [![codecov](https://codecov.io/gh/bugadani/Slots/branch/master/graph/badge.svg)](https://codecov.io/gh/bugadani/Slots)
=====

Fixed size data structure with constant-time operations.

Performance options
=====
 * Slots provide the `verify_owner` feature that can be used to disable key owner verification.
   By default the feature is on and it is recommended to leave it enabled for development builds and disabled for release builds.
