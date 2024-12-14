
## Step 1: Create and Configure the Crate

1. **Initialize Project**  
   ```bash
   cargo new lorawan --lib
   cd lorawan
   ```
2. **Set Up `Cargo.toml`**  
   - Minimal example:
     ```toml
     [package]
     name = "lorawan"
     version = "0.1.0"
     edition = "2021"

     [dependencies]
     embedded-hal = "0.2"
     aes = "0.8"       # For AES-128 encryption
     # Optionally:
     # heapless = "0.7"
     # nb = "1.0"

     [features]
     default = ["sx127x"]
     sx127x = []
     sx126x = []
     ```

3. **`#![no_std]`**  
   - In `src/lib.rs`:
     ```rust
     #![no_std]
     ```

---

## Step 2: Define Core Data Structures & Configuration

1. **Device Configuration**  
   - `DeviceConfig` in `src/config/device.rs`, storing DevEUI, AppEUI, AppKey, ABP parameters (dev_addr, nwk_skey, app_skey), etc.
2. **Session State**  
   - Keep track of activation status, session keys, dev_addr, frame counters, Class mode (A/B/C).
3. **Radio Trait**  
   - In `src/radio/traits.rs`, define the `Radio` trait with initialization and transmit/receive methods:
     ```rust
     pub trait Radio {
         type Error;

         fn init(&mut self) -> Result<(), Self::Error>;
         fn set_frequency(&mut self, freq: u32) -> Result<(), Self::Error>;
         fn set_tx_power(&mut self, power: i8) -> Result<(), Self::Error>;
         fn transmit(&mut self, buffer: &[u8]) -> Result<(), Self::Error>;
         fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error>;
         // Additional class-specific or radio-specific methods
     }
     ```

---

## Step 3: Implement Radio Drivers

1. **SX127x Driver**  
   - `src/radio/sx127x.rs`: Use `embedded-hal` SPI and GPIO traits to communicate with the SX127x chip.  
   - Implement the `Radio` trait methods: `init`, `set_frequency`, `set_tx_power`, `transmit`, and `receive`.
2. **SX126x Driver**  
   - `src/radio/sx126x.rs`: Similarly implement `Radio` trait.  
   - Use feature gating so this driver is only compiled if the `sx126x` feature is enabled.

---

## Step 4: LoRaWAN Protocol Stack

1. **Region Handling** (US915 Focus)  
   - In `src/lorawan/region.rs`, define frequency sub-bands, default channels, data rates for US915.  
   - Provide functions to select sub-band, set data rate, etc.  
2. **PHY Layer** (`src/lorawan/phy.rs`)  
   - Define basic LoRa parameters (SF7 - SF12, bandwidth, coding rate).  
   - Class-specific adjustments to RX windows.
3. **MAC Layer** (`src/lorawan/mac.rs`)  
   - Build and parse LoRaWAN frames (FHDR, FPort, MAC commands).  
   - Confirmed and unconfirmed uplink logic, handling frame counters.  
   - Manage session state transitions (joined/unjoined).
4. **Session Keys & Encryption** (`src/lorawan/keys.rs` & `src/crypto/`)  
   - Generate session keys from join-accept for OTAA.  
   - Encrypt payloads and compute MIC with AES-128-based logic.

---

## Step 5: Classes A, B, and C

Implement separate logic for each LoRaWAN class:

1. **Class A** (`src/class/class_a.rs`)  
   - After an uplink, open two short receive windows.  
   - Timer-based scheduling of RX1 and RX2 windows.
2. **Class B** (`src/class/class_b.rs`)  
   - Handle beacon reception to synchronize.  
   - Manage ping slots for scheduled downlink availability.  
   - Keep track of beacon timing and scheduling multiple RX windows.
3. **Class C** (`src/class/class_c.rs`)  
   - Continuous receive window except when transmitting.  
   - Reconfigure the radio to RX mode after every TX is done.

A unified API can exist in `src/class/mod.rs` to let the user select which class the device should run.

---

## Step 6: Default Downlink Commands

Create a base command handling mechanism in **`src/lorawan/commands.rs`**. For example:

```rust
pub enum DownlinkCommand {
    SetInterval(u32),
    ShowFirmwareVersion,
    Reboot,
    Custom(u8, Vec<u8>), // Catch-all for user-defined commands
}

/// Trait that a device implements to process downlink commands
pub trait CommandHandler {
    fn handle_downlink_cmd(&mut self, command: DownlinkCommand);
}
```

**Default Commands** Implementation Details:
1. **Set Interval**  
   - Downlink payload may contain a `u32` representing a new interval in seconds.  
   - Once parsed, update your device config or application config to the new interval.
2. **Show Firmware Version**  
   - The device can respond (via the next uplink or a queued downlink response) with its firmware version string or integer.
3. **Reboot**  
   - Trigger a software reset or set a flag that your main loop can interpret as a reboot request.

**Extendable Base Class**  
- Users can implement `CommandHandler` themselves, adding match arms for custom commands: 
  ```rust
  impl CommandHandler for MyDevice {
      fn handle_downlink_cmd(&mut self, command: DownlinkCommand) {
          match command {
              DownlinkCommand::SetInterval(interval) => {
                  self.config.app.uplink_interval_s = interval;
                  // apply changes
              }
              DownlinkCommand::ShowFirmwareVersion => {
                  // Prepare a response or set a flag for next uplink
              }
              DownlinkCommand::Reboot => {
                  // Trigger a device reboot
              }
              DownlinkCommand::Custom(cmd_id, data) => {
                  // Handle custom logic
              }
          }
      }
  }
  ```

**Usage**  
- In the downlink parsing logic (e.g., in `mac.rs`), parse the FPort or MAC command field, map it to a `DownlinkCommand`, and call `CommandHandler::handle_downlink_cmd`.  
- Documentation should show how to register or instantiate this command handler in user code.

---

## Step 7: High-Level API

1. **Initialization**  
   - `LorawanDevice::new(radio: R, device_cfg: DeviceConfig, region_cfg: RegionCfg, class: DeviceClass) -> Self`  
   - Set up LoRaWAN parameters for US915, set the class mode, and initialize the radio.
2. **Join Procedure**  
   - `join_otaa(&mut self)` or `activate_abp(&mut self)` to handle device activation flows.
3. **Send / Receive**  
   - `send_uplink(&mut self, port: u8, data: &[u8], confirmed: bool)`  
   - Internally build LoRaWAN frames, encrypt them, update counters, transmit via the radio driver.  
   - Post-transmission, handle Class A, B, or C receive windows.  
   - **Downlink Handling**: Inside receive logic, parse the received frame, decode commands, call `CommandHandler`.
4. **Class Management**  
   - `set_class(&mut self, class: DeviceClass)` to switch between Class A, B, or C. Note that Class B requires beacon synchronization.

---

## Step 8: Testing & Validation

1. **Unit Tests** (`tests/unit_tests.rs`)  
   - Check MAC-layer logic, default downlink commands, AES encryption, MIC calculations.  
   - Validate US915 channel plan behavior (sub-band selection, data rates).
2. **Integration Tests** (`tests/integration_tests.rs`)  
   - Hardware-in-the-loop or simulated environment with a LoRaWAN server (like TTN or ChirpStack).  
   - Test OTAA, uplink/downlink message exchange, verify default downlink commands.
3. **Continuous Integration**  
   - Integrate with GitHub Actions or another CI tool to run tests automatically on commits.

---

## Step 9: Documentation & Examples

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


