//! LoRaWAN protocol implementation in Rust
//!
//! This crate provides a pure Rust implementation of the LoRaWAN protocol stack.
//! It supports Class A, B, and C devices, OTAA and ABP activation, and implements
//! the LoRaWAN 1.0.3 specification.
//!
//! # Features
//! - Complete LoRaWAN 1.0.3 implementation
//! - Class A, B, and C device support
//! - OTAA and ABP activation
//! - Configurable regions (US915, EU868, etc.)
//! - Hardware abstraction layer for radio drivers
//! - No unsafe code
//!
//! # Example
//! ```ignore
//! use lorawan::{
//!     config::device::{DeviceConfig, AESKey},
//!     device::LoRaWANDevice,
//!     class::OperatingMode,
//!     lorawan::region::US915,
//! };
//!
//! // Create device configuration
//! let config = DeviceConfig::new_otaa(
//!     [0x00; 8], // DevEUI
//!     [0x00; 8], // AppEUI
//!     AESKey::new([0x00; 16]), // AppKey
//! );
//!
//! // Create region configuration
//! let region = US915::new();
//!
//! // Create device with radio (implementation not shown)
//! let mut device = LoRaWANDevice::new(radio, config, region, OperatingMode::ClassA).unwrap();
//!
//! // Join network
//! device.join_otaa([0x00; 8], [0x00; 8], AESKey::new([0x00; 16])).unwrap();
//!
//! // Send data
//! let data = b"Hello, LoRaWAN!";
//! device.send_data(1, data, false).unwrap();
//! ```

#![warn(missing_docs)]
#![no_std]

/// Device class implementations (A, B, C)
pub mod class;

/// Device and network configuration
pub mod config;

/// Cryptographic functions
pub mod crypto;

/// High-level device interface
pub mod device;

/// LoRaWAN protocol implementation
pub mod lorawan;

/// Radio hardware abstraction layer
pub mod radio;
