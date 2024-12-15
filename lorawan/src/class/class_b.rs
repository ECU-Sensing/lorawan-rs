//! LoRaWAN Class B device implementation
//!
//! Class B devices extend Class A by adding scheduled receive slots synchronized
//! with a network beacon. This allows for deterministic downlink latency.

use core::time::Duration;

use super::{ClassBState, DeviceClass, OperatingMode};
use crate::config::device::AESKey;
use crate::lorawan::mac::{MacError, MacLayer};
use crate::lorawan::region::Region;
use crate::radio::traits::Radio;

/// Beacon period in seconds
pub const BEACON_PERIOD: u32 = 128;

/// Reserved time at start of beacon window in milliseconds
pub const BEACON_RESERVED: u32 = 2_120;

/// Guard time around beacon window in milliseconds
pub const BEACON_GUARD: u32 = 3_000;

/// Total beacon window duration in milliseconds
pub const BEACON_WINDOW: u32 = 122_880;

/// Duration of each beacon slot in milliseconds
pub const BEACON_SLOT_LEN: u32 = 30;

/// Beacon acquisition states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BeaconState {
    /// Not synchronized with network beacons
    NotSynchronized,
    /// Cold start - scanning all possible channels
    ColdStart,
    /// Warm start - using last known timing
    WarmStart,
    /// Synchronized with network beacons
    Synchronized,
    /// Lost synchronization - missed beacons
    Lost,
}

/// Beacon frame structure
#[derive(Debug, Clone)]
pub struct BeaconFrame {
    /// Network time in seconds since GPS epoch
    pub time: u32,
    /// CRC of the beacon payload
    pub crc: u16,
    /// Gateway specific info
    pub gwspec: u8,
    /// Additional beacon information
    pub info: [u8; 7],
}

impl BeaconFrame {
    /// Parse beacon frame from raw bytes
    pub fn parse<E>(data: &[u8]) -> Result<Self, MacError<E>> {
        if data.len() < 17 {
            return Err(MacError::InvalidLength);
        }

        let time = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let crc = u16::from_be_bytes([data[4], data[5]]);
        let gwspec = data[6];
        let mut info = [0u8; 7];
        info.copy_from_slice(&data[7..14]);

        Ok(Self {
            time,
            crc,
            gwspec,
            info,
        })
    }
}

/// Ping slot timing calculator
pub struct PingSlotCalculator {
    /// Device address used for slot calculation
    dev_addr: u32,
    /// Current beacon time
    beacon_time: u32,
    /// Ping period (32s to 128s)
    ping_period: u32,
}

impl PingSlotCalculator {
    /// Create new ping slot calculator
    pub fn new(dev_addr: u32, beacon_time: u32, ping_period: u32) -> Self {
        Self {
            dev_addr,
            beacon_time,
            ping_period,
        }
    }

    /// Calculate ping offset within the beacon period
    pub fn calculate_ping_offset(&self) -> u32 {
        // Implementation based on LoRaWAN specification
        let rand = self.dev_addr.wrapping_mul(self.beacon_time);
        (rand % self.ping_period) * BEACON_SLOT_LEN
    }
}

/// Ping slot state tracking
#[derive(Debug, Clone)]
pub struct PingSlotState {
    /// Next scheduled ping slot time
    pub next_slot: u32,
    /// Current ping period
    pub period: u32,
    /// Frequency for reception
    pub frequency: u32,
    /// Data rate for reception
    pub data_rate: u8,
    /// Number of missed ping slots
    missed_slots: u32,
}

impl PingSlotState {
    /// Create new ping slot state
    pub fn new(period: u32, frequency: u32, data_rate: u8) -> Self {
        Self {
            next_slot: 0,
            period,
            frequency,
            data_rate,
            missed_slots: 0,
        }
    }

    /// Update next ping slot timing
    pub fn update_next_slot(&mut self, calculator: &PingSlotCalculator) {
        self.next_slot = calculator.calculate_ping_offset();
    }

    /// Record a missed ping slot
    pub fn record_missed_slot(&mut self) {
        self.missed_slots = self.missed_slots.saturating_add(1);
    }

    /// Reset missed slot counter
    pub fn reset_missed_slots(&mut self) {
        self.missed_slots = 0;
    }

    /// Get number of consecutive missed slots
    pub fn get_missed_slots(&self) -> u32 {
        self.missed_slots
    }
}

/// Class B device implementation
pub struct ClassB<R: Radio, REG: Region> {
    /// MAC layer
    mac: MacLayer<R, REG>,
    /// Current beacon state
    beacon_state: BeaconState,
    /// Class B specific state
    class_b: ClassBState,
    /// Last beacon timestamp
    last_beacon: Option<Duration>,
    /// Current time (would be provided by timer in real implementation)
    current_time: Duration,
}

impl<R: Radio, REG: Region> ClassB<R, REG> {
    /// Create new Class B device
    pub fn new(mac: MacLayer<R, REG>) -> Self {
        Self {
            mac,
            beacon_state: BeaconState::NotSynchronized,
            class_b: ClassBState::new(),
            last_beacon: None,
            current_time: Duration::from_secs(0),
        }
    }

    /// Start beacon acquisition
    pub fn start_beacon_acquisition(&mut self) {
        match self.last_beacon {
            Some(_) => {
                // Warm start - we have previous timing info
                self.beacon_state = BeaconState::WarmStart;
            }
            None => {
                // Cold start - scan all channels
                self.beacon_state = BeaconState::ColdStart;
            }
        }
    }

    /// Process beacon reception
    fn process_beacon(&mut self) -> Result<(), MacError<R::Error>> {
        let channel = self.mac.get_next_channel()?;
        self.mac.set_rx_config(
            channel.frequency,
            channel.min_dr,
            0, // Continuous mode
        )?;
        Ok(())
    }

    /// Process ping slots
    fn process_ping_slots(&mut self) -> Result<(), MacError<R::Error>> {
        let channel = self.mac.get_next_channel()?;
        self.mac.set_rx_config(
            channel.frequency,
            channel.min_dr,
            0, // Continuous mode
        )?;
        Ok(())
    }

    /// Update timing
    pub fn update_time(&mut self, time: Duration) {
        self.current_time = time;
    }
}

impl<R: Radio, REG: Region> DeviceClass for ClassB<R, REG> {
    type Error = MacError<R::Error>;

    fn operating_mode(&self) -> OperatingMode {
        OperatingMode::ClassB
    }

    fn process(&mut self) -> Result<(), MacError<R::Error>> {
        // Process beacon
        self.process_beacon()?;

        // Process ping slots
        self.process_ping_slots()?;

        Ok(())
    }

    fn send_data(&mut self, port: u8, data: &[u8], confirmed: bool) -> Result<(), MacError<R::Error>> {
        if confirmed {
            self.mac.send_confirmed(port, data)
        } else {
            self.mac.send_unconfirmed(port, data)
        }
    }

    fn send_join_request(
        &mut self,
        dev_eui: [u8; 8],
        app_eui: [u8; 8],
        app_key: AESKey,
    ) -> Result<(), MacError<R::Error>> {
        // Reset beacon synchronization
        self.beacon_state = BeaconState::NotSynchronized;
        self.last_beacon = None;
        self.class_b.clear_ping_slots();

        // Send join request using MAC layer
        self.mac.join_request(dev_eui, app_eui, app_key)
    }

    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, MacError<R::Error>> {
        // Only receive if synchronized and in appropriate window
        if self.beacon_state != BeaconState::Synchronized {
            return Ok(0);
        }

        // Receive using MAC layer
        self.mac.receive(buffer)
    }
}
