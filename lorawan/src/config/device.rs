//! Device configuration and session state
//!
//! This module provides types for configuring LoRaWAN devices and managing their session state.
//! It includes:
//! - Device address handling
//! - AES key management
//! - Device configuration for OTAA and ABP activation
//! - Session state tracking

/// Device address (4 bytes)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DevAddr {
    bytes: [u8; 4],
}

impl DevAddr {
    /// Create a new device address from raw bytes
    pub fn new(bytes: [u8; 4]) -> Self {
        Self { bytes }
    }

    /// Get the raw bytes of the device address
    pub fn as_bytes(&self) -> &[u8; 4] {
        &self.bytes
    }
}

/// AES-128 key (16 bytes)
#[derive(Debug, Clone)]
pub struct AESKey {
    bytes: [u8; 16],
}

impl AESKey {
    /// Create a new AES key from raw bytes
    pub fn new(bytes: [u8; 16]) -> Self {
        Self { bytes }
    }

    /// Get the raw bytes of the key
    pub fn as_bytes(&self) -> &[u8; 16] {
        &self.bytes
    }
}

/// 64-bit Extended Unique Identifier (EUI)
pub type EUI64 = [u8; 8];

/// Device configuration
#[derive(Debug, Clone)]
pub struct DeviceConfig {
    /// Device EUI (unique device identifier)
    pub dev_eui: EUI64,
    /// Application EUI (unique application identifier)
    pub app_eui: EUI64,
    /// Application key (root key for OTAA)
    pub app_key: AESKey,
    /// Device address (assigned during activation)
    pub dev_addr: Option<DevAddr>,
    /// Network session key (derived during activation)
    pub nwk_skey: Option<AESKey>,
    /// Application session key (derived during activation)
    pub app_skey: Option<AESKey>,
}

impl DeviceConfig {
    /// Create a new device configuration for OTAA activation
    pub fn new_otaa(dev_eui: EUI64, app_eui: EUI64, app_key: AESKey) -> Self {
        Self {
            dev_eui,
            app_eui,
            app_key,
            dev_addr: None,
            nwk_skey: None,
            app_skey: None,
        }
    }

    /// Create a new device configuration for ABP activation
    pub fn new_abp(
        dev_eui: EUI64,
        app_eui: EUI64,
        dev_addr: DevAddr,
        nwk_skey: AESKey,
        app_skey: AESKey,
    ) -> Self {
        Self {
            dev_eui,
            app_eui,
            app_key: AESKey::new([0; 16]), // Not used in ABP
            dev_addr: Some(dev_addr),
            nwk_skey: Some(nwk_skey),
            app_skey: Some(app_skey),
        }
    }
}

/// Session state
#[derive(Debug, Clone)]
pub struct SessionState {
    /// Device address
    pub dev_addr: DevAddr,
    /// Network session key
    pub nwk_skey: AESKey,
    /// Application session key
    pub app_skey: AESKey,
    /// Uplink frame counter
    pub fcnt_up: u32,
    /// Downlink frame counter
    pub fcnt_down: u32,
}

impl SessionState {
    /// Create a new empty session state with default values
    pub fn new() -> Self {
        Self {
            dev_addr: DevAddr::new([0; 4]),
            nwk_skey: AESKey::new([0; 16]),
            app_skey: AESKey::new([0; 16]),
            fcnt_up: 0,
            fcnt_down: 0,
        }
    }

    /// Create a new session state for ABP activation
    pub fn new_abp(dev_addr: DevAddr, nwk_skey: AESKey, app_skey: AESKey) -> Self {
        Self {
            dev_addr,
            nwk_skey,
            app_skey,
            fcnt_up: 0,
            fcnt_down: 0,
        }
    }

    /// Create a new session state from OTAA join response
    pub fn from_join_accept(dev_addr: DevAddr, nwk_skey: AESKey, app_skey: AESKey) -> Self {
        Self {
            dev_addr,
            nwk_skey,
            app_skey,
            fcnt_up: 0,
            fcnt_down: 0,
        }
    }

    /// Reset frame counters
    pub fn reset_counters(&mut self) {
        self.fcnt_up = 0;
        self.fcnt_down = 0;
    }

    /// Check if session is active (has valid keys)
    pub fn is_active(&self) -> bool {
        // Check if keys are non-zero
        !self.nwk_skey.as_bytes().iter().all(|&x| x == 0)
            && !self.app_skey.as_bytes().iter().all(|&x| x == 0)
    }

    /// Check if device is joined to network
    pub fn is_joined(&self) -> bool {
        !self.dev_addr.as_bytes().iter().all(|&x| x == 0) && self.is_active()
    }
}
