# vexv5_serial

A Rust crate for interfacing with Vex V5 Brains. The idea of this crate is to allow vex teams to easily control, recieve input from, and upload files to the Vex V5 Brain for build pipeline automation, debugging, and custom toolchain development.

The protocol implemented in this crate is derived from [PROS-CLI](https://github.com/purduesigbots/pros-cli) as well as LemLib's [v5-serial-protocol](https://github.com/lemlib/v5-serial-protocol).