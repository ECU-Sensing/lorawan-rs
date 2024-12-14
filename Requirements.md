LoRaWAN in Rust

1. **Support for Class A, Class B, and Class C LoRaWAN Devices.**  
2. **Focus on US Frequencies (US915).**  
3. **Default Downlink Commands** (e.g., set interval, show firmware version, reboot) and an extensible base class for adding custom commands.

---

# 1. Requirements Document

## 1.1 Functional Requirements

1. **LoRaWAN Stack Support**  
   - Implement the LoRaWAN protocol stack with support for Class A, Class B, and Class C operations.  
     - **Class A**: End-device uplinks followed by two short receive windows.  
     - **Class B**: Uses scheduled receive windows (beaconing) in addition to Class A windows.  
     - **Class C**: Continuous receive window (largely for low-latency downlinks).
   - Provide device activation flows (OTAA/ABP).
   - Handle uplink and downlink messages, confirmed/unconfirmed frames, frame counters, etc.

2. **US Frequency Support**  
   - Focus on **US915** region by default (e.g., channel plans, data rates).  
   - Provide relevant channel configuration for US sub-bands.  
   - Ensure region-specific compliance (duty cycles, TX power limits, etc. for US915).

3. **Hardware Abstraction**  
   - Use `embedded-hal` traits for SPI, GPIO, timers, etc. to support multiple radio modules (e.g., Semtech SX127x, SX126x).  
   - Ensure memory efficiency suitable for resource-constrained MCU platforms (Cortex-M, RISC-V, etc.).

4. **Security**  
   - Implement AES-128 encryption/decryption using LoRaWAN keys.  
   - Manage session keys for network (NwkSKey) and application (AppSKey).  
   - Compute and validate message integrity code (MIC).

5. **Device Activation**  
   - **OTAA**: Over-the-Air Activation (join procedure, dev_nonce, join_accept).  
   - **ABP**: Activation By Personalization (hard-coded keys and device address).

6. **Configuration & Control**  
   - Provide an intuitive API for setting or updating DevEUI, AppEUI, AppKey, data rates, etc.  
   - Easily select or switch the radio driver via a trait-based abstraction.  
   - Region-specific configuration (initially US915) with the possibility to add more regions in the future.

7. **Default Downlink Command Set**  
   - Out of the box support for handling downlink commands:
     1. **Set Interval**: Adjust the uplink interval.  
     2. **Show Firmware Version**: Respond with the device’s firmware version.  
     3. **Reboot**: Reboot the device.  
   - Provide clear documentation on how these commands work, and instructions on how to extend the base class for custom commands.

8. **LoRaWAN Classes**  
   - Implement scheduling logic for Class A, Class B, and Class C.  
   - For Class B, handle beacons and ping slots.  
   - For Class C, keep the receiver open except during transmit.

9. **Event Handling**  
   - Expose callbacks or async/await interfaces for handling join success/failure, uplink done, downlink received, etc.  
   - Provide easy hooks to process default and custom downlink commands.

10. **no_std Compatibility**  
    - Must support `#![no_std]`.  
    - Use minimal or no dynamic allocation.  
    - Possibly employ `heapless` or similar crates if needed for buffers.

---

## 1.2 Non-Functional Requirements

1. **Performance & Memory Footprint**  
   - Efficient in terms of CPU usage.  
   - Minimal memory overhead (aim to run on devices with < 32 KB RAM).

2. **Portability**  
   - Compatible with different MCUs (Cortex-M0+/M3/M4, RISC-V, etc.).  
   - Support different radio front-ends with an easily swappable trait-based design.

3. **Maintainability & Modularity**  
   - Split code into logical modules: LoRaWAN protocol, encryption, radio drivers, command handling, etc.  
   - Well-documented, clean APIs that can be extended for custom features.

4. **Testability**  
   - Comprehensive unit tests (especially for MAC-layer logic, encryption/MIC).  
   - Integration tests with reference LoRaWAN network servers (TTN, ChirpStack, etc.).  
   - Optional hardware-in-the-loop tests or mocking of the radio driver for CI.

5. **Documentation**  
   - Thorough user documentation including usage examples and explanation of the default downlink commands.  
   - Developer documentation for how to implement or extend the crate with new commands, new radio modules, or additional LoRaWAN regions.

---

# 2. File Structure Documentation

A recommended directory/file structure for this Rust crate:

```
lorawan_node/
├── Cargo.toml
├── README.md
├── LICENSE
├── src/
│   ├── lib.rs
│   ├── lorawan/
│   │   ├── mod.rs
│   │   ├── mac.rs
│   │   ├── phy.rs
│   │   ├── keys.rs
│   │   ├── region.rs
│   │   └── commands.rs
│   ├── radio/
│   │   ├── mod.rs
│   │   ├── traits.rs
│   │   ├── sx127x.rs
│   │   └── sx126x.rs
│   ├── crypto/
│   │   ├── aes.rs
│   │   └── mic.rs
│   ├── config/
│   │   ├── device.rs
│   │   ├── app.rs
│   │   └── region.rs
│   ├── class/
│   │   ├── class_a.rs
│   │   ├── class_b.rs
│   │   ├── class_c.rs
│   │   └── mod.rs
│   └── util/
│       └── timer.rs
└── tests/
    ├── integration_tests.rs
    └── unit_tests.rs
```

### Module & File Descriptions

- **Cargo.toml**  
  - Define crate metadata, Rust edition, and dependencies (e.g., `embedded-hal`, `aes`, `nb`, `heapless`).

- **README.md**  
  - Overview of the crate (purpose, quick start guides, examples).

- **LICENSE**  
  - Your chosen license (e.g., MIT/Apache2).

- **src/lib.rs**  
  - Main entry point of the crate, re-exporting core modules and traits.  
  - Global crate features (e.g., for selecting SX127x vs SX126x driver).

- **src/lorawan/**  
  - **mod.rs**: Re-exports for `mac`, `phy`, `keys`, `region`, `commands`.  
  - **mac.rs**: LoRaWAN MAC-layer logic (frame building, confirmed/unconfirmed frames, etc.).  
  - **phy.rs**: Physical layer settings (spreading factor, bandwidth, channel freq).  
  - **keys.rs**: Key management for OTAA/ABP (NwkSKey, AppSKey, etc.).  
  - **region.rs**: Region-specific parameters (mainly US915 for now).  
  - **commands.rs**: Default downlink commands (Set Interval, Show Firmware Version, Reboot) and trait-based approach for custom commands.

- **src/radio/**  
  - **mod.rs**: Exports the supported radio drivers and their traits.  
  - **traits.rs**: `Radio` trait definition (for radio init, set frequency, transmit, receive, etc.).  
  - **sx127x.rs** / **sx126x.rs**: Implementations of the `Radio` trait for specific Semtech radio families.

- **src/crypto/**  
  - **aes.rs**: AES-128 encryption/decryption routines.  
  - **mic.rs**: Compute message integrity code (MIC).

- **src/config/**  
  - **device.rs**: Device configuration structure (DevEUI, AppEUI, AppKey, etc.).  
  - **app.rs**: Application configuration (uplink intervals, data rate settings).  
  - **region.rs**: Region config constants and data (focused on US915 by default).

- **src/class/**  
  - **mod.rs**: Re-exports the LoRaWAN class modules (A, B, C).  
  - **class_a.rs**: Logic for Class A scheduling (uplink + 2 receive windows).  
  - **class_b.rs**: Handle beaconing, ping slots, and scheduled receive windows.  
  - **class_c.rs**: Keep receiver open except during transmit.

- **src/util/timer.rs**  
  - Timer utilities or traits for scheduling Class B beacons, Class A RX windows, etc.

- **tests/**  
  - **integration_tests.rs**: Tests verifying end-to-end flows (OTAA, uplink, downlink commands).  
  - **unit_tests.rs**: Focus on protocol logic, encryption, and MAC-layer unit testing.

---

1. **Documentation**  
   - Use Rust doc comments to document each module, struct, trait.  
   - Include examples of how to handle default downlink commands in the doc comments.
2. **Example Projects**  
   - Under an `examples/` directory, provide a bare-metal or RTOS-based example demonstrating:
     - OTAA join on US915  
     - Sending periodic uplinks  
     - Handling downlinks (including default commands: set interval, show firmware, reboot)
3. **README & Tutorials**  
   - Show wiring diagrams for radio modules, typical pin connections (SPI, DIO lines, etc.).  
   - Provide a quick start guide on how to integrate the crate into a user’s embedded application.

---

# Conclusion

By following these outlined requirements, file structure, and step-by-step instructions, you will have a robust Rust crate that supports **LoRaWAN Classes A, B, and C**, focuses on **US915 frequencies**, and includes **default downlink commands** (Set Interval, Show Firmware Version, Reboot) that can be extended for custom functionalities.

**Next Steps**:  
- Initialize the repository with this structure.  
- Implement the core LoRaWAN stack with a focus on US915.  
- Integrate the default downlink commands into your downlink parsing logic.  
- Add unit and integration tests to ensure reliability.  
- Document everything thoroughly for easy adoption.

Good luck building your embedded Rust LoRaWAN node crate!