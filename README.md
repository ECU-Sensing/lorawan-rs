# lorawan-rs 📡

[![Crates.io](https://img.shields.io/crates/v/lorawan-rs)](https://crates.io/crates/lorawan-rs)
[![Documentation](https://img.shields.io/badge/docs-github.io-blue)](https://ecu-sensing.github.io/lorawan-rs)
[![Build Status](https://github.com/ECU-Sensing/lorawan-rs/workflows/CI/badge.svg)](https://github.com/ECU-Sensing/lorawan-rs/actions)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](README.md)
[![no_std](https://img.shields.io/badge/no__std-yes-blue)](README.md)

A pure Rust, `no_std` implementation of the LoRaWAN protocol stack for embedded devices. This crate provides a complete LoRaWAN solution with support for Class A, B, and C devices, OTAA/ABP activation, and common radio drivers.

## Features 🚀

- **Full LoRaWAN Stack Implementation**
  - LoRaWAN 1.0.3 compliant
  - Class A, B, and C device support
  - OTAA and ABP activation methods
  - Proper frequency hopping support
  - US915 region implementation (other regions coming soon)

- **Radio Hardware Support**
  - SX127x (SX1276/77/78/79) driver
  - SX126x driver
  - Extensible radio trait system

- **Security**
  - AES-128 encryption
  - MIC verification
  - Secure key management

- **Embedded-First Design**
  - `no_std` compatible
  - Zero-allocation where possible
  - Efficient memory usage
  - Interrupt-safe operations

## Quick Start 🏃‍♂️

Add to your `Cargo.toml`:
```toml
[dependencies]
lorawan-rs = "0.1.0"
```

Basic OTAA example:
```rust
use lorawan::{
    config::device::DeviceConfig,
    device::LoRaWANDevice,
    class::OperatingMode,
    lorawan::region::US915,
};

// Create device configuration
let config = DeviceConfig::new_otaa(
    DEVEUI,  // LSB format
    APPEUI,  // LSB format
    APPKEY,  // MSB format
);

// Initialize device
let mut device = LoRaWANDevice::new(
    radio,
    config,
    US915::new(),
    OperatingMode::ClassA,
)?;

// Join network
device.join_otaa()?;

// Send data
device.send_uplink(1, b"Hello, LoRaWAN!", false)?;
```

## Examples 📚

The crate includes several example applications:

1. **Hello World** (`examples/hello_world.rs`)
   - Basic OTAA join and uplink example
   - Designed for Adafruit Feather M0 with RFM95
   - LED status indicators for debugging

2. **OTAA US915** (`examples/otaa_us915.rs`)
   - Complete US915 OTAA implementation
   - Proper frequency hopping
   - Downlink handling

3. **Periodic Uplink** (`examples/periodic_uplink.rs`)
   - Regular data transmission example
   - Battery-efficient timing
   - Error handling patterns

4. **Downlink Commands** (`examples/downlink_commands.rs`)
   - MAC command handling
   - Downlink message processing
   - Device configuration updates

5. **LoRaWAN Repeater** (`examples/repeater.rs`)
   - Simple packet repeater
   - Lazy frequency hopping
   - LED status feedback

6. **Repeater with Metrics** (`examples/repeater_with_metrics.rs`)
   - Advanced repeater implementation
   - Metrics reporting via LoRaWAN
   - Remote management capabilities

## Hardware Support 🛠️

Currently tested on:
- Adafruit Feather M0 with RFM95
- STM32F4 with SX1276
- Additional platforms coming soon!

## Getting Started with TTN 🌐

To use this library with The Things Network:

1. Create an application in TTN Console
2. Register your device with:
   - LoRaWAN version: 1.0.3
   - Regional Parameters: RP001 1.0.3 revision A
   - Frequency plan: US_915_928 (or your region)

3. Configure your device:
   ```rust
   // Note: TTN shows keys in MSB, but DevEUI/AppEUI need LSB!
   let config = DeviceConfig::new_otaa(
       // If DevEUI in TTN is "0123456789ABCDEF":
       [0xEF, 0xCD, 0xAB, 0x89, 0x67, 0x45, 0x23, 0x01], // LSB!
       
       // If AppEUI in TTN is "FEDCBA9876543210":
       [0x10, 0x32, 0x54, 0x76, 0x98, 0xBA, 0xDC, 0xFE], // LSB!
       
       // AppKey stays in MSB as shown in TTN
       [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF,
        0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF],
   );
   ```

## Contributing 🤝

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

Make sure to:
1. Update documentation
2. Add tests if applicable
3. Follow the existing code style
4. Update the changelog

## License 📄

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments 🙏

- The Things Network for their LoRaWAN implementation
- Semtech for their reference implementations
- The Rust Embedded community

## Safety ⚠️

This crate uses `no_std` and is intended for embedded systems. While we strive for correctness, use in safety-critical systems should be carefully evaluated.

The implementation:
- Avoids unsafe code where possible
- Uses atomic operations for concurrency
- Implements proper error handling
- Follows LoRaWAN Alliance specifications

## Status 📊

- [x] Basic LoRaWAN stack
- [x] OTAA implementation
- [x] US915 region support
- [x] Class A support
- [x] Class B support
- [x] Class C support
- [x] SX127x driver
- [x] SX126x driver
- [ ] EU868 region support (coming soon)

## Roadmap 🗺️

### Phase 1: Regional Support (Q1 2024)
- [ ] EU868 implementation
- [ ] AS923 implementation
- [ ] AU915 implementation
- [ ] CN470 implementation
- [ ] Dynamic region switching

### Phase 2: Advanced Features (Q2 2024)
- [ ] Adaptive Data Rate (ADR)
- [ ] Channel Hopping Optimization
- [ ] Forward Error Correction
- [ ] Duty Cycle Management
- [ ] TX Power Optimization

### Phase 3: Hardware Support (Q3 2024)
- [ ] STM32WL55 Support
- [ ] ESP32 LoRa Support
- [ ] nRF52840 + SX126x Support
- [ ] Generic HAL Implementation
- [ ] Power Consumption Optimization

### Phase 4: Security Enhancements (Q4 2024)
- [ ] LoRaWAN 1.1 Support
- [ ] Key Derivation Functions
- [ ] Secure Element Integration
- [ ] FIPS Compliance Mode
- [ ] Security Audit

### Phase 5: Network Integration (Q1 2025)
- [ ] The Things Network V3 Integration
- [ ] ChirpStack Integration
- [ ] AWS IoT Core Integration
- [ ] Azure IoT Integration
- [ ] Custom Network Server Support

### Phase 6: Application Layer (Q2 2025)
- [ ] Application Callbacks Framework
- [ ] Payload Encoding/Decoding
- [ ] Sensor Data Aggregation
- [ ] OTA Update Support
- [ ] Device Management API

### Phase 7: Testing & Documentation (Ongoing)
- [ ] Automated Integration Tests
- [ ] Hardware-in-the-Loop Testing
- [ ] Performance Benchmarks
- [ ] Certification Support
- [ ] Example Applications

## Device Classes 📡

The crate supports all LoRaWAN device classes:

- **Class A**: ✅ Production Ready
  - Two receive windows after each uplink
  - Complete OTAA and ABP support
  - Confirmed/unconfirmed messages
  
- **Class B**: ✅ Production Ready
  - Beacon synchronization
  - Ping slot management
  - Network-initiated downlink
  - Power-efficient scheduling
  
- **Class C**: ✅ Production Ready
  - Continuous reception
  - Immediate downlink capability
  - Power management
  - Efficient RX window handling

```rust,no_run
use lorawan::class::OperatingMode;

// All device classes are now fully supported
let mut device = LoRaWANDevice::new(
    radio,
    config,
    region,
    OperatingMode::ClassA, // or ClassB, or ClassC
)?;
```

## Need Help? 💡

- Check out the [examples](examples/) directory
- Read the [documentation](https://docs.rs/lorawan-rs)
- Open an [issue](https://github.com/user/lorawan-rs/issues)
- Join our [Discord](https://discord.gg/your-invite-here)
