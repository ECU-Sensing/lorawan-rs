//! LoRaWAN device class implementations
//!
//! This module contains the implementations of the three LoRaWAN device classes:
//! - Class A: Basic bi-directional communication with two receive windows after each uplink
//! - Class B: Scheduled receive slots synchronized with network beacon
//! - Class C: Continuous receive except when transmitting

/// Class A device implementation
pub mod class_a;

/// Class B device implementation
pub mod class_b;
pub use class_b::ClassB;

/// Class C device implementation
pub mod class_c;

use crate::config::device::{AESKey, SessionState};
use crate::lorawan::mac::MacLayer;
use crate::lorawan::region::Region;
use crate::radio::traits::Radio;

/// Device operating mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperatingMode {
    /// Class A: Basic bi-directional communication
    ClassA,
    /// Class B: Scheduled receive slots
    ClassB,
    /// Class C: Continuous receive
    ClassC,
}

/// Common trait for all device classes
pub trait DeviceClass<R: Radio, REG: Region> {
    /// Error type for device operations
    type Error;

    /// Get current operating mode
    fn operating_mode(&self) -> OperatingMode;

    /// Process device operations
    fn process(&mut self) -> Result<(), Self::Error>;

    /// Send data
    fn send_data(&mut self, port: u8, data: &[u8], confirmed: bool) -> Result<(), Self::Error>;

    /// Send join request
    fn send_join_request(
        &mut self,
        dev_eui: [u8; 8],
        app_eui: [u8; 8],
        app_key: AESKey,
    ) -> Result<(), Self::Error>;

    /// Receive data
    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error>;

    /// Get session state
    fn get_session_state(&self) -> SessionState;

    /// Get MAC layer reference
    fn get_mac_layer(&self) -> &MacLayer<R, REG>;
}

/// RX window configuration
#[derive(Debug, Clone)]
pub struct RxConfig {
    /// RX window frequency in Hz
    pub frequency: u32,
    /// RX window data rate index
    pub rx2_data_rate: u8,
    /// RX window timeout in milliseconds
    pub rx_timeout: u32,
}

/// Class B state
#[derive(Debug)]
pub struct ClassBState {
    /// Next ping slot time
    pub next_ping_slot: u32,
    /// Ping slot period
    pub ping_period: u32,
    /// Ping slot frequency
    pub ping_frequency: u32,
    /// Ping slot data rate
    pub ping_data_rate: u8,
}

impl ClassBState {
    /// Create new Class B state
    pub fn new() -> Self {
        Self {
            next_ping_slot: 0,
            ping_period: 32,
            ping_frequency: 0,
            ping_data_rate: 0,
        }
    }

    /// Clear ping slots
    pub fn clear_ping_slots(&mut self) {
        self.next_ping_slot = 0;
    }
}
