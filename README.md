# VEX V5 Serial Protocol

![image](https://github.com/vexide/v5-serial-protocol-rust/assets/42101043/6eea71ca-cc28-4f87-82fb-7b476a0becd3)

This project provides a Rust implementation of the serial communications protocol used by VEX V5 devices (and more!) over USB and Bluetooth.

> [!NOTE]
> Information regarding the protocol is derived from the open-source [PROS-CLI project](https://github.com/purduesigbots/pros-cli) as well as JerryLum's reverse engineering efforts in [v5-serial-protocol](https://github.com/lemlib/v5-serial-protocol).
> 
> *Use this crate at your own risk!* We aren't responsible if you blow up your own hardware due to misusing this protocol and neither is VEX. If you have a problem, please open an issue *here* rather than bothering VEX support. They have better things to do.

## Features

- Asynchronous USB and Bluetooth LE support.
- Most CDC and CDC2 (extended) command packets implemented.
- Experimental support for many different non-V5 products (EXP, AIR, AIM, AI Vision, etc...)
- Optional utility routines for higher-level tasks like program uploading.
- Separate `vex-cdc` bare protocol crate with `#![no_std]` support.
- *Mostly* executor agnostic (the `bluetooth` feature requires a tokio runtime, however).
