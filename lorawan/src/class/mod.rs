pub mod class_a;
pub mod class_b;
pub mod class_c;

use core::time::Duration;
use heapless::Vec;

use crate::lorawan::{
    mac::MacLayer,
    region::Region,
};
use crate::radio::Radio;

/// Maximum number of scheduled ping slots
pub const MAX_PING_SLOTS: usize = 16;

/// Beacon timing parameters
#[derive(Debug, Clone, Copy)]
pub struct BeaconTiming {
    /// Time to next beacon in seconds
    pub time_to_beacon: u32,
    /// Channel for next beacon
    pub channel: u32,
}

/// Device operating mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperatingMode {
    /// Class A: Two receive windows after each uplink
    ClassA,
    /// Class B: Scheduled receive slots using beacons
    ClassB,
    /// Class C: Continuous receive
    ClassC,
}

/// Common trait for all device classes
pub trait DeviceClass {
    /// Radio error type
    type Error;

    /// Get current operating mode
    fn operating_mode(&self) -> OperatingMode;

    /// Process device operations
    fn process(&mut self) -> Result<(), Self::Error>;

    /// Send data (will handle receive windows according to class)
    fn send_data(&mut self, port: u8, data: &[u8], confirmed: bool) -> Result<(), Self::Error>;

    /// Send join request
    fn send_join_request(&mut self, dev_eui: [u8; 8], app_eui: [u8; 8], app_key: [u8; 16]) -> Result<(), Self::Error>;

    /// Receive data
    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error>;
}

/// Class B ping slot timing
#[derive(Debug, Clone, Copy)]
pub struct PingSlot {
    /// Absolute time of ping slot in seconds since beacon
    pub time: u32,
    /// Duration of ping slot in milliseconds
    pub duration: u32,
    /// Channel frequency
    pub frequency: u32,
}

/// Class B state
#[derive(Debug)]
pub struct ClassBState {
    /// Last received beacon timing
    pub beacon_timing: Option<BeaconTiming>,
    /// Scheduled ping slots
    pub ping_slots: Vec<PingSlot, MAX_PING_SLOTS>,
}

impl ClassBState {
    /// Create new Class B state
    pub fn new() -> Self {
        Self {
            beacon_timing: None,
            ping_slots: Vec::new(),
        }
    }

    /// Schedule a new ping slot
    pub fn schedule_ping_slot(&mut self, slot: PingSlot) -> Result<(), ()> {
        self.ping_slots.push(slot).map_err(|_| ())
    }

    /// Clear all ping slots
    pub fn clear_ping_slots(&mut self) {
        self.ping_slots.clear();
    }
}

/// Class C state
#[derive(Debug)]
pub struct ClassCState {
    /// RX2 parameters for continuous receive
    pub rx2_frequency: u32,
    pub rx2_data_rate: u8,
}

impl ClassCState {
    /// Create new Class C state
    pub fn new(rx2_frequency: u32, rx2_data_rate: u8) -> Self {
        Self {
            rx2_frequency,
            rx2_data_rate,
        }
    }
}

pub use class_a::ClassA;
pub use class_b::ClassB;
pub use class_c::ClassC; 