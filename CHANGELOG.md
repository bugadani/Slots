0.2.0
==========
* `try_read` now return `None` for out of bounds reads. [@bugadani](https://github.com/bugadani)
* Reduce memory footprint by 4 or 8 bytes (on 32 or 64 bit architectures) [@bugadani](https://github.com/bugadani)
* Clean up the required trait bounds needed to pass around generic Slots instances [@bugadani](https://github.com/bugadani)
* Verify that keys are used to access data in their associated Slots instances [@bugadani](https://github.com/bugadani)
* Allow compiler to reduce memory footprint if possible [@chrysn](https://github.com/chrysn)
* Allow using FnOnce closures in `try_read`, `read` and `modify` [@chrysn](https://github.com/chrysn)

0.1.1
=====
* Bugfix

0.1.0
=====
* Initial release
