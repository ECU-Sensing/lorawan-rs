//! Radio hardware abstraction layer
//!
//! This module provides traits and implementations for LoRa radio hardware:
//! - Common radio traits for hardware abstraction
//! - SX127x series radio driver (SX1276/77/78/79)
//! - SX126x series radio driver (when enabled with "sx126x" feature)
//! - Configuration types for radio operation

#[cfg(feature = "sx126x")]
/// SX126x series radio driver
pub mod sx126x;

/// SX127x series radio driver
pub mod sx127x;

/// Common traits for radio hardware abstraction
pub mod traits;

#[cfg(feature = "sx126x")]
pub use sx126x::SX126x;

/// Re-export of SX127x radio driver
pub use sx127x::SX127x;

/// Re-export of Radio trait
pub use traits::Radio;
