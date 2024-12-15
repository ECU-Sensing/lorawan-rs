//! LoRaWAN Class B Implementation
//!
//! This module implements LoRaWAN Class B functionality including:
//! - Beacon synchronization and tracking
//! - Ping slot timing and randomization
//! - Network time synchronization
//! - Beacon loss detection and recovery

mod beacon;
mod ping_slot;
mod timing;

use crate::{
    radio::traits::Radio,
    lorawan::{
        region::Region,
        mac::{MacLayer, Error},
    },
};

use self::{
    beacon::{BeaconTracker, BeaconState},
    ping_slot::{PingSlotConfig, PingSlotScheduler},
    timing::NetworkTime,
};

use heapless::Vec;

/// Maximum number of ping slots per beacon period
const MAX_PING_SLOTS: usize = 16;

/// Class B device implementation
pub struct ClassB<R: Radio + Clone, REG: Region> {
    /// MAC layer for radio communication
    mac: MacLayer<R, REG>,
    /// Beacon tracking state
    beacon_tracker: BeaconTracker,
    /// Ping slot configuration
    ping_slot_config: PingSlotConfig,
    /// Ping slot scheduler
    ping_scheduler: PingSlotScheduler,
    /// Network time synchronization
    network_time: NetworkTime,
}

impl<R: Radio + Clone, REG: Region> ClassB<R, REG> {
    /// Create new Class B device
    pub fn new(mac: MacLayer<R, REG>) -> Self {
        Self {
            mac,
            beacon_tracker: BeaconTracker::new(),
            ping_slot_config: PingSlotConfig::default(),
            ping_scheduler: PingSlotScheduler::new(),
            network_time: NetworkTime::new(),
        }
    }

    /// Start Class B operation
    pub fn start(&mut self) -> Result<(), Error<R::Error>> {
        // Start beacon acquisition
        self.beacon_tracker.start_acquisition()?;
        Ok(())
    }

    /// Process Class B operations
    pub fn process(&mut self) -> Result<(), Error<R::Error>> {
        // Process beacon tracking
        self.beacon_tracker.process(&mut self.mac)?;

        // Update network time if beacon synchronized
        if self.beacon_tracker.is_synchronized() {
            self.network_time.update(self.beacon_tracker.last_beacon_time());
        }

        // Process ping slots if synchronized
        if let BeaconState::Synchronized = self.beacon_tracker.state() {
            self.process_ping_slots()?;
        }

        Ok(())
    }

    /// Configure ping slot parameters
    pub fn configure_ping_slots(&mut self, periodicity: u8) -> Result<(), Error<R::Error>> {
        self.ping_slot_config.set_periodicity(periodicity);
        self.ping_scheduler.update_schedule(
            &self.ping_slot_config,
            self.network_time.current_time(),
        );
        Ok(())
    }

    /// Process ping slots
    fn process_ping_slots(&mut self) -> Result<(), Error<R::Error>> {
        let current_time = self.network_time.current_time();
        
        // Check if we need to open a ping slot
        if let Some(slot) = self.ping_scheduler.next_slot(current_time) {
            self.open_ping_slot(slot)?;
        }

        Ok(())
    }

    /// Open a ping receive slot
    fn open_ping_slot(&mut self, slot: u32) -> Result<(), Error<R::Error>> {
        // Configure radio for ping slot reception
        self.mac.set_rx_config(
            self.ping_slot_config.data_rate(),
            self.ping_slot_config.frequency(),
        )?;

        // Start reception for ping slot duration
        self.mac.rx_for_duration(30)?; // 30ms ping slot

        Ok(())
    }
} 