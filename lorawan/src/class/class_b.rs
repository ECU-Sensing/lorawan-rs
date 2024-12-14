use core::time::Duration;
use heapless::Vec;

use crate::lorawan::{
    mac::{MacError, MacLayer},
    region::Region,
};
use crate::radio::Radio;
use super::{BeaconTiming, ClassBState, DeviceClass, OperatingMode, PingSlot};

/// Beacon timing constants
pub const BEACON_PERIOD: u32 = 128; // seconds
pub const BEACON_RESERVED: u32 = 2_120; // ms
pub const BEACON_GUARD: u32 = 3_000; // ms
pub const BEACON_WINDOW: u32 = 122_880; // ms
pub const BEACON_SLOT_LEN: u32 = 30; // ms

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
    pub fn parse(data: &[u8]) -> Result<Self, MacError<Radio::Error>> {
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
    /// Ping slot state
    ping_slot: PingSlotState,
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
            ping_slot: PingSlotState::new(32, 0, 0), // Default values
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
        match self.beacon_state {
            BeaconState::ColdStart => {
                // Scan all possible beacon channels
                let beacon_channels = self.mac.get_beacon_channels();
                for channel in beacon_channels {
                    // Configure radio for beacon reception
                    self.mac.set_rx_config(
                        channel.frequency,
                        channel.data_rate,
                        false, // Not continuous
                    )?;

                    // Try to receive beacon
                    let mut buffer = [0u8; 17];
                    if let Ok(len) = self.mac.receive(&mut buffer) {
                        if let Ok(beacon) = BeaconFrame::parse(&buffer[..len]) {
                            self.handle_beacon(beacon)?;
                            return Ok(());
                        }
                    }
                }
                // No beacon found, stay in cold start
            }
            BeaconState::WarmStart => {
                // Use last known timing
                if let Some(last_beacon) = self.last_beacon {
                    let elapsed = self.current_time - last_beacon;
                    let beacon_period = Duration::from_secs(BEACON_PERIOD as u64);
                    
                    // Calculate time to next beacon window
                    if elapsed >= beacon_period {
                        // Missed beacon, go back to cold start
                        self.beacon_state = BeaconState::ColdStart;
                    } else {
                        let time_to_window = beacon_period - elapsed;
                        if time_to_window.as_millis() <= BEACON_WINDOW as u128 {
                            // In beacon window, try to receive
                            let channel = self.mac.get_next_beacon_channel();
                            if let Some(channel) = channel {
                                self.mac.set_rx_config(
                                    channel.frequency,
                                    channel.data_rate,
                                    false,
                                )?;

                                let mut buffer = [0u8; 17];
                                if let Ok(len) = self.mac.receive(&mut buffer) {
                                    if let Ok(beacon) = BeaconFrame::parse(&buffer[..len]) {
                                        self.handle_beacon(beacon)?;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            BeaconState::Synchronized => {
                // Regular beacon reception
                let mut buffer = [0u8; 17];
                if let Ok(len) = self.mac.receive(&mut buffer) {
                    if let Ok(beacon) = BeaconFrame::parse(&buffer[..len]) {
                        self.handle_beacon(beacon)?;
                    } else {
                        // Failed to parse beacon
                        self.ping_slot.record_missed_slot();
                        if self.ping_slot.get_missed_slots() > 2 {
                            // Lost synchronization after missing 3 beacons
                            self.beacon_state = BeaconState::Lost;
                        }
                    }
                }
            }
            BeaconState::Lost => {
                // Reset and start cold scan
                self.last_beacon = None;
                self.beacon_state = BeaconState::ColdStart;
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle received beacon
    fn handle_beacon(&mut self, beacon: BeaconFrame) -> Result<(), MacError<R::Error>> {
        // Update timing
        self.last_beacon = Some(self.current_time);
        
        // Update ping slot timing if needed
        if let Some(dev_addr) = self.mac.get_device_address() {
            let calculator = PingSlotCalculator::new(
                dev_addr,
                beacon.time,
                self.ping_slot.period,
            );
            self.ping_slot.update_next_slot(&calculator);
        }

        // Mark as synchronized
        self.beacon_state = BeaconState::Synchronized;
        
        Ok(())
    }

    /// Process ping slots
    fn process_ping_slots(&mut self) -> Result<(), MacError<R::Error>> {
        if self.beacon_state != BeaconState::Synchronized {
            return Ok(());
        }

        // Check if we're in a ping slot
        if let Some(last_beacon) = self.last_beacon {
            let elapsed = self.current_time - last_beacon;
            let elapsed_ms = elapsed.as_millis() as u32;
            
            // Calculate if we're in an active ping slot
            let slot_offset = elapsed_ms % (self.ping_slot.period * 1000);
            if slot_offset >= self.ping_slot.next_slot && 
               slot_offset < self.ping_slot.next_slot + BEACON_SLOT_LEN {
                // We're in an active ping slot
                self.mac.set_rx_config(
                    self.ping_slot.frequency,
                    self.ping_slot.data_rate,
                    false,
                )?;

                // Listen for downlink
                let mut buffer = [0u8; 256];
                if let Ok(len) = self.mac.receive(&mut buffer) {
                    // Process received data
                    self.handle_ping_slot_data(&buffer[..len])?;
                    self.ping_slot.reset_missed_slots();
                } else {
                    self.ping_slot.record_missed_slot();
                }
            }
        }

        Ok(())
    }

    /// Handle data received during ping slot
    fn handle_ping_slot_data(&mut self, data: &[u8]) -> Result<(), MacError<R::Error>> {
        // Verify MIC and decrypt payload
        if let Ok(payload) = self.mac.decrypt_payload(data) {
            // Process any MAC commands
            if let Some(commands) = self.mac.extract_mac_commands(&payload) {
                for cmd in commands {
                    self.mac.process_mac_command(cmd)?;
                }
            }
        }
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

    fn process(&mut self) -> Result<(), Self::Error> {
        // Process beacon
        self.process_beacon()?;

        // Process ping slots
        self.process_ping_slots()?;

        Ok(())
    }

    fn send_data(&mut self, port: u8, data: &[u8], confirmed: bool) -> Result<(), Self::Error> {
        // Send data using MAC layer
        if confirmed {
            self.mac.send_confirmed(port, data)?;
        } else {
            self.mac.send_unconfirmed(port, data)?;
        }

        Ok(())
    }

    fn send_join_request(&mut self, dev_eui: [u8; 8], app_eui: [u8; 8], app_key: [u8; 16]) -> Result<(), Self::Error> {
        // Reset beacon synchronization
        self.beacon_state = BeaconState::NotSynchronized;
        self.last_beacon = None;
        self.class_b.clear_ping_slots();

        // Send join request using MAC layer
        self.mac.join_request(dev_eui, app_eui, app_key)
    }

    fn receive(&mut self, buffer: &mut [u8]) -> Result<usize, Self::Error> {
        // Only receive if synchronized and in appropriate window
        if self.beacon_state != BeaconState::Synchronized {
            return Ok(0);
        }

        // Receive using MAC layer
        self.mac.receive(buffer)
    }
} 