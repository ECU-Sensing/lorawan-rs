//! # lorawan-rs
//! 
//! A `no_std` LoRaWAN stack implementation in Rust, supporting Class A, B, and C devices.
//! This crate provides a complete LoRaWAN 1.0.3 protocol implementation with focus on the US915 frequency plan.
//! 
//! ## Features
//! 
//! - Full LoRaWAN 1.0.3 stack implementation
//! - Support for Class A, B, and C devices
//! - US915 frequency plan
//! - OTAA and ABP activation
//! - Default downlink commands
//! - Extensible command handling
//! - `no_std` compatible for embedded systems
//! - Support for SX127x and SX126x radio modules
//! 
//! ## Example
//! 
//! ```rust,no_run
//! use lorawan::{
//!     config::device::DeviceConfig,
//!     device::LoRaWANDevice,
//!     class::OperatingMode,
//!     lorawan::region::US915,
//! };
//! 
//! // Initialize your radio (example with SX127x)
//! let radio = sx127x::SX127x::new(/* your SPI and GPIO pins */);
//! 
//! // Create device configuration for OTAA
//! let config = DeviceConfig::new_otaa(
//!     [0x01; 8], // DevEUI
//!     [0x02; 8], // AppEUI
//!     [0x03; 16], // AppKey
//! );
//! 
//! // Create region configuration
//! let region = US915::new();
//! 
//! // Create LoRaWAN device
//! let mut device = LoRaWANDevice::new(
//!     radio,
//!     config,
//!     region,
//!     OperatingMode::ClassA,
//! )?;
//! 
//! // Join network using OTAA
//! device.join_otaa()?;
//! 
//! // Send uplink data
//! let data = b"Hello LoRaWAN!";
//! device.send_uplink(1, data, false)?;
//! 
//! // Process device (handle receive windows)
//! device.process()?;
//! # Ok::<(), lorawan::Error>(())
//! ```
//! 
//! ## Handling Downlink Commands
//! 
//! The crate provides built-in support for common downlink commands:
//! 
//! ```rust,no_run
//! use lorawan::lorawan::commands::DownlinkCommand;
//! # use lorawan::{config::device::DeviceConfig, device::LoRaWANDevice, class::OperatingMode, lorawan::region::US915};
//! # let radio = ();
//! # let config = DeviceConfig::new_otaa([0; 8], [0; 8], [0; 16]);
//! # let region = US915::new();
//! # let mut device = LoRaWANDevice::new(radio, config, region, OperatingMode::ClassA).unwrap();
//! 
//! // Process device and handle downlink
//! if let Ok(Some(downlink)) = device.process() {
//!     match downlink {
//!         // Change uplink interval
//!         DownlinkCommand::SetInterval(interval_seconds) => {
//!             // Update your transmission interval
//!         }
//!         
//!         // Report firmware version
//!         DownlinkCommand::ShowFirmwareVersion => {
//!             let version = b"1.0.0";
//!             device.send_uplink(224, version, false)?;
//!         }
//!         
//!         // Reboot device
//!         DownlinkCommand::Reboot => {
//!             // Perform system reset
//!         }
//!         
//!         // Handle custom commands
//!         DownlinkCommand::Custom(port, payload) => {
//!             // Process custom command
//!         }
//!     }
//! }
//! # Ok::<(), lorawan::Error>(())
//! ```
//! 
//! ## Device Classes
//! 
//! The crate supports all LoRaWAN device classes:
//! 
//! - **Class A**: Basic class with two receive windows after each uplink
//! - **Class B**: Adds scheduled receive windows synchronized with beacon
//! - **Class C**: Continuous receive except when transmitting
//! 
//! ```rust,no_run
//! use lorawan::class::OperatingMode;
//! # use lorawan::{config::device::DeviceConfig, device::LoRaWANDevice, lorawan::region::US915};
//! # let radio = ();
//! # let config = DeviceConfig::new_otaa([0; 8], [0; 8], [0; 16]);
//! # let region = US915::new();
//! 
//! // Create Class A device
//! let mut device = LoRaWANDevice::new(
//!     radio,
//!     config,
//!     region,
//!     OperatingMode::ClassA,
//! )?;
//! 
//! // Switch to Class C
//! device.set_class(OperatingMode::ClassC)?;
//! # Ok::<(), lorawan::Error>(())
//! ```
//! 
//! ## Radio Support
//! 
//! The crate provides drivers for common LoRa radio modules:
//! 
//! - Semtech SX1276/77/78/79 (SX127x series)
//! - Semtech SX1261/62 (SX126x series)
//! 
//! Custom radio implementations can be added by implementing the `Radio` trait.
//! 
//! ## Safety
//! 
//! This crate uses `#![no_std]` and is intended for use in embedded systems.
//! It has been designed with safety in mind but has not been audited.
//! Use at your own risk in production systems.

#![no_std]
#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod config;
pub mod radio;
pub mod lorawan;
pub mod class;
pub mod device;
pub mod crypto;

/// Error type for the LoRaWAN stack
#[derive(Debug)]
pub enum Error {
    /// Radio hardware error
    Radio,
    /// Invalid configuration
    Config,
    /// Join procedure failed
    Join,
    /// Transmission failed
    Tx,
    /// Reception failed
    Rx,
    /// MAC layer error
    Mac,
    /// Crypto operation failed
    Crypto,
    /// Invalid state for operation
    InvalidState,
    /// Buffer too small
    BufferTooSmall,
    /// Invalid parameter
    InvalidParam,
}

/// Result type for the LoRaWAN stack
pub type Result<T> = core::result::Result<T, Error>;
