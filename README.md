# V5 Serial Protocol

![Serial Experiments VEX](https://github.com/vexide/v5-serial-protocol-rust/assets/42101043/5ecca72d-9307-40ae-a0b5-1d1c9cf74000)

This project provides a partial Rust implementation of the serial communications protocol used by VEX V5 devices over USB and Bluetooth.

> [!NOTE]  
> This information is derived from [PROS-CLI](https://github.com/purduesigbots/pros-cli) as well as JerryLum's reverse engineering efforts in [v5-serial-protocol](https://github.com/lemlib/v5-serial-protocol).

This project is a fork of some prior work done by the [vexv5-serial](https://github.com/vexrs/vexv5_serial) project, but virtually everything except for device port finding has been rewritten from the ground up.
