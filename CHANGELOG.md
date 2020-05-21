0.3..0
==========
* Rename `verify_owner` feature flag to `runtime_checks` [@bugadani]
* Add `UnrestrictedSlots` with relaxed access control [@chrysn] [@bugadani]
* Implement read-only `.iter()` [@bugadani]
* Implement `Default` for `Slots` [@bugadani]

0.2.0
==========
* `try_read` now return `None` for out of bounds reads. [@bugadani]
* Reduce memory footprint by 4 or 8 bytes (on 32 or 64 bit architectures) [@bugadani]
* Clean up the required trait bounds needed to pass around generic Slots instances [@bugadani]
* Verify that keys are used to access data in their associated Slots instances [@bugadani]
* Allow compiler to reduce memory footprint if possible [@chrysn]
* Allow using FnOnce closures in `try_read`, `read` and `modify` [@chrysn]

0.1.1
=====
* Bugfix

0.1.0
=====
* Initial release

[@bugadani]: https://github.com/bugadani
[@chrysn]: https://github.com/chrysn
