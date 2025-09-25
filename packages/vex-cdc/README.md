# vex-cdc

Implementation of the VEX Robotics CDC protocol in Rust.

This crate allows you to encode and decode packets used to communicate
with products sold by [VEX Robotics] using their CDC (**C**ommunications
**D**evice **C**lass) protocol. The protocol can be used to upload programs
and interact with VEX brains and other hardware over USB and bluetooth.

Currently, most packets supported by the [V5 Brain] and [V5 Controller] are
implemented, though the packets provided by this crate are non-exhaustive.

[VEX Robotics]: https://www.vexrobotics.com/
[V5 Brain]: https://www.vexrobotics.com/276-4810.html
[V5 Controller]: https://www.vexrobotics.com/276-4820.html

This crate is used as a backing implementation for vexide's [vex-v5-serial]
library and [cargo-v5].

[vex-v5-serial]: http://crates.io/crates/vex-v5-serial
[cargo-v5]: https://github.com/vexide/cargo-v5