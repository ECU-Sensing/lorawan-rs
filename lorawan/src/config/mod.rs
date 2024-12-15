//! Device and network configuration
//!
//! This module contains types and functions for configuring LoRaWAN devices
//! and network parameters. It includes:
//! - Device configuration (DevEUI, AppEUI, keys)
//! - Session state management
//! - Network parameters

/// Device configuration and session state
pub mod device;

pub use device::DeviceConfig;
