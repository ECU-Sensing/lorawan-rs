# lorawan-rs

A `no_std` LoRaWAN stack implementation in Rust, supporting Class A, B, and C devices with a focus on US915 frequencies.

[![Crates.io](https://img.shields.io/crates/v/lorawan.svg)](https://crates.io/crates/lorawan)
[![Documentation](https://docs.rs/lorawan/badge.svg)](https://docs.rs/lorawan)
[![Build Status](https://github.com/yourusername/lorawan-rs/workflows/CI/badge.svg)](https://github.com/yourusername/lorawan-rs/actions)
[![codecov](https://codecov.io/gh/yourusername/lorawan-rs/branch/main/graph/badge.svg)](https://codecov.io/gh/yourusername/lorawan-rs)

## Features

- Full LoRaWAN 1.0.3 stack implementation
- Support for Class A, B, and C devices
- US915 frequency plan
- OTAA and ABP activation
- Default downlink commands (set interval, show firmware version, reboot)
- Extensible command handling
- `no_std` compatible for embedded systems
- Support for SX127x and SX126x radio modules

## Hardware Setup

### Supported Radio Modules

- Semtech SX1276/77/78/79 (SX127x series)
- Semtech SX1261/62 (SX126x series)

### Wiring Diagram

#### SX127x Connection

```
MCU (SPI)      SX127x
-----------------------
MOSI      -->  MOSI
MISO      <--  MISO
SCK       -->  SCK
NSS       -->  NSS
RESET     -->  RESET
DIO0      <--  DIO0
DIO1      <--  DIO1
```

#### SX126x Connection

```
MCU (SPI)      SX126x
-----------------------
MOSI      -->  MOSI
MISO      <--  MISO
SCK       -->  SCK
NSS       -->  NSS
RESET     -->  RESET
BUSY      <--  BUSY
DIO1      <--  DIO1
```

## Quick Start

1. Add the crate to your `Cargo.toml`:
```toml
[dependencies]
lorawan = "0.1"
```

2. Create a basic device (Class A example):
```rust
use lorawan::{
    config::device::DeviceConfig,
    device::LoRaWANDevice,
    class::OperatingMode,
    lorawan::region::US915,
};

// Initialize your radio (example with SX127x)
let radio = sx127x::SX127x::new(/* your SPI and GPIO pins */);

// Create device configuration
let config = DeviceConfig::new_otaa(
    [0x01; 8], // DevEUI
    [0x02; 8], // AppEUI
    [0x03; 16], // AppKey
);

// Create region configuration
let region = US915::new();

// Create LoRaWAN device
let mut device = LoRaWANDevice::new(
    radio,
    config,
    region,
    OperatingMode::ClassA,
)?;

// Join network using OTAA
device.join_otaa()?;

// Send uplink data
let data = b"Hello LoRaWAN!";
device.send_uplink(1, data, false)?;

// Process device (handle receive windows, etc.)
device.process()?;
```

## Examples

Check out the [examples](examples/) directory for complete working examples:

- [otaa_us915](examples/otaa_us915.rs): Basic OTAA activation and uplink
- [periodic_uplink](examples/periodic_uplink.rs): Periodic data transmission
- [downlink_commands](examples/downlink_commands.rs): Handling default commands

## Default Downlink Commands

The crate provides built-in support for common downlink commands:

1. **Set Interval**: Change the uplink interval
```rust
// Handling SetInterval command (60 seconds)
device.handle_downlink_cmd(DownlinkCommand::SetInterval(60))?;
```

2. **Show Firmware Version**: Request firmware version
```rust
// Handling ShowFirmwareVersion command
device.handle_downlink_cmd(DownlinkCommand::ShowFirmwareVersion)?;
```

3. **Reboot**: Trigger device reboot
```rust
// Handling Reboot command
device.handle_downlink_cmd(DownlinkCommand::Reboot)?;
```

## Custom Commands

You can extend the command handling with your own commands:

```rust
// Custom command with port 10 and payload
let data = vec![0x01, 0x02, 0x03];
device.handle_downlink_cmd(DownlinkCommand::Custom(10, data))?;
```

## Testing

Run the test suite:

```bash
cargo test
```

For hardware-in-the-loop tests (requires actual radio hardware):

```bash
cargo test --features="hardware_test" -- --ignored
```

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contributing

Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details on how to contribute to this project.

## Safety

This crate uses `#![no_std]` and is intended for use in embedded systems. It has been designed with safety in mind but has not been audited. Use at your own risk in production systems. 